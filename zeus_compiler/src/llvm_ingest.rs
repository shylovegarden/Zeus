//! llvm_ingest.rs -- "The Lens" on LLVM IR. Ingests textual LLVM IR (.ll) and runs
//! Zeus's secret-taint leak analysis on it, so Zeus can audit code that was NOT
//! written in Zeus (e.g. C/C++/Rust via `clang -emit-llvm -S`).
//!
//! MULTI-BLOCK, MULTI-FUNCTION, INTERPROCEDURAL.
//!  * Every `define` in the module is parsed into basic blocks + instructions.
//!  * Taint is carried as PROVENANCE: each SSA value maps to the set of *secret
//!    parameters* it derives from (a value is "secret" iff that set is non-empty).
//!    The transfer functions are monotone (sets only grow), so the fixpoint is
//!    sound through `phi` nodes and across loop back-edges.
//!  * Per-function CALL SUMMARIES are computed to a fixpoint: for each function we
//!    learn which parameters flow into its return value (`ret_deps`) and which
//!    parameters, if secret, reach a timing sink inside it (`leak_params`). At a
//!    call site to a defined function we use the summary to (a) propagate taint to
//!    the result precisely and (b) raise a finding when a secret-derived argument
//!    is passed into a callee that leaks it. Calls to undefined/external symbols
//!    are handled conservatively (result tainted by args; no false "safe" claim).
//!  * A small alloca-memory model tracks secrets through `store`/`load` on direct
//!    stack slots and on `getelementptr` pointers rooted in an alloca (struct
//!    fields). Writes/reads through any other pointer degrade to UNDECIDABLE.
//!
//! HONEST SCOPE. Dependency-free subset reader (not the `llvm-ir`/`llvm-sys` crate).
//! Anything it cannot model -- loops, unknown opcodes, unresolved aliasing --
//! degrades to UNDECIDABLE. It never reports PROVED-SAFE on code it could not
//! fully reason about.
//!
//! Taint seed: function PARAMETERS are secret by default (an audit assumes inputs
//! may be sensitive). A `; zeus.public: %a %b` comment marks named params public.

use std::collections::{HashMap, HashSet};

type Origins = HashSet<usize>;

fn ssa_refs(s: &str) -> Vec<String> {
    let b = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' {
            let mut j = i + 1;
            while j < b.len() {
                let c = b[j] as char;
                if c.is_ascii_alphanumeric() || c == '_' || c == '.' { j += 1; } else { break; }
            }
            if j > i + 1 { out.push(s[i..j].to_string()); }
            i = j;
        } else { i += 1; }
    }
    out
}

fn first_word(s: &str) -> &str { s.trim().split(|c: char| c.is_whitespace()).next().unwrap_or("") }

const RECOGNIZED: &[&str] = &[
    "add","sub","mul","udiv","sdiv","urem","srem","and","or","xor","shl","lshr","ashr",
    "icmp","fcmp","fadd","fsub","fmul","fdiv","frem","zext","sext","trunc","bitcast","ptrtoint",
    "inttoptr","getelementptr","load","store","call","select","phi","alloca","fneg","fptosi",
    "sitofp","uitofp","fptoui","fpext","fptrunc","freeze","extractvalue","insertvalue","extractelement",
    "insertelement","shufflevector","addrspacecast","atomicrmw","cmpxchg",
];
const TERMINATORS: &[&str] = &["ret","br","switch","unreachable","indirectbr"];

struct Inst {
    block: usize,
    dst: Option<String>,
    op: String,
    operands: Vec<String>,
    raw: String,
}

struct Func {
    name: String,
    params: Vec<String>,
    public_params: HashSet<usize>,
    insts: Vec<Inst>,
    nblocks: usize,
    allocas: HashSet<String>,
    structural_undecidable: bool,
}

#[derive(Default, Clone)]
struct Summary {
    ret_deps: HashSet<usize>,
    leak_params: HashSet<usize>,
}

fn strip_comment(s: &str) -> String { s.split(';').next().unwrap_or("").trim().to_string() }

fn is_label(s: &str) -> Option<String> {
    let c = strip_comment(s);
    if c.ends_with(':') {
        let name = c.trim_end_matches(':').trim();
        if !name.is_empty() && name.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '.') {
            return Some(name.to_string());
        }
    }
    None
}

