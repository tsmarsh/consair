use std::fmt;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

use im::{HashMap as ImHashMap, HashSet as ImHashSet, Vector as ImVector};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::interner::InternedSymbol;
use crate::interpreter::Environment;
use crate::numeric::NumericType;

// ============================================================================
// Core Type System
// ============================================================================

/// String type - only basic strings with escape sequences
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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

impl Hash for AtomType {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            AtomType::Symbol(s) => s.hash(state),
            AtomType::Number(n) => n.hash(state),
            AtomType::String(s) => s.hash(state),
            AtomType::Bool(b) => b.hash(state),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

/// Vector value - fast mutable vector using Vec
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct VectorValue {
    pub elements: Vec<Value>,
}

/// Persistent vector - immutable with structural sharing using im::Vector
#[derive(Clone, Debug)]
pub struct PersistentVector {
    pub elements: ImVector<Value>,
}

impl PartialEq for PersistentVector {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl Eq for PersistentVector {}

impl Hash for PersistentVector {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_usize(self.elements.len());
        for elem in &self.elements {
            elem.hash(state);
        }
    }
}

/// Map value - fast hash map using FxHash
#[derive(Clone, Debug)]
pub struct MapValue {
    pub entries: FxHashMap<Value, Value>,
}

impl PartialEq for MapValue {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for MapValue {}

impl Hash for MapValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by sorting keys for deterministic ordering
        let mut pairs: Vec<_> = self.entries.iter().collect();
        pairs.sort_by(|(k1, _), (k2, _)| format!("{k1}").cmp(&format!("{k2}")));
        state.write_usize(pairs.len());
        for (k, v) in pairs {
            k.hash(state);
            v.hash(state);
        }
    }
}

/// Persistent map - immutable with structural sharing using im::HashMap
#[derive(Clone, Debug)]
pub struct PersistentMap {
    pub entries: ImHashMap<Value, Value>,
}

impl PartialEq for PersistentMap {
    fn eq(&self, other: &Self) -> bool {
        self.entries == other.entries
    }
}

impl Eq for PersistentMap {}

impl Hash for PersistentMap {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by sorting keys for deterministic ordering
        let mut pairs: Vec<_> = self.entries.iter().collect();
        pairs.sort_by(|(k1, _), (k2, _)| format!("{k1}").cmp(&format!("{k2}")));
        state.write_usize(pairs.len());
        for (k, v) in pairs {
            k.hash(state);
            v.hash(state);
        }
    }
}

/// Set value - fast hash set using FxHash
#[derive(Clone, Debug)]
pub struct SetValue {
    pub elements: FxHashSet<Value>,
}

impl PartialEq for SetValue {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl Eq for SetValue {}

impl Hash for SetValue {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by sorting elements for deterministic ordering
        let mut elems: Vec<_> = self.elements.iter().collect();
        elems.sort_by(|a, b| format!("{a}").cmp(&format!("{b}")));
        state.write_usize(elems.len());
        for e in elems {
            e.hash(state);
        }
    }
}

/// Persistent set - immutable with structural sharing using im::HashSet
#[derive(Clone, Debug)]
pub struct PersistentSet {
    pub elements: ImHashSet<Value>,
}

impl PartialEq for PersistentSet {
    fn eq(&self, other: &Self) -> bool {
        self.elements == other.elements
    }
}

impl Eq for PersistentSet {}

impl Hash for PersistentSet {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Hash by sorting elements for deterministic ordering
        let mut elems: Vec<_> = self.elements.iter().collect();
        elems.sort_by(|a, b| format!("{a}").cmp(&format!("{b}")));
        state.write_usize(elems.len());
        for e in elems {
            e.hash(state);
        }
    }
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
    /// Fast mutable vector (Vec-based)
    Vector(Arc<VectorValue>),
    /// Fast mutable map (FxHashMap-based)
    Map(Arc<MapValue>),
    /// Fast mutable set (FxHashSet-based)
    Set(Arc<SetValue>),
    /// Persistent vector with structural sharing (im::Vector)
    PersistentVector(Arc<PersistentVector>),
    /// Persistent map with structural sharing (im::HashMap)
    PersistentMap(Arc<PersistentMap>),
    /// Persistent set with structural sharing (im::HashSet)
    PersistentSet(Arc<PersistentSet>),
    /// Reduced wrapper - signals early termination in fold/reduce
    Reduced(Box<Value>),
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
            (Value::Map(a), Value::Map(b)) => a == b,
            (Value::Set(a), Value::Set(b)) => a == b,
            (Value::PersistentVector(a), Value::PersistentVector(b)) => a == b,
            (Value::PersistentMap(a), Value::PersistentMap(b)) => a == b,
            (Value::PersistentSet(a), Value::PersistentSet(b)) => a == b,
            (Value::Reduced(a), Value::Reduced(b)) => a == b,
            (Value::NativeFn(a), Value::NativeFn(b)) => {
                // Compare function pointers
                std::ptr::eq(a as *const _, b as *const _)
            }
            _ => false,
        }
    }
}

impl Eq for Value {}

impl Hash for Value {
    fn hash<H: Hasher>(&self, state: &mut H) {
        std::mem::discriminant(self).hash(state);
        match self {
            Value::Atom(a) => a.hash(state),
            Value::Cons(cell) => cell.hash(state),
            Value::Nil => {}
            Value::Lambda(lc) => {
                // Hash params and body (consistent with PartialEq)
                lc.params.hash(state);
                lc.body.hash(state);
            }
            Value::Macro(mc) => {
                // Hash params and body (consistent with PartialEq)
                mc.params.hash(state);
                mc.body.hash(state);
            }
            Value::Vector(v) => v.hash(state),
            Value::Map(m) => m.hash(state),
            Value::Set(s) => s.hash(state),
            Value::PersistentVector(v) => v.hash(state),
            Value::PersistentMap(m) => m.hash(state),
            Value::PersistentSet(s) => s.hash(state),
            Value::Reduced(v) => v.hash(state),
            Value::NativeFn(f) => {
                // Hash function pointer address
                (*f as usize).hash(state);
            }
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
            Value::Map(map) => {
                write!(f, "{{")?;
                let mut first = true;
                for (k, v) in &map.entries {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{k} {v}")?;
                }
                write!(f, "}}")
            }
            Value::Set(set) => {
                write!(f, "#{{")?;
                let mut first = true;
                for elem in &set.elements {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;
                    write!(f, "{elem}")?;
                }
                write!(f, "}}")
            }
            Value::PersistentVector(vec) => {
                write!(f, "#pvec[")?;
                for (i, elem) in vec.elements.iter().enumerate() {
                    if i > 0 {
                        write!(f, " ")?;
                    }
                    write!(f, "{elem}")?;
                }
                write!(f, "]")
            }
            Value::PersistentMap(map) => {
                write!(f, "#pmap{{")?;
                let mut first = true;
                for (k, v) in &map.entries {
                    if !first {
                        write!(f, ", ")?;
                    }
                    first = false;
                    write!(f, "{k} {v}")?;
                }
                write!(f, "}}")
            }
            Value::PersistentSet(set) => {
                write!(f, "#pset{{")?;
                let mut first = true;
                for elem in &set.elements {
                    if !first {
                        write!(f, " ")?;
                    }
                    first = false;
                    write!(f, "{elem}")?;
                }
                write!(f, "}}")
            }
            Value::Reduced(v) => write!(f, "#reduced({v})"),
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
