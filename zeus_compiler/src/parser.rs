use crate::ast::{Expression, Program, Statement};
use crate::lexer::{Lexer, Token};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
    errors: Vec<String>,
    parsing_tensor_dims: bool,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();
        
        Parser {
            lexer,
            current_token,
            peek_token,
            errors: Vec::new(),
            parsing_tensor_dims: false,
        }
    }

    fn next_token(&mut self) {
        self.current_token = self.peek_token.clone();
        self.peek_token = self.lexer.next_token();
    }

    pub fn parse_program(&mut self) -> Program {
        let mut program = Program {
            statements: Vec::new(),
        };

        while self.current_token != Token::Eof {
            let current_line = self.lexer.line_number;
            program.statements.push(Statement::LineDirective(current_line));

            if let Some(stmt) = self.parse_statement() {
                program.statements.push(stmt);
            }
            self.next_token();
        }

        program
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
                    self.next_token(); // move to expected ')'
                    if self.current_token != Token::RParen {
                        self.errors.push("Expected ')' after assert expression".to_string());
                        return None;
                    }
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
                    
                    // Try to parse expression - if it fails, skip to RParen gracefully
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
                _ => return None,
            }
        }

        if is_cfg {
            if self.current_token != Token::LBrace { return None; }
            self.next_token(); // consume '{'
            let mut statements = Vec::new();
            while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                if let Some(stmt) = self.parse_statement() {
                    statements.push(stmt);
                }
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
            // DEBUG: eprintln!("[DEBUG PARSER] Expected Pub or Fn, got {:?}", self.current_token);
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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.next_token();
        }
        // Don't consume RBrace here - let the caller (function body parser) handle it
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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
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
                self.errors.push("Expected identifier after let".to_string());
                return None;
            }
        };

        if self.peek_token != Token::Assign {
            self.errors.push("Expected '=' in let statement".to_string());
            return None;
        }
        self.next_token(); // move to '='
        self.next_token(); // move past '='

        let value = self.parse_expression()?;

        Some(Statement::Let {
            name,
            is_mut,
            is_secret,
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
        Some(Statement::StructDeclaration { name, is_component, fields })
    }

    fn parse_parallel_block(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'parallel'
        if self.current_token != Token::LParen { return None; }
        self.next_token(); // consume '('

        let iterator = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => return None,
        };
        self.next_token(); // consume identifier

        if self.current_token != Token::In { return None; }
        self.next_token(); // consume 'in'

        let start = self.parse_expression()?;
        self.next_token(); // consume last token of expression

        if self.current_token != Token::DoubleDot { return None; }
        self.next_token(); // consume '..'

        let end = self.parse_expression()?;
        self.next_token(); // consume last token of expression

        if self.current_token != Token::RParen { return None; }
        self.next_token(); // consume ')'

        if self.current_token != Token::LBrace { return None; }
        self.next_token(); // consume '{'

        let mut statements = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
            self.next_token();
        }

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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
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
            if let Some(stmt) = self.parse_statement() {
                statements.push(stmt);
            }
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
        self.next_token(); // consume last token of expression

        if self.current_token != Token::DoubleDot { return None; }
        self.next_token(); // consume '..'

        let end = self.parse_expression()?;
        self.next_token(); // consume last token of expression

        if self.current_token != Token::LBrace { return None; }
        self.next_token(); // consume '{'

        let mut body = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.next_token();
        }

        Some(Statement::For {
            iterator,
            start,
            end,
            body,
        })
    }

    fn parse_if_statement(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'if'
        
        let condition = self.parse_expression()?;
        self.next_token(); // move past expression

        if self.current_token != Token::LBrace {
            return None;
        }
        self.next_token(); // consume '{'

        let mut consequence = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            if let Some(stmt) = self.parse_statement() {
                consequence.push(stmt);
            }
            self.next_token();
        }

        let mut alternative = None;
        if self.peek_token == Token::Else {
            self.next_token(); // move to 'else'
            self.next_token(); // consume 'else'
            
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
                    if let Some(stmt) = self.parse_statement() {
                        alt_stmts.push(stmt);
                    }
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
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: current_token: {:?}", self.current_token);
        if self.current_token != Token::Fn { 
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: expected Fn, got {:?}", self.current_token);
            return None; 
        }
        self.next_token(); // consume 'fn'
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming 'fn': {:?}", self.current_token);

        let name = match &self.current_token {
            Token::Identifier(id) => id.clone(),
            _ => { self.errors.push("Expected function name".to_string()); return None; }
        };
        self.next_token(); // consume name
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming name '{}': {:?}", name, self.current_token);

        if self.current_token != Token::LParen { 
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: expected LParen, got {:?}", self.current_token);
            return None; 
        }
        self.next_token(); // consume '('
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming '(': {:?}", self.current_token);

        let mut parameters = Vec::new();
        while self.current_token != Token::RParen && self.current_token != Token::Eof {
            let param_name = match &self.current_token {
                Token::Identifier(id) => id.clone(),
                _ => {
                    // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: expected param name, got {:?}", self.current_token);
                    return None;
                }
            };
            self.next_token(); // consume name
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming param name '{}': {:?}", param_name, self.current_token);
            if self.current_token != Token::Colon { 
                // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: expected Colon, got {:?}", self.current_token);
                return None; 
            }
            self.next_token(); // consume ':'
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming ':': {:?}", self.current_token);

            let param_type = match self.parse_type() {
                Some(t) => t,
                None => {
                    // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: parse_type failed");
                    return None;
                }
            };
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: parsed type: {:?}", param_type);
            parameters.push((param_name, param_type));

            if self.current_token == Token::Comma {
                self.next_token(); // consume ','
            }
        }
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after param loop, current_token: {:?}", self.current_token);
        self.next_token(); // consume ')'
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after consuming ')', current_token: {:?}", self.current_token);

        let mut return_type = None;
        if self.current_token == Token::Arrow {
            self.next_token(); // consume '->'
            return_type = Some(self.parse_type()?);
        }

        if self.current_token != Token::LBrace { 
            println!("FAIL: Expected LBrace, got {:?}", self.current_token);
            return None; 
        }
        self.next_token(); // consume '{'

        let mut body = Vec::new();
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: starting body parsing, current_token: {:?}", self.current_token);
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: in body loop, current_token: {:?}", self.current_token);
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
            self.next_token();
            // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: after next_token in body loop: {:?}", self.current_token);
        }
        // DEBUG: eprintln!("[DEBUG PARSER] parse_function_declaration: body loop done, current_token: {:?}", self.current_token);

        Some(Statement::FunctionDeclaration {
            is_pub,
            name,
            parameters,
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
            if let Some(stmt) = self.parse_statement() {
                body.push(stmt);
            }
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

        let base_type = match &self.current_token {
            Token::Identifier(name) if name == "f64" => {
                self.next_token();
                crate::ast::Type::F64
            }
            Token::Tensor => {
                self.next_token();
                let mut dimensions = Vec::new();
                if self.current_token == Token::LessThan {
                    self.next_token();
                    self.parsing_tensor_dims = true;
                    while self.current_token != Token::GreaterThan && self.current_token != Token::Eof {
                        if let Some(dim) = self.parse_expression() {
                            dimensions.push(dim);
                        }
                        self.next_token();
                        if self.current_token == Token::Comma {
                            self.next_token();
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
                            }
                            // After parse_expression, current_token should be at the next token
                            // If it's RParen or Comma, handle accordingly
                            if self.current_token == Token::RParen {
                                break;
                            }
                            if self.current_token == Token::Comma {
                                self.next_token(); // consume ',' and continue
                            } else {
                                // Unexpected token, break to avoid infinite loop
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
                } else if self.peek_token == Token::LBrace {
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
                            }
                            if self.current_token == Token::Comma {
                                self.next_token();
                            }
                        } else {
                            break;
                        }
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
                self.next_token(); // consume the number
                Expression::Number(n)
            }
            Token::StringLiteral(s) => Expression::StringLiteral(s.clone()),
            Token::Tensor => {
                if self.peek_token != Token::LBracket { return None; }
                self.next_token(); // move to '['
                self.next_token(); // move past '['
                
                let mut dimensions = Vec::new();
                while self.current_token != Token::RBracket && self.current_token != Token::Eof {
                    if let Some(dim) = self.parse_expression() {
                        dimensions.push(dim);
                    }
                    if self.peek_token == Token::Comma {
                        self.next_token(); // move to ','
                    }
                    self.next_token();
                }
                Expression::TensorDefinition { dimensions }
            }
            Token::LParen => {
                self.next_token();
                let expr = self.parse_expression()?;
                if self.peek_token == Token::RParen {
                    self.next_token(); // move to ')'
                }
                expr
            }
            _ => return None,
        };

        while match self.peek_token {
            Token::Plus | Token::Minus | Token::Star | Token::Slash | Token::Assign | 
            Token::LessThan | Token::Equal | Token::Question | Token::LessEqual |
            Token::GreaterEqual |
            Token::BitShiftLeft | Token::BitShiftRight | Token::BitwiseAnd | Token::Pipe | Token::Dot | Token::LBracket => true,
            Token::GreaterThan => !self.parsing_tensor_dims,
            _ => false,
        } {
            if self.peek_token == Token::Question {
                self.next_token(); // move to '?'
                left = Expression::Try(Box::new(left));
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
            } else {
                let op = format!("{:?}", self.peek_token);
                self.next_token(); // move to operator
                self.next_token(); // move past operator
                if let Some(right) = self.parse_expression() {
                    left = Expression::Infix {
                        left: Box::new(left),
                        operator: op,
                        right: Box::new(right),
                    };
                }
            }
        }

        Some(left)
    }

    pub fn errors(&self) -> &Vec<String> {
        &self.errors
    }
}
