use consair::interpreter::Environment;
use consair::language::{AtomType, Value};
use consair::numeric::NumericType;
use std::time::Instant;

fn bench_define_operations(n: usize) -> std::time::Duration {
    let start = Instant::now();

    let mut env = Environment::new();

    for i in 0..n {
        env.define(
            format!("var{i}"),
            Value::Atom(AtomType::Number(NumericType::Int(i as i64))),
        );
    }

    start.elapsed()
}

fn main() {
    println!("Environment::define() Performance Benchmark");
    println!("==========================================\n");

    let test_sizes = vec![10, 100, 1000, 10000];

    for size in test_sizes {
        let duration = bench_define_operations(size);
        let per_op = duration.as_nanos() / size as u128;

        println!("{size:5} definitions: {duration:?} ({per_op} ns/op)");
    }

    println!("\nNote: With Arc::make_mut() optimization:");
    println!("- Cloning only happens when Arc has multiple strong references");
    println!("- Common case (single environment) has O(1) amortized insert cost");
    println!("- Previous implementation cloned entire HashMap on every define()");
}
