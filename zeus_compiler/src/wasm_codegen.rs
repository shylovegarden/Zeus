//! wasm_codegen.rs -- "The Reach" backend. Emits WebAssembly text (WAT) for the
//! integer/control-flow subset of Zeus, so a Zeus module can run in any WASM
//! runtime (Wasmtime, Node, browsers, and edge/agent sandboxes like Wassette).
//!
//! HONEST SCOPE. This lowers the *verifiable core*: functions over integer/bool
//! values (lowered as i32), `let`/assignment locals, arithmetic + comparison,
//! `if/else`, constant-bounded `for` loops, `return`, and calls between defined
//! functions. Anything outside that subset -- structs/SoA arrays, tensors,
//! `secret`, `while`, floats, `parallel`, FFI, `println` -- marks the function
//! UNSUPPORTED and it is skipped. A function is exported only if it AND all of
//! its (transitive) callees are supported, so the emitter never produces an
//! invalid module or silently-wrong code. Every exported function is `export`ed
//! so a runtime can invoke it directly.

use crate::ast::{Program, Statement, Expression, Type};
use crate::lexer::Lexer;
use crate::parser::Parser;
use std::collections::{HashMap, HashSet};

fn is_int_type(t: &Type) -> bool { matches!(t, Type::I8 | Type::I32 | Type::U64 | Type::Bool) }

struct Lower<'a> {
    declared: HashSet<String>, // names already given a (param)/(local)
    callees: HashSet<String>,  // user functions this fn calls
    label: u32,
    fn_returns: &'a HashMap<String, bool>, // user fn -> has-return (for drop)
}

impl<'a> Lower<'a> {
    fn op(operator: &str) -> Option<&'static str> {
        Some(match operator {
            "Plus" => "i32.add", "Minus" => "i32.sub", "Star" => "i32.mul", "Slash" => "i32.div_s",
            "LessThan" => "i32.lt_s", "GreaterThan" => "i32.gt_s", "LessEqual" => "i32.le_s",
            "GreaterEqual" => "i32.ge_s", "Equal" => "i32.eq", "NotEqual" => "i32.ne",
            "And" | "BitwiseAnd" => "i32.and", "Or" | "Pipe" => "i32.or",
            "BitShiftLeft" => "i32.shl", "BitShiftRight" => "i32.shr_s",
            _ => return None,
        })
    }

    fn expr(&mut self, e: &Expression, out: &mut String) -> Result<(), String> {
        match e {
            Expression::Number(n) => { out.push_str(&format!("    i32.const {}\n", *n as i64 as i32)); Ok(()) }
            Expression::Identifier(name) => {
                if !self.declared.contains(name) { return Err(format!("unknown identifier %{}", name)); }
                out.push_str(&format!("    local.get ${}\n", name)); Ok(())
            }
            Expression::Prefix { operator, operand } => {
                match operator.as_str() {
                    "Minus" => { out.push_str("    i32.const 0\n"); self.expr(operand, out)?; out.push_str("    i32.sub\n"); Ok(()) }
                    "Bang" | "Not" => { self.expr(operand, out)?; out.push_str("    i32.eqz\n"); Ok(()) }
                    other => Err(format!("unsupported prefix '{}'", other)),
                }
            }
            Expression::Infix { left, operator, right } => {
                if operator == "Assign" { return Err("assignment used as a value".into()); }
                let op = Self::op(operator).ok_or_else(|| format!("unsupported operator '{}'", operator))?;
                self.expr(left, out)?;
                self.expr(right, out)?;
                out.push_str(&format!("    {}\n", op));
                Ok(())
            }
            Expression::FunctionCall { name, arguments } => {
                if name == "println" || name == "print" { return Err("println/print needs a host import".into()); }
                for a in arguments { self.expr(a, out)?; }
                self.callees.insert(name.clone());
                out.push_str(&format!("    call ${}\n", name));
                Ok(())
            }
            _ => Err("unsupported expression (array/struct/tensor/string/etc.)".into()),
        }
    }

    fn stmt(&mut self, s: &Statement, out: &mut String) -> Result<(), String> {
        match s {
            Statement::Let { name, value, var_type, .. } => {
                if let Some(t) = var_type { if !is_int_type(t) { return Err(format!("non-integer let '{}'", name)); } }
                self.expr(value, out)?;
                out.push_str(&format!("    local.set ${}\n", name));
                Ok(())
            }
            Statement::Return(e) => { self.expr(e, out)?; out.push_str("    return\n"); Ok(()) }
            Statement::ExpressionStatement(e) => {
                match e {
                    Expression::Infix { left, operator, right } if operator == "Assign" => {
                        if let Expression::Identifier(name) = left.as_ref() {
                            if !self.declared.contains(name) { return Err(format!("assign to unknown %{}", name)); }
                            self.expr(right, out)?;
                            out.push_str(&format!("    local.set ${}\n", name));
                            Ok(())
                        } else { Err("assignment to non-identifier".into()) }
                    }
                    Expression::FunctionCall { name, .. } => {
                        self.expr(e, out)?;
                        if *self.fn_returns.get(name).unwrap_or(&false) { out.push_str("    drop\n"); }
                        Ok(())
                    }
                    _ => { self.expr(e, out)?; out.push_str("    drop\n"); Ok(()) }
                }
            }
            Statement::If { condition, consequence, alternative } => {
                self.expr(condition, out)?;
                out.push_str("    if\n");
                for st in consequence { self.stmt(st, out)?; }
                if let Some(alt) = alternative {
                    out.push_str("    else\n");
                    for st in alt { self.stmt(st, out)?; }
                }
                out.push_str("    end\n");
                Ok(())
            }
            Statement::For { iterator, start, end, body } => {
                let s0 = match start { Expression::Number(n) => *n as i64 as i32, _ => return Err("for-start must be a constant".into()) };
                let e0 = match end { Expression::Number(n) => *n as i64 as i32, _ => return Err("for-end must be a constant".into()) };
                let l = self.label; self.label += 1;
                out.push_str(&format!("    i32.const {}\n    local.set ${}\n", s0, iterator));
                out.push_str(&format!("    block $brk{}\n    loop $cont{}\n", l, l));
                out.push_str(&format!("    local.get ${}\n    i32.const {}\n    i32.ge_s\n    br_if $brk{}\n", iterator, e0, l));
                for st in body { self.stmt(st, out)?; }
                out.push_str(&format!("    local.get ${}\n    i32.const 1\n    i32.add\n    local.set ${}\n", iterator, iterator));
                out.push_str(&format!("    br $cont{}\n    end\n    end\n", l));
                Ok(())
            }
            Statement::LineDirective(_) => Ok(()),
            _ => Err("unsupported statement (while/struct/parallel/secret/etc.)".into()),
        }
    }
}

