use consair::{AtomType, Environment, NumericType, Value, eval, language, parse};

// ============================================================================
// Numeric Type Tests
// ============================================================================

#[test]
fn test_int_arithmetic() {
    let a = NumericType::Int(10);
    let b = NumericType::Int(5);

    assert_eq!(a.add(&b).unwrap(), NumericType::Int(15));
    assert_eq!(a.sub(&b).unwrap(), NumericType::Int(5));
    assert_eq!(a.mul(&b).unwrap(), NumericType::Int(50));
    assert_eq!(a.div(&b).unwrap(), NumericType::Int(2));
}

#[test]
fn test_int_overflow_promotion() {
    let max_int = NumericType::Int(i64::MAX);
    let one = NumericType::Int(1);

    // Should promote to BigInt on overflow
    let result = max_int.add(&one).unwrap();
    match result {
        NumericType::BigInt(_) => {} // Success
        _ => panic!("Expected BigInt promotion on overflow, got {result:?}"),
    }
}

#[test]
fn test_int_underflow_promotion() {
    let min_int = NumericType::Int(i64::MIN);
    let one = NumericType::Int(1);

    // Should promote to BigInt on underflow
    let result = min_int.sub(&one).unwrap();
    match result {
        NumericType::BigInt(_) => {} // Success
        _ => panic!("Expected BigInt promotion on underflow, got {result:?}"),
    }
}

#[test]
fn test_exact_division() {
    let five = NumericType::Int(5);
    let two = NumericType::Int(2);

    // 5/2 should return Ratio(5, 2), not truncate to 2
    let result = five.div(&two).unwrap();
    assert_eq!(result, NumericType::Ratio(5, 2));

    // 10/5 should return Int(2) since it divides evenly
    let ten = NumericType::Int(10);
    let result2 = ten.div(&two).unwrap();
    assert_eq!(result2, NumericType::Int(5));
}

#[test]
fn test_ratio_reduction() {
    // 6/9 should automatically reduce to 2/3
    let ratio = NumericType::make_ratio(6, 9).unwrap();
    assert_eq!(ratio, NumericType::Ratio(2, 3));

    // 10/5 should reduce to Int(2)
    let ratio2 = NumericType::make_ratio(10, 5).unwrap();
    assert_eq!(ratio2, NumericType::Int(2));

    // Negative ratios should normalize (denominator always positive)
    let ratio3 = NumericType::make_ratio(-6, -9).unwrap();
    assert_eq!(ratio3, NumericType::Ratio(2, 3));

    let ratio4 = NumericType::make_ratio(6, -9).unwrap();
    assert_eq!(ratio4, NumericType::Ratio(-2, 3));
}

#[test]
fn test_ratio_arithmetic() {
    let one_half = NumericType::Ratio(1, 2);
    let one_third = NumericType::Ratio(1, 3);

    // 1/2 + 1/3 = 3/6 + 2/6 = 5/6
    assert_eq!(one_half.add(&one_third).unwrap(), NumericType::Ratio(5, 6));

    // 1/2 - 1/3 = 3/6 - 2/6 = 1/6
    assert_eq!(one_half.sub(&one_third).unwrap(), NumericType::Ratio(1, 6));

    // 1/2 * 1/3 = 1/6
    assert_eq!(one_half.mul(&one_third).unwrap(), NumericType::Ratio(1, 6));

    // 1/2 / 1/3 = 1/2 * 3/1 = 3/2
    assert_eq!(one_half.div(&one_third).unwrap(), NumericType::Ratio(3, 2));
}

#[test]
fn test_mixed_int_ratio_arithmetic() {
    let two = NumericType::Int(2);
    let one_half = NumericType::Ratio(1, 2);

    // 2 + 1/2 = 5/2
    assert_eq!(two.add(&one_half).unwrap(), NumericType::Ratio(5, 2));

    // 2 - 1/2 = 3/2
    assert_eq!(two.sub(&one_half).unwrap(), NumericType::Ratio(3, 2));

    // 2 * 1/2 = 1
    assert_eq!(two.mul(&one_half).unwrap(), NumericType::Int(1));

    // 2 / 1/2 = 4
    assert_eq!(two.div(&one_half).unwrap(), NumericType::Int(4));
}

