use consair::{AtomType, Environment, NumericType, Value, VectorValue, eval, parse};
use std::rc::Rc;

// ============================================================================
// Vector Parsing Tests
// ============================================================================

#[test]
fn test_empty_vector_parse() {
    let expr = parse("<< >>").unwrap();
    match expr {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 0);
        }
        _ => panic!("Expected vector, got {expr}"),
    }
}

#[test]
fn test_vector_parse_integers() {
    let expr = parse("<< 1 2 3 >>").unwrap();
    match expr {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 3);
            assert_eq!(
                vec.elements[0],
                Value::Atom(AtomType::Number(NumericType::Int(1)))
            );
            assert_eq!(
                vec.elements[1],
                Value::Atom(AtomType::Number(NumericType::Int(2)))
            );
            assert_eq!(
                vec.elements[2],
                Value::Atom(AtomType::Number(NumericType::Int(3)))
            );
        }
        _ => panic!("Expected vector, got {expr}"),
    }
}

#[test]
fn test_vector_parse_mixed_types() {
    let expr = parse("<< 1 2.5 3/4 >>").unwrap();
    match expr {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 3);
            assert_eq!(
                vec.elements[0],
                Value::Atom(AtomType::Number(NumericType::Int(1)))
            );
            assert_eq!(
                vec.elements[1],
                Value::Atom(AtomType::Number(NumericType::Float(2.5)))
            );
            assert_eq!(
                vec.elements[2],
                Value::Atom(AtomType::Number(NumericType::Ratio(3, 4)))
            );
        }
        _ => panic!("Expected vector, got {expr}"),
    }
}

#[test]
fn test_vector_parse_nested_lists() {
    let expr = parse("<< (1 2) (3 4) >>").unwrap();
    match expr {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 2);
            // Each element should be a list
            match &vec.elements[0] {
                Value::Cons(_) => {}
                _ => panic!("Expected cons cell"),
            }
        }
        _ => panic!("Expected vector, got {expr}"),
    }
}

#[test]
fn test_vector_parse_symbols() {
    let expr = parse("<< a b c >>").unwrap();
    match expr {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 3);
            assert_eq!(
                vec.elements[0],
                Value::Atom(AtomType::Symbol("a".to_string()))
            );
            assert_eq!(
                vec.elements[1],
                Value::Atom(AtomType::Symbol("b".to_string()))
            );
            assert_eq!(
                vec.elements[2],
                Value::Atom(AtomType::Symbol("c".to_string()))
            );
        }
        _ => panic!("Expected vector, got {expr}"),
    }
}

#[test]
fn test_vector_parse_error_unclosed() {
    let result = parse("<< 1 2 3");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unclosed vector"));
}

#[test]
fn test_vector_parse_error_unexpected_end() {
    let result = parse(">>");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("Unexpected >>"));
}

// ============================================================================
// Vector Display Tests
// ============================================================================

#[test]
fn test_vector_display_empty() {
    let vec = Value::Vector(Rc::new(VectorValue { elements: vec![] }));
    assert_eq!(format!("{vec}"), "<<>>");
}

#[test]
fn test_vector_display_integers() {
    let vec = Value::Vector(Rc::new(VectorValue {
        elements: vec![
            Value::Atom(AtomType::Number(NumericType::Int(1))),
            Value::Atom(AtomType::Number(NumericType::Int(2))),
            Value::Atom(AtomType::Number(NumericType::Int(3))),
        ],
    }));
    assert_eq!(format!("{vec}"), "<<1 2 3>>");
}

#[test]
fn test_vector_display_mixed() {
    let vec = Value::Vector(Rc::new(VectorValue {
        elements: vec![
            Value::Atom(AtomType::Number(NumericType::Int(1))),
            Value::Atom(AtomType::Number(NumericType::Float(2.5))),
            Value::Atom(AtomType::Symbol("x".to_string())),
        ],
    }));
    assert_eq!(format!("{vec}"), "<<1 2.5 x>>");
}

// ============================================================================
// Vector Evaluation Tests
// ============================================================================

#[test]
fn test_vector_self_evaluating() {
    let mut env = Environment::new();
    let expr = parse("<< 1 2 3 >>").unwrap();
    let result = eval(expr.clone(), &mut env).unwrap();
    assert_eq!(result, expr);
}

