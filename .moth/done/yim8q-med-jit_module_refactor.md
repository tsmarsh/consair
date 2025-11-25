# Refactor JIT Module into Multiple Files

## Problem

The JIT implementation is contained in a single 3669-line file (`consair-core/src/jit.rs`). This monolithic structure makes the code harder to navigate, understand, and maintain. Different concerns (error handling, caching, analysis, compilation) are intermixed.

## Impact

- Difficult to navigate and understand JIT code
- Hard to find specific functionality
- Cognitive overhead when making changes
- Harder for new contributors to understand the codebase
- Merge conflicts more likely when multiple changes touch the same file

## Prompt for Implementation

```
Refactor the JIT module from a single file into a proper module structure:

Current state:
- `consair-core/src/jit.rs` - 3669 lines containing everything
- `consair-core/src/codegen.rs` - 481 lines (already separate)
- `consair-core/src/runtime.rs` - 2289 lines (already separate)

Target structure - create `consair-core/src/jit/` directory:

1. **jit/error.rs** (~80 lines)
   - `JitErrorKind` enum
   - `JitError` struct
   - `impl Display`, `impl Error`, `impl From<JitError> for String`
   - Error constructors: `unsupported()`, `syntax()`, `unbound()`, etc.

2. **jit/cache.rs** (~100 lines)
   - `CacheConfig` struct
   - `CacheStats` struct
   - `hash_expression()` function
   - `is_pure_expression()` function

3. **jit/analysis.rs** (~200 lines)
   - `find_free_variables()` function
   - `find_free_vars_helper()` function
   - `is_builtin()` function
   - `collect_list()` helper function

4. **jit/compiled.rs** (~50 lines)
   - `CompiledExpr` struct
   - `impl CompiledExpr` with `execute()` method

5. **jit/engine.rs** (~2000+ lines)
   - Type aliases: `JitEnv`, `LambdaStore`, `CompiledFns`
   - `ExprFn` type alias
   - `JitEngine` struct
   - All `impl JitEngine` methods (compile, eval, compile_*, etc.)
   - `impl Default for JitEngine`

6. **jit/mod.rs** (re-exports)
   ```rust
   mod analysis;
   mod cache;
   mod compiled;
   mod engine;
   mod error;

   pub use cache::{CacheConfig, CacheStats};
   pub use compiled::CompiledExpr;
   pub use engine::JitEngine;
   pub use error::{JitError, JitErrorKind};
   ```

7. **jit/tests.rs** (optional - tests can stay in engine.rs or move here)
   - All `#[cfg(test)]` test functions

**Implementation Steps:**

1. Create `consair-core/src/jit/` directory
2. Create each submodule file with appropriate `use` statements
3. Move relevant code to each file
4. Update imports in each file to reference sibling modules
5. Create `mod.rs` with re-exports
6. Update `consair-core/src/lib.rs` to use `pub mod jit;` instead of file
7. Ensure all existing tests pass
8. Ensure `pub use jit::{...}` in lib.rs still works

**Key Considerations:**

- Maintain all existing public API unchanged
- Keep `#[cfg(feature = "jit")]` guards in lib.rs
- Ensure internal types used across modules have correct visibility
- Run `cargo test --features jit` after each major move
- Run `cargo clippy --features jit` to catch visibility issues

**Testing:**

- All existing JIT tests must pass
- All integration tests must pass
- All benchmarks must work
- CI must pass
```

## Success Criteria

- [ ] JIT code split into logical modules
- [ ] `jit/error.rs` contains error types
- [ ] `jit/cache.rs` contains caching logic
- [ ] `jit/analysis.rs` contains analysis functions
- [ ] `jit/compiled.rs` contains CompiledExpr
- [ ] `jit/engine.rs` contains JitEngine
- [ ] `jit/mod.rs` re-exports public API
- [ ] All existing tests pass
- [ ] Public API unchanged (lib.rs exports same items)
- [ ] CI passes (tests, clippy, fmt)
- [ ] Benchmarks still work
