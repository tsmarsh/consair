# Add Recursive Function Support

## Problem

The `label` special form cannot define recursive functions because it evaluates the lambda before binding the name to the environment. This means the function name is not available within its own body, making self-reference impossible.

**Current behavior:**
```lisp
(label factorial (lambda (n)
  (cond
    ((= n 0) 1)
    (t (* n (factorial (- n 1)))))))
(factorial 5)
; Error: Unbound symbol: factorial
```

**Root cause** (in `consair-core/src/interpreter.rs:206-218`):
```rust
"label" => {
    // Lambda is evaluated BEFORE the name is defined
    let fn_val = eval_loop(fn_expr, &mut current_env, depth + 1)?;
    // Name is only added AFTER lambda captures its environment
    env.define(name.resolve(), fn_val.clone());
    return Ok(fn_val);
}
```

Since lambdas capture their environment at creation time (line 203: `env: current_env.clone()`), the function name isn't in the captured environment yet.

## Impact

**High Priority** - This is a fundamental limitation that prevents:

- ✗ **Any recursive algorithm**: factorial, fibonacci, tree traversal, etc.
- ✗ **Mutually recursive functions**: is-even/is-odd pairs
- ✗ **Recursive data structure operations**: list-length, tree-depth, etc.
- ✗ **Comprehensive benchmarks**: As noted in `benches/benchmarks.rs:109`:
  ```rust
  // Note: Recursive benchmarks are skipped due to label/environment scoping complexity
  ```

This limitation significantly restricts what programs can be written in Consair and prevents realistic performance benchmarking.

## Proposed Solutions

### Option 1: Fix `label` to Support Self-Reference (Recommended)

Modify `label` to define the name in the environment before evaluating the lambda, similar to Scheme's `letrec`:

```rust
"label" => {
    let name_expr = car(&cell.cdr)?;
    let fn_expr = car(&cdr(&cell.cdr)?)?;

    if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = name_expr {
        // Strategy: Create a placeholder, evaluate lambda, then update
        // OR: Add name to env before eval (requires lazy/mutable binding)

        // Implementation approach 1: Two-pass
        // 1. Add uninitialized binding
        // 2. Eval lambda (can now see binding)
        // 3. Update binding with actual value

        // Implementation approach 2: Modify lambda environment after creation
        // This requires lambdas to have mutable environments

        let fn_val = eval_loop(fn_expr, &mut current_env, depth + 1)?;

        // If using approach 1, we need a way to handle the uninitialized case
        // If using approach 2, we need to mutate the lambda's captured env

        env.define(name.resolve(), fn_val.clone());
        return Ok(fn_val);
    }
}
```

### Option 2: Add Explicit `letrec` or `rec` Special Form

Create a new special form specifically for recursive bindings:

```lisp
; New special form: letrec
(letrec ((factorial (lambda (n)
                      (cond ((= n 0) 1)
                            (t (* n (factorial (- n 1))))))))
  (factorial 5))
```

This makes the recursive intent explicit and allows `label` to remain simple.

### Option 3: Y-Combinator (No Changes Required)

Users can already use the Y-combinator for recursion (no interpreter changes needed):

```lisp
(label Y (lambda (f)
  ((lambda (x) (f (lambda (y) ((x x) y))))
   (lambda (x) (f (lambda (y) ((x x) y)))))))

(label factorial-gen (lambda (rec)
  (lambda (n)
    (cond ((= n 0) 1)
          (t (* n (rec (- n 1))))))))

(label factorial (Y factorial-gen))
(factorial 5)  ; => 120
```

**Pros**: Works today, no code changes
**Cons**: Complex, non-intuitive, poor error messages, performance overhead

## Recommended Implementation: Option 1

Modify `label` to support self-reference using a two-phase binding approach:

1. **Add a `Deferred` or `Uninitialized` value type** to represent "being defined"
2. **Bind the name before evaluating** the lambda expression
3. **Update the binding** once the lambda is created
4. **Handle deferred lookups** by either:
   - Erroring if accessed during its own definition (simple)
   - Allowing it (enables more complex patterns)

### Implementation Steps

1. **Add uninitialized value type** to `language.rs`:
   ```rust
   pub enum Value {
       Atom(AtomType),
       Cons(Arc<ConsCell>),
       Lambda(Arc<LambdaCell>),
       Vector(Arc<VectorCell>),
       NativeFn(NativeFn),
       Uninitialized(String), // Name of value being defined
       Nil,
   }
   ```

