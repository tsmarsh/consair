# Add Tail Call Optimization (TCO) with Recursion Safety

## Problem

The interpreter has no protection against stack overflow and doesn't optimize tail calls, which limits the depth of recursive functions and contradicts idiomatic Lisp programming style.

**Location:** `consair-core/src/interpreter.rs`

## Impact

- Stack overflow crashes on deep recursion
- Cannot write natural tail-recursive functions
- Forces unnatural iterative patterns
- Limits functional programming style
- Poor error messages when crashes occur

## Background: What is Tail Call Optimization?

A **tail call** is a function call that is the last operation before returning. TCO transforms these calls from recursive (consuming stack) to iterative (reusing stack frame):

```lisp
; TAIL RECURSIVE (can be optimized)
(define (factorial n acc)
  (if (= n 0)
      acc
      (factorial (- n 1) (* n acc))))  ; Last operation is the call

; NOT tail recursive (cannot be optimized)
(define (bad-factorial n)
  (if (= n 0)
      1
      (* n (bad-factorial (- n 1)))))  ; Must multiply AFTER the call
```

With TCO, the first version can run forever without stack overflow. Without TCO, both hit stack limits around depth 1000.

## Proposed Solution: Hybrid Approach

Implement TCO with a safety recursion limit:

1. **Transform `eval()` to iterative loop** - optimize tail position calls
2. **Keep depth tracking** - prevent runaway non-tail recursion
3. **Phased implementation** - start simple, add complexity later

### Benefits

✅ Enables unlimited tail recursion (idiomatic Lisp)
✅ Still prevents crashes from non-tail recursion
✅ Matches Scheme semantics (required by R5RS)
✅ Prepares for LLVM backend (which supports TCO)
✅ Enables accumulator-passing style
✅ Educational value - shows power of proper tail calls

### Trade-offs

⚠️ More complex than simple depth limit
⚠️ Stack traces less helpful (fewer frames)
⚠️ Requires careful identification of tail positions
✅ But: worth it for a serious Lisp implementation

## Implementation Plan

### Phase 1: Core TCO (This PR)

Transform `eval()` from recursive to iterative for tail calls:

```rust
// Current (recursive)
pub fn eval(expr: &Value, env: &Environment) -> Result<Value, String> {
    match expr {
        // ... cases that call eval() recursively
    }
}

// New (iterative with TCO)
pub fn eval(expr: &Value, env: &Environment) -> Result<Value, String> {
    let mut expr = expr.clone();
    let mut env = env.clone();
    let mut depth = 0;
    const MAX_DEPTH: usize = 10000;  // Much higher with TCO
    
    loop {
        if depth >= MAX_DEPTH {
            return Err(format!(
                "Maximum recursion depth ({}) exceeded. \
                 Even with tail call optimization, this indicates \
                 a non-tail recursive function went too deep.",
                MAX_DEPTH
            ));
        }
        
        match expr {
            // Self-evaluating - return immediately
            Value::Atom(AtomType::Number(_)) => return Ok(expr),
            Value::Nil => return Ok(expr),
            // ... other self-evaluating cases
            
            // Tail positions - update state and continue loop
            Value::Cons(ref cell) => {
                match identify_form(&cell.car) {
                    Form::If => {
                        // Evaluate condition (non-tail)
                        let cond = eval_non_tail(&get_condition(cell), &env, depth)?;
                        
                        // Branch is tail position - just update and loop
                        if is_truthy(&cond) {
                            expr = get_then_branch(cell)?;
                        } else {
                            expr = get_else_branch(cell)?;
                        }
                        depth += 1;
                        continue;  // <-- TCO magic!
                    }
                    
                    Form::FunctionCall => {
                        // Evaluate function and args (non-tail)
                        let func = eval_non_tail(&cell.car, &env, depth)?;
                        let args = eval_args_non_tail(&cell.cdr, &env, depth)?;
                        
                        match func {
                            Value::Lambda(lambda) => {
                                // Tail call - reuse stack frame
                                let mut new_env = lambda.env.clone();
                                for (param, arg) in lambda.params.iter().zip(args) {
                                    new_env.define(param.clone(), arg);
                                }
                                
                                expr = lambda.body.clone();
                                env = new_env;
                                depth += 1;
                                continue;  // <-- TCO magic!
                            }
                            
                            Value::NativeFn(f) => {
                                // Native functions can't be tail-optimized
                                return f(&args, &env);
                            }
                            
                            _ => return Err("Not a function".to_string()),
                        }
                    }
                }
            }
        }
    }
}

// Helper for non-tail recursive calls
fn eval_non_tail(expr: &Value, env: &Environment, depth: usize) -> Result<Value, String> {
    // This creates a new stack frame (actual recursion)
    // But only for non-tail positions like:
    // - Evaluating function arguments
    // - Evaluating if conditions
    // - Evaluating operator in (+ (f x) (g y))
    
    eval_with_depth(expr, env, depth + 1)
}

fn eval_with_depth(expr: &Value, env: &Environment, depth: usize) -> Result<Value, String> {
    // Wrapper that tracks depth through actual recursive calls
    // This is called from eval_non_tail
    
    let mut expr = expr.clone();
    let mut env = env.clone();
    let mut current_depth = depth;
    
    // Rest of loop logic...
}
```