#[test]
fn test_float_arithmetic() {
    let a = NumericType::Float(3.5);
    let b = NumericType::Float(1.5);

    match a.add(&b).unwrap() {
        NumericType::Float(result) => assert!((result - 5.0).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }

    match a.sub(&b).unwrap() {
        NumericType::Float(result) => assert!((result - 2.0).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }

    match a.mul(&b).unwrap() {
        NumericType::Float(result) => assert!((result - 5.25).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }

    match a.div(&b).unwrap() {
        NumericType::Float(result) => assert!((result - 7.0 / 3.0).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }
}

#[test]
fn test_mixed_float_int_arithmetic() {
    let float_val = NumericType::Float(3.5);
    let int_val = NumericType::Int(2);

    match float_val.add(&int_val).unwrap() {
        NumericType::Float(result) => assert!((result - 5.5).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }
}

#[test]
fn test_division_by_zero() {
    let five = NumericType::Int(5);
    let zero = NumericType::Int(0);

    assert!(five.div(&zero).is_err());

    let ratio = NumericType::Ratio(5, 2);
    assert!(ratio.div(&zero).is_err());
}

#[test]
fn test_cross_type_comparison() {
    // Int(5) == Ratio(10, 2)
    let int_five = NumericType::Int(5);
    let ratio_five = NumericType::Ratio(10, 2);
    assert_eq!(int_five, ratio_five);

    // Int(5) == Float(5.0)
    let float_five = NumericType::Float(5.0);
    assert_eq!(int_five, float_five);

    // Ratio(5, 2) == Float(2.5)
    let ratio = NumericType::Ratio(5, 2);
    let float = NumericType::Float(2.5);
    assert_eq!(ratio, float);
}

#[test]
fn test_numeric_ordering() {
    let one = NumericType::Int(1);
    let two = NumericType::Int(2);
    let one_half = NumericType::Ratio(1, 2);
    let three_halves = NumericType::Ratio(3, 2);

    assert!(one < two);
    assert!(one_half < one);
    assert!(one < three_halves);
    assert!(three_halves < two);
}

#[test]
fn test_negation() {
    let pos = NumericType::Int(5);
    assert_eq!(pos.neg().unwrap(), NumericType::Int(-5));

    let ratio = NumericType::Ratio(3, 4);
    assert_eq!(ratio.neg().unwrap(), NumericType::Ratio(-3, 4));

    let float_val = NumericType::Float(3.15);
    match float_val.neg().unwrap() {
        NumericType::Float(result) => assert!((result + 3.15).abs() < 1e-10),
        _ => panic!("Expected Float result"),
    }
}

#[test]
fn test_bigint_arithmetic() {
    use num_bigint::BigInt;
    use std::rc::Rc;

    let big1 = NumericType::BigInt(Rc::new(BigInt::from(1000000000000i64)));
    let big2 = NumericType::BigInt(Rc::new(BigInt::from(2000000000000i64)));

    match big1.add(&big2).unwrap() {
        NumericType::BigInt(result) => {
            assert_eq!(*result, BigInt::from(3000000000000i64));
        }
        _ => panic!("Expected BigInt result"),
    }
}

// ============================================================================
// Parser Tests
// ============================================================================

#[test]
fn test_parse_integer() {
    let result = parse("42").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 42),
        _ => panic!("Expected Int(42), got {result:?}"),
    }
}

#[test]
fn test_parse_negative_integer() {
    let result = parse("-42").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, -42),
        _ => panic!("Expected Int(-42), got {result:?}"),
    }
}

#[test]
fn test_parse_float() {
    let result = parse("3.15").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Float(f))) => {
            assert!((f - 3.15).abs() < 1e-10);
        }
        _ => panic!("Expected Float(3.15), got {result:?}"),
    }
}

#[test]
fn test_parse_scientific_notation() {
    let result = parse("1.5e10").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Float(f))) => {
            assert!((f - 1.5e10).abs() < 1e-10);
        }
        _ => panic!("Expected Float(1.5e10), got {result:?}"),
    }

    let result2 = parse("2e-5").unwrap();
    match result2 {
        Value::Atom(AtomType::Number(NumericType::Float(f))) => {
            assert!((f - 2e-5).abs() < 1e-15);
        }
        _ => panic!("Expected Float(2e-5), got {result2:?}"),
    }
}

#[test]
fn test_parse_ratio() {
    let result = parse("5/2").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Ratio(num, denom))) => {
            assert_eq!(num, 5);
            assert_eq!(denom, 2);
        }
        _ => panic!("Expected Ratio(5, 2), got {result:?}"),
    }
}

#[test]
fn test_parse_ratio_auto_reduction() {
    let result = parse("6/9").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Ratio(num, denom))) => {
            // Should auto-reduce to 2/3
            assert_eq!(num, 2);
            assert_eq!(denom, 3);
        }
        _ => panic!("Expected Ratio(2, 3), got {result:?}"),
    }
}

#[test]
fn test_parse_ratio_reduces_to_int() {
    let result = parse("10/5").unwrap();
    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => {
            assert_eq!(n, 2);
        }
        _ => panic!("Expected Int(2), got {result:?}"),
    }
}

// ============================================================================
// Interpreter Integration Tests
// ============================================================================

#[test]
fn test_eval_addition() {
    let mut env = Environment::new();
    let expr = parse("(+ 5 3)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 8),
        _ => panic!("Expected Int(8), got {result:?}"),
    }
}

#[test]
fn test_eval_subtraction() {
    let mut env = Environment::new();
    let expr = parse("(- 10 3)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 7),
        _ => panic!("Expected Int(7), got {result:?}"),
    }
}

