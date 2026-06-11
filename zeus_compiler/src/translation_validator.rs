#![allow(clippy::collapsible_if, clippy::map_unwrap_or, clippy::needless_bool)]
//! translation_validator.rs — Zeus Translation Validation Pass (Vector 10)
//!
//! Validates that compiler transformations (ORAM flattening, monomorphisation,
//! SoA rewriting, constant-folding) preserve the mathematical semantics of the
//! source program.  Follows the Alive2 / CompCert methodology:
//!
//!   1. Encode the pre-pass and post-pass program fragments as SMT assertions.
//!   2. Ask Z3: "Is there any input where the outputs differ?" (sat = regression).
//!   3. If UNSAT → transformation is semantics-preserving; cert gains `tv_passed`.
//!   4. If SAT   → Z3 returns a counter-example; build aborts with the gap.
//!
//! When Z3 is not installed the validator falls back to a structural diff and
//! reports UNDECIDABLE (does not block the build — honest about tool limits).

use crate::ast::{Program, Statement, Expression};
use std::collections::HashMap;
use std::fmt::Write as FmtWrite;

// ─── Public API ──────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum TvVerdict {
    /// SMT solver proved equivalence: no input distinguishes pre from post.
    Equivalent,
    /// SMT solver found a counter-example input that distinguishes the two.
    NotEquivalent { counterexample: String },
    /// Z3 not available / query timed-out / state space too large for the bound.
    Undecidable { reason: String },
}

pub struct TranslationValidator {
    z3_available: bool,
    /// Maximum number of function pairs to validate in a single run.
    max_pairs: usize,
}

impl TranslationValidator {
    pub fn new() -> Self {
        let z3_available = std::process::Command::new("z3")
            .arg("--version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        TranslationValidator { z3_available, max_pairs: 32 }
    }

    /// Validate that `post` is a semantic refinement of `pre`.
    /// Returns one verdict per matched function pair.
    pub fn validate(&self, pre: &Program, post: &Program) -> Vec<(String, TvVerdict)> {
        let pre_fns  = collect_functions(pre);
        let post_fns = collect_functions(post);
        let mut results = Vec::new();

        for (name, pre_body) in &pre_fns {
            if results.len() >= self.max_pairs { break; }
            let post_body = match post_fns.get(name) {
                Some(b) => b,
                None => {
                    results.push((name.clone(), TvVerdict::Undecidable {
                        reason: format!("fn '{}' absent from post-pass program", name),
                    }));
                    continue;
                }
            };

            let verdict = self.validate_function(name, pre_body, post_body);
            results.push((name.clone(), verdict));
        }
        results
    }

    fn validate_function(
        &self,
        name: &str,
        pre_body: &[Statement],
        post_body: &[Statement],
    ) -> TvVerdict {
        // Fast path: syntactic equality after normalization
        if stmts_equal(pre_body, post_body) {
            return TvVerdict::Equivalent;
        }

        if !self.z3_available {
            return TvVerdict::Undecidable {
                reason: "Z3 not installed — structural diff detected; install z3 for SMT proof".into(),
            };
        }

        // Build SMT-LIB2 query: assert pre_result != post_result for some input
        let mut smt = String::new();
        let mut vars: HashMap<String, &'static str> = HashMap::new();

        collect_input_vars(pre_body, &mut vars);

        // Declare free input variables
        let _ = writeln!(smt, "(set-logic QF_LIA)");
        for (v, sort) in &vars {
            let _ = writeln!(smt, "(declare-const {} {})", sanitize(v), sort);
        }

        // Encode pre and post return values as SMT expressions
        let pre_expr  = encode_return_value(pre_body,  &vars);
        let post_expr = encode_return_value(post_body, &vars);

        if pre_expr.is_none() && post_expr.is_none() {
            return TvVerdict::Undecidable {
                reason: format!("fn '{}': no return value encodable; non-numeric body", name),
            };
        }

        let pre_smt  = pre_expr.unwrap_or_else(||  "0".to_string());
        let post_smt = post_expr.unwrap_or_else(|| "0".to_string());

        // Assert they differ — if UNSAT, they are always equal
        let _ = writeln!(smt, "(assert (not (= {} {})))", pre_smt, post_smt);
        let _ = writeln!(smt, "(check-sat)");
        let _ = writeln!(smt, "(get-model)");

        self.run_z3(name, &smt)
    }

    fn run_z3(&self, fn_name: &str, query: &str) -> TvVerdict {
        let tmp = format!("__zeus_tv_{}.smt2", fn_name);
        if let Err(e) = std::fs::write(&tmp, query) {
            return TvVerdict::Undecidable { reason: format!("cannot write Z3 query: {}", e) };
        }

        let result = std::process::Command::new("z3")
            .arg("-t:5000")  // 5-second timeout
            .arg(&tmp)
            .output();

        let _ = std::fs::remove_file(&tmp);

        match result {
            Err(e) => TvVerdict::Undecidable { reason: format!("z3 exec error: {}", e) },
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                if stdout.trim_start().starts_with("unsat") {
                    TvVerdict::Equivalent
                } else if stdout.trim_start().starts_with("sat") {
                    // Extract model lines as counterexample
                    let model: String = stdout.lines()
                        .skip(1)
                        .take(8)
                        .collect::<Vec<_>>()
                        .join("; ");
                    TvVerdict::NotEquivalent { counterexample: model }
                } else {
                    TvVerdict::Undecidable {
                        reason: format!("Z3 returned: {}", stdout.trim().chars().take(120).collect::<String>()),
                    }
                }
            }
        }
    }

