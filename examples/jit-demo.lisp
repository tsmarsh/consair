; JIT Compilation Demo
; Run with: cons --jit examples/jit-demo.lisp
; Or start REPL with: cons --jit

; Arithmetic operations are JIT-compiled to native code
(println "=== Arithmetic (JIT compiled) ===")
(println (+ 1 2 3 4 5))           ; 15
(println (* 2 3 4))                ; 24
(println (- 100 (* 2 25)))         ; 50
(println (/ 100 (+ 2 3)))          ; 20

; Recursive functions benefit most from JIT
(println "=== Recursive Functions (JIT compiled) ===")

; Factorial
(label fact
  (lambda (n)
    (cond
      ((= n 0) 1)
      (t (* n (fact (- n 1)))))))

(println (fact 10))  ; 3628800

; Fibonacci
(label fib
  (lambda (n)
    (cond
      ((< n 2) n)
      (t (+ (fib (- n 1)) (fib (- n 2)))))))

(println (fib 20))  ; 6765

; Closures work with JIT
(println "=== Closures (JIT compiled) ===")

(label make-adder
  (lambda (x)
    (lambda (y) (+ x y))))

(label add5 (make-adder 5))
(label add10 (make-adder 10))

(println (add5 3))   ; 8
(println (add10 7))  ; 17

; Higher-order functions
(label compose
  (lambda (f g)
    (lambda (x) (f (g x)))))

(label double (lambda (x) (* x 2)))
(label square (lambda (x) (* x x)))
(label double-then-square (compose square double))

(println (double-then-square 3))  ; 36 (3*2=6, 6*6=36)

; List operations
(println "=== List Operations (JIT compiled) ===")

(label lst '(1 2 3 4 5))
(println lst)
(println (car lst))              ; 1
(println (car (cdr (cdr lst))))  ; 3

; Macros are expanded before JIT compilation
(println "=== Macros (expanded then JIT compiled) ===")

(defmacro unless (condition body)
  `(cond (,condition nil) (t ,body)))

(println (unless nil "This prints!"))    ; "This prints!"
(println (unless t "This doesn't"))      ; nil

(defmacro square-macro (x)
  `(* ,x ,x))

(println (square-macro (+ 1 2)))  ; 9

; Conditionals
(println "=== Conditionals (JIT compiled) ===")

(label abs
  (lambda (n)
    (cond
      ((< n 0) (- 0 n))
      (t n))))

(println (abs -42))   ; 42
(println (abs 17))    ; 17

(label max
  (lambda (a b)
    (cond
      ((> a b) a)
      (t b))))

(println (max 10 20))  ; 20
(println (max 30 15))  ; 30

(println "=== JIT Demo Complete ===")
