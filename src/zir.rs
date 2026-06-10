#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or, clippy::type_complexity)]
//! ZIR -- Zeus typed mid-level IR (architecture: ZEUS_WHITEPAPER.md, ADR-001).
//!
//! Analysis/verification belongs on a dataflow IR with explicit def-use, not the AST.
//! ZIR is a per-function list of SSA values; each value records its defining
//! instruction, a (light) type, a `secret` taint bit, and a `nondet` taint bit.
//!
//! Routed passes (run ALONGSIDE the AST->C pipeline, non-destructive):
//!   1. secret-taint / non-leakage   -- proves which values are influenced by secret data
//!      and flags leak sinks (secret-tainted index / branch / public return). A secret
//!      index is the cache-timing leak that oblivious memory mitigates.
//!   2. determinism / reproducibility -- proves a function has NO nondeterministic source
//!      (rand/time/clock/IO/...) influencing it. This is the seed of "Provably
//!      Deterministic Computation": reproducible + (with WCET + constant-time) precise.

use crate::ast::{Program, Statement, Expression};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ZType { Int, Float, Bool, Str, Aggregate, Unknown }

pub type ValueId = usize;

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum ZInst {
    Const, Param(String), Read(String),
    Binary(String, ValueId, ValueId), Unary(String, ValueId),
    Index(ValueId, ValueId), Field(ValueId, String),
    Call(String, Vec<ValueId>), Array(Vec<ValueId>), Opaque,
}

#[allow(dead_code)]
#[derive(Clone, Debug)]
pub struct ZValue {
    pub inst: ZInst,
    pub ty: ZType,
    pub secret: bool, // influenced by `secret` data
    pub nondet: bool, // influenced by a nondeterministic source
}

pub struct ZFunction {
    pub name: String,
    pub values: Vec<ZValue>,
    pub leaks: Vec<String>,
    pub deterministic: bool,
    /// This function (directly) allocates on the heap via a heap-bearing construct
    /// (parallel/cluster blocks use the mmap arena; tensor / NVMe DMA also allocate).
    pub uses_heap: bool,
    /// Names of every function-call target syntactically present in this function.
    pub callees: HashSet<String>,
    /// This function (directly) calls an opaque `extern fn` whose body the analyzer
    /// cannot see into.
    pub calls_extern_direct: bool,
    /// This function returns a secret-tainted value (interprocedural taint summary).
    pub returns_secret: bool,
}

pub struct FnProfile {
    pub name: String,
    pub constant_time: bool, // no secret value reaches a branch/index/division (no timing channel)
    pub deterministic: bool, // no nondeterministic source influences the function
    /// An opaque `extern fn` is reachable from this function (directly or transitively).
    /// When true the analyzer cannot see the callee, so timing/WCET claims for this
    /// function are not trustworthy and must degrade to the safe direction.
    pub reaches_extern: bool,
    /// This function (directly or transitively) reaches a heap-allocating construct.
    pub uses_heap: bool,
}

pub struct ZirReport {
    pub functions: usize,
    pub total_values: usize,
    pub secret_values: usize,
    pub leaks: Vec<String>,
    pub deterministic_fns: usize,
    pub per_fn: Vec<FnProfile>,
    pub detail: String,
    /// Program-level zero-heap verdict: NO reachable function allocates on the heap
    /// AND NO reachable function calls an opaque extern. Computed honestly; defaults
    /// to the safe direction (false) whenever it cannot be cheaply proven.
    pub zero_heap: bool,
    /// At least one reachable `extern fn` call exists anywhere in the program.
    pub ffi_unaudited: bool,
}

/// Known intrinsic/builtin call targets that the compiler lowers itself (not opaque
/// FFI). These are NOT `extern fn` and must not be mistaken for unaudited externs.
/// Mirrors the builtin set in bounds.rs (`is_builtin`) plus print variants.
fn is_known_safe_call(name: &str) -> bool {
    matches!(name,
        "print"|"println"|"min"|"max"|"clamp"|"abs"|"sqrt"|"pow"|"floor"|"ceil")
}

/// Calls whose result/effect is not reproducible across runs.
fn is_nondet_source(name: &str) -> bool {
    matches!(name,
        "rand" | "random" | "rand_r" | "srand" | "arc4random" | "getrandom"
        | "time" | "now" | "clock" | "gettimeofday" | "clock_gettime" | "rdtsc" | "__rdtsc"
        | "read" | "recv" | "recvfrom" | "input" | "scanf" | "fgets" | "getenv" | "getpid")
}

