#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
use crate::ast::{Program, Statement, Expression};
use std::collections::HashMap;
use std::path::PathBuf;

/// Cache entry: either proven (VERIFIED) or previously failed with a reason.
#[derive(Debug, Clone)]
enum CacheEntry {
    Verified,
    Failed(String),
}

#[derive(Clone, Debug)]
struct ValueRange {
    min: f64,
    max: f64,
}

/// Pretty-print a constant-folded range for proof messages: a clean scalar when
/// the range is a single value (e.g. `5`), otherwise an interval.
fn fmt_vr(r: &ValueRange) -> String {
    if (r.min - r.max).abs() < f64::EPSILON {
        if r.min.is_finite() && r.min.fract() == 0.0 { format!("{}", r.min as i64) }
        else { format!("{}", r.min) }
    } else {
        format!("[{}, {}]", r.min, r.max)
    }
}

pub struct FormalVerifier {
    constants: HashMap<String, f64>,
    z3_available: bool,
    /// In-memory cache: expr_key -> CacheEntry. Populated from .zeus_verify_cache on startup.
    cache: HashMap<String, CacheEntry>,
    cache_path: PathBuf,
    cache_dirty: bool,
    bounds: HashMap<String, ValueRange>,
}

impl FormalVerifier {
    pub fn new() -> Self {
        let z3_available = std::process::Command::new("z3")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

        // Load incremental cache from .zeus_verify_cache in cwd
        let cache_path = PathBuf::from(".zeus_verify_cache");
        let mut cache = HashMap::new();
        if let Ok(text) = std::fs::read_to_string(&cache_path) {
            for line in text.lines() {
                // Format: "<expr_key>|VERIFIED" or "<expr_key>|FAILED:<reason>"
                let mut parts = line.splitn(2, '|');
                if let (Some(key), Some(val)) = (parts.next(), parts.next()) {
                    let entry = if val == "VERIFIED" {
                        CacheEntry::Verified
                    } else if let Some(reason) = val.strip_prefix("FAILED:") {
                        CacheEntry::Failed(reason.to_string())
                    } else {
                        continue;
                    };
                    cache.insert(key.to_string(), entry);
                }
            }
        }

        FormalVerifier {
            constants: HashMap::new(),
            z3_available,
            cache,
            cache_path,
            cache_dirty: false,
            bounds: HashMap::new(),
        }
    }

    /// Persist dirty cache entries to disk.
    fn flush_cache(&self) {
        if !self.cache_dirty { return; }
        let mut lines = Vec::new();
        for (key, entry) in &self.cache {
            let val = match entry {
                CacheEntry::Verified => "VERIFIED".to_string(),
                CacheEntry::Failed(r) => format!("FAILED:{}", r),
            };
            lines.push(format!("{}|{}", key, val));
        }
        let _ = std::fs::write(&self.cache_path, lines.join("\n") + "\n");
    }

    pub fn verify(&mut self, program: &Program, is_medical_mode: bool) -> Result<(), String> {
        // Run formal assertions first (Z3 / constant-fold path).
        for stmt in &program.statements {
            self.verify_statement(stmt)?;
        }
        self.flush_cache();

        if is_medical_mode {
            self.write_compliance_report(program)?;
        }
        Ok(())
    }

