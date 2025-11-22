; Advanced examples demonstrating McCarthy's elegant minimal Lisp

; Define some helper functions
(label null (lambda (x) (atom x)))
(label and (lambda (x y) (cond (x (cond (y t) (t nil))) (t nil))))

; Map function - apply function to each element
(label mapcar
  (lambda (f lst)
    (cond ((null lst) nil)
          (t (cons (f (car lst)) (mapcar f (cdr lst)))))))

; Define a doubling function (using cons since we don't have arithmetic)
(label double-list (lambda (x) (cons x x)))

; Apply it
(mapcar double-list '(a b c))

; Filter function - keep elements matching predicate
(label filter
  (lambda (pred lst)
    (cond ((null lst) nil)
          ((pred (car lst)) (cons (car lst) (filter pred (cdr lst))))
          (t (filter pred (cdr lst))))))

; Keep only atoms
(filter atom '(a (b c) d e (f)))

; Association lists (simple key-value store)
(label assoc
  (lambda (key alist)
    (cond ((null alist) nil)
          ((eq key (car (car alist))) (car alist))
          (t (assoc key (cdr alist))))))

; Define an association list
(label my-alist '((name . Alice) (age . 30) (city . NYC)))

; Lookup values
(assoc 'name my-alist)
(assoc 'age my-alist)
(assoc 'missing my-alist)

; Higher-order functions: compose two functions
(label compose
  (lambda (f g)
    (lambda (x) (f (g x)))))

; Example: car of cdr
(label second (compose car cdr))
(second '(1 2 3 4))

; Demonstrate structure sharing with Rc
(label shared-tail '(3 4 5))
(label list1 (cons 1 shared-tail))
(label list2 (cons 2 shared-tail))

; Both lists share the same tail structure
list1
list2

; Y-combinator for recursion without label
; (This is more theoretical - we have label which is simpler)
(label Y
  (lambda (f)
    ((lambda (x) (f (lambda (y) ((x x) y))))
     (lambda (x) (f (lambda (y) ((x x) y)))))))

; Demonstrates the power of lambda calculus
