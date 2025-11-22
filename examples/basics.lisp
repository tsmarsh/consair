; Basic examples of McCarthy's minimal Lisp

; Quote - return unevaluated
(quote a)
'(1 2 3)

; Atom - test if value is atomic
(atom 'x)
(atom '(1 2))

; Eq - test equality of atoms
(eq 'a 'a)
(eq 1 1)

; Car and Cdr - list operations
(car '(1 2 3))
(cdr '(1 2 3))

; Cons - build lists
(cons 1 '(2 3))
(cons 'a (cons 'b (cons 'c nil)))

; Cond - conditional evaluation
(cond ((eq 1 1) 'yes) (t 'no))
(cond ((atom '(1 2)) 'atomic) ((eq 1 1) 'equal) (t 'neither))
