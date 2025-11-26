pub mod abstractions;
pub mod interner;
pub mod interpreter;
pub mod language;
pub mod lexer;
pub mod native;
pub mod numeric;
pub mod parser;
pub mod stdlib;

// JIT compilation modules (requires LLVM 17.0)
pub mod codegen;
pub mod jit;
pub mod runtime;

// Re-export JIT types
pub use jit::{CompiledExpr, JitError, JitErrorKind};

// Re-export commonly used items for convenience
pub use abstractions::{
    Seq, assoc, conj, count, first, get, hash_map, hash_set, is_callable, is_reduced, next, nth,
    reduced, rest, seq, unreduced,
};
pub use interpreter::{Environment, eval, expand_all_macros, expand_macros};
pub use language::{
    AtomType, ConsCell, LambdaCell, MapValue, NativeFn, SetValue, Value, VectorValue, cons,
};
pub use numeric::NumericType;
pub use parser::parse;
pub use stdlib::register_stdlib;
