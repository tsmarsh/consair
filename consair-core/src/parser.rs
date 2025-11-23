use crate::language::{AtomType, StringPart, StringType, SymbolType, Value, VectorValue, cons};
use crate::numeric::NumericType;
use regex::Regex;
use std::sync::Arc;

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

    fn peek_string(&self, n: usize) -> String {
        self.input.iter().skip(self.position).take(n).collect()
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
        while !self.is_eof() && self.current_char().is_whitespace() {
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

    /// Main entry point for string/sigil parsing
    fn read_string_or_sigil(&mut self) -> Result<Token, String> {
        match self.current_char() {
            '"' => self.read_basic_or_multiline_string(false),
            '$' => {
                self.advance();
                match self.current_char() {
                    '"' => self.read_interpolated_string(false, false),
                    '#' => {
                        self.advance();
                        if self.current_char() == '"' {
                            self.read_interpolated_string(true, false)
                        } else {
                            Err("Expected '\"' after '$#'".to_string())
                        }
                    }
                    _ => Err("Expected '\"' or '#\"' after '$'".to_string()),
                }
            }
            '#' => {
                self.advance();
                match self.current_char() {
                    '"' => self.read_raw_string(0),
                    '#' => {
                        let hash_count = self.count_hashes();
                        self.read_raw_string(hash_count)
                    }
                    'b' => self.read_byte_string(),
                    '\\' => self.read_char_literal(),
                    _ => Err("Invalid prefix after '#'".to_string()),
                }
            }
            '~' => {
                self.advance();
                match self.current_char() {
                    'r' => self.read_regex(),
                    _ => Err("Unknown string sigil".to_string()),
                }
            }
            _ => Err("Not a string".to_string()),
        }
    }

    /// Read basic string or multiline string
    fn read_basic_or_multiline_string(&mut self, interpolated: bool) -> Result<Token, String> {
        if self.peek_string(3) == "\"\"\"" {
            self.read_multiline_string(interpolated)
        } else {
            self.read_basic_string()
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

    /// Read multiline string
    fn read_multiline_string(&mut self, interpolated: bool) -> Result<Token, String> {
        // Consume opening """
        for _ in 0..3 {
            self.expect_char('"')?;
        }

        let mut content = String::new();

        while !self.is_eof() {
            if self.peek_string(3) == "\"\"\"" {
                // Found closing delimiter
                for _ in 0..3 {
                    self.advance();
                }

                return Ok(Token::String(StringType::Multiline {
                    content,
                    interpolated,
                }));
            }

            content.push(self.current_char());
            self.advance();
        }

        Err("Unterminated multiline string".to_string())
    }

    /// Read raw string (no escape processing)
    fn read_raw_string(&mut self, hash_count: u8) -> Result<Token, String> {
        self.expect_char('"')?;
        let mut content = String::new();

        while !self.is_eof() {
            if self.current_char() == '"' {
                // Check if followed by correct number of #'s
                let mut matches = true;
                for i in 1..=hash_count {
                    if self.peek_ahead(i as usize) != '#' {
                        matches = false;
                        break;
                    }
                }

                if matches {
                    // Found closing delimiter
                    self.advance(); // consume "
                    for _ in 0..hash_count {
                        self.advance(); // consume #'s
                    }

                    return Ok(Token::String(StringType::Raw {
                        content,
                        hash_count,
                    }));
                }
            }

            content.push(self.current_char());
            self.advance();
        }

        Err("Unterminated raw string".to_string())
    }

    /// Count consecutive #'s for raw string delimiter
    fn count_hashes(&mut self) -> u8 {
        let mut count = 1; // Already saw first #

        while self.current_char() == '#' {
            count += 1;
            self.advance();
        }

        count
    }

    /// Read interpolated string
    fn read_interpolated_string(
        &mut self,
        is_raw: bool,
        is_multiline: bool,
    ) -> Result<Token, String> {
        if is_multiline {
            // Consume opening """
            for _ in 0..3 {
                self.expect_char('"')?;
            }
        } else {
            self.expect_char('"')?;
        }

        let mut parts = Vec::new();
        let mut current_literal = String::new();

        loop {
            if is_multiline && self.peek_string(3) == "\"\"\"" {
                // End of multiline interpolated string
                break;
            } else if !is_multiline && self.current_char() == '"' {
                // End of single-line interpolated string
                break;
            } else if self.is_eof() {
                return Err("Unterminated interpolated string".to_string());
            }

            if self.current_char() == '{' {
                // Save accumulated literal
                if !current_literal.is_empty() {
                    parts.push(StringPart::Literal(current_literal.clone()));
                    current_literal.clear();
                }

                // Parse interpolated expression
                self.advance(); // consume {
                let expr = self.read_interpolated_expression()?;
                parts.push(StringPart::Expression(Box::new(expr)));
                self.expect_char('}')?;
            } else if self.current_char() == '\\' && !is_raw {
                // Process escape sequence
                self.advance();
                current_literal.push(self.read_escape_sequence()?);
            } else {
                current_literal.push(self.current_char());
                self.advance();
            }
        }

        // Save final literal
        if !current_literal.is_empty() {
            parts.push(StringPart::Literal(current_literal));
        }

        // Consume closing delimiter
        if is_multiline {
            for _ in 0..3 {
                self.advance();
            }
        } else {
            self.expect_char('"')?;
        }

        Ok(Token::String(StringType::Interpolated { parts, is_raw }))
    }

    /// Read expression inside interpolation {}
    fn read_interpolated_expression(&mut self) -> Result<Value, String> {
        let mut depth = 0;
        let mut expr_chars = Vec::new();

        while !self.is_eof() {
            let ch = self.current_char();

            if ch == '(' || ch == '<' {
                depth += 1;
                expr_chars.push(ch);
                self.advance();
            } else if ch == ')' || ch == '>' {
                if depth > 0 {
                    depth -= 1;
                    expr_chars.push(ch);
                    self.advance();
                } else {
                    break;
                }
            } else if ch == '}' && depth == 0 {
                // End of interpolation
                break;
            } else {
                expr_chars.push(ch);
                self.advance();
            }
        }

        let expr_string: String = expr_chars.iter().collect();
        let expr_string = expr_string.trim();

        // Parse the expression
        let mut expr_lexer = Lexer::new(expr_string);
        let mut expr_parser = Parser::new(&mut expr_lexer);
        expr_parser.parse_expression()
    }

    /// Read byte string
    fn read_byte_string(&mut self) -> Result<Token, String> {
        self.advance(); // consume 'b'

        if self.current_char() == '"' {
            // ASCII byte string
            self.advance();
            let mut bytes = Vec::new();

            while self.current_char() != '"' && !self.is_eof() {
                bytes.push(self.current_char() as u8);
                self.advance();
            }

            self.expect_char('"')?;
            Ok(Token::String(StringType::Bytes(bytes)))
        } else if self.current_char() == '[' {
            // Explicit byte array
            self.advance();
            let mut bytes = Vec::new();

            while self.current_char() != ']' && !self.is_eof() {
                self.skip_whitespace();

                if self.current_char() == ']' {
                    break;
                }

                if self.current_char() == '0' && self.peek_ahead(1) == 'x' {
                    // Hex byte
                    self.advance(); // 0
                    self.advance(); // x

                    let mut hex = String::new();
                    for _ in 0..2 {
                        if !self.current_char().is_ascii_hexdigit() {
                            return Err("Invalid hex byte".to_string());
                        }
                        hex.push(self.current_char());
                        self.advance();
                    }

                    let byte = u8::from_str_radix(&hex, 16)
                        .map_err(|e| format!("Invalid hex byte: {e}"))?;
                    bytes.push(byte);
                } else if self.current_char().is_ascii_digit() {
                    // Decimal byte
                    let mut num_str = String::new();
                    while self.current_char().is_ascii_digit() {
                        num_str.push(self.current_char());
                        self.advance();
                    }

                    let byte: u8 = num_str.parse().map_err(|e| format!("Invalid byte: {e}"))?;
                    bytes.push(byte);
                } else {
                    return Err("Expected hex or decimal byte".to_string());
                }
            }

            self.expect_char(']')?;
            Ok(Token::String(StringType::Bytes(bytes)))
        } else {
            Err("Expected '\"' or '[' after '#b'".to_string())
        }
    }

    /// Read character literal: #\a #\newline #\space
    fn read_char_literal(&mut self) -> Result<Token, String> {
        self.advance(); // consume backslash

        let mut name = String::new();
        while !self.is_eof()
            && !self.current_char().is_whitespace()
            && !matches!(self.current_char(), '(' | ')' | '[' | ']' | '{' | '}')
        {
            name.push(self.current_char());
            self.advance();
        }

        if name.is_empty() {
            return Err("Empty character literal".to_string());
        }

        // Handle named characters
        let ch = match name.as_str() {
            "newline" => '\n',
            "space" => ' ',
            "tab" => '\t',
            "return" => '\r',
            s if s.len() == 1 => s.chars().next().unwrap(),
            _ => return Err(format!("Unknown character name: {name}")),
        };

        Ok(Token::Char(ch))
    }

    /// Read regex pattern
    fn read_regex(&mut self) -> Result<Token, String> {
        self.advance(); // consume 'r'

        // Read delimiter (usually /)
        let delimiter = self.current_char();
        if delimiter != '/' {
            return Err("Expected '/' to start regex pattern".to_string());
        }
        self.advance();

        let mut pattern = String::new();
        let mut escaped = false;

        while !self.is_eof() {
            if escaped {
                pattern.push(self.current_char());
                escaped = false;
            } else if self.current_char() == '\\' {
                pattern.push(self.current_char());
                escaped = true;
            } else if self.current_char() == delimiter {
                break;
            } else {
                pattern.push(self.current_char());
            }
            self.advance();
        }

        self.expect_char(delimiter)?;

        // Read flags (i, m, s, x, etc.)
        let mut flags = String::new();
        while self.current_char().is_ascii_alphabetic() {
            flags.push(self.current_char());
            self.advance();
        }

        // Build regex with flags
        let mut regex_pattern = String::new();
        if flags.contains('i') {
            regex_pattern.push_str("(?i)");
        }
        if flags.contains('m') {
            regex_pattern.push_str("(?m)");
        }
        if flags.contains('s') {
            regex_pattern.push_str("(?s)");
        }
        if flags.contains('x') {
            regex_pattern.push_str("(?x)");
        }
        regex_pattern.push_str(&pattern);

        // Compile regex
        let regex = Regex::new(&regex_pattern).map_err(|e| format!("Invalid regex: {e}"))?;

        Ok(Token::String(StringType::Regex(Arc::new(regex))))
    }

    // ========================================================================
    // Keyword Parsing
    // ========================================================================

    /// Read keyword
    fn read_keyword(&mut self) -> Result<Token, String> {
        self.expect_char(':')?;

        // Check for auto-namespaced keyword
        let auto_namespaced = if self.current_char() == ':' {
            self.advance();
            true
        } else {
            false
        };

        let mut name = String::new();
        let mut namespace = None;

        // Read first part
        while self.is_symbol_char(self.current_char()) {
            name.push(self.current_char());
            self.advance();
        }

        // Check for namespace separator
        if self.current_char() == '/' {
            self.advance();
            namespace = Some(name.clone());
            name.clear();

            // Read name part
            while self.is_symbol_char(self.current_char()) {
                name.push(self.current_char());
                self.advance();
            }
        }

        let symbol = if auto_namespaced {
            // Auto-namespace will be resolved at runtime based on current module
            SymbolType::Keyword {
                name,
                namespace: Some("__AUTO__".to_string()), // Placeholder
            }
        } else {
            SymbolType::Keyword { name, namespace }
        };

        Ok(Token::Keyword(symbol))
    }

    /// Check if character is valid in symbol/keyword (excluding '/' for namespace separator)
    fn is_symbol_char(&self, c: char) -> bool {
        c.is_alphanumeric() || matches!(c, '-' | '_' | '+' | '*' | '!' | '?' | '<' | '>' | '=')
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
            '<' => {
                if self.peek_ahead(1) == '<' {
                    self.advance();
                    self.advance();
                    Ok(Token::VectorStart)
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
                    Ok(Token::VectorEnd)
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
            ':' => self.read_keyword(),
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
    Quote,
    VectorStart,
    VectorEnd,
    Symbol(String),
    Keyword(SymbolType),
    Number(NumericType),
    String(StringType),
    Char(char),
    Eof,
}

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
            Token::Char(c) => {
                let value = Value::Atom(AtomType::Char(*c));
                self.advance()?;
                Ok(value)
            }
            Token::Symbol(s) => {
                let value = if s == "nil" {
                    Value::Nil
                } else if s == "t" {
                    Value::Atom(AtomType::Bool(true))
                } else {
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(s.clone())))
                };
                self.advance()?;
                Ok(value)
            }
            Token::Keyword(k) => {
                let value = Value::Atom(AtomType::Symbol(k.clone()));
                self.advance()?;
                Ok(value)
            }
            Token::Quote => {
                self.advance()?;
                let quoted = self.parse_expression()?;
                Ok(cons(
                    Value::Atom(AtomType::Symbol(SymbolType::Symbol("quote".to_string()))),
                    cons(quoted, Value::Nil),
                ))
            }
            Token::VectorStart => {
                self.advance()?;
                let mut elements = Vec::new();

                while !matches!(self.current_token, Token::VectorEnd | Token::Eof) {
                    elements.push(self.parse_expression()?);
                }

                if matches!(self.current_token, Token::Eof) {
                    return Err("Unclosed vector (missing >>)".to_string());
                }

                self.advance()?; // consume >>
                Ok(Value::Vector(Arc::new(VectorValue { elements })))
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
            Token::RParen => Err("Unexpected )".to_string()),
            Token::VectorEnd => Err("Unexpected >>".to_string()),
            Token::Eof => Err("Unexpected end of input".to_string()),
        }
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let mut lexer = Lexer::new(input);
    let mut parser = Parser::new(&mut lexer);
    parser.parse_expression()
}
