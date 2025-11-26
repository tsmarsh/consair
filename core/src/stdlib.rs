//! Standard library native functions
//!
//! This module provides the core native functions that are available
//! in the Consair Lisp environment.

use std::fs;
use std::io::{self, Write};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::interner::InternedSymbol;
use crate::interpreter::Environment;
use crate::language::{AtomType, StringType, SymbolType, Value};
use crate::native::{extract_string, make_int, make_string, vec_to_alist};
use crate::numeric::NumericType;

// ============================================================================
// Standard I/O
// ============================================================================

/// Print values to stdout with newline
/// Usage: (println "hello" "world") => prints "hello world\n", returns nil
pub fn println(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    print_impl(args, true)
}

/// Print values to stdout without newline
/// Usage: (print "hello" "world") => prints "hello world", returns nil
pub fn print(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    print_impl(args, false)
}

/// Internal implementation for print/println
fn print_impl(args: &[Value], newline: bool) -> Result<Value, String> {
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            write!(handle, " ").map_err(|e| format!("print: I/O error: {e}"))?;
        }

        // Display the value
        let display_str = value_to_display_string(arg);
        write!(handle, "{display_str}").map_err(|e| format!("print: I/O error: {e}"))?;
    }

    if newline {
        writeln!(handle).map_err(|e| format!("println: I/O error: {e}"))?;
    }

    handle
        .flush()
        .map_err(|e| format!("print: I/O error: {e}"))?;

    Ok(Value::Nil)
}

/// Convert a Value to its display string
/// Strings are printed without quotes, everything else uses Display
fn value_to_display_string(value: &Value) -> String {
    match value {
        Value::Atom(AtomType::String(crate::language::StringType::Basic(s))) => s.clone(),
        _ => format!("{value}"),
    }
}

// ============================================================================
// File I/O
// ============================================================================

/// Read entire file as string (Clojure's slurp)
/// Usage: (slurp "path/to/file.txt") => "file contents"
pub fn slurp(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("slurp: expected 1 argument (path)".to_string());
    }

    let path = extract_string(&args[0])?;

    let content =
        fs::read_to_string(&path).map_err(|e| format!("slurp: failed to read '{path}': {e}"))?;

    Ok(make_string(content))
}

/// Write string to file (Clojure's spit)
/// Usage: (spit "path/to/file.txt" "content") => nil
pub fn spit(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("spit: expected 2 arguments (path, content)".to_string());
    }

    let path = extract_string(&args[0])?;
    let content = extract_string(&args[1])?;

    fs::write(&path, content).map_err(|e| format!("spit: failed to write '{path}': {e}"))?;

    Ok(Value::Nil)
}

// ============================================================================
// Process Execution
// ============================================================================

/// Execute shell command and return output
/// Usage: (shell "ls -la") => ((:out . "...") (:err . "...") (:exit . 0) (:success . true))
pub fn shell(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("shell: expected 1 argument (command)".to_string());
    }

    let command = extract_string(&args[0])?;

    let output = if cfg!(target_os = "windows") {
        Command::new("cmd").args(["/C", &command]).output()
    } else {
        Command::new("sh").arg("-c").arg(&command).output()
    };

    let output = output.map_err(|e| format!("shell: failed to execute command: {e}"))?;

    // Convert stdout/stderr to strings
    let stdout_str = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr_str = String::from_utf8_lossy(&output.stderr).to_string();
    let exit_code = output.status.code().unwrap_or(-1) as i64;
    let success = output.status.success();

    // Build association list: ((out . "...") (err . "...") (exit . 0) (success . t))
    let result_pairs = vec![
        (
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                "out",
            )))),
            make_string(stdout_str),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                "err",
            )))),
            make_string(stderr_str),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                "exit",
            )))),
            make_int(exit_code),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                "success",
            )))),
            Value::Atom(AtomType::Bool(success)),
        ),
    ];

    Ok(vec_to_alist(result_pairs))
}

// ============================================================================
// Time and Date
// ============================================================================

/// Get current Unix timestamp (seconds since epoch)
/// Usage: (now) => 1699564800
pub fn now(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if !args.is_empty() {
        return Err("now: expected 0 arguments".to_string());
    }

    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| format!("now: failed to get time: {e}"))?;

    Ok(Value::Atom(AtomType::Number(NumericType::Int(
        duration.as_secs() as i64,
    ))))
}

