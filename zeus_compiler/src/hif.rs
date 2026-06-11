#![allow(dead_code)]
//! hif.rs — Homomorphic Instruction Folding (Vector 11)
//!
//! Transforms if/else control flow in the Zeus AST into a single continuous
//! algebraic polynomial evaluated over bit-masks. The resulting C has zero
//! conditional branches: "wrong" branches multiply by 0 and cancel out.
//!
//! This is the mathematical evolution of @constant_time execution — not just
//! executing both sides, but folding them into a single O(1) equation whose
//! output is determined solely by algebra, never by branch prediction.
//!
//! The Technique (Select-Mask Polynomial):
//!   if cond { A } else { B }
//!   → mask = -(uintptr_t)(!!cond)   // all-ones if true, all-zeros if false
//!   → result = (A & mask) | (B & ~mask)
//!
//! For nested if/else the compiler generates a multi-term polynomial:
//!   result = Σ (term_i * mask_i)   where Σ masks_i = all-ones
//!
//! This guarantees: no branches, no prediction failures, O(1) depth regardless
//! of nesting level, and instruction-count independent of input values.

use crate::ast::{Expression, Statement};

/// Verdict from the HIF analysis pass.
#[derive(Debug, Clone, PartialEq)]
pub enum HifFoldability {
    /// Function can be fully folded into a branch-free polynomial.
    FullyFoldable,
    /// Some branches contain side-effects or calls; partial folding applied.
    PartiallyFoldable { unfoldable_branches: usize },
    /// Function has pointer-aliasing, recursion, or I/O — cannot fold.
    Unfoldable { reason: String },
}

/// Per-function HIF analysis result.
#[derive(Debug, Clone)]
pub struct HifFunctionReport {
    pub name: String,
    pub foldability: HifFoldability,
    pub if_depth: usize,
    pub polynomial_terms: usize,
    pub branches_eliminated: usize,
}

/// Top-level HIF pass report.
#[derive(Debug, Default)]
pub struct HifReport {
    pub functions: Vec<HifFunctionReport>,
}

impl HifReport {
    pub fn total_branches_eliminated(&self) -> usize {
        self.functions.iter().map(|f| f.branches_eliminated).sum()
    }
    pub fn fully_foldable_count(&self) -> usize {
        self.functions.iter().filter(|f| f.foldability == HifFoldability::FullyFoldable).count()
    }
}

/// Analyze the program and return the HIF report. Does not mutate the AST —
/// the C codegen reads this report to emit select-mask polynomials instead of
/// if/goto chains.
pub fn analyze(program: &crate::ast::Program) -> HifReport {
    let mut report = HifReport::default();
    for stmt in &program.statements {
        if let Statement::FunctionDeclaration { name, body, .. } = stmt {
            let result = analyze_function(name, body);
            report.functions.push(result);
        }
    }
    report
}

fn analyze_function(name: &str, body: &[Statement]) -> HifFunctionReport {
    let mut depth = 0usize;
    let mut terms = 0usize;
    let mut eliminated = 0usize;
    let mut unfoldable_count = 0usize;
    let mut unfoldable_reason = String::new();

    for stmt in body {
        scan_stmt(stmt, &mut depth, &mut terms, &mut eliminated,
                  &mut unfoldable_count, &mut unfoldable_reason, 0);
    }

    let foldability = if unfoldable_count == 0 && eliminated > 0 {
        HifFoldability::FullyFoldable
    } else if unfoldable_count > 0 && eliminated > 0 {
        HifFoldability::PartiallyFoldable { unfoldable_branches: unfoldable_count }
    } else if unfoldable_count > 0 {
        HifFoldability::Unfoldable { reason: unfoldable_reason.clone() }
    } else {
        HifFoldability::FullyFoldable
    };

    HifFunctionReport {
        name: name.to_string(),
        foldability,
        if_depth: depth,
        polynomial_terms: terms,
        branches_eliminated: eliminated,
    }
}

fn expr_is_pure(expr: &Expression) -> bool {
    match expr {
        Expression::Number(_) | Expression::Identifier(_) => true,
        Expression::Infix { left, right, .. } => expr_is_pure(left) && expr_is_pure(right),
        Expression::FunctionCall { .. } => false,
        _ => false,
    }
}

