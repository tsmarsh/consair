use cons::{eval, register_stdlib};
use consair::language::{AtomType, StringType, SymbolType, Value};
use consair::numeric::NumericType;
use consair::{Environment, parse};
use std::fs;

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_env() -> Environment {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    env
}

fn extract_string(value: &Value) -> String {
    match value {
        Value::Atom(AtomType::String(StringType::Basic(s))) => s.clone(),
        _ => panic!("Expected string, got {value:?}"),
    }
}

fn extract_int(value: &Value) -> i64 {
    match value {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => *n,
        _ => panic!("Expected integer, got {value:?}"),
    }
}

fn extract_bool(value: &Value) -> bool {
    match value {
        Value::Atom(AtomType::Bool(b)) => *b,
        Value::Nil => false,
        _ => true,
    }
}

/// Extract a value from an association list by key (using symbols)
fn alist_get(alist: &Value, key_name: &str) -> Option<Value> {
    let mut current = alist.clone();

    while let Value::Cons(ref outer_cell) = current {
        if let Value::Cons(ref pair) = outer_cell.car
            && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &pair.car
            && name.with_str(|s| s == key_name)
        {
            return Some(pair.cdr.clone());
        }
        current = outer_cell.cdr.clone();
    }

    None
}

// ============================================================================
// File I/O Tests
// ============================================================================

