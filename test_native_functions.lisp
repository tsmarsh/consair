; Test native functions work correctly

(println "Testing list operations:")
(println (atom 1))          ; Should print t
(println (atom (cons 1 2))) ; Should print nil
(println (eq 1 1))          ; Should print t
(println (eq 1 2))          ; Should print nil
(println (car (cons 1 2)))  ; Should print 1
(println (cdr (cons 1 2)))  ; Should print 2

(println "")
(println "Testing arithmetic:")
(println (+ 1 2))           ; Should print 3
(println (+ 1 2 3 4))       ; Should print 10
(println (- 10 3))          ; Should print 7
(println (- 10 3 2))        ; Should print 5
(println (* 2 3))           ; Should print 6
(println (* 2 3 4))         ; Should print 24
(println (/ 10 2))          ; Should print 5
(println (/ 100 10 2))      ; Should print 5

(println "")
(println "Testing comparison:")
(println (< 1 2))           ; Should print t
(println (< 2 1))           ; Should print nil
(println (> 2 1))           ; Should print t
(println (> 1 2))           ; Should print nil
(println (<= 1 1))          ; Should print t
(println (<= 2 1))          ; Should print nil
(println (>= 1 1))          ; Should print t
(println (>= 1 2))          ; Should print nil
(println (= 1 1))           ; Should print t
(println (= 1 2))           ; Should print nil

(println "")
(println "Testing vector constructor:")
(println (vector 1 2 3))    ; Should print << 1 2 3 >>
