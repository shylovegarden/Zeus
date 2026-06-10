#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
use crate::ast::{Expression, Program, Statement, Type};

pub struct MlirCodegen {
    pub cpu_fallback: bool,
}

impl MlirCodegen {
    pub fn new() -> Self {
        MlirCodegen { cpu_fallback: true }
    }

    pub fn generate(&self, program: &Program) -> String {
        let mut sparse_vars = std::collections::HashSet::new();
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { parameters, .. } = stmt {
                for (p_name, p_type) in parameters {
                    if let Type::Tensor { is_sparse: true, .. } = p_type {
                        sparse_vars.insert(p_name.clone());
                    }
                }
            }
        }

        let mut mlir = String::new();
        mlir.push_str("module {\n");
        
        for stmt in &program.statements {
            mlir.push_str(&self.generate_statement(stmt, 1, &sparse_vars));
        }

        mlir.push_str("}\n");
        mlir
    }

    fn generate_statement(&self, stmt: &Statement, indent_level: usize, sparse_vars: &std::collections::HashSet<String>) -> String {
        let pad = "  ".repeat(indent_level);
        match stmt {
            Statement::FunctionDeclaration { name, body, .. } => {
                let mut out = format!("{}func.func @{}(%arg0: !llvm.ptr) -> () {{\n", pad, name);
                for s in body {
                    out.push_str(&self.generate_statement(s, indent_level + 1, sparse_vars));
                }
                out.push_str(&format!("{}  return\n", pad));
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::Let { name, value, .. } => {
                let val_expr = self.generate_expression(value, sparse_vars);
                // For a prototype, just emit a placeholder variable assignment in MLIR comments or local mapping
                // If it's a tensor allocation:
                if let Expression::TensorDefinition { dimensions } = value {
                    let mut shape = String::new();
                    for dim in dimensions {
                        if let Expression::Number(n) = dim {
                            shape.push_str(&format!("{}x", n));
                        } else {
                            shape.push_str("?x");
                        }
                    }
                    format!("{}%{} = tensor.empty() : tensor<{}f64>\n", pad, name, shape)
                } else {
                    format!("{}// let {} = {}\n", pad, name, val_expr)
                }
            }
            Statement::ParallelBlock { statements, .. } => {
                let mut out = format!("{}scf.parallel (%i, %j) = (%c0, %c0) to (%c10, %c10) step (%c1, %c1) {{\n", pad);
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent_level + 1, sparse_vars));
                }
                out.push_str(&format!("{}  scf.yield\n", pad));
                out.push_str(&format!("{}}}\n", pad));
                out
            }
            Statement::TargetBlock { targets, statements } => {
                let mut out = format!("{}// TARGET BLOCK OFF-LOAD: {}\n", pad, targets.join(", "));
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent_level, sparse_vars));
                }
                out
            }
            Statement::ExpressionStatement(expr) => {
                format!("{}{}\n", pad, self.generate_expression(expr, sparse_vars))
            }
            Statement::ExternFunctionDeclaration { name, parameters, return_type } => {
                let mut params = Vec::new();
                for (_, t) in parameters {
                    match t {
                        Type::Tensor { is_sparse: true, .. } => params.push("tensor<?x?xf64, #sparse>".to_string()),
                        Type::Tensor { is_sparse: false, .. } => params.push("tensor<?x?xf64>".to_string()),
                        _ => params.push("!llvm.ptr".to_string()),
                    }
                }
                let ret = match return_type {
                    Some(Type::Tensor { is_sparse: true, .. }) => "tensor<?x?xf64, #sparse>",
                    Some(Type::Tensor { is_sparse: false, .. }) => "tensor<?x?xf64>",
                    _ => "!llvm.ptr",
                };
                format!("{}func.func private @{}({}) -> {}\n", pad, name, params.join(", "), ret)
            }
            Statement::LineDirective(_) => String::new(),
            Statement::StructDeclaration { name, .. } => {
                format!("{}// MLIR Struct: {}\n", pad, name)
            }
            Statement::EnclaveBlock { statements } => {
                let mut out = format!("{}// ENCLAVE BLOCK\n", pad);
                for s in statements {
                    out.push_str(&self.generate_statement(s, indent_level + 1, sparse_vars));
                }
                out
            }
            _ => format!("{}// Unmapped MLIR construct\n", pad),
        }
    }

    fn generate_expression(&self, expr: &Expression, sparse_vars: &std::collections::HashSet<String>) -> String {
        match expr {
            Expression::Number(n) => n.to_string(),
            Expression::Identifier(id) => format!("%{}", id),
            Expression::Infix { left, operator, right } => {
                let op = match operator.as_str() {
                    "Plus" => "addf",
                    "Minus" => "subf",
                    "Star" => "mulf",
                    "Slash" => "divf",
                    _ => "unknown_op"
                };
                format!("arith.{} {}, {}", op, self.generate_expression(left, sparse_vars), self.generate_expression(right, sparse_vars))
            }
            Expression::FunctionCall { name, arguments } => {
                if name == "matmul" && arguments.len() == 2 {
                    let arg0 = self.generate_expression(&arguments[0], sparse_vars);
                    let arg1 = self.generate_expression(&arguments[1], sparse_vars);
                    
                    let mut is_sparse = false;
                    if let Expression::Identifier(id) = &arguments[0] {
                        if sparse_vars.contains(id) { is_sparse = true; }
                    }
                    if let Expression::Identifier(id) = &arguments[1] {
                        if sparse_vars.contains(id) { is_sparse = true; }
                    }

                    if self.cpu_fallback {
                        let mut res = String::new();
                        if is_sparse {
                            res.push_str("// [ZEUS MLIR: SPARSE CSR FORMAT ENABLED. ZERO-PATH SKIPPING ACTIVE]\n");
                            res.push_str(&format!("affine.for %i = 0 to %M {{\n  %row_start = affine.load %row_ptrs[%i] : memref<?xi32>\n  %row_end = affine.load %row_ptrs[%i + 1] : memref<?xi32>\n  affine.for %idx = %row_start to %row_end {{\n    %k = affine.load %col_indices[%idx] : memref<?xi32>\n    %a = affine.load %values[%idx] : memref<?xf64>\n    affine.for %j = 0 to %N {{\n      %b = affine.load {}[%k, %j] : memref<?x?xf64>\n      %c = affine.load %out[%i, %j] : memref<?x?xf64>\n      %prod = arith.mulf %a, %b : f64\n      %sum = arith.addf %c, %prod : f64\n      affine.store %sum, %out[%i, %j] : memref<?x?xf64>\n    }}\n  }}\n}}", arg1));
                        } else {
                            res.push_str(&format!("affine.for %i = 0 to %M {{\n  affine.for %j = 0 to %N {{\n    affine.for %k = 0 to %K {{\n      %a = affine.load {}[%i, %k] : memref<?x?xf64>\n      %b = affine.load {}[%k, %j] : memref<?x?xf64>\n      %c = affine.load %out[%i, %j] : memref<?x?xf64>\n      %prod = arith.mulf %a, %b : f64\n      %sum = arith.addf %c, %prod : f64\n      affine.store %sum, %out[%i, %j] : memref<?x?xf64>\n    }}\n  }}\n}}", arg0, arg1));
                        }
                        res
                    } else {
                        format!("linalg.matmul ins({}, {} : tensor<?x?xf64>, tensor<?x?xf64>) outs(%out : tensor<?x?xf64>) -> tensor<?x?xf64>", arg0, arg1)
                    }
                } else {
                    let mut args = Vec::new();
                    for arg in arguments {
                        args.push(self.generate_expression(arg, sparse_vars));
                    }
                    format!("func.call @{}({}) : () -> ()", name, args.join(", "))
                }
            }
            _ => "<expr>".to_string()
        }
    }
}
