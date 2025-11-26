# Standard Library

Consair's standard library provides essential functions for I/O, list manipulation, arithmetic, and more.

## Core List Operations

### cons
Construct a cons cell (pair).
```lisp
(cons 1 2)           ; => (1 . 2)
(cons 1 nil)         ; => (1)
(cons 1 '(2 3))      ; => (1 2 3)
```

### car
Get the first element of a cons cell.
```lisp
(car '(1 2 3))       ; => 1
(car '(a . b))       ; => a
```

### cdr
Get the rest of a cons cell.
```lisp
(cdr '(1 2 3))       ; => (2 3)
(cdr '(a . b))       ; => b
```

### list
Create a list from arguments.
```lisp
(list 1 2 3)         ; => (1 2 3)
(list 'a 'b)         ; => (a b)
(list)               ; => nil
```

### length
Get the length of a list.
```lisp
(length '(1 2 3))    ; => 3
(length nil)         ; => 0
```

### append
Concatenate two lists.
```lisp
(append '(1 2) '(3 4))  ; => (1 2 3 4)
(append nil '(1 2))     ; => (1 2)
```

### reverse
Reverse a list.
```lisp
(reverse '(1 2 3))   ; => (3 2 1)
```

### nth
Get the nth element (0-indexed).
```lisp
(nth '(a b c d) 0)   ; => a
(nth '(a b c d) 2)   ; => c
(nth '(a b c) 10)    ; => nil (out of bounds)
```

## Type Predicates

### atom
Test if value is an atom (not a cons cell).
```lisp
(atom 42)            ; => t
(atom 'foo)          ; => t
(atom '(1 2))        ; => nil
(atom nil)           ; => t
```

### eq
Test equality of atoms.
```lisp
(eq 'a 'a)           ; => t
(eq 1 1)             ; => t
(eq '(1) '(1))       ; => nil (different cons cells)
```

### nil?
Test if value is nil.
```lisp
(nil? nil)           ; => t
(nil? '())           ; => t
(nil? 0)             ; => nil
```

### cons?
Test if value is a cons cell.
```lisp
(cons? '(1 2))       ; => t
(cons? nil)          ; => nil
(cons? 42)           ; => nil
```

### number?
Test if value is a number.
```lisp
(number? 42)         ; => t
(number? 3.14)       ; => t
(number? "42")       ; => nil
```

### not
Logical negation.
```lisp
(not nil)            ; => t
(not t)              ; => nil
(not 0)              ; => nil (0 is truthy)
```

## Arithmetic

### + (addition)
```lisp
(+ 1 2)              ; => 3
(+ 1 2 3 4)          ; => 10
(+ 1.5 2.5)          ; => 4.0
```

### - (subtraction)
```lisp
(- 5 3)              ; => 2
(- 10 3 2)           ; => 5 (left to right)
```

### * (multiplication)
```lisp
(* 3 4)              ; => 12
(* 2 3 4)            ; => 24
```

### / (division)
```lisp
(/ 10 2)             ; => 5
(/ 1 2)              ; => 1/2 (exact rational)
(/ 1.0 2)            ; => 0.5 (float)
```

## Comparison

### = (numeric equality)
```lisp
(= 1 1)              ; => t
(= 1 2)              ; => nil
(= 1 1.0)            ; => t
```

### < (less than)
```lisp
(< 1 2)              ; => t
(< 2 1)              ; => nil
```

### > (greater than)
```lisp
(> 2 1)              ; => t
(> 1 2)              ; => nil
```

### <= (less than or equal)
```lisp
(<= 1 2)             ; => t
(<= 2 2)             ; => t
```

### >= (greater than or equal)
```lisp
(>= 2 1)             ; => t
(>= 2 2)             ; => t
```

## Vector Operations

### vector
Create a vector from arguments.
```lisp
(vector 1 2 3)       ; => <<1 2 3>>
```

### vector-ref
Get element by index (0-indexed).
```lisp
(vector-ref <<10 20 30>> 0)   ; => 10
(vector-ref <<10 20 30>> 2)   ; => 30
```

### vector-length
Get the length of a vector.
```lisp
(vector-length <<1 2 3 4>>)   ; => 4
```

## I/O Operations

