#![allow(clippy::collapsible_if, clippy::len_zero, clippy::map_unwrap_or,
                  clippy::type_complexity, unused_imports)]
use crate::ast::{Expression, Program, Statement};
use crate::lexer::Token;
use super::Parser;

impl<'a> Parser<'a> {
    pub(crate) fn parse_type(&mut self) -> Option<crate::ast::Type> {
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

}
