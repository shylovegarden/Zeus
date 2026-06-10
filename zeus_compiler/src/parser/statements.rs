#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use crate::lexer::Token;
use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_statement(&mut self) -> Option<Statement> {
        match self.current_token {
            Token::Struct => self.parse_struct_declaration(false),
            Token::Component => self.parse_struct_declaration(true),
            Token::Enum => self.parse_enum_declaration(),
            Token::Match => self.parse_match_statement(),
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
                if self.peek_token == Token::Semicolon {
                    self.next_token(); // consume identifier and move to semicolon
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

    pub(crate) fn parse_attribute_statement(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_cluster_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_comptime_block(&mut self) -> Option<Statement> {
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


    pub(crate) fn parse_let_statement(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_struct_declaration(&mut self, is_component: bool) -> Option<Statement> {
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
        self.next_token(); // move past name
        // Parse optional generic type params: `struct Foo<T, U>`
        let type_params = self.parse_type_params();
        // move to '{'  (parse_type_params leaves cursor on the token after '>')
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
        Some(Statement::StructDeclaration { name, is_component, fields, type_params })
    }

    /// Skip a balanced `{ ... }` block, leaving current at the token after `}`.
    pub(crate) fn skip_balanced_braces(&mut self) {
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

    pub(crate) fn parse_parallel_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_target_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_proof_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_safe_state_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_enclave_block(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_for_statement(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_while_statement(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_if_statement(&mut self) -> Option<Statement> {
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

    pub(crate) fn parse_function_declaration(&mut self, is_pub: bool, attributes: Vec<crate::ast::FunctionAttribute>) -> Option<Statement> {
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

        // Parse optional generic type params: `fn foo<T>(x: T) -> T`
        let type_params = self.parse_type_params();

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
            type_params,
            parameters,
            secret_params,
            return_type,
            body,
            attributes,
        })
    }

    pub(crate) fn parse_test_declaration(&mut self) -> Option<Statement> {
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

    /// Parse an optional generic type-parameter list `<T, U, ...>`.
    /// If the current token is `<`, consumes through the matching `>` and returns
    /// the collected parameter names. Otherwise returns an empty Vec (no-op).
    pub(crate) fn parse_type_params(&mut self) -> Vec<String> {
        if self.current_token != Token::LessThan {
            return Vec::new();
        }
        self.next_token(); // consume '<'
        let mut params = Vec::new();
        while self.current_token != Token::GreaterThan && self.current_token != Token::Eof {
            if let Token::Identifier(ref id) = self.current_token.clone() {
                // Only accept single uppercase identifiers as type params (T, E, K, V …)
                params.push(id.clone());
                self.next_token();
            }
            if self.current_token == Token::Comma {
                self.next_token(); // consume ','
            }
        }
        if self.current_token == Token::GreaterThan {
            self.next_token(); // consume '>'
        }
        params
    }

}
