# Fix Numeric Overflow Bug in Ratio Arithmetic

## Problem

In `consair-core/src/numeric.rs:348`, the ratio subtraction code uses `unwrap_or(i64::MAX)` which can produce incorrect results instead of properly promoting to BigRatio when overflow occurs.

**Location:** `consair-core/src/numeric.rs:348`

**Problematic code:**
```rust
let num = match an.checked_sub(b.checked_mul(*ad).unwrap_or(i64::MAX))
```

## Impact

- Incorrect arithmetic results on overflow
- Silent data corruption instead of proper type promotion
- Inconsistent with the overflow handling in other operations (add, mul)

## Prompt for Implementation

```
Fix the numeric overflow bug in ratio arithmetic:

1. In consair-core/src/numeric.rs:348, the Ratio - Int subtraction has incorrect overflow handling
2. The code uses .unwrap_or(i64::MAX) which produces wrong results instead of promoting to BigRatio

Please:
- Fix the overflow handling to match the pattern used in addition (lines 250-271)
- Ensure that when b.checked_mul(*ad) overflows, we promote to BigRatio
- Add comprehensive tests for this specific edge case:
  * Ratio(large_num, large_denom) - Int(large_int) where multiplication overflows
  * Verify the result is mathematically correct
  * Verify we get BigRatio when appropriate
- Review all other arithmetic operations for similar issues
- Ensure consistency across add, sub, mul, div operations

The fix should follow the established pattern of detecting overflow and promoting to the next numeric type.
```

## Success Criteria

- [ ] Overflow properly promotes to BigRatio
- [ ] Arithmetic results are mathematically correct
- [ ] Tests added for the overflow edge case
- [ ] All existing numeric tests still pass
- [ ] Consistent overflow handling across all operations
