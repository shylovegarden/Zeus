#![no_main]

use libfuzzer_sys::fuzz_target;
use zeus_compiler::lexer::Lexer;
use zeus_compiler::parser::Parser;

// Fuzz target for Zeus lexer and parser
// Tests security limits (input size, recursion depth, AST node count)
// and ensures no panics on malformed input
fuzz_target!(|data: &[u8]| {
    // Convert bytes to string (ignore invalid UTF-8)
    let input = match std::str::from_utf8(data) {
        Ok(s) => s,
        Err(_) => return, // Skip invalid UTF-8
    };

    // Test lexer with fuzzed input
    let mut lexer = Lexer::new(input);
    let _tokens = lexer.tokenize();

    // Test parser with fuzzed input (if lexer succeeded)
    if lexer.errors.is_empty() {
        let mut parser = Parser::new(&lexer);
        let _program = parser.parse_program();

        // Parser should not panic on any input
        // Security limits should prevent DoS:
        // - MAX_INPUT_SIZE: 10MB
        // - MAX_LINE_LENGTH: 10,000 chars
        // - MAX_RECURSION_DEPTH: 1000
        // - MAX_AST_NODES: 100,000
    }
});
