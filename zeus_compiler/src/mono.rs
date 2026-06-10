/// Monomorphization pass for Zeus generics.
///
/// Walks the program AST, finds every call-site that invokes a generic
/// function / instantiates a generic struct with concrete type arguments,
/// and emits a specialised clone with type-parameters substituted.
///
/// Example:
///   Generic definition: `fn identity<T>(x: T) -> T`
///   Call site:          `identity::<f64>(3.14)`  or  `identity(3.14)`
///   Emitted clone:      `fn identity__f64(x: f64) -> f64`
///
/// The pass is non-destructive: generic originals are kept but skipped
/// by codegen (they still contain TypeParam nodes, which `type_to_c`
/// maps to `double` as a fallback, ensuring the file still compiles even
/// when the pass is incomplete).

use std::collections::HashMap;
use crate::ast::{Expression, Program, Statement, Type};

// ── Public entry point ───────────────────────────────────────────────────────

/// Run monomorphization on `program` in-place.  After this pass every
/// call-site that resolves to a generic function will have a concrete
/// specialisation emitted as a new top-level `FunctionDeclaration`.
pub fn monomorphize(program: &mut Program) {
    // 1. Collect all generic function definitions indexed by name.
    let generic_fns: HashMap<String, Statement> = program.statements.iter()
        .filter_map(|s| {
            if let Statement::FunctionDeclaration { name, type_params, .. } = s {
                if !type_params.is_empty() {
                    return Some((name.clone(), s.clone()));
                }
            }
            None
        })
        .collect();

    if generic_fns.is_empty() { return; }

    // 2. Walk all statements collecting call-sites with explicit type args.
    //    Format: `foo::<f64, i32>(args)` — parsed as Identifier "foo__f64__i32"
    //    or inferred from the argument types when called without `::< >`.
    let mut needed: Vec<(String, Vec<Type>)> = Vec::new();
    for stmt in &program.statements {
        collect_callsites(stmt, &generic_fns, &mut needed);
    }
    // Deduplicate by mangled name
    needed.sort_by(|a, b| a.0.cmp(&b.0));
    needed.dedup_by(|a, b| a.0 == b.0 && type_vecs_eq(&a.1, &b.1));

    // 3. Emit a specialisation for every (fn_name, concrete_type_args) pair.
    let mut new_stmts: Vec<Statement> = Vec::new();
    for (fn_name, type_args) in &needed {
        if let Some(generic) = generic_fns.get(fn_name) {
            if let Some(spec) = specialize_fn(generic, type_args) {
                new_stmts.push(spec);
            }
        }
    }

    // 4. Append new specialisations at the top-level.
    program.statements.extend(new_stmts);
}

// ── Call-site collection ─────────────────────────────────────────────────────

fn type_vecs_eq(a: &[Type], b: &[Type]) -> bool {
    a.len() == b.len() && a.iter().zip(b.iter()).all(|(x, y)| type_mangle(x) == type_mangle(y))
}

fn collect_callsites(
    stmt: &Statement,
    generic_fns: &HashMap<String, Statement>,
    out: &mut Vec<(String, Vec<Type>)>,
) {
    match stmt {
        Statement::FunctionDeclaration { body, .. }
        | Statement::TestDeclaration { body, .. } => {
            for s in body { collect_callsites(s, generic_fns, out); }
        }
        Statement::Let { value, .. } => { collect_expr_callsites(value, generic_fns, out); }
        Statement::Return(e) => { collect_expr_callsites(e, generic_fns, out); }
        Statement::ExpressionStatement(e) => { collect_expr_callsites(e, generic_fns, out); }
        Statement::If { condition, consequence, alternative, .. } => {
            collect_expr_callsites(condition, generic_fns, out);
            for s in consequence { collect_callsites(s, generic_fns, out); }
            if let Some(alt) = alternative { for s in alt { collect_callsites(s, generic_fns, out); } }
        }
        Statement::For { body, .. } | Statement::While { body, .. } => {
            for s in body { collect_callsites(s, generic_fns, out); }
        }
        _ => {}
    }
}

fn collect_expr_callsites(
    expr: &Expression,
    generic_fns: &HashMap<String, Statement>,
    out: &mut Vec<(String, Vec<Type>)>,
) {
    match expr {
        Expression::FunctionCall { name, arguments } => {
            // Explicit monomorphization: `foo__f64` (encoded name) or
            // inferred: `foo(3.14)` where foo is known to be generic.
            if let Some(base) = decode_mono_name(name) {
                // Explicit type annotation in the call name itself
                out.push((base, extract_types_from_mono_name(name)));
            } else if generic_fns.contains_key(name.as_str()) {
                // Infer concrete types from argument expressions
                let inferred: Vec<Type> = arguments.iter()
                    .map(infer_expr_type)
                    .collect();
                if !inferred.iter().any(|t| matches!(t, Type::TypeParam(_))) {
                    out.push((name.clone(), inferred));
                }
            }
            for a in arguments { collect_expr_callsites(a, generic_fns, out); }
        }
        Expression::Infix { left, right, .. } => {
            collect_expr_callsites(left, generic_fns, out);
            collect_expr_callsites(right, generic_fns, out);
        }
        _ => {}
    }
}

