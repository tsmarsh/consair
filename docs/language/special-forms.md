# Special Forms

Special forms are language constructs that don't follow standard evaluation rules. Unlike functions, special forms control when and whether their arguments are evaluated.

## quote

Prevents evaluation of an expression, returning it as data.

```lisp
(quote (1 2 3))     ; => (1 2 3)
'(1 2 3)            ; => (1 2 3) (shorthand)

(quote foo)         ; => foo (the symbol, not its value)
'foo                ; => foo

'(+ 1 2)            ; => (+ 1 2) (not 3)
```

Use `quote` to:
- Create literal lists
- Pass code as data
- Refer to symbols themselves

## if

Two or three branch conditional.

```lisp
(if test then)
(if test then else)
```

Evaluates `test`. If truthy, evaluates and returns `then`. Otherwise evaluates and returns `else` (or `nil` if no else clause).

```lisp
(if t "yes" "no")           ; => "yes"
(if nil "yes" "no")         ; => "no"
(if (> 5 3) "bigger")       ; => "bigger"
(if (< 5 3) "smaller")      ; => nil

; Nested
(if (> x 0)
    "positive"
    (if (< x 0)
        "negative"
        "zero"))
```

## cond

Multi-branch conditional (McCarthy's original design).

```lisp
(cond (test1 result1)
      (test2 result2)
      ...
      (t else-result))
```

Evaluates tests in order until one is truthy, then returns the corresponding result. Use `t` as the final test for a default case.

```lisp
(cond ((= x 1) "one")
      ((= x 2) "two")
      ((= x 3) "three")
      (t "other"))

; Classify a number
(label classify (lambda (n)
  (cond ((< n 0) 'negative)
        ((= n 0) 'zero)
        (t 'positive))))
```

If no test matches and there's no `t` clause, returns `nil`.

## lambda

Creates an anonymous function (closure).

```lisp
(lambda (params...) body)
```

Parameters are bound to arguments when the function is called. The body is a single expression (use nested expressions for multiple operations).

```lisp
; Single parameter
(lambda (x) (* x x))

; Multiple parameters
(lambda (x y) (+ x y))

; No parameters
(lambda () 42)

; Immediately invoked
((lambda (x) (* x x)) 5)    ; => 25

; Stored in variable (via label)
(label square (lambda (x) (* x x)))
(square 5)                   ; => 25
```

### Closures

Lambdas capture their lexical environment:

```lisp
(label make-counter (lambda (start)
  (lambda ()
    ; Captures 'start' from enclosing scope
    start)))

(label counter (make-counter 10))
(counter)                    ; => 10
```

Note: Consair closures capture by value, so the counter example doesn't increment. For mutable state, use a different pattern.

### Currying Pattern

```lisp
(label add (lambda (x)
  (lambda (y)
    (+ x y))))

((add 3) 4)                  ; => 7

(label add5 (add 5))
(add5 10)                    ; => 15
```

## label

Defines a named function, enabling recursion.

```lisp
(label name (lambda (params...) body))
```

The `name` is visible within the body, allowing direct recursion.

```lisp
; Recursive factorial
(label factorial (lambda (n)
  (cond ((= n 0) 1)
        (t (* n (factorial (- n 1)))))))

(factorial 5)                ; => 120

; Recursive list length
(label list-length (lambda (lst)
  (cond ((eq lst nil) 0)
        (t (+ 1 (list-length (cdr lst)))))))

(list-length '(a b c d))     ; => 4
```

### Inline Label Pattern

McCarthy's original form calls the function immediately:

```lisp
((label factorial (lambda (n)
   (cond ((= n 0) 1)
         (t (* n (factorial (- n 1)))))))
 5)                          ; => 120
```

### Top-Level Definitions

In Consair, `label` at the top level defines functions available throughout the program:

```lisp
; file.lisp
(label helper (lambda (x) (* x 2)))
(label main (lambda (n) (helper n)))
(main 21)                    ; => 42
```

## defmacro

Defines a macro for compile-time code transformation.

```lisp
(defmacro name (params...) body)
```

Unlike functions, macro arguments are NOT evaluated before being passed. The macro body should return code (as a list) that will then be evaluated.

```lisp
; Simple macro
(defmacro when (test body)
  (list 'cond (list test body)))

(when t (println "hello"))
; Expands to: (cond (t (println "hello")))

; Using quote
(defmacro unless (test body)
  (list 'cond (list (list 'not test) body)))

(unless nil (println "runs"))
; Expands to: (cond ((not nil) (println "runs")))
```

### Gensym for Hygiene

Use `gensym` to create unique symbols and avoid variable capture:

```lisp
(defmacro with-value (expr body)
  (let ((v (gensym "val")))
    (list 'let (list (list v expr))
      body)))
```

### Macro Expansion

Inspect how macros expand:

```lisp
(macroexpand-1 '(when t (println "hi")))
; => (cond (t (println "hi")))

(macroexpand '(when t (println "hi")))
; => fully expanded form
```

## Evaluation Order Summary

| Form | Evaluation |
|------|------------|
| `quote` | Argument NOT evaluated |
| `if` | Test always, then/else conditionally |
| `cond` | Tests in order, first truthy result |
| `lambda` | Body NOT evaluated until call |
| `label` | Binds name, body NOT evaluated until call |
| `defmacro` | Arguments NOT evaluated, result IS evaluated |

## Tail Call Optimization

Consair optimizes tail calls to prevent stack overflow in recursive functions:

```lisp
; Tail-recursive (optimized)
(label factorial-tail (lambda (n acc)
  (cond ((= n 0) acc)
        (t (factorial-tail (- n 1) (* n acc))))))

(factorial-tail 10000 1)     ; Works! No stack overflow

; Non-tail-recursive (not optimized)
(label factorial (lambda (n)
  (cond ((= n 0) 1)
        (t (* n (factorial (- n 1)))))))

(factorial 10000)            ; May overflow
```

A call is in tail position when it's the last thing a function does before returning.