### Phase 2: Identify All Tail Positions

Special forms with tail positions:

```lisp
; if - both branches are tail position
(if test 
    TAIL     ; This is in tail position
    TAIL)    ; This is too

; cond - all result expressions are tail position
(cond
  (test1 TAIL)
  (test2 TAIL)
  (else TAIL))

; begin - only LAST expression is tail position
(begin
  NOT-TAIL   ; Side effects
  NOT-TAIL   ; More side effects
  TAIL)      ; Only this one

; lambda - body is tail position
(lambda (x) TAIL)

; let - body is tail position
(let ((x 1) (y 2))
  TAIL)

; Function call - the call itself is tail position
(f x y)  ; If this is last thing before return
```

### Phase 3: Optimize Special Forms

```rust
impl TailOptimizer {
    fn optimize_special_form(
        &mut self,
        form: SpecialForm,
        expr: &Value,
        env: &mut Environment,
        depth: &mut usize
    ) -> ControlFlow {
        match form {
            SpecialForm::If => {
                let (condition, then_branch, else_branch) = parse_if(expr)?;
                
                // Condition is NOT tail - evaluate with new frame
                let cond_val = eval_non_tail(&condition, env, *depth)?;
                
                // Branch IS tail - update state and continue
                if is_truthy(&cond_val) {
                    self.expr = then_branch;
                } else {
                    self.expr = else_branch;
                }
                *depth += 1;
                ControlFlow::Continue
            }
            
            SpecialForm::Begin => {
                let mut exprs = parse_begin(expr)?;
                
                // All but last are NOT tail
                let last = exprs.pop().ok_or("begin: empty body")?;
                for e in exprs {
                    eval_non_tail(&e, env, *depth)?;
                }
                
                // Last IS tail
                self.expr = last;
                *depth += 1;
                ControlFlow::Continue
            }
            
            SpecialForm::Cond => {
                let clauses = parse_cond(expr)?;
                
                for (test, result) in clauses {
                    // Test is NOT tail
                    let test_val = eval_non_tail(&test, env, *depth)?;
                    
                    if is_truthy(&test_val) {
                        // Result IS tail
                        self.expr = result;
                        *depth += 1;
                        return ControlFlow::Continue;
                    }
                }
                
                ControlFlow::Return(Value::Nil)
            }
            
            // ... other special forms
        }
    }
}
```

## Testing Strategy

### Test 1: Deep Tail Recursion (Should Succeed)

