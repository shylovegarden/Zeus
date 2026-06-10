//! bounds.rs — provably-bounded resource analysis (WCET steps + stack bytes).
//!
//! Worst-case execution time is undecidable in general, but DECIDABLE on Zeus's
//! restricted shape: `for i in a..b` with constant bounds is a finite loop, there is
//! no heap, and the two unbounded constructs (`while`, recursion) are explicitly
//! reported as "cannot bound" rather than guessed. WCET is therefore an
//! `Option<u64>` (None = unprovable -> if `@wcet` was declared, the build FAILS).
//!
//! This is brick 1 of "Provably Deterministic Computation" (ZEUS_WHITEPAPER.md sec 4.6):
//! reproducible + **time-bounded** + leak-free.

use crate::ast::{Program, Statement, Expression, FunctionAttribute, Type};
use std::collections::{HashMap, HashSet};

const BUILTIN_CALL_COST: u64 = 5;
// NOTE: opaque `extern fn` calls used to be assigned a fixed EXTERN_CALL_COST,
// which fabricated a finite WCET for code the analyzer cannot see into. That was
// dishonest: an extern can run for an unbounded time. Extern/unknown calls now
// make WCET unprovable (None) instead. See Cost::expr / Cost::func.
const STACK_BASE: u64 = 64;

pub struct FnBounds {
    pub name: String,
    pub wcet: Option<u64>,
    pub stack: u64,
    pub declared_wcet: Option<u64>,
    pub declared_stack: Option<u64>,
}

pub struct BoundsReport {
    pub fns: Vec<FnBounds>,
    pub violations: Vec<String>,
    pub detail: String,
}

fn is_builtin(name: &str) -> bool {
    matches!(name, "print"|"println"|"min"|"max"|"clamp"|"abs"|"sqrt"|"pow"|"floor"|"ceil")
}

/// Evaluate a constant integer expression (for loop bounds): literals + literal arithmetic.
fn const_i64(e: &Expression) -> Option<i64> {
    match e {
        Expression::Number(n) => Some(*n as i64),
        Expression::Prefix { operator, operand } if operator == "Minus" => const_i64(operand).and_then(|v| v.checked_neg()),
        Expression::Infix { left, operator, right } => {
            let l = const_i64(left)?; let r = const_i64(right)?;
            match operator.as_str() {
                // Use checked arithmetic: a bogus-low WCET (from silent i64 overflow)
                // must never pass an @wcet requirement. On overflow we return None,
                // which is treated as "cannot prove a bound" (the safe direction).
                "Plus" => l.checked_add(r), "Minus" => l.checked_sub(r),
                "Star" => l.checked_mul(r), "Slash" if r != 0 => l.checked_div(r),
                _ => None,
            }
        }
        _ => None,
    }
}

struct Cost<'a> {
    bodies: HashMap<String, &'a [Statement]>,
    visiting: HashSet<String>,
    memo: HashMap<String, Option<u64>>,
}

impl<'a> Cost<'a> {
    fn expr(&mut self, e: &Expression) -> Option<u64> {
        Some(match e {
            Expression::Number(_) | Expression::Identifier(_) | Expression::StringLiteral(_) => 1,
            Expression::Infix { left, right, .. } => 1 + self.expr(left)? + self.expr(right)?,
            Expression::Prefix { operand, .. } => 1 + self.expr(operand)?,
            Expression::IndexAccess { base, index } | Expression::OramAccess { base, index } =>
                1 + self.expr(base)? + self.expr(index)?,
            Expression::FieldAccess { base, .. } => 1 + self.expr(base)?,
            Expression::ArrayLiteral(xs) => { let mut c = 1; for x in xs { c += self.expr(x)?; } c }
            Expression::FunctionCall { name, arguments } => {
                let mut c = 0;
                for a in arguments { c += self.expr(a)?; }
                if is_builtin(name) { c + BUILTIN_CALL_COST }
                else if self.bodies.contains_key(name) { c + self.func(name)? }
                else {
                    // Opaque extern (or unknown) call: the analyzer cannot see its body,
                    // so its execution time is UNKNOWN. Returning None propagates
                    // "cannot prove a bound" instead of inventing a bogus-low constant.
                    return None;
                }
            }
            Expression::Try(i) | Expression::Comptime(i) => self.expr(i)?,
            _ => 1,
        })
    }

