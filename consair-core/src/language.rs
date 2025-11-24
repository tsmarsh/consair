use std::fmt;
use std::sync::Arc;

use crate::interner::InternedSymbol;
use crate::interpreter::Environment;
use crate::numeric::NumericType;

// ============================================================================
// Core Type System
// ============================================================================

/// String type - only basic strings with escape sequences
#[derive(Debug, Clone, PartialEq)]
pub enum StringType {
    /// Basic string with escape sequences processed
    /// Syntax: "hello\nworld"
    Basic(String),
}

/// Symbol type (interned for performance)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SymbolType {
    /// Regular symbol (needs evaluation, interned)
    /// Syntax: 'symbol or symbol
    Symbol(InternedSymbol),
}

impl SymbolType {
    /// Get the name as a String
    pub fn resolve(&self) -> String {
        match self {
            SymbolType::Symbol(s) => s.resolve(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AtomType {
    Symbol(SymbolType),
    Number(NumericType),
    String(StringType),
    Bool(bool),
}

// Implement PartialEq manually to handle NumericType comparison
impl PartialEq for AtomType {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (AtomType::Symbol(a), AtomType::Symbol(b)) => a == b,
            (AtomType::Number(a), AtomType::Number(b)) => a == b,
            (AtomType::String(a), AtomType::String(b)) => a == b,
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

#[derive(Clone)]
pub struct LambdaCell {
    pub params: Vec<InternedSymbol>,
    pub body: Value,
    pub env: Environment,
}

// Manual implementations since Environment uses RwLock (doesn't impl Debug/PartialEq)
impl std::fmt::Debug for LambdaCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LambdaCell")
            .field("params", &self.params)
            .field("body", &self.body)
            .field("env", &"<environment>")
            .finish()
    }
}

impl PartialEq for LambdaCell {
    fn eq(&self, other: &Self) -> bool {
        // Compare only params and body, not environment
        // (environments with same bindings but different Arc pointers would differ)
        self.params == other.params && self.body == other.body
    }
}

#[derive(Clone)]
pub struct MacroCell {
    pub params: Vec<InternedSymbol>,
    pub body: Value,
    pub env: Environment,
}

// Manual implementations since Environment uses RwLock (doesn't impl Debug/PartialEq)
impl std::fmt::Debug for MacroCell {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacroCell")
            .field("params", &self.params)
            .field("body", &self.body)
            .field("env", &"<environment>")
            .finish()
    }
}

impl PartialEq for MacroCell {
    fn eq(&self, other: &Self) -> bool {
        // Compare only params and body, not environment
        self.params == other.params && self.body == other.body
    }
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
    Macro(Arc<MacroCell>),
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
            (Value::Macro(a), Value::Macro(b)) => a == b,
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
        }
    }
}

impl fmt::Display for SymbolType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            SymbolType::Symbol(s) => write!(f, "{s}"),
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
            Value::Macro(_) => write!(f, "<macro>"),
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