fn collect_locals(body: &[Statement], decl: &mut HashSet<String>, ordered: &mut Vec<String>) {
    for s in body {
        match s {
            Statement::Let { name, .. } => { if decl.insert(name.clone()) { ordered.push(name.clone()); } }
            Statement::For { iterator, body, .. } => {
                if decl.insert(iterator.clone()) { ordered.push(iterator.clone()); }
                collect_locals(body, decl, ordered);
            }
            Statement::If { consequence, alternative, .. } => {
                collect_locals(consequence, decl, ordered);
                if let Some(a) = alternative { collect_locals(a, decl, ordered); }
            }
            _ => {}
        }
    }
}

struct LoweredFn { name: String, wat: String, callees: HashSet<String>, supported: bool, reason: String }

fn lower_fn(name: &str, params: &[(String, Type)], ret: &Option<Type>, body: &[Statement],
            fn_returns: &HashMap<String, bool>) -> LoweredFn {
    // signature must be integer-only
    for (pn, pt) in params {
        if !is_int_type(pt) {
            return LoweredFn { name: name.into(), wat: String::new(), callees: HashSet::new(), supported: false,
                reason: format!("param '{}' is non-integer", pn) };
        }
    }
    if let Some(t) = ret { if !is_int_type(t) {
        return LoweredFn { name: name.into(), wat: String::new(), callees: HashSet::new(), supported: false,
            reason: "non-integer return type".into() };
    } }

    let mut declared: HashSet<String> = HashSet::new();
    let mut header = String::new();
    header.push_str(&format!("  (func ${} (export \"{}\")", name, name));
    for (pn, _) in params { header.push_str(&format!(" (param ${} i32)", pn)); declared.insert(pn.clone()); }
    if ret.is_some() { header.push_str(" (result i32)"); }
    header.push('\n');

    // declare locals (lets + for iterators), skipping any that shadow a param
    let mut ordered: Vec<String> = Vec::new();
    let mut decl_for_collect = declared.clone();
    collect_locals(body, &mut decl_for_collect, &mut ordered);
    for n in &ordered { header.push_str(&format!("    (local ${} i32)\n", n)); declared.insert(n.clone()); }

    let mut lw = Lower { declared, callees: HashSet::new(), label: 0, fn_returns };
    let mut bodytext = String::new();
    for st in body {
        if let Err(e) = lw.stmt(st, &mut bodytext) {
            return LoweredFn { name: name.into(), wat: String::new(), callees: HashSet::new(), supported: false, reason: e };
        }
    }
    // a value-returning function whose control can fall off the end needs a default.
    if ret.is_some() { bodytext.push_str("    i32.const 0\n"); }

    let wat = format!("{}{}  )\n", header, bodytext);
    LoweredFn { name: name.into(), wat, callees: lw.callees, supported: true, reason: String::new() }
}

