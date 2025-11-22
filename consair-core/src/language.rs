use std::fmt;
use std::rc::Rc;

use crate::interpreter::Environment;

// ============================================================================
// Core Type System
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AtomType {
    Symbol(String),
    Number(i64),
    Bool(bool),
}

#[derive(Clone)]
pub struct ConsCell {
    pub car: Value,
    pub cdr: Value,
}

#[derive(Clone)]
pub struct LambdaCell {
    pub params: Vec<String>,
    pub body: Value,
    pub env: Environment,
}

#[derive(Clone)]
pub enum Value {
    Atom(AtomType),
    Cons(Rc<ConsCell>),
    Nil,
    Lambda(Rc<LambdaCell>),
}

// ============================================================================
// Display Implementation
// ============================================================================

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Atom(AtomType::Symbol(s)) => write!(f, "{s}"),
            Value::Atom(AtomType::Number(n)) => write!(f, "{n}"),
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
        }
    }
}

// ============================================================================
// Primitive Operations
// ============================================================================

pub fn cons(car: Value, cdr: Value) -> Value {
    Value::Cons(Rc::new(ConsCell { car, cdr }))
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
