#![allow(clippy::collapsible_if, clippy::collapsible_else_if, clippy::map_unwrap_or, clippy::needless_bool)]
use crate::ast::{Expression, Statement};
use crate::vm::opcode::Opcode;
use std::collections::HashMap;

pub struct BytecodeCompiler {
    pub bytecode: Vec<u8>,
    pub constants: Vec<f64>,
    pub var_map: HashMap<String, usize>,
}

impl Default for BytecodeCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        BytecodeCompiler {
            bytecode: Vec::new(),
            constants: Vec::new(),
            var_map: HashMap::new(),
        }
    }

    /// Compile a block of statements to comptime bytecode.
    /// Returns Err with a human-readable reason when the block contains a
    /// construct the comptime VM cannot evaluate. Callers treat Err as
    /// "not statically foldable" and fall back to runtime evaluation -- the
    /// compiler must never panic on user input.
    pub fn compile_block(&mut self, statements: &[Statement]) -> Result<(), String> {
        for stmt in statements {
            self.compile_statement(stmt)?;
        }
        Ok(())
    }

    pub fn compile_statement(&mut self, stmt: &Statement) -> Result<(), String> {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.compile_expression(value)?;
                let var_id = self.var_map.len();
                self.var_map.insert(name.clone(), var_id);
                self.bytecode.push(Opcode::OpSetVar as u8);
                self.bytecode.push(var_id as u8);
                Ok(())
            }
            Statement::ExpressionStatement(expr) => {
                self.compile_expression(expr)?;
                self.bytecode.push(Opcode::OpPop as u8);
                Ok(())
            }
            Statement::Return(expr) => {
                self.compile_expression(expr)?;
                self.bytecode.push(Opcode::OpReturn as u8);
                Ok(())
            }
            // Loops, conditionals, and calls are not yet supported in the
            // comptime VM. Report cleanly instead of crashing.
            other => Err(format!(
                "unsupported comptime statement: {:?}",
                std::mem::discriminant(other)
            )),
        }
    }

    pub fn compile_expression(&mut self, expr: &Expression) -> Result<(), String> {
        match expr {
            Expression::Number(val) => {
                let const_id = self.constants.len();
                self.constants.push(*val);
                self.bytecode.push(Opcode::OpConstant as u8);
                self.bytecode.push(const_id as u8);
                Ok(())
            }
            Expression::Identifier(name) => {
                if let Some(&var_id) = self.var_map.get(name) {
                    self.bytecode.push(Opcode::OpGetVar as u8);
                    self.bytecode.push(var_id as u8);
                    Ok(())
                } else {
                    Err(format!("undefined comptime variable: {}", name))
                }
            }
            Expression::Infix { left, operator, right } => {
                self.compile_expression(left)?;
                self.compile_expression(right)?;
                match operator.as_str() {
                    "Plus" => self.bytecode.push(Opcode::OpAdd as u8),
                    "Minus" => self.bytecode.push(Opcode::OpSubtract as u8),
                    "Star" => self.bytecode.push(Opcode::OpMultiply as u8),
                    "Slash" => self.bytecode.push(Opcode::OpDivide as u8),
                    op => return Err(format!("unsupported comptime operator: {}", op)),
                }
                Ok(())
            }
            other => Err(format!(
                "unsupported comptime expression: {:?}",
                std::mem::discriminant(other)
            )),
        }
    }
}
