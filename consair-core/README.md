# consair-core

Core library for Consair - A minimal Lisp interpreter based on McCarthy's 1960 paper.

## Overview

`consair-core` provides the fundamental building blocks for implementing Lisp interpreters and evaluators in Rust. It implements the seven primitive operators and two special forms from McCarthy's original Lisp specification, using Rust's `Rc` for automatic memory management.

## Features

- **Core Data Types**: Atoms (symbols, numbers, booleans), cons cells, lambdas
- **Seven Primitives**: `quote`, `atom`, `eq`, `car`, `cdr`, `cons`, `cond`
- **Special Forms**: `lambda` (closures), `label` (recursion)
- **Parser**: S-expression tokenizer and parser
- **Evaluator**: Expression evaluator with lexical scoping
- **Environment**: Variable bindings with parent scope support

## Usage

Add this to your `Cargo.toml`:

```toml
[dependencies]
consair-core = { path = "../consair-core" }
# or when published to crates.io:
# consair-core = "0.1"
```

### Basic Example

```rust
use consair::{Environment, parse, eval};

fn main() {
    let mut env = Environment::new();

    // Parse and evaluate an expression
    let expr = parse("(cons 1 (cons 2 nil))").unwrap();
    let result = eval(expr, &mut env).unwrap();

    println!("{}", result); // Prints: (1 2)
}
```

### Creating a REPL

```rust
use consair::{Environment, parse, eval};
use std::io::{self, Write};

fn main() {
    let mut env = Environment::new();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match parse(&input) {
            Ok(expr) => match eval(expr, &mut env) {
                Ok(result) => println!("{}", result),
                Err(e) => eprintln!("Error: {}", e),
            },
            Err(e) => eprintln!("Parse error: {}", e),
        }
    }
}
```

### Defining Functions

```rust
use consair::{Environment, parse, eval};

let mut env = Environment::new();

// Define a function
let define = parse("(label square (lambda (x) (cons x x)))").unwrap();
eval(define, &mut env).unwrap();

// Use it
let call = parse("(square 5)").unwrap();
let result = eval(call, &mut env).unwrap();
println!("{}", result); // Prints: (5 . 5)
```

## API Overview

### Types

- `Value` - Main expression type (atoms, cons cells, lambdas)
- `AtomType` - Atomic values (symbols, numbers, booleans)
- `ConsCell` - Cons cell structure
- `LambdaCell` - Lambda function with captured environment
- `Environment` - Variable bindings

### Functions

- `parse(input: &str) -> Result<Value, String>` - Parse s-expression
- `eval(expr: Value, env: &mut Environment) -> Result<Value, String>` - Evaluate expression
- `cons(car: Value, cdr: Value) -> Value` - Create cons cell

### Primitives

All primitive operations are implemented internally:

- `quote` - Return expression unevaluated
- `atom` - Test if value is atomic
- `eq` - Test equality of atoms
- `car` - Get first element of cons cell
- `cdr` - Get rest of cons cell
- `cons` - Construct cons cell
- `cond` - Conditional evaluation
- `lambda` - Create function
- `label` - Bind name to value

## Memory Model

Uses Rust's `Rc` (reference counting) for automatic memory management:

```rust
pub enum Value {
    Atom(AtomType),           // Immediate values
    Cons(Rc<ConsCell>),       // Shared cons cells
    Nil,                      // Empty list
    Lambda(Rc<LambdaCell>),   // Functions with closures
}
```

Benefits:
- Automatic memory management (no GC needed)
- Structure sharing for efficiency
- Safe - no unsafe code required

Tradeoffs:
- Circular references will leak (acceptable for this use case)
- Not thread-safe (use `Arc` for concurrency)

## JIT Compilation (Optional)

When built with the `jit` feature, `consair-core` provides LLVM-based JIT compilation:

```toml
[dependencies]
consair-core = { path = "../consair-core", features = ["jit"] }
```

### Using the JIT Engine

```rust
use consair::{parse, Environment, JitEngine};

fn main() {
    let mut env = Environment::new();
    let jit = JitEngine::new();

    // Parse an expression
    let expr = parse("(+ (* 2 3) 4)").unwrap();

    // JIT compile and execute
    let result = jit.eval(&expr).unwrap();
    println!("Result: {:?}", result.to_value());  // 10
}
```

### JIT with Environment Bindings

```rust
use consair::{parse, eval, Environment, JitEngine};

let mut env = Environment::new();
let jit = JitEngine::new();

// Define functions in the interpreter
eval(parse("(label square (lambda (x) (* x x)))").unwrap(), &mut env).unwrap();

// Use JIT with environment (macro expansion + JIT)
let expr = parse("(square 5)").unwrap();
let result = jit.eval_with_env(&expr, &mut env).unwrap();
```

### JIT Caching

The JIT engine can cache results of pure expressions:

```rust
use consair::{JitEngine, CacheConfig};

let jit = JitEngine::with_config(CacheConfig {
    enabled: true,
    max_entries: 1000,
});

// First call compiles and caches
jit.eval(&expr).unwrap();

// Second call returns cached result (no recompilation)
jit.eval(&expr).unwrap();

// Check cache statistics
let stats = jit.cache_stats();
println!("Hits: {}, Misses: {}", stats.hits, stats.misses);
```

### JIT API

- `JitEngine::new()` - Create a new JIT engine
- `JitEngine::with_config(config)` - Create with custom cache settings
- `jit.eval(expr)` - JIT compile and evaluate an expression
- `jit.eval_with_env(expr, env)` - Evaluate with macro expansion
- `jit.cache_stats()` - Get cache hit/miss statistics
- `jit.clear_cache()` - Clear the result cache

### RuntimeValue

JIT-compiled code uses a C-ABI compatible value representation:

```rust
#[repr(C)]
pub struct RuntimeValue {
    pub tag: u8,    // Type discriminator
    pub data: u64,  // Payload (int, float bits, or pointer)
}
```

Convert between `RuntimeValue` and `Value`:
```rust
let runtime_val: RuntimeValue = (&value).into();
let value: Value = runtime_val.to_value()?;
```

## Examples

See the `cons` executable for a complete REPL implementation.

## License

MIT OR Apache-2.0
