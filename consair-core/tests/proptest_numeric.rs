use consair::NumericType;
use proptest::prelude::*;

// ============================================================================
// Strategies for Generating Numeric Values
// ============================================================================

/// Strategy for generating integers that won't overflow easily
fn small_i64() -> impl Strategy<Value = i64> {
    -1_000_000i64..1_000_000i64
}

/// Strategy for generating integers that might cause overflow
fn large_i64() -> impl Strategy<Value = i64> {
    prop_oneof![
        Just(i64::MAX),
        Just(i64::MIN),
        Just(i64::MAX - 1),
        Just(i64::MIN + 1),
        i64::MAX / 2..i64::MAX,
        i64::MIN..i64::MIN / 2,
    ]
}

/// Strategy for non-zero integers (for division)
fn non_zero_i64() -> impl Strategy<Value = i64> {
    small_i64().prop_filter("Must be non-zero", |x| *x != 0)
}

/// Strategy for generating valid ratios
fn small_ratio() -> impl Strategy<Value = NumericType> {
    (small_i64(), non_zero_i64())
        .prop_map(|(num, denom)| NumericType::make_ratio(num, denom).unwrap())
}

/// Strategy for generating NumericType values
fn numeric_value() -> impl Strategy<Value = NumericType> {
    prop_oneof![
        small_i64().prop_map(NumericType::Int),
        small_ratio(),
        any::<f64>()
            .prop_filter("Must be finite", |f| f.is_finite())
            .prop_map(NumericType::Float),
    ]
}

