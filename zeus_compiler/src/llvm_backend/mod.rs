// LLVM Backend for Zeus
// Generates LLVM IR for native performance and GPU targets

use crate::ast::{Program, Statement, Expression, Type, FunctionAttribute};
use crate::backend::{Backend, Artifact, CompileError};
use inkwell::context::Context;
use inkwell::builder::Builder;
use inkwell::module::Module;
use inkwell::values::{FunctionValue, BasicValueEnum};
use inkwell::types::{BasicTypeEnum, FunctionType};
use inkwell::OptimizationLevel;
use std::collections::HashMap;

pub struct LLVMBackend<'ctx> {
    context: &'ctx Context,
    module: Module<'ctx>,
    builder: Builder<'ctx>,
    functions: HashMap<String, FunctionValue<'ctx>>,
    variables: HashMap<String, BasicValueEnum<'ctx>>,
}

impl<'ctx> LLVMBackend<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();
        
        LLVMBackend {
            context,
            module,
            builder,
            functions: HashMap::new(),
            variables: HashMap::new(),
        }
    }
    
    /// Compile Zeus program to LLVM IR
    pub fn compile(&mut self, program: &Program) -> Result<String, CompileError> {
        // Generate function declarations first
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { 
                name, 
                parameters, 
                return_type, 
                .. 
            } = stmt {
                self.declare_function(name, parameters, return_type)?;
            }
        }
        
        // Generate function bodies
        for stmt in &program.statements {
            if let Statement::FunctionDeclaration { 
                name,
                parameters,
                body,
                return_type,
                .. 
            } = stmt {
                self.generate_function_body(name, parameters, body, return_type)?;
            }
        }
        
        // Generate main if no functions exist
        if self.functions.is_empty() {
            self.generate_main(&program.statements)?;
        }
        
        // Verify the module
        self.module.verify().map_err(|e| {
            CompileError::EmissionError(format!("LLVM verification failed: {}", e))
        })?;
        
        // Return LLVM IR as string
        Ok(self.module.print_to_string().to_string())
    }
    
    fn declare_function(
        &mut self,
        name: &str,
        parameters: &[(String, Type)],
        return_type: &Option<Type>,
    ) -> Result<(), CompileError> {
        let param_types: Vec<BasicTypeEnum> = parameters
            .iter()
            .map(|(_, ty)| self.type_to_llvm(ty))
            .collect::<Result<Vec<_>, _>>()?;
        
        let fn_type = match return_type {
            Some(ty) => {
                let ret_ty = self.type_to_llvm(ty)?;
                ret_ty.fn_type(&param_types, false)
            }
            None => self.context.void_type().fn_type(&param_types, false),
        };
        
        let function = self.module.add_function(name, fn_type, None);
        self.functions.insert(name.to_string(), function);
        
        Ok(())
    }
    
    fn generate_function_body(
        &mut self,
        name: &str,
        parameters: &[(String, Type)],
        body: &[Statement],
        _return_type: &Option<Type>,
    ) -> Result<(), CompileError> {
        let function = self.functions.get(name).cloned()
            .ok_or_else(|| CompileError::EmissionError(format!("Function {} not found", name)))?;
        
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);
        
        // Clear variables for new function scope
        self.variables.clear();
        
        // Store parameters as variables
        for (i, (param_name, _)) in parameters.iter().enumerate() {
            let param = function.get_nth_param(i as u32)
                .ok_or_else(|| CompileError::EmissionError("Parameter not found".to_string()))?;
            self.variables.insert(param_name.clone(), param);
        }
        
        // Generate statements
        for stmt in body {
            self.generate_statement(stmt)?;
        }
        
        // Add return if not present
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_return(None)
                .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        }
        
        Ok(())
    }
    
    fn generate_statement(&mut self, stmt: &Statement) -> Result<(), CompileError> {
        match stmt {
            Statement::Let { name, value, .. } => {
                let val = self.generate_expression(value)?;
                self.variables.insert(name.clone(), val);
            }
            Statement::ExpressionStatement(expr) => {
                self.generate_expression(expr)?;
            }
            Statement::Return(expr) => {
                let val = self.generate_expression(expr)?;
                self.builder.build_return(Some(&val))
                    .map_err(|e| CompileError::EmissionError(e.to_string()))?;
            }
            Statement::If { condition, consequence, alternative } => {
                self.generate_if(condition, consequence, alternative.as_ref())?;
            }
            Statement::While { condition, body } => {
                self.generate_while(condition, body)?;
            }
            _ => {}
        }
        
        Ok(())
    }
    
    fn generate_if(
        &mut self,
        condition: &Expression,
        consequence: &[Statement],
        alternative: Option<&Vec<Statement>>,
    ) -> Result<(), CompileError> {
        let cond_val = self.generate_expression(condition)?;
        
        let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
        let then_block = self.context.append_basic_block(function, "then");
        let else_block = self.context.append_basic_block(function, "else");
        let merge_block = self.context.append_basic_block(function, "ifcont");
        
        self.builder.build_conditional_branch(
            cond_val.into_int_value(),
            then_block,
            else_block
        ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        // Then block
        self.builder.position_at_end(then_block);
        for stmt in consequence {
            self.generate_statement(stmt)?;
        }
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(merge_block)
                .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        }
        
        // Else block
        self.builder.position_at_end(else_block);
        if let Some(alt) = alternative {
            for stmt in alt {
                self.generate_statement(stmt)?;
            }
        }
        if !self.builder.get_insert_block().unwrap().get_terminator().is_some() {
            self.builder.build_unconditional_branch(merge_block)
                .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        }
        
        // Merge block
        self.builder.position_at_end(merge_block);
        
        Ok(())
    }
    
    fn generate_while(
        &mut self,
        condition: &Expression,
        body: &[Statement],
    ) -> Result<(), CompileError> {
        let function = self.builder.get_insert_block().unwrap().get_parent().unwrap();
        let loop_block = self.context.append_basic_block(function, "loop");
        let body_block = self.context.append_basic_block(function, "loopbody");
        let end_block = self.context.append_basic_block(function, "loopend");
        
        // Branch to loop header
        self.builder.build_unconditional_branch(loop_block)
            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        // Loop header: check condition
        self.builder.position_at_end(loop_block);
        let cond_val = self.generate_expression(condition)?;
        self.builder.build_conditional_branch(
            cond_val.into_int_value(),
            body_block,
            end_block
        ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        // Loop body
        self.builder.position_at_end(body_block);
        for stmt in body {
            self.generate_statement(stmt)?;
        }
        self.builder.build_unconditional_branch(loop_block)
            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        // Continue after loop
        self.builder.position_at_end(end_block);
        
        Ok(())
    }
    
    fn generate_expression(&mut self, expr: &Expression) -> Result<BasicValueEnum, CompileError> {
        match expr {
            Expression::Number(n) => {
                let val = *n as i64;
                Ok(self.context.i64_type().const_int(val as u64, false).into())
            }
            Expression::Identifier(name) => {
                self.variables.get(name)
                    .cloned()
                    .ok_or_else(|| CompileError::EmissionError(format!("Variable {} not found", name)))
            }
            Expression::Infix { left, operator, right } => {
                let l = self.generate_expression(left)?;
                let r = self.generate_expression(right)?;
                self.generate_binary_op(l, operator, r)
            }
            Expression::FunctionCall { name, arguments } => {
                self.generate_call(name, arguments)
            }
            Expression::BoolLiteral(b) => {
                Ok(self.context.bool_type().const_int(*b as u64, false).into())
            }
            _ => Err(CompileError::EmissionError("Unsupported expression".to_string())),
        }
    }
    
    fn generate_binary_op(
        &mut self,
        left: BasicValueEnum,
        op: &str,
        right: BasicValueEnum,
    ) -> Result<BasicValueEnum, CompileError> {
        let l = left.into_int_value();
        let r = right.into_int_value();
        
        let result = match op {
            "Plus" => self.builder.build_int_add(l, r, "add")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Minus" => self.builder.build_int_sub(l, r, "sub")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Star" => self.builder.build_int_mul(l, r, "mul")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Slash" => self.builder.build_int_signed_div(l, r, "div")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "LessThan" => self.builder.build_int_compare(
                inkwell::IntPredicate::SLT, l, r, "lt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "GreaterThan" => self.builder.build_int_compare(
                inkwell::IntPredicate::SGT, l, r, "gt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Equal" => self.builder.build_int_compare(
                inkwell::IntPredicate::EQ, l, r, "eq"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            _ => return Err(CompileError::EmissionError(format!("Unknown operator: {}", op))),
        };
        
        Ok(result.into())
    }
    
    fn generate_call(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<BasicValueEnum, CompileError> {
        // Built-in functions
        if name == "println" {
            // For now, return 0 (placeholder)
            return Ok(self.context.i64_type().const_int(0, false).into());
        }
        
        let function = self.functions.get(name).cloned()
            .ok_or_else(|| CompileError::EmissionError(format!("Function {} not found", name)))?;
        
        let args: Vec<BasicValueEnum> = arguments
            .iter()
            .map(|arg| self.generate_expression(arg))
            .collect::<Result<Vec<_>, _>>()?;
        
        let args_ref: Vec<_> = args.iter().map(|a| a as _).collect();
        
        let call = self.builder.build_call(function, &args_ref, &format!("call_{}", name))
            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        match call.try_as_basic_value().left() {
            Some(val) => Ok(val),
            None => Ok(self.context.i64_type().const_int(0, false).into()),
        }
    }
    
    fn type_to_llvm(&self, ty: &Type) -> Result<BasicTypeEnum, CompileError> {
        match ty {
            Type::I8 => Ok(self.context.i8_type().into()),
            Type::I32 => Ok(self.context.i32_type().into()),
            Type::U64 => Ok(self.context.i64_type().into()),
            Type::F32 => Ok(self.context.f32_type().into()),
            Type::F64 => Ok(self.context.f64_type().into()),
            Type::Bool => Ok(self.context.bool_type().into()),
            _ => Err(CompileError::EmissionError(format!("Unsupported type: {:?}", ty))),
        }
    }
    
    fn generate_main(&mut self, statements: &[Statement]) -> Result<(), CompileError> {
        let fn_type = self.context.i32_type().fn_type(&[], false);
        let function = self.module.add_function("main", fn_type, None);
        let basic_block = self.context.append_basic_block(function, "entry");
        self.builder.position_at_end(basic_block);
        
        for stmt in statements {
            self.generate_statement(stmt)?;
        }
        
        self.builder.build_return(Some(&self.context.i32_type().const_int(0, false)))
            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        Ok(())
    }
    
    /// Optimize the module
    pub fn optimize(&self, level: OptimizationLevel) {
        // Run LLVM optimization passes
        let pass_manager = inkwell::passes::PassManager::create(&self.module);
        
        match level {
            OptimizationLevel::None => {}
            OptimizationLevel::Less => {
                pass_manager.add_instruction_combining_pass();
            }
            OptimizationLevel::Default => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
            }
            OptimizationLevel::Aggressive => {
                pass_manager.add_instruction_combining_pass();
                pass_manager.add_reassociate_pass();
                pass_manager.add_gvn_pass();
                pass_manager.add_cfg_simplification_pass();
                pass_manager.add_basic_alias_analysis_pass();
                pass_manager.add_promote_memory_to_register_pass();
                pass_manager.add_dead_store_elimination_pass();
                pass_manager.add_aggressive_dce_pass();
            }
        }
        
        pass_manager.run_on(&self.module);
    }
}

/// Compile to native object code
pub fn compile_to_object(llvm_ir: &str, target: &str) -> Result<Vec<u8>, CompileError> {
    // Use LLVM's JIT or save to file
    // This would integrate with LLVM's target machine
    Ok(llvm_ir.as_bytes().to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Program;
    
    #[test]
    fn test_llvm_basic() {
        let context = Context::create();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program { statements: vec![] };
        let result = backend.compile(&program);
        
        assert!(result.is_ok());
        let ir = result.unwrap();
        assert!(ir.contains("; ModuleID = 'test'"));
    }
}
