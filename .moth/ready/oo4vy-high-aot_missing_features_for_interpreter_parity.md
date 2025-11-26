# AOT Missing Features for Interpreter Parity

## Summary

The AOT compiler (`cadr`) is missing several features that the interpreter supports.

**Update**: String literals and variadic print are now implemented! The binary-trees benchmark now works with AOT.

## Completed Features

### String Literals ✓
- **Status**: Implemented
- Stores strings in LLVM data section as global constants
- Creates RuntimeString via `rt_make_string` at runtime
- Works with `println`, `print`, and all string operations

### Variadic println/print ✓
- **Status**: Implemented
- `(println "x=" x " y=" y)` now works in AOT
- Arguments separated by spaces, newline at end (for println)

## Missing Features

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

### File I/O Implementation

New runtime functions should go in `aot/runtime_ir.rs`:
- `rt_slurp(path: RuntimeValue) -> RuntimeValue`
- `rt_spit(path: RuntimeValue, content: RuntimeValue) -> RuntimeValue`
- `rt_shell(cmd: RuntimeValue) -> RuntimeValue`
- `rt_now() -> RuntimeValue` (stub exists, needs real implementation)

## Test Cases

These features still need to work:

```lisp
; File I/O
(spit "/tmp/test.txt" "data")
(println (slurp "/tmp/test.txt"))

; Shell
(shell "echo hello")

; Time (real timestamp, not 0)
(println "Timestamp:" (now))
```

## Benchmark Results

With string literals and variadic print implemented:

| Mode | Time (binary-trees n=10) |
|------|--------------------------|
| Interpreter | 0.38s |
| JIT | 0.38s |
| **AOT** | **0.03s** (10x faster!) |
