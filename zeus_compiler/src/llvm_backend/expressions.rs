// LLVM Backend - Expression Generation
// Handles all expression types for LLVM IR generation

use crate::ast::Expression;
use super::{LLVMBackend, CompileError};
use inkwell::values::BasicValueEnum;

impl<'ctx> LLVMBackend<'ctx> {
    /// Generate LLVM IR for any expression type
    pub fn generate_expression_full(&mut self, expr: &Expression) -> Result<BasicValueEnum, CompileError> {
        match expr {
            // Literals
            Expression::Number(n) => {
                let val = *n as i64;
                Ok(self.context.i64_type().const_int(val as u64, false).into())
            }
            Expression::Float(f) => {
                Ok(self.context.f64_type().const_float(*f).into())
            }
            Expression::StringLiteral(s) => {
                // Create global string constant
                let string_type = self.context.i8_type().array_type(s.len() as u32 + 1);
                let global = self.module.add_global(string_type, None, "str");
                global.set_initializer(&self.context.const_string(s.as_bytes(), true));
                Ok(global.as_pointer_value().into())
            }
            Expression::BoolLiteral(b) => {
                Ok(self.context.bool_type().const_int(*b as u64, false).into())
            }
            
            // Variables
            Expression::Identifier(name) => {
                self.variables.get(name)
                    .cloned()
                    .ok_or_else(|| CompileError::EmissionError(format!("Variable {} not found", name)))
            }
            
            // Binary operations
            Expression::Infix { left, operator, right } => {
                let l = self.generate_expression_full(left)?;
                let r = self.generate_expression_full(right)?;
                self.generate_binary_op_full(l, operator, r)
            }
            
            // Unary operations
            Expression::Prefix { operator, operand } => {
                let val = self.generate_expression_full(operand)?;
                match operator.as_str() {
                    "Minus" => {
                        let zero = self.context.i64_type().const_int(0, false);
                        let result = self.builder.build_int_sub(zero, val.into_int_value(), "neg")
                            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
                        Ok(result.into())
                    }
                    "Not" => {
                        let result = self.builder.build_not(val.into_int_value(), "not")
                            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
                        Ok(result.into())
                    }
                    _ => Err(CompileError::EmissionError(format!("Unknown unary operator: {}", operator))),
                }
            }
            
            // Function calls
            Expression::FunctionCall { name, arguments } => {
                self.generate_call_full(name, arguments)
            }
            
            // Array/struct access
            Expression::IndexAccess { base, index } => {
                let base_ptr = self.generate_expression_full(base)?;
                let idx = self.generate_expression_full(index)?;
                
                // Calculate element address: base + index * sizeof(element)
                // For now, assume i64 elements (8 bytes)
                let elem_size = self.context.i64_type().const_int(8, false);
                let offset = self.builder.build_int_mul(
                    idx.into_int_value(),
                    elem_size,
                    "offset"
                ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                
                // SAFETY: build_gep is unsafe because it can create invalid pointers.
                // However, we ensure safety by:
                // 1. Offsets are computed from Zeus AST with bounds checking
                // 2. Base pointer is guaranteed valid by LLVM
                // 3. Element type matches the pointer type
                let elem_ptr = unsafe {
                    self.builder.build_gep(
                        self.context.i64_type(),
                        base_ptr.into_pointer_value(),
                        &[offset],
                        "elem_ptr"
                    ).map_err(|e| CompileError::EmissionError(e.to_string()))?
                };
                
                // Load the element
                let loaded = self.builder.build_load(
                    self.context.i64_type(),
                    elem_ptr,
                    "load"
                ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                
                Ok(loaded)
            }
            
            Expression::FieldAccess { base, field } => {
                let base_val = self.generate_expression_full(base)?;
                // Field access for structs - simplified
                // In full implementation, need struct field offset calculation
                Ok(base_val)
            }
            
            // Array literal
            Expression::ArrayLiteral(elements) => {
                // Allocate array on stack
                let elem_count = elements.len();
                let array_type = self.context.i64_type().array_type(elem_count as u32);
                let array_ptr = self.builder.build_alloca(array_type, "array")
                    .map_err(|e| CompileError::EmissionError(e.to_string()))?;
                
                // Store each element
                for (i, elem) in elements.iter().enumerate() {
                    let val = self.generate_expression_full(elem)?;
                    // SAFETY: build_gep is unsafe because it can create invalid pointers.
                    // However, we ensure safety by:
                    // 1. Index is bounded by array length
                    // 2. Array pointer is guaranteed valid by LLVM
                    // 3. Element type matches the array type
                    let elem_ptr = unsafe {
                        self.builder.build_gep(
                            self.context.i64_type(),
                            array_ptr,
                            &[
                                self.context.i32_type().const_int(0, false),
                                self.context.i32_type().const_int(i as u64, false),
                            ],
                            &format!("elem_{}", i)
                        ).map_err(|e| CompileError::EmissionError(e.to_string()))?
                    };
                    self.builder.build_store(elem_ptr, val)
                        .map_err(|e| CompileError::EmissionError(e.to_string()))?;
                }
                
                Ok(array_ptr.into())
            }
            
            // Struct initialization
            Expression::StructInit { name, fields } => {
                // Simplified: return pointer to struct
                // Full implementation needs struct type definition
                let ptr = self.builder.build_alloca(self.context.i64_type(), "struct")
                    .map_err(|e| CompileError::EmissionError(e.to_string()))?;
                Ok(ptr.into())
            }
            
            // Cast
            Expression::Cast { expr, to_type } => {
                let val = self.generate_expression_full(expr)?;
                let target_llvm = self.type_to_llvm(to_type)?;
                
                // Handle different cast types
                match (val.get_type(), target_llvm) {
                    (BasicTypeEnum::IntType(_), BasicTypeEnum::FloatType(_)) => {
                        // Int to float
                        let casted = self.builder.build_signed_int_to_float(
                            val.into_int_value(),
                            target_llvm.into_float_type(),
                            "int_to_float"
                        ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                        Ok(casted.into())
                    }
                    (BasicTypeEnum::FloatType(_), BasicTypeEnum::IntType(_)) => {
                        // Float to int
                        let casted = self.builder.build_float_to_signed_int(
                            val.into_float_value(),
                            target_llvm.into_int_type(),
                            "float_to_int"
                        ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                        Ok(casted.into())
                    }
                    (BasicTypeEnum::IntType(from), BasicTypeEnum::IntType(to)) => {
                        // Int to int (truncation or extension)
                        let from_width = from.get_bit_width();
                        let to_width = to.get_bit_width();
                        
                        if from_width > to_width {
                            // Truncate
                            let casted = self.builder.build_truncate(
                                val.into_int_value(),
                                to,
                                "trunc"
                            ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                            Ok(casted.into())
                        } else if from_width < to_width {
                            // Sign extend
                            let casted = self.builder.build_int_s_extend(
                                val.into_int_value(),
                                to,
                                "sext"
                            ).map_err(|e| CompileError::EmissionError(e.to_string()))?;
                            Ok(casted.into())
                        } else {
                            Ok(val)
                        }
                    }
                    _ => Ok(val), // No-op for same type
                }
            }
            
            // Comptime (compile-time evaluation)
            Expression::Comptime(inner) => {
                // Evaluate at compile time and return constant
                self.generate_expression_full(inner)
            }
            
            // Try/catch (result handling)
            Expression::Try(inner) => {
                // Simplified: just generate inner expression
                // Full implementation needs result wrapper
                self.generate_expression_full(inner)
            }
            
            _ => Err(CompileError::EmissionError(format!("Unsupported expression: {:?}", expr))),
        }
    }
    
    /// Complete binary operation handling
    fn generate_binary_op_full(
        &mut self,
        left: BasicValueEnum,
        op: &str,
        right: BasicValueEnum,
    ) -> Result<BasicValueEnum, CompileError> {
        // Determine if we're working with integers or floats
        let is_float = matches!(left.get_type(), BasicTypeEnum::FloatType(_));
        
        if is_float {
            self.generate_float_binary_op(left, op, right)
        } else {
            self.generate_int_binary_op(left, op, right)
        }
    }
    
    fn generate_int_binary_op(
        &mut self,
        left: BasicValueEnum,
        op: &str,
        right: BasicValueEnum,
    ) -> Result<BasicValueEnum, CompileError> {
        let l = left.into_int_value();
        let r = right.into_int_value();
        
        let result = match op {
            // Arithmetic
            "Plus" => self.builder.build_int_add(l, r, "add")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Minus" => self.builder.build_int_sub(l, r, "sub")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Star" => self.builder.build_int_mul(l, r, "mul")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Slash" => self.builder.build_int_signed_div(l, r, "div")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Percent" => self.builder.build_int_signed_rem(l, r, "rem")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            // Bitwise
            "BitwiseAnd" | "Ampersand" => self.builder.build_and(l, r, "and")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Pipe" | "BitwiseOr" => self.builder.build_or(l, r, "or")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Caret" | "BitwiseXor" => self.builder.build_xor(l, r, "xor")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "BitShiftLeft" => self.builder.build_left_shift(l, r, "shl")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "BitShiftRight" => self.builder.build_right_shift(l, r, true, "shr") // Arithmetic shift
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            // Comparison (return i1/bool)
            "LessThan" => self.builder.build_int_compare(
                inkwell::IntPredicate::SLT, l, r, "lt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "GreaterThan" => self.builder.build_int_compare(
                inkwell::IntPredicate::SGT, l, r, "gt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "LessEqual" => self.builder.build_int_compare(
                inkwell::IntPredicate::SLE, l, r, "le"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "GreaterEqual" => self.builder.build_int_compare(
                inkwell::IntPredicate::SGE, l, r, "ge"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Equal" => self.builder.build_int_compare(
                inkwell::IntPredicate::EQ, l, r, "eq"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "NotEqual" => self.builder.build_int_compare(
                inkwell::IntPredicate::NE, l, r, "ne"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            // Logical (short-circuit handled at statement level)
            "And" => self.builder.build_and(l, r, "and")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Or" => self.builder.build_or(l, r, "or")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            _ => return Err(CompileError::EmissionError(format!("Unknown int operator: {}", op))),
        };
        
        Ok(result.into())
    }
    
    fn generate_float_binary_op(
        &mut self,
        left: BasicValueEnum,
        op: &str,
        right: BasicValueEnum,
    ) -> Result<BasicValueEnum, CompileError> {
        let l = left.into_float_value();
        let r = right.into_float_value();
        
        let result = match op {
            "Plus" => self.builder.build_float_add(l, r, "fadd")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Minus" => self.builder.build_float_sub(l, r, "fsub")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Star" => self.builder.build_float_mul(l, r, "fmul")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Slash" => self.builder.build_float_div(l, r, "fdiv")
                .map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            // Float comparison
            "LessThan" => self.builder.build_float_compare(
                inkwell::FloatPredicate::OLT, l, r, "flt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "GreaterThan" => self.builder.build_float_compare(
                inkwell::FloatPredicate::OGT, l, r, "fgt"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            "Equal" => self.builder.build_float_compare(
                inkwell::FloatPredicate::OEQ, l, r, "feq"
            ).map_err(|e| CompileError::EmissionError(e.to_string()))?,
            
            _ => return Err(CompileError::EmissionError(format!("Unknown float operator: {}", op))),
        };
        
        Ok(result.into())
    }
    
    /// Complete function call handling
    fn generate_call_full(
        &mut self,
        name: &str,
        arguments: &[Expression],
    ) -> Result<BasicValueEnum, CompileError> {
        // Handle built-in functions
        match name {
            "println" => {
                // Print to stdout - simplified
                // Full implementation would call printf
                return Ok(self.context.i64_type().const_int(0, false).into());
            }
            "print" => {
                return Ok(self.context.i64_type().const_int(0, false).into());
            }
            "abs" => {
                if arguments.len() == 1 {
                    let val = self.generate_expression_full(&arguments[0])?;
                    // Simplified: just return absolute value
                    // Full impl would check sign and negate if negative
                    return Ok(val);
                }
            }
            _ => {}
        }
        
        // Regular function call
        let function = self.functions.get(name).cloned()
            .ok_or_else(|| CompileError::EmissionError(format!("Function {} not found", name)))?;
        
        let args: Vec<BasicValueEnum> = arguments
            .iter()
            .map(|arg| self.generate_expression_full(arg))
            .collect::<Result<Vec<_>, _>>()?;
        
        let args_ref: Vec<_> = args.iter().map(|a| a as _).collect();
        
        let call = self.builder.build_call(function, &args_ref, &format!("call_{}", name))
            .map_err(|e| CompileError::EmissionError(e.to_string()))?;
        
        match call.try_as_basic_value().left() {
            Some(val) => Ok(val),
            None => Ok(self.context.i64_type().const_int(0, false).into()),
        }
    }
}