#[test]
fn test_eval_multiplication() {
    let mut env = Environment::new();
    let expr = parse("(* 6 7)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 42),
        _ => panic!("Expected Int(42), got {result:?}"),
    }
}

#[test]
fn test_eval_division_exact() {
    let mut env = Environment::new();
    let expr = parse("(/ 5 2)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    // Should return exact ratio, not truncated integer
    match result {
        Value::Atom(AtomType::Number(NumericType::Ratio(num, denom))) => {
            assert_eq!(num, 5);
            assert_eq!(denom, 2);
        }
        _ => panic!("Expected Ratio(5, 2), got {result:?}"),
    }
}

#[test]
fn test_eval_division_evenly() {
    let mut env = Environment::new();
    let expr = parse("(/ 10 5)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 2),
        _ => panic!("Expected Int(2), got {result:?}"),
    }
}

#[test]
fn test_eval_ratio_arithmetic() {
    let mut env = Environment::new();

    // 1/2 + 1/3 = 5/6
    let expr = parse("(+ 1/2 1/3)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Ratio(num, denom))) => {
            assert_eq!(num, 5);
            assert_eq!(denom, 6);
        }
        _ => panic!("Expected Ratio(5, 6), got {result:?}"),
    }
}

#[test]
fn test_eval_float_arithmetic() {
    let mut env = Environment::new();
    let expr = parse("(+ 3.5 2.5)").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Float(f))) => {
            assert!((f - 6.0).abs() < 1e-10);
        }
        _ => panic!("Expected Float(6.0), got {result:?}"),
    }
}

#[test]
fn test_eval_comparison_less_than() {
    let mut env = Environment::new();

    let expr = parse("(< 5 10)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Bool(true)));

    let expr2 = parse("(< 10 5)").unwrap();
    let result2 = eval(expr2, &mut env).unwrap();
    assert_eq!(result2, Value::Atom(AtomType::Bool(false)));
}

#[test]
fn test_eval_comparison_greater_than() {
    let mut env = Environment::new();

    let expr = parse("(> 10 5)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Bool(true)));

    let expr2 = parse("(> 5 10)").unwrap();
    let result2 = eval(expr2, &mut env).unwrap();
    assert_eq!(result2, Value::Atom(AtomType::Bool(false)));
}

#[test]
fn test_eval_comparison_equals() {
    let mut env = Environment::new();

    // Test int equality
    let expr = parse("(= 5 5)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Bool(true)));

    // Test cross-type equality: 5 == 10/2
    let expr2 = parse("(= 5 10/2)").unwrap();
    let result2 = eval(expr2, &mut env).unwrap();
    assert_eq!(result2, Value::Atom(AtomType::Bool(true)));
}

#[test]
fn test_eval_nested_arithmetic() {
    let mut env = Environment::new();

    // (+ (* 2 3) (/ 10 5))
    // = (+ 6 2)
    // = 8
    let expr = parse("(+ (* 2 3) (/ 10 5))").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 8),
        _ => panic!("Expected Int(8), got {result:?}"),
    }
}

#[test]
fn test_eval_overflow_in_expression() {
    let mut env = Environment::new();

    // This should cause overflow and promote to BigInt
    let expr_str = format!("(+ {} 1)", i64::MAX);
    let expr = parse(&expr_str).unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::BigInt(_))) => {} // Success
        _ => panic!("Expected BigInt promotion, got {result:?}"),
    }
}

#[test]
fn test_ratio_in_conditional() {
    let mut env = Environment::new();

    // (cond ((< 1/2 1) 'yes) (t 'no))
    let expr = parse("(cond ((< 1/2 1) 'yes) (t 'no))").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Symbol(language::SymbolType::Symbol(s))) => assert_eq!(s, "yes"),
        _ => panic!("Expected symbol 'yes', got {result:?}"),
    }
}

#[test]
fn test_numeric_precision_preservation() {
    let mut env = Environment::new();

    // Compute 1/3 + 1/3 + 1/3 - should equal 1 exactly (not 0.99999...)
    let expr = parse("(+ 1/3 (+ 1/3 1/3))").unwrap();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => assert_eq!(n, 1),
        _ => panic!("Expected exact Int(1), got {result:?}"),
    }
}

// ============================================================================
// Edge Cases and Error Handling
// ============================================================================

#[test]
fn test_error_division_by_zero() {
    let mut env = Environment::new();
    let expr = parse("(/ 5 0)").unwrap();
    let result = eval(expr, &mut env);

    assert!(result.is_err());
}

#[test]
fn test_error_arithmetic_on_non_number() {
    let mut env = Environment::new();
    let expr = parse("(+ 5 'symbol)").unwrap();
    let result = eval(expr, &mut env);

    assert!(result.is_err());
}

#[test]
fn test_zero_denominator_in_ratio() {
    let result = NumericType::make_ratio(5, 0);
    assert!(result.is_err());
}
