# Add Property-Based Testing

## Problem

Current tests use specific examples which may miss edge cases. Property-based testing would automatically explore the input space and find corner cases, especially in the numeric tower and parser.

## Impact

- Edge cases may be missed
- Numeric operations not validated across full range
- Parser correctness uncertain for unusual inputs

## Prompt for Implementation

```
Add property-based tests using proptest to improve test coverage and find edge cases:

1. Current tests are example-based and may miss corner cases
2. Numeric tower and parser would benefit from property-based testing

Please:
- Add proptest dependency: proptest = "1.4"
- Create property-based tests for numeric operations:

  **Numeric Properties:**
  ```rust
  proptest! {
      // Arithmetic properties
      #[test]
      fn add_commutative(a: i64, b: i64) {
          // a + b = b + a
      }

      #[test]
      fn add_associative(a: i64, b: i64, c: i64) {
          // (a + b) + c = a + (b + c)
      }

      #[test]
      fn mul_distributive(a: i64, b: i64, c: i64) {
          // a * (b + c) = a*b + a*c
      }

      #[test]
      fn div_mul_inverse(a: i64, b: i64) {
          // (a / b) * b = a (when b != 0)
      }

      #[test]
      fn ratio_normalization(n: i64, d: i64) {
          // Ratios are always in reduced form
      }

      #[test]
      fn overflow_promotes(a: i64, b: i64) {
          // Operations that overflow promote to BigInt
      }
  }
  ```

  **Parser Properties:**
  ```rust
  proptest! {
      #[test]
      fn parse_print_roundtrip(value: Value) {
          // parse(print(value)) = value (for serializable values)
      }

      #[test]
      fn parse_never_panics(s: String) {
          // Parser should never panic, always return Ok or Err
      }

      #[test]
      fn valid_numbers_parse(n: i64) {
          // n.to_string() should parse back to n
      }
  }
  ```

- Add custom strategies for:
  * Valid Lisp identifiers
  * Balanced s-expressions
  * Valid numeric strings
  * String literals with escapes

- Configure proptest:
  ```rust
  proptest! {
      #![proptest_config(ProptestConfig::with_cases(1000))]
      // More cases for thorough testing
  }
  ```

- Add to CI to catch regressions

- Document any failing properties (may reveal bugs!)

Expected to find edge cases in:
- Numeric overflow boundaries
- Ratio reduction
- Cross-type comparisons
- Unicode handling
- String escaping

## Success Criteria

- [ ] Property tests for all arithmetic operations
- [ ] Property tests for parser
- [ ] Custom strategies for Lisp values
- [ ] Tests run in CI
- [ ] Any found bugs are fixed
- [ ] 1000+ cases per property
