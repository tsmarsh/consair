use consair::{Environment, eval, parse, register_stdlib};
use rustyline::error::ReadlineError;
use rustyline::{Config, Editor};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

/// Check if an expression has balanced parentheses and is complete
fn is_complete_expression(input: &str) -> bool {
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for ch in input.chars() {
        if in_string {
            if escape_next {
                escape_next = false;
                continue;
            }
            if ch == '\\' {
                escape_next = true;
                continue;
            }
            if ch == '"' {
                in_string = false;
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => depth -= 1,
            _ => {}
        }
    }

    depth == 0 && !in_string
}

/// Print help information
fn print_help() {
    println!("Consair REPL - Interactive Lisp Interpreter");
    println!();
    println!("Special Commands:");
    println!("  :help, :h        Show this help message");
    println!("  :quit, :q        Exit the REPL");
    println!("  :env             Show current environment bindings");
    println!();
    println!("Keyboard Shortcuts:");
    println!("  Ctrl-C           Clear current input");
    println!("  Ctrl-D           Exit REPL");
    println!("  Up/Down          Navigate command history");
    println!("  Ctrl-R           Reverse history search");
    println!();
    println!("Multi-line Input:");
    println!("  If you have unclosed parentheses, press Enter to continue");
    println!("  on the next line. The prompt will change to '......> '");
    println!();
    println!("Examples:");
    println!("  (+ 1 2 3)");
    println!("  (label square (lambda (x) (* x x)))");
    println!("  (square 5)");
}

/// Show environment bindings (simplified - just shows that env exists)
fn print_env_info(env: &Environment) {
    println!("Environment is active with standard library loaded.");
    println!("Use (quote env-name) to inspect specific bindings.");
    // Note: Full environment introspection would require adding methods to Environment
    // For now, we just acknowledge it exists
    let _ = env; // Suppress unused warning
}

fn repl() {
    let mut env = Environment::new();
    register_stdlib(&mut env);

    // Configure rustyline
    let config = Config::builder()
        .auto_add_history(true)
        .history_ignore_space(true)
        .build();

    let mut rl = Editor::<(), _>::with_config(config).unwrap();

    // Set up history file
    let history_file = dirs::home_dir()
        .map(|h| h.join(".consair_history"))
        .unwrap_or_else(|| PathBuf::from(".consair_history"));

    // Load history
    if rl.load_history(&history_file).is_ok() {
        // History loaded successfully (silent)
    }

    // Welcome message
    println!("Consair Lisp REPL v{}", env!("CARGO_PKG_VERSION"));
    println!("Type :help for help, :quit to exit");
    println!();

    let mut accumulated_input = String::new();

    loop {
        let prompt = if accumulated_input.is_empty() {
            "consair> "
        } else {
            "......> "
        };

        match rl.readline(prompt) {
            Ok(line) => {
                // Add to accumulated input
                if !accumulated_input.is_empty() {
                    accumulated_input.push('\n');
                }
                accumulated_input.push_str(&line);

                let trimmed = accumulated_input.trim();

                // Skip empty input
                if trimmed.is_empty() {
                    accumulated_input.clear();
                    continue;
                }

                // Check for special commands (only at start of input)
                if accumulated_input.lines().count() == 1 {
                    match trimmed {
                        ":help" | ":h" => {
                            print_help();
                            accumulated_input.clear();
                            continue;
                        }
                        ":quit" | ":q" => {
                            break;
                        }
                        ":env" => {
                            print_env_info(&env);
                            accumulated_input.clear();
                            continue;
                        }
                        _ => {}
                    }
                }

                // Check for traditional exit command
                if trimmed == "(exit)" || trimmed == "exit" {
                    break;
                }

                // Check if expression is complete
                if !is_complete_expression(&accumulated_input) {
                    // Continue accumulating input
                    continue;
                }

                // Try to parse and evaluate
                match parse(&accumulated_input) {
                    Ok(expr) => match eval(expr, &mut env) {
                        Ok(result) => println!("{result}"),
                        Err(e) => eprintln!("⚠ Error: {e}"),
                    },
                    Err(e) => eprintln!("⚠ Parse error: {e}"),
                }

                accumulated_input.clear();
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: Clear current input
                if !accumulated_input.is_empty() {
                    println!("^C");
                    accumulated_input.clear();
                } else {
                    println!("^C");
                    println!("(Press Ctrl-D or type :quit to exit)");
                }
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D: Exit
                println!();
                break;
            }
            Err(err) => {
                eprintln!("Error: {err:?}");
                break;
            }
        }
    }

    // Save history on exit
    if let Err(e) = rl.save_history(&history_file) {
        eprintln!("Warning: Failed to save history: {e}");
    }
}

fn run_file(filename: &str) -> Result<(), String> {
    let contents = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{filename}': {e}"))?;

    let mut env = Environment::new();
    register_stdlib(&mut env);
    let mut last_result = None;

    // Split the file into expressions and evaluate each one
    // Simple approach: parse and evaluate complete s-expressions
    let trimmed = contents.trim();
    if trimmed.is_empty() {
        return Ok(());
    }

    // Try to parse the entire content as a sequence of expressions
    let mut remaining = trimmed;
    while !remaining.trim().is_empty() {
        // Find the next complete s-expression
        let expr_result = parse_next_expr(remaining)?;
        let (expr, rest) = expr_result;

        match eval(expr, &mut env) {
            Ok(result) => last_result = Some(result),
            Err(e) => return Err(format!("Evaluation error: {e}")),
        }

        remaining = rest;
    }

    // Print the last result
    if let Some(result) = last_result {
        println!("{result}");
    }

    Ok(())
}

// Helper function to parse the next expression from a string
fn parse_next_expr(input: &str) -> Result<(consair::Value, &str), String> {
    // Skip leading whitespace and comments to find the next expression start
    let trimmed = skip_whitespace_and_comments(input);
    if trimmed.is_empty() {
        return Err("No expression found".to_string());
    }

    // Find the end of the expression
    let mut depth = 0;
    let mut vec_depth = 0;
    let mut in_string = false;
    let mut in_raw_string = false;
    let mut raw_hash_count = 0;
    let mut escape_next = false;
    let mut end_pos = 0;

    let chars_vec: Vec<char> = trimmed.chars().collect();
    let mut i = 0;

    // Handle atoms (non-list/non-vector expressions that don't start with special chars)
    if !trimmed.starts_with('(')
        && !trimmed.starts_with('\'')
        && !trimmed.starts_with('<')
        && !trimmed.starts_with('"')
    {
        // Find the end of the atom (whitespace or delimiter)
        while i < chars_vec.len() {
            let ch = chars_vec[i];
            if ch.is_whitespace() || ch == '(' || ch == ')' || ch == '<' || ch == '>' || ch == ';' {
                end_pos = i;
                break;
            }
            i += 1;
        }
        if end_pos == 0 {
            end_pos = trimmed.len();
        }
    } else {
        // Handle complex expressions (lists, vectors, strings, quoted expressions)
        while i < chars_vec.len() {
            let ch = chars_vec[i];

            // Handle escape sequences in strings
            if in_string && !in_raw_string {
                if escape_next {
                    escape_next = false;
                    i += 1;
                    continue;
                }
                if ch == '\\' {
                    escape_next = true;
                    i += 1;
                    continue;
                }
                if ch == '"' {
                    in_string = false;
                    i += 1;
                    continue;
                }
                i += 1;
                continue;
            }

            // Handle raw strings
            if in_raw_string {
                if ch == '"' && i + raw_hash_count < chars_vec.len() {
                    // Check if followed by correct number of #
                    let mut hash_match = true;
                    for j in 1..=raw_hash_count {
                        if i + j >= chars_vec.len() || chars_vec[i + j] != '#' {
                            hash_match = false;
                            break;
                        }
                    }
                    if hash_match {
                        in_raw_string = false;
                        i += raw_hash_count + 1;
                        // If we're at top level, we're done
                        if depth == 0 && vec_depth == 0 {
                            end_pos = i;
                            break;
                        }
                        continue;
                    }
                }
                i += 1;
                continue;
            }

            // Handle regular parsing
            match ch {
                // Raw string detection: r" or r#"
                'r' if !in_string && i + 1 < chars_vec.len() => {
                    let mut j = i + 1;
                    let mut hashes = 0;
                    while j < chars_vec.len() && chars_vec[j] == '#' {
                        hashes += 1;
                        j += 1;
                    }
                    if j < chars_vec.len() && chars_vec[j] == '"' {
                        in_raw_string = true;
                        raw_hash_count = hashes;
                        i = j + 1;
                        continue;
                    }
                    i += 1;
                }
                '"' if !in_string => {
                    in_string = true;
                    i += 1;
                }
                '(' if !in_string => {
                    depth += 1;
                    i += 1;
                }
                ')' if !in_string => {
                    depth -= 1;
                    if depth == 0 && vec_depth == 0 {
                        end_pos = i + 1;
                        break;
                    }
                    if depth < 0 {
                        return Err("Unmatched closing parenthesis".to_string());
                    }
                    i += 1;
                }
                '<' if !in_string && i + 1 < chars_vec.len() && chars_vec[i + 1] == '<' => {
                    vec_depth += 1;
                    i += 2;
                }
                '>' if !in_string && i + 1 < chars_vec.len() && chars_vec[i + 1] == '>' => {
                    vec_depth -= 1;
                    if vec_depth == 0 && depth == 0 {
                        end_pos = i + 2;
                        break;
                    }
                    if vec_depth < 0 {
                        return Err("Unmatched closing vector delimiter".to_string());
                    }
                    i += 2;
                }
                '\'' if !in_string && depth == 0 && vec_depth == 0 => {
                    // Quote at top level - the quoted expression is the complete expression
                    // Need to find the end of the quoted expression
                    i += 1;
                    let mut quote_depth = 0;
                    let mut quote_vec_depth = 0;
                    let mut quote_in_string = false;
                    let mut quote_escape = false;

                    while i < chars_vec.len() {
                        let qch = chars_vec[i];
                        if quote_in_string {
                            if quote_escape {
                                quote_escape = false;
                            } else if qch == '\\' {
                                quote_escape = true;
                            } else if qch == '"' {
                                quote_in_string = false;
                            }
                            i += 1;
                            continue;
                        }

                        match qch {
                            '"' => quote_in_string = true,
                            '(' => quote_depth += 1,
                            ')' => {
                                quote_depth -= 1;
                                if quote_depth == 0 && quote_vec_depth == 0 {
                                    end_pos = i + 1;
                                    break;
                                }
                            }
                            '<' if i + 1 < chars_vec.len() && chars_vec[i + 1] == '<' => {
                                quote_vec_depth += 1;
                                i += 1;
                            }
                            '>' if i + 1 < chars_vec.len() && chars_vec[i + 1] == '>' => {
                                quote_vec_depth -= 1;
                                if quote_vec_depth == 0 && quote_depth == 0 {
                                    end_pos = i + 2;
                                    break;
                                }
                                i += 1;
                            }
                            c if c.is_whitespace() && quote_depth == 0 && quote_vec_depth == 0 => {
                                end_pos = i;
                                break;
                            }
                            _ => {}
                        }
                        i += 1;
                    }
                    if end_pos == 0 {
                        end_pos = chars_vec.len();
                    }
                    break;
                }
                _ => {
                    i += 1;
                }
            }
        }
    }

    if end_pos == 0 {
        if depth > 0 {
            return Err("Unclosed opening parenthesis".to_string());
        } else if vec_depth > 0 {
            return Err("Unclosed vector delimiter".to_string());
        } else if in_string {
            return Err("Unclosed string literal".to_string());
        }
        return Err("Incomplete expression".to_string());
    }

    let expr_str = &trimmed[..end_pos];
    let rest = &trimmed[end_pos..];

    parse(expr_str).map(|expr| (expr, rest))
}

// Helper function to skip whitespace and comments between expressions
// Note: Comments WITHIN expressions are now handled natively by the lexer
// This function is only needed to skip comments BETWEEN top-level expressions
fn skip_whitespace_and_comments(input: &str) -> &str {
    let mut remaining = input;
    loop {
        // Skip whitespace
        remaining = remaining.trim_start();
        if remaining.is_empty() {
            return remaining;
        }

        // Skip comments (from ; to end of line)
        if remaining.starts_with(';') {
            if let Some(newline_pos) = remaining.find('\n') {
                remaining = &remaining[newline_pos + 1..];
            } else {
                // Comment to end of file
                return "";
            }
        } else {
            // No more comments or whitespace
            break;
        }
    }
    remaining
}

fn print_usage() {
    eprintln!("Usage:");
    eprintln!("  cons              Start interactive REPL");
    eprintln!("  cons <file.lisp>  Run a Lisp file");
    eprintln!("  cons --help       Show this help message");
}

fn main() {
    let args: Vec<String> = env::args().collect();

    match args.len() {
        1 => {
            // No arguments: start REPL
            repl();
        }
        2 => {
            let arg = &args[1];
            if arg == "--help" || arg == "-h" {
                print_usage();
            } else {
                // Run file
                if let Err(e) = run_file(arg) {
                    eprintln!("{e}");
                    process::exit(1);
                }
            }
        }
        _ => {
            eprintln!("Error: Too many arguments");
            print_usage();
            process::exit(1);
        }
    }
}
