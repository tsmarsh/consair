use crate::language::{AtomType, Value, VectorValue, cons};
use crate::numeric::NumericType;
use std::rc::Rc;

// ============================================================================
// Parser
// ============================================================================

#[derive(Debug)]
enum Token {
    LParen,
    RParen,
    Quote,
    VectorStart, // <<
    VectorEnd,   // >>
    Symbol(String),
    Number(NumericType),
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '\'' => {
                tokens.push(Token::Quote);
                chars.next();
            }
            '<' => {
                chars.next();
                if let Some(&'<') = chars.peek() {
                    chars.next();
                    tokens.push(Token::VectorStart);
                } else if let Some(&'=') = chars.peek() {
                    // This is <=, parse as symbol
                    chars.next();
                    tokens.push(Token::Symbol("<=".to_string()));
                } else {
                    // Single '<' as a symbol
                    tokens.push(Token::Symbol("<".to_string()));
                }
            }
            '>' => {
                chars.next();
                if let Some(&'>') = chars.peek() {
                    chars.next();
                    tokens.push(Token::VectorEnd);
                } else if let Some(&'=') = chars.peek() {
                    // This is >=, parse as symbol
                    chars.next();
                    tokens.push(Token::Symbol(">=".to_string()));
                } else {
                    // Single '>' as a symbol
                    tokens.push(Token::Symbol(">".to_string()));
                }
            }
            ch if ch.is_whitespace() => {
                chars.next();
            }
            ch if ch.is_numeric() || ch == '-' => {
                let mut num = String::new();
                if ch == '-' {
                    num.push(ch);
                    chars.next();
                }

                // Collect digits and check for decimal point or slash
                let mut has_dot = false;
                let mut has_slash = false;

                while let Some(&ch) = chars.peek() {
                    if ch.is_numeric() {
                        num.push(ch);
                        chars.next();
                    } else if ch == '.' && !has_dot && !has_slash {
                        has_dot = true;
                        num.push(ch);
                        chars.next();
                    } else if ch == '/' && !has_dot && !has_slash {
                        has_slash = true;
                        num.push(ch);
                        chars.next();
                    } else if (ch == 'e' || ch == 'E') && !has_slash {
                        // Scientific notation for floats (e.g., "2e-5" or "1.5e10")
                        num.push(ch);
                        chars.next();
                        // Handle optional +/- after e
                        if let Some(&sign) = chars.peek() {
                            if sign == '+' || sign == '-' {
                                num.push(sign);
                                chars.next();
                            }
                        }
                        // Mark as float since we have scientific notation
                        has_dot = true;
                    } else {
                        break;
                    }
                }

                // Parse the number based on its format
                if has_slash {
                    // Ratio: "5/2"
                    let parts: Vec<&str> = num.split('/').collect();
                    if parts.len() == 2 {
                        if let (Ok(numerator), Ok(denominator)) =
                            (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                        {
                            match NumericType::make_ratio(numerator, denominator) {
                                Ok(ratio) => tokens.push(Token::Number(ratio)),
                                Err(_) => tokens.push(Token::Symbol(num)),
                            }
                        } else {
                            tokens.push(Token::Symbol(num));
                        }
                    } else {
                        tokens.push(Token::Symbol(num));
                    }
                } else if has_dot || num.contains('e') || num.contains('E') {
                    // Float: "3.14" or "1e-5"
                    if let Ok(f) = num.parse::<f64>() {
                        tokens.push(Token::Number(NumericType::Float(f)));
                    } else {
                        tokens.push(Token::Symbol(num));
                    }
                } else {
                    // Integer: "42"
                    if let Ok(n) = num.parse::<i64>() {
                        tokens.push(Token::Number(NumericType::Int(n)));
                    } else {
                        tokens.push(Token::Symbol(num));
                    }
                }
            }
            _ => {
                let mut symbol = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() || ch == '(' || ch == ')' || ch == '\'' {
                        break;
                    }
                    symbol.push(ch);
                    chars.next();
                }
                tokens.push(Token::Symbol(symbol));
            }
        }
    }

    tokens
}

fn parse_tokens(tokens: &[Token]) -> Result<(Value, usize), String> {
    if tokens.is_empty() {
        return Err("Unexpected end of input".to_string());
    }

    match &tokens[0] {
        Token::Number(n) => Ok((Value::Atom(AtomType::Number(n.clone())), 1)),
        Token::Symbol(s) => {
            if s == "nil" {
                Ok((Value::Nil, 1))
            } else if s == "t" {
                Ok((Value::Atom(AtomType::Bool(true)), 1))
            } else {
                Ok((Value::Atom(AtomType::Symbol(s.clone())), 1))
            }
        }
        Token::Quote => {
            let (quoted, consumed) = parse_tokens(&tokens[1..])?;
            let quote_list = cons(
                Value::Atom(AtomType::Symbol("quote".to_string())),
                cons(quoted, Value::Nil),
            );
            Ok((quote_list, consumed + 1))
        }
        Token::VectorStart => {
            let mut values = Vec::new();
            let mut i = 1;

            while i < tokens.len() {
                if matches!(tokens[i], Token::VectorEnd) {
                    return Ok((
                        Value::Vector(Rc::new(VectorValue { elements: values })),
                        i + 1,
                    ));
                }

                let (value, consumed) = parse_tokens(&tokens[i..])?;
                values.push(value);
                i += consumed;
            }

            Err("Unclosed vector (missing >>)".to_string())
        }
        Token::VectorEnd => Err("Unexpected >>".to_string()),
        Token::LParen => {
            let mut values = Vec::new();
            let mut i = 1;

            while i < tokens.len() {
                if matches!(tokens[i], Token::RParen) {
                    let list = values
                        .into_iter()
                        .rev()
                        .fold(Value::Nil, |acc, val| cons(val, acc));
                    return Ok((list, i + 1));
                }

                let (value, consumed) = parse_tokens(&tokens[i..])?;
                values.push(value);
                i += consumed;
            }

            Err("Unclosed parenthesis".to_string())
        }
        Token::RParen => Err("Unexpected )".to_string()),
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let tokens = tokenize(input);
    let (value, _) = parse_tokens(&tokens)?;
    Ok(value)
}
