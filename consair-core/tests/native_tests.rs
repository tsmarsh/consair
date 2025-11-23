use consair::interpreter::Environment;
use consair::language::{AtomType, StringType, Value};
use consair::native::{check_arity_exact, make_int, make_string};
use consair::{eval, parse};

// ============================================================================
// Example Native Functions
// ============================================================================

/// A simple native function that adds 1 to an integer
fn add_one(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    use consair::numeric::NumericType;

    check_arity_exact("add-one", args, 1)?;

    match &args[0] {
        Value::Atom(AtomType::Number(NumericType::Int(n))) => Ok(make_int(n + 1)),
        _ => Err("add-one: expected integer".to_string()),
    }
}

/// A native function that concatenates strings
fn str_concat(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut result = String::new();

    for arg in args {
        match arg {
            Value::Atom(AtomType::String(StringType::Basic(s))) => {
                result.push_str(s);
            }
            _ => return Err(format!("str-concat: expected string, got {arg}")),
        }
    }

    Ok(make_string(result))
}

/// A native function that returns the length of a list
fn list_length(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    check_arity_exact("list-length", args, 1)?;

    let mut count = 0;
    let mut current = args[0].clone();

    while let Value::Cons(ref cell) = current {
        count += 1;
        current = cell.cdr.clone();
    }

    if current != Value::Nil {
        return Err("list-length: expected proper list".to_string());
    }

    Ok(make_int(count))
}

// ============================================================================
// Tests
// ============================================================================

#[test]
fn test_native_function_registration() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Verify the function can be called (indirect test of registration)
    let result = eval(parse("(add-one 5)").unwrap(), &mut env);
    assert!(result.is_ok());
}

#[test]
fn test_native_function_call() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Call the native function
    let result = eval(parse("(add-one 5)").unwrap(), &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(n)) => {
            assert_eq!(n.to_string(), "6");
        }
        _ => panic!("Expected number, got {result:?}"),
    }
}

#[test]
fn test_native_function_str_concat() {
    let mut env = Environment::new();
    env.define("str-concat".to_string(), Value::NativeFn(str_concat));

    // Call with multiple arguments
    let result = eval(
        parse(r#"(str-concat "Hello" " " "World")"#).unwrap(),
        &mut env,
    )
    .unwrap();

    match result {
        Value::Atom(AtomType::String(StringType::Basic(s))) => {
            assert_eq!(s, "Hello World");
        }
        _ => panic!("Expected string, got {result:?}"),
    }
}

#[test]
fn test_native_function_list_length() {
    let mut env = Environment::new();
    env.define("list-length".to_string(), Value::NativeFn(list_length));

    // Test with a list
    let result = eval(parse("(list-length '(1 2 3 4 5))").unwrap(), &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(n)) => {
            assert_eq!(n.to_string(), "5");
        }
        _ => panic!("Expected number, got {result:?}"),
    }
}

#[test]
fn test_native_function_arity_check() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Call with wrong number of arguments
    let result = eval(parse("(add-one 1 2)").unwrap(), &mut env);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("add-one: expected 1 argument"));
}

#[test]
fn test_native_function_type_check() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Call with wrong type
    let result = eval(parse(r#"(add-one "not a number")"#).unwrap(), &mut env);

    assert!(result.is_err());
    assert!(result.unwrap_err().contains("expected integer"));
}

#[test]
fn test_native_function_composition() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Compose native function with itself
    let result = eval(parse("(add-one (add-one 5))").unwrap(), &mut env).unwrap();

    match result {
        Value::Atom(AtomType::Number(n)) => {
            assert_eq!(n.to_string(), "7");
        }
        _ => panic!("Expected number, got {result:?}"),
    }
}

#[test]
fn test_native_function_with_lambda() {
    let mut env = Environment::new();
    env.define("add-one".to_string(), Value::NativeFn(add_one));

    // Use native function with lambda
    let result = eval(
        parse("((lambda (f x) (f (f x))) add-one 10)").unwrap(),
        &mut env,
    )
    .unwrap();

    match result {
        Value::Atom(AtomType::Number(n)) => {
            assert_eq!(n.to_string(), "12");
        }
        _ => panic!("Expected number, got {result:?}"),
    }
}
