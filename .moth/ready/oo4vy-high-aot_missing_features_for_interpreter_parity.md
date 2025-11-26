# AOT Missing Features for Interpreter Parity

## Summary

The AOT compiler (`cadr`) is missing several features that the interpreter supports. This blocks running benchmarks like binary-trees that use string output.

## Missing Features

### High Priority (blocks benchmarks)

#### String Literals
- **Status**: Not implemented
- **Error**: `String literals not yet supported in AOT`
- **Needed for**: Any program with string output
- **Implementation**: Need to store strings in data section, return pointer in RuntimeValue

#### Variadic println/print
- **Status**: Only single argument supported
- **Interpreter**: `(println "x=" x " y=" y)` works
- **AOT**: Only `(println x)` works
- **Implementation**: Loop over arguments, print each with space separator

### Medium Priority (extended features)

#### Macros (defmacro)
- **Status**: Not supported
- **Error**: Will fail on any `defmacro` form
- **Workaround**: Expand macros before AOT compilation
- **Implementation**: Could do macro expansion pass before codegen

#### File I/O (slurp, spit)
- **Status**: Not implemented
- **Implementation**: Add runtime functions using libc file operations

#### Shell command (shell)
- **Status**: Not implemented
- **Implementation**: Add runtime function using libc system/popen

#### Time (now)
- **Status**: Not implemented
- **Implementation**: Add runtime function using libc time

### Lower Priority (data structures)

#### Maps `{:key value}`
- **Status**: Not implemented
- **Implementation**: Complex - need hash table in runtime

#### Sets `#{elements}`
- **Status**: Not implemented
- **Implementation**: Complex - need hash set in runtime

#### Persistent collections
- **Status**: Not implemented
- **Implementation**: Very complex - need persistent data structure runtime

#### Collection abstractions (%seq, %first, %next, etc.)
- **Status**: Not implemented
- **Implementation**: Depends on maps/sets

### Not Planned

#### gensym
- Only useful for macros, which run at compile time

#### macroexpand-1, macroexpand
- Debugging tools for macro development

## Implementation Notes

### String Literals

Strings need to be stored in the LLVM data section:

```llvm
@str.0 = private unnamed_addr constant [13 x i8] c"Hello World!\00"
```

RuntimeValue needs a TAG_STRING and pointer to string data.

### Variadic Functions

Option 1: Generate unrolled code for each argument
Option 2: Build a list/vector of args, pass to runtime function

### File Structure

New runtime functions should go in `aot/runtime_ir.rs`:
- `rt_slurp(path: RuntimeValue) -> RuntimeValue`
- `rt_spit(path: RuntimeValue, content: RuntimeValue) -> RuntimeValue`
- `rt_shell(cmd: RuntimeValue) -> RuntimeValue`
- `rt_now() -> RuntimeValue`

## Test Cases

Once implemented, these should work:

```lisp
; String literals
(println "Hello, World!")

; Variadic print
(println "Result:" (+ 1 2) "done")

; File I/O
(spit "/tmp/test.txt" "data")
(println (slurp "/tmp/test.txt"))

; Shell
(shell "echo hello")

; Time
(println "Timestamp:" (now))
```

## Related

- binary-trees benchmark needs strings for output
- Most real-world programs need string support