// ── Specialisation ───────────────────────────────────────────────────────────

/// Clone a generic FunctionDeclaration with TypeParams substituted by
/// the given concrete types.  Returns `None` if the arity doesn't match.
fn specialize_fn(generic: &Statement, concrete: &[Type]) -> Option<Statement> {
    if let Statement::FunctionDeclaration {
        is_pub, name, type_params, parameters, secret_params, return_type, body, attributes,
    } = generic {
        if type_params.len() != concrete.len() { return None; }

        // Build substitution map: T -> f64, E -> i32, etc.
        let subst: HashMap<String, Type> = type_params.iter().cloned()
            .zip(concrete.iter().cloned())
            .collect();

        // Mangled name: `identity__f64`
        let mangled = format!("{}__{}", name,
            concrete.iter().map(type_mangle).collect::<Vec<_>>().join("_"));

        let new_params: Vec<(String, Type)> = parameters.iter()
            .map(|(n, t)| (n.clone(), subst_type(t, &subst)))
            .collect();
        let new_ret = return_type.as_ref().map(|t| subst_type(t, &subst));
        let new_body: Vec<Statement> = body.iter()
            .map(|s| subst_stmt(s, &subst))
            .collect();

        Some(Statement::FunctionDeclaration {
            is_pub: *is_pub,
            name: mangled,
            type_params: vec![], // concrete — no longer generic
            parameters: new_params,
            secret_params: secret_params.clone(),
            return_type: new_ret,
            body: new_body,
            attributes: attributes.clone(),
        })
    } else {
        None
    }
}

// ── Type substitution ────────────────────────────────────────────────────────

fn subst_type(t: &Type, subst: &HashMap<String, Type>) -> Type {
    match t {
        Type::TypeParam(n) => subst.get(n).cloned().unwrap_or_else(|| t.clone()),
        Type::Array(b, s) => Type::Array(Box::new(subst_type(b, subst)), s.clone()),
        Type::Pointer(b)  => Type::Pointer(Box::new(subst_type(b, subst))),
        Type::Result(ok, err) => Type::Result(
            Box::new(subst_type(ok, subst)),
            Box::new(subst_type(err, subst)),
        ),
        other => other.clone(),
    }
}

fn subst_stmt(stmt: &Statement, subst: &HashMap<String, Type>) -> Statement {
    match stmt {
        Statement::Let { name, is_mut, is_secret, var_type, value } => Statement::Let {
            name: name.clone(),
            is_mut: *is_mut,
            is_secret: *is_secret,
            var_type: var_type.as_ref().map(|t| subst_type(t, subst)),
            value: value.clone(),
        },
        Statement::Return(e) => Statement::Return(e.clone()),
        other => other.clone(),
    }
}

// ── Name mangling helpers ────────────────────────────────────────────────────

/// Produce a stable C-safe name fragment for a type, e.g. `f64` → `f64`.
fn type_mangle(t: &Type) -> String {
    match t {
        Type::I8  => "i8".into(),
        Type::I32 => "i32".into(),
        Type::U64 => "u64".into(),
        Type::F32 => "f32".into(),
        Type::F64 => "f64".into(),
        Type::Bool => "bool".into(),
        Type::Struct(n) => n.clone(),
        Type::Unknown(n) => n.clone(),
        Type::TypeParam(n) => n.clone(),
        Type::Array(b, _) => format!("arr_{}", type_mangle(b)),
        Type::Pointer(b)  => format!("ptr_{}", type_mangle(b)),
        Type::Result(ok, err) => format!("result_{}_{}", type_mangle(ok), type_mangle(err)),
        Type::Tensor { .. } => "tensor".into(),
    }
}

/// Decode a call name like `identity__f64` → Some("identity").
fn decode_mono_name(name: &str) -> Option<String> {
    if name.contains("__") {
        Some(name.splitn(2, "__").next()?.to_string())
    } else {
        None
    }
}

/// Extract the concrete types encoded in a mangled call name.
/// E.g. `identity__f64__i32` → [Type::F64, Type::I32]
fn extract_types_from_mono_name(name: &str) -> Vec<Type> {
    name.splitn(2, "__")
        .nth(1)
        .map(|suffix| {
            suffix.split("__")
                .map(|s| match s {
                    "i8"   => Type::I8,
                    "i32"  => Type::I32,
                    "u64"  => Type::U64,
                    "f32"  => Type::F32,
                    "f64"  => Type::F64,
                    "bool" => Type::Bool,
                    other  => Type::Unknown(other.to_string()),
                })
                .collect()
        })
        .unwrap_or_default()
}

// ── Type inference from expression ──────────────────────────────────────────

/// Best-effort type inference for a single expression (for call-site inference).
fn infer_expr_type(e: &Expression) -> Type {
    match e {
        Expression::Number(n) => {
            if n.fract() == 0.0 && *n >= i32::MIN as f64 && *n <= i32::MAX as f64 {
                Type::I32
            } else {
                Type::F64
            }
        }
        Expression::StringLiteral(_) => Type::Unknown("str".to_string()),
        _ => Type::TypeParam("_".to_string()), // unknown — skip inference
    }
}