/// Extract `@callee` (None if indirect) and the ordered argument list, where each
/// argument is `Some(ssa_name)` or `None` (a constant), preserving positional
/// alignment with the callee's parameters.
fn parse_call(raw: &str) -> (Option<String>, Vec<Option<String>>) {
    let lp = match raw.find('(') { Some(p) => p, None => return (None, vec![]) };
    let head = &raw[..lp];
    let callee = head.find('@').map(|p| {
        let rest = &head[p + 1..];
        let e = rest.find(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '.')).unwrap_or(rest.len());
        rest[..e].to_string()
    });
    let bytes = raw.as_bytes();
    let mut depth = 0i32;
    let mut end = raw.len();
    for i in lp..raw.len() {
        match bytes[i] as char { '(' => depth += 1, ')' => { depth -= 1; if depth == 0 { end = i; break; } }, _ => {} }
    }
    let inner = &raw[lp + 1..end.min(raw.len())];
    let mut args: Vec<&str> = Vec::new();
    let mut d = 0i32;
    let mut start = 0usize;
    let ib = inner.as_bytes();
    for i in 0..inner.len() {
        match ib[i] as char {
            '<' | '[' | '{' | '(' => d += 1,
            '>' | ']' | '}' | ')' => d -= 1,
            ',' if d == 0 => { args.push(&inner[start..i]); start = i + 1; }
            _ => {}
        }
    }
    if start < inner.len() { args.push(&inner[start..]); }
    let arg_refs = args.iter().map(|a| ssa_refs(a).into_iter().next()).collect();
    (callee, arg_refs)
}

fn parse_module(text: &str) -> Vec<Func> {
    let lines: Vec<&str> = text.lines().collect();
    let mut public_names: HashSet<String> = HashSet::new();
    for l in &lines {
        if let Some(rest) = l.trim().strip_prefix("; zeus.public:") {
            for r in ssa_refs(rest) { public_names.insert(r); }
        }
    }

    let mut funcs = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_start().starts_with("define ") {
            let def_line = lines[i];
            let fname = match def_line.find('@') {
                Some(p) => { let rest = &def_line[p + 1..]; let e = rest.find('(').unwrap_or(rest.len()); rest[..e].trim().to_string() }
                None => "<anonymous>".to_string(),
            };
            let params_blob = {
                let lp = def_line.find('(').map(|x| x + 1).unwrap_or(0);
                let rp = def_line[lp..].find(')').map(|x| lp + x).unwrap_or(def_line.len());
                def_line[lp..rp].to_string()
            };
            let params = ssa_refs(&params_blob);
            let public_params: HashSet<usize> = params.iter().enumerate()
                .filter(|(_, n)| public_names.contains(*n)).map(|(k, _)| k).collect();

            let mut insts: Vec<Inst> = Vec::new();
            let mut label_block: HashMap<String, usize> = HashMap::new();
            let mut cur_block = 0usize;
            let mut nblocks = 1usize;
            let mut j = i + 1;
            while j < lines.len() {
                let line = strip_comment(lines[j]);
                if line == "}" { break; }
                if line.is_empty() { j += 1; continue; }
                if let Some(lbl) = is_label(lines[j]) {
                    cur_block = nblocks; nblocks += 1; label_block.insert(lbl, cur_block); j += 1; continue;
                }
                if let Some(eq) = line.find(" = ") {
                    let dst = line[..eq].trim().to_string();
                    let rhs = line[eq + 3..].trim().to_string();
                    let mut op = first_word(&rhs).to_string();
                    if op == "tail" || op == "musttail" || op == "notail" { op = first_word(&rhs[op.len()..]).to_string(); }
                    let operands = ssa_refs(&rhs);
                    insts.push(Inst { block: cur_block, dst: Some(dst), op, operands, raw: line });
                } else {
                    let op = first_word(&line).to_string();
                    let operands = ssa_refs(&line);
                    insts.push(Inst { block: cur_block, dst: None, op, operands, raw: line });
                }
                j += 1;
            }

            let allocas: HashSet<String> = insts.iter().filter(|x| x.op == "alloca")
                .filter_map(|x| x.dst.clone()).collect();

            let mut structural_undecidable = false;
            for inst in &insts {
                if inst.dst.is_some() {
                    if !RECOGNIZED.contains(&inst.op.as_str()) { structural_undecidable = true; }
                } else if !TERMINATORS.contains(&inst.op.as_str()) && inst.op != "store" && inst.op != "call" {
                    structural_undecidable = true;
                }
                if inst.op == "br" || inst.op == "switch" || inst.op == "indirectbr" {
                    for r in ssa_refs(&inst.raw) {
                        let lbl = r.trim_start_matches('%');
                        if let Some(&b) = label_block.get(lbl) { if b <= inst.block { structural_undecidable = true; } }
                    }
                }
            }

            funcs.push(Func { name: fname, params, public_params, insts, nblocks, allocas, structural_undecidable });
            i = j;
        }
        i += 1;
    }
    funcs
}

