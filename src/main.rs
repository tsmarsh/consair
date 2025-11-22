use consair::{Environment, eval, parse};
use std::io::{self, Write};

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

fn main() {
    repl();
}
