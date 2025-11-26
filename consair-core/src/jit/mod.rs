//! JIT execution engine for Consair.
//!
//! This module provides the ability to compile and immediately execute
//! Consair expressions using LLVM's JIT compilation.
//!
//! ## Caching
//!
//! The JIT engine supports optional expression caching to avoid recompiling
//! the same expression multiple times. When caching is enabled:
//! - Expressions are normalized to a canonical string form
//! - A hash of the normalized expression is used as the cache key
//! - Compiled function pointers and execution engines are cached
//! - Subsequent evaluations of the same expression reuse the cached code
//!
//! ## Error Handling
//!
//! JIT compilation errors are categorized into different types:
//! - `UnsupportedExpression`: Expression type not yet supported by JIT
//! - `UnsupportedType`: Data type not supported (e.g., BigInt)
//! - `InvalidSyntax`: Malformed expression structure
//! - `UnboundVariable`: Reference to undefined variable
//! - `CompilationError`: LLVM compilation failure
//! - `ExecutionError`: Runtime execution failure

pub mod analysis;
mod cache;
mod compiled;
mod engine;
mod error;

pub use cache::{CacheConfig, CacheStats};
pub use compiled::CompiledExpr;
pub use engine::JitEngine;
pub use error::{JitError, JitErrorKind};
