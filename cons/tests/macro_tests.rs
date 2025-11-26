use cons::{eval, register_stdlib};
use consair::{Environment, parse};

fn eval_str(input: &str) -> Result<String, String> {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    let expr = parse(input)?;
    let result = eval(expr, &mut env)?;
    Ok(format!("{}", result))
}

fn eval_multi(inputs: &[&str]) -> Result<String, String> {
    let mut env = Environment::new();
    register_stdlib(&mut env);
    let mut result = String::new();
    for input in inputs {
        let expr = parse(input)?;
        let val = eval(expr, &mut env)?;
        result = format!("{}", val);
    }
    Ok(result)
}

#[test]
fn test_quasiquote_basic() {
    // Basic quasiquote
    assert_eq!(eval_str("`x").unwrap(), "x");
    assert_eq!(eval_str("`(a b c)").unwrap(), "(a b c)");
}

#[test]
fn test_quasiquote_with_unquote() {
    // Quasiquote with unquote
    assert_eq!(eval_str("`(a ,(+ 1 2) c)").unwrap(), "(a 3 c)");
    assert_eq!(eval_str("`(a ,(cons 'x 'y) b)").unwrap(), "(a (x . y) b)");
}

#[test]
fn test_quasiquote_with_unquote_splicing() {
    // Quasiquote with unquote-splicing
    assert_eq!(
        eval_str("`(a ,@(cons 1 (cons 2 nil)) b)").unwrap(),
        "(a 1 2 b)"
    );
}

#[test]
fn test_defmacro_when() {
    let code = r#"
        (label defmacro-when
            (defmacro when (condition body)
                `(cond (,condition ,body) (t nil))))
    "#;
    assert_eq!(eval_str(code).unwrap(), "<macro>");
}

#[test]
fn test_macro_when_usage() {
    let result = eval_multi(&[
        "(defmacro when (condition body) `(cond (,condition ,body) (t nil)))",
        "(when t 42)",
    ])
    .unwrap();
    assert_eq!(result, "42");

    let result2 = eval_multi(&[
        "(defmacro when (condition body) `(cond (,condition ,body) (t nil)))",
        "(when nil 42)",
    ])
    .unwrap();
    assert_eq!(result2, "nil");
}

#[test]
fn test_macro_unless() {
    let result = eval_multi(&[
        "(defmacro unless (condition body) `(cond (,condition nil) (t ,body)))",
        "(unless nil 99)",
    ])
    .unwrap();
    assert_eq!(result, "99");

    let result2 = eval_multi(&[
        "(defmacro unless (condition body) `(cond (,condition nil) (t ,body)))",
        "(unless t 99)",
    ])
    .unwrap();
    assert_eq!(result2, "nil");
}

#[test]
fn test_macro_expansion_macroexpand_1() {
    let result = eval_multi(&[
        "(defmacro when (condition body) `(cond (,condition ,body) (t nil)))",
        "(macroexpand-1 '(when t 42))",
    ])
    .unwrap();
    assert!(result.contains("cond"));
}

#[test]
fn test_gensym() {
    // Test gensym generates unique symbols
    let code = "(gensym)";
    let result1 = eval_str(code).unwrap();
    let result2 = eval_str(code).unwrap();
    assert!(result1.starts_with("g__"));
    assert!(result2.starts_with("g__"));
    assert_ne!(result1, result2); // Each call should produce a different symbol

    // Test gensym with prefix
    let code_prefix = "(gensym \"temp\")";
    let result3 = eval_str(code_prefix).unwrap();
    assert!(result3.starts_with("temp__"));
}

#[test]
fn test_nested_quasiquote() {
    // Test nested quasiquote/unquote
    let code = "``(a ,,1)";
    let result = eval_str(code).unwrap();
    assert_eq!(result, "(quasiquote (a (unquote 1)))");
}

#[test]
fn test_macro_variable_capture() {
    // Test that macros can capture variables (unhygienic behavior)
    let result =
        eval_multi(&["(defmacro set-x (val) `(label x ,val))", "(set-x 100)", "x"]).unwrap();
    assert_eq!(result, "100");
}

#[test]
fn test_macroexpand_full() {
    // Test full macro expansion
    let result = eval_multi(&[
        "(defmacro when (condition body) `(cond (,condition ,body) (t nil)))",
        "(macroexpand '(when (eq 1 1) 42))",
    ])
    .unwrap();
    assert!(result.contains("cond"));
}
