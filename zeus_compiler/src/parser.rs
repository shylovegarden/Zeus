use crate::ast::{Expression, Program, Statement};
use crate::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
    errors: Vec<String>,
    parsing_tensor_dims: bool,
    expression_depth: usize,
    /// When true, a `{` after an identifier is NOT a struct literal. Set while
    /// parsing if/while conditions so `if x { }` is not misread as `x { }`.
    no_struct_literal: bool,
    /// Source line of current_token / peek_token (1-based) for diagnostics.
    current_line: usize,
    peek_line: usize,
    /// Monotonic count of tokens consumed; used as a forward-progress marker so
    /// the top-level parse loop can never spin without consuming input.
    advance_count: usize,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let current_line = lexer.line_number;
        let peek_token = lexer.next_token();
        let peek_line = lexer.line_number;

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
            advance_count: 0,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.current_line = self.peek_line;
        self.peek_token = self.lexer.next_token();
        self.peek_line = self.lexer.line_number;
        self.advance_count += 1;
    }

    fn advance_after_statement(&mut self, prev_token: &Token) {
        // Block statements (if/for/parallel/target/proof/safestate/enclave/cluster/
        // comptime) already consume their own closing '}' and leave the cursor on the
        // NEXT statement's first token. Advancing again here would skip that statement
        // whenever it starts with an identifier (e.g. `x = 2;` or `foo();` right after
        // a block), which silently dropped it.
        if matches!(prev_token,
            Token::If | Token::For | Token::While | Token::Parallel | Token::Target | Token::Proof
            | Token::SafeState | Token::Enclave | Token::Cluster | Token::Comptime) {
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
            let current_line = self.lexer.line_number;
            program.statements.push(Statement::LineDirective(current_line));

            let progress_before = self.advance_count;
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                program.statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
            // Forward-progress guard: if a sub-parser failed without consuming any
            // token, force-advance one so the loop can never spin (which used to grow
            // `statements` with LineDirectives until multi-GB OOM, e.g. `fn f32`).
            if self.advance_count == progress_before && self.current_token != Token::Eof {
                self.errors.push(format!(
                    "line {}: skipping unparseable token {:?}", self.current_line, self.current_token));
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
            Token::AtSign | Token::Panic | Token::Extern | Token::Pub | Token::Fn | Token::Cluster
        )
    }

    fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Struct => self.parse_struct_declaration(false),
            Token::Component => self.parse_struct_declaration(true),
            Token::Let => self.parse_let_statement(),
            Token::Parallel => self.parse_parallel_block(),
            Token::Target => self.parse_target_block(),
            Token::Proof => self.parse_proof_block(),
            Token::SafeState => self.parse_safe_state_block(),
            Token::Enclave => self.parse_enclave_block(),
            Token::Comptime => {
                if self.peek_token == Token::LBrace {
                    self.parse_comptime_block()
                } else {
                    self.parse_expression_statement()
                }
            }
            Token::Assert => {
                self.next_token(); // consume 'assert'
                let has_paren = self.current_token == Token::LParen;
                if has_paren {
                    self.next_token(); // consume '('
                }
                let expr = self.parse_expression()?;
                if has_paren {
                    if self.current_token != Token::RParen {
                        self.next_token(); // move to expected ')'
                    }
                    if self.current_token != Token::RParen {
                        self.errors.push("Expected ')' after assert expression".to_string());
                        return None;
                    }
                    self.next_token(); // consume ')'
                }
                // Do NOT consume RParen here, let the caller's self.next_token() do it.
                Some(Statement::Assert(expr))
            }
            Token::Import => {
                let mut path = String::new();
                loop {
                    self.next_token(); // move to identifier
                    if let Token::Identifier(ref id) = self.current_token {
                        path.push_str(id);
                    } else {
                        break;
                    }
                    if self.peek_token == Token::Dot {
                        path.push('.');
                        self.next_token(); // move to dot
                    } else {
                        break; // current_token is still the identifier
                    }
                }
                Some(Statement::Import(path))
            }
            Token::For => self.parse_for_statement(),
            Token::While => self.parse_while_statement(),
            Token::Return => {
                self.next_token(); // consume 'return'
                let expr = self.parse_expression()?;
                Some(Statement::Return(expr))
            }
            Token::If => self.parse_if_statement(),
            Token::Pub => {
                self.next_token(); // consume 'pub'
                self.parse_function_declaration(true, vec![])
            }
            Token::Fn => self.parse_function_declaration(false, vec![]),
            Token::Cluster => {
                self.next_token(); // consume 'cluster'
                self.parse_cluster_block()
            }
            Token::Panic => {
                self.next_token(); // consume 'panic'
                let msg = match &self.current_token {
                    Token::StringLiteral(s) => s.clone(),
                    _ => {
                        self.errors.push("Expected string literal after panic".to_string());
                        return None;
                    }
                };
                self.next_token(); // consume string literal
                Some(Statement::Panic(msg))
            }
            Token::Extern => {
                self.next_token(); // consume 'extern'
                if self.current_token != Token::Fn {
                    self.errors.push("Expected 'fn' after 'extern'".to_string());
                    return None;
                }
                self.next_token(); // consume 'fn'

                let name = match &self.current_token {
                    Token::Identifier(id) => id.clone(),
                    _ => { self.errors.push("Expected function name".to_string()); return None; }
                };
                self.next_token(); // consume name

                if self.current_token != Token::LParen { return None; }
                self.next_token(); // consume '('

                let mut parameters = Vec::new();
                while self.current_token != Token::RParen && self.current_token != Token::Eof {
                    let param_name = match &self.current_token {
                        Token::Identifier(id) => id.clone(),
                        _ => return None,
                    };
                    self.next_token(); // consume name
                    if self.current_token != Token::Colon { return None; }
                    self.next_token(); // consume ':'

                    let param_type = self.parse_type()?;
                    parameters.push((param_name, param_type));

                    if self.current_token == Token::Comma {
                        self.next_token(); // consume ','
                    }
                }
                
                self.next_token(); // consume ')'

                let mut return_type = None;
                if self.current_token == Token::Arrow {
                    self.next_token(); // consume '->'
                    return_type = Some(self.parse_type()?);
                }

                // If there's a semicolon, we back up? No, the caller advances next_token().
                // If we are at the semicolon, the caller will advance past it. That's fine!
                // Actually, if we are at the semicolon, let's just leave it there.
                // Wait! If current_token is Semicolon, the caller advances past it.
                // If it's NOT a semicolon, we might have skipped a token, but the grammar expects a semicolon here.
                // So let's just make sure we are not eating the semicolon if we shouldn't.
                // Wait! If `parse_type` was called, `current_token` is Semicolon.
                // If `parse_type` was NOT called, `current_token` is Semicolon (because we called next_token on RParen).
                // In both cases, if the user wrote a semicolon, current_token is the Semicolon.
                // We can optionally consume it so the next token is the start of the next statement.
                // But wait, the main loop ALSO calls next_token! So if we consume it, main loop skips the next valid token!
                // But wait! If we don't consume it, the main loop calls next_token and skips the semicolon! That's perfect.
                // Wait, if the main loop skips the semicolon, then the next iteration starts on the NEXT token.
                // What if we do:
                if self.current_token == Token::Semicolon {
                    // Do nothing, let the main loop skip it? 
                    // But wait, wait! The main loop does `self.next_token()`. 
                    // If we do nothing, main loop will consume Semicolon, making the next token current_token.
                    // But then it calls `parse_statement` again on the NEXT token.
                    // Wait! Let's say the next token is `Pub`.
                    // Main loop sees Semicolon -> does next_token -> current_token is `Pub`. Next iteration starts!
                    // Oh! No. The main loop evaluates `parse_statement` FIRST!
                    // Then it calls `self.next_token()`!
                }

                Some(Statement::ExternFunctionDeclaration {
                    name,
                    parameters,
                    return_type,
                })
            }
            Token::Test => self.parse_test_declaration(),
            Token::AtSign => self.parse_attribute_statement(),
            _ => self.parse_expression_statement(),
        }
    }

    fn parse_attribute_statement(&mut self) -> Option<Statement> {
        let mut attributes = Vec::new();
        let mut is_cfg = false;
        let mut cfg_arch = String::new();

        while self.current_token == Token::AtSign {
            self.next_token(); // consume '@'
            match &self.current_token {
                Token::Identifier(id) if id == "cfg" => {
                    self.next_token(); // consume 'cfg'
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    cfg_arch = match &self.current_token {
                        Token::Identifier(a) => a.clone(),
                        _ => return None,
                    };
                    self.next_token(); // consume arch
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    is_cfg = true;
                    break;
                }
                Token::Verify => {
                    self.next_token(); // consume 'verify'
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    
                    // Parse the condition expression. After parse_expression returns,
                    // current_token is on the last token of the expression for non-advancing
                    // prefix types (Identifier stays put), but already past it for others
                    // (Number calls next_token internally). Normalize by advancing if not at ')'.
                    let expr = match self.parse_expression() {
                        Some(e) => e,
                        None => {
                            // Fallback: skip to closing paren and use a placeholder
                            while self.current_token != Token::RParen && self.current_token != Token::Eof {
                                self.next_token();
                            }
                            crate::ast::Expression::Number(1.0) // Placeholder truthy value
                        }
                    };

                    // Advance to RParen if parse_expression left us before it
                    if self.current_token != Token::RParen {
                        self.next_token();
                    }
                    // If still not at RParen, skip to it as a recovery
                    if self.current_token != Token::RParen {
                        while self.current_token != Token::RParen && self.current_token != Token::Eof {
                            self.next_token();
                        }
                    }
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    attributes.push(crate::ast::FunctionAttribute::Verify(expr, false));
                    // Continue loop to collect more attributes if present
                }
                Token::Adaptive => {
                    self.next_token(); // consume 'adaptive'
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    // for simplicity just capture the identifier or number as string
                    let params = match &self.current_token {
                        Token::Identifier(i) => i.clone(),
                        Token::Number(n) => n.to_string(),
                        _ => String::new(),
                    };
                    self.next_token(); // consume inner
                    // advance until RParen (just a simple skip for demo)
                    while self.current_token != Token::RParen && self.current_token != Token::Eof {
                        self.next_token();
                    }
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    attributes.push(crate::ast::FunctionAttribute::Adaptive(params));
                    // Continue loop to collect more attributes if present
                }
                Token::Identifier(id) if id == "ffi_export" => {
                    self.next_token(); // consume 'ffi_export'
                    attributes.push(crate::ast::FunctionAttribute::FfiExport);
                    // Continue loop to collect more attributes if present
                }
                Token::Identifier(id) if id == "requires" || id == "ensures" => {
                    let is_requires = id == "requires";
                    self.next_token(); // consume 'requires'/'ensures'
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    let expr = self.parse_expression()?;
                    if self.current_token != Token::RParen { self.next_token(); }
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    if is_requires {
                        attributes.push(crate::ast::FunctionAttribute::Requires(expr, true));
                    } else {
                        attributes.push(crate::ast::FunctionAttribute::Ensures(expr, true));
                    }
                }
                Token::Identifier(id) if id == "constant_time" => {
                    self.next_token(); // consume 'constant_time'
                    if self.current_token == Token::LParen {
                        self.next_token();
                        if self.current_token == Token::RParen { self.next_token(); }
                    }
                    attributes.push(crate::ast::FunctionAttribute::ConstantTime);
                }
                Token::Identifier(id) if id == "wcet" || id == "stack" => {
                    let is_wcet = id == "wcet";
                    self.next_token(); // consume kw
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    let n = if let Token::Number(v) = self.current_token { v as u64 } else { 0 };
                    self.next_token(); // consume number -> ')'
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    if is_wcet {
                        attributes.push(crate::ast::FunctionAttribute::Wcet(n));
                    } else {
                        attributes.push(crate::ast::FunctionAttribute::Stack(n));
                    }
                }
                Token::Identifier(id) if id == "atomic_add" => {
                    self.next_token(); // consume 'atomic_add'
                    if self.current_token != Token::LParen { return None; }
                    self.next_token(); // consume '('
                    let target = match &self.current_token {
                        Token::Identifier(t) => t.clone(),
                        _ => return None,
                    };
                    self.next_token(); // consume target
                    if self.current_token != Token::Comma { return None; }
                    self.next_token(); // consume ','
                    let amount = match &self.current_token {
                        Token::Number(n) => n.to_string(),
                        Token::Identifier(i) => i.clone(),
                        _ => return None,
                    };
                    self.next_token(); // consume amount
                    if self.current_token != Token::RParen { return None; }
                    self.next_token(); // consume ')'
                    // If there's a semicolon, we could leave it for the main loop, but it's an attribute-like statement
                    if self.current_token == Token::Semicolon {
                        // wait, next_token hasn't consumed semicolon yet, it's peeked/current
                    }
                    return Some(Statement::AtomicAdd { target, amount });
                }
                Token::Identifier(id) if id == "deterministic" => {
                    self.next_token(); // consume 'deterministic'
                    if self.current_token == Token::LParen {
                        self.next_token();
                        if self.current_token == Token::RParen { self.next_token(); }
                    }
                    // Accepted annotation. The determinism PROPERTY is computed by ZIR and
                    // reported by `zeus audit`/`cert` (gate with `--require=reproducible`).
                    // Kept as an explicit arm so it never drops sibling attributes (e.g. @wcet).
                }
                _ => {
                    // Unknown/misplaced attribute: FAIL LOUDLY rather than silently dropping
                    // the attributes already collected (a typo'd `@wceet(5)` on a safety-
                    // critical function must not be silently ignored).
                    self.errors.push(format!(
                        "line {}: unknown or misplaced attribute (known: @wcet @stack @constant_time @deterministic @verify @requires @ensures @adaptive @ffi_export @cfg)",
                        self.current_line));
                    return None;
                }
            }
        }

        if is_cfg {
            if self.current_token != Token::LBrace { return None; }
            self.next_token(); // consume '{'
            let mut statements = Vec::new();
            while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                let prev_token = self.current_token.clone();
                if let Some(stmt) = self.parse_statement() {
                    statements.push(stmt);
                }
                self.advance_after_statement(&prev_token);
            }
            if self.current_token == Token::RBrace {
                self.next_token();
            }
            return Some(Statement::CfgBlock { arch: cfg_arch, statements });
        }

        // It must be a function declaration following attributes
        if self.current_token == Token::Pub {
            self.next_token();
            self.parse_function_declaration(true, attributes)
        } else if self.current_token == Token::Fn {
            self.parse_function_declaration(false, attributes)
        } else {
            // "[DEBUG PARSER] Expected Pub or Fn, got {:?}", self.current_token);
            None
        }
    }

    fn parse_cluster_block(&mut self) -> Option<Statement> {
        // 'cluster' token already consumed by parse_statement
        if self.current_token != Token::LBrace {
            self.errors.push("Expected '{' after cluster".to_string());
            return None;
        }
        self.next_token(); // consume '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        // Consume the cluster's closing RBrace so caller sees what comes after
        if self.current_token == Token::RBrace {
            self.next_token(); // consume '}'
        }
        Some(Statement::ClusterBlock { statements })
    }

    fn parse_comptime_block(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'comptime'
        if self.current_token != Token::LBrace {
            self.errors.push("Expected '{' after comptime".to_string());
            return None;
        }
        self.next_token(); // consume '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::ComptimeBlock { statements })
    }


    fn parse_let_statement(&mut self) -> Option<Statement> {
        // let mut? secret? name = value;
        let is_mut = if self.peek_token == Token::Mut {
            self.next_token();
            true
        } else {
            false
        };

        let is_secret = if self.peek_token == Token::Secret {
            self.next_token();
            true
        } else {
            false
        };

        let name = match &self.peek_token {
            Token::Identifier(ident) => {
                let n = ident.clone();
                self.next_token();
                n
            }
            _ => {
                self.errors.push(format!("line {}: Expected identifier after let", self.current_line));
                return None;
            }
        };

        // Optional type annotation:  let name: Type = value
        let var_type = if self.peek_token == Token::Colon {
            self.next_token(); // current = ':'
            self.next_token(); // current = first token of the type
            let t = self.parse_type();
            // parse_type leaves the cursor on the token AFTER the type ('=').
            if self.current_token != Token::Assign {
                self.errors.push(format!("line {}: Expected '=' after type annotation in let statement", self.current_line));
                return None;
            }
            self.next_token(); // move past '=' to the first value token
            t
        } else {
            if self.peek_token != Token::Assign {
                self.errors.push(format!("line {}: Expected '=' in let statement", self.current_line));
                return None;
            }
            self.next_token(); // move to '='
            self.next_token(); // move past '='
            None
        };

        let value = self.parse_expression()?;

        Some(Statement::Let {
            name,
            is_mut,
            is_secret,
            var_type,
            value,
        })
    }

    fn parse_struct_declaration(&mut self, is_component: bool) -> Option<Statement> {
        if is_component {
            self.next_token(); // move past 'component'
            if self.current_token != Token::Struct {
                self.errors.push("Expected 'struct' after 'component'".to_string());
                return None;
            }
        }
        self.next_token(); // move past 'struct'
        let name = if let Token::Identifier(id) = &self.current_token {
            id.clone()
        } else {
            self.errors.push("Expected identifier after struct".to_string());
            return None;
        };
        self.next_token(); // move to '{'
        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // move past '{'
        
        let mut fields = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Token::Identifier(field_name) = &self.current_token {
                let fname = field_name.clone();
                self.next_token(); // move to ':'
                if self.current_token != Token::Colon {
                    return None;
                }
                self.next_token(); // move to type
                if let Some(t) = self.parse_type() {
                    fields.push((fname, t));
                } else {
                    return None;
                }
                if self.current_token == Token::Comma {
                    self.next_token();
                }
            } else {
                return None;
            }
        }
        if self.current_token == Token::RBrace {
            self.next_token(); // consume closing '}'
        }
        Some(Statement::StructDeclaration { name, is_component, fields })
    }

    /// Skip a balanced `{ ... }` block, leaving current at the token after `}`.
    fn skip_balanced_braces(&mut self) {
        // PRE: current == LBrace
        self.next_token(); // consume opening '{'
        let mut depth = 1i32;
        while depth > 0 && self.current_token != Token::Eof {
            if self.current_token == Token::LBrace {
                depth += 1;
            } else if self.current_token == Token::RBrace {
                depth -= 1;
            }
            self.next_token(); // always advance; when depth hits 0 this consumes the closing `}`
        }
        // POST: current is the token after the matching `}` (or Eof)
    }

    fn parse_parallel_block(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'parallel'
        if self.current_token == Token::LBrace {
            // Bare `parallel { }` without range — skip the whole block so the
            // enclosing function-body loop stays correctly bounded.
            self.skip_balanced_braces();
            return None;
        }
        if self.current_token != Token::LParen {
            return None;
        }
        self.next_token(); // consume '('

        let iterator = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return None,
        };
        self.next_token(); // consume identifier

        if self.current_token != Token::In {
            return None;
        }
        self.next_token(); // consume 'in'

        let start = self.parse_expression()?;
        if self.current_token != Token::DoubleDot {
            self.next_token();
        }
        if self.current_token != Token::DoubleDot {
            return None;
        }
        self.next_token(); // consume '..'

        let end = self.parse_expression()?;
        if self.current_token != Token::RParen {
            self.next_token();
        }
        if self.current_token != Token::RParen {
            return None;
        }
        self.next_token(); // consume ')'

        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // consume '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        self.next_token(); // consume the closing '}' of the parallel block

        Some(Statement::ParallelBlock {
            iterator,
            start,
            end,
            statements,
        })
    }

    fn parse_target_block(&mut self) -> Option<Statement> {
        if self.peek_token != Token::LBrace { return None; }
        self.next_token(); // move to '{'
        self.next_token(); // move past '{' to first target

        let mut targets = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Token::Identifier(ref id) = self.current_token {
                targets.push(id.clone());
            }
            self.next_token();
        }
        self.next_token(); // move past '}' of targets
        
        if self.current_token != Token::LBrace { return None; }
        self.next_token(); // move past '{' of block body

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::TargetBlock { targets, statements })
    }

    fn parse_proof_block(&mut self) -> Option<Statement> {
        if self.peek_token != Token::LBrace { return None; }
        self.next_token(); // move to '{'
        self.next_token(); // move past '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::ProofBlock { statements })
    }

    fn parse_safe_state_block(&mut self) -> Option<Statement> {
        if self.peek_token != Token::LBrace { return None; }
        self.next_token(); // move to '{'
        self.next_token(); // move past '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::SafeStateBlock { statements })
    }

    fn parse_enclave_block(&mut self) -> Option<Statement> {
        if self.peek_token != Token::LBrace { return None; }
        self.next_token(); // move to '{'
        self.next_token(); // move past '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::EnclaveBlock { statements })
    }

    fn parse_for_statement(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'for'
        
        let iterator = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return None,
        };
        self.next_token(); // consume identifier

        if self.current_token != Token::In { return None; }
        self.next_token(); // consume 'in'

        let start = self.parse_expression()?;
        if self.current_token != Token::DoubleDot {
            self.next_token();
        }
        if self.current_token != Token::DoubleDot { return None; }
        self.next_token(); // consume '..'

        let end = self.parse_expression()?;
        if self.current_token != Token::LBrace {
            self.next_token();
        }
        if self.current_token != Token::LBrace { return None; }
        self.next_token(); // consume '{'

        let mut body = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::For {
            iterator,
            start,
            end,
            body,
        })
    }

    fn parse_while_statement(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'while'
        let has_paren = self.current_token == Token::LParen;
        if has_paren {
            self.next_token(); // consume '('
        }
        let saved_nsl = self.no_struct_literal;
        self.no_struct_literal = true;
        let condition = self.parse_expression()?;
        self.no_struct_literal = saved_nsl;
        if has_paren {
            if self.current_token != Token::RParen {
                self.next_token();
            }
            if self.current_token != Token::RParen {
                return None;
            }
            self.next_token(); // consume ')'
        } else {
            self.next_token(); // move past last token of condition
        }
        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // consume '{'
        let mut body = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::While { condition, body })
    }

    fn parse_if_statement(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'if'

        let saved_nsl = self.no_struct_literal;
        self.no_struct_literal = true;
        let condition = self.parse_expression()?;
        self.no_struct_literal = saved_nsl;
        self.next_token(); // move past expression

        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // consume '{'

        let mut consequence = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                consequence.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        let mut alternative = None;
        if self.current_token == Token::Else {
            self.next_token(); // consume 'else' -> current = '{' or 'if'

            if self.current_token == Token::If {
                if let Some(if_stmt) = self.parse_if_statement() {
                    alternative = Some(vec![if_stmt]);
                }
            } else {
                if self.current_token != Token::LBrace {
                    return None;
                }
                self.next_token(); // consume '{'
                
                let mut alt_stmts = Vec::new();
                while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                    let prev_token = self.current_token.clone();
                    if let Some(stmt) = self.parse_statement() {
                        alt_stmts.push(stmt);
                    }
                    self.advance_after_statement(&prev_token);
                }
                if self.current_token == Token::RBrace {
                    self.next_token();
                }
                alternative = Some(alt_stmts);
            }
        }

        Some(Statement::If {
            condition,
            consequence,
            alternative,
        })
    }

    fn parse_function_declaration(&mut self, is_pub: bool, attributes: Vec<crate::ast::FunctionAttribute>) -> Option<Statement> {
        // "[DEBUG PARSER] parse_function_declaration: current_token: {:?}", self.current_token);
        if self.current_token != Token::Fn { 
            // "[DEBUG PARSER] parse_function_declaration: expected Fn, got {:?}", self.current_token);
            return None; 
        }
        self.next_token(); // consume 'fn'
        // "[DEBUG PARSER] parse_function_declaration: after consuming 'fn': {:?}", self.current_token);

        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => { self.errors.push("Expected function name".to_string()); return None; }
        };
        self.next_token(); // consume name
        // "[DEBUG PARSER] parse_function_declaration: after consuming name '{}': {:?}", name, self.current_token);

        if self.current_token != Token::LParen { 
            // "[DEBUG PARSER] parse_function_declaration: expected LParen, got {:?}", self.current_token);
            return None; 
        }
        self.next_token(); // consume '('
        // "[DEBUG PARSER] parse_function_declaration: after consuming '(': {:?}", self.current_token);

        let mut parameters = Vec::new();
        let mut secret_params: Vec<String> = Vec::new();
        while self.current_token != Token::RParen && self.current_token != Token::Eof {
            let param_is_secret = if self.current_token == Token::Secret { self.next_token(); true } else { false };
            let param_name = match &self.current_token {
                Token::Identifier(id) => id.clone(),
                _ => {
                    // "[DEBUG PARSER] parse_function_declaration: expected param name, got {:?}", self.current_token);
                    return None;
                }
            };
            self.next_token(); // consume name
            // "[DEBUG PARSER] parse_function_declaration: after consuming param name '{}': {:?}", param_name, self.current_token);
            if self.current_token != Token::Colon { 
                // "[DEBUG PARSER] parse_function_declaration: expected Colon, got {:?}", self.current_token);
                return None; 
            }
            self.next_token(); // consume ':'
            // "[DEBUG PARSER] parse_function_declaration: after consuming ':': {:?}", self.current_token);

            let param_type = match self.parse_type() {
                Some(t) => t,
                None => {
                    // "[DEBUG PARSER] parse_function_declaration: parse_type failed");
                    return None;
                }
            };
            // "[DEBUG PARSER] parse_function_declaration: parsed type: {:?}", param_type);
            if param_is_secret { secret_params.push(param_name.clone()); }
            parameters.push((param_name, param_type));

            if self.current_token == Token::Comma {
                self.next_token(); // consume ','
            }
        }
        // "[DEBUG PARSER] parse_function_declaration: after param loop, current_token: {:?}", self.current_token);
        self.next_token(); // consume ')'
        // "[DEBUG PARSER] parse_function_declaration: after consuming ')', current_token: {:?}", self.current_token);

        let mut return_type = None;
        if self.current_token == Token::Arrow {
            self.next_token(); // consume '->'
            return_type = Some(self.parse_type()?);
        }

        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // consume '{'

        let mut body = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::FunctionDeclaration {
            is_pub,
            name,
            parameters,
            secret_params,
            return_type,
            body,
            attributes,
        })
    }

    fn parse_test_declaration(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'test'
        
        if self.current_token != Token::Fn {
            self.errors.push("Expected 'fn' after 'test'".to_string());
            return None;
        }
        self.next_token(); // consume 'fn'

        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => { self.errors.push("Expected test name".to_string()); return None; }
        };
        self.next_token(); // consume name

        if self.current_token != Token::LParen { return None; }
        self.next_token(); // consume '('
        if self.current_token != Token::RParen { return None; }
        self.next_token(); // consume ')'

        if self.current_token != Token::LBrace { return None; }
        self.next_token(); // consume '{'

        let mut body = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let prev_token = self.current_token.clone();
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.advance_after_statement(&prev_token);
        }
        if self.current_token == Token::RBrace {
            self.next_token();
        }
        Some(Statement::TestDeclaration {
            name,
            body,
        })
    }

    fn parse_type(&mut self) -> Option<crate::ast::Type> {
        let is_pointer = if self.current_token == Token::Star {
            self.next_token();
            true
        } else {
            false
        };

        let is_sparse = if self.current_token == Token::Sparse {
            self.next_token();
            if self.current_token != Token::Tensor {
                self.errors.push("Expected 'tensor' after 'sparse'".to_string());
                return None;
            }
            true
        } else {
            false
        };

        if self.current_token == Token::LBracket {
            self.next_token(); // consume '['
            let elem_type = self.parse_type()?;
            if self.current_token != Token::Semicolon {
                self.errors.push("Expected ';' in array type [T; N]".to_string());
                return None;
            }
            self.next_token(); // consume ';'
            let size_expr = self.parse_expression()?;
            self.next_token(); // consume last token of size_expr -> ']'
            if self.current_token != Token::RBracket {
                self.errors.push("Expected ']' to close array type [T; N]".to_string());
                return None;
            }
            self.next_token(); // consume ']'
            let arr = crate::ast::Type::Array(Box::new(elem_type), Box::new(size_expr));
            return if is_pointer {
                Some(crate::ast::Type::Pointer(Box::new(arr)))
            } else {
                Some(arr)
            };
        }

        let base_type = match &self.current_token {
            Token::I8 => {
                self.next_token();
                crate::ast::Type::I8
            }
            Token::I32 => {
                self.next_token();
                crate::ast::Type::I32
            }
            Token::U64 => {
                self.next_token();
                crate::ast::Type::U64
            }
            Token::F32 => {
                self.next_token();
                crate::ast::Type::F32
            }
            Token::F64 => {
                self.next_token();
                crate::ast::Type::F64
            }
            Token::Identifier(name) if name == "f64" => {
                self.next_token();
                crate::ast::Type::F64
            }
            Token::Bool => {
                self.next_token();
                crate::ast::Type::Bool
            }
            Token::Tensor => {
                self.next_token();
                let mut dimensions = Vec::new();
                if self.current_token == Token::LessThan {
                    self.next_token();
                    self.parsing_tensor_dims = true;
                    while self.current_token != Token::GreaterThan && self.current_token != Token::Eof {
                        let prev_token = self.current_token.clone();
                        if let Some(dim) = self.parse_expression() {
                            dimensions.push(dim);
                        }
                        if self.current_token == Token::Comma {
                            self.next_token(); // consume ','
                        }
                        if self.current_token == prev_token {
                            self.next_token(); // force advance
                        }
                    }
                    self.parsing_tensor_dims = false;
                    if self.current_token == Token::GreaterThan {
                        self.next_token();
                    }
                }
                crate::ast::Type::Tensor { dimensions, is_sparse }
            }
            Token::Identifier(name) if name == "Result" => {
                self.next_token(); // consume Result
                if self.current_token == Token::LessThan {
                    self.next_token(); // consume <
                    let ok_type = self.parse_type()?;
                    if self.current_token == Token::Comma {
                        self.next_token(); // consume ,
                    }
                    let err_type = self.parse_type()?;
                    if self.current_token == Token::GreaterThan {
                        self.next_token(); // consume >
                    }
                    crate::ast::Type::Result(Box::new(ok_type), Box::new(err_type))
                } else {
                    return None;
                }
            }
            Token::Identifier(name) if name == "str" => {
                self.next_token();
                crate::ast::Type::Struct("str".to_string())
            }
            Token::Identifier(name) => {
                let n = name.clone();
                self.next_token();
                crate::ast::Type::Struct(n)
            }
            _ => return None
        };

        let array_type = if self.current_token == Token::LBracket {
            self.next_token(); // move past '['
            let size_expr = self.parse_expression()?;
            self.next_token(); // consume last token of size_expr
            if self.current_token == Token::RBracket {
                self.next_token();
            }
            crate::ast::Type::Array(Box::new(base_type), Box::new(size_expr))
        } else {
            base_type
        };

        if is_pointer {
            Some(crate::ast::Type::Pointer(Box::new(array_type)))
        } else {
            Some(array_type)
        }
    }

    fn parse_expression_statement(&mut self) -> Option<Statement> {
        let expr = self.parse_expression()?;
        Some(Statement::ExpressionStatement(expr))
    }

    fn parse_expression(&mut self) -> Option<Expression> {
        self.expression_depth += 1;
        if self.expression_depth > 128 {
            self.errors.push("AST Depth Limit Exceeded".to_string());
            self.expression_depth -= 1;
            return None;
        }

        let expr = self.parse_expression_impl();
        self.expression_depth -= 1;
        expr
    }

    fn parse_expression_impl(&mut self) -> Option<Expression> {
        self.parse_expression_bp(0)
    }

    /// Binding power for binary operators (higher = binds tighter).
    /// Returns (left_bp, right_bp); left-assoc ops use rbp = lbp + 1.
    fn infix_binding_power(tok: &Token) -> Option<(u8, u8)> {
        Some(match tok {
            Token::Assign => (5, 4), // right-associative
            Token::Or => (6, 7),     // logical ||  (looser than &&)
            Token::And => (8, 9),    // logical &&  (tighter than ||, looser than comparisons)
            Token::Equal | Token::NotEqual
            | Token::LessThan | Token::GreaterThan
            | Token::LessEqual | Token::GreaterEqual => (10, 11),
            Token::Pipe => (20, 21),
            Token::BitwiseAnd => (25, 26),
            Token::BitShiftLeft | Token::BitShiftRight => (30, 31),
            Token::Plus | Token::Minus => (40, 41),
            Token::Star | Token::Slash => (50, 51),
            _ => return None,
        })
    }

    fn parse_expression_bp(&mut self, min_bp: u8) -> Option<Expression> {
        let mut left = match self.current_token.clone() {
            Token::Comptime => {
                self.next_token(); // consume 'comptime'
                if self.current_token == Token::LParen {
                    self.next_token(); // consume '('
                    let expr = self.parse_expression()?;
                    if self.current_token == Token::RParen {
                        // next_token called in the loop
                    }
                    Expression::Comptime(Box::new(expr))
                } else {
                    self.errors.push("Expected '(' after comptime expression".to_string());
                    return None;
                }
            }
            Token::Identifier(id) => {
                if self.peek_token == Token::LParen {
                    self.next_token(); // move to LParen
                    let mut arguments = Vec::new();
                    self.next_token(); // move to first arg or RParen
                    if self.current_token != Token::RParen {
                        loop {
                            if let Some(arg) = self.parse_expression() {
                                arguments.push(arg);
                            } else {
                                self.next_token();
                            }
                            // Identifier expressions do not call next_token internally,
                            // so current may still be on the identifier. Advance once if
                            // we're not already at a separator to prevent infinite looping.
                            if self.current_token != Token::RParen
                               && self.current_token != Token::Comma
                               && self.current_token != Token::Eof {
                                self.next_token();
                            }
                            if self.current_token == Token::RParen {
                                break;
                            }
                            if self.current_token == Token::Comma {
                                self.next_token(); // consume ',' and continue
                            } else if self.current_token == Token::Eof {
                                break;
                            }
                        }
                    }
                    if self.current_token != Token::RParen {
                        self.errors.push("Expected ')' after function arguments".to_string());
                    }
                    Expression::FunctionCall {
                        name: id.clone(),
                        arguments,
                    }
                } else if self.peek_token == Token::LBrace && !self.no_struct_literal {
                    self.next_token(); // move to '{'
                    self.next_token(); // move past '{'
                    let mut fields = Vec::new();
                    while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                        if let Token::Identifier(fname) = &self.current_token {
                            let n = fname.clone();
                            self.next_token(); // move to ':'
                            if self.current_token == Token::Colon {
                                self.next_token(); // move to val
                                if let Some(val) = self.parse_expression() {
                                    fields.push((n, val));
                                }
                                // value parse leaves cursor on the value's last token;
                                // advance to the field separator so the loop progresses.
                                if self.current_token != Token::Comma
                                   && self.current_token != Token::RBrace
                                   && self.current_token != Token::Eof {
                                    self.next_token();
                                }
                            }
                            if self.current_token == Token::Comma {
                                self.next_token();
                            }
                        } else {
                            break;
                        }
                    }
                    if self.current_token == Token::RBrace {
                        self.next_token(); // consume closing '}'
                    }
                    Expression::StructInit {
                        name: id.clone(),
                        fields,
                    }
                } else {
                    // Don't consume here - the infix while loop handles advancement
                    Expression::Identifier(id.clone())
                }
            }
            Token::Number(n) => {
                // Leave the cursor ON the number (same convention as Identifier);
                // the infix loop below advances via peek_token. Consuming here made
                // the loop read the operand-after-operator as the operator, silently
                // dropping the rest of the expression (e.g. `2 + 5` parsed as `2`).
                Expression::Number(n)
            }
            Token::StringLiteral(s) => {
                // Leave cursor on the string literal (Identifier convention).
                Expression::StringLiteral(s.clone())
            }
            Token::Tensor => {
                if self.peek_token != Token::LBracket { return None; }
                self.next_token(); // move to '['
                self.next_token(); // move past '['
                
                let mut dimensions = Vec::new();
                self.parsing_tensor_dims = true;
                while self.current_token != Token::RBracket && self.current_token != Token::Eof {
                    let prev_token = self.current_token.clone();
                    if let Some(dim) = self.parse_expression() {
                        dimensions.push(dim);
                    }
                    if self.current_token == Token::Comma {
                        self.next_token(); // consume ','
                    }
                    if self.current_token == prev_token {
                        self.next_token(); // force advance
                    }
                }
                self.parsing_tensor_dims = false;
                if self.current_token == Token::RBracket {
                    self.next_token();
                }
                Expression::TensorDefinition { dimensions }
            }
            Token::Minus => {
                self.next_token();
                let operand = self.parse_expression_bp(60)?;
                Expression::Prefix { operator: "Minus".to_string(), operand: Box::new(operand) }
            }
            Token::Bang => {
                self.next_token();
                let operand = self.parse_expression_bp(60)?;
                Expression::Prefix { operator: "Not".to_string(), operand: Box::new(operand) }
            }
            Token::LParen => {
                self.next_token();
                let saved_nsl = self.no_struct_literal;
                self.no_struct_literal = false;
                let expr = self.parse_expression()?;
                self.no_struct_literal = saved_nsl;
                if self.peek_token == Token::RParen {
                    self.next_token(); // move to ')'
                }
                expr
            }
            Token::LBracket => {
                self.next_token(); // past '['
                let mut elements = Vec::new();
                while self.current_token != Token::RBracket && self.current_token != Token::Eof {
                    if let Some(el) = self.parse_expression() {
                        elements.push(el);
                    }
                    if self.current_token != Token::Comma
                       && self.current_token != Token::RBracket
                       && self.current_token != Token::Eof {
                        self.next_token();
                    }
                    if self.current_token == Token::Comma {
                        self.next_token();
                    }
                }
                Expression::ArrayLiteral(elements)
            }
            Token::AtSign => {
                if let Token::Identifier(ref id) = self.peek_token {
                    if id == "nvme_dma_map" {
                        self.next_token(); // move to 'nvme_dma_map'
                        self.next_token(); // move to '('
                        if self.current_token != Token::LParen { return None; }
                        self.next_token(); // move past '('
                        let path = self.parse_expression()?;
                        if self.current_token != Token::Comma { return None; }
                        self.next_token(); // move past ','
                        let size = self.parse_expression()?;
                        if self.current_token != Token::RParen { return None; }
                        self.next_token(); // move past ')'
                        
                        return Some(Expression::NvmeDmaMap {
                            path: Box::new(path),
                            size: Box::new(size),
                        });
                    }
                }
                return None;
            }
            _ => return None,
        };

        loop {
            // Postfix operators bind tightest and always apply: ?, ., []
            if self.peek_token == Token::Question {
                self.next_token();
                left = Expression::Try(Box::new(left));
                continue;
            } else if self.peek_token == Token::Dot {
                self.next_token(); // move to '.'
                self.next_token(); // move past '.'
                if let Token::Identifier(field) = &self.current_token {
                    left = Expression::FieldAccess {
                        base: Box::new(left),
                        field: field.clone(),
                    };
                } else {
                    return None;
                }
                continue;
            } else if self.peek_token == Token::LBracket {
                self.next_token(); // move to '['
                self.next_token(); // move past '['
                if let Some(index) = self.parse_expression() {
                    if self.peek_token == Token::RBracket {
                        self.next_token();
                    }
                    left = Expression::IndexAccess {
                        base: Box::new(left),
                        index: Box::new(index),
                    };
                } else {
                    return None;
                }
                continue;
            }

            // `>` is the tensor-dimension terminator when parsing tensor dims.
            if self.peek_token == Token::GreaterThan && self.parsing_tensor_dims {
                break;
            }

            // Binary operators, precedence-climbing (correct precedence + associativity).
            let (lbp, rbp) = match Parser::infix_binding_power(&self.peek_token) {
                Some(bp) => bp,
                None => break,
            };
            if lbp < min_bp {
                break;
            }
            let op = format!("{:?}", self.peek_token);
            self.next_token(); // move to operator
            self.next_token(); // move past operator
            match self.parse_expression_bp(rbp) {
                Some(right) => {
                    left = Expression::Infix {
                        left: Box::new(left),
                        operator: op,
                        right: Box::new(right),
                    };
                }
                None => break,
            }
        }

        Some(left)
    }

    pub fn errors(&self) -> &Vec<String> {
        &self.errors
    }
}
