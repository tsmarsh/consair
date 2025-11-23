<p align="center">
  <img src="docs/images/logo.png" alt="Consair Logo" width="200"/>
</p>

# Consair - Minimal Lisp in Rust

[![CI](https://github.com/tsmarsh/consair/workflows/CI/badge.svg)](https://github.com/tsmarsh/consair/actions)
[![codecov](https://codecov.io/gh/tsmarsh/consair/branch/main/graph/badge.svg)](https://codecov.io/gh/tsmarsh/consair)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A minimal Lisp interpreter based on Paul Graham's exposition of McCarthy's 1960 paper, implemented in Rust using reference counting instead of traditional garbage collection.

## Design Philosophy

This implementation demonstrates how Rust's ownership system and `Rc` (reference counting) can provide automatic memory management for a Lisp interpreter without traditional garbage collection. The key insight is that immutable cons cells can be safely shared via `Rc`, and memory is freed automatically when references are dropped.

## Core Features

### Seven Primitive Operators

1. **quote** - Returns argument unevaluated
2. **atom** - Tests if value is an atom (not a list)
3. **eq** - Tests equality of two atoms
4. **car** - Returns first element of a list
5. **cdr** - Returns rest of a list
6. **cons** - Constructs a new list by prepending an element
7. **cond** - Conditional expression

### Two Special Forms

- **lambda** - Creates anonymous functions with closures
- **label** - Names functions (enables recursion)

## Memory Model

```rust
use std::rc::Rc;

enum Value {
    Atom(AtomType),           // immediate values (numbers, symbols, etc.)
    Cons(Rc<ConsCell>),       // shared ownership of cons cells
    Nil,                      // empty list
    Lambda(Rc<LambdaCell>),   // functions with captured environments
}
```

### Key Design Decisions

- ✅ **Rc-based sharing**: Cons cells and lambdas use `Rc` for safe structure sharing
- ✅ **Immutability**: All values are immutable after creation
- ✅ **Automatic memory management**: Reference counting handles deallocation
- ✅ **No unsafe code**: Passes the borrow checker without any unsafe blocks
- ❌ **Circular references leak**: This is acceptable for a minimal Lisp

## Installation

### Download Pre-built Binaries

Download the latest release for your platform from the [Releases](../../releases) page:

- **Linux x86_64**: `cons-linux-x86_64`
- **Linux ARM64**: `cons-linux-aarch64`
- **macOS Intel**: `cons-macos-x86_64`
- **macOS Apple Silicon**: `cons-macos-aarch64`
- **Windows**: `cons-windows-x86_64.exe`

Make the binary executable (Linux/macOS):
```bash
chmod +x cons-*
./cons-*
```

### Build from Source

#### Build the project
```bash
cargo build --release
```

The binary will be available at `target/release/cons`.

#### Run the REPL
```bash
cargo run --release
# or
./target/release/cons
```

#### Run a Lisp file
```bash
./target/release/cons examples/simple.lisp
# or with cargo
cargo run --release -- examples/simple.lisp
```

#### Run tests
```bash
cargo test
```

## Usage

### Interactive REPL

Start the REPL with no arguments:
```bash
cons
```

### Execute Lisp Files

Run a file containing Lisp expressions:
```bash
cons program.lisp
```

The interpreter will evaluate all expressions in the file and print the result of the last expression.

### Example Files

The `examples/` directory contains sample Lisp programs:

- `simple.lisp` - Basic operations (cons, car, cdr, lambda)
- `list-ops.lisp` - List manipulation examples
- `closures.lisp` - Closure demonstration
- `factorial.lisp` - Factorial using recursion

Try them:
```bash
cons examples/simple.lisp
cons examples/closures.lisp
```

### Command-Line Options

```bash
cons              # Start interactive REPL
cons <file.lisp>  # Run a Lisp file
cons --help       # Show help message
```

## Example Usage

### Basic Primitives

```lisp
> (quote (1 2 3))
(1 2 3)

> '(a b c)
(a b c)

> (atom 'x)
t

> (atom '(1 2))
nil

> (eq 'a 'a)
t

> (car '(1 2 3))
1

> (cdr '(1 2 3))
(2 3)

> (cons 1 '(2 3))
(1 2 3)
```

### Conditional Expressions

```lisp
> (cond ((eq 1 1) 'yes) (t 'no))
yes

> (cond ((eq 1 2) 'first) ((atom 'x) 'second) (t 'third))
second
```

### Lambda Functions

```lisp
> ((lambda (x) x) 42)
42

> ((lambda (x y) (cons x y)) 1 2)
(1 . 2)

> ((lambda (x) (cons x '(2 3))) 1)
(1 2 3)
```

### Named Functions with Label

```lisp
> (label identity (lambda (x) x))
<lambda>

> (identity 42)
42
```

### Closures

Lambdas capture their environment, enabling closures:

```lisp
> (label make-adder (lambda (x) (lambda (y) (cons x y))))
<lambda>

> (label add-5 (make-adder 5))
<lambda>

> (add-5 10)
(5 . 10)
```

### Recursive Functions

While this minimal Lisp doesn't have built-in arithmetic, you can define recursive functions:

```lisp
> (label append
    (lambda (x y)
      (cond ((atom x) y)
            (t (cons (car x) (append (cdr x) y))))))
<lambda>

> (append '(1 2) '(3 4))
(1 2 3 4)
```

## Implementation Details

### Structure Sharing

Multiple lists can share the same tail structure thanks to `Rc`:

```lisp
> (label tail '(3 4))
<lambda>

> (cons 1 tail)
(1 3 4)

> (cons 2 tail)
(2 3 4)
```

Both results share the same underlying cons cells for `(3 4)`.

### Memory Management

- **cons** allocates: Creates new `Rc<ConsCell>`
- **lambda** allocates: Creates new `Rc<LambdaCell>` with captured environment
- **Other primitives**: No allocation, only read/compare existing values
- **Deallocation**: Automatic when last `Rc` is dropped

### Limitations

1. **No arithmetic**: This is McCarthy's pure Lisp - use cons cells to represent numbers if needed
2. **Circular references leak**: Don't create circular data structures
3. **No tail call optimization**: Deep recursion will overflow the stack
4. **Single-threaded**: `Rc` is not thread-safe (use `Arc` for concurrent access)

## Architecture

The project is organized as a Cargo workspace with two main components:

```
consair/
├── Cargo.toml           # Workspace definition
├── consair-core/        # Core library
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs           # Module exports and public API
│   │   ├── language.rs      # Core type system and primitives
│   │   │   ├── Value, AtomType, ConsCell, LambdaCell
│   │   │   ├── Display implementation
│   │   │   └── Primitives: cons, car, cdr, eq, is_atom
│   │   ├── parser.rs        # Tokenizer and parser
│   │   │   ├── Token types
│   │   │   ├── tokenize() - string → tokens
│   │   │   └── parse() - tokens → AST
│   │   └── interpreter.rs   # Evaluator and environment
│   │       ├── Environment - variable bindings
│   │       └── eval() - evaluates expressions
│   └── tests/
│       └── integration_tests.rs
└── cons/                # Interpreter executable
    ├── Cargo.toml
    └── src/
        └── main.rs      # REPL and file executor
```

### Component Responsibilities

#### `consair-core` (Library)
The core Lisp interpreter library that can be embedded in other applications:
- **language.rs**: Defines the core Lisp data types and primitive operations
- **parser.rs**: Converts s-expressions into the AST representation
- **interpreter.rs**: Evaluates AST nodes in the context of an environment
- **lib.rs**: Re-exports public API for external use

#### `cons` (Executable)
The command-line interpreter for running Lisp programs:
- **main.rs**: Interactive REPL and file execution

This workspace structure makes it easy to:
- Use `consair-core` as a library in other Rust projects
- Add new executables (formatters, debuggers, etc.) alongside `cons`
- Maintain a clean separation between library and CLI concerns

## Testing

All seven primitives and both special forms are tested:

```bash
cargo test
```

Tests cover:
- Basic primitives (quote, atom, eq, car, cdr, cons, cond)
- Lambda functions
- Closures and environment capture
- Named functions with label
- Nested expressions
- List construction

## Success Criteria

This implementation achieves all design goals:

✅ All seven primitives work correctly
✅ Lambda and recursion via label are supported
✅ Structure sharing works (multiple references to same tail)
✅ Passes the borrow checker without unsafe code
✅ Automatic memory management (no manual free)
✅ Can implement complex functions in terms of primitives

## References

- [Paul Graham - The Roots of Lisp](http://www.paulgraham.com/rootsoflisp.html)
- [John McCarthy - Recursive Functions of Symbolic Expressions (1960)](http://www-formal.stanford.edu/jmc/recursive.pdf)
- [Rust Rc documentation](https://doc.rust-lang.org/std/rc/struct.Rc.html)

## License

This is an educational implementation demonstrating Rust's ownership system applied to Lisp interpretation.
