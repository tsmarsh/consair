---
layout: default
title: Consair - Minimal Lisp in Rust
---

# Consair

[![CI](https://github.com/tsmarsh/consair/workflows/CI/badge.svg)](https://github.com/tsmarsh/consair/actions)
[![Coverage](https://codecov.io/gh/tsmarsh/consair/branch/main/graph/badge.svg)](https://codecov.io/gh/tsmarsh/consair)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)

## Minimal Lisp in Rust

**No garbage collector. No global interpreter lock.**

A pure implementation of McCarthy's 1960 Lisp with LLVM JIT compilation.

Sub-4-nanosecond execution | Deterministic memory | True parallelism

[Download Latest Release](https://github.com/tsmarsh/consair/releases/latest) | [View on GitHub](https://github.com/tsmarsh/consair)

---

## Key Features

### LLVM JIT Compilation
Optional native code compilation via LLVM 17. Pre-compiled expressions execute in **3-4 nanoseconds**. Full closure and recursion support.

### McCarthy's Seven Primitives
Pure implementation of `quote`, `atom`, `eq`, `car`, `cdr`, `cons`, and `cond`

### Comprehensive Numeric Tower
Integers, floats, ratios, and arbitrary-precision big integers with automatic promotion

### Rich String System
Basic strings, raw strings, multiline strings with escape sequences

### Vectors
Efficient indexed collections with `<< >>` syntax

### No Garbage Collector
Atomic reference counting frees memory **instantly** when the last reference drops. No GC pauses, no tuning, deterministic latency.

### No Global Interpreter Lock
True parallelism - multiple threads can execute Consair code simultaneously. All values implement `Send + Sync`.

### Arc-Based Memory
Zero-copy structure sharing with atomic reference counting. Immutable cons cells safely shared across threads.

### Closures
First-class functions with lexical scoping and environment capture

### Recursion
Named recursive functions via `label`

### Standard Library
I/O (print, println), files (slurp, spit), shell commands, and time functions

---

## Quick Examples

### Classic Lisp Primitives

```lisp
> (car '(1 2 3))
1

> (cdr '(1 2 3))
(2 3)

> (cons 1 '(2 3))
(1 2 3)
```

### Numeric Tower

```lisp
> (+ 1/3 1/3)
2/3

> (+ 1/3 1.5)
1.8333333333333335

> (+ 999999999999999999999 1)
1000000000000000000000
```

### Lambda and Closures

```lisp
> ((lambda (x y) (+ x y)) 10 20)
30

> (label make-counter
    (lambda (n)
      (lambda () (+ n 1))))

> (label inc (make-counter 5))
> (inc)
6
```

### Vectors

```lisp
> <<1 2 3 4 5>>
<<1 2 3 4 5>>

> (vector-ref <<10 20 30>> 1)
20

> (vector-length <<a b c>>)
3
```

### Standard Library

```lisp
> (println "Hello, World!")
Hello, World!
nil

> (spit "/tmp/test.txt" "data")
nil

> (slurp "/tmp/test.txt")
"data"

> (now)
1763867177

> (shell "echo hello")
((:out . "hello\n") (:err . "") (:exit . 0) (:success . t))
```

### JIT Compilation Mode

```lisp
> :jit
JIT compilation enabled

consair[jit]> (label factorial
    (lambda (n)
      (cond ((= n 0) 1)
            (t (* n (factorial (- n 1)))))))

consair[jit]> (factorial 10)
3628800  ; Compiled to native code!

; Toggle JIT off
consair[jit]> :jit
JIT compilation disabled
```

---

## Getting Started

### Homebrew (macOS/Linux)

```bash
brew tap tsmarsh/consair
brew install consair
```

### APT (Debian/Ubuntu)

```bash
# Add the repository
echo "deb [trusted=yes] https://tsmarsh.github.io/apt-consair stable main" | sudo tee /etc/apt/sources.list.d/consair.list

# Update and install
sudo apt update
sudo apt install consair
```

### Download Pre-built Binary

Get the latest release for your platform from the [releases page](https://github.com/tsmarsh/consair/releases):

```bash
chmod +x cons-*
./cons-*
```

### Build from Source

```bash
git clone https://github.com/tsmarsh/consair.git
cd consair
cargo build --release
./target/release/cons
```

### Build with JIT Support

```bash
# macOS
brew install llvm@17
export LLVM_SYS_170_PREFIX=$(brew --prefix llvm@17)

# Linux (Ubuntu/Debian)
wget https://apt.llvm.org/llvm.sh && chmod +x llvm.sh
sudo ./llvm.sh 17 && sudo apt install libpolly-17-dev
export LLVM_SYS_170_PREFIX=/usr/lib/llvm-17

# Build with JIT
cargo build --release --features jit
./target/release/cons --jit
```

### Run the REPL

```bash
cons                    # Interactive REPL
cons program.lisp       # Run a file
cons --help            # Show help
```

### Use as a Library

```rust
use consair::{parse, eval, Environment};

let mut env = Environment::new();
let result = eval(parse("(+ 1 2)").unwrap(), &mut env);
println!("{}", result.unwrap());  // Prints: 3
```

---

## Documentation

- [Language Overview](language/README.md) - Introduction to Consair Lisp
- [Data Types](language/types.md) - Numbers, strings, symbols, lists, vectors, maps, sets
- [Special Forms](language/special-forms.md) - `quote`, `if`, `cond`, `lambda`, `label`, `defmacro`
- [Standard Library](language/stdlib.md) - Built-in functions
- [cons CLI](tools/cons.md) - Interactive REPL and interpreter
- [cadr Compiler](tools/cadr.md) - Ahead-of-time compiler to LLVM IR
- [Architecture](internals/architecture.md) - Interpreter, JIT, and AOT design
- [Examples](examples/README.md) - Sample code and tutorials

---

## Links

- [GitHub Repository](https://github.com/tsmarsh/consair)
- [The Roots of Lisp](http://www.paulgraham.com/rootsoflisp.html) by Paul Graham
- [McCarthy's Original Paper](http://www-formal.stanford.edu/jmc/recursive.pdf) (1960)

---

Built with Rust | Based on McCarthy's 1960 paper "Recursive Functions of Symbolic Expressions"
