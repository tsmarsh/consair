//! Clojure-inspired runtime abstractions for consair.
//!
//! This module provides polymorphic behaviors that collections and values implement,
//! enabling uniform operations across different data types. These abstractions are
//! engine-level and dialect-agnostic.

// Value types can be used as FxHashMap/FxHashSet keys. While Value contains Arc<LambdaCell>
// which has interior mutability, lambdas as keys is an unusual use case and the Hash/Eq
// implementations are based on structural equality, not runtime state.
#![allow(clippy::mutable_key_type)]

use std::sync::Arc;

use im::{HashMap as ImHashMap, HashSet as ImHashSet, Vector as ImVector};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::language::{
    AtomType, ConsCell, MapValue, PersistentMap, PersistentSet, PersistentVector, SetValue,
    StringType, SymbolType, Value, VectorValue, cons,
};
use crate::numeric::NumericType;

// ============================================================================
// Abstraction Traits
// ============================================================================

/// Trait for collections that can report their size in O(1).
pub trait Counted {
    fn count(&self) -> usize;
}

/// Trait for collections that support indexed access.
pub trait Indexed {
    fn nth(&self, index: usize) -> Option<Value>;
}

/// Trait for collections that support key-based lookup.
pub trait Lookup {
    fn get_value(&self, key: &Value) -> Option<Value>;
}

/// Trait for collections that support associative updates.
/// Returns a new collection with the key-value association.
pub trait Associative {
    fn assoc(&self, key: Value, val: Value) -> Result<Self, String>
    where
        Self: Sized;
}

/// Trait for collections that support adding elements.
/// Returns a new collection with the element added.
pub trait Conjable {
    fn conj(&self, item: Value) -> Result<Self, String>
    where
        Self: Sized;
}

/// Trait for types that can be converted to a sequence.
pub trait Seqable {
    fn to_seq(&self) -> Option<Seq>;
}

// ============================================================================
// Trait Implementations - VectorValue (fast, Vec-based)
// ============================================================================

impl Counted for VectorValue {
    fn count(&self) -> usize {
        self.elements.len()
    }
}

impl Indexed for VectorValue {
    fn nth(&self, index: usize) -> Option<Value> {
        self.elements.get(index).cloned()
    }
}

