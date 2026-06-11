// Unit Tests: Code Generation (Coverage Push)

#[cfg(test)]
mod codegen_tests {
    use zeus_compiler::codegen::{CCodegen, Backend};
    use zeus_compiler::ast::{Program, Statement, Expression, Type};

    // Test 1: C backend creation
    #[test]
    fn test_c_backend_creation() {
        let backend = CCodegen::new();
        assert!(backend.is_initialized());
    }

    // Test 2: Generate C header
    #[test]
    fn test_generate_header() {
        let backend = CCodegen::new();
        let header = backend.generate_header(&Program { statements: vec![] });
        assert!(header.contains("#ifndef"));
        assert!(header.contains("#define"));
    }

    // Test 3: Generate includes
    #[test]
    fn test_generate_includes() {
        let backend = CCodegen::new();
        let includes = backend.generate_includes();
        assert!(includes.contains("#include"));
    }

    // Test 4: Generate struct declaration
    #[test]
    fn test_generate_struct() {
        let backend = CCodegen::new();
        let stmt = Statement::StructDeclaration {
            name: "Point".to_string(),
            is_component: false,
            fields: vec![
                ("x".to_string(), Type::I32),
                ("y".to_string(), Type::I32),
            ],
            type_params: vec![],
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("struct Point"));
        assert!(result.contains("int32_t x"));
        assert!(result.contains("int32_t y"));
    }

    // Test 5: Generate function declaration
    #[test]
    fn test_generate_function() {
        let backend = CCodegen::new();
        let stmt = Statement::FunctionDeclaration {
            is_pub: true,
            name: "add".to_string(),
            type_params: vec![],
            parameters: vec![
                ("a".to_string(), Type::I32),
                ("b".to_string(), Type::I32),
            ],
            secret_params: vec![],
            return_type: Some(Type::I32),
            body: vec![Statement::Return(Expression::Number(0))],
            attributes: vec![],
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("int32_t add"));
        assert!(result.contains("int32_t a"));
        assert!(result.contains("int32_t b"));
    }

    // Test 6: Generate let statement
    #[test]
    fn test_generate_let() {
        let backend = CCodegen::new();
        let stmt = Statement::Let {
            name: "x".to_string(),
            is_mut: false,
            is_secret: false,
            var_type: Some(Type::I32),
            value: Expression::Number(42),
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("int32_t x"));
        assert!(result.contains("42"));
    }