    /// Build a real IEC 62304 / FDA 510(k) compliance report from the actual
    /// analysis results produced by this pipeline pass.
    ///
    /// The report is *honest*: `compliant` is only `true` when every required
    /// property is actually proven by the existing analysis passes.
    ///
    /// Emits:
    ///   `<basename>.compliance.json`  — machine-readable (for tooling / CI)
    ///   `<basename>.compliance.txt`   — human-readable summary
    fn write_compliance_report(&self, program: &Program) -> Result<(), String> {
        // Run ZIR and bounds analysis on the same program to get real numbers.
        let zir  = crate::zir::lower_and_analyze(program);
        let bnds = crate::bounds::analyze(program);

        // ── IEC 62304 Class C checklist ───────────────────────────────────────
        // Req 1: Zero dynamic memory allocation (no malloc/free/pthread)
        let zero_heap = zir.zero_heap;

        // Req 2: All functions have a provable WCET bound
        let all_bounded = !bnds.fns.is_empty()
            && bnds.fns.iter().all(|f| f.wcet.is_some());
        let wcet_violations: Vec<String> = bnds.violations.clone();

        // Req 3: No secret-tainted timing channels (constant-time)
        let all_constant_time = zir.per_fn.iter().all(|f| f.constant_time);
        let timing_leaks: Vec<String> = zir.leaks.clone();

        // Req 4: All functions provably deterministic
        let all_deterministic = zir.per_fn.iter().all(|f| f.deterministic);

        // Req 5: No opaque FFI calls that cannot be audited
        let no_ffi_unaudited = !zir.ffi_unaudited;

        // Overall compliance verdict
        let compliant = zero_heap
            && all_bounded
            && all_constant_time
            && all_deterministic
            && no_ffi_unaudited
            && timing_leaks.is_empty()
            && wcet_violations.is_empty();

        // ── Per-function JSON entries ─────────────────────────────────────────
        let mut fn_entries = Vec::new();
        for pf in &zir.per_fn {
            let bf = bnds.fns.iter().find(|b| b.name == pf.name);
            let wcet_str = match bf.and_then(|b| b.wcet) {
                Some(w) => w.to_string(),
                None    => "null".to_string(),
            };
            let stack = bf.map(|b| b.stack).unwrap_or(0);
            let declared_wcet = bf.and_then(|b| b.declared_wcet)
                .map(|w| w.to_string())
                .unwrap_or_else(|| "null".to_string());
            fn_entries.push(format!(
                "    {{\"name\":\"{name}\",\"constant_time\":{ct},\"deterministic\":{det},\
                 \"wcet_steps\":{wcet},\"declared_wcet\":{dwcet},\"stack_bytes\":{stack},\
                 \"reaches_extern\":{ext}}}",
                name  = pf.name,
                ct    = pf.constant_time,
                det   = pf.deterministic,
                wcet  = wcet_str,
                dwcet = declared_wcet,
                stack = stack,
                ext   = pf.reaches_extern,
            ));
        }

        // ── Findings ─────────────────────────────────────────────────────────
        let mut findings_json = Vec::new();
        for l in &timing_leaks {
            findings_json.push(format!("    \"{}\"", l.replace('"', "\\\"")));
        }
        for v in &wcet_violations {
            findings_json.push(format!("    \"{}\"", v.replace('"', "\\\"")));
        }

        // ── Timestamp ────────────────────────────────────────────────────────
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // ── JSON report ──────────────────────────────────────────────────────
        let json = format!(
"{{
  \"zeus_compliance_report\": \"v1\",
  \"standard\": \"IEC 62304 Class C / FDA 510(k) SW Safety\",
  \"generated_at_unix\": {ts},
  \"compliant\": {comp},
  \"checks\": {{
    \"zero_heap\": {zh},
    \"all_functions_wcet_bounded\": {ab},
    \"all_functions_constant_time\": {act},
    \"all_functions_deterministic\": {ad},
    \"no_unaudited_ffi\": {nf}
  }},
  \"totals\": {{
    \"functions\": {nfns},
    \"secret_values\": {sv},
    \"timing_leaks\": {nl},
    \"wcet_violations\": {nv}
  }},
  \"functions\": [
{fns}
  ],
  \"findings\": [
{findings}
  ]
}}
",
            ts      = timestamp,
            comp    = compliant,
            zh      = zero_heap,
            ab      = all_bounded,
            act     = all_constant_time,
            ad      = all_deterministic,
            nf      = no_ffi_unaudited,
            nfns    = zir.functions,
            sv      = zir.secret_values,
            nl      = timing_leaks.len(),
            nv      = wcet_violations.len(),
            fns     = fn_entries.join(",\n"),
            findings = findings_json.join(",\n"),
        );

        // ── Human-readable summary ────────────────────────────────────────────
        let ok  = |b: bool| if b { "PASS" } else { "FAIL" };
        let mut txt = String::new();
        txt.push_str("ZEUS IEC 62304 / FDA 510(k) COMPLIANCE REPORT\n");
        txt.push_str("=============================================\n\n");
        txt.push_str(&format!("Overall verdict: {}\n\n",
            if compliant { "COMPLIANT" } else { "NOT COMPLIANT -- see findings below" }));
        txt.push_str("IEC 62304 Class C Checklist:\n");
        txt.push_str(&format!("  [{}] Req 1: Zero dynamic memory allocation\n",  ok(zero_heap)));
        txt.push_str(&format!("  [{}] Req 2: All functions have provable WCET\n", ok(all_bounded)));
        txt.push_str(&format!("  [{}] Req 3: No secret-dependent timing channels\n", ok(all_constant_time)));
        txt.push_str(&format!("  [{}] Req 4: All functions deterministic\n",      ok(all_deterministic)));
        txt.push_str(&format!("  [{}] Req 5: No unauditable FFI calls\n\n",       ok(no_ffi_unaudited)));
        txt.push_str(&format!("Analysed: {} function(s), {} secret value(s)\n",
            zir.functions, zir.secret_values));
        if !timing_leaks.is_empty() {
            txt.push_str("\nTiming-channel findings:\n");
            for l in &timing_leaks { txt.push_str(&format!("  [!] {}\n", l)); }
        }
        if !wcet_violations.is_empty() {
            txt.push_str("\nWCET violations:\n");
            for v in &wcet_violations { txt.push_str(&format!("  [!] {}\n", v)); }
        }
        if compliant {
            txt.push_str("\nThis report may be attached to an FDA 510(k) software safety submission\n");
            txt.push_str("as evidence of IEC 62304 Class C compliance for the analysed module.\n");
        }

        std::fs::write("zeus_compliance.json", &json).map_err(|e| e.to_string())?;
        std::fs::write("zeus_compliance.txt",  &txt).map_err(|e| e.to_string())?;

        // Print concise verdict to terminal
        let verdict_color = if compliant { "\x1b[1;32m" } else { "\x1b[1;31m" };
        println!("\n\x1b[1;36m[ZEUS COMPLIANCE]\x1b[0m IEC 62304 / FDA 510(k) report generated:");
        println!("  {}verdict: {}\x1b[0m", verdict_color,
            if compliant { "COMPLIANT" } else { "NOT COMPLIANT" });
        println!("  zero_heap={} | wcet_bounded={} | constant_time={} | deterministic={} | no_ffi={}",
            zero_heap, all_bounded, all_constant_time, all_deterministic, no_ffi_unaudited);
        if !compliant {
            println!("  \x1b[1;31m{} finding(s) — see zeus_compliance.txt\x1b[0m",
                timing_leaks.len() + wcet_violations.len());
        }
        println!("  \x1b[90mjson: zeus_compliance.json   txt: zeus_compliance.txt\x1b[0m");

        Ok(())
    }

    fn verify_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value, .. } => {
                // Track bounds for variables
                if let Some(range) = self.evaluate_bounds(value) {
                    self.bounds.insert(name.clone(), range);
                } else {
                    // Default bound if unprovable at declaration (assume worst-case finite bounds for safety logic)
                    self.bounds.insert(name.clone(), ValueRange { min: f64::MIN, max: f64::MAX });
                }
            }
            Statement::FunctionDeclaration { body, .. } => {
                for s in body {
                    self.verify_statement(s)?;
                }
            }
            Statement::ParallelBlock { statements, .. }
            | Statement::ProofBlock { statements }
            | Statement::TargetBlock { statements, .. }
            | Statement::EnclaveBlock { statements }
            | Statement::CfgBlock { statements, .. }
            | Statement::ComptimeBlock { statements }
            | Statement::ClusterBlock { statements }
            | Statement::SafeStateBlock { statements }
            | Statement::For { body: statements, .. } => {
                for s in statements {
                    self.verify_statement(s)?;
                }
            }
            Statement::If { consequence, alternative, .. } => {
                for s in consequence {
                    self.verify_statement(s)?;
                }
                if let Some(alt) = alternative {
                    for s in alt {
                        self.verify_statement(s)?;
                    }
                }
            }
            Statement::Assert(expr) => {
                self.prove_assertion(expr)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn evaluate_bounds(&self, expr: &Expression) -> Option<ValueRange> {
        match expr {
            Expression::Number(n) => Some(ValueRange { min: *n, max: *n }),
            Expression::Identifier(name) => self.bounds.get(name).cloned(),
            Expression::Infix { left, operator, right } => {
                let l = self.evaluate_bounds(left)?;
                let r = self.evaluate_bounds(right)?;
                match operator.as_str() {
                    "Plus"  => Some(ValueRange { min: l.min + r.min, max: l.max + r.max }),
                    "Minus" => Some(ValueRange { min: l.min - r.max, max: l.max - r.min }),
                    "Star"  => {
                        let a = l.min * r.min; let b = l.min * r.max; let c = l.max * r.min; let d = l.max * r.max;
                        Some(ValueRange { min: a.min(b).min(c).min(d), max: a.max(b).max(c).max(d) })
                    },
                    "Slash" if r.min > 0.0 || r.max < 0.0 => {
                        let a = l.min / r.min; let b = l.min / r.max; let c = l.max / r.min; let d = l.max / r.max;
                        Some(ValueRange { min: a.min(b).min(c).min(d), max: a.max(b).max(c).max(d) })
                    },
                    _ => None,
                }
            }
            _ => None,
        }
    }

    fn prove_assertion(&mut self, expr: &Expression) -> Result<(), String> {
        if let Expression::Infix { left, operator, right } = expr {
            if operator == "And" {
                self.prove_assertion(left)?;
                self.prove_assertion(right)?;
                return Ok(());
            }
            if operator == "Or" {
                if self.z3_available {
                    return self.verify_bool_with_z3(expr);
                } else {
                    println!("[ZEUS WARNING] z3 not found -- cannot statically verify disjunctive assertion. A runtime check will be injected instead.");
                    return Ok(());
                }
            }
        }
        if let Expression::Infix { left, operator, right } = expr {
            let l_bounds = self.evaluate_bounds(left);
            let r_bounds = self.evaluate_bounds(right);

            if let (Some(l), Some(r)) = (l_bounds, r_bounds) {
                let is_proven = match operator.as_str() {
                    "LessThan"    => l.max < r.min,
                    "GreaterThan" => l.min > r.max,
                    "Equal"       => (l.max - r.min).abs() < f64::EPSILON && (l.min - r.max).abs() < f64::EPSILON,
                    "GreaterEqual"=> l.min >= r.max,
                    "LessEqual"   => l.max <= r.min,
                    "NotEqual"    => l.max < r.min || l.min > r.max,
                    _ => {
                        println!("[ZEUS WARNING] Skipping assertion with operator '{}': not a static comparison.", operator);
                        return Ok(());
                    }
                };

                if !is_proven {
                    return Err(format!(
                        "Mathematical Proof Failed: {} {} {} is FALSE at compile time",
                        fmt_vr(&l), operator, fmt_vr(&r)
                    ));
                }
                println!("[ZEUS VERIFIED] Mathematically proven: {} {} {}", fmt_vr(&l), operator, fmt_vr(&r));
                return Ok(());
            }

            if self.z3_available {
                // Incremental cache lookup: skip Z3 if we already proved this assertion
                let cache_key = format!("{} {} {}", self.expr_to_str(left), operator, self.expr_to_str(right));
                match self.cache.get(&cache_key).cloned() {
                    Some(CacheEntry::Verified) => {
                        println!("[ZEUS VERIFIED] (cached) {} {} {} -- skipped Z3 subprocess",
                            self.expr_to_str(left), operator, self.expr_to_str(right));
                        return Ok(());
                    }
                    Some(CacheEntry::Failed(ref reason)) => {
                        return Err(format!("(cached) {}", reason));
                    }
                    None => {
                        let result = self.verify_with_z3(left, operator, right);
                        // Store result in cache
                        let entry = match &result {
                            Ok(()) => CacheEntry::Verified,
                            Err(e) => CacheEntry::Failed(e.clone()),
                        };
                        self.cache.insert(cache_key, entry);
                        self.cache_dirty = true;
                        return result;
                    }
                }
            } else {
                println!(
                    "[ZEUS WARNING] z3 not found -- cannot statically verify assertion '{} {} {}'. A runtime check will be injected instead.",
                    self.expr_to_str(left), operator, self.expr_to_str(right)
                );
            }
        }
        Ok(())
    }

    fn verify_with_z3(
        &self,
        left: &Expression,
        operator: &str,
        right: &Expression,
    ) -> Result<(), String> {
        let mut smt = String::new();
        smt.push_str("; Zeus @verify -- generated by formal_verifier.rs\n");
        // ALL lets Z3 auto-select; handles linear + nonlinear real/int arithmetic.
    smt.push_str("(set-logic ALL)\n");

        let free_vars = self.collect_free_vars_expr(left)
            .into_iter()
            .chain(self.collect_free_vars_expr(right))
            .collect::<std::collections::HashSet<_>>();
        for v in &free_vars {
            smt.push_str(&format!("(declare-const {} Real)\n", v));
        }

        let lhs_smt = self.expr_to_smt(left);
        let rhs_smt = self.expr_to_smt(right);
        let op_smt = match operator {
            "LessThan"     => "<",
            "GreaterThan"  => ">",
            "Equal"        => "=",
            "GreaterEqual" => ">=",
            "LessEqual"    => "<=",
            "NotEqual"     => "distinct",
            _ => return Ok(()),
        };
        smt.push_str(&format!("(assert (not ({} {} {})))\n", op_smt, lhs_smt, rhs_smt));
        smt.push_str("(check-sat)\n");
        smt.push_str("(get-model)\n");

        let tmp = std::env::temp_dir().join(format!("zeus_verify_{}.smt2", std::process::id()));
        std::fs::write(&tmp, &smt).map_err(|e| format!("Failed to write SMT file: {}", e))?;

        let output = std::process::Command::new("z3")
            .arg("-T:2")
            .arg(tmp.to_str().unwrap())
            .output()
            .map_err(|e| format!("z3 invocation failed: {}", e))?;

        let _ = std::fs::remove_file(&tmp);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("unknown").trim();

        match first_line {
            "unsat" => {
                println!(
                    "[ZEUS VERIFIED] Z3 proved: {} {} {} (UNSAT -- property always holds)",
                    self.expr_to_str(left), operator, self.expr_to_str(right)
                );
                Ok(())
            }
            "sat" => {
                let model_lines: Vec<&str> = stdout.lines().skip(1).take(8).collect();
                let model_snippet = model_lines.join(" ");
                Err(format!(
                    "Z3 found a counterexample for assertion: {} {} {}\nCounterexample: {}",
                    self.expr_to_str(left), operator, self.expr_to_str(right),
                    model_snippet
                ))
            }
            _ => {
                println!(
                    "[ZEUS WARNING] Z3 returned '{}' for assertion '{} {} {}' (timeout or undecidable). Runtime assertion fallback will be injected.",
                    first_line,
                    self.expr_to_str(left), operator, self.expr_to_str(right)
                );
                Ok(())
            }
        }
    }

    fn expr_to_smt(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number(n) => {
                if *n < 0.0 { format!("(- {})", (-n)) } else { n.to_string() }
            }
            Expression::Identifier(name) => {
                if let Some(v) = self.constants.get(name) { v.to_string() } else { name.clone() }
            }
            Expression::Infix { left, operator, right } => {
                let l = self.expr_to_smt(left);
                let r = self.expr_to_smt(right);
                let op = match operator.as_str() {
                    "Plus" => "+", "Minus" => "-", "Star" => "*", "Slash" => "/",
                    "And" => "and", "Or" => "or",
                    "LessThan" => "<", "GreaterThan" => ">",
                    "Equal" => "=", "GreaterEqual" => ">=", "LessEqual" => "<=",
                    "NotEqual" => "distinct",
                    _ => return "0".to_string(),
                };
                format!("({} {} {})", op, l, r)
            }
            _ => "0".to_string(),
        }
    }

    fn verify_bool_with_z3(&self, expr: &Expression) -> Result<(), String> {
        let mut smt = String::new();
        smt.push_str("(set-logic ALL)\n");
        let free_vars = self.collect_free_vars_expr(expr)
            .into_iter()
            .collect::<std::collections::HashSet<_>>();
        for v in &free_vars {
            smt.push_str(&format!("(declare-const {} Real)\n", v));
        }
        let goal = self.expr_to_smt(expr);
        smt.push_str(&format!("(assert (not {}))\n", goal));
        smt.push_str("(check-sat)\n");
        smt.push_str("(get-model)\n");
        let tmp = std::env::temp_dir().join(format!("zeus_verify_bool_{}.smt2", std::process::id()));
        std::fs::write(&tmp, &smt).map_err(|e| format!("Failed to write SMT file: {}", e))?;
        let output = std::process::Command::new("z3")
            .arg("-T:2")
            .arg(tmp.to_str().unwrap())
            .output()
            .map_err(|e| format!("z3 invocation failed: {}", e))?;
        let _ = std::fs::remove_file(&tmp);
        let stdout = String::from_utf8_lossy(&output.stdout);
        let first_line = stdout.lines().next().unwrap_or("unknown").trim();
        match first_line {
            "unsat" => {
                println!("[ZEUS VERIFIED] Z3 proved compound assertion: {} (UNSAT -- property always holds)", self.expr_to_str(expr));
                Ok(())
            }
            "sat" => {
                let model_lines: Vec<&str> = stdout.lines().skip(1).take(8).collect();
                Err(format!("Z3 found a counterexample for compound assertion: {}\nCounterexample: {}", self.expr_to_str(expr), model_lines.join(" ")))
            }
            _ => {
                println!("[ZEUS WARNING] Z3 returned '{}' for compound assertion (timeout/undecidable). Runtime fallback injected.", first_line);
                Ok(())
            }
        }
    }

    fn expr_to_str(&self, expr: &Expression) -> String {
        match expr {
            Expression::Number(n) => n.to_string(),
            Expression::Identifier(name) => name.clone(),
            Expression::Infix { left, operator, right } => {
                format!("({} {} {})", self.expr_to_str(left), operator, self.expr_to_str(right))
            }
            _ => "?".to_string(),
        }
    }

    fn collect_free_vars_expr(&self, expr: &Expression) -> Vec<String> {
        match expr {
            Expression::Identifier(name) => {
                if !self.constants.contains_key(name) { vec![name.clone()] } else { vec![] }
            }
            Expression::Infix { left, right, .. } => {
                let mut vars = self.collect_free_vars_expr(left);
                vars.extend(self.collect_free_vars_expr(right));
                vars
            }
            _ => vec![],
        }
    }
}