    fn stmt(&mut self, s: &Statement) -> Option<u64> {
        Some(match s {
            Statement::Let { value, .. } => 1 + self.expr(value)?,
            Statement::ExpressionStatement(e) | Statement::Assert(e) | Statement::Return(e) => 1 + self.expr(e)?,
            Statement::If { condition, consequence, alternative } => {
                let c = self.expr(condition)?;
                let t = self.block(consequence)?;
                let f = alternative.as_ref().map_or(Some(0), |a| self.block(a))?;
                c + t.max(f) + 1
            }
            Statement::While { .. } => return None, // unbounded iteration count: cannot prove WCET
            Statement::For { start, end, body, .. } => {
                let lo = const_i64(start); let hi = const_i64(end);
                match (lo, hi) {
                    (Some(a), Some(b)) if b >= a => {
                        // checked_sub guards against i64 overflow in the bound subtraction
                        // (e.g. b = i64::MAX, a = i64::MIN). On overflow: cannot prove.
                        let iters = match b.checked_sub(a) {
                            Some(d) => d as u64,
                            None => return None,
                        };
                        let per = self.block(body)?;
                        iters.saturating_mul(per).saturating_add(1)
                    }
                    _ => return None, // non-constant loop bounds: cannot prove
                }
            }
            Statement::ParallelBlock { statements, .. }
            | Statement::TargetBlock { statements, .. } | Statement::ProofBlock { statements }
            | Statement::EnclaveBlock { statements } | Statement::SafeStateBlock { statements }
            | Statement::CfgBlock { statements, .. } | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements } => self.block(statements)?,
            _ => 1,
        })
    }

    fn block(&mut self, stmts: &[Statement]) -> Option<u64> {
        let mut c = 0u64;
        for s in stmts { c = c.saturating_add(self.stmt(s)?); }
        Some(c)
    }

    fn func(&mut self, name: &str) -> Option<u64> {
        if let Some(v) = self.memo.get(name) { return *v; }
        if self.visiting.contains(name) { return None; } // recursion: cannot bound
        // Opaque extern/unknown callee: body is invisible -> WCET is unprovable.
        let body = match self.bodies.get(name) { Some(b) => *b, None => return None };
        self.visiting.insert(name.to_string());
        let r = self.block(body);
        self.visiting.remove(name);
        self.memo.insert(name.to_string(), r);
        r
    }
}

fn stack_of(body: &[Statement], params: usize, struct_fields: &HashMap<String, usize>) -> u64 {
    fn arr_size(value: &Expression, var_type: &Option<Type>, sf: &HashMap<String, usize>) -> Option<u64> {
        // `[T; N]` annotation
        if let Some(Type::Array(_, size)) = var_type {
            if let Some(n) = const_i64(size) { return Some((n.max(0) as u64) * 8); }
        }
        // SoA `Struct[N]` (Index/Oram with Identifier base)
        if let Expression::IndexAccess { base, index } | Expression::OramAccess { base, index } = value {
            if let Expression::Identifier(sn) = base.as_ref() {
                if let Some(n) = const_i64(index) {
                    let fields = *sf.get(sn).unwrap_or(&1) as u64;
                    return Some((n.max(0) as u64) * fields * 8);
                }
            }
        }
        None
    }
    fn walk(stmts: &[Statement], sf: &HashMap<String, usize>) -> u64 {
        let mut s = 0u64;
        for st in stmts {
            match st {
                Statement::Let { value, var_type, .. } => {
                    s += arr_size(value, var_type, sf).unwrap_or(8);
                }
                Statement::If { consequence, alternative, .. } => {
                    s += walk(consequence, sf);
                    if let Some(a) = alternative { s += walk(a, sf); }
                }
                Statement::For { body, .. } | Statement::While { body, .. } => s += walk(body, sf),
                Statement::ParallelBlock { statements, .. }
                | Statement::TargetBlock { statements, .. } | Statement::ProofBlock { statements }
                | Statement::EnclaveBlock { statements } | Statement::SafeStateBlock { statements }
                | Statement::CfgBlock { statements, .. } | Statement::ComptimeBlock { statements }
                | Statement::ClusterBlock { statements } => s += walk(statements, sf),
                _ => {}
            }
        }
        s
    }
    STACK_BASE + (params as u64) * 8 + walk(body, struct_fields)
}

