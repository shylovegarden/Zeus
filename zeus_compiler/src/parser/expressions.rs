#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use crate::lexer::Token;
use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_expression_statement(&mut self) -> Option<Statement> {
        let expr = self.parse_expression()?;
        Some(Statement::ExpressionStatement(expr))
    }

    pub(crate) fn parse_expression(&mut self) -> Option<Expression> {
        self.expression_depth += 1;
        if self.expression_depth > super::MAX_RECURSION_DEPTH {
            self.errors.push(format!(
                "AST depth exceeds maximum of {} (DoS protection)",
                super::MAX_RECURSION_DEPTH
            ));
            self.expression_depth -= 1;
            return None;
        }

        let expr = self.parse_expression_impl();
        self.expression_depth -= 1;
        expr
    }

    pub(crate) fn parse_expression_impl(&mut self) -> Option<Expression> {
        self.parse_expression_bp(0)
    }

    /// Binding power for binary operators (higher = binds tighter).
    /// Returns (left_bp, right_bp); left-assoc ops use rbp = lbp + 1.
    pub(crate) fn infix_binding_power(tok: &Token) -> Option<(u8, u8)> {
        Some(match tok {
            Token::Assign | Token::PlusAssign | Token::MinusAssign | Token::StarAssign | Token::SlashAssign | Token::PercentAssign => (5, 4), // right-associative
            Token::Or => (6, 7),     // logical ||  (looser than &&)
            Token::And => (8, 9),    // logical &&  (tighter than ||, looser than comparisons)
            Token::Equal | Token::NotEqual
            | Token::LessThan | Token::GreaterThan
            | Token::LessEqual | Token::GreaterEqual => (10, 11),
            Token::Pipe => (20, 21),
            Token::BitwiseAnd => (25, 26),
            Token::BitShiftLeft | Token::BitShiftRight => (30, 31),
            Token::Plus | Token::Minus => (40, 41),
            Token::Star | Token::Slash | Token::Percent => (50, 51),
            _ => return None,
        })
    }

    pub(crate) fn parse_expression_bp(&mut self, min_bp: u8) -> Option<Expression> {
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
                } else if self.peek_token == Token::DoubleColon {
                    // Enum variant: MyEnum::Variant or MyEnum::Variant(args)
                    self.next_token(); // consume identifier, now on ::
                    self.next_token(); // consume ::, now on variant name
                    let variant = if let Token::Identifier(ref v) = self.current_token {
                        let v = v.clone();
                        v
                    } else {
                        self.errors.push("Expected variant name after ::".to_string());
                        return None;
                    };
                    // Check for tuple payload
                    let payload = if self.peek_token == Token::LParen {
                        self.next_token(); // consume variant name, now on (
                        self.next_token(); // consume (, now on first arg
                        let mut args = Vec::new();
                        while self.current_token != Token::RParen && self.current_token != Token::Eof {
                            if let Some(arg) = self.parse_expression() { args.push(arg); }
                            if self.current_token == Token::Comma { self.next_token(); }
                            else if self.current_token != Token::RParen { self.next_token(); }
                        }
                        // leave cursor on RParen; infix loop will advance
                        args
                    } else {
                        Vec::new()
                    };
                    Expression::EnumVariant { enum_name: id.clone(), variant, payload }
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
    pub(crate) fn parse_enum_declaration(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'enum'
        let name = if let Token::Identifier(id) = &self.current_token {
            id.clone()
        } else {
            self.errors.push("Expected enum name".to_string());
            return None;
        };
        self.next_token(); // consume name, move to '{'
        if self.current_token != Token::LBrace {
            self.errors.push("Expected '{' after enum name".to_string());
            return None;
        }
        self.next_token(); // consume '{'
        let mut variants = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            let vname = if let Token::Identifier(id) = &self.current_token {
                id.clone()
            } else {
                self.next_token();
                continue;
            };
            self.next_token(); // consume variant name
            // Optional payload: Variant(Type, Type, ...)
            let payload = if self.current_token == Token::LParen {
                self.next_token(); // consume '('
                let mut types = Vec::new();
                while self.current_token != Token::RParen && self.current_token != Token::Eof {
                    if let Some(t) = self.parse_type() {
                        types.push(t);
                    }
                    if self.current_token == Token::Comma { self.next_token(); }
                }
                if self.current_token == Token::RParen { self.next_token(); } // consume ')'
                Some(types)
            } else {
                None
            };
            variants.push(crate::ast::EnumVariantDef { name: vname, payload });
            if self.current_token == Token::Comma { self.next_token(); }
        }
        if self.current_token == Token::RBrace { self.next_token(); } // consume '}'
        Some(Statement::EnumDeclaration { name, variants })
    }

    pub(crate) fn parse_match_statement(&mut self) -> Option<Statement> {
        self.next_token(); // consume 'match'
        self.no_struct_literal = true;
        let scrutinee = self.parse_expression()?;
        self.no_struct_literal = false;
        // advance past the scrutinee to '{'
        if self.current_token != Token::LBrace { self.next_token(); }
        if self.current_token != Token::LBrace {
            self.errors.push("Expected '{' after match scrutinee".to_string());
            return None;
        }
        self.next_token(); // consume '{'
        let mut arms = Vec::new();
        while self.current_token != Token::RBrace && self.current_token != Token::Eof {
            // Parse pattern
            let pattern = self.parse_match_pattern()?;
            // Expect =>
            if self.current_token == Token::FatArrow { self.next_token(); }
            else {
                self.errors.push("Expected '=>' in match arm".to_string());
                return None;
            }
            // Parse arm body: either { stmts } or a single expression
            let body = if self.current_token == Token::LBrace {
                self.next_token(); // consume '{'
                let mut stmts = Vec::new();
                while self.current_token != Token::RBrace && self.current_token != Token::Eof {
                    if let Some(s) = self.parse_statement() {
                        stmts.push(s);
                    } else {
                        self.next_token();
                    }
                }
                if self.current_token == Token::RBrace { self.next_token(); } // consume '}'
                stmts
            } else {
                // single expression arm
                if let Some(expr) = self.parse_expression() {
                    if self.current_token == Token::Comma { self.next_token(); }
                    vec![Statement::ExpressionStatement(expr)]
                } else {
                    self.next_token();
                    Vec::new()
                }
            };
            arms.push(crate::ast::MatchArm { pattern, body });
            // optional comma between arms
            if self.current_token == Token::Comma { self.next_token(); }
        }
        if self.current_token == Token::RBrace { self.next_token(); } // consume '}'
        Some(Statement::MatchStatement { scrutinee, arms })
    }

    pub(crate) fn parse_match_pattern(&mut self) -> Option<crate::ast::MatchPattern> {
        match self.current_token.clone() {
            Token::Identifier(ref name) if name == "_" => {
                self.next_token();
                Some(crate::ast::MatchPattern::Wildcard)
            }
            Token::Number(n) => {
                self.next_token();
                Some(crate::ast::MatchPattern::Literal(n))
            }
            Token::Identifier(enum_name) => {
                self.next_token(); // consume enum name
                if self.current_token != Token::DoubleColon {
                    // plain wildcard identifier — treat as wildcard binding
                    return Some(crate::ast::MatchPattern::Wildcard);
                }
                self.next_token(); // consume ::
                let variant = if let Token::Identifier(ref v) = self.current_token {
                    let v = v.clone();
                    self.next_token(); // consume variant
                    v
                } else {
                    self.errors.push("Expected variant name in pattern".to_string());
                    return None;
                };
                if self.current_token == Token::LParen {
                    self.next_token(); // consume '('
                    let mut bindings = Vec::new();
                    while self.current_token != Token::RParen && self.current_token != Token::Eof {
                        if let Token::Identifier(b) = &self.current_token {
                            bindings.push(b.clone());
                        }
                        self.next_token();
                        if self.current_token == Token::Comma { self.next_token(); }
                    }
                    if self.current_token == Token::RParen { self.next_token(); } // consume ')'
                    Some(crate::ast::MatchPattern::VariantTuple { enum_name, variant, bindings })
                } else {
                    Some(crate::ast::MatchPattern::Variant { enum_name, variant })
                }
            }
            _ => {
                self.next_token();
                Some(crate::ast::MatchPattern::Wildcard)
            }
        }
    }

}
