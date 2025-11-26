//! AOT (Ahead-of-Time) compilation module.
//!
//! This module provides AOT compilation capabilities for Consair,
//! allowing Lisp source code to be compiled to LLVM IR that can
//! then be compiled to native code using standard LLVM tools.
//!
//! # Example
//!
//! ```ignore
//! use consair::aot::AotCompiler;
//! use std::path::Path;
//!
//! let compiler = AotCompiler::new();
//! compiler.compile_file(
//!     Path::new("input.lisp"),
//!     Some(Path::new("output.ll"))
//! ).unwrap();
//! ```
//!
//! The resulting `.ll` file can be compiled with clang:
//!
//! ```bash
//! clang -O3 output.ll -o output
//! ```

mod compiler;
mod runtime_ir;

pub use compiler::{AotCompiler, AotError};
