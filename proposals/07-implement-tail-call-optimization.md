# Implement Tail Call Optimization

## Problem

The interpreter currently has a documented limitation: "No tail call optimization: Deep recursion will overflow the stack." This prevents writing idiomatic recursive functional code.

**Location:** `consair-core/src/interpreter.rs`

## Impact

- Recursive functions limited by stack depth
- Cannot write idiomatic tail-recursive code
- Stack overflow crashes on deep recursion
- Major limitation for a Lisp

## Prompt for Implementation

```
Implement tail call optimization (TCO) to enable unbounded recursion in tail position:

1. Current interpreter evaluates recursively, limited by Rust stack
2. Tail recursive calls should be optimized to iteration

Please implement TCO using one of these approaches:

**Option A: Trampolining**
- Return a Thunk instead of Value from eval
- eval() becomes a loop that bounces on thunks
- Tail calls return unevaluated thunk
- Non-tail calls evaluate fully

**Option B: Explicit Loop Form**
- Add a "loop" primitive that iterates
- Transform tail recursion to loop at compile/eval time
- Simpler but requires code transformation

**Option C: CPS (Continuation Passing Style)**
- Transform to CPS at parse or eval time
- All calls become tail calls
- More complex but most general

Recommend Option A (trampolining) for balance of simplicity and generality.

Please:
- Implement the chosen approach
- Add tests for:
  * Simple tail recursion (factorial, fibonacci)
  * Mutual tail recursion
  * Deep recursion (10000+ calls)
  * Non-tail recursion still works (returns proper values)
  * Mixed tail and non-tail calls
- Add benchmarks comparing:
  * Tail recursive vs iterative performance
  * Memory usage (should be constant for tail calls)
  * Overhead on non-tail code
- Update documentation:
  * Remove "no TCO" limitation from README
  * Add examples of tail recursive code
  * Document performance characteristics

If using trampolining, consider this structure:
```rust
enum Bounce {
    Done(Value),
    Call { expr: Value, env: Environment },
}

pub fn eval_trampoline(expr: Value, env: &mut Environment) -> Result<Value, String> {
    let mut bounce = Bounce::Call { expr, env: env.clone() };
    loop {
        match bounce {
            Bounce::Done(value) => return Ok(value),
            Bounce::Call { expr, env } => {
                bounce = eval_step(expr, env)?;
            }
        }
    }
}
```

## Success Criteria

- [ ] Tail recursive functions don't overflow stack
- [ ] Deep recursion (10000+ calls) works
- [ ] All existing tests pass
- [ ] Performance is reasonable (< 2x overhead)
- [ ] Documentation updated
- [ ] Examples demonstrate TCO
