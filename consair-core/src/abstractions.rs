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

use rustc_hash::{FxHashMap, FxHashSet};

use crate::language::{
    AtomType, ConsCell, MapValue, SetValue, StringType, SymbolType, Value, VectorValue, cons,
};
use crate::numeric::NumericType;

// ============================================================================
// Seq Abstraction - Uniform iteration over values
// ============================================================================

/// A sequence - lazy or realized view of sequential elements.
#[derive(Clone, Debug)]
pub enum Seq {
    /// A cons-based sequence (linked list)
    ConsBased(Arc<ConsCell>),
    /// A vector being iterated (with current index)
    VectorSeq { vec: Arc<VectorValue>, index: usize },
    /// A map being iterated (as key-value pairs)
    MapSeq {
        entries: Vec<(Value, Value)>,
        index: usize,
    },
    /// A set being iterated
    SetSeq { elements: Vec<Value>, index: usize },
    /// A string being iterated (as characters)
    StringSeq { chars: Vec<char>, index: usize },
}

impl Seq {
    /// Get the first element of this sequence.
    pub fn first(&self) -> Value {
        match self {
            Seq::ConsBased(cell) => cell.car.clone(),
            Seq::VectorSeq { vec, index } => {
                vec.elements.get(*index).cloned().unwrap_or(Value::Nil)
            }
            Seq::MapSeq { entries, index } => {
                if let Some((k, v)) = entries.get(*index) {
                    // Return as a two-element vector [key value]
                    Value::Vector(Arc::new(VectorValue {
                        elements: vec![k.clone(), v.clone()],
                    }))
                } else {
                    Value::Nil
                }
            }
            Seq::SetSeq { elements, index } => elements.get(*index).cloned().unwrap_or(Value::Nil),
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
                if next_index < vec.elements.len() {
                    Some(Seq::VectorSeq {
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
// Seqable - Types that can produce a Seq
// ============================================================================

/// Convert a value to a sequence if possible.
pub fn seq(value: &Value) -> Option<Seq> {
    match value {
        Value::Nil => None,
        Value::Cons(cell) => Some(Seq::ConsBased(cell.clone())),
        Value::Vector(vec) => {
            if vec.elements.is_empty() {
                None
            } else {
                Some(Seq::VectorSeq {
                    vec: vec.clone(),
                    index: 0,
                })
            }
        }
        Value::Map(map) => {
            if map.entries.is_empty() {
                None
            } else {
                let entries: Vec<_> = map
                    .entries
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect();
                Some(Seq::MapSeq { entries, index: 0 })
            }
        }
        Value::Set(set) => {
            if set.elements.is_empty() {
                None
            } else {
                let elements: Vec<_> = set.elements.iter().cloned().collect();
                Some(Seq::SetSeq { elements, index: 0 })
            }
        }
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

// ============================================================================
// Counted - O(1) count when possible
// ============================================================================

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
        Value::Vector(vec) => Some(vec.elements.len()),
        Value::Map(map) => Some(map.entries.len()),
        Value::Set(set) => Some(set.elements.len()),
        Value::Atom(AtomType::String(StringType::Basic(s))) => Some(s.chars().count()),
        _ => None,
    }
}

// ============================================================================
// Indexed - Random access by index
// ============================================================================

/// Get the nth element of a collection.
/// Returns default_val (or Nil) if index out of bounds.
pub fn nth(value: &Value, index: usize, default_val: Option<&Value>) -> Value {
    let default = default_val.cloned().unwrap_or(Value::Nil);
    match value {
        Value::Vector(vec) => vec.elements.get(index).cloned().unwrap_or(default),
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

// ============================================================================
// Lookup - Keyed access (get semantics)
// ============================================================================

/// Get a value by key from a collection.
/// Returns default_val (or Nil) if key not found.
pub fn get(coll: &Value, key: &Value, default_val: Option<&Value>) -> Value {
    let default = default_val.cloned().unwrap_or(Value::Nil);
    match coll {
        Value::Map(map) => map.entries.get(key).cloned().unwrap_or(default),
        Value::Set(set) => {
            // For sets, get returns the element itself if present
            if set.elements.contains(key) {
                key.clone()
            } else {
                default
            }
        }
        Value::Vector(vec) => {
            // Vector lookup by integer index
            if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = key {
                if *idx >= 0 {
                    vec.elements.get(*idx as usize).cloned().unwrap_or(default)
                } else {
                    default
                }
            } else {
                default
            }
        }
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

// ============================================================================
// Associative - Keyed updates (assoc semantics)
// ============================================================================

/// Associate a key with a value in a collection.
/// Returns a new collection with the association.
pub fn assoc(coll: &Value, key: Value, val: Value) -> Result<Value, String> {
    match coll {
        Value::Map(map) => {
            let mut new_entries = map.entries.clone();
            new_entries.insert(key, val);
            Ok(Value::Map(Arc::new(MapValue {
                entries: new_entries,
            })))
        }
        Value::Vector(vec) => {
            if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = &key {
                let idx = *idx as usize;
                if idx <= vec.elements.len() {
                    let mut new_elements = vec.elements.clone();
                    if idx == vec.elements.len() {
                        new_elements.push(val);
                    } else {
                        new_elements[idx] = val;
                    }
                    Ok(Value::Vector(Arc::new(VectorValue {
                        elements: new_elements,
                    })))
                } else {
                    Err(format!(
                        "Index {} out of bounds for vector of length {}",
                        idx,
                        vec.elements.len()
                    ))
                }
            } else {
                Err("Vector assoc requires integer key".to_string())
            }
        }
        Value::Nil => {
            // Assoc on nil creates a new map
            let mut entries = FxHashMap::default();
            entries.insert(key, val);
            Ok(Value::Map(Arc::new(MapValue { entries })))
        }
        _ => Err(format!("Cannot assoc on {}", coll)),
    }
}

// ============================================================================
// Conj - Polymorphic insertion
// ============================================================================

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
        Value::Vector(vec) => {
            // Add at end
            let mut new_elements = vec.elements.clone();
            new_elements.push(item);
            Ok(Value::Vector(Arc::new(VectorValue {
                elements: new_elements,
            })))
        }
        Value::Set(set) => {
            let mut new_elements = set.elements.clone();
            new_elements.insert(item);
            Ok(Value::Set(Arc::new(SetValue {
                elements: new_elements,
            })))
        }
        Value::Map(map) => {
            // Expect item to be a [key value] vector or (key . value) cons
            match &item {
                Value::Vector(pair) if pair.elements.len() == 2 => {
                    let key = pair.elements[0].clone();
                    let val = pair.elements[1].clone();
                    let mut new_entries = map.entries.clone();
                    new_entries.insert(key, val);
                    Ok(Value::Map(Arc::new(MapValue {
                        entries: new_entries,
                    })))
                }
                Value::Cons(pair) => {
                    let key = pair.car.clone();
                    let val = pair.cdr.clone();
                    let mut new_entries = map.entries.clone();
                    new_entries.insert(key, val);
                    Ok(Value::Map(Arc::new(MapValue {
                        entries: new_entries,
                    })))
                }
                _ => Err("Map conj expects [key value] vector or (key . value) pair".to_string()),
            }
        }
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
// Constructor helpers
// ============================================================================

/// Create an empty map.
pub fn empty_map() -> Value {
    Value::Map(Arc::new(MapValue {
        entries: FxHashMap::default(),
    }))
}

/// Create a map from key-value pairs.
pub fn hash_map(pairs: Vec<(Value, Value)>) -> Value {
    let mut entries = FxHashMap::default();
    for (k, v) in pairs {
        entries.insert(k, v);
    }
    Value::Map(Arc::new(MapValue { entries }))
}

/// Create an empty set.
pub fn empty_set() -> Value {
    Value::Set(Arc::new(SetValue {
        elements: FxHashSet::default(),
    }))
}

/// Create a set from elements.
pub fn hash_set(elements: Vec<Value>) -> Value {
    let elems: FxHashSet<Value> = elements.into_iter().collect();
    Value::Set(Arc::new(SetValue { elements: elems }))
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
            | Value::Set(_)
            | Value::Vector(_)
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
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(1), make_int(2), make_int(3)],
        }));
        let s = seq(&vec).unwrap();
        assert_eq!(s.first(), make_int(1));
        let s2 = s.next().unwrap();
        assert_eq!(s2.first(), make_int(2));
    }

    #[test]
    fn test_count() {
        let list = cons(make_int(1), cons(make_int(2), Value::Nil));
        assert_eq!(count(&list), Some(2));

        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(1), make_int(2), make_int(3)],
        }));
        assert_eq!(count(&vec), Some(3));

        assert_eq!(count(&Value::Nil), Some(0));
    }

    #[test]
    fn test_nth() {
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(10), make_int(20), make_int(30)],
        }));
        assert_eq!(nth(&vec, 0, None), make_int(10));
        assert_eq!(nth(&vec, 1, None), make_int(20));
        assert_eq!(nth(&vec, 2, None), make_int(30));
        assert_eq!(nth(&vec, 3, None), Value::Nil);
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
    fn test_get_vector() {
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(10), make_int(20)],
        }));
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
    fn test_assoc_vector() {
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(1), make_int(2)],
        }));
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
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![make_int(1), make_int(2)],
        }));
        let vec2 = conj(&vec, make_int(3)).unwrap();
        // Vector conj adds at end
        assert_eq!(nth(&vec2, 2, None), make_int(3));
    }

    #[test]
    fn test_conj_set() {
        let set = empty_set();
        let set2 = conj(&set, make_int(1)).unwrap();
        let set3 = conj(&set2, make_int(2)).unwrap();
        assert_eq!(count(&set3), Some(2));
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
}
