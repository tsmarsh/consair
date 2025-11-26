use cons::{eval, register_stdlib};
use consair::{Environment, parse};

fn eval_expr(expr: &str) -> String {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    match parse(expr) {
        Ok(parsed) => match eval(parsed, &mut env) {
            Ok(result) => result.to_string(),
            Err(e) => format!("Error: {e}"),
        },
        Err(e) => format!("Parse error: {e}"),
    }
}

#[test]
fn test_quote() {
    assert_eq!(eval_expr("(quote a)"), "a");
    assert_eq!(eval_expr("(quote (1 2 3))"), "(1 2 3)");
    assert_eq!(eval_expr("'a"), "a");
    assert_eq!(eval_expr("'(1 2 3)"), "(1 2 3)");
}

#[test]
fn test_atom() {
    assert_eq!(eval_expr("(atom 'a)"), "t");
    assert_eq!(eval_expr("(atom 123)"), "t");
    assert_eq!(eval_expr("(atom '(1 2))"), "nil");
    assert_eq!(eval_expr("(atom nil)"), "t");
}

#[test]
fn test_eq() {
    assert_eq!(eval_expr("(eq 'a 'a)"), "t");
    assert_eq!(eval_expr("(eq 'a 'b)"), "nil");
    assert_eq!(eval_expr("(eq 42 42)"), "t");
    assert_eq!(eval_expr("(eq 42 43)"), "nil");
    assert_eq!(eval_expr("(eq nil nil)"), "t");
}

#[test]
fn test_car_cdr() {
    assert_eq!(eval_expr("(car '(1 2 3))"), "1");
    assert_eq!(eval_expr("(cdr '(1 2 3))"), "(2 3)");
    assert_eq!(eval_expr("(car (cdr '(1 2 3)))"), "2");
    assert_eq!(eval_expr("(cdr (cdr '(1 2)))"), "nil");
}

#[test]
fn test_cons() {
    assert_eq!(eval_expr("(cons 1 '(2 3))"), "(1 2 3)");
    assert_eq!(eval_expr("(cons 'a 'b)"), "(a . b)");
    assert_eq!(eval_expr("(cons 1 nil)"), "(1)");
}

#[test]
fn test_cond() {
    assert_eq!(eval_expr("(cond ((eq 1 1) 'yes) (t 'no))"), "yes");
    assert_eq!(eval_expr("(cond ((eq 1 2) 'yes) (t 'no))"), "no");
    assert_eq!(eval_expr("(cond (nil 'a) (t 'b))"), "b");
}

#[test]
fn test_lambda() {
    assert_eq!(eval_expr("((lambda (x) x) 42)"), "42");
    assert_eq!(eval_expr("((lambda (x y) (cons x y)) 1 2)"), "(1 . 2)");
    assert_eq!(eval_expr("((lambda (x) (cons x '(2 3))) 1)"), "(1 2 3)");
}

#[test]
fn test_numbers() {
    assert_eq!(eval_expr("42"), "42");
    assert_eq!(eval_expr("-17"), "-17");
    assert_eq!(eval_expr("(cons 1 (cons 2 (cons 3 nil)))"), "(1 2 3)");
}

#[test]
fn test_label_and_recursion() {
    // Test that label defines a function
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define a simple identity function
    let define = parse("(label identity (lambda (x) x))").unwrap();
    eval(define, &mut env).unwrap();

    // Use the function
    let use_fn = parse("(identity 42)").unwrap();
    let result = eval(use_fn, &mut env).unwrap();
    assert_eq!(result.to_string(), "42");
}

#[test]
fn test_closure() {
    // Test that lambdas capture their environment
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Define a function that returns a closure
    let define = parse("(label make-const (lambda (x) (lambda (y) x)))").unwrap();
    eval(define, &mut env).unwrap();

    // Create a closure
    let create_closure = parse("(label my-const (make-const 42))").unwrap();
    eval(create_closure, &mut env).unwrap();

    // Use the closure - should always return 42
    let use_closure = parse("(my-const 99)").unwrap();
    let result = eval(use_closure, &mut env).unwrap();
    assert_eq!(result.to_string(), "42");
}

