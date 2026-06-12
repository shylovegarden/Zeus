#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or, clippy::type_complexity)]
use crate::ast::{Program, Statement, Expression, Type};
use std::collections::HashMap;
use crate::comptime::compiler::BytecodeCompiler;
use crate::vm::machine::Machine;

// ── Strict type-kind (bidirectional inference engine) ─────────────
#[derive(PartialEq)]
enum TyKind { Num, StrK, StructK(String), ArrK, TenK, PtrK, ResultK, UnknownK }

fn ty_kind(t: &Type) -> TyKind {
    match t {
        Type::I8 | Type::I32 | Type::U64 | Type::F32 | Type::F64 | Type::Bool => TyKind::Num,
        Type::Struct(n) if n == "str" => TyKind::StrK,
        Type::Struct(n) => TyKind::StructK(n.clone()),
        Type::Array(..) => TyKind::ArrK,
        Type::Tensor { .. } => TyKind::TenK,
        Type::Pointer(_) => TyKind::PtrK,
        Type::Result(..) => TyKind::ResultK,
        _ => TyKind::UnknownK,
    }
}

// ── Strict width-aware numeric type system (--strict-types) ──────────────────
/// Each numeric type is its own lane; any cross-lane assignment without an
/// explicit cast is a compile error in strict mode.
#[derive(Debug, PartialEq, Clone, Copy)]
enum StrictNumericKind {
    I8,   // signed  8-bit
    I32,  // signed 32-bit
    U64,  // unsigned 64-bit
    F32,  // 32-bit float
    F64,  // 64-bit float
    Bool, // boolean (not numeric, but often used with numerics)
    /// An untyped number literal (e.g. `42`, `3.14`). Assignable to any
    /// numeric type — the literal value carries no width commitment.
    Literal,
    /// Not a numeric type (struct, array, str, etc.) — skip strict check.
    NonNumeric,
}

fn numeric_strict_kind(t: &Type) -> StrictNumericKind {
    match t {
        Type::I8    => StrictNumericKind::I8,
        Type::I32   => StrictNumericKind::I32,
        Type::U64   => StrictNumericKind::U64,
        Type::F32   => StrictNumericKind::F32,
        Type::F64   => StrictNumericKind::F64,
        Type::Bool  => StrictNumericKind::Bool,
        _           => StrictNumericKind::NonNumeric,
    }
}

/// Returns `Ok(())` if assigning `rhs` to a slot typed `lhs` is safe in strict
/// mode, or `Err(message)` describing the width/signedness violation.
///
/// Rules:
///  1. If either side is `NonNumeric` → not our problem; permissive path handles it.
///  2. If `rhs` is `Literal` → always OK (no precision loss on a constant).
///  3. If `lhs == rhs` kind → OK.
///  4. Otherwise → reject with an informative error.
fn numerics_width_compatible(lhs: StrictNumericKind, rhs: StrictNumericKind) -> Result<(), String> {
    use StrictNumericKind::*;
    match (lhs, rhs) {
        // Non-numeric types: fall through to the permissive checker
        (NonNumeric, _) | (_, NonNumeric) => Ok(()),
        // Untyped literal fits any numeric slot
        (_, Literal) => Ok(()),
        // Identical types always OK
        (a, b) if a == b => Ok(()),
        // Everything else is a width/kind mismatch
        (lhs_k, rhs_k) => Err(format!(
            "strict type error: cannot implicitly assign {:?} value to a {:?} slot \
             (use an explicit cast or change the type annotation)",
            rhs_k, lhs_k
        )),
    }
}

/// Infer the `StrictNumericKind` of an expression — `Literal` for bare number
/// literals so they satisfy any numeric annotation without precision-loss.
fn strict_kind_of_expr(expr: &Expression, inferred_type: &Type) -> StrictNumericKind {
    if matches!(expr, Expression::Number(_)) {
        return StrictNumericKind::Literal;
    }
    numeric_strict_kind(inferred_type)
}