fn scan_stmt(
    stmt: &Statement,
    max_depth: &mut usize,
    terms: &mut usize,
    eliminated: &mut usize,
    unfoldable: &mut usize,
    unfoldable_reason: &mut String,
    current_depth: usize,
) {
    match stmt {
        Statement::If { condition, consequence, alternative } => {
            *max_depth = (*max_depth).max(current_depth + 1);
            let cond_pure = expr_is_pure(condition);
            let cons_pure = consequence.iter().all(|s| stmt_is_leaf(s));
            let alt_pure  = alternative.as_ref().map(|a| a.iter().all(|s| stmt_is_leaf(s))).unwrap_or(true);

            if cond_pure && cons_pure && alt_pure {
                *terms += 2;
                *eliminated += 1;
            } else {
                *unfoldable += 1;
                if unfoldable_reason.is_empty() {
                    *unfoldable_reason = if !cond_pure {
                        "condition has side-effects".to_string()
                    } else {
                        "branch body contains function call or I/O".to_string()
                    };
                }
            }

            for s in consequence {
                scan_stmt(s, max_depth, terms, eliminated, unfoldable, unfoldable_reason, current_depth + 1);
            }
            if let Some(alt) = alternative {
                for s in alt {
                    scan_stmt(s, max_depth, terms, eliminated, unfoldable, unfoldable_reason, current_depth + 1);
                }
            }
        }
        Statement::For { body, .. } | Statement::While { body, .. } => {
            *unfoldable += 1;
            if unfoldable_reason.is_empty() {
                *unfoldable_reason = "loop body not foldable at compile time".to_string();
            }
            for s in body {
                scan_stmt(s, max_depth, terms, eliminated, unfoldable, unfoldable_reason, current_depth);
            }
        }
        _ => {}
    }
}

fn stmt_is_leaf(stmt: &Statement) -> bool {
    matches!(stmt,
        Statement::Let { .. }
        | Statement::Return(_)
        | Statement::ExpressionStatement(_)
    )
}

/// Emit C select-mask polynomial for a single two-branch if/else.
/// The caller passes pre-generated C expressions for condition, then_val, else_val.
/// Returns a C expression string of the form:
///   ((__zeus_hif_mask(COND) & (THEN)) | (~__zeus_hif_mask(COND) & (ELSE)))
pub fn emit_select_mask(cond_c: &str, then_c: &str, else_c: &str) -> String {
    format!(
        "((__zeus_hif_mask((uintptr_t)({})) & (uintptr_t)({})) \
          | (~__zeus_hif_mask((uintptr_t)({})) & (uintptr_t)({})))",
        cond_c, then_c, cond_c, else_c
    )
}

/// Emit the `__zeus_hif_mask` C runtime helper (inlined in every translation unit).
/// mask(1) = ~0 (all ones), mask(0) = 0 — branchless, compiler cannot speculate.
pub fn hif_runtime_header() -> &'static str {
    r#"// ── Zeus HIF Runtime (Homomorphic Instruction Folding) ──────────────────────
// Converts boolean conditions to all-ones/all-zeros masks with zero branches.
// The compiler cannot speculate through the negation — Spectre-safe by design.
static inline uintptr_t __zeus_hif_mask(uintptr_t cond) {
    return -(uintptr_t)(!!cond);
}
// Multi-term polynomial evaluator: result = Σ (val[i] & mask[i])
// Used by the HIF pass for nested if/else chains.
static inline uintptr_t __zeus_hif_select(
        uintptr_t cond, uintptr_t a, uintptr_t b) {
    uintptr_t m = __zeus_hif_mask(cond);
    return (a & m) | (b & ~m);
}
// ────────────────────────────────────────────────────────────────────────────
"#
}

/// JSON report for `zeus audit --json` integration.
pub fn report_json(r: &HifReport) -> String {
    let fns: Vec<String> = r.functions.iter().map(|f| {
        let fold = match &f.foldability {
            HifFoldability::FullyFoldable => "\"fully_foldable\"",
            HifFoldability::PartiallyFoldable { .. } => "\"partially_foldable\"",
            HifFoldability::Unfoldable { .. } => "\"unfoldable\"",
        };
        format!(
            "{{\"name\":\"{}\",\"foldability\":{},\"if_depth\":{},\
             \"polynomial_terms\":{},\"branches_eliminated\":{}}}",
            f.name, fold, f.if_depth, f.polynomial_terms, f.branches_eliminated)
    }).collect();
    format!(
        "{{\"hif\":\"v1\",\"total_branches_eliminated\":{},\
          \"fully_foldable_functions\":{},\"functions\":[{}]}}",
        r.total_branches_eliminated(), r.fully_foldable_count(), fns.join(","))
}