```lisp
; Tail recursive - should handle millions of calls
(define (count-down n)
  (if (= n 0)
      "done"
      (count-down (- n 1))))

(count-down 1000000)  ; Should succeed with TCO
```

### Test 2: Deep Non-Tail Recursion (Should Fail Gracefully)

```lisp
; Non-tail recursive - should hit depth limit
(define (bad-count n)
  (if (= n 0)
      0
      (+ 1 (bad-count (- n 1)))))

(bad-count 100000)  ; Should error with clear message
```

### Test 3: Tail Recursive Accumulator Pattern

```lisp
; Classic accumulator pattern
(define (factorial n acc)
  (if (= n 0)
      acc
      (factorial (- n 1) (* n acc))))

(factorial 10000 1)  ; Should succeed
```

### Test 4: Mutual Tail Recursion

```lisp
; Mutually recursive tail calls
(define (even? n)
  (if (= n 0)
      true
      (odd? (- n 1))))

(define (odd? n)
  (if (= n 0)
      false
      (even? (- n 1))))

(even? 1000000)  ; Should succeed with TCO
```

### Test 5: Mixed Tail and Non-Tail

```lisp
; Some calls optimized, some not
(define (fib-tail n a b)
  (if (= n 0)
      a
      (fib-tail (- n 1) b (+ a b))))  ; Tail call - optimized

(define (fib-bad n)
  (if (< n 2)
      n
      (+ (fib-bad (- n 1))           ; Non-tail - not optimized
         (fib-bad (- n 2)))))        ; Non-tail - not optimized

(fib-tail 100000 0 1)  ; Should succeed
(fib-bad 30)           ; Should succeed (not deep enough)
(fib-bad 10000)        ; Should fail (too deep, non-tail)
```

### Test 6: Verify Stack Depth

```rust
#[test]
fn test_tail_call_constant_stack() {
    let env = Environment::new();
    
    // This should use constant stack space
    let code = r#"
        (define (loop n)
          (if (= n 0)
              "done"
              (loop (- n 1))))
        (loop 1000000)
    "#;
    
    let result = eval_str(code, &env);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), string("done"));
}

#[test]
fn test_non_tail_hits_limit() {
    let env = Environment::new();
    
    let code = r#"
        (define (non-tail n)
          (if (= n 0)
              0
              (+ 1 (non-tail (- n 1)))))
        (non-tail 100000)
    "#;
    
    let result = eval_str(code, &env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("recursion depth"));
}

#[test]
fn test_mutual_recursion_tco() {
    let env = Environment::new();
    
    let code = r#"
        (define (even? n)
          (if (= n 0) true (odd? (- n 1))))
        (define (odd? n)
          (if (= n 0) false (even? (- n 1))))
        (even? 1000000)
    "#;
    
    let result = eval_str(code, &env);
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::Atom(AtomType::Bool(true)));
}

#[test]
fn test_begin_tail_position() {
    let env = Environment::new();
    
    let code = r#"
        (define (loop n)
          (if (= n 0)
              "done"
              (begin
                (println n)      ; NOT tail
                (loop (- n 1))))) ; IS tail
        (loop 10000)
    "#;
    
    let result = eval_str(code, &env);
    assert!(result.is_ok());
}
```

## Error Messages

### Before (Stack Overflow)
```
thread 'main' panicked at 'stack overflow'
```

### After (With TCO)
```
Error: Maximum recursion depth (10000) exceeded.
Even with tail call optimization, this indicates a non-tail 
recursive function went too deep.

Common causes:
- Using non-tail recursion (e.g., (+ 1 (f n)))
- Infinite recursion with no base case
- Mutual recursion that isn't tail-recursive

Consider:
- Converting to tail-recursive form with accumulator
- Using iteration instead of recursion
- Adding a base case to stop recursion
```

## Documentation Updates

Add to language documentation:

