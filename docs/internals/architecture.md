# Architecture

Consair is implemented in Rust and provides three execution backends: interpreter, JIT compiler, and AOT compiler. All three share common infrastructure for parsing, types, and the standard library.

## High-Level Overview

```
                    ┌─────────────────┐
                    │  Source Code    │
                    │   (.lisp)       │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │     Lexer       │
                    │   (lexer.rs)    │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │     Parser      │
                    │  (parser.rs)    │
                    └────────┬────────┘
                             │
                    ┌────────▼────────┐
                    │      AST        │
                    │   (Value)       │
                    └────────┬────────┘
                             │
          ┌──────────────────┼──────────────────┐
          │                  │                  │
┌─────────▼─────────┐ ┌──────▼──────┐ ┌─────────▼─────────┐
│   Interpreter     │ │    JIT      │ │       AOT         │
│ (interpreter.rs)  │ │ (jit/*.rs)  │ │   (aot/*.rs)      │
└─────────┬─────────┘ └──────┬──────┘ └─────────┬─────────┘
          │                  │                  │
          │           ┌──────▼──────┐           │
          │           │    LLVM     │           │
          │           │   Codegen   │           │
          │           │(codegen.rs) │           │
          │           └──────┬──────┘           │
          │                  │                  │
┌─────────▼─────────┐ ┌──────▼──────┐ ┌─────────▼─────────┐
│    Result         │ │Machine Code │ │    LLVM IR        │
│   (Value)         │ │  (JIT)      │ │    (.ll file)     │
└───────────────────┘ └─────────────┘ └───────────────────┘
```

## Source Organization

```
consair/
├── consair-core/          # Core library
│   └── src/
│       ├── lib.rs         # Library entry point
│       ├── lexer.rs       # Tokenizer
│       ├── parser.rs      # S-expression parser
│       ├── language.rs    # Value types (AST)
│       ├── numeric.rs     # Numeric type system
│       ├── interner.rs    # Symbol interning
│       ├── interpreter.rs # Tree-walking interpreter
│       ├── stdlib.rs      # Standard library functions
│       ├── native.rs      # Native function helpers
│       ├── abstractions.rs# Collection abstractions
│       ├── runtime.rs     # Runtime value tags
│       ├── codegen.rs     # Shared LLVM code generation
│       ├── jit/           # JIT compiler
│       │   ├── mod.rs
│       │   ├── engine.rs  # JIT compilation engine
│       │   ├── cache.rs   # Compiled function cache
│       │   └── analysis.rs# Free variable analysis
│       └── aot/           # AOT compiler
│           ├── mod.rs
│           ├── compiler.rs# AOT compilation
│           └── runtime_ir.rs # Runtime IR generation
├── cons/                  # Interpreter/REPL binary
│   └── src/main.rs
└── cadr/                  # AOT compiler binary
    └── src/main.rs
```

## Core Types

### Value (language.rs)

The central AST type representing all Consair values:

```rust
pub enum Value {
    Atom(AtomType),              // Symbols, numbers, strings, bools
    Cons(Arc<ConsCell>),         // Pairs / list nodes
    Nil,                         // Empty list / false
    Lambda(Arc<LambdaCell>),     // Closures
    Macro(Arc<MacroCell>),       // Macros
    Vector(Arc<VectorValue>),    // Fast vectors
    Map(Arc<MapValue>),          // Hash maps
    Set(Arc<SetValue>),          // Hash sets
    PersistentVector(...),       // Immutable vector
    PersistentMap(...),          // Immutable map
    PersistentSet(...),          // Immutable set
    Reduced(Box<Value>),         // Early termination wrapper
    NativeFn(NativeFn),          // Built-in functions
}
```

### AtomType

```rust
pub enum AtomType {
    Symbol(SymbolType),   // Interned symbols
    Number(NumericType),  // All numeric types
    String(StringType),   // Strings
    Bool(bool),           // Booleans
}
```

### NumericType (numeric.rs)

```rust
pub enum NumericType {
    Int(i64),                    // Standard integer
    BigInt(Arc<BigInteger>),     // Arbitrary precision
    Ratio(i64, i64),             // Exact rational
    BigRatio(Arc<NumRatio>),     // Arbitrary precision rational
    Float(f64),                  // IEEE 754 double
}
```

## Interpreter

The interpreter (`interpreter.rs`) is a tree-walking evaluator that directly interprets the AST.

### Evaluation Model

```rust
pub fn eval(expr: Value, env: &mut Environment) -> Result<Value, String>
```

1. **Atoms**: Numbers, strings, bools self-evaluate
2. **Symbols**: Looked up in environment
3. **Lists**: First element determines handling:
   - Special form (`quote`, `if`, `cond`, `lambda`, `label`, `defmacro`)
   - Macro call (expand and re-evaluate)
   - Function call (evaluate args, apply function)

### Environment

Environments are implemented as a chain of frames with lexical scoping:

```rust
pub struct Environment {
    bindings: Arc<RwLock<HashMap<String, Value>>>,
    parent: Option<Box<Environment>>,
}
```

### Tail Call Optimization

The interpreter implements TCO for `cond` and `if` in tail position, preventing stack overflow in recursive functions.

## JIT Compiler

The JIT compiler (`jit/engine.rs`) compiles Consair expressions to machine code at runtime using LLVM.

### Architecture

