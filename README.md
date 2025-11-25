<p align="center">
  <img src="docs/images/logo.png" alt="Consair Logo" width="200"/>
</p>

# Consair - Minimal Lisp in Rust

[![CI](https://github.com/tsmarsh/consair/workflows/CI/badge.svg)](https://github.com/tsmarsh/consair/actions)
[![codecov](https://codecov.io/gh/tsmarsh/consair/branch/main/graph/badge.svg)](https://codecov.io/gh/tsmarsh/consair)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

A minimal Lisp interpreter based on Paul Graham's exposition of McCarthy's 1960 paper, implemented in Rust.

**No garbage collector. No global interpreter lock. Just fast, predictable execution.**

Optional LLVM JIT compilation delivers **sub-4-nanosecond** arithmetic execution.

## Design Philosophy

Consair proves that a Lisp doesn't need a garbage collector or a GIL. By leveraging Rust's ownership system with `Arc` (atomic reference counting), memory is freed **instantly** when the last reference is dropped - no stop-the-world pauses, no GC tuning, no unpredictable latency spikes.

**Key principles:**
- **No GC**: Atomic reference counting provides deterministic memory management
- **No GIL**: True parallelism - multiple threads can execute Consair code simultaneously
- **Thread-safe**: All values implement `Send + Sync` by design
- **Structure sharing**: Immutable cons cells are safely shared via `Arc`

## Core Features

### Seven Primitive Operators

1. **quote** - Returns argument unevaluated
2. **atom** - Tests if value is an atom (not a list)
3. **eq** - Tests equality of two atoms
4. **car** - Returns first element of a list
5. **cdr** - Returns rest of a list
6. **cons** - Constructs a new list by prepending an element
7. **cond** - Conditional expression

### Special Forms

- **lambda** - Creates anonymous functions with closures
- **label** - Names functions (enables recursion)
- **defmacro** - Defines macros for code transformation
- **quasiquote** - Template construction with `` ` `` syntax
- **unquote** - Evaluate expressions in templates with `,` syntax
- **unquote-splicing** - Splice lists in templates with `,@` syntax

### Macro System

Consair supports **unhygienic macros** in the Common Lisp style, enabling powerful meta-programming:

```lisp
; Define a when macro
(defmacro when (condition body)
  `(cond (,condition ,body) (t nil)))

; Use the macro
(when t (println "This runs!"))  ; => "This runs!"
(when nil (println "This doesn't"))  ; => nil

; Macros expand before evaluation
(macroexpand '(when t 42))  ; => (cond (t 42) (t nil))
```

**Macro Features:**
- **`defmacro`** - Define macros that receive unevaluated arguments
- **`` ` `` (quasiquote)** - Construct code templates
- **`,` (unquote)** - Insert evaluated expressions into templates
- **`,@` (unquote-splicing)** - Splice lists into templates
- **`macroexpand`** / **`macroexpand-1`** - Debug macro expansion
- **`gensym`** - Generate unique symbols for hygiene

### Standard Library

**I/O Functions:**
- **print** / **println** - Output to stdout
- **slurp** / **spit** - Read/write files (Clojure-style)

**System Functions:**
- **shell** - Execute shell commands, returns `((:out . "...") (:err . "...") (:exit . 0) (:success . t))`
- **now** - Get current Unix timestamp

**Macro Utilities:**
- **gensym** - Generate unique symbols for macro hygiene
- **macroexpand** - Fully expand macros in an expression
- **macroexpand-1** - Expand macros one level

## Memory Model

```rust
use std::sync::Arc;

enum Value {
    Atom(AtomType),           // immediate values (numbers, symbols, etc.)
    Cons(Arc<ConsCell>),      // thread-safe shared ownership of cons cells
    Nil,                      // empty list
    Lambda(Arc<LambdaCell>),  // functions with captured environments
    Macro(Arc<MacroCell>),    // macros for code transformation
}
```

### Key Design Decisions

- ✅ **Arc-based sharing**: Cons cells and lambdas use `Arc` for thread-safe structure sharing
- ✅ **Thread-safe**: Value implements Send + Sync, can be safely shared across threads
- ✅ **Immutability**: All values are immutable after creation
- ✅ **Automatic memory management**: Atomic reference counting handles deallocation
- ✅ **Minimal unsafe code**: Only two unsafe impl blocks for Send + Sync (verified safe)
- ❌ **Circular references leak**: This is acceptable for a minimal Lisp

## Installation

### Package Managers

#### Homebrew (macOS/Linux)

```bash
brew tap tsmarsh/consair
brew install consair
```

#### APT (Debian/Ubuntu)

```bash
# Add the repository
echo "deb [trusted=yes] https://tsmarsh.github.io/apt-consair stable main" | sudo tee /etc/apt/sources.list.d/consair.list

# Update package list
sudo apt update

# Install consair
sudo apt install consair
```

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

#### Build with JIT Compilation (Experimental)

Consair supports optional JIT compilation via LLVM for significantly faster execution of compute-intensive code.

**Prerequisites:**
- LLVM 17 must be installed on your system
- The `LLVM_SYS_170_PREFIX` environment variable should point to your LLVM installation

**Linux (Ubuntu/Debian):**
```bash
# Install LLVM 17
wget https://apt.llvm.org/llvm.sh
chmod +x llvm.sh
sudo ./llvm.sh 17

# Set environment variable
export LLVM_SYS_170_PREFIX=/usr/lib/llvm-17

# Build with JIT support
cargo build --release --features jit
```

**macOS:**
```bash
# Install LLVM 17
brew install llvm@17

# Set environment variable
export LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)

# Build with JIT support
cargo build --release --features jit
```

**Arch Linux:**
```bash
# Install LLVM 17
sudo pacman -S llvm17

# Set environment variable
export LLVM_SYS_170_PREFIX=/usr

# Build with JIT support
cargo build --release --features jit
```

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
- `stdlib.lisp` - Standard library functions (print, file I/O, shell, time)

Try them:
```bash
cons examples/simple.lisp
cons examples/closures.lisp
cons examples/stdlib.lisp
```

### Command-Line Options

```bash
cons              # Start interactive REPL
cons <file.lisp>  # Run a Lisp file
cons --help       # Show help message
cons --jit        # Start REPL with JIT compilation enabled (requires jit feature)
```

### JIT Compilation Mode

When built with the `jit` feature, Consair compiles expressions to native machine code via LLVM 17, delivering **blazing fast execution**.

#### Performance

Pre-compiled expressions execute at near-native speeds:

| Operation | Execution Time |
|-----------|---------------|
| Simple arithmetic `(+ 1 2 3 4 5)` | **3.6 ns** |
| Nested arithmetic | **3.5 ns** |
| Comparisons | **3.8 ns** |
| cons/car/cdr | **68 ns** |
| Vector operations | **83 ns** |
| Conditional expressions | **3.6 ns** |

*These are pure execution times after compilation. Initial compilation takes ~600μs-1.5ms.*

#### Usage

**Starting in JIT mode:**
```bash
# Build with JIT support
cargo build --release --features jit

# Start REPL with JIT enabled
./target/release/cons --jit

# Run a file with JIT
./target/release/cons --jit program.lisp
```

**Toggling JIT in the REPL:**
```
consair> :jit
JIT compilation enabled
consair[jit]> (+ 1 2)
3
consair[jit]> :jit
JIT compilation disabled
consair>
```

#### JIT Features

- **LLVM Backend**: Compiles to optimized native code via LLVM 17
- **Closures**: Full closure support with captured variables
- **Tail Call Optimization**: Recursive functions marked for TCO
- **Macro Expansion**: Macros expanded before compilation
- **Result Caching**: Pure expressions cache their results
- **Graceful Fallback**: Unsupported expressions fall back to interpreter

**What gets JIT compiled:**
- Arithmetic: `+`, `-`, `*`, `/`
- Comparisons: `<`, `>`, `<=`, `>=`, `=`, `eq`
- List operations: `cons`, `car`, `cdr`, `length`, `append`, `reverse`, `nth`
- Control flow: `cond`, `lambda`, `label`
- Vectors: `vector`, `vector-ref`, `vector-length`
- Type predicates: `atom`, `nil?`, `number?`, `cons?`, `not`
- Macros (expanded before compilation)

**What falls back to interpreter:**
- I/O operations: `print`, `println`, `slurp`, `spit`
- System calls: `shell`
- String operations (coming soon)
- Definitions at REPL (values are JIT-compiled, bindings handled by interpreter)

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

### Standard Library Functions

```lisp
> (println "Hello, World!")
Hello, World!
nil

> (spit "/tmp/test.txt" "Hello from Consair!")
nil

> (slurp "/tmp/test.txt")
"Hello from Consair!"

> (now)
1763867177

> (shell "echo hello")
((:out . "hello\n") (:err . "") (:exit . 0) (:success . t))
```

## Implementation Details

### Structure Sharing

Multiple lists can share the same tail structure thanks to `Arc`:

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

- **cons** allocates: Creates new `Arc<ConsCell>` (thread-safe)
- **lambda** allocates: Creates new `Arc<LambdaCell>` with captured environment
- **Other primitives**: No allocation, only read/compare existing values
- **Deallocation**: Automatic when last `Arc` is dropped (atomic reference counting)

### String Interning

Consair uses **string interning** for all symbols and keywords to reduce memory usage and improve performance:

- **Shared storage**: Identical symbols (e.g., multiple occurrences of `foo`) share the same underlying string storage
- **Fast comparisons**: Symbol equality uses pointer comparison (O(1)) instead of string comparison
- **Memory efficient**: Only one copy of each unique symbol is stored in memory
- **Thread-safe**: Global `RwLock`-based interner allows safe sharing across threads
- **Zero-copy symbols**: `InternedSymbol` implements `Copy`, eliminating cloning overhead

Example benefit: A program with 1000 occurrences of the symbol `lambda` only stores the string "lambda" once in memory.

### Tail Call Optimization

Consair implements **full tail call optimization (TCO)**, enabling unbounded recursion for tail-recursive functions:

```lisp
; This countdown function uses tail recursion
; Without TCO, this would overflow the stack
> (label countdown (lambda (n)
    (cond
      ((= n 0) "done")
      (t (countdown (- n 1))))))
<lambda>

; Can handle arbitrarily deep recursion
> (countdown 50000)
"done"
```

**How TCO Works:**
- The interpreter transforms tail-recursive calls into iteration using a loop
- Tail positions (final expressions in `cond` branches, lambda body returns) reuse the same stack frame
- Non-tail recursive calls still use the call stack and are limited to a depth of 10,000

**Tail vs Non-Tail Recursion:**
```lisp
; TAIL RECURSIVE (unbounded):
(label sum-tail (lambda (n acc)
  (cond
    ((= n 0) acc)
    (t (sum-tail (- n 1) (+ acc n))))))  ; Last operation is the recursive call

; NON-TAIL RECURSIVE (limited to depth 10,000):
(label factorial (lambda (n)
  (cond
    ((= n 0) 1)
    (t (* n (factorial (- n 1)))))))  ; Recursive call followed by multiplication
```

### Limitations

1. **Circular references leak**: Don't create circular data structures
2. **Non-tail recursion depth**: Limited to 10,000 levels (tail calls have no limit)

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
│   │   ├── interpreter.rs   # Evaluator and environment
│   │   │   ├── Environment - variable bindings
│   │   │   └── eval() - evaluates expressions
│   │   ├── native.rs        # Native function utilities
│   │   │   ├── Value extraction helpers
│   │   │   ├── Arity checking
│   │   │   └── Value construction
│   │   └── stdlib.rs        # Standard library
│   │       ├── I/O: print, println
│   │       ├── Files: slurp, spit
│   │       ├── System: shell, now
│   │       └── register_stdlib()
│   └── tests/
│       ├── integration_tests.rs
│       ├── native_tests.rs
│       └── stdlib_tests.rs
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
- **native.rs**: Utilities for implementing native Rust functions callable from Lisp
- **stdlib.rs**: Standard library (I/O, file operations, shell, time)
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
✅ Thread-safe (Value implements Send + Sync)
✅ Minimal unsafe code (only verified Send + Sync impls)
✅ Automatic memory management (no manual free)
✅ Can implement complex functions in terms of primitives
✅ Tail call optimization enables unbounded recursion
✅ String interning for efficient symbol storage and comparison
✅ Optional LLVM JIT compilation with nanosecond execution times

## References

- [Paul Graham - The Roots of Lisp](http://www.paulgraham.com/rootsoflisp.html)
- [John McCarthy - Recursive Functions of Symbolic Expressions (1960)](http://www-formal.stanford.edu/jmc/recursive.pdf)
- [Rust Arc documentation](https://doc.rust-lang.org/std/sync/struct.Arc.html)

## License

This is an educational implementation demonstrating Rust's ownership system applied to Lisp interpretation.