// ============================================================================
// Arithmetic Property Tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    // ========================================================================
    // Addition Properties
    // ========================================================================

    #[test]
    fn add_commutative(a in small_i64(), b in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        let ab = num_a.add(&num_b).ok();
        let ba = num_b.add(&num_a).ok();

        // a + b = b + a
        prop_assert_eq!(ab, ba);
    }

    #[test]
    fn add_associative(a in small_i64(), b in small_i64(), c in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);
        let num_c = NumericType::Int(c);

        // (a + b) + c
        let ab = num_a.add(&num_b).ok();
        let abc = ab.and_then(|x| x.add(&num_c).ok());

        // a + (b + c)
        let bc = num_b.add(&num_c).ok();
        let abc2 = bc.and_then(|x| num_a.add(&x).ok());

        prop_assert_eq!(abc, abc2);
    }

    #[test]
    fn add_identity(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let zero = NumericType::Int(0);

        // a + 0 = a
        let result = num_a.add(&zero).unwrap();
        prop_assert_eq!(result, num_a);
    }

    #[test]
    fn add_overflow_promotes(a in large_i64(), b in large_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        // Addition should never fail, even on overflow
        let result = num_a.add(&num_b);
        prop_assert!(result.is_ok());

        // If it would overflow i64, should be BigInt
        if a.checked_add(b).is_none() {
            prop_assert!(matches!(result.unwrap(), NumericType::BigInt(_)));
        }
    }

    // ========================================================================
    // Subtraction Properties
    // ========================================================================

    #[test]
    fn sub_inverse_of_add(a in small_i64(), b in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        // (a + b) - b = a
        if let Ok(ab) = num_a.add(&num_b) {
            if let Ok(result) = ab.sub(&num_b) {
                prop_assert_eq!(result, num_a);
            }
        }
    }

    #[test]
    fn sub_identity(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let zero = NumericType::Int(0);

        // a - 0 = a
        let result = num_a.sub(&zero).unwrap();
        prop_assert_eq!(result, num_a);
    }

    #[test]
    fn sub_self_is_zero(a in small_i64()) {
        let num_a = NumericType::Int(a);

        // a - a = 0
        let result = num_a.sub(&num_a).unwrap();
        prop_assert_eq!(result, NumericType::Int(0));
    }

    // ========================================================================
    // Multiplication Properties
    // ========================================================================

    #[test]
    fn mul_commutative(a in small_i64(), b in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        let ab = num_a.mul(&num_b).ok();
        let ba = num_b.mul(&num_a).ok();

        // a * b = b * a
        prop_assert_eq!(ab, ba);
    }

    #[test]
    fn mul_associative(a in small_i64(), b in small_i64(), c in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);
        let num_c = NumericType::Int(c);

        // (a * b) * c
        let ab = num_a.mul(&num_b).ok();
        let abc = ab.and_then(|x| x.mul(&num_c).ok());

        // a * (b * c)
        let bc = num_b.mul(&num_c).ok();
        let abc2 = bc.and_then(|x| num_a.mul(&x).ok());

        prop_assert_eq!(abc, abc2);
    }

    #[test]
    fn mul_identity(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let one = NumericType::Int(1);

        // a * 1 = a
        let result = num_a.mul(&one).unwrap();
        prop_assert_eq!(result, num_a);
    }

    #[test]
    fn mul_zero(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let zero = NumericType::Int(0);

        // a * 0 = 0
        let result = num_a.mul(&zero).unwrap();
        prop_assert_eq!(result, NumericType::Int(0));
    }

    #[test]
    fn mul_distributive(a in small_i64(), b in small_i64(), c in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);
        let num_c = NumericType::Int(c);

        // a * (b + c)
        let bc = num_b.add(&num_c).ok();
        let a_bc = bc.and_then(|x| num_a.mul(&x).ok());

        // a * b + a * c
        let ab = num_a.mul(&num_b).ok();
        let ac = num_a.mul(&num_c).ok();
        let ab_ac = ab.and_then(|x| ac.and_then(|y| x.add(&y).ok()));

        // Should be equal (within numerical precision for floats)
        prop_assert_eq!(a_bc, ab_ac);
    }

    // ========================================================================
    // Division Properties
    // ========================================================================

    #[test]
    fn div_by_zero_fails(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let zero = NumericType::Int(0);

        // a / 0 should error
        let result = num_a.div(&zero);
        prop_assert!(result.is_err());
    }

    #[test]
    fn div_identity(a in small_i64()) {
        let num_a = NumericType::Int(a);
        let one = NumericType::Int(1);

        // a / 1 = a
        let result = num_a.div(&one).unwrap();
        prop_assert_eq!(result, num_a);
    }

    #[test]
    fn div_self_is_one(a in non_zero_i64()) {
        let num_a = NumericType::Int(a);

        // a / a = 1
        let result = num_a.div(&num_a).unwrap();
        prop_assert_eq!(result, NumericType::Int(1));
    }

    #[test]
    fn div_mul_inverse(a in small_i64(), b in non_zero_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        // (a / b) * b should equal a (for exact division)
        // or at least be very close for ratios
        if let Ok(ab) = num_a.div(&num_b) {
            if let Ok(result) = ab.mul(&num_b) {
                // For integers and ratios, this should be exact
                // We can't use exact equality due to potential type promotion
                // but we can check the numeric value is the same
                let orig_float = num_a.to_float();
                let result_float = result.to_float();

                // Allow small floating point error
                prop_assert!((orig_float - result_float).abs() < 1e-10);
            }
        }
    }

    #[test]
    fn div_exact_when_evenly_divisible(a in small_i64(), b in non_zero_i64()) {
        let a_times_b = a.saturating_mul(b);
        let num_ab = NumericType::Int(a_times_b);
        let num_b = NumericType::Int(b);

        // (a * b) / b = a (exactly)
        if let Ok(result) = num_ab.div(&num_b) {
            // Should be Int(a) or equivalent
            match result {
                NumericType::Int(n) => prop_assert_eq!(n, a),
                NumericType::Ratio(n, d) => {
                    // Should reduce to a
                    prop_assert_eq!(n / d, a);
                }
                _ => {} // Other types possible due to overflow
            }
        }
    }

    // ========================================================================
    // Ratio Properties
    // ========================================================================

    #[test]
    fn ratio_always_normalized(num in small_i64(), denom in non_zero_i64()) {
        if let Ok(ratio) = NumericType::make_ratio(num, denom) {
            match ratio {
                NumericType::Ratio(n, d) => {
                    // Denominator should always be positive
                    prop_assert!(d > 0);

                    // Should be in reduced form (gcd = 1)
                    fn gcd(mut a: i64, mut b: i64) -> i64 {
                        a = a.abs();
                        b = b.abs();
                        while b != 0 {
                            let temp = b;
                            b = a % b;
                            a = temp;
                        }
                        a
                    }

                    let g = gcd(n, d);
                    prop_assert_eq!(g, 1);
                }
                NumericType::Int(_) => {
                    // If denominator was 1 (or -1), should reduce to Int
                    prop_assert_eq!(denom.abs(), 1);
                }
                _ => prop_assert!(false, "Unexpected type"),
            }
        }
    }

    #[test]
    fn ratio_addition_correct(
        a_num in small_i64(), a_den in non_zero_i64(),
        b_num in small_i64(), b_den in non_zero_i64()
    ) {
        if let (Ok(a), Ok(b)) = (
            NumericType::make_ratio(a_num, a_den),
            NumericType::make_ratio(b_num, b_den),
        ) {
            if let Ok(result) = a.add(&b) {
                // Check using floating point (allowing for rounding)
                let expected = (a_num as f64 / a_den as f64) + (b_num as f64 / b_den as f64);
                let actual = result.to_float();

                // Allow for floating point imprecision
                let diff = (expected - actual).abs();
                let tolerance = 1e-9 * expected.abs().max(1.0);
                prop_assert!(diff < tolerance, "Expected {}, got {}", expected, actual);
            }
        }
    }

    // ========================================================================
    // Negation Properties
    // ========================================================================

    #[test]
    fn neg_neg_is_identity(a in small_i64()) {
        let num_a = NumericType::Int(a);

        // -(-a) = a
        let neg_a = num_a.neg().unwrap();
        let neg_neg_a = neg_a.neg().unwrap();
        prop_assert_eq!(neg_neg_a, num_a);
    }

    #[test]
    fn neg_distributes_over_add(a in small_i64(), b in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        // -(a + b) = -a + -b
        if let Ok(ab) = num_a.add(&num_b) {
            let neg_ab = ab.neg().unwrap();

            let neg_a = num_a.neg().unwrap();
            let neg_b = num_b.neg().unwrap();
            if let Ok(neg_a_plus_neg_b) = neg_a.add(&neg_b) {
                prop_assert_eq!(neg_ab, neg_a_plus_neg_b);
            }
        }
    }

    // ========================================================================
    // Comparison Properties
    // ========================================================================

    #[test]
    fn comparison_reflexive(a in small_i64()) {
        let num_a = NumericType::Int(a);

        // a == a
        prop_assert_eq!(num_a.clone(), num_a.clone());

        // a <= a and a >= a
        prop_assert!(num_a.clone() <= num_a.clone());
        prop_assert!(num_a.clone() >= num_a);
    }

    #[test]
    fn comparison_antisymmetric(a in small_i64(), b in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);

        // if a <= b and b <= a, then a == b
        if num_a <= num_b && num_b <= num_a {
            prop_assert_eq!(num_a, num_b);
        }
    }

    #[test]
    fn comparison_transitive(a in small_i64(), b in small_i64(), c in small_i64()) {
        let num_a = NumericType::Int(a);
        let num_b = NumericType::Int(b);
        let num_c = NumericType::Int(c);

        // if a <= b and b <= c, then a <= c
        if num_a <= num_b && num_b <= num_c {
            prop_assert!(num_a <= num_c);
        }
    }

    #[test]
    fn cross_type_equality_consistent(a in small_i64(), b in non_zero_i64()) {
        let int_val = NumericType::Int(a);

        // a == a/1
        if let Ok(ratio_val) = NumericType::make_ratio(a, 1) {
            prop_assert_eq!(int_val.clone(), ratio_val);
        }

        // a == (a*b)/b
        if let Ok(ratio_val) = NumericType::make_ratio(a.saturating_mul(b), b) {
            match &ratio_val {
                NumericType::Int(n) => prop_assert_eq!(*n, a),
                NumericType::Ratio(_, _) => {
                    // Floating point comparison
                    let diff = (int_val.to_float() - ratio_val.to_float()).abs();
                    prop_assert!(diff < 1e-10);
                }
                _ => {}
            }
        }
    }

    // ========================================================================
    // Type Stability Properties
    // ========================================================================

    #[test]
    fn operations_never_panic(a in numeric_value(), b in numeric_value()) {
        // All operations should return Result, never panic
        let _ = a.add(&b);
        let _ = a.sub(&b);
        let _ = a.mul(&b);
        let _ = a.div(&b); // May return Err for division by zero
        let _ = a.neg();
        let _ = a.to_float();
        let _ = a.is_zero();

        // Comparisons should never panic
        let _ = a == b;
        let _ = a.partial_cmp(&b);
    }

    #[test]
    fn to_float_preserves_sign(a in small_i64()) {
        let num = NumericType::Int(a);
        let f = num.to_float();

        if a > 0 {
            prop_assert!(f > 0.0);
        } else if a < 0 {
            prop_assert!(f < 0.0);
        } else {
            prop_assert_eq!(f, 0.0);
        }
    }

    #[test]
    fn is_zero_consistent_with_equality(a in numeric_value()) {
        let zero = NumericType::Int(0);

        if a.is_zero() {
            prop_assert_eq!(a.clone(), zero.clone());
        }

        if a.clone() == zero {
            prop_assert!(a.is_zero());
        }
    }
}
