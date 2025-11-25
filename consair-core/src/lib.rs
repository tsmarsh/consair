pub mod interner;
pub mod interpreter;
pub mod language;
pub mod lexer;
pub mod native;
pub mod numeric;
pub mod parser;
pub mod stdlib;

// JIT compilation modules (optional)
#[cfg(feature = "jit")]
pub mod codegen;
#[cfg(feature = "jit")]
pub mod jit;
#[cfg(feature = "jit")]
pub mod runtime;

// Re-export commonly used items for convenience
pub use interpreter::{Environment, eval};
pub use language::{AtomType, ConsCell, LambdaCell, NativeFn, Value, VectorValue, cons};
pub use numeric::NumericType;
pub use parser::parse;
pub use stdlib::register_stdlib;