```markdown
## Tail Call Optimization

This Lisp implementation provides proper tail call optimization (TCO)
as required by the Scheme standard. This means tail-recursive functions
can run indefinitely without stack overflow.

### What is Tail Position?

A call is in tail position if it's the last operation before returning:

```lisp
; Tail recursive - optimized ✓
(define (countdown n)
  (if (= n 0)
      "done"
      (countdown (- n 1))))  ; Last operation

; Not tail recursive - NOT optimized ✗
(define (bad-sum n)
  (if (= n 0)
      0
      (+ n (bad-sum (- n 1)))))  ; Addition happens AFTER call
```

### Converting to Tail-Recursive Form

Use an accumulator parameter:

```lisp
; Non-tail version
(define (factorial n)
  (if (= n 0)
      1
      (* n (factorial (- n 1)))))

; Tail-recursive version
(define (factorial n)
  (define (fact-helper n acc)
    (if (= n 0)
        acc
        (fact-helper (- n 1) (* n acc))))
  (fact-helper n 1))
```

### Recursion Limits

- Tail calls: unlimited (constant stack space)
- Non-tail calls: limited to 10,000 depth
- This prevents crashes while allowing natural recursion
```

## Implementation Checklist

### Phase 1: Basic TCO (Week 1)
- [ ] Transform `eval()` to loop-based structure
- [ ] Identify tail positions in function calls
- [ ] Optimize tail calls in `if` special form
- [ ] Add depth tracking for non-tail calls
- [ ] Write basic tests (tail vs non-tail)
- [ ] Update error messages

### Phase 2: Special Forms (Week 2)
- [ ] Optimize `cond` tail positions
- [ ] Optimize `begin` tail position (last expr)
- [ ] Optimize `let` tail position (body)
- [ ] Optimize `lambda` tail position (body)
- [ ] Add tests for each special form

### Phase 3: Advanced Cases (Week 3)
- [ ] Handle mutual tail recursion
- [ ] Optimize `and`/`or` tail positions
- [ ] Add comprehensive test suite
- [ ] Benchmark performance impact
- [ ] Update documentation

### Phase 4: LLVM Integration (Future)
- [ ] Verify LLVM backend supports TCO
- [ ] Add `musttail` attribute to LLVM calls
- [ ] Test compiled code maintains TCO

## Success Criteria

- ✅ Tail-recursive functions can execute millions of iterations
- ✅ Non-tail recursive functions fail gracefully with clear errors
- ✅ All special forms correctly identify tail positions
- ✅ Mutual recursion works with TCO
- ✅ No performance regression for non-recursive code
- ✅ Stack traces still useful for debugging
- ✅ Tests cover boundary conditions
- ✅ Documentation explains TCO clearly

## Performance Considerations

**Expected impact:**
- Tail calls: ~5-10% faster (no stack allocation)
- Non-tail calls: ~2-5% slower (depth tracking)
- Overall: neutral to slightly positive

**Benchmarks to run:**
- Tail recursive vs iterative
- Deep recursion vs shallow recursion
- Mutual recursion overhead

## Future Enhancements

1. **Trampoline for non-TCO calls** - allow deeper non-tail recursion
2. **Stack trace preservation** - maintain debugging info despite TCO
3. **TCO statistics** - report which calls were optimized
4. **Warning system** - detect non-tail recursion that could be tail
5. **REPL commands** - `:tco-info` to show optimization status

## References

- [Scheme R5RS - Proper Tail Recursion](https://www.scheme.com/tspl4/further.html#./further:h6)
- [SICP - Recursion and Iteration](https://mitpress.mit.edu/sites/default/files/sicp/full-text/book/book-Z-H-11.html)
- [Proper Tail Calls in JavaScript](https://webkit.org/blog/6240/ecmascript-6-proper-tail-calls-in-webkit/)
- [LLVM Tail Call Optimization](https://llvm.org/docs/CodeGenerator.html#tail-call-optimization)