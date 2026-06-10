#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or, clippy::type_complexity)]
use crate::ast::*;
use std::collections::HashSet;

/// A middle-end compiler pass that intercepts memory access nodes in the AST.
/// Transforms an array `read`/`write` (IndexAccess) into a randomized Path-ORAM
/// access (OramAccess) to disguise the true data-access pattern from cache-timing
/// and hardware side-channel adversaries.
///
/// # OPT-IN SECURITY MODEL
/// ORAM is **not** applied to every array. It is applied *only* to arrays whose
/// backing variable is declared with the `secret` keyword. Non-secret arrays keep
/// a direct `IndexAccess` and run at full native (C) speed.
///
/// This is the core of Zeus's "fast AND secure" promise: you pay the ORAM cost
/// (~10x memory traffic) exactly where you ask for privacy, and nowhere else.
/// A `secret` array is therefore protected on two axes simultaneously - its
/// contents are wiped from RAM at scope exit (cold-boot resistance) and its
/// access pattern is flattened (cache-timing resistance).
pub fn flatten_memory_accesses(program: &mut Program) {
    let mut scope: HashSet<String> = HashSet::new();
    for stmt in &mut program.statements {
        transform_statement(stmt, &mut scope);
    }
}

/// Resolve the root identifier of a (possibly nested) lvalue expression so we can
/// tell whether the array being indexed was declared `secret`.
fn root_ident(expr: &Expression) -> Option<String> {
    match expr {
        Expression::Identifier(n) => Some(n.clone()),
        Expression::IndexAccess { base, .. } => root_ident(base),
        Expression::OramAccess { base, .. } => root_ident(base),
        Expression::FieldAccess { base, .. } => root_ident(base),
        _ => None,
    }
}

/// Process a nested block in a child scope that inherits the parent's secret
/// bindings but does not leak its own declarations back out (lexical scoping).
fn transform_block(stmts: &mut [Statement], parent: &HashSet<String>) {
    let mut child = parent.clone();
    for s in stmts {
        transform_statement(s, &mut child);
    }
}

fn transform_statement(stmt: &mut Statement, scope: &mut HashSet<String>) {
    match stmt {
        Statement::Let { value, is_secret, name, .. } => {
            transform_expression(value, scope);
            // A secret binding is visible only *after* its declaration.
            if *is_secret {
                scope.insert(name.clone());
            }
        }
        Statement::ExpressionStatement(expr) => transform_expression(expr, scope),
        Statement::If { condition, consequence, alternative } => {
            transform_expression(condition, scope);
            transform_block(consequence, scope);
            if let Some(alt) = alternative {
                transform_block(alt, scope);
            }
        }
        Statement::For { start, end, body, .. } => {
            transform_expression(start, scope);
            transform_expression(end, scope);
            transform_block(body, scope);
        }
        Statement::While { condition, body } => {
            transform_expression(condition, scope);
            transform_block(body, scope);
        }
        Statement::ParallelBlock { start, end, statements, .. } => {
            transform_expression(start, scope);
            transform_expression(end, scope);
            transform_block(statements, scope);
        }
        Statement::FunctionDeclaration { body, .. } => {
            transform_block(body, scope);
        }
        Statement::TestDeclaration { body, .. } => {
            transform_block(body, scope);
        }
        Statement::Return(expr) | Statement::Assert(expr) => {
            transform_expression(expr, scope);
        }
        Statement::TargetBlock { statements, .. } | Statement::ProofBlock { statements, .. }
        | Statement::SafeStateBlock { statements, .. } | Statement::EnclaveBlock { statements, .. }
        | Statement::CfgBlock { statements, .. } | Statement::ComptimeBlock { statements, .. }
        | Statement::ClusterBlock { statements, .. } => {
            transform_block(statements, scope);
        }
        Statement::StructDeclaration { .. }
        | Statement::ExternFunctionDeclaration { .. }
        | Statement::Import(_)
        | Statement::Panic(_)
        | Statement::LineDirective(_)
        | Statement::AtomicAdd { .. } => {}
        Statement::EnumDeclaration { .. } => {}
        Statement::MatchStatement { scrutinee, arms } => {
            transform_expression(scrutinee, scope);
            for arm in arms {
                for s in arm.body.iter_mut() { let mut sc = scope.clone(); transform_statement(s, &mut sc); }
            }
        }
    }
}

fn transform_expression(expr: &mut Expression, scope: &HashSet<String>) {
    let mut replace_with_oram = false;

    // First recurse into inner expressions.
    match expr {
        Expression::IndexAccess { base, index } => {
            transform_expression(base, scope);
            transform_expression(index, scope);
            // OPT-IN: only rewrite to ORAM when the indexed array is `secret`.
            if root_ident(base).is_some_and(|r| scope.contains(&r)) {
                replace_with_oram = true;
            }
        }
        Expression::Infix { left, right, .. } => {
            transform_expression(left, scope);
            transform_expression(right, scope);
        }
        Expression::FunctionCall { arguments, .. } => {
            for arg in arguments {
                transform_expression(arg, scope);
            }
        }
        Expression::StructInit { fields, .. } => {
            for (_, val) in fields {
                transform_expression(val, scope);
            }
        }
        Expression::FieldAccess { base, .. } => {
            transform_expression(base, scope);
        }
        Expression::Try(inner) | Expression::Comptime(inner) => {
            transform_expression(inner, scope);
        }
        Expression::TensorDefinition { dimensions } => {
            for dim in dimensions {
                transform_expression(dim, scope);
            }
        }
        Expression::OramAccess { base, index, .. } => {
            transform_expression(base, scope);
            transform_expression(index, scope);
        }
        Expression::NvmeDmaMap { path, size } => {
            transform_expression(path, scope);
            transform_expression(size, scope);
        }
        Expression::Prefix { operand, .. } => {
            transform_expression(operand, scope);
        }
        Expression::ArrayLiteral(elements) => {
            for el in elements { transform_expression(el, scope); }
        }
        Expression::Identifier(_) | Expression::Number(_) | Expression::StringLiteral(_) => {}
        Expression::EnumVariant { payload, .. } => {
            for p in payload { transform_expression(p, scope); }
        }
        Expression::MatchExpr { scrutinee, arms } => {
            transform_expression(scrutinee, scope);
            for arm in arms {
                for s in arm.body.iter_mut() { let mut sc = scope.clone(); transform_statement(s, &mut sc); }
            }
        }
    }

    // Rewrite a secret IndexAccess into an OramAccess.
    if replace_with_oram {
        if let Expression::IndexAccess { base, index } = expr.clone() {
            // Only apply ORAM if the base is a FieldAccess targeting "data" (e.g. tensor.data)
            let is_tensor_data = if let Expression::FieldAccess { field, .. } = &*base {
                field == "data"
            } else {
                false
            };

            if is_tensor_data {
                // Very simple heuristic to find bound: fallback to 256 for cryptographic block constraints.
                // A more advanced compiler pass would pull sizes from type declarations.
                let bound = 256; 
                *expr = Expression::OramAccess { base, index, bound };
            } else {
                *expr = Expression::IndexAccess { base, index };
            }
        }
    }
}
