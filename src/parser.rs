use crate::language::{AtomType, Value, cons};

// ============================================================================
// Parser
// ============================================================================

#[derive(Debug)]
enum Token {
    LParen,
    RParen,
    Quote,
    Symbol(String),
    Number(i64),
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
            ch if ch.is_whitespace() => {
                chars.next();
            }
            ch if ch.is_numeric() || ch == '-' => {
                let mut num = String::new();
                if ch == '-' {
                    num.push(ch);
                    chars.next();
                }
                while let Some(&ch) = chars.peek() {
                    if ch.is_numeric() {
                        num.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num.parse::<i64>() {
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Symbol(num));
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
        Token::Number(n) => Ok((Value::Atom(AtomType::Number(*n)), 1)),
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