// ============================================================================
// Macro Support
// ============================================================================

/// Global counter for gensym
static GENSYM_COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// Generate a unique symbol (for macro hygiene)
/// Usage: (gensym) => g__123
/// Usage: (gensym "prefix") => prefix__123
pub fn gensym(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let prefix = if args.is_empty() {
        "g".to_string()
    } else if args.len() == 1 {
        extract_string(&args[0])?
    } else {
        return Err("gensym: expected 0 or 1 arguments".to_string());
    };

    let counter = GENSYM_COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    let symbol = format!("{prefix}__{counter}");

    Ok(Value::Atom(AtomType::Symbol(SymbolType::Symbol(
        crate::interner::InternedSymbol::new(&symbol),
    ))))
}

/// Expand a macro call once
/// Usage: (macroexpand-1 '(when condition body)) => (cond (condition body))
pub fn macroexpand_1(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("macroexpand-1: expected 1 argument".to_string());
    }

    let expr = args[0].clone();

    // Use the internal expand_macro_once function
    if let Value::Cons(cell) = &expr
        && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &cell.car
        && let Some(Value::Macro(macro_cell)) = env.lookup(&name.resolve())
    {
        // Collect unevaluated arguments
        let mut macro_args = Vec::new();
        let mut current = cell.cdr.clone();
        while let Value::Cons(ref arg_cell) = current {
            macro_args.push(arg_cell.car.clone());
            current = arg_cell.cdr.clone();
        }

        // Check argument count
        if macro_args.len() != macro_cell.params.len() {
            return Err(format!(
                "macro: expected {} arguments, got {}",
                macro_cell.params.len(),
                macro_args.len()
            ));
        }

        // Create environment for macro expansion
        let mut macro_env = macro_cell.env.extend(&macro_cell.params, &macro_args);

        // Evaluate macro body to get expanded code
        return crate::interpreter::eval(macro_cell.body.clone(), &mut macro_env);
    }

    // Not a macro call, return unchanged
    Ok(expr)
}

/// Fully expand all macros in an expression
/// Usage: (macroexpand '(when condition body)) => fully expanded form
pub fn macroexpand(args: &[Value], env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("macroexpand: expected 1 argument".to_string());
    }

    let mut expr = args[0].clone();
    let mut expanded = true;

    // Keep expanding until no more macros
    while expanded {
        let new_expr = macroexpand_1(&[expr.clone()], env)?;
        expanded = new_expr != expr;
        expr = new_expr;
    }

    Ok(expr)
}

// ============================================================================
// Core List Operations (de-sugared from special forms)
// ============================================================================

/// Test if value is an atom
pub fn atom(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("atom: expected 1 argument".to_string());
    }
    let is_atom = matches!(args[0], Value::Atom(_) | Value::Nil);
    Ok(Value::Atom(AtomType::Bool(is_atom)))
}

/// Test equality of two atoms
pub fn eq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("eq: expected 2 arguments".to_string());
    }
    let result = match (&args[0], &args[1]) {
        (Value::Atom(a1), Value::Atom(a2)) => a1 == a2,
        (Value::Nil, Value::Nil) => true,
        _ => false,
    };
    Ok(Value::Atom(AtomType::Bool(result)))
}

/// Get first element of a list
pub fn car(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("car: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Cons(cell) => Ok(cell.car.clone()),
        _ => Err(format!("car: expected cons cell, got {}", args[0])),
    }
}

/// Get rest of a list
pub fn cdr(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cdr: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Cons(cell) => Ok(cell.cdr.clone()),
        _ => Err(format!("cdr: expected cons cell, got {}", args[0])),
    }
}

/// Construct a cons cell
pub fn cons_fn(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("cons: expected 2 arguments".to_string());
    }
    Ok(crate::language::cons(args[0].clone(), args[1].clone()))
}

// ============================================================================
// Type Predicates (for JIT/AOT parity)
// ============================================================================

/// Test if value is nil
pub fn nil_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("nil?: expected 1 argument".to_string());
    }
    Ok(Value::Atom(AtomType::Bool(matches!(args[0], Value::Nil))))
}

