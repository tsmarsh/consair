# Split Parser Module and Add Comment Support

## Problem

The parser module is quite large (898 lines) and combines both tokenization (lexing) and parsing into a single module. Additionally, the parser doesn't handle comments natively, forcing the file runner (`cons/src/main.rs`) to implement complex workarounds with `parse_next_expr()` and `skip_whitespace_and_comments()` functions that duplicate parser logic.

**Locations:**
- `consair-core/src/parser.rs` (898 lines, mixed concerns)
- `cons/src/main.rs` (complex comment-stripping workarounds in lines 82-330)

## Impact

- Large module is harder to navigate
- Mixing concerns (tokenization vs parsing)
- Testing is less granular
- **Dual parser problem**: File runner duplicates parser logic to handle comments
- **Comments inside expressions not supported**: Parser can't handle `(cons ; comment\n 1 2)`
- **Fragile workarounds**: Comment stripping logic is complex and error-prone

## Prompt for Implementation

```
Split the parser.rs module into separate lexer.rs and parser.rs modules, and add native comment support to eliminate the dual parser problem:

1. Current parser.rs is 898 lines combining tokenization and parsing
2. File runner (cons/src/main.rs) has 250 lines of complex comment-stripping code
3. Comments can't appear inside expressions

Goal: Create a lexer that handles comments natively, so the file runner can simply call `parse()` repeatedly.

Please implement in this order:

**Phase 1: Create Lexer Module with Comment Support**

Create `consair-core/src/lexer.rs` with:

```rust
/// Token types produced by the lexer
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    OpenParen,
    CloseParen,
    OpenVector,    // <<
    CloseVector,   // >>
    Quote,         // '
    Number(String),
    Symbol(String),
    StringLit(String),
    // No Comment token - comments are stripped during lexing
}

/// Main tokenization function that strips comments
pub fn tokenize(input: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let mut chars = input.char_indices().peekable();

    while let Some((pos, ch)) = chars.next() {
        match ch {
            // Skip whitespace
            ' ' | '\t' | '\n' | '\r' => continue,

            // Handle comments: semicolon to end of line
            ';' => {
                // Skip until newline
                while let Some((_, ch)) = chars.peek() {
                    if *ch == '\n' {
                        chars.next(); // consume newline
                        break;
                    }
                    chars.next();
                }
                continue;
            }

            // Tokenize other characters...
            '(' => tokens.push(Token::OpenParen),
            ')' => tokens.push(Token::CloseParen),
            '\'' => tokens.push(Token::Quote),
            '"' => tokens.push(tokenize_string(&mut chars, pos)?),
            '<' => {
                if matches!(chars.peek(), Some((_, '<'))) {
                    chars.next();
                    tokens.push(Token::OpenVector);
                } else {
                    return Err(format!("Unexpected '<' at position {}", pos));
                }
            }
            '>' => {
                if matches!(chars.peek(), Some((_, '>'))) {
                    chars.next();
                    tokens.push(Token::CloseVector);
                } else {
                    return Err(format!("Unexpected '>' at position {}", pos));
                }
            }
            _ if ch.is_numeric() || ch == '-' => {
                tokens.push(tokenize_number(&mut chars, ch, pos)?);
            }
            _ => {
                tokens.push(tokenize_symbol(&mut chars, ch)?);
            }
        }
    }

    Ok(tokens)
}

