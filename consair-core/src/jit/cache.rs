//! JIT caching logic for avoiding recompilation of pure expressions.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use crate::language::{AtomType, SymbolType, Value};

/// Compute a hash of an expression for cache lookup.
pub fn hash_expression(expr: &Value) -> u64 {
    let mut hasher = DefaultHasher::new();
    // Use the Display representation for hashing
    format!("{}", expr).hash(&mut hasher);
    hasher.finish()
}

/// Check if an expression is pure (no side effects, no free variables).
/// Pure expressions can have their results cached.
pub fn is_pure_expression(expr: &Value) -> bool {
    match expr {
        Value::Nil => true,
        Value::Atom(AtomType::Number(_)) => true,
        Value::Atom(AtomType::String(_)) => true,
        Value::Atom(AtomType::Bool(_)) => true,
        // Symbols are not pure - they reference variables
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(_))) => false,
        Value::Cons(cell) => {
            // Check if operator is a pure function
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = &cell.car {
                let name = sym.resolve();

                // Quote is always pure - it returns its argument unevaluated
                if name.as_str() == "quote" {
                    return true;
                }

                // Pure operators that don't have side effects
                let pure_ops = [
                    "+",
                    "-",
                    "*",
                    "/",
                    "=",
                    "<",
                    ">",
                    "<=",
                    ">=",
                    "eq",
                    "atom",
                    "nil?",
                    "number?",
                    "cons?",
                    "not",
                    "cons",
                    "car",
                    "cdr",
                    "cond",
                    "vector",
                    "vector-length",
                    "vector-ref",
                    "length",
                    "append",
                    "reverse",
                    "nth",
                    "t",
                    "nil",
                ];
                if pure_ops.contains(&name.as_str()) {
                    // Check all arguments are pure
                    let mut current = cell.cdr.clone();
                    while let Value::Cons(arg_cell) = current {
                        if !is_pure_expression(&arg_cell.car) {
                            return false;
                        }
                        current = arg_cell.cdr.clone();
                    }
                    return true;
                }
            }
            false
        }
        Value::Lambda(_) => false,
        Value::Macro(_) => false,
        Value::NativeFn(_) => false,
        Value::Vector(v) => v.elements.iter().all(is_pure_expression),
        Value::Map(m) => m
            .entries
            .iter()
            .all(|(k, v)| is_pure_expression(k) && is_pure_expression(v)),
        Value::Set(s) => s.elements.iter().all(is_pure_expression),
        Value::Reduced(v) => is_pure_expression(v),
    }
}

/// Configuration for JIT compilation caching.
#[derive(Clone, Debug)]
pub struct CacheConfig {
    /// Enable caching of pure expression results
    pub enabled: bool,
    /// Maximum number of entries in the cache
    pub max_entries: usize,
}

impl Default for CacheConfig {
    fn default() -> Self {
        CacheConfig {
            enabled: true,
            max_entries: 1000,
        }
    }
}

/// Statistics about JIT cache usage.
#[derive(Clone, Debug, Default)]
pub struct CacheStats {
    /// Number of cache hits
    pub hits: usize,
    /// Number of cache misses
    pub misses: usize,
    /// Number of compilations avoided
    pub compilations_avoided: usize,
}
