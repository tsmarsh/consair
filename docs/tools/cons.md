# cons - Consair Interpreter & REPL

`cons` is the main Consair executable. It provides an interactive REPL and can run Lisp files.

## Usage

```bash
cons                    # Start interactive REPL
cons <file.lisp>        # Run a Lisp file
cons --jit              # Start REPL with JIT compilation
cons --jit <file.lisp>  # Run file with JIT compilation
cons --help             # Show help
```

## Interactive REPL

Start the REPL by running `cons` with no arguments:

```
$ cons
Consair Lisp REPL v0.2.0
JIT compilation available (mode: disabled)
Type :help for help, :quit to exit

consair> (+ 1 2 3)
6
consair> (label square (lambda (x) (* x x)))
<lambda>
consair> (square 5)
25
```

### REPL Commands

| Command | Description |
|---------|-------------|
| `:help`, `:h` | Show help message |
| `:quit`, `:q` | Exit the REPL |
| `:env` | Show environment info |
| `:jit` | Toggle JIT compilation mode |
| `(exit)` | Exit the REPL |

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl-C` | Clear current input |
| `Ctrl-D` | Exit REPL |
| `Up/Down` | Navigate command history |
| `Ctrl-R` | Reverse history search |

### Multi-line Input

The REPL automatically detects incomplete expressions:

```
consair> (label factorial
......>   (lambda (n)
......>     (cond ((= n 0) 1)
......>           (t (* n (factorial (- n 1)))))))
<lambda>
```

## Running Files

Execute a Lisp file:

```bash
cons program.lisp
```

The file is parsed and executed. The result of the last expression is printed.

### Example File

```lisp
; factorial.lisp
(label factorial (lambda (n)
  (cond ((= n 0) 1)
        (t (* n (factorial (- n 1)))))))

(println "5! =" (factorial 5))
(println "10! =" (factorial 10))
(factorial 20)
```

```bash
$ cons factorial.lisp
5! = 120
10! = 3628800
2432902008176640000
```

## JIT Compilation

Enable JIT compilation for faster execution:

```bash
# REPL with JIT
cons --jit

# File with JIT
cons --jit program.lisp
```

When JIT is enabled:
- The prompt changes to `consair[jit]>`
- Code is compiled to native machine code via LLVM
- Fallback to interpreter for unsupported features
- Toggle with `:jit` command in REPL

### JIT Limitations

Some features fall back to the interpreter:
- Macros (definition and expansion)
- Some complex closures
- Certain collection operations

The JIT automatically falls back when needed, so all valid programs work.

## Configuration

### History File

Command history is saved to `~/.consair_history`.

### Environment

The standard library is automatically loaded, providing:
- Core functions: `cons`, `car`, `cdr`, `atom`, `eq`
- Arithmetic: `+`, `-`, `*`, `/`, `=`, `<`, `>`, etc.
- I/O: `print`, `println`, `slurp`, `spit`, `shell`
- And more (see [Standard Library](../language/stdlib.md))

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (parse error, evaluation error, file not found) |

## Examples

### Simple Calculations

```bash
$ echo "(* 6 7)" | cons
Consair Lisp REPL v0.2.0
...
42
```

### File with Multiple Expressions

```lisp
; test.lisp
(println "Starting...")
(label double (lambda (x) (* x 2)))
(println "Double of 21:" (double 21))
"done"
```

```bash
$ cons test.lisp
Starting...
Double of 21: 42
done
```

## See Also

- [cadr](cadr.md) - AOT compiler
- [Language Overview](../language/README.md)
- [Standard Library](../language/stdlib.md)