2. **Modify `label` in interpreter.rs**:
   ```rust
   "label" => {
       let name_expr = car(&cell.cdr)?;
       let fn_expr = car(&cdr(&cell.cdr)?)?;

       if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = name_expr {
           let name_str = name.resolve();

           // Phase 1: Add placeholder binding
           current_env.define(name_str.clone(), Value::Uninitialized(name_str.clone()));

           // Phase 2: Evaluate lambda (can now see the name in scope)
           let fn_val = eval_loop(fn_expr, &mut current_env, depth + 1)?;

           // Phase 3: Replace placeholder with actual value
           current_env.define(name_str.clone(), fn_val.clone());

           return Ok(fn_val);
       }
   }
   ```

3. **Handle `Uninitialized` in lookups**:
   ```rust
   Value::Atom(AtomType::Symbol(ref sym)) => {
       return match sym {
           SymbolType::Symbol(name) => name.with_str(|s| {
               match current_env.lookup(s) {
                   Some(Value::Uninitialized(n)) => {
                       Err(format!("Cannot use '{}' within its own definition", n))
                   }
                   Some(val) => Ok(val),
                   None => Err(format!("Unbound symbol: {}", name)),
               }
           }),
           SymbolType::Keyword { .. } => Ok(expr),
       };
   }
   ```

4. **Add tests** for recursive functions:
   ```rust
   #[test]
   fn test_recursive_factorial() {
       let result = run_lisp_file(r#"
   (label factorial (lambda (n)
     (cond ((= n 0) 1)
           (t (* n (factorial (- n 1)))))))
   (factorial 5)
   "#);
       assert_eq!(result.unwrap(), "120");
   }

   #[test]
   fn test_mutually_recursive() {
       let result = run_lisp_file(r#"
   (label is-even (lambda (n)
     (cond ((= n 0) t)
           (t (is-odd (- n 1))))))
   (label is-odd (lambda (n)
     (cond ((= n 0) nil)
           (t (is-even (- n 1))))))
   (is-even 4)
   "#);
       assert_eq!(result.unwrap(), "t");
   }

   #[test]
   fn test_list_length() {
       let result = run_lisp_file(r#"
   (label length (lambda (lst)
     (cond ((atom lst) 0)
           (t (+ 1 (length (cdr lst)))))))
   (length '(1 2 3 4 5))
   "#);
       assert_eq!(result.unwrap(), "5");
   }
   ```

5. **Add recursive benchmarks** (from proposal 09):
   ```rust
   fn bench_eval_factorial(c: &mut Criterion) {
       let mut env = Environment::new();
       register_stdlib(&mut env);

       // Define factorial
       let setup = parse(r#"
   (label factorial (lambda (n)
     (cond ((= n 0) 1)
           (t (* n (factorial (- n 1)))))))
   "#).unwrap();
       eval(setup, &mut env).unwrap();

       c.bench_function("eval recursive factorial(10)", |b| {
           b.iter(|| {
               let expr = parse("(factorial 10)").unwrap();
               black_box(eval(expr, &mut env.clone()).unwrap())
           })
       });
   }
   ```

6. **Update documentation**:
   - Add recursion examples to README.md
   - Document the self-reference limitation and solution
   - Add to language guide with examples

## Alternative: Environment Mutation Approach

Instead of `Uninitialized`, allow modifying lambda environments after creation:

1. Make `LambdaCell.env` mutable via `RefCell` or similar
2. After creating lambda, update its environment to include itself
3. More complex but avoids the `Uninitialized` type

## Success Criteria

- [x] `label` can define recursive functions
- [x] Factorial, fibonacci, and list-length work correctly
- [x] Mutually recursive functions work (is-even/is-odd)
- [x] No performance regression for non-recursive code
- [x] Tests for edge cases (deep recursion, mutual recursion)
- [x] Recursive benchmarks added and working
- [x] Documentation updated with recursion examples
- [x] Error messages clear when recursion depth exceeded

## Testing Plan

1. **Unit tests**: Basic recursive functions
2. **Property tests**: Recursive vs iterative equivalence
3. **Performance tests**: Deep recursion (100+ levels)
4. **Edge cases**:
   - Empty list recursion
   - Mutual recursion cycles
   - Self-reference in non-function contexts
5. **Regression tests**: Ensure non-recursive code still works

## Notes

- This is blocking comprehensive benchmarking (proposal 09)
- Required for realistic Lisp programs
- Once fixed, many classic Lisp examples become possible
- Consider if this affects garbage collection (recursive structures)
