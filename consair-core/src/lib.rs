pub mod interpreter;
pub mod language;
pub mod parser;

// Re-export commonly used items for convenience
pub use interpreter::{Environment, eval};
pub use language::{AtomType, ConsCell, LambdaCell, Value, cons};
pub use parser::parse;
