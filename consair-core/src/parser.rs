use std::sync::Arc;

#[cfg(feature = "persistent")]
use im::Vector as ImVector;

use crate::interner::InternedSymbol;
use crate::language::{AtomType, SymbolType, Value, VectorValue, cons};
use crate::lexer::{Lexer, Token};

// ============================================================================
// Parser
// ============================================================================

pub struct Parser<'a> {
    lexer: &'a mut Lexer,
    current_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: &'a mut Lexer) -> Self {
        let current_token = lexer.next_token().unwrap_or(Token::Eof);
        Parser {
            lexer,
            current_token,
        }
    }

    fn advance(&mut self) -> Result<(), String> {
        self.current_token = self.lexer.next_token()?;
        Ok(())
    }

    pub fn parse_expression(&mut self) -> Result<Value, String> {
        match &self.current_token.clone() {
            Token::Number(n) => {
                let value = Value::Atom(AtomType::Number(n.clone()));
                self.advance()?;
                Ok(value)
            }
            Token::String(s) => {
                let value = Value::Atom(AtomType::String(s.clone()));
                self.advance()?;
                Ok(value)
            }
            Token::Symbol(s) => {
                let value = if s == "nil" {
                    Value::Nil
                } else if s == "t" {
                    Value::Atom(AtomType::Bool(true))
                } else {
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(s))))
                };
                self.advance()?;
                Ok(value)
            }
            Token::Quote => {
                self.advance()?;
                let quoted = self.parse_expression()?;
                Ok(cons(
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                        "quote",
                    )))),
                    cons(quoted, Value::Nil),
                ))
            }
            Token::Quasiquote => {
                self.advance()?;
                let quoted = self.parse_expression()?;
                Ok(cons(
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                        "quasiquote",
                    )))),
                    cons(quoted, Value::Nil),
                ))
            }
            Token::Unquote => {
                self.advance()?;
                let unquoted = self.parse_expression()?;
                Ok(cons(
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                        "unquote",
                    )))),
                    cons(unquoted, Value::Nil),
                ))
            }
            Token::UnquoteSplicing => {
                self.advance()?;
                let unquoted = self.parse_expression()?;
                Ok(cons(
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                        "unquote-splicing",
                    )))),
                    cons(unquoted, Value::Nil),
                ))
            }
            Token::LParen => {
                self.advance()?;
                let mut elements = Vec::new();

                while !matches!(self.current_token, Token::RParen | Token::Eof) {
                    elements.push(self.parse_expression()?);
                }

                if matches!(self.current_token, Token::Eof) {
                    return Err("Unclosed parenthesis".to_string());
                }

                self.advance()?; // consume )

                let list = elements
                    .into_iter()
                    .rev()
                    .fold(Value::Nil, |acc, val| cons(val, acc));
                Ok(list)
            }
            Token::VectorOpen => {
                self.advance()?;
                let mut vec_elements = Vec::new();

                while !matches!(self.current_token, Token::VectorClose | Token::Eof) {
                    vec_elements.push(self.parse_expression()?);
                }

                if matches!(self.current_token, Token::Eof) {
                    return Err("Unclosed vector literal".to_string());
                }

                self.advance()?; // consume >>

                #[cfg(not(feature = "persistent"))]
                let elements = vec_elements;
                #[cfg(feature = "persistent")]
                let elements = ImVector::from(vec_elements);
                Ok(Value::Vector(Arc::new(VectorValue { elements })))
            }
            Token::RParen => Err("Unexpected )".to_string()),
            Token::VectorClose => Err("Unexpected >>".to_string()),
            Token::Eof => Err("Unexpected end of input".to_string()),
        }
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut lexer = Lexer::new(input);
    let mut parser = Parser::new(&mut lexer);
    parser.parse_expression()
}
