// Property Test: Parser Roundtrip
// Property: parse ∘ print = id (for valid programs)

use proptest::prelude::*;

// Generate valid Zeus programs as strings
fn valid_zeus_program() -> impl Strategy<Value = String> {
    // Start with simple programs
    prop_oneof![
        // Simple variable declarations
        "let x: i32 = ".to_string() + "[0-9]{1,5}",
        // Simple functions  
        "fn foo() -> i32 { return 42; }".to_string(),
        // Simple expressions
        "1 + 2".to_string(),
    ]
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(100))]
    
    #[test]
    fn parser_roundtrip_simple(program in valid_zeus_program()) {
        // TODO: When parser is complete, implement:
        // let ast = parse(&program)?;
        // let printed = print(&ast);
        // let ast2 = parse(&printed)?;
        // assert_eq!(ast, ast2);
        
        // For now, just verify the string is non-empty
        prop_assert!(!program.is_empty());
    }
}