/// Rigorous type compatibility: enforces bidirectional inference bounds.
/// Unknowns are rejected to prevent 'any'-type leaking.
fn types_compatible(a: &Type, b: &Type) -> bool {
    use TyKind::*;
    match (ty_kind(a), ty_kind(b)) {
        // Enforce strict unifications. Unknowns are no longer wildcards.
        (Num, Num) => true,
        (StrK, StrK) => true,
        (ArrK, ArrK) | (TenK, TenK) | (ArrK, TenK) | (TenK, ArrK) => true,
        (StructK(x), StructK(y)) => x == y,
        (PtrK, PtrK) => true,
        (ResultK, ResultK) => true,
        _ => false,
    }
}

fn ty_name(t: &Type) -> String {
    match t {
        Type::I8 => "i8".into(), Type::I32 => "i32".into(), Type::U64 => "u64".into(),
        Type::F32 => "f32".into(), Type::F64 => "f64".into(), Type::Bool => "bool".into(),
        Type::Struct(n) if n == "str" => "str".into(),
        Type::Struct(n) => format!("struct {}", n),
        Type::Array(..) => "array".into(),
        Type::Tensor { .. } => "tensor".into(),
        Type::Unknown(_) => "<unknown>".into(),
        Type::Pointer(_) => "pointer".into(),
        Type::Result(..) => "Result".into(),
        Type::TypeParam(n) => n.clone(),
    }
}

pub struct SemanticAnalyzer {
    scopes: Vec<HashMap<String, (bool, Type)>>, // lexical scope stack of (is_mut, type)
    struct_schemas: HashMap<String, Vec<(String, crate::ast::Type)>>,
    function_types: HashMap<String, Type>,
    function_arity: HashMap<String, usize>,
    function_param_types: HashMap<String, Vec<Type>>,
    current_return: Vec<Option<Type>>,
    /// When true, numeric assignments are checked for width/signedness compatibility.
    /// Activated by `SemanticAnalyzer::new_strict()` (flag: `--strict-types`).
    pub strict_types: bool,
    pub current_line: usize,
}