#[test]
fn test_list_construction() {
    // Test building lists manually
    assert_eq!(eval_expr("(cons 'a (cons 'b (cons 'c nil)))"), "(a b c)");

    // Test nested lists
    assert_eq!(eval_expr("(cons (cons 1 2) (cons 3 4))"), "((1 . 2) 3 . 4)");
}

#[test]
fn test_nested_lambdas() {
    // Test nested lambda application
    assert_eq!(
        eval_expr("((lambda (x) ((lambda (y) (cons x y)) 2)) 1)"),
        "(1 . 2)"
    );
}

// ============================================================================
// Abstraction Tests (Clojure-inspired)
// ============================================================================

#[test]
fn test_seq_builtin() {
    assert_eq!(eval_expr("(%seq nil)"), "nil");
    assert_eq!(eval_expr("(%seq '(1 2 3))"), "(1 2 3)");
    assert_eq!(eval_expr("(%seq <<1 2 3>>)"), "(1 2 3)");
}

#[test]
fn test_first_builtin() {
    assert_eq!(eval_expr("(%first '(1 2 3))"), "1");
    assert_eq!(eval_expr("(%first <<10 20>>)"), "10");
    assert_eq!(eval_expr("(%first nil)"), "nil");
}

#[test]
fn test_next_builtin() {
    assert_eq!(eval_expr("(%next '(1 2 3))"), "(2 3)");
    assert_eq!(eval_expr("(%next '(1))"), "nil");
}

#[test]
fn test_count_builtin() {
    assert_eq!(eval_expr("(%count nil)"), "0");
    assert_eq!(eval_expr("(%count '(1 2 3))"), "3");
    assert_eq!(eval_expr("(%count <<1 2 3 4 5>>)"), "5");
}

#[test]
fn test_nth_builtin() {
    assert_eq!(eval_expr("(%nth <<10 20 30>> 0)"), "10");
    assert_eq!(eval_expr("(%nth <<10 20 30>> 1)"), "20");
    assert_eq!(eval_expr("(%nth <<10 20 30>> 2)"), "30");
    assert_eq!(eval_expr("(%nth <<1 2 3>> 10)"), "nil");
    assert_eq!(eval_expr("(%nth <<1 2 3>> 10 42)"), "42");
}

#[test]
fn test_hash_map_builtin() {
    assert_eq!(eval_expr("(%count (%hash-map 1 2 3 4))"), "2");
    assert_eq!(eval_expr("(%get (%hash-map 1 100 2 200) 1)"), "100");
    assert_eq!(eval_expr("(%get (%hash-map 1 2) 999)"), "nil");
}

#[test]
fn test_assoc_builtin() {
    assert_eq!(eval_expr("(%get (%assoc (%hash-map) 1 100) 1)"), "100");
}

#[test]
fn test_conj_builtin() {
    // List conj adds at front
    assert_eq!(eval_expr("(%first (%conj '(2 3) 1))"), "1");
    // Vector conj adds at end
    assert_eq!(eval_expr("(%nth (%conj <<1 2>> 3) 2)"), "3");
}

#[test]
fn test_hash_set_builtin() {
    assert_eq!(eval_expr("(%count (%hash-set 1 2 3))"), "3");
    // Sets deduplicate
    assert_eq!(eval_expr("(%count (%hash-set 1 1 1 2 2 3))"), "3");
}

#[test]
fn test_reduced_builtin() {
    assert_eq!(eval_expr("(%reduced? (%reduced 42))"), "t");
    assert_eq!(eval_expr("(%reduced? 42)"), "nil");
    assert_eq!(eval_expr("(%unreduced (%reduced 42))"), "42");
}

#[test]
fn test_empty_builtin() {
    assert_eq!(eval_expr("(%empty? nil)"), "t");
    assert_eq!(eval_expr("(%empty? <<>>)"), "t");
    assert_eq!(eval_expr("(%empty? <<1>>)"), "nil");
}

#[test]
fn test_contains_builtin() {
    assert_eq!(eval_expr("(%contains? (%hash-set 1 2 3) 2)"), "t");
    assert_eq!(eval_expr("(%contains? (%hash-set 1 2 3) 999)"), "nil");
}
