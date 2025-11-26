# Examine AOT Standalone Label Handling

## Summary

The AOT compiler currently needs special handling for standalone `label` definitions that differs from the interpreter and JIT. This should be examined to determine if there's a cleaner unified approach.

## Current Behavior

Two patterns for `label`:

1. **Inline/immediate call** (original McCarthy pattern):
   ```lisp
   ((label factorial (lambda (n) ...)) 5)
   ```
   Function is created and immediately called.

2. **Standalone definition** (extension):
   ```lisp
   (label factorial (lambda (n) ...))
   (factorial 5)  ; called later
   ```
   Function is defined at top-level, callable from anywhere.

## Why AOT Needs Special Handling

AOT compiles each top-level expression to a separate LLVM function. Without special handling:
- Each expression gets its own empty `compiled_fns` map
- `(label factorial ...)` creates a function in expression 1
- `(factorial 5)` in expression 2 can't see it

Current fix does a multi-pass approach:
1. First pass: scan for top-level `(label name (lambda ...))` patterns
2. Pre-declare all labeled functions in LLVM
3. Compile label bodies
4. Compile all expressions with shared function table

## Questions to Examine

1. Should standalone `label` be a first-class feature or should we introduce `define`/`defun`?
2. Can the interpreter, JIT, and AOT share more of this logic?
3. Is the multi-pass approach the right design, or should we restructure compilation?
4. Does this match McCarthy's original semantics for `label`?

## Files Involved

- `consair-core/src/aot/compiler.rs` - AOT label handling
- `consair-core/src/jit/engine.rs` - JIT label handling
- `consair-core/src/eval.rs` - Interpreter label handling

## Test Case

```lisp
(label nil? (lambda (x) (eq x nil)))
(label make-tree (lambda (depth)
  (cond ((= depth 0) (cons nil nil))
        (t (cons (make-tree (- depth 1))
                 (make-tree (- depth 1)))))))
(make-tree 3)
```
