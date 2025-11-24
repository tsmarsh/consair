; Test the example macros from the proposal

; Define when macro
(defmacro when (condition body)
  `(cond (,condition ,body) (t nil)))

; Test when
(println "Testing when macro:")
(println (when t 42))  ; Should print 42
(println (when nil 99))  ; Should print nil

; Define unless macro
(defmacro unless (condition body)
  `(cond (,condition nil) (t ,body)))

; Test unless
(println "Testing unless macro:")
(println (unless nil 100))  ; Should print 100
(println (unless t 200))    ; Should print nil

; Define and macro
(defmacro and (a b)
  `(cond (,a ,b) (t nil)))

; Test and
(println "Testing and macro:")
(println (and t t))        ; Should print t
(println (and t nil))      ; Should print nil
(println (and nil t))      ; Should print nil

; Define or macro
(defmacro or (a b)
  `(cond (,a t) (t ,b)))

; Test or
(println "Testing or macro:")
(println (or t nil))       ; Should print t
(println (or nil t))       ; Should print t
(println (or nil nil))     ; Should print nil

; Test macroexpand
(println "Testing macroexpand:")
(println (macroexpand '(when t 42)))

; Test gensym
(println "Testing gensym:")
(println (gensym))
(println (gensym "temp"))
