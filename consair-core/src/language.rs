use regex::Regex;
use std::fmt;
use std::sync::Arc;

use crate::interner::InternedSymbol;
use crate::interpreter::Environment;
use crate::numeric::NumericType;

// ============================================================================
// Core Type System
// ============================================================================

/// String types supporting various literal forms
#[derive(Debug, Clone)]
pub enum StringType {
    /// Basic string with escape sequences processed
    /// Syntax: "hello\nworld"
    Basic(String),

    /// Raw string with no escape processing
    /// Syntax: #"C:\path\to\file"
    /// Syntax: ##"string with # chars"##
    Raw { content: String, hash_count: u8 },

    /// Interpolated string with embedded expressions
    /// Syntax: $"Hello {name}!"
    Interpolated {
        parts: Vec<StringPart>,
        is_raw: bool,
    },

    /// Multiline string preserving whitespace
    /// Syntax: """line1\nline2"""
    Multiline { content: String, interpolated: bool },

    /// Compiled regex pattern
    /// Syntax: ~r/pattern/flags
    Regex(Arc<Regex>),

    /// Binary byte string
    /// Syntax: #b"binary" or #b[0xFF 0x00]
    Bytes(Vec<u8>),
}

impl PartialEq for StringType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (StringType::Basic(a), StringType::Basic(b)) => a == b,
            (
                StringType::Raw {
                    content: a,
                    hash_count: ha,
                },
                StringType::Raw {
                    content: b,
                    hash_count: hb,
                },
            ) => a == b && ha == hb,
            (
                StringType::Interpolated {
                    parts: a,
                    is_raw: ra,
                },
                StringType::Interpolated {
                    parts: b,
                    is_raw: rb,
                },
            ) => a == b && ra == rb,
            (
                StringType::Multiline {
                    content: a,
                    interpolated: ia,
                },
                StringType::Multiline {
                    content: b,
                    interpolated: ib,
                },
            ) => a == b && ia == ib,
            (StringType::Regex(a), StringType::Regex(b)) => a.as_str() == b.as_str(),
            (StringType::Bytes(a), StringType::Bytes(b)) => a == b,
            _ => false,
        }
    }
}

/// Parts of an interpolated string
#[derive(Debug, Clone, PartialEq)]
pub enum StringPart {
    /// Literal string segment
    Literal(String),

    /// Expression to evaluate and insert
    Expression(Box<Value>),
}

/// Symbol and keyword types
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolType {
    /// Regular symbol (needs evaluation, now interned)
    /// Syntax: 'symbol or symbol
    Symbol(InternedSymbol),

    /// Keyword (self-evaluating, also interned)
    /// Syntax: :keyword or :namespace/keyword
    Keyword {
        name: InternedSymbol,
        namespace: Option<InternedSymbol>,
    },
}

impl SymbolType {
    /// Create a simple keyword
    pub fn keyword(name: impl Into<String>) -> Self {
        SymbolType::Keyword {
            name: InternedSymbol::new(&name.into()),
            namespace: None,
        }
    }

    /// Create a namespaced keyword
    pub fn namespaced_keyword(namespace: impl Into<String>, name: impl Into<String>) -> Self {
        SymbolType::Keyword {
            name: InternedSymbol::new(&name.into()),
            namespace: Some(InternedSymbol::new(&namespace.into())),
        }
    }

    /// Get the name as a String (for symbols only)
    pub fn resolve(&self) -> String {
        match self {
            SymbolType::Symbol(s) => s.resolve(),
            SymbolType::Keyword { .. } => panic!("Cannot use resolve() on keyword"),
        }
    }

    /// Check if this is a symbol (not a keyword)
    pub fn is_symbol(&self) -> bool {
        matches!(self, SymbolType::Symbol(_))
    }

    /// Check if this is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(self, SymbolType::Keyword { .. })
    }
}

#[derive(Debug, Clone)]
pub enum AtomType {
    Symbol(SymbolType),
    Number(NumericType),
    String(StringType),
    Char(char),
    Bool(bool),
}

