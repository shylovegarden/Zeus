use crate::ast::*;

/// A middle-end compiler pass that intercepts memory access nodes in the AST.
/// Transforms a simple `read` or `write` (IndexAccess) into a randomized Path ORAM tree access
/// (OramAccess) to disguise the true data access pattern from hardware side-channel attacks.
pub fn flatten_memory_accesses(program: &mut Program) {
    for stmt in &mut program.statements {
        transform_statement(stmt);
    }
}

fn transform_statement(stmt: &mut Statement) {
    match stmt {
        Statement::Let { value, .. } => transform_expression(value),
        Statement::ExpressionStatement(expr) => transform_expression(expr),
        Statement::If { condition, consequence, alternative } => {
            transform_expression(condition);
            for s in consequence {
                transform_statement(s);
            }
            if let Some(alt) = alternative {
                for s in alt {
                    transform_statement(s);
                }
            }
        }
        Statement::For { start, end, body, .. } => {
            transform_expression(start);
            transform_expression(end);
            for s in body {
                transform_statement(s);
            }
        }
        Statement::ParallelBlock { start, end, statements, .. } => {
            transform_expression(start);
            transform_expression(end);
            for s in statements {
                transform_statement(s);
            }
        }
        Statement::FunctionDeclaration { body, .. } => {
            for s in body {
                transform_statement(s);
            }
        }
        Statement::TestDeclaration { body, .. } => {
            for s in body {
                transform_statement(s);
            }
        }
        Statement::Return(expr) | Statement::Assert(expr) => {
            transform_expression(expr);
        }
        Statement::TargetBlock { statements, .. } | Statement::ProofBlock { statements, .. }
        | Statement::SafeStateBlock { statements, .. } | Statement::EnclaveBlock { statements, .. }
        | Statement::CfgBlock { statements, .. } | Statement::ComptimeBlock { statements, .. }
        | Statement::ClusterBlock { statements, .. } => {
            for s in statements {
                transform_statement(s);
            }
        }
        Statement::StructDeclaration { .. }
        | Statement::ExternFunctionDeclaration { .. }
        | Statement::Import(_)
        | Statement::Panic(_)
        | Statement::LineDirective(_)
        | Statement::AtomicAdd { .. } => {}
    }
}

fn transform_expression(expr: &mut Expression) {
    let mut replace_with_oram = false;

    // First recurse into inner expressions
    match expr {
        Expression::IndexAccess { base, index } => {
            transform_expression(base);
            transform_expression(index);
            replace_with_oram = true;
        }
        Expression::Infix { left, right, .. } => {
            transform_expression(left);
            transform_expression(right);
        }
        Expression::FunctionCall { arguments, .. } => {
            for arg in arguments {
                transform_expression(arg);
            }
        }
        Expression::StructInit { fields, .. } => {
            for (_, val) in fields {
                transform_expression(val);
            }
        }
        Expression::FieldAccess { base, .. } => {
            transform_expression(base);
        }
        Expression::Try(inner) | Expression::Comptime(inner) => {
            transform_expression(inner);
        }
        Expression::TensorDefinition { dimensions } => {
            for dim in dimensions {
                transform_expression(dim);
            }
        }
        Expression::OramAccess { base, index } => {
            transform_expression(base);
            transform_expression(index);
        }
        Expression::NvmeDmaMap { path, size } => {
            transform_expression(path);
            transform_expression(size);
        }
        Expression::Identifier(_) | Expression::Number(_) | Expression::StringLiteral(_) => {}
    }

    // Now if it is an IndexAccess, rewrite it to OramAccess
    if replace_with_oram {
        if let Expression::IndexAccess { base, index } = expr.clone() {
            *expr = Expression::OramAccess { base, index };
        }
    }
}
