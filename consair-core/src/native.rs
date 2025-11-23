//! Native function utilities and helpers
//!
//! This module provides utility functions for implementing native Rust functions
//! that can be called from Lisp code.

use crate::interner::InternedSymbol;
use crate::language::{AtomType, StringType, SymbolType, Value, cons};
use crate::numeric::NumericType;

// ============================================================================
// Value Extraction Helpers
// ============================================================================

/// Extract a string from a Value
pub fn extract_string(value: &Value) -> Result<String, String> {
    match value {
        Value::Atom(AtomType::String(StringType::Basic(s))) => Ok(s.clone()),
        Value::Atom(AtomType::String(StringType::Raw { content, .. })) => Ok(content.clone()),
        _ => Err(format!("Expected string, got {value}")),
    }
}

/// Extract an integer from a Value
pub fn extract_int(value: &Value) -> Result<i64, String> {
    match value {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => Ok(*n),
        _ => Err(format!("Expected integer, got {value}")),
    }
}

/// Extract a float from a Value (converting integers if needed)
pub fn extract_float(value: &Value) -> Result<f64, String> {
    match value {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => Ok(*n as f64),
        Value::Atom(AtomType::Number(NumericType::Float(f))) => Ok(*f),
        _ => Err(format!("Expected number, got {value}")),
    }
}

/// Extract a boolean from a Value
pub fn extract_bool(value: &Value) -> Result<bool, String> {
    match value {
        Value::Atom(AtomType::Bool(b)) => Ok(*b),
        Value::Nil => Ok(false),
        _ => Err(format!("Expected boolean, got {value}")),
    }
}

/// Extract bytes from a Value
pub fn extract_bytes(value: &Value) -> Result<Vec<u8>, String> {
    match value {
        Value::Atom(AtomType::String(StringType::Bytes(bytes))) => Ok(bytes.clone()),
        Value::Atom(AtomType::String(StringType::Basic(s))) => Ok(s.as_bytes().to_vec()),
        _ => Err(format!("Expected bytes or string, got {value}")),
    }
}

/// Extract a symbol name from a Value
pub fn extract_symbol(value: &Value) -> Result<String, String> {
    match value {
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) => Ok(s.resolve()),
        _ => Err(format!("Expected symbol, got {value}")),
    }
}

// ============================================================================
// List Manipulation Helpers
// ============================================================================

/// Convert a Lisp list to a Vec<Value>
pub fn list_to_vec(list: &Value) -> Result<Vec<Value>, String> {
    let mut result = Vec::new();
    let mut current = list.clone();

    while let Value::Cons(ref cell) = current {
        result.push(cell.car.clone());
        current = cell.cdr.clone();
    }

    if current != Value::Nil {
        return Err("Expected proper list (ending in nil)".to_string());
    }

    Ok(result)
}

/// Convert a Vec<Value> to a Lisp list
pub fn vec_to_list(items: Vec<Value>) -> Value {
    items
        .into_iter()
        .rev()
        .fold(Value::Nil, |acc, item| cons(item, acc))
}

/// Extract a list of strings from a Value
pub fn extract_string_list(value: &Value) -> Result<Vec<String>, String> {
    let vec = list_to_vec(value)?;
    vec.iter().map(extract_string).collect()
}

/// Extract a list of integers from a Value
pub fn extract_int_list(value: &Value) -> Result<Vec<i64>, String> {
    let vec = list_to_vec(value)?;
    vec.iter().map(extract_int).collect()
}

// ============================================================================
// Association List (alist) Helpers
// ============================================================================

/// Convert an association list to a vector of key-value pairs
/// Expects: ((key1 . val1) (key2 . val2) ...)
pub fn alist_to_vec(alist: &Value) -> Result<Vec<(Value, Value)>, String> {
    let mut result = Vec::new();
    let mut current = alist.clone();

    while let Value::Cons(ref cell) = current {
        // Each element should be a cons cell (key . value)
        match &cell.car {
            Value::Cons(pair) => {
                result.push((pair.car.clone(), pair.cdr.clone()));
            }
            _ => return Err("alist element must be a cons cell".to_string()),
        }
        current = cell.cdr.clone();
    }

    if current != Value::Nil {
        return Err("Expected proper list (ending in nil)".to_string());
    }

    Ok(result)
}

/// Convert a vector of key-value pairs to an association list
/// Creates: ((key1 . val1) (key2 . val2) ...)
pub fn vec_to_alist(pairs: Vec<(Value, Value)>) -> Value {
    pairs
        .into_iter()
        .rev()
        .fold(Value::Nil, |acc, (key, val)| cons(cons(key, val), acc))
}

// ============================================================================
// Argument Checking Helpers
// ============================================================================

/// Check that the number of arguments is exactly n
pub fn check_arity_exact(name: &str, args: &[Value], expected: usize) -> Result<(), String> {
    if args.len() != expected {
        return Err(format!(
            "{name}: expected {expected} argument{}, got {}",
            if expected == 1 { "" } else { "s" },
            args.len()
        ));
    }
    Ok(())
}

/// Check that the number of arguments is at least n
pub fn check_arity_min(name: &str, args: &[Value], min: usize) -> Result<(), String> {
    if args.len() < min {
        return Err(format!(
            "{name}: expected at least {min} argument{}, got {}",
            if min == 1 { "" } else { "s" },
            args.len()
        ));
    }
    Ok(())
}

/// Check that the number of arguments is in range [min, max]
pub fn check_arity_range(name: &str, args: &[Value], min: usize, max: usize) -> Result<(), String> {
    if args.len() < min || args.len() > max {
        return Err(format!(
            "{name}: expected {min}-{max} arguments, got {}",
            args.len()
        ));
    }
    Ok(())
}

// ============================================================================
// Value Construction Helpers
// ============================================================================

/// Create a string Value
pub fn make_string(s: impl Into<String>) -> Value {
    Value::Atom(AtomType::String(StringType::Basic(s.into())))
}

/// Create an integer Value
pub fn make_int(n: i64) -> Value {
    Value::Atom(AtomType::Number(NumericType::Int(n)))
}

/// Create a float Value
pub fn make_float(f: f64) -> Value {
    Value::Atom(AtomType::Number(NumericType::Float(f)))
}

/// Create a boolean Value
pub fn make_bool(b: bool) -> Value {
    Value::Atom(AtomType::Bool(b))
}

/// Create a keyword Value
pub fn make_keyword(name: impl Into<String>) -> Value {
    Value::Atom(AtomType::Symbol(SymbolType::keyword(name)))
}

/// Create a symbol Value
pub fn make_symbol(name: impl Into<String>) -> Value {
    Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
        &name.into(),
    ))))
}

/// Create a bytes Value
pub fn make_bytes(bytes: Vec<u8>) -> Value {
    Value::Atom(AtomType::String(StringType::Bytes(bytes)))
}

// ============================================================================
// Truthiness
// ============================================================================

/// Check if a value is truthy (everything except nil and false)
pub fn is_truthy(value: &Value) -> bool {
    !matches!(value, Value::Nil | Value::Atom(AtomType::Bool(false)))
}

/// Check if a value is falsy (nil or false)
pub fn is_falsy(value: &Value) -> bool {
    !is_truthy(value)
}