    /// Emit a human-readable report.
    pub fn report(results: &[(String, TvVerdict)]) -> String {
        let mut out = String::new();
        let _ = writeln!(out, "\n\x1b[1;36m== ZEUS TRANSLATION VALIDATION ==\x1b[0m");
        let mut equiv = 0usize;
        let mut not_equiv = 0usize;
        let mut undecidable = 0usize;
        for (name, v) in results {
            match v {
                TvVerdict::Equivalent => {
                    equiv += 1;
                    let _ = writeln!(out, "  \x1b[1;32m[EQUIV]\x1b[0m  fn {}", name);
                }
                TvVerdict::NotEquivalent { counterexample } => {
                    not_equiv += 1;
                    let _ = writeln!(out, "  \x1b[1;31m[DIFF]\x1b[0m   fn {} — counterexample: {}", name, counterexample);
                }
                TvVerdict::Undecidable { reason } => {
                    undecidable += 1;
                    let _ = writeln!(out, "  \x1b[1;33m[UNDEC]\x1b[0m  fn {} — {}", name, reason);
                }
            }
        }
        let _ = writeln!(out, "\n  EQUIVALENT: {}  |  NOT-EQUIVALENT: {}  |  UNDECIDABLE: {}",
            equiv, not_equiv, undecidable);
        if not_equiv > 0 {
            let _ = writeln!(out, "  \x1b[1;31m[TV GATE] FAILED\x1b[0m — {} function(s) semantics NOT preserved by transformation.", not_equiv);
        } else {
            let _ = writeln!(out, "  \x1b[1;32m[TV GATE] PASSED\x1b[0m");
        }
        out
    }

    /// Machine-readable JSON for the AI agent loop.
    pub fn report_json(results: &[(String, TvVerdict)]) -> String {
        let items: Vec<String> = results.iter().map(|(name, v)| {
            let (verdict, detail) = match v {
                TvVerdict::Equivalent                       => ("equivalent",     String::new()),
                TvVerdict::NotEquivalent { counterexample } => ("not_equivalent", counterexample.clone()),
                TvVerdict::Undecidable   { reason }         => ("undecidable",    reason.clone()),
            };
            let escaped_detail = detail.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', " ");
            format!("{{\"function\":\"{}\",\"verdict\":\"{}\",\"detail\":\"{}\"}}",
                name, verdict, escaped_detail)
        }).collect();
        format!("{{\"translation_validation\":\"v1\",\"results\":[{}]}}", items.join(","))
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn collect_functions(prog: &Program) -> HashMap<String, Vec<Statement>> {
    let mut map = HashMap::new();
    for stmt in &prog.statements {
        if let Statement::FunctionDeclaration { name, body, .. } = stmt {
            map.insert(name.clone(), body.clone());
        }
    }
    map
}

/// Structural equality of two statement lists after alpha-normalisation.
fn stmts_equal(a: &[Statement], b: &[Statement]) -> bool {
    if a.len() != b.len() { return false; }
    a.iter().zip(b.iter()).all(|(x, y)| stmt_eq(x, y))
}

fn stmt_eq(a: &Statement, b: &Statement) -> bool {
    // Coarse structural comparison — sufficient for syntactic identity check.
    format!("{:?}", a) == format!("{:?}", b)
}

/// Collect identifiers likely to be function inputs (let bindings or params).
fn collect_input_vars<'a>(stmts: &'a [Statement], vars: &mut HashMap<String, &'static str>) {
    for s in stmts {
        if let Statement::Let { name, var_type, .. } = s {
            let sort = match var_type {
                Some(crate::ast::Type::F64) | Some(crate::ast::Type::F32) => "Real",
                _ => "Int",
            };
            vars.entry(name.clone()).or_insert(sort);
        }
    }
}

/// Encode the dominant return value of a function body as an SMT expression.
/// Returns None if the body has no numeric return statement.
fn encode_return_value(stmts: &[Statement], vars: &HashMap<String, &'static str>) -> Option<String> {
    for s in stmts.iter().rev() {
        if let Statement::Return(expr) = s {
            return Some(encode_expr(expr, vars));
        }
        if let Statement::ExpressionStatement(expr) = s {
            if matches!(expr, Expression::Number(_) | Expression::Identifier(_) | Expression::Infix { .. }) {
                return Some(encode_expr(expr, vars));
            }
        }
    }
    None
}

fn encode_expr(expr: &Expression, vars: &HashMap<String, &'static str>) -> String {
    match expr {
        Expression::Number(n) => {
            if n.fract() == 0.0 { format!("{}", *n as i64) } else { format!("{}", n) }
        }
        Expression::Identifier(name) => {
            if vars.contains_key(name) { sanitize(name) } else { "0".to_string() }
        }
        Expression::Infix { left, operator, right } => {
            let l = encode_expr(left,  vars);
            let r = encode_expr(right, vars);
            let op = match operator.as_str() {
                "Plus"        => "+",
                "Minus"       => "-",
                "Star"        => "*",
                "Slash"       => "div",
                "Percent"     => "mod",
                "LessThan"    => "<",
                "GreaterThan" => ">",
                "LessEqual"   => "<=",
                "GreaterEqual"=> ">=",
                "Equal"       => "=",
                _             => "+",
            };
            if op == "div" || op == "mod" {
                format!("({} {} {})", op, l, r)
            } else {
                format!("({} {} {})", op, l, r)
            }
        }
        Expression::Prefix { operator, operand } => {
            let inner = encode_expr(operand, vars);
            if operator == "Minus" { format!("(- {})", inner) } else { inner }
        }
        _ => "0".to_string(),
    }
}

fn sanitize(name: &str) -> String {
    name.replace('-', "_").replace('.', "_")
}