/// Test if value is a cons cell (list)
pub fn cons_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("cons?: expected 1 argument".to_string());
    }
    Ok(Value::Atom(AtomType::Bool(matches!(
        args[0],
        Value::Cons(_)
    ))))
}

/// Test if value is a number
pub fn number_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("number?: expected 1 argument".to_string());
    }
    let is_num = matches!(args[0], Value::Atom(AtomType::Number(_)));
    Ok(Value::Atom(AtomType::Bool(is_num)))
}

/// Logical not
pub fn not_fn(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("not: expected 1 argument".to_string());
    }
    let is_false = matches!(args[0], Value::Nil | Value::Atom(AtomType::Bool(false)));
    Ok(Value::Atom(AtomType::Bool(is_false)))
}

// ============================================================================
// List Operations (for JIT/AOT parity)
// ============================================================================

/// Get length of a list
pub fn length(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("length: expected 1 argument".to_string());
    }
    let mut count: i64 = 0;
    let mut current = &args[0];
    while let Value::Cons(cell) = current {
        count += 1;
        current = &cell.cdr;
    }
    Ok(make_int(count))
}

/// Append two lists
pub fn append(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("append: expected 2 arguments".to_string());
    }

    // If first list is nil, return second
    if matches!(args[0], Value::Nil) {
        return Ok(args[1].clone());
    }

    // Collect elements from first list
    let mut elements = Vec::new();
    let mut current = &args[0];
    while let Value::Cons(cell) = current {
        elements.push(cell.car.clone());
        current = &cell.cdr;
    }

    // Build result by consing onto second list
    let mut result = args[1].clone();
    for elem in elements.into_iter().rev() {
        result = crate::language::cons(elem, result);
    }
    Ok(result)
}

/// Reverse a list
pub fn reverse(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("reverse: expected 1 argument".to_string());
    }

    let mut result = Value::Nil;
    let mut current = &args[0];
    while let Value::Cons(cell) = current {
        result = crate::language::cons(cell.car.clone(), result);
        current = &cell.cdr;
    }
    Ok(result)
}

/// Create a list from arguments
pub fn list(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    let mut result = Value::Nil;
    for arg in args.iter().rev() {
        result = crate::language::cons(arg.clone(), result);
    }
    Ok(result)
}

/// Get nth element of a list (0-indexed)
pub fn nth(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("nth: expected 2 arguments (list, index)".to_string());
    }

    let n = match &args[1] {
        Value::Atom(AtomType::Number(NumericType::Int(i))) => *i as usize,
        _ => return Err("nth: index must be an integer".to_string()),
    };

    let mut current = &args[0];
    let mut i = 0;
    while let Value::Cons(cell) = current {
        if i == n {
            return Ok(cell.car.clone());
        }
        i += 1;
        current = &cell.cdr;
    }

    Ok(Value::Nil) // Return nil if index out of bounds
}

// ============================================================================
// Vector Operations (for JIT/AOT parity)
// ============================================================================

/// Get length of a vector
pub fn vector_length(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("vector-length: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Vector(v) => Ok(make_int(v.elements.len() as i64)),
        _ => Err(format!("vector-length: expected vector, got {}", args[0])),
    }
}

/// Get element from vector by index
pub fn vector_ref(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("vector-ref: expected 2 arguments (vector, index)".to_string());
    }

    let vec = match &args[0] {
        Value::Vector(v) => v,
        _ => return Err(format!("vector-ref: expected vector, got {}", args[0])),
    };

    let idx = match &args[1] {
        Value::Atom(AtomType::Number(NumericType::Int(i))) => *i as usize,
        _ => return Err("vector-ref: index must be an integer".to_string()),
    };

    if idx < vec.elements.len() {
        Ok(vec.elements[idx].clone())
    } else {
        Err(format!(
            "vector-ref: index {} out of bounds for vector of length {}",
            idx,
            vec.elements.len()
        ))
    }
}

// ============================================================================
// Arithmetic Operations (de-sugared from special forms)
// ============================================================================

