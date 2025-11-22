(label make-adder (lambda (x) (lambda (y) (cons x y))))
(label add-5 (make-adder 5))
(add-5 10)