### print
Print values without newline.
```lisp
(print "hello")              ; prints: hello
(print 1 2 3)                ; prints: 1 2 3
```

### println
Print values with newline.
```lisp
(println "hello")            ; prints: hello\n
(println "sum:" (+ 1 2))     ; prints: sum: 3\n
```

### slurp
Read entire file as string.
```lisp
(slurp "file.txt")           ; => "file contents..."
```

### spit
Write string to file.
```lisp
(spit "output.txt" "Hello, World!")
```

### shell
Execute shell command, return result map.
```lisp
(shell "ls -la")
; => ((out . "...") (err . "") (exit . 0) (success . t))
```

## Time

### now
Get current Unix timestamp.
```lisp
(now)                ; => 1732635600
```

## Macro Support

### gensym
Generate unique symbol (for macro hygiene).
```lisp
(gensym)             ; => g__0
(gensym "temp")      ; => temp__1
```

### macroexpand-1
Expand a macro call once.
```lisp
(macroexpand-1 '(when t (println "hi")))
```

### macroexpand
Fully expand all macros.
```lisp
(macroexpand '(when t (println "hi")))
```

## Collection Abstractions

These functions work with multiple collection types (lists, vectors, maps, sets).

### %seq
Convert collection to sequence.
```lisp
(%seq '(1 2 3))      ; => (1 2 3)
(%seq <<1 2 3>>)     ; => (1 2 3)
```

### %first
Get first element.
```lisp
(%first '(1 2 3))    ; => 1
(%first <<a b c>>)   ; => a
```

### %next
Get rest of sequence (nil if empty).
```lisp
(%next '(1 2 3))     ; => (2 3)
(%next '(1))         ; => nil
```

### %rest
Get rest of sequence (empty seq if empty).
```lisp
(%rest '(1 2 3))     ; => (2 3)
```

### %count
Count elements.
```lisp
(%count '(1 2 3))    ; => 3
(%count <<a b>>)     ; => 2
```

### %nth
Get nth element with optional default.
```lisp
(%nth <<1 2 3>> 1)           ; => 2
(%nth <<1 2 3>> 10 :missing) ; => :missing
```

### %get
Get value by key.
```lisp
(%get {:a 1 :b 2} :a)        ; => 1
(%get {:a 1} :x :default)    ; => :default
```

### %assoc
Associate key with value (returns new collection).
```lisp
(%assoc {:a 1} :b 2)         ; => {:a 1 :b 2}
(%assoc <<1 2 3>> 0 10)      ; => <<10 2 3>>
```

### %conj
Add item to collection.
```lisp
(%conj '(2 3) 1)             ; => (1 2 3)
(%conj <<1 2>> 3)            ; => <<1 2 3>>
(%conj #{1 2} 3)             ; => #{1 2 3}
```

### %hash-map
Create hash map from key-value pairs.
```lisp
(%hash-map :a 1 :b 2)        ; => {:a 1 :b 2}
```

### %hash-set
Create hash set from elements.
```lisp
(%hash-set 1 2 3)            ; => #{1 2 3}
```

### %empty?
Test if collection is empty.
```lisp
(%empty? '())                ; => t
(%empty? <<>>)               ; => t
(%empty? '(1))               ; => nil
```

### %contains?
Test if collection contains key/element.
```lisp
(%contains? {:a 1} :a)       ; => t
(%contains? #{1 2 3} 2)      ; => t
```

### %keys
Get keys from map.
```lisp
(%keys {:a 1 :b 2})          ; => (:a :b)
```

### %vals
Get values from map.
```lisp
(%vals {:a 1 :b 2})          ; => (1 2)
```

### %dissoc
Remove key from map.
```lisp
(%dissoc {:a 1 :b 2} :a)     ; => {:b 2}
```

### %disj
Remove element from set.
```lisp
(%disj #{1 2 3} 2)           ; => #{1 3}
```

### %reduced
Wrap value for early termination in reduce.
```lisp
(%reduced 42)                ; => #reduced(42)
```

### %reduced?
Test if value is reduced.
```lisp
(%reduced? (%reduced 42))    ; => t
```

### %unreduced
Unwrap reduced value.
```lisp
(%unreduced (%reduced 42))   ; => 42
```
