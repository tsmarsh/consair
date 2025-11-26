//! cadr - AOT compiler for Consair Lisp
//!
//! Compiles Consair Lisp source files to LLVM IR.
//!
//! # Usage
//!
//! ```bash
//! # Compile to LLVM IR file
//! cadr input.lisp -o output.ll
//!
//! # Output to stdout
//! cadr input.lisp
//!
//! # Then compile to native with clang
//! clang -O3 output.ll -o output
//! ```

use std::env;
use std::path::Path;
use std::process;

use cadr::aot::AotCompiler;

fn print_usage() {
    eprintln!("cadr - AOT compiler for Consair Lisp");
    eprintln!();
    eprintln!("Usage:");
    eprintln!("  cadr <input.lisp>              Compile to LLVM IR (stdout)");
    eprintln!("  cadr <input.lisp> -o <out.ll>  Compile to LLVM IR file");
    eprintln!("  cadr --help                    Show this help");
    eprintln!("  cadr --version                 Show version");
    eprintln!();
    eprintln!("Examples:");
    eprintln!("  cadr factorial.lisp -o factorial.ll");
    eprintln!("  clang -O3 factorial.ll -o factorial");
}

fn print_version() {
    eprintln!("cadr {}", env!("CARGO_PKG_VERSION"));
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(1);
    }

    // Handle flags
    match args[1].as_str() {
        "--help" | "-h" => {
            print_usage();
            return;
        }
        "--version" | "-V" => {
            print_version();
            return;
        }
        _ => {}
    }

    let input = &args[1];

    // Check for -o flag
    let output = if args.len() >= 4 && args[2] == "-o" {
        Some(args[3].as_str())
    } else {
        None
    };

    // Compile
    let compiler = AotCompiler::new();
    let input_path = Path::new(input);

    if !input_path.exists() {
        eprintln!("Error: File not found: {}", input);
        process::exit(1);
    }

    match compiler.compile_file(input_path, output.map(Path::new)) {
        Ok(()) => {
            if let Some(out) = output {
                eprintln!("Compiled {} to {}", input, out);
            }
        }
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}
