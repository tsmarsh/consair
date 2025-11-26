# Consair Language Overview

Consair is a Lisp implementation inspired by John McCarthy's original 1960 paper "Recursive Functions of Symbolic Expressions and Their Computation by Machine". It extends the classic seven primitives with modern features while maintaining the elegant simplicity of the original design.

## Quick Start

```lisp
; Define a function using label
(label factorial (lambda (n)
  (cond ((= n 0) 1)
        (t (* n (factorial (- n 1)))))))

; Call it
(factorial 5)  ; => 120

; Lists
(cons 1 (cons 2 (cons 3 nil)))  ; => (1 2 3)
(car '(1 2 3))                   ; => 1
(cdr '(1 2 3))                   ; => (2 3)

; Vectors (fast random access)
<<1 2 3 4 5>>
(vector-ref <<10 20 30>> 1)      ; => 20

; Strings
"Hello, World!\n"

; Arithmetic
(+ 1 2 3)      ; => 6
(/ 1 2)        ; => 1/2 (exact rational)
(* 1.5 2)      ; => 3.0 (floating point)
```

## Core Primitives

Consair implements McCarthy's seven primitives:

| Primitive | Description |
|-----------|-------------|
| `cons` | Construct a pair (cons cell) |
| `car` | Get first element of a pair |
| `cdr` | Get second element of a pair |
| `atom` | Test if value is an atom (not a cons cell) |
| `eq` | Test equality of atoms |
| `cond` | Conditional expression |
| `lambda` | Anonymous function |
| `label` | Named (potentially recursive) function |

## Extended Features

Beyond McCarthy's original design, Consair adds:

- **Multiple numeric types**: Integers, rationals, floats, arbitrary precision
- **Strings**: With escape sequences (`\n`, `\t`, `\\`, `\"`)
- **Vectors**: Fast random-access arrays with `<<...>>` syntax
- **Maps and Sets**: Hash-based collections with `{...}` and `#{...}` syntax
- **Persistent collections**: Immutable with structural sharing
- **Macros**: Compile-time code transformation with `defmacro`
- **I/O**: File operations, shell commands, printing
- **JIT compilation**: LLVM-based just-in-time compilation
- **AOT compilation**: Ahead-of-time compilation to native code

## Execution Modes

Consair supports three execution modes:

1. **Interpreter**: Direct evaluation, full feature support
2. **JIT**: LLVM-based compilation, faster execution
3. **AOT**: Compile to native executables via LLVM IR

```bash
# Interpreted (default)
cons program.lisp

# With JIT
cons --jit program.lisp

# AOT compilation
cadr program.lisp -o program.ll
clang -O3 program.ll -o program
./program
```

## Truthiness

In Consair:
- `nil` is false
- `t` is the canonical true value
- Everything else is truthy (including `0`, empty strings, etc.)

```lisp
(cond (nil "never")
      (t "always"))  ; => "always"

(if 0 "truthy" "falsy")  ; => "truthy"
```

## Comments

Consair uses semicolon comments:

```lisp
; This is a comment
(+ 1 2)  ; inline comment
```

## See Also

- [Data Types](types.md) - Complete type reference
- [Special Forms](special-forms.md) - Language constructs
- [Standard Library](stdlib.md) - Built-in functions