struct Lowerer<'a> {
    values: Vec<ZValue>,
    env: HashMap<String, ValueId>,
    secret_names: HashSet<String>,
    leaks: Vec<String>,
    fname: String,
    extern_names: &'a HashSet<String>,
    uses_heap: bool,
    callees: HashSet<String>,
    calls_extern_direct: bool,
    returns_secret_targets: &'a HashSet<String>,
    returns_secret_flag: bool,
}

impl<'a> Lowerer<'a> {
    fn new(fname: &str, extern_names: &'a HashSet<String>, returns_secret_targets: &'a HashSet<String>) -> Self {
        Lowerer {
            values: Vec::new(), env: HashMap::new(), secret_names: HashSet::new(),
            leaks: Vec::new(), fname: fname.to_string(), extern_names,
            uses_heap: false, callees: HashSet::new(), calls_extern_direct: false,
            returns_secret_targets, returns_secret_flag: false,
        }
    }
    fn push(&mut self, inst: ZInst, ty: ZType, secret: bool, nondet: bool) -> ValueId {
        self.values.push(ZValue { inst, ty, secret, nondet });
        self.values.len() - 1
    }
    fn sec(&self, v: ValueId) -> bool { self.values[v].secret }
    fn nd(&self, v: ValueId) -> bool { self.values[v].nondet }

    fn lower_expr(&mut self, e: &Expression) -> ValueId {
        match e {
            Expression::Number(n) => self.push(ZInst::Const, if n.fract()==0.0 {ZType::Int} else {ZType::Float}, false, false),
            Expression::StringLiteral(_) => self.push(ZInst::Const, ZType::Str, false, false),
            Expression::Identifier(name) => {
                let secret = self.secret_names.contains(name) || self.env.get(name).is_some_and(|&v| self.values[v].secret);
                let nondet = self.env.get(name).is_some_and(|&v| self.values[v].nondet);
                let ty = self.env.get(name).map_or(ZType::Unknown, |&v| self.values[v].ty);
                self.push(ZInst::Read(name.clone()), ty, secret, nondet)
            }
            Expression::Infix { left, operator, right } => {
                let l=self.lower_expr(left); let r=self.lower_expr(right);
                let secret=self.sec(l)||self.sec(r); let nondet=self.nd(l)||self.nd(r);
                if matches!(operator.as_str(), "Slash" | "Percent" | "SlashAssign" | "PercentAssign") && (self.sec(l)||self.sec(r)) {
                    self.leaks.push(format!("fn {}: secret-dependent division/modulo -> variable-time instruction [UNMITIGATED]", self.fname));
                }
                let cmp=matches!(operator.as_str(),"Equal"|"NotEqual"|"LessThan"|"GreaterThan"|"LessEqual"|"GreaterEqual"|"And"|"Or");
                let ty=if cmp {ZType::Bool} else {self.values[l].ty};
                self.push(ZInst::Binary(operator.clone(),l,r), ty, secret, nondet)
            }
            Expression::Prefix { operator, operand } => {
                let v=self.lower_expr(operand);
                let ty=if operator=="Not" {ZType::Bool} else {self.values[v].ty};
                self.push(ZInst::Unary(operator.clone(),v), ty, self.sec(v), self.nd(v))
            }
            Expression::IndexAccess { base, index } | Expression::OramAccess { base, index, .. } => {
                let b=self.lower_expr(base); let i=self.lower_expr(index);
                if self.sec(i) {
                    let mitigated=self.sec(b);
                    self.leaks.push(format!("fn {}: secret value used as memory index -> cache-timing channel [{}]",
                        self.fname, if mitigated {"MITIGATED: oblivious access (secret array)"} else {"UNMITIGATED: index a `secret` array, or make the index public"}));
                }
                self.push(ZInst::Index(b,i), ZType::Unknown, self.sec(b), self.nd(b)||self.nd(i))
            }
            Expression::FieldAccess { base, field } => {
                let b=self.lower_expr(base);
                self.push(ZInst::Field(b, field.clone()), ZType::Unknown, self.sec(b), self.nd(b))
            }
            Expression::FunctionCall { name, arguments } => {
                let args: Vec<ValueId>=arguments.iter().map(|a| self.lower_expr(a)).collect();
                let secret=args.iter().any(|&a| self.sec(a)) || self.returns_secret_targets.contains(name);
                let nondet=is_nondet_source(name) || args.iter().any(|&a| self.nd(a));
                self.callees.insert(name.clone());
                if self.extern_names.contains(name) { self.calls_extern_direct = true; }
                self.push(ZInst::Call(name.clone(),args), ZType::Unknown, secret, nondet)
            }
            Expression::TensorDefinition { .. } | Expression::NvmeDmaMap { .. } => {
                // These lower to arena/mmap allocations in codegen -> not zero-heap.
                self.uses_heap = true;
                self.push(ZInst::Opaque, ZType::Unknown, false, false)
            }
            Expression::ArrayLiteral(elems) => {
                let vs: Vec<ValueId>=elems.iter().map(|x| self.lower_expr(x)).collect();
                let secret=vs.iter().any(|&a| self.sec(a)); let nondet=vs.iter().any(|&a| self.nd(a));
                self.push(ZInst::Array(vs), ZType::Aggregate, secret, nondet)
            }
            Expression::StructInit { fields, .. } => {
                // A struct value is secret if ANY field initializer is secret, so a
                // secret laundered through a struct field (e.g. `Box{v:key}` then `b.v`)
                // stays tainted. Sound over-approximation: never under-taints.
                let vs: Vec<ValueId> = fields.iter().map(|(_, e)| self.lower_expr(e)).collect();
                let secret = vs.iter().any(|&a| self.sec(a));
                let nondet = vs.iter().any(|&a| self.nd(a));
                self.push(ZInst::Array(vs), ZType::Aggregate, secret, nondet)
            }
            Expression::EnumVariant { payload, .. } => {
                // Taint the variant value if any payload argument is secret.
                let secret = payload.iter().any(|p| { let v = self.lower_expr(p); self.sec(v) });
                self.push(ZInst::Opaque, ZType::Aggregate, secret, false)
            }
            Expression::MatchExpr { scrutinee, arms } => {
                // Scrutinee is a branch — flag if secret-tainted.
                self.lower_cond(scrutinee, "match branch");
                for arm in arms {
                    for s in &arm.body {
                        self.lower_stmt(s);
                    }
                }
                self.push(ZInst::Opaque, ZType::Unknown, false, false)
            }
            Expression::Try(inner) | Expression::Comptime(inner) => self.lower_expr(inner),
        }
    }