/// Addition
pub fn add(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("+: expected at least 2 arguments".to_string());
    }

    let mut result = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n.clone(),
        _ => return Err(format!("+: expected number, got {}", args[0])),
    };

    for arg in &args[1..] {
        let num = match arg {
            Value::Atom(AtomType::Number(n)) => n,
            _ => return Err(format!("+: expected number, got {}", arg)),
        };
        result = result.add(num)?;
    }

    Ok(Value::Atom(AtomType::Number(result)))
}

/// Subtraction (variadic: subtracts successive arguments from first)
pub fn sub(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("-: expected at least 2 arguments".to_string());
    }

    let mut result = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n.clone(),
        _ => return Err(format!("-: expected number, got {}", args[0])),
    };

    for arg in &args[1..] {
        let num = match arg {
            Value::Atom(AtomType::Number(n)) => n,
            _ => return Err(format!("-: expected number, got {}", arg)),
        };
        result = result.sub(num)?;
    }

    Ok(Value::Atom(AtomType::Number(result)))
}

/// Multiplication
pub fn mul(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("*: expected at least 2 arguments".to_string());
    }

    let mut result = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n.clone(),
        _ => return Err(format!("*: expected number, got {}", args[0])),
    };

    for arg in &args[1..] {
        let num = match arg {
            Value::Atom(AtomType::Number(n)) => n,
            _ => return Err(format!("*: expected number, got {}", arg)),
        };
        result = result.mul(num)?;
    }

    Ok(Value::Atom(AtomType::Number(result)))
}

/// Division (variadic: divides first by successive arguments)
pub fn div(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("/: expected at least 2 arguments".to_string());
    }

    let mut result = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n.clone(),
        _ => return Err(format!("/: expected number, got {}", args[0])),
    };

    for arg in &args[1..] {
        let num = match arg {
            Value::Atom(AtomType::Number(n)) => n,
            _ => return Err(format!("/: expected number, got {}", arg)),
        };
        result = result.div(num)?;
    }

    Ok(Value::Atom(AtomType::Number(result)))
}

// ============================================================================
// Comparison Operations (de-sugared from special forms)
// ============================================================================

/// Less than
pub fn lt(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("<: expected 2 arguments".to_string());
    }

    let num1 = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("<: expected number, got {}", args[0])),
    };

    let num2 = match &args[1] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("<: expected number, got {}", args[1])),
    };

    Ok(Value::Atom(AtomType::Bool(num1 < num2)))
}

/// Greater than
pub fn gt(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(">: expected 2 arguments".to_string());
    }

    let num1 = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!(">: expected number, got {}", args[0])),
    };

    let num2 = match &args[1] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!(">: expected number, got {}", args[1])),
    };

    Ok(Value::Atom(AtomType::Bool(num1 > num2)))
}

/// Less than or equal
pub fn lte(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("<=: expected 2 arguments".to_string());
    }

    let num1 = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("<=: expected number, got {}", args[0])),
    };

    let num2 = match &args[1] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("<=: expected number, got {}", args[1])),
    };

    Ok(Value::Atom(AtomType::Bool(num1 <= num2)))
}

/// Greater than or equal
pub fn gte(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err(">=: expected 2 arguments".to_string());
    }

    let num1 = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!(">=: expected number, got {}", args[0])),
    };

    let num2 = match &args[1] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!(">=: expected number, got {}", args[1])),
    };

    Ok(Value::Atom(AtomType::Bool(num1 >= num2)))
}

/// Numeric equality
pub fn num_eq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("=: expected 2 arguments".to_string());
    }

    let num1 = match &args[0] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("=: expected number, got {}", args[0])),
    };

    let num2 = match &args[1] {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("=: expected number, got {}", args[1])),
    };

    Ok(Value::Atom(AtomType::Bool(num1 == num2)))
}

// ============================================================================
// Vector Constructor (de-sugared from << >> syntax)
// ============================================================================

/// Construct a fast vector from arguments
pub fn vector(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Vector(Arc::new(crate::language::VectorValue {
        elements: args.to_vec(),
    })))
}

// ============================================================================
// Engine Abstractions (Clojure-inspired)
// ============================================================================

/// Sequence abstraction - return a seq over a collection
/// Usage: (%seq '(1 2 3)) => (1 2 3)
/// Usage: (%seq <<1 2 3>>) => (1 2 3)
pub fn builtin_seq(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%seq: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::seq(&args[0]).map_or(Value::Nil, |s| s.to_list()))
}