#[test]
fn test_slurp_spit() {
    let mut env = create_test_env();
    let test_file = std::env::temp_dir().join("consair_test_slurp_spit.txt");
    let test_file_str = test_file.to_str().unwrap();
    let test_content = "Hello, Consair!";

    // Clean up any existing file
    let _ = fs::remove_file(&test_file);

    // Write content
    let write_code = format!(r#"(spit "{test_file_str}" "{test_content}")"#);
    let write_result = eval(parse(&write_code).unwrap(), &mut env).unwrap();
    assert_eq!(write_result, Value::Nil);

    // Verify file exists
    assert!(test_file.exists());

    // Read content back
    let read_code = format!(r#"(slurp "{test_file_str}")"#);
    let read_result = eval(parse(&read_code).unwrap(), &mut env).unwrap();
    let content = extract_string(&read_result);
    assert_eq!(content, test_content);

    // Clean up
    fs::remove_file(&test_file).unwrap();
}

#[test]
fn test_slurp_nonexistent_file() {
    let mut env = create_test_env();
    let result = eval(
        parse(r#"(slurp "/nonexistent/file/that/does/not/exist.txt")"#).unwrap(),
        &mut env,
    );

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("slurp"));
}

#[test]
fn test_spit_multiline() {
    let mut env = create_test_env();
    let test_file = std::env::temp_dir().join("consair_test_multiline.txt");
    let test_file_str = test_file.to_str().unwrap();
    let test_content = "Line 1\nLine 2\nLine 3";

    // Clean up
    let _ = fs::remove_file(&test_file);

    // Write multiline content
    let write_code = format!(r#"(spit "{test_file_str}" "{test_content}")"#);
    eval(parse(&write_code).unwrap(), &mut env).unwrap();

    // Read it back
    let read_code = format!(r#"(slurp "{test_file_str}")"#);
    let result = eval(parse(&read_code).unwrap(), &mut env).unwrap();
    let content = extract_string(&result);
    assert_eq!(content, test_content);

    // Clean up
    fs::remove_file(&test_file).unwrap();
}

// ============================================================================
// Shell Command Tests
// ============================================================================

#[test]
fn test_shell_simple_command() {
    let mut env = create_test_env();

    // Use a command that works on all platforms
    let cmd = "echo hello";

    let code = format!(r#"(shell "{cmd}")"#);
    let result = eval(parse(&code).unwrap(), &mut env).unwrap();

    // Extract stdout from result alist
    let stdout = alist_get(&result, "out").expect("Expected :out key");
    let stdout_str = extract_string(&stdout);
    assert!(stdout_str.contains("hello"));

    // Check exit code
    let exit_code = alist_get(&result, "exit").expect("Expected :exit key");
    assert_eq!(extract_int(&exit_code), 0);

    // Check success flag
    let success = alist_get(&result, "success").expect("Expected :success key");
    assert!(extract_bool(&success));
}

#[test]
fn test_shell_failing_command() {
    let mut env = create_test_env();

    // Use a command that will fail on all platforms
    let cmd = if cfg!(target_os = "windows") {
        "cmd /c exit 1"
    } else {
        "sh -c 'exit 1'"
    };

    let code = format!(r#"(shell "{cmd}")"#);
    let result = eval(parse(&code).unwrap(), &mut env).unwrap();

    // Check exit code is non-zero
    let exit_code = alist_get(&result, "exit").expect("Expected :exit key");
    assert_eq!(extract_int(&exit_code), 1);

    // Check success flag is false
    let success = alist_get(&result, "success").expect("Expected :success key");
    assert!(!extract_bool(&success));
}

#[test]
fn test_shell_with_stderr() {
    let mut env = create_test_env();

    // Command that writes to stderr
    let cmd = if cfg!(target_os = "windows") {
        "echo error 1>&2"
    } else {
        "echo error >&2"
    };

    let code = format!(r#"(shell "{cmd}")"#);
    let result = eval(parse(&code).unwrap(), &mut env).unwrap();

    // Check stderr contains output
    let stderr = alist_get(&result, "err").expect("Expected :err key");
    let stderr_str = extract_string(&stderr);
    assert!(stderr_str.contains("error"));
}

// ============================================================================
// Time Tests
// ============================================================================

#[test]
fn test_now() {
    let mut env = create_test_env();

    let result = eval(parse("(now)").unwrap(), &mut env).unwrap();

    // Should be a positive integer (Unix timestamp)
    let timestamp = extract_int(&result);
    assert!(timestamp > 0);

    // Should be a reasonable timestamp (after 2020 and before 2100)
    let year_2020 = 1577836800; // Jan 1, 2020
    let year_2100 = 4102444800; // Jan 1, 2100
    assert!(timestamp > year_2020);
    assert!(timestamp < year_2100);
}

#[test]
fn test_now_with_args_fails() {
    let mut env = create_test_env();

    let result = eval(parse("(now 123)").unwrap(), &mut env);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected 0 arguments"));
}

// ============================================================================
// Print Tests (these capture stdout, so we test behavior not output)
// ============================================================================

#[test]
fn test_print_returns_nil() {
    let mut env = create_test_env();

    let result = eval(parse(r#"(print "hello")"#).unwrap(), &mut env).unwrap();

    assert_eq!(result, Value::Nil);
}

#[test]
fn test_println_returns_nil() {
    let mut env = create_test_env();

    let result = eval(parse(r#"(println "hello")"#).unwrap(), &mut env).unwrap();

    assert_eq!(result, Value::Nil);
}

#[test]
fn test_print_multiple_args() {
    let mut env = create_test_env();

    // Should not error with multiple arguments
    let result = eval(parse(r#"(print "hello" "world" 123)"#).unwrap(), &mut env);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::Nil);
}

#[test]
fn test_println_with_numbers() {
    let mut env = create_test_env();

    let result = eval(parse("(println 1 2 3)").unwrap(), &mut env);

    assert!(result.is_ok());
    assert_eq!(result.unwrap(), Value::Nil);
}

// ============================================================================
// Integration Tests
// ============================================================================

#[test]
fn test_slurp_result_can_be_printed() {
    let mut env = create_test_env();
    let test_file = std::env::temp_dir().join("consair_print_test.txt");
    let test_file_str = test_file.to_str().unwrap();
    let test_content = "Content to print";

    // Clean up
    let _ = fs::remove_file(&test_file);

    // Write file
    eval(
        parse(&format!(r#"(spit "{test_file_str}" "{test_content}")"#)).unwrap(),
        &mut env,
    )
    .unwrap();

    // Read and print (should not error)
    let result = eval(
        parse(&format!(r#"(println (slurp "{test_file_str}"))"#)).unwrap(),
        &mut env,
    );

    assert!(result.is_ok());

    // Clean up
    fs::remove_file(&test_file).unwrap();
}

#[test]
fn test_arity_checking() {
    let mut env = create_test_env();

    // slurp needs exactly 1 arg
    assert!(eval(parse("(slurp)").unwrap(), &mut env).is_err());
    assert!(eval(parse(r#"(slurp "a" "b")"#).unwrap(), &mut env).is_err());

    // spit needs exactly 2 args
    assert!(eval(parse(r#"(spit "a")"#).unwrap(), &mut env).is_err());
    assert!(eval(parse(r#"(spit "a" "b" "c")"#).unwrap(), &mut env).is_err());

    // shell needs exactly 1 arg
    assert!(eval(parse("(shell)").unwrap(), &mut env).is_err());
    assert!(eval(parse(r#"(shell "a" "b")"#).unwrap(), &mut env).is_err());

    // now needs exactly 0 args
    assert!(eval(parse("(now 1)").unwrap(), &mut env).is_err());
}
