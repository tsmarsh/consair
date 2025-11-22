use consair::{Environment, eval, parse};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process;

fn repl() {
    let mut env = Environment::new();

    println!("Minimal Lisp REPL");
    println!("Type expressions to evaluate, or (exit) to quit");
    println!();

    loop {
        print!("> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        if io::stdin().read_line(&mut input).is_err() {
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        // Check for exit
        if input == "(exit)" || input == "exit" {
            break;
        }

        match parse(input) {
            Ok(expr) => match eval(expr, &mut env) {
                Ok(result) => println!("{result}"),
                Err(e) => eprintln!("Error: {e}"),
            },
            Err(e) => eprintln!("Parse error: {e}"),
        }
    }
}

fn run_file(filename: &str) -> Result<(), String> {
    let contents = fs::read_to_string(filename)
        .map_err(|e| format!("Failed to read file '{filename}': {e}"))?;

    let mut env = Environment::new();
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
    let trimmed = input.trim_start();
    if trimmed.is_empty() {
        return Err("No expression found".to_string());
    }

    // Find the end of the expression
    let mut depth = 0;
    let in_string = false;
    let chars = trimmed.chars().enumerate();
    let mut end_pos = 0;

    // Handle atoms (non-list/non-vector expressions)
    if !trimmed.starts_with('(') && !trimmed.starts_with('\'') && !trimmed.starts_with('<') {
        // Find the end of the atom (whitespace or paren)
        for (i, ch) in chars {
            if ch.is_whitespace() || ch == '(' || ch == ')' || ch == '<' || ch == '>' {
                end_pos = i;
                break;
            }
            end_pos = i + 1;
        }
        if end_pos == 0 {
            end_pos = trimmed.len();
        }
    } else {
        // Handle lists, vectors, and quoted expressions
        let mut vec_depth = 0;
        let mut chars_iter = trimmed.chars().enumerate().peekable();

        while let Some((i, ch)) = chars_iter.next() {
            match ch {
                '(' if !in_string => depth += 1,
                ')' if !in_string => {
                    depth -= 1;
                    if depth == 0 && vec_depth == 0 {
                        end_pos = i + 1;
                        break;
                    }
                }
                '<' if !in_string => {
                    if let Some(&(_, '<')) = chars_iter.peek() {
                        vec_depth += 1;
                        chars_iter.next(); // consume second <
                    }
                }
                '>' if !in_string => {
                    if let Some(&(_, '>')) = chars_iter.peek() {
                        vec_depth -= 1;
                        chars_iter.next(); // consume second >
                        if vec_depth == 0 && depth == 0 {
                            end_pos = i + 2;
                            break;
                        }
                    }
                }
                '\'' if !in_string => {
                    // Quote followed by expression
                    continue;
                }
                _ => {}
            }
        }
    }

    if end_pos == 0 {
        return Err("Incomplete expression".to_string());
    }

    let expr_str = &trimmed[..end_pos];
    let rest = &trimmed[end_pos..];

    parse(expr_str).map(|expr| (expr, rest))
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
