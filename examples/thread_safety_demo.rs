//! Demo showing that Value is Send + Sync and can be shared across threads
//!
//! Run with: cargo run --example thread_safety_demo

use consair_core::{parse, Value};
use std::sync::Arc;
use std::thread;

fn main() {
    println!("=== Thread Safety Demo ===\n");

    // Parse a value
    let value = parse("(quote (hello from multiple threads))").unwrap();
    println!("Original value: {}\n", value);

    // Wrap in Arc to share across threads
    let value_arc = Arc::new(value);

    let mut handles = vec![];

    // Spawn 5 threads that all read the same value
    println!("Spawning 5 threads...");
    for i in 0..5 {
        let value_clone = Arc::clone(&value_arc);
        let handle = thread::spawn(move || {
            // Each thread can read the value
            println!("  Thread {}: {}", i, value_clone);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    println!("\n✅ Successfully shared Value across 5 threads!");
    println!("✅ Value implements Send + Sync (thread-safe)");
}
