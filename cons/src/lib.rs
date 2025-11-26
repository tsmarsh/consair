//! Consair runtime - Interpreter and JIT compiler
//!
//! This crate provides the runtime execution engines for Consair:
//! - Tree-walking interpreter
//! - JIT compiler using LLVM
//! - Standard library functions
//! - Runtime helpers for compiled code

pub mod codegen;
pub mod interpreter;
pub mod jit;
pub mod native;
pub mod runtime;
pub mod stdlib;

// Re-export JIT types
pub use jit::{CompiledExpr, JitError, JitErrorKind};

// Re-export interpreter types
pub use interpreter::{Environment, eval, expand_all_macros, expand_macros};

// Re-export stdlib registration
pub use stdlib::register_stdlib;

// Re-export codegen for cadr to use
pub use codegen::Codegen;
