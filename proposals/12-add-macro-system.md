# Add Macro System

## Problem

The language currently has no macro system, limiting meta-programming capabilities. Users cannot define their own special forms or syntactic abstractions.

## Impact

- Limited expressiveness
- Cannot create DSLs
- Repetitive code patterns
- Cannot extend syntax

## Prompt for Implementation

```
Add a macro system with defmacro, quote, unquote, and splice support:

1. Currently no macros - cannot extend syntax
2. Need compile-time code transformation

Please implement a macro system with:

**Core Macro Features:**

1. **defmacro special form:**
   ```lisp
   (defmacro when (condition . body)
     `(cond (,condition ,@body) (t nil)))

   (when (> x 5)
     (println "x is large")
     (println "very large"))

   ; Expands to:
   ; (cond ((> x 5) (println "x is large") (println "very large")) (t nil))
   ```

2. **Quasiquote (`) and unquote (,):**
   ```lisp
   (define x 42)
   `(a b ,x d)        ; => (a b 42 d)
   `(a b ,(+ 1 2) d)  ; => (a b 3 d)
   ```

3. **Unquote-splicing (,@):**
   ```lisp
   (define items '(1 2 3))
   `(a ,@items b)  ; => (a 1 2 3 b)
   ```

4. **Macro expansion:**
   ```lisp
   (macroexpand '(when (> x 5) (println x)))
   ; => (cond ((> x 5) (println x)) (t nil))
   ```

**Implementation Steps:**

1. **Add quasiquote reader macro:**
   - Modify parser to recognize ` (backtick)
   - Parse into Quasiquote AST node
   - Handle nested quasiquotes

2. **Add unquote and splice:**
   - Parse `,expr` as Unquote
   - Parse `,@expr` as UnquoteSplice
   - Track quasiquote nesting depth

3. **Implement macro expansion:**
   ```rust
   // In interpreter.rs
   fn expand_macro(macro_fn: Value, args: &[Value]) -> Result<Value, String> {
       // Call macro with unevaluated args
       // Return expanded code
   }

   fn expand_all(expr: Value, env: &Environment) -> Result<Value, String> {
       // Recursively expand all macros
       // Until no more macros found
   }
   ```

4. **Add defmacro to interpreter:**
   ```rust
   // Like lambda but mark as macro
   enum FunctionType {
       Function(Lambda),
       Macro(Lambda),
   }

   // In eval():
   "defmacro" => {
       let (name, params, body) = parse_defmacro(args)?;
       let macro_fn = Value::Macro(Lambda { params, body, env });
       env.define(name, macro_fn);
   }
   ```

5. **Evaluation order:**
   - Macro expansion happens before evaluation
   - Macros receive unevaluated arguments
   - Macro result is evaluated in original context

**Hygiene Considerations:**

For now, implement simple (unhygienic) macros like Common Lisp.
Document that variable capture is possible:

```lisp
(defmacro bad-macro (x)
  `(let ((tmp 1))  ; 'tmp' might capture caller's variable!
     (+ ,x tmp)))
```

Future enhancement: Add gensym for hygiene:
```lisp
(defmacro good-macro (x)
  (let ((tmp-sym (gensym)))
    `(let ((,tmp-sym 1))
       (+ ,x ,tmp-sym))))
```

**Testing:**

- Add tests for:
  * Basic quasiquote/unquote
  * Nested quasiquotes
  * Unquote-splicing
  * Simple macros (when, unless, and, or)
  * Recursive macros
  * Macro expansion order
  * Error cases (unquote outside quasiquote, etc.)

**Examples to support:**

```lisp
; unless macro
(defmacro unless (condition . body)
  `(cond (,condition nil) (t ,@body)))

; and macro
(defmacro and args
  (cond ((null args) t)
        ((null (cdr args)) (car args))
        (t `(cond (,(car args) (and ,@(cdr args)))
                  (t nil)))))

; let macro (if not built-in)
(defmacro let (bindings . body)
  `((lambda ,(map car bindings) ,@body)
    ,@(map cadr bindings)))
```

**Documentation:**

- Add macro tutorial to docs/
- Explain macro expansion phase
- Show common macro patterns
- Warn about hygiene issues
- Provide debugging tips (macroexpand)

## Success Criteria

- [ ] defmacro special form works
- [ ] Quasiquote/unquote/splice work
- [ ] Macros expand before evaluation
- [ ] Macros receive unevaluated args
- [ ] macroexpand function for debugging
- [ ] Tests for all macro features
- [ ] Standard macros (when, unless, and, or)
- [ ] Documentation with examples
- [ ] Hygiene issues documented
