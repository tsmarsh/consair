//! Consair AOT Compiler
//!
//! This crate provides ahead-of-time compilation for Consair Lisp,
//! generating LLVM IR that can be compiled to native executables.

pub mod aot;

// Re-export AOT types
pub use aot::{AotCompiler, AotError};
