# Data Types

Consair has a rich type system that extends McCarthy's original atoms and cons cells with modern data structures.

## Atoms

Atoms are indivisible values. In Consair, atoms include:

### Symbols

Symbols are named identifiers used for variables and function names.

```lisp
foo          ; symbol
my-function  ; symbol with hyphen
+            ; symbol (the addition function)
nil?         ; symbol with question mark
```

Symbols are interned for efficient comparison.

### Numbers

Consair supports multiple numeric types with automatic promotion:

#### Integers (i64)
```lisp
42
-17
0
```

#### Big Integers (arbitrary precision)
```lisp
; Automatically promoted on overflow
99999999999999999999999999999999
```

#### Rationals (exact fractions)
```lisp
1/2          ; one half
3/4          ; three quarters
(/ 1 3)      ; => 1/3 (exact, not 0.333...)
```

#### Floats (IEEE 754 double)
```lisp
3.14159
-0.5
1.0e10       ; scientific notation
```

Numeric operations automatically promote types as needed:
- `Int + Int = Int` (promotes to BigInt on overflow)
- `Int / Int = Ratio` (exact division)
- `Any + Float = Float`

### Strings

Strings are enclosed in double quotes with escape sequences:

```lisp
"Hello, World!"
"Line 1\nLine 2"     ; newline
"Tab\there"          ; tab
"Quote: \"hi\""      ; escaped quote
"Backslash: \\"      ; escaped backslash
```

### Booleans

```lisp
t            ; true (actually the symbol t)
nil          ; false/empty list
```

Note: `t` is the symbol `t` bound to itself. Any non-nil value is truthy.

## Nil

`nil` represents both the empty list and false:

```lisp
nil          ; empty list / false
'()          ; also nil (quoted empty list)
```

## Cons Cells (Lists)

The fundamental compound data structure. A cons cell holds two values: `car` (head) and `cdr` (tail).

```lisp
; Create cons cells
(cons 1 2)           ; => (1 . 2) - a dotted pair
(cons 1 nil)         ; => (1) - a one-element list
(cons 1 (cons 2 nil)); => (1 2) - a two-element list

; Access
(car '(1 2 3))       ; => 1
(cdr '(1 2 3))       ; => (2 3)

; Quoted lists (shorthand)
'(1 2 3)             ; => (1 2 3)
'(a b c)             ; => (a b c)
```

### Proper Lists vs Improper Lists

A proper list ends in `nil`:
```lisp
'(1 2 3)             ; proper list: (cons 1 (cons 2 (cons 3 nil)))
```

An improper list (dotted pair) ends in a non-nil value:
```lisp
(cons 1 2)           ; improper: (1 . 2)
'(1 2 . 3)           ; improper: (cons 1 (cons 2 3))
```

## Vectors

Vectors provide fast random access (O(1)) unlike lists (O(n)):

```lisp
; Literal syntax
<<1 2 3 4 5>>

; With expressions
<<(+ 1 2) (* 3 4) 5>>  ; => <<3 12 5>>

; Access
(vector-ref <<10 20 30>> 0)   ; => 10
(vector-ref <<10 20 30>> 2)   ; => 30

; Length
(vector-length <<1 2 3>>)     ; => 3

; Constructor function
(vector 1 2 3)                ; => <<1 2 3>>
```

## Maps

Hash maps store key-value pairs:

```lisp
; Literal syntax
{:name "Alice" :age 30}

; Access
(%get {:a 1 :b 2} :a)         ; => 1
(%get {:a 1} :missing :default); => :default

; Modify (returns new map)
(%assoc {:a 1} :b 2)          ; => {:a 1 :b 2}
(%dissoc {:a 1 :b 2} :a)      ; => {:b 2}

; Query
(%keys {:a 1 :b 2})           ; => (:a :b)
(%vals {:a 1 :b 2})           ; => (1 2)
(%contains? {:a 1} :a)        ; => t
```

## Sets

Hash sets store unique values:

```lisp
; Literal syntax
#{1 2 3}

; Operations
(%conj #{1 2} 3)              ; => #{1 2 3}
(%disj #{1 2 3} 2)            ; => #{1 3}
(%contains? #{1 2 3} 2)       ; => t
```

## Persistent Collections

For functional programming with structural sharing:

```lisp
; Persistent vector
#pvec[1 2 3]

; Persistent map
#pmap{:a 1 :b 2}

; Persistent set
#pset{1 2 3}
```

These collections are immutable - operations return new collections while sharing structure with the original for efficiency.

## Lambdas

First-class functions:

```lisp
(lambda (x) (* x x))          ; anonymous function

; With closure
(label make-adder (lambda (n)
  (lambda (x) (+ x n))))
((make-adder 5) 10)           ; => 15
```

## Macros

Compile-time code transformers:

```lisp
(defmacro when (test body)
  (list 'cond (list test body)))

(when t (println "hello"))    ; expands to (cond (t (println "hello")))
```

## Type Predicates

```lisp
(atom x)      ; t if x is an atom (not a cons cell)
(nil? x)      ; t if x is nil
(cons? x)     ; t if x is a cons cell
(number? x)   ; t if x is a number
```

## Type Coercion

Consair performs minimal implicit coercion:
- Numeric types promote as needed in arithmetic
- No implicit string conversion

For explicit conversion, use printing:
```lisp
(println 42)  ; prints "42"
```