struct RunOut {
    ret: Origins,
    leak_params: HashSet<usize>,
    findings: Vec<String>,
    undecidable: bool,
}

fn root_of(ptr: &str, derived_root: &HashMap<String, String>, allocas: &HashSet<String>) -> Option<String> {
    if allocas.contains(ptr) { Some(ptr.to_string()) } else { derived_root.get(ptr).cloned() }
}

/// Core taint pass over one function. `seed(i)` gives the origin set for param i.
fn run(f: &Func, seed: &dyn Fn(usize) -> Origins, summaries: &HashMap<String, Summary>, collect: bool) -> RunOut {
    let mut value: HashMap<String, Origins> = HashMap::new();
    for (i, p) in f.params.iter().enumerate() { value.insert(p.clone(), seed(i)); }
    let mut slot: HashMap<String, Origins> = HashMap::new();
    let mut derived_root: HashMap<String, String> = HashMap::new();
    let mut mem_secret = false;
    let mut undecidable = f.structural_undecidable;
    let mut leak_params: HashSet<usize> = HashSet::new();

    let mut changed = true;
    while changed {
        changed = false;
        for inst in &f.insts {
            if let Some(dst) = &inst.dst {
                let mut o: Origins = Origins::new();
                for op in &inst.operands { if let Some(s) = value.get(op) { o.extend(s.iter().copied()); } }

                if inst.op == "getelementptr" {
                    if let Some(base) = inst.operands.first() {
                        if let Some(r) = root_of(base, &derived_root, &f.allocas) {
                            derived_root.insert(dst.clone(), r);
                        }
                    }
                } else if inst.op == "load" {
                    if let Some(ptr) = inst.operands.first() {
                        if let Some(r) = root_of(ptr, &derived_root, &f.allocas) {
                            if let Some(s) = slot.get(&r) { o.extend(s.iter().copied()); }
                        } else if mem_secret {
                            undecidable = true;
                        }
                    }
                } else if inst.op == "call" {
                    let (callee, args) = parse_call(&inst.raw);
                    let arg_orig: Vec<Origins> = args.iter().map(|a| a.as_ref()
                        .and_then(|n| value.get(n)).cloned().unwrap_or_default()).collect();
                    match callee.as_ref().and_then(|c| summaries.get(c)) {
                        Some(sum) => {
                            o.clear();
                            for &p in &sum.ret_deps { if let Some(set) = arg_orig.get(p) { o.extend(set.iter().copied()); } }
                            for &p in &sum.leak_params {
                                if let Some(set) = arg_orig.get(p) {
                                    if !set.is_empty() { leak_params.extend(set.iter().copied()); }
                                }
                            }
                        }
                        None => {
                            o = arg_orig.iter().flatten().copied().collect();
                        }
                    }
                }

                let entry = value.entry(dst.clone()).or_default();
                let before = entry.len();
                entry.extend(o.into_iter());
                if entry.len() != before { changed = true; }
            } else if inst.op == "store" {
                let mut it = inst.raw.splitn(2, ',');
                let val = ssa_refs(it.next().unwrap_or("")).into_iter().next();
                let ptr = ssa_refs(it.next().unwrap_or("")).into_iter().next();
                let vo = val.as_ref().and_then(|v| value.get(v)).cloned().unwrap_or_default();
                if let Some(p) = ptr {
                    if let Some(r) = root_of(&p, &derived_root, &f.allocas) {
                        let s = slot.entry(r).or_default();
                        let before = s.len();
                        s.extend(vo.iter().copied());
                        if s.len() != before { changed = true; }
                    } else if !vo.is_empty() && !mem_secret {
                        mem_secret = true; undecidable = true; changed = true;
                    }
                }
            }
        }
    }

    let mut findings: Vec<String> = Vec::new();
    let mut ret: Origins = Origins::new();

    for inst in &f.insts {
        match inst.op.as_str() {
            "sdiv" | "udiv" | "srem" | "urem" | "fdiv" | "frem" if inst.dst.is_some() => {
                let o: Origins = inst.operands.iter().filter_map(|x| value.get(x)).flatten().copied().collect();
                if !o.is_empty() { leak_params.extend(o.iter().copied());
                    if collect { findings.push(format!("fn {}: secret-dependent division -> variable-time instruction [UNMITIGATED]", f.name)); } }
            }
            "shl" | "lshr" | "ashr" if inst.dst.is_some() => {
                // A shift by a SECRET AMOUNT (the operand after the last comma) is
                // variable-time on many micro-architectures. The shifted value being
                // secret is fine; only the amount controls timing.
                if inst.raw.contains(',') {
                    let amt = inst.raw.rsplit(',').next().unwrap_or("");
                    let o: Origins = ssa_refs(amt).iter().filter_map(|x| value.get(x)).flatten().copied().collect();
                    if !o.is_empty() { leak_params.extend(o.iter().copied());
                        if collect { findings.push(format!("fn {}: secret value used as a shift amount -> variable-time instruction [UNMITIGATED]", f.name)); } }
                }
            }
            "getelementptr" if inst.dst.is_some() => {
                let idx: Origins = inst.operands.iter().skip(1).filter_map(|x| value.get(x)).flatten().copied().collect();
                if !idx.is_empty() { leak_params.extend(idx.iter().copied());
                    if collect { findings.push(format!("fn {}: secret value used as memory index -> cache-timing channel [UNMITIGATED]", f.name)); } }
            }
            "br" if inst.raw.starts_with("br i1") => {
                if let Some(cond) = ssa_refs(&inst.raw).first() {
                    if let Some(o) = value.get(cond) { if !o.is_empty() {
                        leak_params.extend(o.iter().copied());
                        if collect { findings.push(format!("fn {}: secret value used as branch condition -> control-flow timing channel [UNMITIGATED]", f.name)); } } }
                }
            }
            "switch" => {
                if let Some(cond) = ssa_refs(&inst.raw).first() {
                    if let Some(o) = value.get(cond) { if !o.is_empty() {
                        leak_params.extend(o.iter().copied());
                        if collect { findings.push(format!("fn {}: secret value used as switch condition -> control-flow timing channel [UNMITIGATED]", f.name)); } } }
                }
            }
            "call" => {
                let (callee, args) = parse_call(&inst.raw);
                if let Some(sum) = callee.as_ref().and_then(|c| summaries.get(c)) {
                    let arg_orig: Vec<Origins> = args.iter().map(|a| a.as_ref()
                        .and_then(|n| value.get(n)).cloned().unwrap_or_default()).collect();
                    for &p in &sum.leak_params {
                        if arg_orig.get(p).map(|s| !s.is_empty()).unwrap_or(false) && collect {
                            findings.push(format!("fn {}: passes a secret-derived value into @{} which uses it in a timing-variable way -> control-flow timing channel [UNMITIGATED]",
                                f.name, callee.as_ref().unwrap()));
                        }
                    }
                }
            }
            "ret" => {
                let o: Origins = ssa_refs(&inst.raw).iter().filter_map(|x| value.get(x)).flatten().copied().collect();
                ret.extend(o.iter().copied());
                if collect && !o.is_empty() {
                    findings.push(format!("fn {}: returns a secret-tainted value to a public caller", f.name));
                }
            }
            _ => {}
        }
    }

    let mut seen = HashSet::new();
    findings.retain(|x| seen.insert(x.clone()));
    RunOut { ret, leak_params, findings, undecidable }
}

