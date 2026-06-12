#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or, clippy::type_complexity)]
use crate::ast::{Program, Statement};
use crate::lexer::{Lexer, Token};

/// Security limits for DoS prevention
pub const MAX_RECURSION_DEPTH: usize = 1000;
pub const MAX_AST_NODES: usize = 100_000;

pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current_token: Token,
    pub(crate) peek_token: Token,
    pub(crate) errors: Vec<String>,
    pub(crate) parsing_tensor_dims: bool,
    pub(crate) expression_depth: usize,
    /// When true, a `{` after an identifier is NOT a struct literal. Set while
    /// parsing if/while conditions so `if x { }` is not misread as `x { }`.
    pub(crate) no_struct_literal: bool,
    /// Source line/col of current_token / peek_token (1-based) for diagnostics.
    pub(crate) current_line: usize,
    pub(crate) peek_line: usize,
    pub(crate) current_col: usize,
    pub(crate) peek_col: usize,
    /// Monotonic count of tokens consumed; used as a forward-progress marker so
    /// the top-level parse loop can never spin without consuming input.
    pub(crate) advance_count: usize,
    /// Monotonic count of AST nodes created; used to prevent DoS via massive ASTs.
    pub(crate) ast_node_count: usize,
}


pub(crate) mod statements;
pub(crate) mod types;
pub(crate) mod expressions;

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let current_line = lexer.token_line;
        let current_col  = lexer.token_col;
        let peek_token   = lexer.next_token();
        let peek_line    = lexer.token_line;
        let peek_col     = lexer.token_col;

        Parser {
            lexer,
            current_token,
            peek_token,
            errors: Vec::new(),
            parsing_tensor_dims: false,
            expression_depth: 0,
            no_struct_literal: false,
            current_line,
            peek_line,
            current_col,
            peek_col,
            advance_count: 0,
            ast_node_count: 0,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.current_line  = self.peek_line;
        self.current_col   = self.peek_col;
        self.peek_token    = self.lexer.next_token();
        self.peek_line     = self.lexer.token_line;
        self.peek_col      = self.lexer.token_col;
        self.advance_count += 1;
    }

    /// Push a parse error anchored at the current token position.
    pub(crate) fn error_here(&mut self, msg: &str) {
        self.errors.push(format!("{}:{}: {}", self.current_line, self.current_col, msg));
    }

    fn advance_after_statement(&mut self, prev_token: &Token) {
        // Block statements (if/for/parallel/target/proof/safestate/enclave/cluster/
        // comptime) already consume their own closing '}' and leave the cursor on the
        // NEXT statement's first token. Advancing again here would skip that statement
        // whenever it starts with an identifier (e.g. `x = 2;` or `foo();` right after
        // a block), which silently dropped it.
        if matches!(prev_token,
            Token::If | Token::For | Token::While | Token::Parallel | Token::Target | Token::Proof
            | Token::SafeState | Token::Enclave | Token::Cluster | Token::Comptime | Token::Match | Token::Enum) {
            return;
        }
        if self.current_token == Token::Semicolon {
            self.next_token();
        } else if !self.is_statement_start()
               && self.current_token != Token::RBrace
               && self.current_token != Token::Eof {
            // We're left on a non-statement-starting token after a parse (e.g. the closing ')'
            // of a function call expression). Advance once to prevent an infinite loop.
            // We deliberately do NOT advance when current_token is a statement-start keyword
            // (let, pub, fn, parallel, etc.) because that would eat the next valid statement —
            // which was the root cause of the original "consecutive let" bug.
            self.next_token();
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program {
            statements: Vec::new(),
        };

        while self.current_token != Token::Eof {
            // Security: Check AST node count limit to prevent DoS
            if self.ast_node_count >= MAX_AST_NODES {
                self.errors.push(format!(
                    "AST node count exceeds maximum of {} (DoS protection)",
                    MAX_AST_NODES
                ));
                break;
            }

            let current_line = self.lexer.line_number;
            program.statements.push(Statement::LineDirective(current_line));
            self.ast_node_count += 1;

            let progress_before = self.advance_count;
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                self.ast_node_count += 1;
                program.statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
            // Forward-progress guard: if a sub-parser failed without consuming any
            // token, force-advance one so the loop can never spin (which used to grow
            // `statements` with LineDirectives until multi-GB OOM, e.g. `fn f32`).
            if self.advance_count == progress_before && self.current_token != Token::Eof {
                self.errors.push(format!(
                    "{}:{}: unexpected token {:?}", self.current_line, self.current_col, self.current_token));
                self.next_token();
            }
        }

        // Surface any lexer-level errors (e.g. out-of-range numeric literals).
        let mut lex_errs = std::mem::take(&mut self.lexer.errors);
        self.errors.append(&mut lex_errs);
        program
    }

    // Check if the current token can start a statement
    fn is_statement_start(&self) -> bool {
        matches!(self.current_token,
            Token::Struct | Token::Component | Token::Let | Token::Parallel |
            Token::Target | Token::Proof | Token::SafeState | Token::Enclave |
            Token::Comptime | Token::If | Token::For | Token::While | Token::Return |
            Token::Assert | Token::Test |
            Token::AtSign | Token::Panic | Token::Extern | Token::Pub | Token::Fn | Token::Cluster |
            Token::Enum | Token::Match
        )
    }

}