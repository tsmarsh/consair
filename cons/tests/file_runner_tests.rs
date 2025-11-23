use std::fs;
use std::path::PathBuf;
use std::process::Command;

// Helper function to get the path to the cons binary
fn cons_binary() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop(); // Go to workspace root
    path.push("target");
    path.push("debug");
    path.push("cons");
    path
}

// Helper function to create a temp file and run it
fn run_lisp_file(content: &str) -> Result<String, String> {
    let temp_dir = std::env::temp_dir();
    let file_path = temp_dir.join(format!("test_{}.lisp", rand::random::<u32>()));

    fs::write(&file_path, content).map_err(|e| e.to_string())?;

    let output = Command::new(cons_binary())
        .arg(&file_path)
        .output()
        .map_err(|e| e.to_string())?;

    fs::remove_file(&file_path).ok();

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).trim().to_string())
    }
}

#[test]
fn test_multiple_expressions() {
    let result = run_lisp_file(
        r#"
(cons 1 2)
(cons 3 4)
(cons 5 6)
"#,
    );
    // Should return the last expression result
    assert_eq!(result.unwrap(), "(5 . 6)");
}

#[test]
fn test_string_with_parentheses() {
    let result = run_lisp_file(
        r#"
(println "hello (world)")
(cons 1 2)
"#,
    );
    // Should handle string with parens correctly
    assert!(result.is_ok());
}

#[test]
fn test_string_with_brackets() {
    let result = run_lisp_file(
        r#"
(println "test <<vector>>")
(cons 1 2)
"#,
    );
    assert!(result.is_ok());
}

#[test]
fn test_semicolon_comments() {
    let result = run_lisp_file(
        r#"
; This is a comment
(cons 1 2) ; inline comment
; Another comment
(cons 3 4)
"#,
    );
    assert_eq!(result.unwrap(), "(3 . 4)");
}

#[test]
fn test_comment_only_file() {
    let result = run_lisp_file(
        r#"
; Just comments
; Nothing else
"#,
    );
    // Comment-only files strip to empty, which returns "No expression found" error
    assert!(result.is_err());
}

#[test]
fn test_multiline_string() {
    let result = run_lisp_file(
        r#"
(println "line1
line2
line3")
42
"#,
    );
    // println outputs, then final result (both are printed)
    let output = result.unwrap();
    assert!(output.contains("line1"));
    assert!(output.contains("42"));
}

#[test]
fn test_escaped_quotes_in_string() {
    let result = run_lisp_file(
        r#"
(println "She said \"hello\"")
100
"#,
    );
    // println outputs, then final result
    let output = result.unwrap();
    assert!(output.contains("hello"));
    assert!(output.contains("100"));
}

#[test]
fn test_raw_string_basic() {
    // Raw strings are not supported by the Consair Lisp parser
    // The 'r' is treated as a symbol, so this will be a parse error
    let result = run_lisp_file(
        r#"
r"raw string (with) parens"
(cons 1 2)
"#,
    );
    // Should fail because 'r' followed by string literal isn't valid Lisp syntax
    assert!(result.is_err());
}

#[test]
fn test_raw_string_with_hashes() {
    // Raw strings are not supported by the Consair Lisp parser
    let result = run_lisp_file(
        r##"
r#"raw string with "quotes""#
(cons 1 2)
"##,
    );
    // Should fail - raw strings aren't valid Lisp syntax
    assert!(result.is_err());
}

#[test]
fn test_quoted_expression_with_parens() {
    let content = "
(quote (1 2 3))
(quote (a b c))
";
    let result = run_lisp_file(content);
    assert_eq!(result.unwrap(), "(a b c)");
}

#[test]
fn test_nested_expressions() {
    let result = run_lisp_file(
        r#"
(cons (cons 1 2) (cons 3 4))
"#,
    );
    assert_eq!(result.unwrap(), "((1 . 2) 3 . 4)");
}

#[test]
fn test_vector_expressions() {
    let result = run_lisp_file(
        r#"
<<1 2 3>>
<<4 5 6>>
"#,
    );
    assert_eq!(result.unwrap(), "<<4 5 6>>");
}

#[test]
fn test_mixed_expressions() {
    let result = run_lisp_file(
        r#"
; First expression
(cons 1 2)
; Second with string
(println "test (paren)")
; Third with vector
<<1 2 3>>
; Fourth quoted
(quote (a b c))
; Final result
42
"#,
    );
    // println outputs, followed by final result
    let output = result.unwrap();
    assert!(output.contains("test (paren)"));
    assert!(output.contains("42"));
}

#[test]
fn test_empty_file() {
    let result = run_lisp_file("");
    // Empty files succeed with no output
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_whitespace_only_file() {
    let result = run_lisp_file(
        "



   ",
    );
    // Whitespace-only files succeed with no output
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "");
}

#[test]
fn test_unclosed_paren() {
    let input = String::from("(cons 1 2"); // missing closing paren
    let result = run_lisp_file(&input);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Unclosed") || err.contains("parenthesis"));
}

#[test]
fn test_unclosed_string() {
    let result = run_lisp_file(r#"(println "hello)"#);
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(err.contains("Unclosed") || err.contains("string"));
}

#[test]
fn test_unmatched_closing_paren() {
    let mut input = String::from("(cons 1 2"); // will add extra closing paren
    input.push(')'); // balanced
    input.push(')'); // extra
    let result = run_lisp_file(&input);
    assert!(result.is_err());
}

#[test]
fn test_comment_after_expression() {
    let result = run_lisp_file(
        r#"
(cons 1 2) ; this works
; final value
100
"#,
    );
    assert_eq!(result.unwrap(), "100");
}

#[test]
fn test_string_with_escaped_backslash() {
    let result = run_lisp_file(
        r#"
(println "path\\to\\file")
42
"#,
    );
    // println outputs, followed by final result
    let output = result.unwrap();
    assert!(output.contains("path"));
    assert!(output.contains("42"));
}

#[test]
fn test_atoms_separated_by_whitespace() {
    let result = run_lisp_file(
        r#"
42
100
200
"#,
    );
    assert_eq!(result.unwrap(), "200");
}

#[test]
fn test_lambda_definition() {
    let result = run_lisp_file(
        r#"
(label identity (lambda (x) x))
(identity 42)
"#,
    );
    assert_eq!(result.unwrap(), "42");
}

#[test]
fn test_complex_nested_structure() {
    let result = run_lisp_file(
        r#"
; Define a function
(label make-pair (lambda (a b) (cons a b)))
; Use it
(make-pair 1 2)
; Nested usage
(cons (make-pair 3 4) (make-pair 5 6))
"#,
    );
    assert_eq!(result.unwrap(), "((3 . 4) 5 . 6)");
}

#[test]
fn test_comment_between_list_elements() {
    let result = run_lisp_file(
        r#"
(cons
  ; first arg
  1
  ; second arg
  2)
"#,
    );
    // Comments are now natively supported in the lexer
    assert_eq!(result.unwrap(), "(1 . 2)");
}
