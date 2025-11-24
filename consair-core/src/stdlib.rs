//! Standard library native functions
//!
//! This module provides the core native functions that are available
//! in the Consair Lisp environment.

use std::fs;
use std::io::{self, Write};
use std::process::Command;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::interpreter::Environment;
use crate::language::{AtomType, SymbolType, Value};
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
        Value::Atom(AtomType::String(crate::language::StringType::Raw { content, .. })) => {
            content.clone()
        }
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

    // Build association list: ((:out . "...") (:err . "...") (:exit . 0) (:success . t))
    let result_pairs = vec![
        (
            Value::Atom(AtomType::Symbol(SymbolType::keyword("out"))),
            make_string(stdout_str),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::keyword("err"))),
            make_string(stderr_str),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::keyword("exit"))),
            make_int(exit_code),
        ),
        (
            Value::Atom(AtomType::Symbol(SymbolType::keyword("success"))),
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

/// Construct a vector from arguments
pub fn vector(args: &[Value], _env: &mut Environment) -> Result<Value, String> {
    Ok(Value::Vector(Arc::new(crate::language::VectorValue {
        elements: args.to_vec(),
    })))
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
}
