fn main() {
    let input = std::fs::read_to_string(""C:\\Zeus\\Zeus\\tests\\failed\\dynamic_heap_error.zs"").unwrap();
    let mut lexer = zeus_compiler::lexer::Lexer::new(&input);
    loop {
        let t = lexer.next_token();
        println!(""{:?}"", t);
        if t == zeus_compiler::lexer::Token::Eof { break; }
    }
}
