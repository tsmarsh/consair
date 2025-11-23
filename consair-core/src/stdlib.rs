//! Standard library native functions
//!
//! This module provides the core native functions that are available
//! in the Consair Lisp environment.

use std::fs;
use std::io::{self, Write};
use std::process::Command;
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
}
