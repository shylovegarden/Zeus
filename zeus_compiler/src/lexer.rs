#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    Let,
    Mut,
    Fn,
    Return,
    If,
    Else,
    For,
    In,
    Parallel,
    Tensor,
    Safe,
    Unsafe,
    Target,
    Proof,
    Assert,
    Pub,
    Import,
    Struct,
    Test,
    Extern,
    Panic,
    Secret,
    SafeState,
    Component,
    Enclave,
    Sparse,
    Comptime,
    Verify,
    Adaptive,
    Cluster,

    // Types
    I8,
    I32,
    U64,
    F32,
    F64,
    Bool,

    // Identifiers and Literals
    Identifier(String),
    Number(f64),
    StringLiteral(String),

    // Symbols & Operators
    Assign,       // =
    Plus,         // +
    Minus,        // -
    Star,         // *
    Slash,        // /
    Dot,          // .
    Arrow,        // ->
    DoubleDot,    // ..
    Pipe,         // |
    BitwiseAnd,   // &
    BitShiftLeft, // <<
    BitShiftRight,// >>

    // Brackets
    LParen,       // (
    RParen,       // )
    LBrace,       // {
    RBrace,       // }
    LBracket,     // [
    RBracket,     // ]
    
    // Comparators
    Equal,        // ==
    NotEqual,     // !=
    GreaterThan,  // >
    LessThan,     // <
    GreaterEqual, // >=
    LessEqual,    // <=

    // Punctuation
    Colon,        // :
    Comma,        // ,
    Question,     // ?
    Semicolon,    // ;
    AtSign,       // @

    Eof,
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
    read_position: usize,
    ch: Option<char>,
    pub line_number: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input,
            position: 0,
            read_position: 0,
            ch: None,
            line_number: 1,
        };
        lexer.read_char();
        lexer
    }

    fn read_char(&mut self) {
        if let Some('\n') = self.ch {
            self.line_number += 1;
        }
        if self.read_position >= self.input.len() {
            self.ch = None;
        } else {
            self.ch = self.input.chars().nth(self.read_position);
        }
        self.position = self.read_position;
        self.read_position += 1;
    }

    fn peek_char(&self) -> Option<char> {
        if self.read_position >= self.input.len() {
            None
        } else {
            self.input.chars().nth(self.read_position)
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(ch) = self.ch {
            if ch.is_whitespace() {
                self.read_char();
            } else {
                break;
            }
        }
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        // Skip comments //
        while let Some(ch) = self.ch {
            if ch == '/' && self.peek_char() == Some('/') {
                while let Some(c) = self.ch {
                    if c == '\n' {
                        break;
                    }
                    self.read_char();
                }
                self.skip_whitespace();
            } else {
                break;
            }
        }

        let token = match self.ch {
            Some('=') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::Equal
                } else {
                    Token::Assign
                }
            }
            Some('+') => Token::Plus,
            Some('-') => {
                if self.peek_char() == Some('>') {
                    self.read_char();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }
            Some('*') => Token::Star,
            Some('/') => Token::Slash,
            Some('>') => {
                if self.peek_char() == Some('>') {
                    self.read_char();
                    Token::BitShiftRight
                } else if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::GreaterEqual
                } else {
                    Token::GreaterThan
                }
            }
            Some('<') => {
                if self.peek_char() == Some('<') {
                    self.read_char();
                    Token::BitShiftLeft
                } else if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::LessEqual
                } else {
                    Token::LessThan
                }
            }
            Some('&') => Token::BitwiseAnd,
            Some('(') => Token::LParen,
            Some(')') => Token::RParen,
            Some('{') => Token::LBrace,
            Some('}') => Token::RBrace,
            Some('[') => Token::LBracket,
            Some(']') => Token::RBracket,
            Some(',') => Token::Comma,
            Some(';') => Token::Semicolon,
            Some(':') => Token::Colon,
            Some('?') => Token::Question,
            Some('|') => Token::Pipe,
            Some('@') => Token::AtSign,
            Some('.') => {
                if self.peek_char() == Some('.') {
                    self.read_char();
                    Token::DoubleDot
                } else {
                    Token::Dot
                }
            }
            Some('"') => {
                let string_lit = self.read_string();
                return Token::StringLiteral(string_lit);
            }
            Some(ch) if ch.is_alphabetic() || ch == '_' => {
                let ident = self.read_identifier();
                return match ident.as_str() {
                    "let" => Token::Let,
                    "mut" => Token::Mut,
                    "fn" => Token::Fn,
                    "return" => Token::Return,
                    "if" => Token::If,
                    "else" => Token::Else,
                    "for" => Token::For,
                    "in" => Token::In,
                    "parallel" => Token::Parallel,
                    "tensor" => Token::Tensor,
                    "safe" => Token::Safe,
                    "unsafe" => Token::Unsafe,
                    "target" => Token::Target,
                    "proof" => Token::Proof,
                    "assert" => Token::Assert,
                    "pub" => Token::Pub,
                    "import" => Token::Import,
                    "struct" => Token::Struct,
                    "test" => Token::Test,
                    "extern" => Token::Extern,
                    "panic" => Token::Panic,
                    "secret" => Token::Secret,
                    "safestate" => Token::SafeState,
                    "component" => Token::Component,
                    "enclave" => Token::Enclave,
                    "sparse" => Token::Sparse,
                    "comptime" => Token::Comptime,
                    "verify" => Token::Verify,
                    "adaptive" => Token::Adaptive,
                    "cluster" => Token::Cluster,
                    "i8" => Token::I8,
                    "i32" => Token::I32,
                    "u64" => Token::U64,
                    "f32" => Token::F32,
                    "f64" => Token::F64,
                    "bool" => Token::Bool,
                    _ => Token::Identifier(ident),
                };
            }
            Some(ch) if ch.is_digit(10) => {
                return Token::Number(self.read_number());
            }
            None => return Token::Eof,
            _ => Token::Identifier(self.ch.unwrap().to_string()), // catch-all for errors currently
        };

        self.read_char();
        token
    }

    fn read_identifier(&mut self) -> String {
        let position = self.position;
        while let Some(ch) = self.ch {
            if ch.is_alphanumeric() || ch == '_' {
                self.read_char();
            } else {
                break;
            }
        }
        self.input[position..self.position].to_string()
    }

    fn read_number(&mut self) -> f64 {
        let position = self.position;
        while let Some(ch) = self.ch {
            if ch.is_digit(10) {
                self.read_char();
            } else if ch == '.' {
                if self.peek_char() == Some('.') {
                    break;
                }
                self.read_char();
            } else {
                break;
            }
        }
        self.input[position..self.position].parse().unwrap_or(0.0)
    }

    fn read_string(&mut self) -> String {
        let position = self.position + 1; // skip the opening quote
        self.read_char();
        while let Some(ch) = self.ch {
            if ch == '"' {
                break;
            }
            self.read_char();
        }
        let result = self.input[position..self.position].to_string();
        self.read_char(); // skip closing quote
        result
    }
}