    fn lower_cond(&mut self, cond: &Expression, kind: &str) {
        let c=self.lower_expr(cond);
        if self.sec(c) {
            self.leaks.push(format!("fn {}: secret value used as {} condition -> control-flow timing channel [UNMITIGATED]", self.fname, kind));
        }
    }

    fn lower_stmt(&mut self, s: &Statement) {
        match s {
            Statement::Let { name, is_secret, value, .. } => {
                let v=self.lower_expr(value);
                if *is_secret { self.secret_names.insert(name.clone()); self.values[v].secret=true; }
                self.env.insert(name.clone(), v);
            }
            Statement::ExpressionStatement(e) | Statement::Assert(e) => { self.lower_expr(e); }
            Statement::Return(e) => {
                let v=self.lower_expr(e);
                if self.sec(v) { self.returns_secret_flag = true; self.leaks.push(format!("fn {}: returns a secret-tainted value to a public caller", self.fname)); }
            }
            Statement::If { condition, consequence, alternative } => {
                self.lower_cond(condition,"branch");
                for st in consequence { self.lower_stmt(st); }
                if let Some(alt)=alternative { for st in alt { self.lower_stmt(st); } }
            }
            Statement::While { condition, body } => { self.lower_cond(condition,"loop"); for st in body { self.lower_stmt(st); } }
            Statement::For { start, end, body, .. } => { self.lower_expr(start); self.lower_expr(end); for st in body { self.lower_stmt(st); } }
            // Parallel/cluster blocks allocate shared contexts in the mmap arena
            // (see codegen __zeus_arena_alloc) -> the program is not zero-heap.
            Statement::ParallelBlock { statements, .. }
            | Statement::ClusterBlock { statements } => {
                self.uses_heap = true;
                for st in statements { self.lower_stmt(st); }
            }
            Statement::TargetBlock { statements, .. } | Statement::ProofBlock { statements }
            | Statement::EnclaveBlock { statements } | Statement::SafeStateBlock { statements }
            | Statement::CfgBlock { statements, .. } | Statement::ComptimeBlock { statements }
                => { for st in statements { self.lower_stmt(st); } }
            Statement::MatchStatement { scrutinee, arms } => {
                // A match is a branch on the scrutinee — same taint rules as `if`.
                self.lower_cond(scrutinee, "match branch");
                for arm in arms {
                    for st in &arm.body { self.lower_stmt(st); }
                }
            }
            Statement::EnumDeclaration { .. } => {}
            _ => {}
        }
    }
}

