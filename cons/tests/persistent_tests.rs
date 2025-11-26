//! Tests for persistent data structure behavior
//!
//! These tests verify that the persistent feature provides proper
//! structural sharing for efficient immutable operations.

use cons::{eval, register_stdlib};
use consair::abstractions::{assoc, conj, count, empty_map, get, hash_map, hash_set};
use consair::{Environment, parse};

fn setup_env() -> Environment {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    env
}

fn eval_str(code: &str, env: &mut Environment) -> consair::language::Value {
    let expr = parse(code).expect("parse failed");
    eval(expr, env).expect("eval failed")
}

// Test that assoc returns a new map without modifying the original
#[test]
fn test_map_immutability() {
    let map1 = hash_map(vec![(parse("'a").unwrap(), parse("1").unwrap())]);
    let map2 = assoc(&map1, parse("'b").unwrap(), parse("2").unwrap()).unwrap();

    // Original should be unchanged
    assert_eq!(count(&map1), Some(1));
    // New map has both entries
    assert_eq!(count(&map2), Some(2));
}

// Test that conj returns a new vector without modifying the original
#[test]
fn test_vector_immutability() {
    let mut env = setup_env();

    let vec1 = eval_str("<<1 2 3>>", &mut env);
    let vec2 = conj(&vec1, parse("4").unwrap()).unwrap();

    // Original should be unchanged
    assert_eq!(count(&vec1), Some(3));
    // New vector has 4 elements
    assert_eq!(count(&vec2), Some(4));
}

// Test that conj returns a new set without modifying the original
#[test]
fn test_set_immutability() {
    let set1 = hash_set(vec![parse("1").unwrap(), parse("2").unwrap()]);
    let set2 = conj(&set1, parse("3").unwrap()).unwrap();

    // Original should be unchanged
    assert_eq!(count(&set1), Some(2));
    // New set has 3 elements
    assert_eq!(count(&set2), Some(3));
}

// Test nested operations preserve structure
#[test]
fn test_nested_operations() {
    let map1 = empty_map();
    let map2 = assoc(&map1, parse("'x").unwrap(), parse("10").unwrap()).unwrap();
    let map3 = assoc(&map2, parse("'y").unwrap(), parse("20").unwrap()).unwrap();
    let map4 = assoc(&map3, parse("'z").unwrap(), parse("30").unwrap()).unwrap();

    // All maps should exist independently
    assert_eq!(count(&map1), Some(0));
    assert_eq!(count(&map2), Some(1));
    assert_eq!(count(&map3), Some(2));
    assert_eq!(count(&map4), Some(3));

    // Values should be correct
    assert_eq!(
        get(&map4, &parse("'x").unwrap(), None),
        parse("10").unwrap()
    );
    assert_eq!(
        get(&map4, &parse("'y").unwrap(), None),
        parse("20").unwrap()
    );
    assert_eq!(
        get(&map4, &parse("'z").unwrap(), None),
        parse("30").unwrap()
    );
}

// Test that persistent operations work in the interpreter
#[test]
fn test_persistent_in_interpreter() {
    let mut env = setup_env();

    // Create a vector using the interpreter and verify it has 3 elements
    let vec = eval_str("<<1 2 3>>", &mut env);
    assert_eq!(count(&vec), Some(3));

    // Use conj to add an element
    let vec2 = conj(&vec, eval_str("4", &mut env)).unwrap();
    assert_eq!(count(&vec2), Some(4));

    // Original should be unchanged
    assert_eq!(count(&vec), Some(3));
}