pub fn analyze(program: &Program) -> BoundsReport {
    // struct field counts (for SoA stack sizing)
    let mut struct_fields: HashMap<String, usize> = HashMap::new();
    fn collect_structs(stmts: &[Statement], sf: &mut HashMap<String, usize>) {
        for s in stmts {
            if let Statement::StructDeclaration { name, fields, .. } = s { sf.insert(name.clone(), fields.len()); }
        }
    }
    collect_structs(&program.statements, &mut struct_fields);

    // function bodies
    let mut bodies: HashMap<String, &[Statement]> = HashMap::new();
    fn collect_fns<'a>(stmts: &'a [Statement], out: &mut Vec<(&'a str, &'a Vec<(String, Type)>, &'a Vec<Statement>, &'a Vec<FunctionAttribute>)>) {
        for s in stmts {
            if let Statement::FunctionDeclaration { name, parameters, body, attributes, .. } = s {
                out.push((name.as_str(), parameters, body, attributes));
                collect_fns(body, out);
            }
        }
    }
    let mut decls = Vec::new();
    collect_fns(&program.statements, &mut decls);
    for (n, _, b, _) in &decls { bodies.insert(n.to_string(), b.as_slice()); }

    let mut cost = Cost { bodies, visiting: HashSet::new(), memo: HashMap::new() };
    let mut fns = Vec::new();
    let mut violations = Vec::new();

    for (name, params, body, attrs) in &decls {
        let wcet = cost.func(name);
        let stack = stack_of(body, params.len(), &struct_fields);
        let mut declared_wcet = None; let mut declared_stack = None;
        for a in attrs.iter() {
            match a {
                FunctionAttribute::Wcet(n) => declared_wcet = Some(*n),
                FunctionAttribute::Stack(n) => declared_stack = Some(*n),
                _ => {}
            }
        }
        if let Some(d) = declared_wcet {
            match wcet {
                None => violations.push(format!("fn {}: @wcet({}) declared but the compiler CANNOT prove a bound (unbounded `while` or recursion)", name, d)),
                Some(c) if c > d => violations.push(format!("fn {}: proven WCET {} steps EXCEEDS @wcet({})", name, c, d)),
                _ => {}
            }
        }
        if let Some(d) = declared_stack {
            if stack > d { violations.push(format!("fn {}: estimated stack {} bytes EXCEEDS @stack({})", name, stack, d)); }
        }
        fns.push(FnBounds { name: name.to_string(), wcet, stack, declared_wcet, declared_stack });
    }

    let mut detail = String::new();
    detail.push_str("Provable resource bounds (WCET steps / stack bytes)\n");
    detail.push_str("===================================================\n");
    for f in &fns {
        let w = f.wcet.map_or("UNBOUNDED".to_string(), |v| format!("{}", v));
        let dw = f.declared_wcet.map_or(String::new(), |d| format!("  (@wcet {})", d));
        let ds = f.declared_stack.map_or(String::new(), |d| format!("  (@stack {})", d));
        detail.push_str(&format!("fn {:<16} : WCET {:>10} steps | stack {:>6} bytes{}{}\n", f.name, w, f.stack, dw, ds));
    }
    BoundsReport { fns, violations, detail }
}


#[cfg(test)]
mod tests {
    use super::*;
    fn parse(src: &str) -> crate::ast::Program {
        let lx = crate::lexer::Lexer::new(src);
        let mut p = crate::parser::Parser::new(lx);
        p.parse_program()
    }
    #[test]
    fn constant_loop_is_bounded() {
        let p = parse("fn f(n: i32) -> i32 { let mut s: i32 = 0; for i in 0..10 { s = s + n; } return s; }");
        let r = analyze(&p);
        assert!(r.fns.iter().find(|x| x.name == "f").unwrap().wcet.is_some());
    }
    #[test]
    fn while_loop_is_unbounded() {
        let p = parse("fn g(n: i32) -> i32 { let mut s: i32 = 0; while n > 0 { s = s + 1; } return s; }");
        let r = analyze(&p);
        assert!(r.fns.iter().find(|x| x.name == "g").unwrap().wcet.is_none());
    }
}