#[test]
fn test_vector_with_expressions() {
    let mut env = Environment::new();
    let expr = parse("<< (+ 1 2) (* 3 4) >>").unwrap();

    // The vector literal itself is self-evaluating, but if we want
    // evaluated elements, we'd need a 'vector' constructor function
    match eval(expr, &mut env).unwrap() {
        Value::Vector(vec) => {
            // Elements are unevaluated expressions in the literal
            assert_eq!(vec.elements.len(), 2);
        }
        _ => panic!("Expected vector"),
    }
}

// ============================================================================
// Vector Operation Tests
// ============================================================================

#[test]
fn test_vector_length_empty() {
    let mut env = Environment::new();
    let expr = parse("(vector-length << >>)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(0))));
}

#[test]
fn test_vector_length_nonempty() {
    let mut env = Environment::new();
    let expr = parse("(vector-length << 1 2 3 4 5 >>)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(5))));
}

#[test]
fn test_vector_length_error_not_vector() {
    let mut env = Environment::new();
    let expr = parse("(vector-length (quote (1 2 3)))").unwrap();
    let result = eval(expr, &mut env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected vector"));
}

#[test]
fn test_vector_ref_basic() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> 0)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(10))));
}

#[test]
fn test_vector_ref_middle() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> 1)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(20))));
}

#[test]
fn test_vector_ref_last() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> 2)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(30))));
}

#[test]
fn test_vector_ref_out_of_bounds_positive() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> 3)").unwrap();
    let result = eval(expr, &mut env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_vector_ref_out_of_bounds_negative() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> -1)").unwrap();
    let result = eval(expr, &mut env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_vector_ref_error_not_vector() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref (quote (1 2 3)) 0)").unwrap();
    let result = eval(expr, &mut env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be a vector"));
}

#[test]
fn test_vector_ref_error_not_integer() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 1 2 3 >> 1.5)").unwrap();
    let result = eval(expr, &mut env);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be an integer"));
}

#[test]
fn test_vector_ref_with_computed_index() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref << 10 20 30 >> (+ 1 1))").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(30))));
}

// ============================================================================
// Vector Nested Structure Tests
// ============================================================================

#[test]
fn test_nested_vectors() {
    let expr = parse("<< << 1 2 >> << 3 4 >> >>").unwrap();
    match expr {
        Value::Vector(outer) => {
            assert_eq!(outer.elements.len(), 2);
            match &outer.elements[0] {
                Value::Vector(inner) => {
                    assert_eq!(inner.elements.len(), 2);
                }
                _ => panic!("Expected nested vector"),
            }
        }
        _ => panic!("Expected vector"),
    }
}

#[test]
fn test_vector_of_vectors_display() {
    let inner1 = Value::Vector(Rc::new(VectorValue {
        elements: vec![
            Value::Atom(AtomType::Number(NumericType::Int(1))),
            Value::Atom(AtomType::Number(NumericType::Int(2))),
        ],
    }));
    let inner2 = Value::Vector(Rc::new(VectorValue {
        elements: vec![
            Value::Atom(AtomType::Number(NumericType::Int(3))),
            Value::Atom(AtomType::Number(NumericType::Int(4))),
        ],
    }));
    let outer = Value::Vector(Rc::new(VectorValue {
        elements: vec![inner1, inner2],
    }));
    assert_eq!(format!("{outer}"), "<<<<1 2>> <<3 4>>>>");
}

#[test]
fn test_vector_ref_nested() {
    let mut env = Environment::new();
    let expr = parse("(vector-ref (vector-ref << << 1 2 >> << 3 4 >> >> 1) 0)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(3))));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_vector_in_list() {
    let expr = parse("(quote (<< 1 2 3 >> x y))").unwrap();
    let mut env = Environment::new();
    let result = eval(expr, &mut env).unwrap();

    match result {
        Value::Cons(cell) => match &cell.car {
            Value::Vector(_) => {}
            _ => panic!("Expected vector as first element"),
        },
        _ => panic!("Expected list"),
    }
}

#[test]
fn test_comparison_operators_still_work() {
    let mut env = Environment::new();

    // Make sure < still works as comparison
    let expr = parse("(< 3 5)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Bool(true)));

    // Make sure > still works as comparison
    let expr = parse("(> 3 5)").unwrap();
    let result = eval(expr, &mut env).unwrap();
    assert_eq!(result, Value::Atom(AtomType::Bool(false)));
}

#[test]
fn test_single_angle_bracket_as_symbol() {
    // A single < or > should be treated as a symbol
    let expr = parse("(< 1 2)").unwrap();
    let mut env = Environment::new();
    let result = eval(expr, &mut env).unwrap();
    // This should work as the less-than comparison
    assert_eq!(result, Value::Atom(AtomType::Bool(true)));
}