/// First element of a sequence
/// Usage: (%first '(1 2 3)) => 1
pub fn builtin_first(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%first: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::first(&args[0]))
}

/// Next elements of a sequence (rest, but returns nil for empty)
/// Usage: (%next '(1 2 3)) => (2 3)
pub fn builtin_next(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%next: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::next(&args[0]))
}

/// Rest of a sequence (like next but returns () for empty)
/// Usage: (%rest '(1 2 3)) => (2 3)
pub fn builtin_rest(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%rest: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::rest(&args[0]))
}

/// Count elements in a collection
/// Usage: (%count '(1 2 3)) => 3
pub fn builtin_count(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%count: expected 1 argument".to_string());
    }
    crate::abstractions::count(&args[0])
        .map(|n| Value::Atom(AtomType::Number(NumericType::Int(n as i64))))
        .ok_or_else(|| format!("%count: cannot count {}", args[0]))
}

/// Get nth element of a collection
/// Usage: (%nth <<1 2 3>> 1) => 2
/// Usage: (%nth <<1 2 3>> 5 :default) => :default
pub fn builtin_nth(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("%nth: expected 2-3 arguments (coll, index, [default])".to_string());
    }
    let index = match &args[1] {
        Value::Atom(AtomType::Number(NumericType::Int(n))) if *n >= 0 => *n as usize,
        _ => return Err("%nth: index must be a non-negative integer".to_string()),
    };
    let default = args.get(2);
    Ok(crate::abstractions::nth(&args[0], index, default))
}

/// Get value by key from collection
/// Usage: (%get {:a 1 :b 2} :a) => 1
/// Usage: (%get <<1 2 3>> 0) => 1
pub fn builtin_get(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 || args.len() > 3 {
        return Err("%get: expected 2-3 arguments (coll, key, [default])".to_string());
    }
    let default = args.get(2);
    Ok(crate::abstractions::get(&args[0], &args[1], default))
}

/// Associate a key with a value in a collection
/// Usage: (%assoc {:a 1} :b 2) => {:a 1 :b 2}
/// Usage: (%assoc <<1 2 3>> 0 10) => <<10 2 3>>
pub fn builtin_assoc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 3 || args.len().is_multiple_of(2) {
        return Err(
            "%assoc: expected odd number of arguments >= 3 (coll, key, val, ...)".to_string(),
        );
    }
    let mut result = args[0].clone();
    for chunk in args[1..].chunks(2) {
        result = crate::abstractions::assoc(&result, chunk[0].clone(), chunk[1].clone())?;
    }
    Ok(result)
}

/// Add item(s) to a collection
/// Usage: (%conj '(2 3) 1) => (1 2 3)
/// Usage: (%conj <<1 2>> 3) => <<1 2 3>>
pub fn builtin_conj(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("%conj: expected at least 2 arguments (coll, item, ...)".to_string());
    }
    let mut result = args[0].clone();
    for item in &args[1..] {
        result = crate::abstractions::conj(&result, item.clone())?;
    }
    Ok(result)
}

/// Wrap a value in Reduced for early termination
/// Usage: (%reduced 42) => #reduced(42)
pub fn builtin_reduced(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%reduced: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::reduced(args[0].clone()))
}

/// Check if a value is reduced
/// Usage: (%reduced? #reduced(42)) => t
pub fn builtin_reduced_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%reduced?: expected 1 argument".to_string());
    }
    Ok(Value::Atom(AtomType::Bool(
        crate::abstractions::is_reduced(&args[0]),
    )))
}

/// Unwrap a reduced value
/// Usage: (%unreduced #reduced(42)) => 42
pub fn builtin_unreduced(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%unreduced: expected 1 argument".to_string());
    }
    Ok(crate::abstractions::unreduced(&args[0]))
}

/// Create a hash map from key-value pairs
/// Usage: (%hash-map :a 1 :b 2) => {:a 1, :b 2}
pub fn builtin_hash_map(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if !args.len().is_multiple_of(2) {
        return Err("%hash-map: expected even number of arguments (key-value pairs)".to_string());
    }
    let pairs: Vec<(Value, Value)> = args
        .chunks(2)
        .map(|chunk| (chunk[0].clone(), chunk[1].clone()))
        .collect();
    Ok(crate::abstractions::hash_map(pairs))
}