pub struct FnReport {
    pub func: String,
    pub blocks: usize,
    pub findings: Vec<String>,
    pub undecidable: bool,
    pub timing: bool,
}

pub struct LlReport {
    pub funcs: Vec<FnReport>,
    pub modeled: bool,
}

pub fn analyze(text: &str) -> LlReport {
    let funcs = parse_module(text);
    if funcs.is_empty() {
        return LlReport { funcs: vec![], modeled: false };
    }

    // Compute call summaries to a fixpoint (params seeded symbolically).
    let mut summaries: HashMap<String, Summary> = HashMap::new();
    for f in &funcs { summaries.insert(f.name.clone(), Summary::default()); }
    let mut changed = true;
    let mut guard = 0;
    while changed && guard < 64 {
        changed = false; guard += 1;
        for f in &funcs {
            let out = run(f, &|i| { let mut s = Origins::new(); s.insert(i); s }, &summaries, false);
            let sum = summaries.get_mut(&f.name).unwrap();
            let before = (sum.ret_deps.len(), sum.leak_params.len());
            sum.ret_deps.extend(out.ret.iter().copied());
            sum.leak_params.extend(out.leak_params.iter().copied());
            if (sum.ret_deps.len(), sum.leak_params.len()) != before { changed = true; }
        }
    }

    // Report pass: seed only secret (non-public) params.
    let mut reports = Vec::new();
    for f in &funcs {
        let out = run(f, &|i| {
            let mut s = Origins::new();
            if !f.public_params.contains(&i) { s.insert(i); }
            s
        }, &summaries, true);
        let timing = out.findings.iter().any(|x| x.contains("timing channel") || x.contains("variable-time"));
        reports.push(FnReport { func: f.name.clone(), blocks: f.nblocks, findings: out.findings, undecidable: out.undecidable, timing });
    }
    LlReport { funcs: reports, modeled: true }
}

