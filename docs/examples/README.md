# Example Programs

This directory contains example Consair programs demonstrating various language features.

## Running Examples

```bash
# With interpreter
cons examples/factorial.lisp

# With JIT
cons --jit examples/factorial.lisp

# With AOT
cadr examples/factorial.lisp -o /tmp/factorial.ll
lli /tmp/factorial.ll
```

## Basic Examples

### Hello World
```lisp
(println "Hello, World!")
```

### Factorial
```lisp
(label factorial (lambda (n)
  (cond ((= n 0) 1)
        (t (* n (factorial (- n 1)))))))

(println "5! =" (factorial 5))
(println "10! =" (factorial 10))
```

### Fibonacci
```lisp
(label fib (lambda (n)
  (cond ((< n 2) n)
        (t (+ (fib (- n 1))
              (fib (- n 2)))))))

(println "fib(10) =" (fib 10))
```

### Tail-Recursive Factorial
```lisp
; Uses accumulator to enable tail call optimization
(label factorial-tail (lambda (n acc)
  (cond ((= n 0) acc)
        (t (factorial-tail (- n 1) (* n acc))))))

(println "20! =" (factorial-tail 20 1))
```

## List Processing

### Map (apply function to each element)
```lisp
(label map (lambda (f lst)
  (cond ((eq lst nil) nil)
        (t (cons (f (car lst))
                 (map f (cdr lst)))))))

(label square (lambda (x) (* x x)))
(println (map square '(1 2 3 4 5)))
; => (1 4 9 16 25)
```

### Filter (keep elements matching predicate)
```lisp
(label filter (lambda (pred lst)
  (cond ((eq lst nil) nil)
        ((pred (car lst))
         (cons (car lst) (filter pred (cdr lst))))
        (t (filter pred (cdr lst))))))

(label even? (lambda (n) (= 0 (- n (* 2 (/ n 2))))))
(println (filter even? '(1 2 3 4 5 6 7 8)))
; => (2 4 6 8)
```

### Reduce (fold list to single value)
```lisp
(label reduce (lambda (f init lst)
  (cond ((eq lst nil) init)
        (t (reduce f (f init (car lst)) (cdr lst))))))

(println (reduce + 0 '(1 2 3 4 5)))
; => 15
```

### Reverse
```lisp
(label reverse-acc (lambda (lst acc)
  (cond ((eq lst nil) acc)
        (t (reverse-acc (cdr lst) (cons (car lst) acc))))))

(label reverse (lambda (lst) (reverse-acc lst nil)))

(println (reverse '(1 2 3 4 5)))
; => (5 4 3 2 1)
```

## Higher-Order Functions

### Compose
```lisp
(label compose (lambda (f g)
  (lambda (x) (f (g x)))))

(label double (lambda (x) (* x 2)))
(label inc (lambda (x) (+ x 1)))

(label double-then-inc (compose inc double))
(println (double-then-inc 5))
; => 11
```

### Curry
```lisp
(label curry2 (lambda (f)
  (lambda (x)
    (lambda (y)
      (f x y)))))

(label add (lambda (x y) (+ x y)))
(label curried-add (curry2 add))
(label add5 ((curried-add) 5))
(println (add5 10))
; => 15
```

## Working with Vectors

```lisp
; Create a vector
(label v <<1 2 3 4 5>>)

; Access elements
(println "First:" (vector-ref v 0))
(println "Third:" (vector-ref v 2))
(println "Length:" (vector-length v))

; Sum vector elements
(label sum-vec (lambda (v i acc)
  (cond ((= i (vector-length v)) acc)
        (t (sum-vec v (+ i 1) (+ acc (vector-ref v i)))))))

(println "Sum:" (sum-vec v 0 0))
```

## Mutual Recursion

```lisp
(label even? (lambda (n)
  (cond ((= n 0) t)
        (t (odd? (- n 1))))))

(label odd? (lambda (n)
  (cond ((= n 0) nil)
        (t (even? (- n 1))))))

(println "even?(10):" (even? 10))
(println "odd?(10):" (odd? 10))
```

## Closures

```lisp
; Counter factory
(label make-counter (lambda (start)
  (lambda () start)))  ; Note: Consair closures capture by value

; Adder factory
(label make-adder (lambda (n)
  (lambda (x) (+ x n))))

(label add10 (make-adder 10))
(println (add10 5))   ; => 15
(println (add10 20))  ; => 30
```

## Arithmetic with Rationals

```lisp
; Exact rational arithmetic
(println "1/2 + 1/3 =" (+ (/ 1 2) (/ 1 3)))
; => 5/6

(println "1/2 * 2/3 =" (* (/ 1 2) (/ 2 3)))
; => 1/3

; Convert to float
(println "As float:" (* 1.0 (/ 1 3)))
; => 0.3333...
```

## File I/O

```lisp
; Write to file
(spit "/tmp/hello.txt" "Hello from Consair!\n")

; Read from file
(println (slurp "/tmp/hello.txt"))

; Process file lines (simple approach)
(label content (slurp "/tmp/hello.txt"))
(println "File contents:" content)
```

## Shell Commands

```lisp
(label result (shell "echo 'Hello from shell'"))
(println "stdout:" (cdr (car result)))
(println "exit code:" (cdr (car (cdr (cdr result)))))
```

## Macros

```lisp
; Define 'when' macro
(defmacro when (test body)
  (list 'cond (list test body)))

(when t (println "This prints"))
(when nil (println "This doesn't"))

; Define 'unless' macro
(defmacro unless (test body)
  (list 'cond (list (list 'not test) body)))

(unless nil (println "This prints"))

; See macro expansion
(println (macroexpand '(when t (println "hi"))))
```

## Trees (Binary)

```lisp
; Tree as (value left right) or nil for empty

(label make-tree (lambda (val left right)
  (cons val (cons left (cons right nil)))))

(label tree-val (lambda (tree) (car tree)))
(label tree-left (lambda (tree) (car (cdr tree))))
(label tree-right (lambda (tree) (car (cdr (cdr tree)))))

(label tree-sum (lambda (tree)
  (cond ((eq tree nil) 0)
        (t (+ (tree-val tree)
              (+ (tree-sum (tree-left tree))
                 (tree-sum (tree-right tree))))))))

; Build a tree:      5
;                   / \
;                  3   7
;                 /     \
;                1       9

(label t (make-tree 5
           (make-tree 3 (make-tree 1 nil nil) nil)
           (make-tree 7 nil (make-tree 9 nil nil))))

(println "Tree sum:" (tree-sum t))
; => 25
```

## Quicksort

```lisp
(label filter (lambda (pred lst)
  (cond ((eq lst nil) nil)
        ((pred (car lst))
         (cons (car lst) (filter pred (cdr lst))))
        (t (filter pred (cdr lst))))))

(label quicksort (lambda (lst)
  (cond ((eq lst nil) nil)
        (t (append
             (quicksort (filter (lambda (x) (< x (car lst))) (cdr lst)))
             (cons (car lst)
               (quicksort (filter (lambda (x) (>= x (car lst))) (cdr lst)))))))))

(println (quicksort '(3 1 4 1 5 9 2 6 5 3)))
; => (1 1 2 3 3 4 5 5 6 9)
```