/// Create a hash set from elements
/// Usage: (%hash-set 1 2 3) => #{1 2 3}
pub fn builtin_hash_set(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(crate::abstractions::hash_set(args.to_vec()))
}

/// Check if a value is empty
/// Usage: (%empty? '()) => t
/// Usage: (%empty? <<>>) => t
pub fn builtin_empty_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%empty?: expected 1 argument".to_string());
    }
    let is_empty = match &args[0] {
        Value::Nil => true,
        Value::Cons(_) => false,
        Value::Vector(v) => v.elements.is_empty(),
        Value::Map(m) => m.entries.is_empty(),
        Value::Set(s) => s.elements.is_empty(),
        Value::Atom(AtomType::String(StringType::Basic(s))) => s.is_empty(),
        _ => false,
    };
    Ok(Value::Atom(AtomType::Bool(is_empty)))
}

/// Check if a value contains a key/element
/// Usage: (%contains? {:a 1} :a) => t
/// Usage: (%contains? #{1 2 3} 2) => t
pub fn builtin_contains_p(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 2 {
        return Err("%contains?: expected 2 arguments (coll, key)".to_string());
    }
    let contains = match &args[0] {
        Value::Map(m) => m.entries.contains_key(&args[1]),
        Value::Set(s) => s.elements.contains(&args[1]),
        Value::Vector(v) => {
            if let Value::Atom(AtomType::Number(NumericType::Int(idx))) = &args[1] {
                *idx >= 0 && (*idx as usize) < v.elements.len()
            } else {
                false
            }
        }
        _ => false,
    };
    Ok(Value::Atom(AtomType::Bool(contains)))
}

/// Get keys from a map
/// Usage: (%keys {:a 1 :b 2}) => (:a :b)
pub fn builtin_keys(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%keys: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let mut result = Value::Nil;
            for k in m.entries.keys() {
                result = crate::language::cons(k.clone(), result);
            }
            Ok(result)
        }
        _ => Err(format!("%keys: expected map, got {}", args[0])),
    }
}

/// Get values from a map
/// Usage: (%vals {:a 1 :b 2}) => (1 2)
pub fn builtin_vals(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() != 1 {
        return Err("%vals: expected 1 argument".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let mut result = Value::Nil;
            for v in m.entries.values() {
                result = crate::language::cons(v.clone(), result);
            }
            Ok(result)
        }
        _ => Err(format!("%vals: expected map, got {}", args[0])),
    }
}

/// Remove a key from a map or element from a set
/// Usage: (%dissoc {:a 1 :b 2} :a) => {:b 2}
/// Usage: (%disj #{1 2 3} 2) => #{1 3}
#[allow(clippy::mutable_key_type)]
pub fn builtin_dissoc(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("%dissoc: expected at least 2 arguments (map, key, ...)".to_string());
    }
    match &args[0] {
        Value::Map(m) => {
            let mut entries = m.entries.clone();
            for key in &args[1..] {
                entries.remove(key);
            }
            Ok(Value::Map(Arc::new(crate::language::MapValue { entries })))
        }
        _ => Err(format!("%dissoc: expected map, got {}", args[0])),
    }
}

/// Remove an element from a set
/// Usage: (%disj #{1 2 3} 2) => #{1 3}
#[allow(clippy::mutable_key_type)]
pub fn builtin_disj(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    if args.len() < 2 {
        return Err("%disj: expected at least 2 arguments (set, elem, ...)".to_string());
    }
    match &args[0] {
        Value::Set(s) => {
            let mut elements = s.elements.clone();
            for elem in &args[1..] {
                elements.remove(elem);
            }
            Ok(Value::Set(Arc::new(crate::language::SetValue { elements })))
        }
        _ => Err(format!("%disj: expected set, got {}", args[0])),
    }
}

// ============================================================================
// Registration
// ============================================================================

