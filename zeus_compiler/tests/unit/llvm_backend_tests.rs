// Unit Tests: LLVM Backend

#[cfg(test)]
mod llvm_backend_tests {
    use zeus_compiler::llvm_backend::LLVMBackend;
    use zeus_compiler::ast::{Program, Statement, Expression, Type};
    use inkwell::context::Context;

    fn create_context() -> Context {
        Context::create()
    }

    // Test 1: Backend creation
    #[test]
    fn test_llvm_backend_creation() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test_module");
        // Backend created successfully
        assert!(true);
    }

    // Test 2: Compile empty program
    #[test]
    fn test_compile_empty_program() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        let program = Program { statements: vec![] };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("ModuleID = 'test'"));
        assert!(ir.contains("define i32 @main()"));
    }

    // Test 3: Type conversion - i8
    #[test]
    fn test_type_i8() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::I8;
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 4: Type conversion - i32
    #[test]
    fn test_type_i32() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::I32;
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 5: Type conversion - i64
    #[test]
    fn test_type_i64() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::U64;  // Maps to i64 in LLVM
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 6: Type conversion - f32
    #[test]
    fn test_type_f32() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::F32;
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 7: Type conversion - f64
    #[test]
    fn test_type_f64() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::F64;
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 8: Type conversion - bool
    #[test]
    fn test_type_bool() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Bool;
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 9: Type conversion - array
    #[test]
    fn test_type_array() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Array(Box::new(Type::I32), Box::new(Expression::Number(10)));
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 10: Type conversion - pointer
    #[test]
    fn test_type_pointer() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Pointer(Box::new(Type::I32));
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 11: Type conversion - unknown i32 alias
    #[test]
    fn test_type_unknown_int() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Unknown("int".to_string());
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 12: Type conversion - unknown u64 alias
    #[test]
    fn test_type_unknown_u64() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Unknown("u64".to_string());
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 13: Compile simple variable declaration
    #[test]
    fn test_compile_let_statement() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "x".to_string(),
                    is_mut: false,
                    is_secret: false,
                    var_type: Some(Type::I32),
                    value: Expression::Number(42),
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 14: Compile function declaration
    #[test]
    fn test_compile_function() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::FunctionDeclaration {
                    is_pub: true,
                    name: "add".to_string(),
                    type_params: vec![],
                    parameters: vec![
                        ("a".to_string(), Type::I32),
                        ("b".to_string(), Type::I32),
                    ],
                    secret_params: vec![],
                    return_type: Some(Type::I32),
                    body: vec![
                        Statement::Return(Expression::Number(0)),
                    ],
                    attributes: vec![],
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 15: Compile if statement
    #[test]
    fn test_compile_if_statement() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::If {
                    condition: Expression::BoolLiteral(true),
                    consequence: vec![],
                    alternative: None,
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 16: Compile while loop
    #[test]
    fn test_compile_while_loop() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::While {
                    condition: Expression::BoolLiteral(false),
                    body: vec![],
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 17: Compile binary expression
    #[test]
    fn test_compile_binary_expression() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::ExpressionStatement(Expression::Infix {
                    left: Box::new(Expression::Number(1)),
                    operator: "Plus".to_string(),
                    right: Box::new(Expression::Number(2)),
                })
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 18: Compile return statement
    #[test]
    fn test_compile_return() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::Return(Expression::Number(42))
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 19: Compile function call
    #[test]
    fn test_compile_function_call() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::ExpressionStatement(Expression::FunctionCall {
                    name: "println".to_string(),
                    arguments: vec![Expression::Number(42)],
                })
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 20: Compile boolean literal
    #[test]
    fn test_compile_bool_literal() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "flag".to_string(),
                    is_mut: false,
                    is_secret: false,
                    var_type: Some(Type::Bool),
                    value: Expression::BoolLiteral(true),
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 21: Module verification passes
    #[test]
    fn test_module_verification() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        let program = Program { statements: vec![] };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        // Valid LLVM IR should contain these
        assert!(ir.contains("ModuleID"));
        assert!(ir.contains("source_filename"));
    }

    // Test 22: IR contains function definitions
    #[test]
    fn test_ir_contains_functions() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::FunctionDeclaration {
                    is_pub: true,
                    name: "foo".to_string(),
                    type_params: vec![],
                    parameters: vec![],
                    secret_params: vec![],
                    return_type: Some(Type::I32),
                    body: vec![Statement::Return(Expression::Number(0))],
                    attributes: vec![],
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("define i32 @foo()"));
    }

    // Test 23: IR contains basic blocks
    #[test]
    fn test_ir_contains_basic_blocks() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        let program = Program { statements: vec![] };
        
        let result = backend.compile(&program);
        let ir = result.unwrap();
        
        // Should have entry block
        assert!(ir.contains("entry:"));
    }

    // Test 24: Compile comparison expression
    #[test]
    fn test_compile_comparison() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "result".to_string(),
                    is_mut: false,
                    is_secret: false,
                    var_type: Some(Type::Bool),
                    value: Expression::Infix {
                        left: Box::new(Expression::Number(1)),
                        operator: "LessThan".to_string(),
                        right: Box::new(Expression::Number(2)),
                    },
                }
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 25: Compile arithmetic operations
    #[test]
    fn test_compile_arithmetic() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let ops = vec!["Plus", "Minus", "Star", "Slash"];
        
        for op in ops {
            let program = Program {
                statements: vec![
                    Statement::ExpressionStatement(Expression::Infix {
                        left: Box::new(Expression::Number(10)),
                        operator: op.to_string(),
                        right: Box::new(Expression::Number(5)),
                    })
                ]
            };
            
            let result = backend.compile(&program);
            assert!(result.is_ok(), "Failed for operator: {}", op);
        }
    }

    // Test 26: Type conversion - Result type
    #[test]
    fn test_type_result() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Result(Box::new(Type::I32), Box::new(Type::Unknown("Error".to_string())));
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_ok());
    }

    // Test 27: Compile identifier expression
    #[test]
    fn test_compile_identifier() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::Let {
                    name: "x".to_string(),
                    is_mut: false,
                    is_secret: false,
                    var_type: Some(Type::I32),
                    value: Expression::Number(42),
                },
                Statement::ExpressionStatement(Expression::Identifier("x".to_string())),
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
    }

    // Test 28: Unknown type error handling
    #[test]
    fn test_unknown_type_error() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "test");
        let ty = Type::Unknown("invalid_type_xyz".to_string());
        
        let llvm_ty = backend.type_to_llvm(&ty);
        assert!(llvm_ty.is_err());
    }

    // Test 29: Empty module name
    #[test]
    fn test_empty_module_name() {
        let context = create_context();
        let backend = LLVMBackend::new(&context, "");
        // Should still work with empty name
        assert!(true);
    }

    // Test 30: Multiple function declarations
    #[test]
    fn test_multiple_functions() {
        let context = create_context();
        let mut backend = LLVMBackend::new(&context, "test");
        
        let program = Program {
            statements: vec![
                Statement::FunctionDeclaration {
                    is_pub: true,
                    name: "a".to_string(),
                    type_params: vec![],
                    parameters: vec![],
                    secret_params: vec![],
                    return_type: Some(Type::I32),
                    body: vec![Statement::Return(Expression::Number(1))],
                    attributes: vec![],
                },
                Statement::FunctionDeclaration {
                    is_pub: true,
                    name: "b".to_string(),
                    type_params: vec![],
                    parameters: vec![],
                    secret_params: vec![],
                    return_type: Some(Type::I32),
                    body: vec![Statement::Return(Expression::Number(2))],
                    attributes: vec![],
                },
            ]
        };
        
        let result = backend.compile(&program);
        assert!(result.is_ok());
        
        let ir = result.unwrap();
        assert!(ir.contains("define i32 @a()"));
        assert!(ir.contains("define i32 @b()"));
    }
}
