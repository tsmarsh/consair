(label factorial (lambda (n) (cond ((eq n 0) 1) (t (cons n (factorial (cdr (cons nil n))))))))
(factorial 5)
