# Consair Examples

This directory contains example Lisp programs demonstrating various features of the Consair interpreter.

## Running Examples

From the project root:

```bash
cargo build --release
./target/release/cons examples/<filename>.lisp
```

Or directly with cargo:

```bash
cargo run --release -- examples/<filename>.lisp
```

## Available Examples

### `simple.lisp`
Basic operations demonstrating:
- `cons`, `car`, `cdr` for list manipulation
- `label` for defining functions
- `lambda` for anonymous functions
- Function application

```bash
cons examples/simple.lisp
```

### `list-ops.lisp`
List operations showing:
- List construction
- List deconstruction
- Atom testing
- Equality checking

```bash
cons examples/list-ops.lisp
```

### `closures.lisp`
Closure demonstration:
- Functions that return functions
- Lexical scoping
- Environment capture

```bash
cons examples/closures.lisp
```

### `factorial.lisp`
Recursive function example (note: uses cons cells to represent numbers)

```bash
cons examples/factorial.lisp
```

### `jit-demo.lisp`
JIT compilation demonstration (requires `jit` feature):
- Arithmetic operations compiled to native code
- Recursive functions (factorial, fibonacci)
- Closures and higher-order functions
- Vector operations
- Macro expansion before JIT compilation

```bash
# Build with JIT support first
cargo build --release --features jit

# Run the demo
./target/release/cons --jit examples/jit-demo.lisp
```

## Creating Your Own Programs

Create a `.lisp` file with Lisp expressions, one per line or spanning multiple lines:

```lisp
(label square (lambda (x) (cons x x)))
(square 5)
```

Run it:

```bash
cons myprogram.lisp
```

The interpreter will:
1. Read all expressions from the file
2. Evaluate them in order
3. Print the result of the last expression

## Notes

- All expressions in a file share the same environment
- Functions defined with `label` are available to subsequent expressions
- The last expression's result is printed to stdout
- Errors are printed to stderr

## Writing Lisp Programs

### Multiple Expressions

```lisp
(label identity (lambda (x) x))
(label pair (lambda (a b) (cons a b)))
(pair (identity 1) (identity 2))
```

### Using Closures

```lisp
(label make-multiplier (lambda (x) (lambda (y) (cons x y))))
(label times-3 (make-multiplier 3))
(times-3 7)
```

### Conditional Logic

```lisp
(label is-nil (lambda (x) (cond ((atom x) t) (t nil))))
(is-nil 'a)
(is-nil '(1 2))
```

## Limitations

- Each expression must be complete
- Recursive functions may have limitations depending on the implementation
