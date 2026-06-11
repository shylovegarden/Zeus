/// Source position: 1-based line and column numbers.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Span {
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(line: usize, col: usize) -> Self { Span { line, col } }
}

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
    While,
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
    Enum,
    Match,

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
    Percent,      // %
    PlusAssign,    // +=
    MinusAssign,   // -=
    StarAssign,    // *=
    SlashAssign,   // /=
    PercentAssign, // %=
    Dot,          // .
    Arrow,        // ->
    DoubleDot,    // ..
    Pipe,         // |
    BitwiseAnd,   // &
    And,          // &&
    Or,           // ||
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
    Bang,         // ! (logical not)
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
    FatArrow,     // =>
    DoubleColon,  // ::

    Eof,
}

pub struct Lexer<'a> {
    #[allow(dead_code)]
    input: &'a str,
    chars: Vec<char>,
    pub errors: Vec<String>,
    position: usize,
    read_position: usize,
    ch: Option<char>,
    pub line_number: usize,
    /// Current column of `ch` (1-based).
    col: usize,
    /// Line/column snapshotted at the start of the most recently consumed token.
    pub token_line: usize,
    pub token_col: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let mut lexer = Lexer {
            input,
            chars: input.chars().collect(),
            errors: Vec::new(),
            position: 0,
            read_position: 0,
            ch: None,
            line_number: 1,
            col: 0,
            token_line: 1,
            token_col: 1,
        };
        lexer.read_char();
        lexer
    }

    /// Return the span (line, col) of the token most recently started by `next_token`.
    pub fn span(&self) -> Span {
        Span::new(self.token_line, self.token_col)
    }

    fn read_char(&mut self) {
        if let Some('\n') = self.ch {
            self.line_number += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        if self.read_position >= self.chars.len() {
            self.ch = None;
        } else {
            self.ch = Some(self.chars[self.read_position]);
        }
        self.position = self.read_position;
        self.read_position += 1;
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.read_position).copied()
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

        // Snapshot position of this token's first character.
        self.token_line = self.line_number;
        self.token_col = self.col;

        let token = match self.ch {
            Some('=') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::Equal
                } else if self.peek_char() == Some('>') {
                    self.read_char();
                    Token::FatArrow
                } else {
                    Token::Assign
                }
            }
            Some('+') => { if self.peek_char() == Some('=') { self.read_char(); Token::PlusAssign } else { Token::Plus } }
            Some('-') => {
                if self.peek_char() == Some('>') { self.read_char(); Token::Arrow }
                else if self.peek_char() == Some('=') { self.read_char(); Token::MinusAssign }
                else { Token::Minus }
            }
            Some('*') => { if self.peek_char() == Some('=') { self.read_char(); Token::StarAssign } else { Token::Star } }
            Some('/') => { if self.peek_char() == Some('=') { self.read_char(); Token::SlashAssign } else { Token::Slash } }
            Some('%') => { if self.peek_char() == Some('=') { self.read_char(); Token::PercentAssign } else { Token::Percent } }
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
            Some('!') => {
                if self.peek_char() == Some('=') {
                    self.read_char();
                    Token::NotEqual
                } else {
                    Token::Bang
                }
            }
            Some('&') => {
                if self.peek_char() == Some('&') {
                    self.read_char();
                    Token::And
                } else {
                    Token::BitwiseAnd
                }
            }
            Some('(') => Token::LParen,
            Some(')') => Token::RParen,
            Some('{') => Token::LBrace,
            Some('}') => Token::RBrace,
            Some('[') => Token::LBracket,
            Some(']') => Token::RBracket,
            Some(',') => Token::Comma,
            Some(';') => Token::Semicolon,
            Some(':') => {
                if self.peek_char() == Some(':') {
                    self.read_char();
                    Token::DoubleColon
                } else {
                    Token::Colon
                }
            }
            Some('?') => Token::Question,
            Some('|') => {
                if self.peek_char() == Some('|') {
                    self.read_char();
                    Token::Or
                } else {
                    Token::Pipe
                }
            }
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
                    "while" => Token::While,
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
                    "enum" => Token::Enum,
                    "match" => Token::Match,
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
            Some(ch) if ch.is_ascii_digit() => {
                return Token::Number(self.read_number());
            }
            Some(ch) => {
                // Characters that are not valid at the start of any Zeus token
                self.errors.push(format!("{}:{}: unexpected character '{}'",
                    self.line_number, self.col, ch));
                Token::Identifier(ch.to_string()) // still emit a token so the parser can keep going
            }
            None => return Token::Eof,
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
        self.chars[position..self.position].iter().collect()
    }

    fn read_number(&mut self) -> f64 {
        let position = self.position;
        while let Some(ch) = self.ch {
            if ch.is_ascii_digit() {
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
        let text: String = self.chars[position..self.position].iter().collect();
        let val: f64 = text.parse().unwrap_or(0.0);
        if !val.is_finite() {
            self.errors.push(format!("line {}: numeric literal '{}' is out of range", self.line_number, text));
        } else if text.chars().all(|c| c.is_ascii_digit()) && text.parse::<i64>().is_err() {
            self.errors.push(format!("line {}: integer literal '{}' exceeds 64-bit range", self.line_number, text));
        }
        val
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
        let result = self.chars[position..self.position].iter().collect();
        self.read_char(); // skip closing quote
        result
    }
}
