use consair::{AtomType, Environment, NumericType, Value, eval, parse, register_stdlib};

fn eval_vector(expr: &str) -> Value {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    let parsed = parse(expr).unwrap();
    eval(parsed, &mut env).unwrap()
}

fn eval_vector_result(expr: &str) -> Result<Value, String> {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    let parsed = parse(expr)?;
    eval(parsed, &mut env)
}

// ============================================================================
// Vector Construction Tests
// ============================================================================

#[test]
fn test_empty_vector() {
    let result = eval_vector("(vector)");
    match result {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 0);
        }
        _ => panic!("Expected vector, got {result}"),
    }
}

#[test]
fn test_vector_integers() {
    let result = eval_vector("(vector 1 2 3)");
    match result {
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
        _ => panic!("Expected vector, got {result}"),
    }
}

#[test]
fn test_vector_mixed_types() {
    let result = eval_vector("(vector 1 2.5 3/4)");
    match result {
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
        _ => panic!("Expected vector, got {result}"),
    }
}

// ============================================================================
// Vector Display Tests
// ============================================================================

#[test]
fn test_vector_display_empty() {
    let result = eval_vector("(vector)");
    assert_eq!(format!("{result}"), "<<>>");
}

#[test]
fn test_vector_display_integers() {
    let result = eval_vector("(vector 1 2 3)");
    assert_eq!(format!("{result}"), "<<1 2 3>>");
}

#[test]
fn test_vector_display_mixed() {
    let result = eval_vector("(vector 1 2.5)");
    assert_eq!(format!("{result}"), "<<1 2.5>>");
}

// ============================================================================
// Vector with Expressions Tests
// ============================================================================

#[test]
fn test_vector_with_evaluated_expressions() {
    let result = eval_vector("(vector (+ 1 2) (* 3 4))");
    match result {
        Value::Vector(vec) => {
            assert_eq!(vec.elements.len(), 2);
            assert_eq!(
                vec.elements[0],
                Value::Atom(AtomType::Number(NumericType::Int(3)))
            );
            assert_eq!(
                vec.elements[1],
                Value::Atom(AtomType::Number(NumericType::Int(12)))
            );
        }
        _ => panic!("Expected vector, got {result}"),
    }
}

// ============================================================================
// Vector Operation Tests
// ============================================================================

#[test]
fn test_vector_length_empty() {
    let result = eval_vector("(vector-length (vector))");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(0))));
}

#[test]
fn test_vector_length_nonempty() {
    let result = eval_vector("(vector-length (vector 1 2 3 4 5))");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(5))));
}

#[test]
fn test_vector_length_error_not_vector() {
    let result = eval_vector_result("(vector-length (quote (1 2 3)))");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected vector"));
}

#[test]
fn test_vector_ref_basic() {
    let result = eval_vector("(vector-ref (vector 10 20 30) 0)");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(10))));
}

#[test]
fn test_vector_ref_middle() {
    let result = eval_vector("(vector-ref (vector 10 20 30) 1)");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(20))));
}

#[test]
fn test_vector_ref_last() {
    let result = eval_vector("(vector-ref (vector 10 20 30) 2)");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(30))));
}

#[test]
fn test_vector_ref_out_of_bounds_positive() {
    let result = eval_vector_result("(vector-ref (vector 10 20 30) 3)");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_vector_ref_out_of_bounds_negative() {
    let result = eval_vector_result("(vector-ref (vector 10 20 30) -1)");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("out of bounds"));
}

#[test]
fn test_vector_ref_error_not_vector() {
    let result = eval_vector_result("(vector-ref (quote (1 2 3)) 0)");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be a vector"));
}

#[test]
fn test_vector_ref_error_not_integer() {
    let result = eval_vector_result("(vector-ref (vector 1 2 3) 1.5)");
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be an integer"));
}

#[test]
fn test_vector_ref_with_computed_index() {
    let result = eval_vector("(vector-ref (vector 10 20 30) (+ 1 1))");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(30))));
}

// ============================================================================
// Nested Vector Tests
// ============================================================================

#[test]
fn test_nested_vectors() {
    let result = eval_vector("(vector (vector 1 2) (vector 3 4))");
    match result {
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
    let result = eval_vector("(vector (vector 1 2) (vector 3 4))");
    assert_eq!(format!("{result}"), "<<<<1 2>> <<3 4>>>>");
}

#[test]
fn test_vector_ref_nested() {
    let result = eval_vector("(vector-ref (vector-ref (vector (vector 1 2) (vector 3 4)) 1) 0)");
    assert_eq!(result, Value::Atom(AtomType::Number(NumericType::Int(3))));
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_vector_in_list() {
    let result = eval_vector("(quote ((vector 1 2 3) x y))");
    match result {
        Value::Cons(cell) => match &cell.car {
            Value::Cons(_) => {} // First element is the (vector ...) expression as a list
            _ => panic!("Expected cons as first element"),
        },
        _ => panic!("Expected list"),
    }
}
