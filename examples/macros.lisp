; Macro System Examples for Consair Lisp
; Demonstrates unhygienic macros in Common Lisp style

(println "=== Macro System Examples ===")
(println "")

; Example 1: Basic quasiquote
(println "1. Basic Quasiquote:")
(println `(a b c))  ; => (a b c)
(println "")

; Example 2: Quasiquote with unquote
(println "2. Quasiquote with Unquote:")
(println `(1 ,(+ 2 3) 4))  ; => (1 5 4)
(println "")

; Example 3: Quasiquote with unquote-splicing
(println "3. Unquote-Splicing:")
(println `(a ,@(cons 1 (cons 2 nil)) b))  ; => (a 1 2 b)
(println "")

; Example 4: Define and use 'when' macro
(println "4. When Macro:")
(defmacro when (condition body)
  `(cond (,condition ,body) (t nil)))

(when t (println "  This executes!"))
(when nil (println "  This doesn't execute"))
(println "")

; Example 5: Define and use 'unless' macro
(println "5. Unless Macro:")
(defmacro unless (condition body)
  `(cond (,condition nil) (t ,body)))

(unless nil (println "  This executes!"))
(unless t (println "  This doesn't execute"))
(println "")

; Example 6: Define custom 'and' macro
(println "6. And Macro:")
(defmacro and (a b)
  `(cond (,a ,b) (t nil)))

(println (and t t))     ; => t
(println (and t nil))   ; => nil
(println (and nil t))   ; => nil
(println "")

; Example 7: Define custom 'or' macro
(println "7. Or Macro:")
(defmacro or (a b)
  `(cond (,a t) (t ,b)))

(println (or t nil))    ; => t
(println (or nil t))    ; => t
(println (or nil nil))  ; => nil
(println "")

; Example 8: Macro expansion debugging
(println "8. Macro Expansion:")
(println "Original: (when (> 5 3) 42)")
(println "Expanded:")
(println (macroexpand '(when (> 5 3) 42)))
(println "")

; Example 9: Generate unique symbols with gensym
(println "9. Gensym for Hygiene:")
(println (gensym))
(println (gensym "temp"))
(println (gensym "temp"))  ; Different from previous
(println "")

; Example 10: More complex macro - let-like binding
(println "10. Complex Macro - Simple Let:")
(defmacro simple-let (var val body)
  `((lambda (,var) ,body) ,val))

(println (simple-let x 100 (+ x 10)))  ; => 110
(println "")

(println "=== All Examples Complete ===")