// Implement PartialEq manually to handle NumericType comparison
impl PartialEq for AtomType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AtomType::Symbol(a), AtomType::Symbol(b)) => a == b,
            (AtomType::Number(a), AtomType::Number(b)) => a == b,
            (AtomType::String(a), AtomType::String(b)) => a == b,
            (AtomType::Char(a), AtomType::Char(b)) => a == b,
            (AtomType::Bool(a), AtomType::Bool(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for AtomType {}

#[derive(Clone, Debug, PartialEq)]
pub struct ConsCell {
    pub car: Value,
    pub cdr: Value,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LambdaCell {
    pub params: Vec<InternedSymbol>,
    pub body: Value,
    pub env: Environment,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VectorValue {
    pub elements: Vec<Value>,
}

/// Native function type - Rust functions callable from Lisp
pub type NativeFn = fn(&[Value], &mut Environment) -> Result<Value, String>;

#[derive(Clone, Debug)]
pub enum Value {
    Atom(AtomType),
    Cons(Arc<ConsCell>),
    Nil,
    Lambda(Arc<LambdaCell>),
    Vector(Arc<VectorValue>),
    NativeFn(NativeFn),
}

// Manual PartialEq implementation because function pointers need special handling
impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Atom(a), Value::Atom(b)) => a == b,
            (Value::Cons(a), Value::Cons(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Lambda(a), Value::Lambda(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => a == b,
            (Value::NativeFn(a), Value::NativeFn(b)) => {
                // Compare function pointers
                std::ptr::eq(a as *const _, b as *const _)
            }
            _ => false,
        }
    }
}

// Make Value thread-safe
// SAFETY: All interior data is either:
// - Immutable and wrapped in Arc (thread-safe)
// - Function pointers (stateless, thread-safe)
// - Basic types that are Send + Sync
unsafe impl Send for Value {}
unsafe impl Sync for Value {}

// ============================================================================
// Display Implementation
// ============================================================================

impl fmt::Display for StringType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StringType::Basic(s) => write!(f, "\"{}\"", escape_string(s)),
            StringType::Raw {
                content,
                hash_count,
            } => {
                let hashes = "#".repeat(*hash_count as usize);
                write!(f, "{hashes}\"{content}\"")
            }
            StringType::Interpolated { parts, is_raw } => {
                let prefix = if *is_raw { "$#" } else { "$" };
                write!(f, "{prefix}\"")?;
                for part in parts {
                    match part {
                        StringPart::Literal(s) => write!(f, "{s}")?,
                        StringPart::Expression(_) => write!(f, "{{...}}")?,
                    }
                }
                write!(f, "\"")
            }
            StringType::Multiline {
                content,
                interpolated,
            } => {
                let prefix = if *interpolated { "$" } else { "" };
                write!(f, "{prefix}\"\"\"{content}\"\"\"")
            }
            StringType::Regex(r) => write!(f, "~r/{}/", r.as_str()),
            StringType::Bytes(bytes) => {
                write!(f, "#b[")?;
                for (i, byte) in bytes.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "0x{byte:02X}")?;
                }
                write!(f, "]")
            }
        }
    }
}

impl fmt::Display for SymbolType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SymbolType::Symbol(s) => write!(f, "{s}"),
            SymbolType::Keyword {
                name,
                namespace: None,
            } => write!(f, ":{name}"),
            SymbolType::Keyword {
                name,
                namespace: Some(ns),
            } => ns.with_str(|ns_str| {
                if ns_str == "__AUTO__" {
                    write!(f, "::{name}")
                } else {
                    write!(f, ":{ns}/{name}")
                }
            }),
        }
    }
}

fn escape_string(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            '\n' => result.push_str("\\n"),
            '\t' => result.push_str("\\t"),
            '\r' => result.push_str("\\r"),
            '\\' => result.push_str("\\\\"),
            '"' => result.push_str("\\\""),
            c => result.push(c),
        }
    }
    result
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Atom(AtomType::Symbol(s)) => write!(f, "{s}"),
            Value::Atom(AtomType::Number(n)) => write!(f, "{n}"),
            Value::Atom(AtomType::String(s)) => write!(f, "{s}"),
            Value::Atom(AtomType::Char(c)) => write!(f, "#\\{c}"),
            Value::Atom(AtomType::Bool(b)) => write!(f, "{}", if *b { "t" } else { "nil" }),
            Value::Nil => write!(f, "nil"),
            Value::Cons(_) => {
                write!(f, "(")?;
                let mut current = self.clone();
                while let Value::Cons(ref cell) = current {
                    write!(f, "{}", cell.car)?;
                    match cell.cdr {
                        Value::Nil => break,
                        Value::Cons(_) => {
                            write!(f, " ")?;
                            current = cell.cdr.clone();
                        }
                        ref other => {
                            write!(f, " . {other}")?;
                            break;
                        }
                    }
                }
                write!(f, ")")
            }
            Value::Lambda(_) => write!(f, "<lambda>"),
            Value::Vector(vec) => {
                write!(f, "<<")?;
                for (i, elem) in vec.elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, ">>")
            }
            Value::NativeFn(_) => write!(f, "<native-fn>"),
        }
    }
}

// ============================================================================
// Primitive Operations
// ============================================================================

pub fn cons(car: Value, cdr: Value) -> Value {
    Value::Cons(Arc::new(ConsCell { car, cdr }))
}

pub fn car(value: &Value) -> Result<Value, String> {
    match value {
        Value::Cons(cell) => Ok(cell.car.clone()),
        _ => Err(format!("car: expected cons cell, got {value}")),
    }
}

pub fn cdr(value: &Value) -> Result<Value, String> {
    match value {
        Value::Cons(cell) => Ok(cell.cdr.clone()),
        _ => Err(format!("cdr: expected cons cell, got {value}")),
    }
}

pub fn is_atom(value: &Value) -> bool {
    matches!(value, Value::Atom(_) | Value::Nil)
}

pub fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Atom(a1), Value::Atom(a2)) => a1 == a2,
        (Value::Nil, Value::Nil) => true,
        _ => false,
    }
}