impl Lookup for VectorValue {
    fn get_value(&self, key: &Value) -> Option<Value> {
        if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = key {
            if *idx >= 0 {
                self.elements.get(*idx as usize).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Associative for VectorValue {
    fn assoc(&self, key: Value, val: Value) -> Result<Self, String> {
        if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = &key {
            let idx = *idx as usize;
            if idx <= self.elements.len() {
                let mut new_elements = self.elements.clone();
                if idx == self.elements.len() {
                    new_elements.push(val);
                } else {
                    new_elements[idx] = val;
                }
                Ok(VectorValue {
                    elements: new_elements,
                })
            } else {
                Err(format!(
                    "Index {} out of bounds for vector of length {}",
                    idx,
                    self.elements.len()
                ))
            }
        } else {
            Err("Vector assoc requires integer key".to_string())
        }
    }
}

impl Conjable for VectorValue {
    fn conj(&self, item: Value) -> Result<Self, String> {
        let mut new_elements = self.elements.clone();
        new_elements.push(item);
        Ok(VectorValue {
            elements: new_elements,
        })
    }
}

impl Seqable for VectorValue {
    fn to_seq(&self) -> Option<Seq> {
        if self.elements.is_empty() {
            None
        } else {
            Some(Seq::VectorSeq {
                vec: Arc::new(self.clone()),
                index: 0,
            })
        }
    }
}

// ============================================================================
// Trait Implementations - PersistentVector (im::Vector-based)
// ============================================================================

impl Counted for PersistentVector {
    fn count(&self) -> usize {
        self.elements.len()
    }
}

impl Indexed for PersistentVector {
    fn nth(&self, index: usize) -> Option<Value> {
        self.elements.get(index).cloned()
    }
}

impl Lookup for PersistentVector {
    fn get_value(&self, key: &Value) -> Option<Value> {
        if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = key {
            if *idx >= 0 {
                self.elements.get(*idx as usize).cloned()
            } else {
                None
            }
        } else {
            None
        }
    }
}

impl Associative for PersistentVector {
    fn assoc(&self, key: Value, val: Value) -> Result<Self, String> {
        if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = &key {
            let idx = *idx as usize;
            if idx <= self.elements.len() {
                let new_elements = if idx == self.elements.len() {
                    let mut v = self.elements.clone();
                    v.push_back(val);
                    v
                } else {
                    self.elements.update(idx, val)
                };
                Ok(PersistentVector {
                    elements: new_elements,
                })
            } else {
                Err(format!(
                    "Index {} out of bounds for vector of length {}",
                    idx,
                    self.elements.len()
                ))
            }
        } else {
            Err("Vector assoc requires integer key".to_string())
        }
    }
}

impl Conjable for PersistentVector {
    fn conj(&self, item: Value) -> Result<Self, String> {
        let mut new_elements = self.elements.clone();
        new_elements.push_back(item);
        Ok(PersistentVector {
            elements: new_elements,
        })
    }
}

impl Seqable for PersistentVector {
    fn to_seq(&self) -> Option<Seq> {
        if self.elements.is_empty() {
            None
        } else {
            Some(Seq::PersistentVectorSeq {
                vec: Arc::new(self.clone()),
                index: 0,
            })
        }
    }
}

// ============================================================================
// Trait Implementations - MapValue (fast, FxHashMap-based)
// ============================================================================

impl Counted for MapValue {
    fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Lookup for MapValue {
    fn get_value(&self, key: &Value) -> Option<Value> {
        self.entries.get(key).cloned()
    }
}

impl Associative for MapValue {
    fn assoc(&self, key: Value, val: Value) -> Result<Self, String> {
        let mut new_entries = self.entries.clone();
        new_entries.insert(key, val);
        Ok(MapValue {
            entries: new_entries,
        })
    }
}

impl Conjable for MapValue {
    fn conj(&self, item: Value) -> Result<Self, String> {
        // Expect item to be a [key value] vector or (key . value) cons
        match &item {
            Value::Vector(pair) if pair.elements.len() == 2 => {
                let key = pair.elements[0].clone();
                let val = pair.elements[1].clone();
                self.assoc(key, val)
            }
            Value::PersistentVector(pair) if pair.elements.len() == 2 => {
                let key = pair.elements.get(0).cloned().unwrap();
                let val = pair.elements.get(1).cloned().unwrap();
                self.assoc(key, val)
            }
            Value::Cons(pair) => {
                let key = pair.car.clone();
                let val = pair.cdr.clone();
                self.assoc(key, val)
            }
            _ => Err("Map conj expects [key value] vector or (key . value) pair".to_string()),
        }
    }
}

impl Seqable for MapValue {
    fn to_seq(&self) -> Option<Seq> {
        if self.entries.is_empty() {
            None
        } else {
            let entries: Vec<_> = self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Some(Seq::MapSeq { entries, index: 0 })
        }
    }
}

// ============================================================================
// Trait Implementations - PersistentMap (im::HashMap-based)
// ============================================================================

impl Counted for PersistentMap {
    fn count(&self) -> usize {
        self.entries.len()
    }
}

impl Lookup for PersistentMap {
    fn get_value(&self, key: &Value) -> Option<Value> {
        self.entries.get(key).cloned()
    }
}

impl Associative for PersistentMap {
    fn assoc(&self, key: Value, val: Value) -> Result<Self, String> {
        let new_entries = self.entries.update(key, val);
        Ok(PersistentMap {
            entries: new_entries,
        })
    }
}

impl Conjable for PersistentMap {
    fn conj(&self, item: Value) -> Result<Self, String> {
        // Expect item to be a [key value] vector or (key . value) cons
        match &item {
            Value::Vector(pair) if pair.elements.len() == 2 => {
                let key = pair.elements[0].clone();
                let val = pair.elements[1].clone();
                self.assoc(key, val)
            }
            Value::PersistentVector(pair) if pair.elements.len() == 2 => {
                let key = pair.elements.get(0).cloned().unwrap();
                let val = pair.elements.get(1).cloned().unwrap();
                self.assoc(key, val)
            }
            Value::Cons(pair) => {
                let key = pair.car.clone();
                let val = pair.cdr.clone();
                self.assoc(key, val)
            }
            _ => Err("Map conj expects [key value] vector or (key . value) pair".to_string()),
        }
    }
}

impl Seqable for PersistentMap {
    fn to_seq(&self) -> Option<Seq> {
        if self.entries.is_empty() {
            None
        } else {
            let entries: Vec<_> = self
                .entries
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            Some(Seq::PersistentMapSeq { entries, index: 0 })
        }
    }
}

// ============================================================================
// Trait Implementations - SetValue (fast, FxHashSet-based)
// ============================================================================

impl Counted for SetValue {
    fn count(&self) -> usize {
        self.elements.len()
    }
}

impl Lookup for SetValue {
    fn get_value(&self, key: &Value) -> Option<Value> {
        // For sets, get returns the element itself if present
        if self.elements.contains(key) {
            Some(key.clone())
        } else {
            None
        }
    }
}

impl Conjable for SetValue {
    fn conj(&self, item: Value) -> Result<Self, String> {
        let mut new_elements = self.elements.clone();
        new_elements.insert(item);
        Ok(SetValue {
            elements: new_elements,
        })
    }
}

impl Seqable for SetValue {
    fn to_seq(&self) -> Option<Seq> {
        if self.elements.is_empty() {
            None
        } else {
            let elements: Vec<_> = self.elements.iter().cloned().collect();
            Some(Seq::SetSeq { elements, index: 0 })
        }
    }
}

// ============================================================================
// Trait Implementations - PersistentSet (im::HashSet-based)
// ============================================================================

impl Counted for PersistentSet {
    fn count(&self) -> usize {
        self.elements.len()
    }
}

impl Lookup for PersistentSet {
    fn get_value(&self, key: &Value) -> Option<Value> {
        // For sets, get returns the element itself if present
        if self.elements.contains(key) {
            Some(key.clone())
        } else {
            None
        }
    }
}

impl Conjable for PersistentSet {
    fn conj(&self, item: Value) -> Result<Self, String> {
        let new_elements = self.elements.update(item);
        Ok(PersistentSet {
            elements: new_elements,
        })
    }
}

impl Seqable for PersistentSet {
    fn to_seq(&self) -> Option<Seq> {
        if self.elements.is_empty() {
            None
        } else {
            let elements: Vec<_> = self.elements.iter().cloned().collect();
            Some(Seq::PersistentSetSeq { elements, index: 0 })
        }
    }
}

// ============================================================================
// Seq Abstraction - Uniform iteration over values
// ============================================================================

/// A sequence - lazy or realized view of sequential elements.
#[derive(Clone, Debug)]
pub enum Seq {
    /// A cons-based sequence (linked list)
    ConsBased(Arc<ConsCell>),
    /// A fast vector being iterated (with current index)
    VectorSeq { vec: Arc<VectorValue>, index: usize },
    /// A persistent vector being iterated (with current index)
    PersistentVectorSeq {
        vec: Arc<PersistentVector>,
        index: usize,
    },
    /// A fast map being iterated (as key-value pairs)
    MapSeq {
        entries: Vec<(Value, Value)>,
        index: usize,
    },
    /// A persistent map being iterated (as key-value pairs)
    PersistentMapSeq {
        entries: Vec<(Value, Value)>,
        index: usize,
    },
    /// A fast set being iterated
    SetSeq { elements: Vec<Value>, index: usize },
    /// A persistent set being iterated
    PersistentSetSeq { elements: Vec<Value>, index: usize },
    /// A string being iterated (as characters)
    StringSeq { chars: Vec<char>, index: usize },
}

impl Seq {
    /// Get the first element of this sequence.
    pub fn first(&self) -> Value {
        match self {
            Seq::ConsBased(cell) => cell.car.clone(),
            Seq::VectorSeq { vec, index } => vec.nth(*index).unwrap_or(Value::Nil),
            Seq::PersistentVectorSeq { vec, index } => vec.nth(*index).unwrap_or(Value::Nil),
            Seq::MapSeq { entries, index } | Seq::PersistentMapSeq { entries, index } => {
                if let Some((k, v)) = entries.get(*index) {
                    // Return as a two-element vector [key value]
                    Value::Vector(Arc::new(VectorValue {
                        elements: vec![k.clone(), v.clone()],
                    }))
                } else {
                    Value::Nil
                }
            }
            Seq::SetSeq { elements, index } | Seq::PersistentSetSeq { elements, index } => {
                elements.get(*index).cloned().unwrap_or(Value::Nil)
            }
            Seq::StringSeq { chars, index } => chars.get(*index).map_or(Value::Nil, |c| {
                Value::Atom(AtomType::String(StringType::Basic(c.to_string())))
            }),
        }
    }

    /// Get the rest of this sequence (everything after first).
    /// Returns None if there are no more elements.
    pub fn next(&self) -> Option<Seq> {
        match self {
            Seq::ConsBased(cell) => match &cell.cdr {
                Value::Cons(next_cell) => Some(Seq::ConsBased(next_cell.clone())),
                Value::Nil => None,
                _ => None,
            },
            Seq::VectorSeq { vec, index } => {
                let next_index = index + 1;
                if next_index < vec.count() {
                    Some(Seq::VectorSeq {
                        vec: vec.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::PersistentVectorSeq { vec, index } => {
                let next_index = index + 1;
                if next_index < vec.count() {
                    Some(Seq::PersistentVectorSeq {
                        vec: vec.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::MapSeq { entries, index } => {
                let next_index = index + 1;
                if next_index < entries.len() {
                    Some(Seq::MapSeq {
                        entries: entries.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::PersistentMapSeq { entries, index } => {
                let next_index = index + 1;
                if next_index < entries.len() {
                    Some(Seq::PersistentMapSeq {
                        entries: entries.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::SetSeq { elements, index } => {
                let next_index = index + 1;
                if next_index < elements.len() {
                    Some(Seq::SetSeq {
                        elements: elements.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::PersistentSetSeq { elements, index } => {
                let next_index = index + 1;
                if next_index < elements.len() {
                    Some(Seq::PersistentSetSeq {
                        elements: elements.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
            Seq::StringSeq { chars, index } => {
                let next_index = index + 1;
                if next_index < chars.len() {
                    Some(Seq::StringSeq {
                        chars: chars.clone(),
                        index: next_index,
                    })
                } else {
                    None
                }
            }
        }
    }

    /// Convert this sequence to a proper list (cons cells).
    pub fn to_list(&self) -> Value {
        let mut result = Value::Nil;
        let mut items = Vec::new();

        // Collect all items
        let mut current = Some(self.clone());
        while let Some(seq) = current {
            items.push(seq.first());
            current = seq.next();
        }

        // Build list in reverse
        for item in items.into_iter().rev() {
            result = cons(item, result);
        }

        result
    }
}

// ============================================================================
// Public API Functions - Dispatching through traits
// ============================================================================

/// Convert a value to a sequence if possible.
pub fn seq(value: &Value) -> Option<Seq> {
    match value {
        Value::Nil => None,
        Value::Cons(cell) => Some(Seq::ConsBased(cell.clone())),
        Value::Vector(vec) => vec.to_seq(),
        Value::PersistentVector(vec) => vec.to_seq(),
        Value::Map(map) => map.to_seq(),
        Value::PersistentMap(map) => map.to_seq(),
        Value::Set(set) => set.to_seq(),
        Value::PersistentSet(set) => set.to_seq(),
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            let chars: Vec<char> = s.chars().collect();
            if chars.is_empty() {
                None
            } else {
                Some(Seq::StringSeq { chars, index: 0 })
            }
        }
        _ => None,
    }
}

/// Get the first element of a value.
pub fn first(value: &Value) -> Value {
    seq(value).map_or(Value::Nil, |s| s.first())
}

/// Get the rest of a value as a sequence (returns Nil if empty).
pub fn next(value: &Value) -> Value {
    seq(value)
        .and_then(|s| s.next())
        .map_or(Value::Nil, |s| s.to_list())
}

/// Like next but returns empty list instead of nil for empty.
pub fn rest(value: &Value) -> Value {
    seq(value)
        .and_then(|s| s.next())
        .map_or(Value::Nil, |s| s.to_list())
}

/// Count the number of elements in a collection.
/// Returns None for uncountable types.
pub fn count(value: &Value) -> Option<usize> {
    match value {
        Value::Nil => Some(0),
        Value::Cons(_) => {
            // O(n) for lists - traverse to count
            let mut count = 0;
            let mut current = value.clone();
            while let Value::Cons(cell) = current {
                count += 1;
                current = cell.cdr.clone();
            }
            Some(count)
        }
        Value::Vector(vec) => Some(vec.count()),
        Value::PersistentVector(vec) => Some(vec.count()),
        Value::Map(map) => Some(map.count()),
        Value::PersistentMap(map) => Some(map.count()),
        Value::Set(set) => Some(set.count()),
        Value::PersistentSet(set) => Some(set.count()),
        Value::Atom(AtomType::String(StringType::Basic(s))) => Some(s.chars().count()),
        _ => None,
    }
}

/// Get the nth element of a collection.
/// Returns default_val (or Nil) if index out of bounds.
pub fn nth(value: &Value, index: usize, default_val: Option<&Value>) -> Value {
    let default = default_val.cloned().unwrap_or(Value::Nil);
    match value {
        Value::Vector(vec) => vec.nth(index).unwrap_or(default),
        Value::PersistentVector(vec) => vec.nth(index).unwrap_or(default),
        Value::Cons(_) => {
            // O(n) traversal for lists
            let mut current = value.clone();
            for _ in 0..index {
                if let Value::Cons(cell) = current {
                    current = cell.cdr.clone();
                } else {
                    return default;
                }
            }
            if let Value::Cons(cell) = current {
                cell.car.clone()
            } else {
                default
            }
        }
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            s.chars().nth(index).map_or(default, |c| {
                Value::Atom(AtomType::String(StringType::Basic(c.to_string())))
            })
        }
        _ => default,
    }
}

/// Get a value by key from a collection.
/// Returns default_val (or Nil) if key not found.
pub fn get(coll: &Value, key: &Value, default_val: Option<&Value>) -> Value {
    let default = default_val.cloned().unwrap_or(Value::Nil);
    match coll {
        Value::Map(map) => map.get_value(key).unwrap_or(default),
        Value::PersistentMap(map) => map.get_value(key).unwrap_or(default),
        Value::Set(set) => set.get_value(key).unwrap_or(default),
        Value::PersistentSet(set) => set.get_value(key).unwrap_or(default),
        Value::Vector(vec) => vec.get_value(key).unwrap_or(default),
        Value::PersistentVector(vec) => vec.get_value(key).unwrap_or(default),
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            // String lookup by integer index
            if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = key {
                if *idx >= 0 {
                    s.chars().nth(*idx as usize).map_or(default, |c| {
                        Value::Atom(AtomType::String(StringType::Basic(c.to_string())))
                    })
                } else {
                    default
                }
            } else {
                default
            }
        }
        _ => default,
    }
}

/// Associate a key with a value in a collection.
/// Returns a new collection with the association.
pub fn assoc(coll: &Value, key: Value, val: Value) -> Result<Value, String> {
    match coll {
        Value::Map(map) => Ok(Value::Map(Arc::new(map.assoc(key, val)?))),
        Value::PersistentMap(map) => Ok(Value::PersistentMap(Arc::new(map.assoc(key, val)?))),
        Value::Vector(vec) => Ok(Value::Vector(Arc::new(vec.assoc(key, val)?))),
        Value::PersistentVector(vec) => Ok(Value::PersistentVector(Arc::new(vec.assoc(key, val)?))),
        Value::Nil => {
            // Assoc on nil creates a new fast map
            let mut entries = FxHashMap::default();
            entries.insert(key, val);
            Ok(Value::Map(Arc::new(MapValue { entries })))
        }
        _ => Err(format!("Cannot assoc on {}", coll)),
    }
}

/// Add an item to a collection.
/// Behavior depends on collection type:
/// - List: adds at front
/// - Vector: adds at end
/// - Set: adds element
/// - Map: expects a [key value] pair
pub fn conj(coll: &Value, item: Value) -> Result<Value, String> {
    match coll {
        Value::Nil => {
            // Conj on nil creates a list
            Ok(cons(item, Value::Nil))
        }
        Value::Cons(_) => {
            // Add at front (like Clojure)
            Ok(cons(item, coll.clone()))
        }
        Value::Vector(vec) => Ok(Value::Vector(Arc::new(vec.conj(item)?))),
        Value::PersistentVector(vec) => Ok(Value::PersistentVector(Arc::new(vec.conj(item)?))),
        Value::Set(set) => Ok(Value::Set(Arc::new(set.conj(item)?))),
        Value::PersistentSet(set) => Ok(Value::PersistentSet(Arc::new(set.conj(item)?))),
        Value::Map(map) => Ok(Value::Map(Arc::new(map.conj(item)?))),
        Value::PersistentMap(map) => Ok(Value::PersistentMap(Arc::new(map.conj(item)?))),
        _ => Err(format!("Cannot conj onto {}", coll)),
    }
}

// ============================================================================
// Reduced - Early termination in folds/reductions
// ============================================================================

/// Check if a value is a Reduced wrapper.
pub fn is_reduced(value: &Value) -> bool {
    matches!(value, Value::Reduced(_))
}

/// Wrap a value in Reduced for early termination.
pub fn reduced(value: Value) -> Value {
    Value::Reduced(Box::new(value))
}

/// Unwrap a Reduced value, or return the value unchanged if not reduced.
pub fn unreduced(value: &Value) -> Value {
    match value {
        Value::Reduced(inner) => (**inner).clone(),
        _ => value.clone(),
    }
}

// ============================================================================
// Constructor helpers - Fast collections
// ============================================================================

/// Create an empty fast map.
pub fn empty_map() -> Value {
    Value::Map(Arc::new(MapValue {
        entries: FxHashMap::default(),
    }))
}

/// Create a fast map from key-value pairs.
pub fn hash_map(pairs: Vec<(Value, Value)>) -> Value {
    let mut entries = FxHashMap::default();
    for (k, v) in pairs {
        entries.insert(k, v);
    }
    Value::Map(Arc::new(MapValue { entries }))
}

/// Create an empty fast set.
pub fn empty_set() -> Value {
    Value::Set(Arc::new(SetValue {
        elements: FxHashSet::default(),
    }))
}

/// Create a fast set from elements.
pub fn hash_set(elements: Vec<Value>) -> Value {
    let elems: FxHashSet<Value> = elements.into_iter().collect();
    Value::Set(Arc::new(SetValue { elements: elems }))
}

/// Create an empty fast vector.
pub fn empty_vector() -> Value {
    Value::Vector(Arc::new(VectorValue { elements: vec![] }))
}

/// Create a fast vector from elements.
pub fn vector(elements: Vec<Value>) -> Value {
    Value::Vector(Arc::new(VectorValue { elements }))
}

// ============================================================================
// Constructor helpers - Persistent collections
// ============================================================================

/// Create an empty persistent map.
pub fn empty_persistent_map() -> Value {
    Value::PersistentMap(Arc::new(PersistentMap {
        entries: ImHashMap::new(),
    }))
}

/// Create a persistent map from key-value pairs.
pub fn persistent_hash_map(pairs: Vec<(Value, Value)>) -> Value {
    let entries: ImHashMap<Value, Value> = pairs.into_iter().collect();
    Value::PersistentMap(Arc::new(PersistentMap { entries }))
}

/// Create an empty persistent set.
pub fn empty_persistent_set() -> Value {
    Value::PersistentSet(Arc::new(PersistentSet {
        elements: ImHashSet::new(),
    }))
}

/// Create a persistent set from elements.
pub fn persistent_hash_set(elements: Vec<Value>) -> Value {
    let elems: ImHashSet<Value> = elements.into_iter().collect();
    Value::PersistentSet(Arc::new(PersistentSet { elements: elems }))
}

/// Create an empty persistent vector.
pub fn empty_persistent_vector() -> Value {
    Value::PersistentVector(Arc::new(PersistentVector {
        elements: ImVector::new(),
    }))
}

/// Create a persistent vector from elements.
pub fn persistent_vector(elements: Vec<Value>) -> Value {
    let elems: ImVector<Value> = elements.into_iter().collect();
    Value::PersistentVector(Arc::new(PersistentVector { elements: elems }))
}

// ============================================================================
// Callable abstraction - IFn-like behavior
// ============================================================================

/// Check if a value is callable (can be invoked as a function).
pub fn is_callable(value: &Value) -> bool {
    matches!(
        value,
        Value::Lambda(_)
            | Value::NativeFn(_)
            | Value::Map(_)
            | Value::PersistentMap(_)
            | Value::Set(_)
            | Value::PersistentSet(_)
            | Value::Vector(_)
            | Value::PersistentVector(_)
            | Value::Atom(AtomType::Symbol(SymbolType::Symbol(_)))
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::interner::InternedSymbol;

    fn make_symbol(s: &str) -> Value {
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(s))))
    }

    fn make_int(n: i64) -> Value {
        Value::Atom(AtomType::Number(NumericType::Int(n)))
    }

    fn make_string(s: &str) -> Value {
        Value::Atom(AtomType::String(StringType::Basic(s.to_string())))
    }

    #[test]
    fn test_seq_list() {
        let list = cons(
            make_int(1),
            cons(make_int(2), cons(make_int(3), Value::Nil)),
        );
        let s = seq(&list).unwrap();
        assert_eq!(s.first(), make_int(1));
        let s2 = s.next().unwrap();
        assert_eq!(s2.first(), make_int(2));
        let s3 = s2.next().unwrap();
        assert_eq!(s3.first(), make_int(3));
        assert!(s3.next().is_none());
    }

    #[test]
    fn test_seq_vector() {
        let vec = vector(vec![make_int(1), make_int(2), make_int(3)]);
        let s = seq(&vec).unwrap();
        assert_eq!(s.first(), make_int(1));
        let s2 = s.next().unwrap();
        assert_eq!(s2.first(), make_int(2));
    }

    #[test]
    fn test_seq_persistent_vector() {
        let vec = persistent_vector(vec![make_int(1), make_int(2), make_int(3)]);
        let s = seq(&vec).unwrap();
        assert_eq!(s.first(), make_int(1));
        let s2 = s.next().unwrap();
        assert_eq!(s2.first(), make_int(2));
    }

    #[test]
    fn test_count() {
        let list = cons(make_int(1), cons(make_int(2), Value::Nil));
        assert_eq!(count(&list), Some(2));

        let vec = vector(vec![make_int(1), make_int(2), make_int(3)]);
        assert_eq!(count(&vec), Some(3));

        let pvec = persistent_vector(vec![make_int(1), make_int(2)]);
        assert_eq!(count(&pvec), Some(2));

        assert_eq!(count(&Value::Nil), Some(0));
    }

    #[test]
    fn test_nth() {
        let vec = vector(vec![make_int(10), make_int(20), make_int(30)]);
        assert_eq!(nth(&vec, 0, None), make_int(10));
        assert_eq!(nth(&vec, 1, None), make_int(20));
        assert_eq!(nth(&vec, 2, None), make_int(30));
        assert_eq!(nth(&vec, 3, None), Value::Nil);

        let pvec = persistent_vector(vec![make_int(100), make_int(200)]);
        assert_eq!(nth(&pvec, 0, None), make_int(100));
        assert_eq!(nth(&pvec, 1, None), make_int(200));
    }

    #[test]
    fn test_get_map() {
        let map = hash_map(vec![
            (make_symbol("a"), make_int(1)),
            (make_symbol("b"), make_int(2)),
        ]);
        assert_eq!(get(&map, &make_symbol("a"), None), make_int(1));
        assert_eq!(get(&map, &make_symbol("b"), None), make_int(2));
        assert_eq!(get(&map, &make_symbol("c"), None), Value::Nil);
    }

    #[test]
    fn test_get_persistent_map() {
        let map = persistent_hash_map(vec![
            (make_symbol("x"), make_int(10)),
            (make_symbol("y"), make_int(20)),
        ]);
        assert_eq!(get(&map, &make_symbol("x"), None), make_int(10));
        assert_eq!(get(&map, &make_symbol("y"), None), make_int(20));
        assert_eq!(get(&map, &make_symbol("z"), None), Value::Nil);
    }

    #[test]
    fn test_get_vector() {
        let vec = vector(vec![make_int(10), make_int(20)]);
        assert_eq!(get(&vec, &make_int(0), None), make_int(10));
        assert_eq!(get(&vec, &make_int(1), None), make_int(20));
    }

    #[test]
    fn test_assoc_map() {
        let map = empty_map();
        let map2 = assoc(&map, make_symbol("a"), make_int(1)).unwrap();
        assert_eq!(get(&map2, &make_symbol("a"), None), make_int(1));
    }

    #[test]
    fn test_assoc_persistent_map() {
        let map = empty_persistent_map();
        let map2 = assoc(&map, make_symbol("a"), make_int(1)).unwrap();
        assert_eq!(get(&map2, &make_symbol("a"), None), make_int(1));
        // Original unchanged
        assert_eq!(count(&map), Some(0));
    }

    #[test]
    fn test_assoc_vector() {
        let vec = vector(vec![make_int(1), make_int(2)]);
        let vec2 = assoc(&vec, make_int(0), make_int(10)).unwrap();
        assert_eq!(nth(&vec2, 0, None), make_int(10));
        assert_eq!(nth(&vec2, 1, None), make_int(2));
    }

    #[test]
    fn test_conj_list() {
        let list = cons(make_int(2), cons(make_int(3), Value::Nil));
        let list2 = conj(&list, make_int(1)).unwrap();
        // List conj adds at front
        assert_eq!(first(&list2), make_int(1));
    }

    #[test]
    fn test_conj_vector() {
        let vec = vector(vec![make_int(1), make_int(2)]);
        let vec2 = conj(&vec, make_int(3)).unwrap();
        // Vector conj adds at end
        assert_eq!(nth(&vec2, 2, None), make_int(3));
    }

    #[test]
    fn test_conj_persistent_vector() {
        let vec = persistent_vector(vec![make_int(1), make_int(2)]);
        let vec2 = conj(&vec, make_int(3)).unwrap();
        // Persistent vector conj adds at end
        assert_eq!(nth(&vec2, 2, None), make_int(3));
        // Original unchanged
        assert_eq!(count(&vec), Some(2));
    }

    #[test]
    fn test_conj_set() {
        let set = empty_set();
        let set2 = conj(&set, make_int(1)).unwrap();
        let set3 = conj(&set2, make_int(2)).unwrap();
        assert_eq!(count(&set3), Some(2));
    }

    #[test]
    fn test_conj_persistent_set() {
        let set = empty_persistent_set();
        let set2 = conj(&set, make_int(1)).unwrap();
        let set3 = conj(&set2, make_int(2)).unwrap();
        assert_eq!(count(&set3), Some(2));
        // Original unchanged
        assert_eq!(count(&set), Some(0));
    }

    #[test]
    fn test_reduced() {
        let val = make_int(42);
        let r = reduced(val.clone());
        assert!(is_reduced(&r));
        assert!(!is_reduced(&val));
        assert_eq!(unreduced(&r), val);
    }

    #[test]
    fn test_seq_string() {
        let s = make_string("abc");
        let seq_s = seq(&s).unwrap();
        assert_eq!(seq_s.first(), make_string("a"));
        let seq_s2 = seq_s.next().unwrap();
        assert_eq!(seq_s2.first(), make_string("b"));
    }

    #[test]
    fn test_first_next() {
        let list = cons(
            make_int(1),
            cons(make_int(2), cons(make_int(3), Value::Nil)),
        );
        assert_eq!(first(&list), make_int(1));
        let rest_list = next(&list);
        assert_eq!(first(&rest_list), make_int(2));
    }

    #[test]
    fn test_persistent_immutability() {
        // Test that persistent operations don't modify originals
        let vec1 = persistent_vector(vec![make_int(1), make_int(2)]);
        let vec2 = conj(&vec1, make_int(3)).unwrap();

        assert_eq!(count(&vec1), Some(2)); // Original unchanged
        assert_eq!(count(&vec2), Some(3)); // New has 3

        let map1 = empty_persistent_map();
        let map2 = assoc(&map1, make_symbol("a"), make_int(1)).unwrap();

        assert_eq!(count(&map1), Some(0)); // Original unchanged
        assert_eq!(count(&map2), Some(1)); // New has 1
    }
}