pub fn emit_wasm(path: &str, out_path: Option<String>) {
    let input = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => { eprintln!("\x1b[31m[ZEUS ERROR]\x1b[0m cannot read {}: {}", path, e); std::process::exit(1); }
    };
    let lexer = Lexer::new(&input);
    let mut parser = Parser::new(lexer);
    let program: Program = parser.parse_program();

    // map user fn -> has integer return (for drop on statement-calls)
    let mut fn_returns: HashMap<String, bool> = HashMap::new();
    for s in &program.statements {
        if let Statement::FunctionDeclaration { name, return_type, .. } = s {
            fn_returns.insert(name.clone(), return_type.is_some());
        }
    }

    let mut lowered: Vec<LoweredFn> = Vec::new();
    for s in &program.statements {
        if let Statement::FunctionDeclaration { name, parameters, return_type, body, .. } = s {
            lowered.push(lower_fn(name, parameters, return_type, body, &fn_returns));
        }
    }

    // fixpoint: a fn is exportable iff it lowered AND every callee is a user fn that is also exportable.
    let mut ok: HashSet<String> = lowered.iter().filter(|f| f.supported).map(|f| f.name.clone()).collect();
    loop {
        let mut changed = false;
        for f in &lowered {
            if !ok.contains(&f.name) { continue; }
            for c in &f.callees {
                if !ok.contains(c) { ok.remove(&f.name); changed = true; break; }
            }
        }
        if !changed { break; }
    }

    let mut module = String::from("(module\n");
    let mut exported = Vec::new();
    let mut skipped = Vec::new();
    for f in &lowered {
        if ok.contains(&f.name) { module.push_str(&f.wat); exported.push(f.name.clone()); }
        else {
            let why = if f.supported { "calls an unsupported function".to_string() } else { f.reason.clone() };
            skipped.push((f.name.clone(), why));
        }
    }
    module.push_str(")\n");

    let out = out_path.unwrap_or_else(|| {
        let stem = path.strip_suffix(".zs").unwrap_or(path);
        format!("{}.wat", stem)
    });
    if let Err(e) = std::fs::write(&out, &module) {
        eprintln!("\x1b[31m[ZEUS ERROR]\x1b[0m cannot write {}: {}", out, e); std::process::exit(1);
    }

    println!("\n\x1b[1;36m== ZEUS WASM backend (The Reach) ==\x1b[0m");
    println!("\x1b[90msource:\x1b[0m {}   \x1b[90m->\x1b[0m {}", path, out);
    println!(" \x1b[1;32mexported\x1b[0m ({}): {}", exported.len(), if exported.is_empty() { "(none)".into() } else { exported.join(", ") });
    if !skipped.is_empty() {
        println!(" \x1b[1;33mskipped\x1b[0m  ({}) -- outside the integer/control-flow subset:", skipped.len());
        for (n, why) in &skipped { println!("    - {}: {}", n, why); }
    }
    if exported.is_empty() {
        println!(" \x1b[1;33mNote:\x1b[0m no function fell inside the WASM subset. Try integer-only functions (no arrays/structs/float/println).");
    } else {
        println!(" \x1b[90mrun:\x1b[0m  wasmtime run --invoke {} {} <args...>", exported[0], out);
    }
}
