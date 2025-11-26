# AOT Compiler for Consair

## Goal
Add ahead-of-time (AOT) compilation that outputs LLVM IR files (`.ll`), which can then be compiled to native code using standard LLVM tools (`llc`, `clang`). The primary use case is performance - avoiding JIT compilation overhead.

## Approach: LLVM IR Emission with Embedded Runtime

Leverage the existing JIT codegen infrastructure to emit standalone LLVM IR files that include all runtime function definitions. This creates self-contained `.ll` files that can be compiled offline.

**Rationale**: Starting with IR emission is the simplest path - `Codegen::emit_ir()` already exists. Static runtime embedding avoids shared library complexity. Users can use `clang` to produce optimized binaries.

## Architecture

```
Source (.lisp)
    ↓
parse() → Value AST
    ↓
AotCompiler::compile()
    ├─ Codegen::new() - Create LLVM context/module
    ├─ emit_runtime_definitions() - Embed all rt_* functions as LLVM IR
    ├─ compile_value() - Generate IR for expressions (reuse JIT logic)
    ├─ generate_main() - Create entry point that calls compiled code
    └─ emit_ir() → .ll file
    ↓
External: clang -O3 output.ll -o output
```

## Implementation Steps

### 1. Create the cadr binary crate
```
cadr/
├── Cargo.toml
└── src/
    └── main.rs
```

**Cargo.toml:**
```toml
[package]
name = "cadr"
version = "0.1.0"
edition = "2021"
description = "AOT compiler for Consair Lisp"

[[bin]]
name = "cadr"
path = "src/main.rs"

[dependencies]
consair = { path = "../consair-core" }
```

### 2. Create AOT module in consair-core
```
consair-core/src/aot/
├── mod.rs         # Exports AotCompiler, AotError
├── compiler.rs    # Main AotCompiler implementation
└── runtime_ir.rs  # LLVM IR definitions for runtime functions
```

### 3. Implement runtime_ir.rs - LLVM IR for runtime functions

The runtime functions (rt_cons, rt_car, etc.) need to be emitted as LLVM IR rather than linked as external symbols.

**Approach: Hand-written LLVM IR templates**
- Define each rt_* function as LLVM IR text
- Embed in Rust as string constants
- Append to generated module

Key runtime functions to implement in IR:
- `rt_cons`, `rt_car`, `rt_cdr` - List operations
- `rt_add`, `rt_sub`, `rt_mul`, `rt_div` - Arithmetic
- `rt_lt`, `rt_gt`, `rt_eq` etc. - Comparisons
- `rt_incref`, `rt_decref` - Reference counting
- `rt_make_closure`, `rt_closure_fn_ptr`, `rt_closure_env_get` - Closures

### 4. Implement AotCompiler in compiler.rs

```rust
pub struct AotCompiler {
    optimization_level: OptimizationLevel,
}

impl AotCompiler {
    pub fn new() -> Self;

    /// Compile a Lisp source file to LLVM IR
    pub fn compile_file(&self, input: &Path, output: &Path) -> Result<(), AotError>;

    /// Compile a single expression to LLVM IR
    pub fn compile_expr(&self, expr: &Value) -> Result<String, AotError>;
}
```

**Key implementation details:**
- Reuse `JitEngine::compile_value()` logic for expression compilation
- Generate a `main` function that:
  1. Calls the compiled expression
  2. Prints result based on RuntimeValue tag
  3. Returns 0 or error code
- Prepend runtime function IR definitions

### 5. Implement cadr/src/main.rs

```rust
use consair::aot::AotCompiler;
use std::env;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();

    // cadr input.lisp              -> output to stdout
    // cadr input.lisp -o output.ll -> output to file

    let input = args.get(1).expect("Usage: cadr <input.lisp> [-o output.ll]");
    let output = if args.get(2) == Some(&"-o".to_string()) {
        args.get(3).map(|s| s.as_str())
    } else {
        None
    };

    let compiler = AotCompiler::new();
    match compiler.compile_file(Path::new(input), output.map(Path::new)) {
        Ok(()) => {}
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }
}
```

### 6. Update workspace Cargo.toml

Add `cadr` to workspace members:
```toml
[workspace]
members = ["consair-core", "cons", "cadr"]
```

### 7. Update consair-core/src/lib.rs

```rust
pub mod aot;
pub use aot::{AotCompiler, AotError};
```

## File Changes Summary

| File | Change |
|------|--------|
| `Cargo.toml` (workspace) | Add `cadr` to members |
| `cadr/Cargo.toml` | New - cadr binary crate |
| `cadr/src/main.rs` | New - CLI for AOT compiler |
| `consair-core/src/aot/mod.rs` | New - module exports |
| `consair-core/src/aot/compiler.rs` | New - AotCompiler implementation |
| `consair-core/src/aot/runtime_ir.rs` | New - LLVM IR for runtime functions |
| `consair-core/src/lib.rs` | Add `pub mod aot;` |

## Runtime IR Strategy

The runtime functions will be defined in LLVM IR directly. Key structures:

```llvm
; RuntimeValue type: { i8 tag, i64 data }
%RuntimeValue = type { i8, i64 }

; Tag constants
@TAG_NIL = constant i8 0
@TAG_BOOL = constant i8 1
@TAG_INT = constant i8 2
@TAG_FLOAT = constant i8 3
@TAG_CONS = constant i8 4

; RuntimeConsCell: { %RuntimeValue car, %RuntimeValue cdr, i32 refcount }
%RuntimeConsCell = type { %RuntimeValue, %RuntimeValue, i32 }
```

## Usage Workflow

```bash
# Compile Lisp to LLVM IR
cadr factorial.lisp -o factorial.ll

# Compile IR to native (using system LLVM)
clang -O3 factorial.ll -o factorial

# Run
./factorial
```

## Testing Strategy

1. **Unit tests**: `AotCompiler::compile_expr()` produces valid IR
2. **Integration tests**: Compile + run with clang, verify output
3. **Comparison tests**: AOT result matches interpreter result

## Critical Files to Reference

- `consair-core/src/jit/engine.rs` - `compile_value()` logic to reuse
- `consair-core/src/codegen.rs` - LLVM IR generation, `emit_ir()`
- `consair-core/src/runtime.rs` - Runtime function signatures
- `cons/src/main.rs` - CLI structure example