/// Register all standard library functions in the given environment
pub fn register_stdlib(env: &mut Environment) {
    // Standard I/O
    env.define("print".to_string(), Value::NativeFn(print));
    env.define("println".to_string(), Value::NativeFn(println));

    // File I/O
    env.define("slurp".to_string(), Value::NativeFn(slurp));
    env.define("spit".to_string(), Value::NativeFn(spit));

    // Process execution
    env.define("shell".to_string(), Value::NativeFn(shell));

    // Time
    env.define("now".to_string(), Value::NativeFn(now));

    // Macro support
    env.define("gensym".to_string(), Value::NativeFn(gensym));
    env.define("macroexpand-1".to_string(), Value::NativeFn(macroexpand_1));
    env.define("macroexpand".to_string(), Value::NativeFn(macroexpand));

    // List operations (de-sugaring special forms)
    env.define("atom".to_string(), Value::NativeFn(atom));
    env.define("eq".to_string(), Value::NativeFn(eq));
    env.define("car".to_string(), Value::NativeFn(car));
    env.define("cdr".to_string(), Value::NativeFn(cdr));
    env.define("cons".to_string(), Value::NativeFn(cons_fn));

    // Type predicates (for JIT/AOT parity)
    env.define("nil?".to_string(), Value::NativeFn(nil_p));
    env.define("cons?".to_string(), Value::NativeFn(cons_p));
    env.define("number?".to_string(), Value::NativeFn(number_p));
    env.define("not".to_string(), Value::NativeFn(not_fn));

    // List operations (for JIT/AOT parity)
    env.define("length".to_string(), Value::NativeFn(length));
    env.define("append".to_string(), Value::NativeFn(append));
    env.define("reverse".to_string(), Value::NativeFn(reverse));
    env.define("list".to_string(), Value::NativeFn(list));
    env.define("nth".to_string(), Value::NativeFn(nth));

    // Vector operations (for JIT/AOT parity)
    env.define("vector-length".to_string(), Value::NativeFn(vector_length));
    env.define("vector-ref".to_string(), Value::NativeFn(vector_ref));

    // Arithmetic operations (de-sugaring special forms)
    env.define("+".to_string(), Value::NativeFn(add));
    env.define("-".to_string(), Value::NativeFn(sub));
    env.define("*".to_string(), Value::NativeFn(mul));
    env.define("/".to_string(), Value::NativeFn(div));

    // Comparison operations (de-sugaring special forms)
    env.define("<".to_string(), Value::NativeFn(lt));
    env.define(">".to_string(), Value::NativeFn(gt));
    env.define("<=".to_string(), Value::NativeFn(lte));
    env.define(">=".to_string(), Value::NativeFn(gte));
    env.define("=".to_string(), Value::NativeFn(num_eq));

    // Vector constructor (de-sugaring vector syntax)
    env.define("vector".to_string(), Value::NativeFn(vector));

    // Engine abstractions (Clojure-inspired)
    env.define("%seq".to_string(), Value::NativeFn(builtin_seq));
    env.define("%first".to_string(), Value::NativeFn(builtin_first));
    env.define("%next".to_string(), Value::NativeFn(builtin_next));
    env.define("%rest".to_string(), Value::NativeFn(builtin_rest));
    env.define("%count".to_string(), Value::NativeFn(builtin_count));
    env.define("%nth".to_string(), Value::NativeFn(builtin_nth));
    env.define("%get".to_string(), Value::NativeFn(builtin_get));
    env.define("%assoc".to_string(), Value::NativeFn(builtin_assoc));
    env.define("%conj".to_string(), Value::NativeFn(builtin_conj));
    env.define("%reduced".to_string(), Value::NativeFn(builtin_reduced));
    env.define("%reduced?".to_string(), Value::NativeFn(builtin_reduced_p));
    env.define("%unreduced".to_string(), Value::NativeFn(builtin_unreduced));
    env.define("%hash-map".to_string(), Value::NativeFn(builtin_hash_map));
    env.define("%hash-set".to_string(), Value::NativeFn(builtin_hash_set));
    env.define("%empty?".to_string(), Value::NativeFn(builtin_empty_p));
    env.define(
        "%contains?".to_string(),
        Value::NativeFn(builtin_contains_p),
    );
    env.define("%keys".to_string(), Value::NativeFn(builtin_keys));
    env.define("%vals".to_string(), Value::NativeFn(builtin_vals));
    env.define("%dissoc".to_string(), Value::NativeFn(builtin_dissoc));
    env.define("%disj".to_string(), Value::NativeFn(builtin_disj));
}
