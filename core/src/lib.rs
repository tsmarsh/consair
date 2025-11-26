//! Core language definition for Consair
//!
//! This crate contains the fundamental types, parser, and abstractions
//! for the Consair Lisp language. It does not include execution engines
//! (interpreter, JIT, AOT) - those are in the `cons` and `cadr` crates.

pub mod abstractions;
pub mod environment;
pub mod interner;
pub mod language;
pub mod lexer;
pub mod numeric;
pub mod parser;

// Re-export commonly used items for convenience
pub use abstractions::{
    Seq, assoc, conj, count, first, get, hash_map, hash_set, is_callable, is_reduced, next, nth,
    reduced, rest, seq, unreduced,
};
pub use environment::Environment;
pub use interner::InternedSymbol;
pub use language::{
    AtomType, ConsCell, LambdaCell, MacroCell, MapValue, NativeFn, PersistentMap, PersistentSet,
    PersistentVector, SetValue, StringType, SymbolType, Value, VectorValue, cons,
};
pub use numeric::NumericType;
pub use parser::parse;
