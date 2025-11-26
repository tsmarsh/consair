//! Integration tests for Clojure-inspired abstractions.

use consair::{AtomType, Environment, NumericType, Value, eval, parse, register_stdlib};

fn run(code: &str) -> Result<Value, String> {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    let expr = parse(code)?;
    eval(expr, &mut env)
}

fn run_bool(code: &str) -> bool {
    match run(code) {
        Ok(Value::Atom(AtomType::Bool(b))) => b,
        Ok(Value::Nil) => false,
        _ => panic!("Expected bool result from: {}", code),
    }
}

fn run_int(code: &str) -> i64 {
    match run(code) {
        Ok(Value::Atom(AtomType::Number(NumericType::Int(n)))) => n,
        other => panic!("Expected int result from: {}, got {:?}", code, other),
    }
}

// ============================================================================
// Seq/First/Next Tests
// ============================================================================

#[test]
fn test_seq_on_list() {
    let result = run("(%seq '(1 2 3))").unwrap();
    assert_eq!(format!("{}", result), "(1 2 3)");
}

#[test]
fn test_seq_on_vector() {
    let result = run("(%seq <<1 2 3>>)").unwrap();
    assert_eq!(format!("{}", result), "(1 2 3)");
}

#[test]
fn test_seq_on_nil() {
    let result = run("(%seq nil)").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_first_on_list() {
    assert_eq!(run_int("(%first '(10 20 30))"), 10);
}

#[test]
fn test_first_on_vector() {
    assert_eq!(run_int("(%first <<100 200 300>>)"), 100);
}

#[test]
fn test_first_on_nil() {
    let result = run("(%first nil)").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_next_on_list() {
    let result = run("(%next '(1 2 3))").unwrap();
    assert_eq!(format!("{}", result), "(2 3)");
}

#[test]
fn test_next_on_single_element() {
    let result = run("(%next '(1))").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_rest_on_list() {
    let result = run("(%rest '(1 2 3))").unwrap();
    assert_eq!(format!("{}", result), "(2 3)");
}

// ============================================================================
// Count Tests
// ============================================================================

#[test]
fn test_count_list() {
    assert_eq!(run_int("(%count '(1 2 3))"), 3);
}

#[test]
fn test_count_vector() {
    assert_eq!(run_int("(%count <<1 2 3 4 5>>)"), 5);
}

#[test]
fn test_count_nil() {
    assert_eq!(run_int("(%count nil)"), 0);
}

#[test]
fn test_count_string() {
    assert_eq!(run_int("(%count \"hello\")"), 5);
}

// ============================================================================
// Nth Tests
// ============================================================================

#[test]
fn test_nth_vector() {
    assert_eq!(run_int("(%nth <<10 20 30>> 0)"), 10);
    assert_eq!(run_int("(%nth <<10 20 30>> 1)"), 20);
    assert_eq!(run_int("(%nth <<10 20 30>> 2)"), 30);
}

#[test]
fn test_nth_list() {
    assert_eq!(run_int("(%nth '(100 200 300) 1)"), 200);
}

#[test]
fn test_nth_with_default() {
    assert_eq!(run_int("(%nth <<1 2 3>> 10 42)"), 42);
}

#[test]
fn test_nth_out_of_bounds_returns_nil() {
    let result = run("(%nth <<1 2 3>> 10)").unwrap();
    assert_eq!(result, Value::Nil);
}

// ============================================================================
// Hash Map Tests
// ============================================================================

#[test]
fn test_hash_map_creation() {
    let result = run("(%hash-map)").unwrap();
    assert!(format!("{}", result).contains("{"));
}

#[test]
fn test_hash_map_with_values() {
    assert_eq!(run_int("(%count (%hash-map 1 2 3 4))"), 2);
}

#[test]
fn test_get_from_map() {
    assert_eq!(run_int("(%get (%hash-map 1 100 2 200) 1)"), 100);
}

#[test]
fn test_get_missing_returns_nil() {
    let result = run("(%get (%hash-map 1 2) 999)").unwrap();
    assert_eq!(result, Value::Nil);
}

#[test]
fn test_get_with_default() {
    assert_eq!(run_int("(%get (%hash-map 1 2) 999 42)"), 42);
}

#[test]
fn test_assoc_map() {
    assert_eq!(run_int("(%get (%assoc (%hash-map) 1 100) 1)"), 100);
}

#[test]
fn test_assoc_multiple() {
    let code = "(%count (%assoc (%hash-map) 1 10 2 20 3 30))";
    assert_eq!(run_int(code), 3);
}

#[test]
fn test_dissoc() {
    let code = "(%count (%dissoc (%hash-map 1 10 2 20 3 30) 2))";
    assert_eq!(run_int(code), 2);
}

#[test]
fn test_keys() {
    let code = "(%count (%keys (%hash-map 1 10 2 20)))";
    assert_eq!(run_int(code), 2);
}

#[test]
fn test_vals() {
    let code = "(%count (%vals (%hash-map 1 10 2 20)))";
    assert_eq!(run_int(code), 2);
}

// ============================================================================
// Hash Set Tests
// ============================================================================

#[test]
fn test_hash_set_creation() {
    let result = run("(%hash-set)").unwrap();
    assert!(format!("{}", result).contains("#{"));
}

#[test]
fn test_hash_set_with_values() {
    assert_eq!(run_int("(%count (%hash-set 1 2 3))"), 3);
}

#[test]
fn test_hash_set_deduplicates() {
    assert_eq!(run_int("(%count (%hash-set 1 1 1 2 2 3))"), 3);
}

#[test]
fn test_conj_set() {
    assert_eq!(run_int("(%count (%conj (%hash-set) 1 2 3))"), 3);
}

#[test]
fn test_disj_set() {
    let code = "(%count (%disj (%hash-set 1 2 3) 2))";
    assert_eq!(run_int(code), 2);
}

#[test]
fn test_contains_set() {
    assert!(run_bool("(%contains? (%hash-set 1 2 3) 2)"));
    assert!(!run_bool("(%contains? (%hash-set 1 2 3) 999)"));
}

// ============================================================================
// Vector Abstraction Tests
// ============================================================================

#[test]
fn test_conj_vector() {
    let result = run("(%conj <<1 2>> 3)").unwrap();
    assert_eq!(format!("{}", result), "<<1 2 3>>");
}

#[test]
fn test_assoc_vector() {
    let result = run("(%assoc <<1 2 3>> 0 100)").unwrap();
    assert_eq!(format!("{}", result), "<<100 2 3>>");
}

#[test]
fn test_get_vector() {
    assert_eq!(run_int("(%get <<10 20 30>> 1)"), 20);
}

#[test]
fn test_contains_vector() {
    assert!(run_bool("(%contains? <<1 2 3>> 0)"));
    assert!(run_bool("(%contains? <<1 2 3>> 2)"));
    assert!(!run_bool("(%contains? <<1 2 3>> 3)"));
}

// ============================================================================
// List Abstraction Tests
// ============================================================================

#[test]
fn test_conj_list() {
    let result = run("(%conj '(2 3) 1)").unwrap();
    assert_eq!(format!("{}", result), "(1 2 3)");
}

#[test]
fn test_conj_nil() {
    let result = run("(%conj nil 1)").unwrap();
    assert_eq!(format!("{}", result), "(1)");
}

// ============================================================================
// Reduced Tests
// ============================================================================

#[test]
fn test_reduced_creation() {
    let result = run("(%reduced 42)").unwrap();
    assert!(format!("{}", result).contains("reduced"));
}

#[test]
fn test_reduced_predicate() {
    assert!(run_bool("(%reduced? (%reduced 42))"));
    assert!(!run_bool("(%reduced? 42)"));
}

#[test]
fn test_unreduced() {
    assert_eq!(run_int("(%unreduced (%reduced 42))"), 42);
}

#[test]
fn test_unreduced_non_reduced() {
    assert_eq!(run_int("(%unreduced 42)"), 42);
}

// ============================================================================
// Empty Tests
// ============================================================================

#[test]
fn test_empty_nil() {
    assert!(run_bool("(%empty? nil)"));
}

#[test]
fn test_empty_vector() {
    assert!(run_bool("(%empty? <<>>)"));
    assert!(!run_bool("(%empty? <<1>>)"));
}

#[test]
fn test_empty_list() {
    // Note: '() parses as nil
    assert!(run_bool("(%empty? nil)"));
    assert!(!run_bool("(%empty? '(1))"));
}

#[test]
fn test_empty_map() {
    assert!(run_bool("(%empty? (%hash-map))"));
    assert!(!run_bool("(%empty? (%hash-map 1 2))"));
}

#[test]
fn test_empty_set() {
    assert!(run_bool("(%empty? (%hash-set))"));
    assert!(!run_bool("(%empty? (%hash-set 1))"));
}

#[test]
fn test_empty_string() {
    assert!(run_bool("(%empty? \"\")"));
    assert!(!run_bool("(%empty? \"hello\")"));
}

// ============================================================================
// Seq over Map/Set
// ============================================================================

#[test]
fn test_seq_map() {
    // Map seq should return key-value pairs
    let code = "(%count (%seq (%hash-map 1 2 3 4)))";
    assert_eq!(run_int(code), 2);
}

#[test]
fn test_seq_set() {
    let code = "(%count (%seq (%hash-set 1 2 3)))";
    assert_eq!(run_int(code), 3);
}

// ============================================================================
// String Seq
// ============================================================================

#[test]
fn test_seq_string() {
    let result = run("(%first \"abc\")").unwrap();
    assert_eq!(format!("{}", result), "\"a\"");
}

#[test]
fn test_count_string_unicode() {
    // Unicode string with multi-byte characters
    assert_eq!(run_int("(%count \"日本語\")"), 3);
}