```
┌─────────────┐    ┌─────────────┐    ┌─────────────┐
│   Value     │───▶│  Codegen    │───▶│ LLVM Module │
│   (AST)     │    │             │    │             │
└─────────────┘    └─────────────┘    └──────┬──────┘
                                             │
                                    ┌────────▼────────┐
                                    │ LLVM Execution  │
                                    │    Engine       │
                                    └────────┬────────┘
                                             │
                                    ┌────────▼────────┐
                                    │ RuntimeValue    │
                                    │   (result)      │
                                    └─────────────────┘
```

### RuntimeValue

JIT-compiled code uses a tagged union representation:

```rust
// 8-byte tag + 64-bit payload
pub struct RuntimeValue {
    tag: u8,    // Type tag
    data: u64,  // Value data (or pointer)
}

// Tags
pub const TAG_NIL: u8 = 0;
pub const TAG_BOOL: u8 = 1;
pub const TAG_INT: u8 = 2;
pub const TAG_CONS: u8 = 3;
// ... etc
```

### JIT Engine

```rust
pub struct JitEngine {
    context: Context,
    execution_engine: ExecutionEngine<'ctx>,
}

impl JitEngine {
    pub fn eval(&self, expr: &Value) -> Result<RuntimeValue, JitError>;
    pub fn eval_with_env(&self, expr: &Value, env: &mut Environment)
        -> Result<RuntimeValue, JitError>;
}
```

### Compilation Process

1. **Analysis**: Find free variables, identify closures
2. **Codegen**: Generate LLVM IR for expression
3. **Optimization**: Run LLVM optimization passes
4. **Execution**: JIT compile and execute

## AOT Compiler

The AOT compiler (`aot/compiler.rs`) generates standalone LLVM IR files.

### Compilation Phases

1. **Parse**: Source → AST
2. **Collect Labels**: Find top-level `label` definitions
3. **Pre-declare Functions**: Create function declarations
4. **Compile Bodies**: Generate IR for each function
5. **Compile Expressions**: Generate IR for remaining expressions
6. **Generate Main**: Create entry point

### Runtime IR

The AOT output includes a runtime (`aot/runtime_ir.rs`) with:
- Memory allocation (`rt_cons`, `rt_make_closure`)
- Type operations (`rt_car`, `rt_cdr`, `rt_atom`, `rt_eq`)
- Arithmetic (`rt_add`, `rt_sub`, `rt_mul`, `rt_div`)
- Comparison (`rt_num_eq`, `rt_lt`, `rt_gt`)
- I/O (`rt_println`, `print_value`)

### Output Structure

```llvm
; Type definition
%RuntimeValue = type { i8, i64 }

; Runtime functions (linked)
define %RuntimeValue @rt_cons(%RuntimeValue, %RuntimeValue) { ... }

; User-defined functions
define %RuntimeValue @__consair_labeled_foo_0(%RuntimeValue) { ... }

; Top-level expressions
define %RuntimeValue @__consair_expr_0() { ... }

; Entry point
define i32 @main() {
  call %RuntimeValue @__consair_expr_0()
  ; ... print final result
  ret i32 0
}
```

## Shared Codegen

Both JIT and AOT use `codegen.rs` for LLVM IR generation:

```rust
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    pub value_type: StructType<'ctx>,

    // Runtime function declarations
    pub rt_cons: FunctionValue<'ctx>,
    pub rt_car: FunctionValue<'ctx>,
    // ... etc
}
```

## Symbol Interning

Symbols are interned (`interner.rs`) for:
- Fast equality comparison (pointer comparison)
- Memory efficiency (single copy per unique symbol)
- Thread safety (global intern table with locking)

```rust
pub struct InternedSymbol {
    inner: &'static str,  // Interned string
}

impl InternedSymbol {
    pub fn new(s: &str) -> Self;      // Intern a string
    pub fn resolve(&self) -> String;  // Get string back
}
```

## Standard Library

Native functions (`stdlib.rs`) are Rust functions with signature:

```rust
pub type NativeFn = fn(&[Value], &mut Environment) -> Result<Value, String>;
```

They're registered in the environment at startup:

```rust
pub fn register_stdlib(env: &mut Environment) {
    env.define("print".to_string(), Value::NativeFn(print));
    env.define("+".to_string(), Value::NativeFn(add));
    // ... etc
}
```

## Memory Management

- **Interpreter**: Rust's ownership and `Arc` for shared data
- **JIT/AOT**: Manual allocation with `malloc`/`free` in runtime

Currently no garbage collection - cons cells are allocated but not freed during execution. For long-running programs, this is a known limitation.

## Thread Safety

`Value` is `Send + Sync`, allowing multi-threaded use:
- Immutable data wrapped in `Arc`
- Symbol interning uses synchronized global table
- Environment uses `RwLock` for mutation

## Extension Points

### Adding New Special Forms

1. Add pattern match in `interpreter.rs` `eval` function
2. Add corresponding case in `jit/engine.rs` `compile_value`
3. Add corresponding case in `aot/compiler.rs` `compile_cons`

### Adding New Native Functions

1. Implement function in `stdlib.rs`
2. Register in `register_stdlib`
3. For JIT/AOT: Add runtime implementation in `runtime_ir.rs`

### Adding New Types

1. Add variant to `Value` enum in `language.rs`
2. Add display formatting
3. Add evaluation handling in interpreter
4. Add compilation handling in JIT/AOT (if supported)
