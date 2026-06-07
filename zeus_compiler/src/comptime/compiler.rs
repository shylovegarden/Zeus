use crate::ast::{Expression, Statement};
use crate::vm::opcode::Opcode;
use std::collections::HashMap;

pub struct BytecodeCompiler {
    pub bytecode: Vec<u8>,
    pub constants: Vec<f64>,
    pub var_map: HashMap<String, usize>,
}

impl BytecodeCompiler {
    pub fn new() -> Self {
        BytecodeCompiler {
            bytecode: Vec::new(),
            constants: Vec::new(),
            var_map: HashMap::new(),
        }
    }

    pub fn compile_block(&mut self, statements: &[Statement]) {
        for stmt in statements {
            self.compile_statement(stmt);
        }
    }

    pub fn compile_statement(&mut self, stmt: &Statement) {
        match stmt {
            Statement::Let { name, value, .. } => {
                self.compile_expression(value);
                let var_id = self.var_map.len();
                self.var_map.insert(name.clone(), var_id);
                self.bytecode.push(Opcode::OpSetVar as u8);
                self.bytecode.push(var_id as u8);
            }
            Statement::ExpressionStatement(expr) => {
                self.compile_expression(expr);
                self.bytecode.push(Opcode::OpPop as u8);
            }
            Statement::Return(expr) => {
                self.compile_expression(expr);
                self.bytecode.push(Opcode::OpReturn as u8);
            }
            // For loops and other constructs will be added here
            _ => panic!("Unsupported comptime statement: {:?}", stmt),
        }
    }

    pub fn compile_expression(&mut self, expr: &Expression) {
        match expr {
            Expression::Number(val) => {
                let const_id = self.constants.len();
                self.constants.push(*val);
                self.bytecode.push(Opcode::OpConstant as u8);
                self.bytecode.push(const_id as u8);
            }
            Expression::Identifier(name) => {
                if let Some(&var_id) = self.var_map.get(name) {
                    self.bytecode.push(Opcode::OpGetVar as u8);
                    self.bytecode.push(var_id as u8);
                } else {
                    panic!("Undefined comptime variable: {}", name);
                }
            }
            Expression::Infix { left, operator, right } => {
                self.compile_expression(left);
                self.compile_expression(right);
                
                match operator.as_str() {
                    "Plus" => self.bytecode.push(Opcode::OpAdd as u8),
                    "Minus" => self.bytecode.push(Opcode::OpSubtract as u8),
                    "Star" => self.bytecode.push(Opcode::OpMultiply as u8),
                    "Slash" => self.bytecode.push(Opcode::OpDivide as u8),
                    _ => panic!("Unsupported comptime operator: {}", operator),
                }
            }
            _ => panic!("Unsupported comptime expression: {:?}", expr),
        }
    }
}
