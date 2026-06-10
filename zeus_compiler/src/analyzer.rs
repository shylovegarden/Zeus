use crate::ast::{Program, Statement, Expression, Type};
use std::collections::HashMap;
use crate::comptime::compiler::BytecodeCompiler;
use crate::vm::machine::Machine;

#[derive(PartialEq)]
enum TyKind { Num, StrK, StructK(String), ArrK, TenK, Wild }

fn ty_kind(t: &Type) -> TyKind {
    match t {
        Type::I8 | Type::I32 | Type::U64 | Type::F32 | Type::F64 | Type::Bool => TyKind::Num,
        Type::Struct(n) if n == "str" => TyKind::StrK,
        Type::Struct(n) => TyKind::StructK(n.clone()),
        Type::Array(..) => TyKind::ArrK,
        Type::Tensor { .. } => TyKind::TenK,
        _ => TyKind::Wild, // Unknown / Pointer / Result -> wildcard, never flagged
    }
}

/// Conservative type compatibility: only INCOMPATIBLE when both sides are known,
/// concrete, DIFFERENT kinds (e.g. str vs numeric, struct A vs struct B). All
/// numerics are mutually compatible (number literals are f64 until annotated), and
/// any Unknown/Pointer/Result is a wildcard, so loose programs are never falsely
/// rejected -- the checker never produces a false positive.
fn types_compatible(a: &Type, b: &Type) -> bool {
    use TyKind::*;
    match (ty_kind(a), ty_kind(b)) {
        (Wild, _) | (_, Wild) => true,
        (Num, Num) => true,
        (StrK, StrK) => true,
        (ArrK, ArrK) | (TenK, TenK) | (ArrK, TenK) | (TenK, ArrK) => true,
        (StructK(x), StructK(y)) => x == y,
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
    }
}

pub struct SemanticAnalyzer {
    scopes: Vec<HashMap<String, (bool, Type)>>, // lexical scope stack of (is_mut, type)
    struct_schemas: HashMap<String, Vec<(String, crate::ast::Type)>>,
    function_types: HashMap<String, Type>,
    function_arity: HashMap<String, usize>,
    current_return: Vec<Option<Type>>,
}

impl SemanticAnalyzer {
    pub fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            struct_schemas: HashMap::new(),
            function_types: HashMap::new(),
            function_arity: HashMap::new(),
            current_return: Vec::new(),
        }
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

    pub fn analyze(&mut self, program: &mut Program) -> Result<(), String> {
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
                }
                Statement::ExternFunctionDeclaration { name, return_type, parameters, .. } => {
                    let ret_ty = match return_type {
                        Some(ty) => ty.clone(),
                        None => Type::Unknown("void".to_string()),
                    };
                    self.function_types.insert(name.clone(), ret_ty);
                    self.function_arity.insert(name.clone(), parameters.len());
                }
                _ => {}
            }
        }

        for stmt in &mut program.statements {
            self.analyze_statement(stmt)?;
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
                } else {
                    *var_type = Some(inferred.clone());
                }
                let declared = var_type.clone().unwrap_or(inferred);
                self.declare(name, (*is_mut, declared));
            }
            Statement::ExpressionStatement(expr) => {
                // Check if it's an assignment
                if let Expression::Infix { left, operator, right: _ } = expr {
                    if operator == "Assign" {
                        if let Expression::Identifier(name) = &**left {
                            if let Some(&(is_mut, _)) = self.lookup(name) {
                                if !is_mut {
                                    return Err(format!("Immutable variable '{}' cannot be reassigned. Use 'let mut'.", name));
                                }
                            } else {
                                return Err(format!("Assignment to undeclared variable '{}'.", name));
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
                for s in body {
                    self.analyze_statement(s)?;
                }
                self.current_return.pop();
                self.pop_scope();
            }
            Statement::For { iterator, body, .. } => {
                self.push_scope();
                // The loop iterator is essentially a mutable local
                self.declare(iterator, (true, Type::I32));
                for s in body {
                    self.analyze_statement(s)?;
                }
                self.pop_scope();
            }
            Statement::While { condition, body } => {
                self.analyze_expression(condition)?;
                self.push_scope();
                for s in body {
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
                if let Some(Some(rt)) = self.current_return.last().cloned() {
                    let got = self.infer_type(expr);
                    if !types_compatible(&rt, &got) {
                        return Err(format!("return type mismatch: function returns {} but a `return` yields {}",
                            ty_name(&rt), ty_name(&got)));
                    }
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
                for (_, val) in fields {
                    self.analyze_expression(val)?;
                }
            }
            Expression::Infix { left, right, .. } => {
                self.analyze_expression(left)?;
                self.analyze_expression(right)?;
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
            }
            Expression::FieldAccess { base, .. } => {
                self.analyze_expression(base)?;
            }
            Expression::IndexAccess { base, index } => {
                self.analyze_expression(base)?;
                self.analyze_expression(index)?;
            }
            Expression::OramAccess { base, index, bound: _ } => {
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
            _ => Type::Unknown("UnknownExpr".to_string())
        }
    }
}
