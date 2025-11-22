# Consair - Minimal Lisp in Rust

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

## Building and Running

### Build the project
```bash
cargo build
```

### Run the REPL
```bash
cargo run
```

### Run tests
```bash
cargo test
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

The implementation is organized into modules:

- **lib.rs**: Core interpreter (types, parser, evaluator)
  - Type system (`Value`, `ConsCell`, `LambdaCell`)
  - Environment (variable bindings)
  - Parser (s-expression → AST)
  - Evaluator (AST → Value)

- **main.rs**: REPL interface

- **tests/**: Integration tests for all primitives and features

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
