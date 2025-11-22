; Functions and closures in minimal Lisp

; Anonymous functions (lambda)
((lambda (x) x) 42)
((lambda (x y) (cons x y)) 'a 'b)

; Named functions using label
(label identity (lambda (x) x))
(identity 'hello)

; Function that returns a function (closure)
(label make-const (lambda (x) (lambda (y) x)))
(label always-42 (make-const 42))
(always-42 'anything)

; Pair manipulation
(label first (lambda (p) (car p)))
(label second (lambda (p) (cdr p)))
(label make-pair (lambda (x y) (cons x y)))

(label my-pair (make-pair 'a 'b))
(first my-pair)
(second my-pair)

; List utilities
(label null (lambda (x) (atom x)))
(label append
  (lambda (x y)
    (cond ((null x) y)
          (t (cons (car x) (append (cdr x) y))))))

(append '(1 2) '(3 4))

; Reverse a list
(label reverse-helper
  (lambda (lst acc)
    (cond ((null lst) acc)
          (t (reverse-helper (cdr lst) (cons (car lst) acc))))))

(label reverse (lambda (lst) (reverse-helper lst nil)))
(reverse '(1 2 3 4))
