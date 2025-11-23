use consair::parse;
use std::sync::Arc;
use std::thread;

#[test]
fn test_value_is_send_sync() {
    // Parse a value
    let value = parse("(quote (hello from multiple threads))").unwrap();

    // Wrap in Arc to share across threads
    let value_arc = Arc::new(value);

    let mut handles = vec![];

    // Spawn 5 threads that all read the same value
    for i in 0..5 {
        let value_clone = Arc::clone(&value_arc);
        let handle = thread::spawn(move || {
            // Each thread can read the value
            let _ = format!("Thread {}: {}", i, value_clone);
        });
        handles.push(handle);
    }

    // Wait for all threads to complete
    for handle in handles {
        handle.join().unwrap();
    }

    // If we got here, Value is Send + Sync!
}

#[test]
fn test_value_can_be_cloned_across_threads() {
    let value = parse("'(1 2 3)").unwrap();

    let handle1 = thread::spawn(move || {
        let cloned = value.clone();
        format!("{}", cloned)
    });

    let result = handle1.join().unwrap();
    assert_eq!(result, "(quote (1 2 3))");
}