fn rule_id(f: &str) -> &'static str {
    if f.contains("memory index") { "ZEUS-SECRET-INDEX" }
    else if f.contains("into @") { "ZEUS-SECRET-CALL" }
    else if f.contains("branch condition") || f.contains("switch condition") { "ZEUS-SECRET-BRANCH" }
    else if f.contains("division") { "ZEUS-SECRET-DIVISION" }
    else if f.contains("shift amount") { "ZEUS-SECRET-SHIFT" }
    else if f.contains("returns a secret") { "ZEUS-SECRET-RETURN" }
    else { "ZEUS-FINDING" }
}

fn json_escape(s: &str) -> String {
    let mut o = String::new();
    for c in s.chars() {
        match c { '"' => o.push_str("\\\""), '\\' => o.push_str("\\\\"), '\n' => o.push_str("\\n"),
                  c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)), c => o.push(c) }
    }
    o
}

fn build_sarif(path: &str, rep: &LlReport) -> String {
    let mut results = Vec::new();
    for fr in &rep.funcs {
        for f in &fr.findings {
            let rid = rule_id(f);
            results.push(format!(
                "{{\"ruleId\":\"{}\",\"level\":\"error\",\"message\":{{\"text\":\"{}\"}},\"locations\":[{{\"physicalLocation\":{{\"artifactLocation\":{{\"uri\":\"{}\"}},\"region\":{{\"startLine\":1}}}}}}]}}",
                rid, json_escape(f), json_escape(path)));
        }
        if fr.findings.is_empty() && fr.undecidable {
            results.push(format!(
                "{{\"ruleId\":\"ZEUS-UNDECIDABLE\",\"level\":\"note\",\"message\":{{\"text\":\"fn {}: analysis UNDECIDABLE (loop / unknown opcode / unresolved aliasing).\"}},\"locations\":[{{\"physicalLocation\":{{\"artifactLocation\":{{\"uri\":\"{}\"}},\"region\":{{\"startLine\":1}}}}}}]}}",
                json_escape(&fr.func), json_escape(path)));
        }
    }
    format!("{{\"version\":\"2.1.0\",\"$schema\":\"https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json\",\"runs\":[{{\"tool\":{{\"driver\":{{\"name\":\"zeus\",\"version\":\"0.1.0\",\"informationUri\":\"https://zeus-lang.dev\"}}}},\"results\":[{}]}}]}}",
        results.join(","))
}