impl SemanticAnalyzer {
    /// Default constructor — permissive numeric type checking (backward-compatible).
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            struct_schemas: HashMap::new(),
            function_types: HashMap::new(),
            function_arity: HashMap::new(),
            function_param_types: HashMap::new(),
            current_return: Vec::new(),
            strict_types: false,
            current_line: 1,
        }
    }

    /// Strict constructor — enables width-aware numeric type checking.
    /// Use when `--strict-types` is passed on the CLI.
    pub fn new_strict() -> Self {
        Self { strict_types: true, ..Self::new() }
    }

    /// Run the strict numeric width check if strict mode is enabled.
    /// `ann` is the declared/expected type; `val_expr` is the initialising expression.
    fn check_numeric_width(
        &self,
        ann: &Type,
        val_expr: &Expression,
        inferred: &Type,
        context: &str,
    ) -> Result<(), String> {
        if !self.strict_types { return Ok(()); }
        let lhs_k = numeric_strict_kind(ann);
        let rhs_k = strict_kind_of_expr(val_expr, inferred);
        numerics_width_compatible(lhs_k, rhs_k)
            .map_err(|e| format!("{}: {}", context, e))
    }

    fn push_scope(&mut self) { self.scopes.push(HashMap::new()); }
    fn pop_scope(&mut self) { self.scopes.pop(); }
    fn declare(&mut self, name: &str, info: (bool, Type)) {
        self.scopes.last_mut().expect("scope stack underflow").insert(name.to_string(), info);
    }
    fn lookup(&self, name: &str) -> Option<&(bool, Type)> {
        for scope in self.scopes.iter().rev() {
            if let Some(v) = scope.get(name) { return Some(v); }
        }
        None
    }

    pub fn analyze(&mut self, program: &mut Program) -> Result<(), crate::diagnostics::Diagnostic> {
        // Pre-pass: Register all function return types
        for stmt in &program.statements {
            match stmt {
                Statement::FunctionDeclaration { name, return_type, parameters, .. } => {
                    let ret_ty = match return_type {
                        Some(ty) => ty.clone(),
                        None => Type::Unknown("void".to_string()),
                    };
                    self.function_types.insert(name.clone(), ret_ty);
                    self.function_arity.insert(name.clone(), parameters.len());
                    self.function_param_types.insert(name.clone(), parameters.iter().map(|(_, t)| t.clone()).collect());
                }
                Statement::ExternFunctionDeclaration { name, return_type, parameters, .. } => {
                    let ret_ty = match return_type {
                        Some(ty) => ty.clone(),
                        None => Type::Unknown("void".to_string()),
                    };
                    self.function_types.insert(name.clone(), ret_ty);
                    self.function_arity.insert(name.clone(), parameters.len());
                    self.function_param_types.insert(name.clone(), parameters.iter().map(|(_, t)| t.clone()).collect());
                }
                _ => {}
            }
        }

        for stmt in &mut program.statements {
            if let Statement::LineDirective(l) = stmt {
                self.current_line = *l;
            }
            if let Err(e) = self.analyze_statement(stmt) {
                return Err(crate::diagnostics::Diagnostic::new(
                    crate::diagnostics::Severity::Error,
                    e,
                    self.current_line,
                    1, // column fallback
                    1, // span length fallback
                    None, // source path will be injected by main.rs
                ));
            }
        }
        Ok(())
    }

    fn analyze_statement(&mut self, stmt: &mut Statement) -> Result<(), String> {
        match stmt {
            Statement::StructDeclaration { name, fields, .. } => {
                for (_, ty) in &mut *fields {
                    self.analyze_type(ty)?;
                }
                self.struct_schemas.insert(name.clone(), fields.clone());
            }
            Statement::Let { name, is_mut, is_secret: _, value, var_type } => {
                self.analyze_expression(value)?;
                let inferred = self.infer_type(value);
                // Type check: if there is an explicit annotation, the initializer must
                // be compatible with it (conservative -- only clear mismatches flagged).
                if let Some(ann) = var_type.clone() {
                    if !types_compatible(&ann, &inferred) {
                        return Err(format!("type mismatch: '{}' is declared {} but initialized with {}",
                            name, ty_name(&ann), ty_name(&inferred)));
                    }
                    // Strict width check (only fires when --strict-types is active)
                    self.check_numeric_width(
                        &ann, value, &inferred,
                        &format!("let '{}':", name),
                    )?;
                } else {
                    *var_type = Some(inferred.clone());
                }
                let declared = var_type.clone().unwrap_or(inferred);
                self.declare(name, (*is_mut, declared));
            }
            Statement::ExpressionStatement(expr) => {
                // Check if it's an assignment
                if let Expression::Infix { left, operator, right } = expr.clone() {
                    if matches!(operator.as_str(), "Assign"|"PlusAssign"|"MinusAssign"|"StarAssign"|"SlashAssign"|"PercentAssign") {
                        if let Expression::Identifier(name) = &*left {
                            match self.lookup(name.as_str()) {
                                Some(&(is_mut, ref decl_ty)) => {
                                    if !is_mut {
                                        return Err(format!("Immutable variable '{}' cannot be reassigned. Use 'let mut'.", name));
                                    }
                                    // Type-check plain assignment (not compound)
                                    if operator == "Assign" {
                                        let rhs_ty = self.infer_type(&right);
                                        let dt = decl_ty.clone();
                                        if !types_compatible(&dt, &rhs_ty) {
                                            return Err(format!(
                                                "type mismatch: cannot assign {} to '{}' which is {}",
                                                ty_name(&rhs_ty), name, ty_name(&dt)
                                            ));
                                        }
                                        // Strict width check
                                        self.check_numeric_width(
                                            &dt, &right, &rhs_ty,
                                            &format!("assignment to '{}':", name),
                                        )?;
                                    }
                                }
                                None => {
                                    return Err(format!("Assignment to undeclared variable '{}'.", name));
                                }
                            }
                        }
                    }
                }
                self.analyze_expression(expr)?;
            }
            Statement::FunctionDeclaration { parameters, return_type, body, attributes, name, .. } => {
                for attr in attributes.iter_mut() {
                    match attr {
                        crate::ast::FunctionAttribute::Verify(expr, has_timed_out) => {
                            // HONESTY: no static SMT proof is actually performed here (z3 is not
                            // invoked from this path), so do not claim one. We always fall back to
                            // an injected runtime check, which is the safe, sound behavior.
                            println!("\x1b[35m[ZEUS @verify]\x1b[0m fn {}(): no static proof attempted; enforcing constraint with an injected runtime check: {:?}", name, expr);
                            *has_timed_out = true;
                        }
                        crate::ast::FunctionAttribute::Requires(expr, _) => {
                            println!("\x1b[35m[ZEUS CONTRACT]\x1b[0m fn {}() @requires {:?} (runtime-enforced)", name, expr);
                        }
                        crate::ast::FunctionAttribute::Ensures(expr, _) => {
                            println!("\x1b[35m[ZEUS CONTRACT]\x1b[0m fn {}() @ensures {:?} (runtime-enforced)", name, expr);
                        }
                        crate::ast::FunctionAttribute::Adaptive(params) => {
                            eprintln!("\x1b[33m[ZEUS]\x1b[0m note: @adaptive on fn {}() recorded; runtime JIT mutation is a research stub (no-op). Params: {}", name, params);
                        }
                        crate::ast::FunctionAttribute::Wcet(_) | crate::ast::FunctionAttribute::Stack(_) | crate::ast::FunctionAttribute::ConstantTime => {}
                        crate::ast::FunctionAttribute::FfiExport => {}
                    }
                }
                for (_, ty) in parameters.iter_mut() {
                    self.analyze_type(ty)?;
                }
                if let Some(rt) = return_type {
                    self.analyze_type(rt)?;
                }
                // In a real compiler, we would push a new scope block here.
                // For this prototype, we'll register parameters as immutable in the global map
                self.push_scope();
                for (p_name, ty) in parameters.iter() {
                    self.declare(p_name, (false, ty.clone()));
                }
                self.current_return.push(return_type.clone());
                for s in body.iter_mut() {
                    self.analyze_statement(s)?;
                }
                // Check implicit return: if the last statement is a bare expression
                // and the function has a declared non-void return type, type-check it.
                if let Some(Some(rt)) = self.current_return.last().cloned() {
                    if let Some(Statement::ExpressionStatement(last_expr)) = body.last() {
                        // Only flag if it looks like a value expression (not an assignment)
                        let is_assign = matches!(last_expr, Expression::Infix { operator, .. }
                            if matches!(operator.as_str(), "Assign"|"PlusAssign"|"MinusAssign"|"StarAssign"|"SlashAssign"|"PercentAssign"));
                        if !is_assign {
                            let got = self.infer_type(last_expr);
                            if !types_compatible(&rt, &got) {
                                return Err(format!(
                                    "implicit return type mismatch in '{}': function returns {} but last expression is {}",
                                    name, ty_name(&rt), ty_name(&got)
                                ));
                            }
                        }
                    }
                }
                self.current_return.pop();
                self.pop_scope();
            }
            Statement::For { iterator, body, .. } => {
                self.push_scope();
                // The loop iterator is essentially a mutable local
                self.declare(iterator, (true, Type::I32));
                for s in body.iter_mut() {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::While { condition, body } => {
                self.analyze_expression(condition)?;
                self.push_scope();
                for s in body.iter_mut() {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::ParallelBlock { iterator, start, end, statements } => {
                self.analyze_expression(start)?;
                self.analyze_expression(end)?;
                self.push_scope();
                self.declare(iterator, (false, Type::U64));
                for s in statements {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::TargetBlock { statements, .. } 
            | Statement::EnclaveBlock { statements }
            | Statement::ProofBlock { statements } => {
                self.push_scope();
                for s in statements {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::ClusterBlock { statements } => {
                eprintln!("\x1b[33m[ZEUS]\x1b[0m note: `cluster {{}}` is not implemented (no-op; the block runs locally).");
                self.push_scope();
                for s in statements {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::ComptimeBlock { statements } => {
                // Run the Bytecode Compiler to flatten the AST
                let mut compiler = BytecodeCompiler::new();
                // A comptime block that contains constructs the VM can't evaluate is
                // NOT a hard error: we leave it as ordinary runtime code rather than
                // crashing the compiler.
                match compiler.compile_block(statements) {
                    Ok(()) => {
                        let mut vm = Machine::new();
                        if let Err(e) = vm.run(&compiler.bytecode, &compiler.constants) {
                            return Err(format!("Comptime VM Error: {}", e));
                        }
                    }
                    Err(e) => {
                        eprintln!("[ZEUS] comptime block not statically evaluable ({}); left as runtime code.", e);
                    }
                }
            }
            Statement::ExternFunctionDeclaration { parameters, return_type, .. } => {
                for (_, ty) in parameters.iter_mut() {
                    self.analyze_type(ty)?;
                }
                if let Some(rt) = return_type {
                    self.analyze_type(rt)?;
                }
            }
            Statement::Return(expr) => {
                self.analyze_expression(expr)?;
                if let Some(Some(rt)) = self.current_return.last().cloned() {
                    let got = self.infer_type(expr);
                    if !types_compatible(&rt, &got) {
                        return Err(format!("return type mismatch: function returns {} but a `return` yields {}",
                            ty_name(&rt), ty_name(&got)));
                    }
                    // Strict width check on return value
                    self.check_numeric_width(
                        &rt, expr, &got,
                        "return:",
                    )?;
                }
            }
            Statement::LineDirective(_) => {}
            Statement::AtomicAdd { target, amount: _ } => {
                // Must be a mutable variable or pointer in a real compiler
                if self.lookup(target).is_none() {
                    return Err(format!("Atomic add to undeclared variable '{}'", target));
                }
            }
            Statement::If { consequence, alternative, .. } => {
                self.push_scope();
                for s in consequence {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
                if let Some(alt) = alternative {
                    self.push_scope();
                    for s in alt {
                        self.analyze_statement(s)?;
                    }
                    self.pop_scope();
                }
            }
            Statement::EnumDeclaration { name, variants } => {
                // Register enum type: each variant becomes a constructor
                let mut schema = Vec::new();
                for v in variants {
                    // Store variant name as a zero-field struct for type lookups
                    schema.push((v.name.clone(), crate::ast::Type::Unknown(format!("{}::{}", name, v.name))));
                }
                self.struct_schemas.insert(name.clone(), schema);
                // Register enum as a known type
                self.function_types.insert(name.clone(), crate::ast::Type::Struct(name.clone()));
            }
            Statement::MatchStatement { scrutinee, arms } => {
                self.analyze_expression(scrutinee)?;
                for arm in arms {
                    self.push_scope();
                    // Bind tuple-variant payload bindings as Unknown type
                    if let crate::ast::MatchPattern::VariantTuple { bindings, .. } = &arm.pattern {
                        for b in bindings {
                            self.declare(b, (false, crate::ast::Type::Unknown("EnumPayload".to_string())));
                        }
                    }
                    for s in &mut arm.body {
                        self.analyze_statement(s)?;
                    }
                    self.pop_scope();
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_type(&mut self, ty: &mut Type) -> Result<(), String> {
        match ty {
            Type::Tensor { dimensions, .. } => {
                for dim in dimensions {
                    if let Expression::Identifier(_) = dim {
                        return Err("Zero-Heap Policy Violation: Dynamic memory allocation is strictly prohibited in safety-critical mode.".to_string());
                    }
                    self.analyze_expression(dim)?;
                }
            }
            Type::Array(inner, size) => {
                if let Expression::Identifier(_) = &**size {
                    return Err("Zero-Heap Policy Violation: Dynamic memory allocation is strictly prohibited in safety-critical mode.".to_string());
                }
                self.analyze_expression(size)?;
                self.analyze_type(inner)?;
            }
            Type::Result(ok, err) => {
                self.analyze_type(ok)?;
                self.analyze_type(err)?;
            }
            _ => {}
        }
        Ok(())
    }

    fn analyze_expression(&mut self, expr: &mut Expression) -> Result<(), String> {
        match expr {
            Expression::TensorDefinition { dimensions } => {
                for dim in dimensions {
                    if let Expression::Identifier(_) = dim {
                        return Err("Zero-Heap Policy Violation: Dynamic memory allocation is strictly prohibited in safety-critical mode.".to_string());
                    }
                    self.analyze_expression(dim)?;
                }
            }
            Expression::StructInit { name, fields } => {
                if !self.struct_schemas.contains_key(name) {
                    return Err(format!("Cannot initialize unknown struct '{}'", name));
                }
                let schema = self.struct_schemas.get(name).cloned().unwrap_or_default();
                for (fname, val) in fields.iter_mut() {
                    self.analyze_expression(val)?;
                    if let Some((_, field_ty)) = schema.iter().find(|(n, _)| n == fname) {
                        let val_ty = self.infer_type(val);
                        if !types_compatible(field_ty, &val_ty) {
                            return Err(format!(
                                "type mismatch: struct '{}' field '{}' is {} but initialized with {}",
                                name, fname, ty_name(field_ty), ty_name(&val_ty)
                            ));
                        }
                    }
                }
            }
            Expression::Infix { left, operator, right } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
                if matches!(operator.as_str(), "Slash" | "Percent" | "SlashAssign" | "PercentAssign") {
                    if let Expression::Number(n) = &**right {
                        if *n == 0.0 {
                            return Err("division or modulo by a constant zero".to_string());
                        }
                    }
                }
            }
            Expression::Prefix { operand, .. } => {
                self.analyze_expression(operand)?;
            }
            Expression::FunctionCall { name, arguments } => {
                for arg in arguments.iter_mut() {
                    self.analyze_expression(arg)?;
                }
                // Conservative type check: arity for USER-DEFINED functions only.
                // Builtins (println, sqrt, ...) are not in the table, so never flagged.
                if let Some(&arity) = self.function_arity.get(name.as_str()) {
                    if arguments.len() != arity {
                        return Err(format!(
                            "call to '{}' has {} argument(s) but it is defined with {}",
                            name, arguments.len(), arity));
                    }
                }
                // Conservative argument type check (user-defined fns only; same
                // clear-mismatch rule as `let`/`return`, so no false positives).
                if let Some(ptypes) = self.function_param_types.get(name.as_str()).cloned() {
                    for (i, arg) in arguments.iter().enumerate() {
                        if let Some(pt) = ptypes.get(i) {
                            let at = self.infer_type(arg);
                            if !types_compatible(pt, &at) {
                                return Err(format!("argument {} to '{}' has type {} but the parameter is {}",
                                    i + 1, name, ty_name(&at), ty_name(pt)));
                            }
                            // Strict width check on call arguments
                            self.check_numeric_width(
                                pt, arg, &at,
                                &format!("argument {} to '{}':", i + 1, name),
                            )?;
                        }
                    }
                }
            }
            Expression::FieldAccess { base, .. } => {
                self.analyze_expression(base)?;
            }
            Expression::IndexAccess { base, index } => {
                self.analyze_expression(base)?;
                self.analyze_expression(index)?;
                let base_ty = self.infer_type(base);
                if let Type::Array(_, size_expr) = base_ty {
                    let mut fv = crate::formal_verifier::FormalVerifier::new();
                    let geq0 = Expression::Infix {
                        left: index.clone(),
                        operator: "GreaterEqual".to_string(),
                        right: Box::new(Expression::Number(0.0)),
                    };
                    if let Err(e) = fv.prove_assertion(&geq0) {
                        return Err(format!("Z3 Array Bounds Violation: cannot prove index >= 0: {}", e));
                    }
                    let lt_size = Expression::Infix {
                        left: index.clone(),
                        operator: "LessThan".to_string(),
                        right: size_expr.clone(),
                    };
                    if let Err(e) = fv.prove_assertion(&lt_size) {
                        return Err(format!("Z3 Array Bounds Violation: cannot prove index < size: {}", e));
                    }
                }
            }
            Expression::OramAccess { base, index, .. } => {
                self.analyze_expression(base)?;
                self.analyze_expression(index)?;
            }
            Expression::ArrayLiteral(elements) => {
                for el in elements { self.analyze_expression(el)?; }
            }
            Expression::Comptime(inner) => {
                // Best-effort constant folding. If the expression isn't foldable
                // by the comptime VM, leave it for runtime evaluation rather than
                // failing the build.
                let mut compiler = BytecodeCompiler::new();
                if compiler.compile_expression(inner).is_ok() {
                    let mut vm = Machine::new();
                    if vm.run(&compiler.bytecode, &compiler.constants).is_ok() {
                        if let Some(result) = vm.stack.pop() {
                            *expr = Expression::Number(result);
                        }
                    }
                } else {
                    eprintln!("[ZEUS] comptime() expression not foldable; evaluated at runtime.");
                }
            }
            Expression::EnumVariant { payload, .. } => {
                for p in payload { self.analyze_expression(p)?; }
            }
            Expression::MatchExpr { scrutinee, arms } => {
                self.analyze_expression(scrutinee)?;
                for arm in arms {
                    self.push_scope();
                    if let crate::ast::MatchPattern::VariantTuple { bindings, .. } = &arm.pattern {
                        for b in bindings {
                            self.declare(b, (false, crate::ast::Type::Unknown("EnumPayload".to_string())));
                        }
                    }
                    for s in &mut arm.body {
                        self.analyze_statement(s)?;
                    }
                    self.pop_scope();
                }
            }
            _ => {}
        }
        Ok(())
    }

    pub fn infer_type(&self, expr: &Expression) -> Type {
        match expr {
            Expression::Number(_) => Type::F64,
            Expression::StringLiteral(_) => Type::Struct("str".to_string()),
            Expression::Identifier(name) => {
                if let Some((_, ty)) = self.lookup(name) {
                    ty.clone()
                } else {
                    Type::Unknown(name.clone())
                }
            }
            Expression::StructInit { name, .. } => Type::Struct(name.clone()),
            Expression::Infix { left, operator, .. } => {
                if operator == "Equal" || operator == "NotEqual" || operator == "LessThan" || operator == "GreaterThan" {
                    return Type::Bool;
                }
                self.infer_type(left)
            }
            Expression::Prefix { operator, operand } => {
                if operator == "Not" { return Type::Bool; }
                self.infer_type(operand)
            }
            Expression::FunctionCall { name, arguments } => {
                match name.as_str() {
                    "sqrt" | "pow" | "floor" | "ceil" => return Type::F64,
                    "abs" | "min" | "max" | "clamp" => {
                        return arguments.first().map(|a| self.infer_type(a)).unwrap_or(Type::F64);
                    }
                    _ => {}
                }
                if let Some(ty) = self.function_types.get(name) {
                    ty.clone()
                } else {
                    Type::Unknown("FuncResult".to_string())
                }
            }
            Expression::FieldAccess { base, field } => {
                let base_ty = self.infer_type(base);
                if let Type::Struct(struct_name) = base_ty.clone() {
                    if let Some(fields) = self.struct_schemas.get(&struct_name) {
                        for (fname, ftype) in fields {
                            if fname == field {
                                return ftype.clone();
                            }
                        }
                    }
                } else if let Type::Tensor { .. } = base_ty {
                    if field == "data" {
                        return Type::Array(Box::new(Type::F64), Box::new(crate::ast::Expression::Number(0.0)));
                    }
                }
                Type::Unknown(format!("Field_{}", field))
            }
            Expression::IndexAccess { base, .. } => {
                let base_ty = self.infer_type(base);
                match base_ty {
                    Type::Array(inner, _) => *inner,
                    Type::Tensor { .. } => Type::F64,
                    _ => Type::Unknown("ArrayElem".to_string())
                }
            }
            Expression::OramAccess { base, .. } => {
                let base_ty = self.infer_type(base);
                match base_ty {
                    Type::Array(inner, _) => *inner,
                    Type::Tensor { .. } => Type::F64,
                    _ => Type::Unknown("ArrayElem".to_string())
                }
            }
            Expression::TensorDefinition { dimensions } => {
                Type::Tensor { dimensions: dimensions.clone(), is_sparse: false }
            }
            Expression::NvmeDmaMap { .. } => {
                Type::Pointer(Box::new(Type::Unknown("void".to_string())))
            }
            Expression::Try(inner) => self.infer_type(inner),
            Expression::Comptime(inner) => self.infer_type(inner),
            Expression::ArrayLiteral(elements) => {
                let elem_ty = elements.first().map(|e| self.infer_type(e)).unwrap_or(Type::Unknown("ArrayElem".to_string()));
                Type::Array(Box::new(elem_ty), Box::new(crate::ast::Expression::Number(elements.len() as f64)))
            }
            Expression::EnumVariant { enum_name, .. } => Type::Struct(enum_name.clone()),
            Expression::MatchExpr { .. } => Type::Unknown("MatchResult".to_string()),
        }
    }
}