    // Test 7: Generate return statement
    #[test]
    fn test_generate_return() {
        let backend = CCodegen::new();
        let stmt = Statement::Return(Expression::Number(42));
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("return 42"));
    }

    // Test 8: Generate if statement
    #[test]
    fn test_generate_if() {
        let backend = CCodegen::new();
        let stmt = Statement::If {
            condition: Expression::BoolLiteral(true),
            consequence: vec![],
            alternative: None,
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("if"));
        assert!(result.contains("true"));
    }

    // Test 9: Generate while loop
    #[test]
    fn test_generate_while() {
        let backend = CCodegen::new();
        let stmt = Statement::While {
            condition: Expression::BoolLiteral(false),
            body: vec![],
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("while"));
    }

    // Test 10: Generate binary expression
    #[test]
    fn test_generate_binary() {
        let backend = CCodegen::new();
        let expr = Expression::Infix {
            left: Box::new(Expression::Number(1)),
            operator: "Plus".to_string(),
            right: Box::new(Expression::Number(2)),
        };
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("1 + 2"));
    }

    // Test 11: Type to C type mapping
    #[test]
    fn test_type_mapping_i32() {
        let backend = CCodegen::new();
        assert_eq!(backend.type_to_c(&Type::I32), "int32_t");
    }

    // Test 12: Type to C type mapping - i64
    #[test]
    fn test_type_mapping_i64() {
        let backend = CCodegen::new();
        assert_eq!(backend.type_to_c(&Type::U64), "uint64_t");
    }

    // Test 13: Type to C type mapping - bool
    #[test]
    fn test_type_mapping_bool() {
        let backend = CCodegen::new();
        assert_eq!(backend.type_to_c(&Type::Bool), "bool");
    }

    // Test 14: Generate function call
    #[test]
    fn test_generate_call() {
        let backend = CCodegen::new();
        let expr = Expression::FunctionCall {
            name: "foo".to_string(),
            arguments: vec![Expression::Number(1), Expression::Number(2)],
        };
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("foo(1, 2)"));
    }

    // Test 15: Generate identifier
    #[test]
    fn test_generate_identifier() {
        let backend = CCodegen::new();
        let expr = Expression::Identifier("x".to_string());
        
        let result = backend.generate_expression(&expr);
        assert_eq!(result, "x");
    }

    // Test 16: Generate number literal
    #[test]
    fn test_generate_number() {
        let backend = CCodegen::new();
        let expr = Expression::Number(42);
        
        let result = backend.generate_expression(&expr);
        assert_eq!(result, "42");
    }

    // Test 17: Generate boolean literal
    #[test]
    fn test_generate_bool() {
        let backend = CCodegen::new();
        let expr = Expression::BoolLiteral(true);
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("true"));
    }

    // Test 18: Generate secret variable
    #[test]
    fn test_generate_secret_let() {
        let backend = CCodegen::new();
        let stmt = Statement::Let {
            name: "secret_key".to_string(),
            is_mut: false,
            is_secret: true,
            var_type: Some(Type::Array(Box::new(Type::I8), Box::new(Expression::Number(32)))),
            value: Expression::ArrayLiteral(vec![]),
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("ZEUS_SECRET"));
        assert!(result.contains("secret_key"));
    }

    // Test 19: Generate extern function
    #[test]
    fn test_generate_extern() {
        let backend = CCodegen::new();
        let stmt = Statement::ExternFunctionDeclaration {
            name: "external_fn".to_string(),
            parameters: vec![("x".to_string(), Type::I32)],
            return_type: Some(Type::I32),
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("extern"));
        assert!(result.contains("external_fn"));
    }

    // Test 20: Generate assertion
    #[test]
    fn test_generate_assert() {
        let backend = CCodegen::new();
        let stmt = Statement::Assert(Expression::BoolLiteral(true));
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("assert"));
    }

    // Test 21: Generate main function wrapper
    #[test]
    fn test_generate_main_wrapper() {
        let backend = CCodegen::new();
        let program = Program { statements: vec![] };
        
        let source = backend.generate_source(&program);
        assert!(source.contains("int main("));
    }

    // Test 22: Generate with panic
    #[test]
    fn test_generate_panic() {
        let backend = CCodegen::new();
        let stmt = Statement::Panic("error message".to_string());
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("fprintf") || result.contains("exit"));
    }

    // Test 23: Generate array type
    #[test]
    fn test_generate_array_type() {
        let backend = CCodegen::new();
        let ty = Type::Array(Box::new(Type::I32), Box::new(Expression::Number(10)));
        
        let result = backend.type_to_c(&ty);
        assert!(result.contains("int32_t"));
        assert!(result.contains("[10]"));
    }

    // Test 24: Generate pointer type
    #[test]
    fn test_generate_pointer_type() {
        let backend = CCodegen::new();
        let ty = Type::Pointer(Box::new(Type::I32));
        
        let result = backend.type_to_c(&ty);
        assert!(result.contains("int32_t*"));
    }

    // Test 25: Generate comparison operators
    #[test]
    fn test_generate_comparison() {
        let backend = CCodegen::new();
        
        let ops = vec![
            ("LessThan", "<"),
            ("GreaterThan", ">"),
            ("Equal", "=="),
            ("NotEqual", "!="),
        ];
        
        for (op_name, expected) in ops {
            let expr = Expression::Infix {
                left: Box::new(Expression::Number(1)),
                operator: op_name.to_string(),
                right: Box::new(Expression::Number(2)),
            };
            
            let result = backend.generate_expression(&expr);
            assert!(
                result.contains(expected),
                "Expected {} in {}", expected, result
            );
        }
    }

    // Test 26: Generate cast expression
    #[test]
    fn test_generate_cast() {
        let backend = CCodegen::new();
        let expr = Expression::Cast {
            expr: Box::new(Expression::Number(42)),
            to_type: Type::F64,
        };
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("double") || result.contains("(double)"));
    }

    // Test 27: Generate prefix expression
    #[test]
    fn test_generate_prefix() {
        let backend = CCodegen::new();
        let expr = Expression::Prefix {
            operator: "Minus".to_string(),
            operand: Box::new(Expression::Number(5)),
        };
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("-5"));
    }

    // Test 28: Generate for loop
    #[test]
    fn test_generate_for() {
        let backend = CCodegen::new();
        let stmt = Statement::For {
            iterator: "i".to_string(),
            start: Expression::Number(0),
            end: Expression::Number(10),
            body: vec![],
        };
        
        let result = backend.generate_statement(&stmt);
        assert!(result.contains("for"));
        assert!(result.contains("i"));
    }

    // Test 29: Generate array literal
    #[test]
    fn test_generate_array_literal() {
        let backend = CCodegen::new();
        let expr = Expression::ArrayLiteral(vec![
            Expression::Number(1),
            Expression::Number(2),
            Expression::Number(3),
        ]);
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("{"));
        assert!(result.contains("1"));
        assert!(result.contains("3"));
        assert!(result.contains("}"));
    }

    // Test 30: Generate string literal
    #[test]
    fn test_generate_string() {
        let backend = CCodegen::new();
        let expr = Expression::StringLiteral("hello".to_string());
        
        let result = backend.generate_expression(&expr);
        assert!(result.contains("\"hello\""));
    }
}