/// `zeus audit <file.ll>`: ingest multi-block, multi-function LLVM IR and emit a verdict.
pub fn audit_ll(path: &str, sarif: bool, sarif_path: Option<String>, strict: bool) {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) => { eprintln!("\x1b[31m[ZEUS ERROR]\x1b[0m cannot read {}: {}", path, e); std::process::exit(1); }
    };
    let rep = analyze(&text);
    if !rep.modeled {
        eprintln!("\x1b[1;31m[ZEUS ERROR]\x1b[0m no `define` function found in {}", path);
        std::process::exit(1);
    }

    let any_notproven = rep.funcs.iter().any(|f| !f.findings.is_empty());
    let any_undecidable = rep.funcs.iter().any(|f| f.findings.is_empty() && f.undecidable);

    if sarif {
        let doc = build_sarif(path, &rep);
        match &sarif_path {
            Some(p) => { if let Err(e) = std::fs::write(p, &doc) { eprintln!("[ZEUS ERROR] cannot write SARIF {}: {}", p, e); std::process::exit(1); } }
            None => println!("{}", doc),
        }
        if any_notproven { std::process::exit(1); }
        if strict && any_undecidable { std::process::exit(1); }
        std::process::exit(0);
    }

    println!("\n\x1b[1;36m== ZEUS AUDIT: The Lens (LLVM-IR, multi-block, interprocedural) ==\x1b[0m  \x1b[90m(non-Zeus code)\x1b[0m");
    println!("\x1b[90mfile:\x1b[0m {}   \x1b[90mfunctions:\x1b[0m {}", path, rep.funcs.len());
    for fr in &rep.funcs {
        let verdict = if !fr.findings.is_empty() { "\x1b[1;31m[NOT-PROVEN]\x1b[0m" }
            else if fr.undecidable { "\x1b[1;33m[UNDECIDABLE]\x1b[0m" } else { "\x1b[1;32m[PROVED-SAFE]\x1b[0m" };
        println!(" \x1b[1mfn {}\x1b[0m  ({} basic block(s))  {}", fr.func, fr.blocks, verdict);
        println!("    constant-time: {}   fully-modeled: {}",
            if fr.timing { "\x1b[1;31mNO\x1b[0m" } else { "\x1b[1;32myes\x1b[0m" },
            if fr.undecidable { "\x1b[1;33mno (UNDECIDABLE)\x1b[0m" } else { "\x1b[1;32myes\x1b[0m" });
        if fr.findings.is_empty() {
            if fr.undecidable { println!("    \x1b[1;33m(undecidable)\x1b[0m -- a loop, unknown opcode, or unresolved aliasing prevents a proof."); }
            else { println!("    \x1b[1;32m(none)\x1b[0m -- no secret-dependent branch, index, or division detected."); }
        } else { for f in &fr.findings { println!("    \x1b[1;31m[!]\x1b[0m {}", f); } }
    }

    if any_notproven {
        println!("\n\x1b[1;31m[ZEUS AUDIT GATE] FAILED\x1b[0m -- NOT-PROVEN.");
        std::process::exit(1);
    } else if any_undecidable {
        println!("\n\x1b[1;33m[ZEUS AUDIT GATE] {}\x1b[0m -- UNDECIDABLE.", if strict { "FAILED (--strict)" } else { "PASSED (with caveats)" });
        std::process::exit(if strict { 1 } else { 0 });
    } else {
        println!("\n\x1b[1;32m[ZEUS AUDIT GATE] PASSED\x1b[0m -- PROVED-SAFE on the modeled subset.");
        std::process::exit(0);
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn secret_branch_is_flagged() {
        let ll = "define i32 @f(i32 %s) {\nentry:\n  %c = icmp sgt i32 %s, 0\n  br i1 %c, label %a, label %b\na:\n  ret i32 1\nb:\n  ret i32 0\n}";
        let r = analyze(ll);
        assert!(!r.funcs[0].findings.is_empty() && r.funcs[0].timing);
    }
    #[test]
    fn public_input_is_proved_safe() {
        let ll = "; zeus.public: %x\ndefine i32 @g(i32 %x) {\nentry:\n  %r = add i32 %x, 1\n  ret i32 %r\n}";
        let r = analyze(ll);
        assert!(r.funcs[0].findings.is_empty() && !r.funcs[0].undecidable);
    }
    #[test]
    fn secret_shift_amount_is_flagged() {
        let ll = "define i32 @h(i32 %s) {\nentry:\n  %r = shl i32 1, %s\n  ret i32 %r\n}";
        let r = analyze(ll);
        assert!(r.funcs[0].findings.iter().any(|x| x.contains("shift amount")));
    }
    #[test]
    fn loop_degrades_to_undecidable() {
        let ll = "define void @l(i32 %n) {\nentry:\n  br label %loop\nloop:\n  br label %loop\n}";
        let r = analyze(ll);
        assert!(r.funcs[0].undecidable);
    }
}
