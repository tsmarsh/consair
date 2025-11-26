use crate::language::StringType;
use crate::numeric::NumericType;

// ============================================================================
// Lexer
// ============================================================================

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Lexer {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn current_char(&self) -> char {
        if self.position < self.input.len() {
            self.input[self.position]
        } else {
            '\0'
        }
    }

    fn peek_ahead(&self, n: usize) -> char {
        if self.position + n < self.input.len() {
            self.input[self.position + n]
        } else {
            '\0'
        }
    }

    fn advance(&mut self) {
        if self.position < self.input.len() {
            self.position += 1;
        }
    }

    fn is_eof(&self) -> bool {
        self.position >= self.input.len()
    }

    fn skip_whitespace(&mut self) {
        loop {
            // Skip whitespace
            while !self.is_eof() && self.current_char().is_whitespace() {
                self.advance();
            }

            // Skip comments (semicolon to end of line)
            if self.current_char() == ';' {
                self.skip_comment();
            } else {
                break;
            }
        }
    }

    fn skip_comment(&mut self) {
        // Skip from semicolon to end of line (or EOF)
        while !self.is_eof() && self.current_char() != '\n' {
            self.advance();
        }
        // Advance past the newline if present
        if self.current_char() == '\n' {
            self.advance();
        }
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        if self.current_char() == expected {
            self.advance();
            Ok(())
        } else {
            Err(format!(
                "Expected '{}', found '{}'",
                expected,
                self.current_char()
            ))
        }
    }

    // ========================================================================
    // String Parsing
    // ========================================================================

    /// Main entry point for string parsing - only basic strings supported
    fn read_string_or_sigil(&mut self) -> Result<Token, String> {
        match self.current_char() {
            '"' => self.read_basic_string(),
            _ => Err("Not a string".to_string()),
        }
    }

    /// Read basic string with escape sequences
    fn read_basic_string(&mut self) -> Result<Token, String> {
        self.expect_char('"')?;
        let mut content = String::new();

        while self.current_char() != '"' && !self.is_eof() {
            if self.current_char() == '\\' {
                self.advance();
                content.push(self.read_escape_sequence()?);
            } else {
                content.push(self.current_char());
                self.advance();
            }
        }

        if self.is_eof() {
            return Err("Unterminated string".to_string());
        }

        self.expect_char('"')?;
        Ok(Token::String(StringType::Basic(content)))
    }

    /// Read escape sequence after backslash
    fn read_escape_sequence(&mut self) -> Result<char, String> {
        let c = self.current_char();
        self.advance();

        match c {
            'n' => Ok('\n'),
            't' => Ok('\t'),
            'r' => Ok('\r'),
            '\\' => Ok('\\'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            '0' => Ok('\0'),
            'u' => self.read_unicode_escape(),
            'x' => self.read_hex_escape(),
            _ => Err(format!("Unknown escape sequence: \\{c}")),
        }
    }

    /// Read Unicode escape: \u{1F600}
    fn read_unicode_escape(&mut self) -> Result<char, String> {
        self.expect_char('{')?;
        let mut hex = String::new();

        while self.current_char() != '}' && !self.is_eof() {
            if !self.current_char().is_ascii_hexdigit() {
                return Err("Invalid hex digit in unicode escape".to_string());
            }
            hex.push(self.current_char());
            self.advance();
        }

        self.expect_char('}')?;

        let code_point = u32::from_str_radix(&hex, 16)
            .map_err(|e| format!("Invalid unicode code point: {e}"))?;

        char::from_u32(code_point)
            .ok_or_else(|| format!("Invalid unicode code point: {code_point}"))
    }

    /// Read hex escape: \xFF
    fn read_hex_escape(&mut self) -> Result<char, String> {
        let mut hex = String::new();

        for _ in 0..2 {
            if !self.current_char().is_ascii_hexdigit() {
                return Err("Invalid hex digit in escape sequence".to_string());
            }
            hex.push(self.current_char());
            self.advance();
        }

        let byte = u8::from_str_radix(&hex, 16).map_err(|e| format!("Invalid hex escape: {e}"))?;

        Ok(byte as char)
    }

    /// Check if character is valid in symbol (excluding '/' for namespace separator)
    fn is_symbol_char(&self, c: char) -> bool {
        c.is_alphanumeric()
            || matches!(c, '-' | '_' | '+' | '*' | '!' | '?' | '<' | '>' | '=' | '%')
    }

    /// Check if character is valid in symbol (including '/')
    fn is_symbol_char_with_slash(&self, c: char) -> bool {
        self.is_symbol_char(c) || c == '/'
    }

    // ========================================================================
    // Number and Symbol Parsing
    // ========================================================================

    fn read_number_or_symbol(&mut self) -> Token {
        let mut text = String::new();

        // Collect the text
        if self.current_char() == '-' {
            text.push(self.current_char());
            self.advance();
        }

        let mut has_dot = false;
        let mut has_slash = false;

        while !self.is_eof() {
            let ch = self.current_char();
            if ch.is_numeric() {
                text.push(ch);
                self.advance();
            } else if ch == '.' && !has_dot && !has_slash {
                has_dot = true;
                text.push(ch);
                self.advance();
            } else if ch == '/' && !has_dot && !has_slash {
                has_slash = true;
                text.push(ch);
                self.advance();
            } else if (ch == 'e' || ch == 'E') && !has_slash {
                text.push(ch);
                self.advance();
                if let Some(&sign) = self.input.get(self.position)
                    && (sign == '+' || sign == '-')
                {
                    text.push(sign);
                    self.advance();
                }
                has_dot = true; // Mark as float
            } else {
                break;
            }
        }

        // Parse the number
        if has_slash {
            let parts: Vec<&str> = text.split('/').collect();
            if parts.len() == 2
                && let (Ok(numerator), Ok(denominator)) =
                    (parts[0].parse::<i64>(), parts[1].parse::<i64>())
                && let Ok(ratio) = NumericType::make_ratio(numerator, denominator)
            {
                return Token::Number(ratio);
            }
            Token::Symbol(text)
        } else if has_dot || text.contains('e') || text.contains('E') {
            if let Ok(f) = text.parse::<f64>() {
                Token::Number(NumericType::Float(f))
            } else {
                Token::Symbol(text)
            }
        } else if let Ok(n) = text.parse::<i64>() {
            Token::Number(NumericType::Int(n))
        } else {
            Token::Symbol(text)
        }
    }

    fn read_symbol(&mut self) -> Token {
        let mut symbol = String::new();

        while !self.is_eof() {
            let ch = self.current_char();
            if ch.is_whitespace() || matches!(ch, '(' | ')' | '\'' | '<' | '>' | '[' | ']' | ':') {
                break;
            }
            // Allow '/' in symbols (for things like function names)
            if self.is_symbol_char_with_slash(ch) {
                symbol.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        Token::Symbol(symbol)
    }

    // ========================================================================
    // Main Tokenization
    // ========================================================================

    pub fn next_token(&mut self) -> Result<Token, String> {
        self.skip_whitespace();

        if self.is_eof() {
            return Ok(Token::Eof);
        }

        let ch = self.current_char();

        match ch {
            '(' => {
                self.advance();
                Ok(Token::LParen)
            }
            ')' => {
                self.advance();
                Ok(Token::RParen)
            }
            '\'' => {
                self.advance();
                Ok(Token::Quote)
            }
            '`' => {
                self.advance();
                Ok(Token::Quasiquote)
            }
            ',' => {
                if self.peek_ahead(1) == '@' {
                    self.advance();
                    self.advance();
                    Ok(Token::UnquoteSplicing)
                } else {
                    self.advance();
                    Ok(Token::Unquote)
                }
            }
            '<' => {
                if self.peek_ahead(1) == '<' {
                    self.advance();
                    self.advance();
                    Ok(Token::VectorOpen)
                } else if self.peek_ahead(1) == '=' {
                    self.advance();
                    self.advance();
                    Ok(Token::Symbol("<=".to_string()))
                } else {
                    self.advance();
                    Ok(Token::Symbol("<".to_string()))
                }
            }
            '>' => {
                if self.peek_ahead(1) == '>' {
                    self.advance();
                    self.advance();
                    Ok(Token::VectorClose)
                } else if self.peek_ahead(1) == '=' {
                    self.advance();
                    self.advance();
                    Ok(Token::Symbol(">=".to_string()))
                } else {
                    self.advance();
                    Ok(Token::Symbol(">".to_string()))
                }
            }
            '"' | '$' | '#' | '~' => self.read_string_or_sigil(),
            ch if ch.is_numeric() => Ok(self.read_number_or_symbol()),
            '-' => {
                if self.peek_ahead(1).is_numeric() {
                    Ok(self.read_number_or_symbol())
                } else {
                    Ok(self.read_symbol())
                }
            }
            _ => Ok(self.read_symbol()),
        }
    }
}

// ============================================================================
// Token Types
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    LParen,
    RParen,
    VectorOpen,  // <<
    VectorClose, // >>
    Quote,
    Quasiquote,
    Unquote,
    UnquoteSplicing,
    Symbol(String),
    Number(NumericType),
    String(StringType),
    Eof,
}