fn tokenize_string(chars: &mut Peekable<CharIndices>, start: usize) -> Result<Token, String> {
    let mut s = String::new();
    let mut escaped = false;

    while let Some((pos, ch)) = chars.next() {
        if escaped {
            match ch {
                'n' => s.push('\n'),
                't' => s.push('\t'),
                'r' => s.push('\r'),
                '\\' => s.push('\\'),
                '"' => s.push('"'),
                _ => {
                    s.push('\\');
                    s.push(ch);
                }
            }
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '"' {
            return Ok(Token::StringLit(s));
        } else {
            s.push(ch);
        }
    }

    Err(format!("Unclosed string starting at position {}", start))
}

fn tokenize_number(chars: &mut Peekable<CharIndices>, first: char, pos: usize) -> Result<Token, String> {
    // Implementation similar to current parser
    // ... handle integers, ratios, floats ...
}

fn tokenize_symbol(chars: &mut Peekable<CharIndices>, first: char) -> Result<Token, String> {
    // Implementation similar to current parser
    // ... collect symbol characters ...
}
```

**Tests for lexer.rs:**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_comments() {
        let tokens = tokenize("; comment\n(cons 1 2)").unwrap();
        assert_eq!(tokens, vec![
            Token::OpenParen,
            Token::Symbol("cons".to_string()),
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::CloseParen,
        ]);
    }

    #[test]
    fn test_inline_comments() {
        let tokens = tokenize("(cons 1 ; first arg\n 2)").unwrap();
        assert_eq!(tokens, vec![
            Token::OpenParen,
            Token::Symbol("cons".to_string()),
            Token::Number("1".to_string()),
            Token::Number("2".to_string()),
            Token::CloseParen,
        ]);
    }

    #[test]
    fn test_comment_in_string_preserved() {
        let tokens = tokenize(r#"(println "foo ; not a comment")"#).unwrap();
        // The semicolon inside the string should be preserved
        match &tokens[2] {
            Token::StringLit(s) => assert_eq!(s, "foo ; not a comment"),
            _ => panic!("Expected string literal"),
        }
    }

    #[test]
    fn test_tokenize_number() {
        let tokens = tokenize("42").unwrap();
        assert_eq!(tokens, vec![Token::Number("42".to_string())]);
    }

    #[test]
    fn test_tokenize_string() {
        let tokens = tokenize(r#""hello world""#).unwrap();
        assert_eq!(tokens, vec![Token::StringLit("hello world".to_string())]);
    }

    #[test]
    fn test_tokenize_list() {
        let tokens = tokenize("(a b c)").unwrap();
        assert_eq!(tokens, vec![
            Token::OpenParen,
            Token::Symbol("a".to_string()),
            Token::Symbol("b".to_string()),
            Token::Symbol("c".to_string()),
            Token::CloseParen,
        ]);
    }
}
```

**Phase 2: Update Parser to Use Tokens**

Update `consair-core/src/parser.rs`:

```rust
use crate::lexer::{Token, tokenize};
use crate::value::Value;

/// Parse a string into a Value
pub fn parse_str(input: &str) -> Result<Value, String> {
    let tokens = tokenize(input)?;
    parse_tokens(&tokens)
}

/// Parse multiple expressions from a string
pub fn parse_all(input: &str) -> Result<Vec<Value>, String> {
    let tokens = tokenize(input)?;
    let mut values = Vec::new();
    let mut remaining = &tokens[..];

    while !remaining.is_empty() {
        let (value, rest) = parse_one(remaining)?;
        values.push(value);
        remaining = rest;
    }

    Ok(values)
}

/// Parse one expression from token slice, return value and remaining tokens
fn parse_one(tokens: &[Token]) -> Result<(Value, &[Token]), String> {
    if tokens.is_empty() {
        return Err("Unexpected end of tokens".to_string());
    }

    match &tokens[0] {
        Token::OpenParen => parse_list(&tokens[1..]),
        Token::OpenVector => parse_vector(&tokens[1..]),
        Token::Quote => {
            let (quoted, rest) = parse_one(&tokens[1..])?;
            Ok((Value::List(vec![
                Value::Atom(Atom::Symbol("quote".to_string())),
                quoted
            ]), rest))
        }
        Token::Number(n) => Ok((parse_number(n)?, &tokens[1..])),
        Token::Symbol(s) => Ok((Value::Atom(Atom::Symbol(s.clone())), &tokens[1..])),
        Token::StringLit(s) => Ok((Value::Atom(Atom::String(s.clone())), &tokens[1..])),
        Token::CloseParen => Err("Unexpected ')'".to_string()),
        Token::CloseVector => Err("Unexpected '>>'".to_string()),
    }
}

fn parse_list(tokens: &[Token]) -> Result<(Value, &[Token]), String> {
    let mut elements = Vec::new();
    let mut remaining = tokens;

    loop {
        if remaining.is_empty() {
            return Err("Unclosed '('".to_string());
        }

        match &remaining[0] {
            Token::CloseParen => {
                return Ok((Value::List(elements), &remaining[1..]));
            }
            _ => {
                let (element, rest) = parse_one(remaining)?;
                elements.push(element);
                remaining = rest;
            }
        }
    }
}

fn parse_vector(tokens: &[Token]) -> Result<(Value, &[Token]), String> {
    // Similar to parse_list but returns Value::Vector
    // ...
}

fn parse_number(s: &str) -> Result<Value, String> {
    // Convert string to numeric Value
    // ...
}
```

**Phase 3: Simplify File Runner**

Update `cons/src/main.rs` to remove the complex `parse_next_expr()` function:

```rust
use consair::parse_all;
use std::fs;

fn run_file(path: &Path) -> Result<(), String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;

    // Parse all expressions (comments handled by lexer)
    let expressions = parse_all(&content)?;

    if expressions.is_empty() {
        return Ok(()); // Empty file is ok
    }

    let mut env = Environment::new();
    let mut last_result = None;

    for expr in expressions {
        last_result = Some(eval(expr, &mut env)?);
    }

    // Print the final result if any
    if let Some(result) = last_result {
        println!("{}", result);
    }

    Ok(())
}
```

This replaces the entire `parse_next_expr()` and `skip_whitespace_and_comments()` functions (lines 82-330) with a simple call to `parse_all()`.

**Phase 4: Update Library Interface**

Update `consair-core/src/lib.rs`:

```rust
mod lexer;
mod parser;
mod value;
mod eval;

// Re-export key functions
pub use parser::{parse_str, parse_all};
pub use lexer::Token; // For testing/advanced use
pub use value::Value;
pub use eval::eval;
```

**Phase 5: Testing**

Add comprehensive tests:

```rust
// In consair-core/src/parser.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_comments() {
        let result = parse_all("; comment\n(cons 1 2)\n; another\n(cons 3 4)").unwrap();
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_parse_comment_inside_expression() {
        let result = parse_str("(cons ; first\n 1 ; second\n 2)").unwrap();
        // Should parse successfully
        assert!(matches!(result, Value::List(_)));
    }

    #[test]
    fn test_comment_in_string_not_stripped() {
        let result = parse_str(r#""hello ; world""#).unwrap();
        match result {
            Value::Atom(Atom::String(s)) => assert_eq!(s, "hello ; world"),
            _ => panic!("Expected string"),
        }
    }
}
```

Update file runner tests to remove the `#[ignore]` attribute from `test_comment_between_list_elements` since it should now work:

```rust
// In cons/tests/file_runner_tests.rs
#[test]
fn test_comment_between_list_elements() {
    let result = run_lisp_file(
        r#"
(cons
  ; first arg
  1
  ; second arg
  2)
"#,
    );
    // Should now work with comments inside expressions
    assert_eq!(result.unwrap(), "(1 . 2)");
}
```

**Success Metrics:**

1. Lexer module is ~300 lines (tokenization only)
2. Parser module is ~400 lines (AST construction only)
3. File runner main.rs is simplified to ~100 lines (no parse_next_expr)
4. All 155+ tests pass
5. Comments work inside expressions
6. Public API unchanged (parse_str still works)

**Documentation Updates:**

- Add module-level docs to lexer.rs explaining tokenization and comment handling
- Add module-level docs to parser.rs explaining token → AST transformation
- Update README with examples of comment support
```

## Success Criteria

**Module Organization:**
- [ ] lexer.rs created (~300 lines) with all tokenization logic
- [ ] parser.rs updated (~400 lines) with only AST construction
- [ ] lib.rs exports both modules appropriately
- [ ] Module documentation is clear and comprehensive

**Comment Support:**
- [ ] Lexer strips semicolon comments during tokenization
- [ ] Comments work inside expressions: `(cons ; comment\n 1 2)`
- [ ] Comments in strings are preserved: `"foo ; bar"`
- [ ] Inline comments work: `(cons 1 2) ; comment`
- [ ] Comment-only files handled correctly

**File Runner Simplification:**
- [ ] cons/src/main.rs simplified to ~100 lines
- [ ] `parse_next_expr()` function removed (was 220+ lines)
- [ ] `skip_whitespace_and_comments()` function removed (was 24 lines)
- [ ] File runner uses simple `parse_all()` call

**Testing:**
- [ ] All existing 155+ tests pass
- [ ] New lexer tests added (comment stripping, tokenization)
- [ ] New parser tests added (parse_all, comment support)
- [ ] `test_comment_between_list_elements` no longer ignored
- [ ] Tests split into lexer tests, parser tests, integration tests

**API Compatibility:**
- [ ] Public API unchanged (parse_str still works)
- [ ] parse_all() exported for multi-expression parsing
- [ ] Backward compatibility maintained

**Documentation:**
- [ ] lexer.rs has module-level docs explaining tokenization
- [ ] parser.rs has module-level docs explaining parsing
- [ ] README updated with comment syntax examples
- [ ] Migration guide for anyone using internal APIs