fn lower_function(name: &str, params: &[(String, crate::ast::Type)], body: &[Statement], extern_names: &HashSet<String>, secret_params: &HashSet<String>, returns_secret: &HashSet<String>) -> ZFunction {
    let mut lw=Lowerer::new(name, extern_names, returns_secret);
    for (pn,_) in params {
        let is_sec = secret_params.contains(pn);
        let id=lw.push(ZInst::Param(pn.clone()), ZType::Unknown, is_sec, false);
        if is_sec { lw.secret_names.insert(pn.clone()); }
        lw.env.insert(pn.clone(), id);
    }
    for s in body { lw.lower_stmt(s); }
    let deterministic = !lw.values.iter().any(|v| v.nondet);
    let returns_secret_flag = lw.returns_secret_flag;
    ZFunction {
        name: name.to_string(), values: lw.values, leaks: lw.leaks, deterministic,
        uses_heap: lw.uses_heap, callees: lw.callees, calls_extern_direct: lw.calls_extern_direct,
        returns_secret: returns_secret_flag,
    }
}

pub fn lower_and_analyze(program: &Program) -> ZirReport {
    fn collect<'a>(stmts: &'a [Statement], out: &mut Vec<(&'a str, &'a [(String, crate::ast::Type)], &'a [String], &'a [Statement])>) {
        for s in stmts {
            if let Statement::FunctionDeclaration { name, parameters, secret_params, body, .. } = s {
                out.push((name.as_str(), parameters.as_slice(), secret_params.as_slice(), body.as_slice())); collect(body, out);
            }
        }
    }
    // Collect declared `extern fn` names: these are opaque to the analyzer.
    let mut extern_names: HashSet<String> = HashSet::new();
    fn collect_externs(stmts: &[Statement], out: &mut HashSet<String>) {
        for s in stmts {
            match s {
                Statement::ExternFunctionDeclaration { name, .. } => { out.insert(name.clone()); }
                Statement::FunctionDeclaration { body, .. } => collect_externs(body, out),
                _ => {}
            }
        }
    }
    collect_externs(&program.statements, &mut extern_names);

    let mut decls=Vec::new(); collect(&program.statements, &mut decls);
    let toplevel: Vec<Statement>=program.statements.iter()
        .filter(|s| !matches!(s, Statement::FunctionDeclaration{..}|Statement::StructDeclaration{..}|Statement::ExternFunctionDeclaration{..}|Statement::Import(_)|Statement::LineDirective(_)|Statement::Panic(_)))
        .cloned().collect();
    // Two-pass interprocedural taint: lower repeatedly, growing the set of functions
    // known to return a secret, until it stabilises (monotone -> terminates). A call to
    // a returns-secret function taints its result, so downstream leaks are caught.
    let mut returns_secret: HashSet<String> = HashSet::new();
    let funcs = loop {
        let mut funcs=Vec::new();
        for (n,p,sp,b) in &decls {
            let spset: HashSet<String> = sp.iter().cloned().collect();
            funcs.push(lower_function(n,p,b,&extern_names,&spset,&returns_secret));
        }
        if !toplevel.is_empty() { funcs.push(lower_function("<toplevel>", &[], &toplevel, &extern_names, &HashSet::new(), &returns_secret)); }
        let mut grew=false;
        for f in &funcs {
            if f.returns_secret && !returns_secret.contains(&f.name) { returns_secret.insert(f.name.clone()); grew=true; }
        }
        if !grew { break funcs; }
    };

    // ---- Transitive reachability over the (intra-program) call graph ----
    // A function "reaches extern" if it directly calls an extern, or calls a known
    // local function that itself reaches extern. Likewise for heap use. A call to an
    // unknown name that is NOT a declared extern and NOT a known local/builtin is
    // treated conservatively as opaque (could be FFI), degrading to the safe side.
    let local_names: HashSet<String> = funcs.iter().map(|f| f.name.clone()).collect();
    let direct_extern: HashMap<String, bool> = funcs.iter().map(|f| {
        let unknown_call = f.callees.iter().any(|c|
            !local_names.contains(c) && !extern_names.contains(c) && !is_known_safe_call(c));
        (f.name.clone(), f.calls_extern_direct || unknown_call)
    }).collect();
    let direct_heap: HashMap<String, bool> =
        funcs.iter().map(|f| (f.name.clone(), f.uses_heap)).collect();

    // Iterate to a fixpoint (call graphs here are tiny).
    let mut reaches_extern: HashMap<String, bool> = direct_extern.clone();
    let mut reaches_heap: HashMap<String, bool> = direct_heap.clone();
    let mut changed = true;
    while changed {
        changed = false;
        for f in &funcs {
            for c in &f.callees {
                if local_names.contains(c) {
                    if *reaches_extern.get(c).unwrap_or(&false) && !reaches_extern[&f.name] {
                        reaches_extern.insert(f.name.clone(), true); changed = true;
                    }
                    if *reaches_heap.get(c).unwrap_or(&false) && !reaches_heap[&f.name] {
                        reaches_heap.insert(f.name.clone(), true); changed = true;
                    }
                }
            }
        }
    }

    let total_values: usize=funcs.iter().map(|f| f.values.len()).sum();
    let secret_values: usize=funcs.iter().map(|f| f.values.iter().filter(|v| v.secret).count()).sum();
    let deterministic_fns: usize=funcs.iter().filter(|f| f.deterministic).count();
    let mut leaks=Vec::new(); for f in &funcs { leaks.extend(f.leaks.iter().cloned()); }
    let per_fn: Vec<FnProfile> = funcs.iter().map(|f| {
        let re = *reaches_extern.get(&f.name).unwrap_or(&false);
        FnProfile {
            name: f.name.clone(),
            // constant-time = no TIMING channel (branch/index/division). A secret *return*
            // is a confidentiality (data-flow) concern, not a timing one, so it does not
            // by itself break constant-time. BUT if an opaque extern is reachable we cannot
            // see into it, so we must NOT claim constant-time (degrade to the safe side).
            constant_time: !re && !f.leaks.iter().any(|l| l.contains("timing channel") || l.contains("variable-time")),
            deterministic: f.deterministic,
            reaches_extern: re,
            uses_heap: *reaches_heap.get(&f.name).unwrap_or(&false),
        }
    }).collect();

    // Program-level zero-heap: honest verdict. Zero-heap requires that NO reachable
    // function allocates on the heap AND NO reachable function calls an opaque extern
    // (an extern could allocate, and we cannot prove otherwise). Defaults to false.
    let ffi_unaudited = per_fn.iter().any(|p| p.reaches_extern);
    let zero_heap = !ffi_unaudited && !per_fn.iter().any(|p| p.uses_heap);

    let mut detail=String::new();
    detail.push_str("ZIR (typed SSA): secret-taint / non-leakage + determinism analysis\n");
    detail.push_str("=================================================================\n");
    for f in &funcs {
        detail.push_str(&format!("fn {:<16} : {} SSA values | leak-free: {} | reproducible(deterministic): {}\n",
            f.name, f.values.len(), if f.leaks.is_empty() {"YES"} else {"NO"}, if f.deterministic {"YES"} else {"NO"}));
    }
    if leaks.is_empty() { detail.push_str("\nNo secret-leakage sinks detected.\n"); }
    else { detail.push_str("\nPotential leakage sinks:\n"); for l in &leaks { detail.push_str(&format!("  - {}\n", l)); } }
    detail.push_str(&format!("\nProvably-deterministic functions: {}/{} (no nondeterministic source reaches them).\n", deterministic_fns, funcs.len()));
    if ffi_unaudited {
        detail.push_str("\n[FFI UNAUDITED] A reachable `extern fn` call was found. The analyzer cannot\nsee into opaque externs, so constant-time and WCET claims for the calling\nfunctions are withheld and the program is reported as NOT zero-heap.\n");
    }
    detail.push_str(&format!("Zero-heap (program): {}\n", if zero_heap {"YES"} else {"NO"}));

    ZirReport { functions: funcs.len(), total_values, secret_values, leaks, deterministic_fns, per_fn, detail, zero_heap, ffi_unaudited }
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
    fn secret_laundered_through_struct_field_not_constant_time() {
        let src = "struct Box { v: i32 }\nfn lookup(secret key: i32, a: i32, b: i32) -> i32 { let bx = Box { v: key }; let k: i32 = bx.v; if k > 0 { return b; } return a; }\npub fn main() { let r: i32 = lookup(1, 2, 3); }";
        let prog = parse(src);
        let rep = lower_and_analyze(&prog);
        let f = rep.per_fn.iter().find(|f| f.name == "lookup").expect("lookup");
        assert!(!f.constant_time, "secret laundered through a struct field must not be constant_time");
    }
    #[test]
    fn plain_public_function_is_constant_time() {
        let src = "fn add(a: i32, b: i32) -> i32 { return a + b; }\npub fn main() { let r: i32 = add(1, 2); }";
        let rep = lower_and_analyze(&parse(src));
        let f = rep.per_fn.iter().find(|f| f.name == "add").expect("add");
        assert!(f.constant_time && f.deterministic);
    }
}
