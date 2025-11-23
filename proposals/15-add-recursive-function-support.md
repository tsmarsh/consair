# Proposal 015: Recursive Function Support (Implementation Spec)

* **Status:** Ready for Implementation
* **Date:** 2025-11-23
* **Target:** Small LLM / Coding Agent
* **Priority:** Critical

## Objective
Enable recursive functions (e.g., factorial) by refactoring the `Environment` struct to use **Interior Mutability**. This ensures that when a function name is bound via `label`, the `Lambda` (which has already captured the environment) sees the new binding immediately.

## 1. Architecture Refactor: Shared Environment

**Current Issue:** `Environment` is likely a simple struct. When passed to a Lambda, it is cloned (creating a snapshot). Updates to the original environment (like defining the function name) are not reflected inside the Lambda's snapshot.

**Required Change:** Wrap the inner state in `Arc<RwLock<...>>` so all clones point to the same storage.

### File: `consair-core/src/interpreter.rs` (or `environment.rs`)

Replace the existing `Environment` struct definition with:

```rust
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use crate::language::Value;

// Internal state holding the data and parent pointer
struct EnvironmentState {
    data: HashMap<String, Value>,
    parent: Option<Arc<Environment>>, // Point to the wrapper, not the state
}

// The public wrapper that is cheap to clone
#[derive(Clone)]
pub struct Environment {
    state: Arc<RwLock<EnvironmentState>>,
}

impl Environment {
    /// Create a new, empty global environment
    pub fn new() -> Self {
        Environment {
            state: Arc::new(RwLock::new(EnvironmentState {
                data: HashMap::new(),
                parent: None,
            })),
        }
    }

    /// Create a child environment extending the current one
    pub fn extend(&self, params: &[String], args: &[Value]) -> Self {
        let mut data = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            data.insert(param.clone(), arg.clone());
        }

        Environment {
            state: Arc::new(RwLock::new(EnvironmentState {
                data,
                // The child holds a reference to the parent's wrapper
                parent: Some(Arc::new(self.clone())), 
            })),
        }
    }

    /// Define a variable in the CURRENT scope (mutating the shared state)
    pub fn define(&self, name: String, value: Value) {
        let mut state = self.state.write().unwrap();
        state.data.insert(name, value);
    }

    /// Look up a variable, walking up the parent chain
    pub fn lookup(&self, name: &str) -> Option<Value> {
        let state = self.state.read().unwrap();
        
        if let Some(val) = state.data.get(name) {
            return Some(val.clone());
        }

        // Recursive lookup in parent
        match &state.parent {
            Some(parent) => parent.lookup(name),
            None => None,
        }
    }
}
````

## 2\. Logic Update: `label` Special Form

Update the `eval` function to rely on this shared state.

### File: `consair-core/src/interpreter.rs` (inside `eval` function)

Locate the `match` arm for `"label"`. Replace it with:

```rust
"label" => {
    // 1. Parse arguments
    // Expected syntax: (label name (lambda ...))
    let name_expr = car(&cell.cdr)?;
    let value_expr = car(&cdr(&cell.cdr)?)?;

    let name = match name_expr {
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) => s.resolve(),
        _ => return Err("label expects a symbol as the first argument".to_string()),
    };

    // 2. Evaluate the value (usually a Lambda)
    // We pass 'env.clone()'. Because Env uses Arc<RwLock>, this captures 
    // a pointer to the SAME environment storage we are holding.
    let value = eval(value_expr, env.clone())?;

    // 3. Define the name in the environment
    // Because of the shared pointer, the 'value' (if it is a Lambda) 
    // will immediately be able to see this new definition.
    env.define(name, value.clone());

    Ok(value)
}
```

## 3\. Implementation Verification (Tests)

Add the following tests to verify recursion works.

### File: `consair-core/tests/integration_tests.rs`

```rust
#[test]
fn test_recursive_factorial() {
    let code = r#"
        (label factorial (lambda (n)
            (cond 
                ((= n 0) 1)
                (t (* n (factorial (- n 1)))))))
        (factorial 5)
    "#;
    let result = run_code(code).unwrap(); // Assuming helper 'run_code' exists
    assert_eq!(result, Value::Atom(AtomType::Number(120)));
}

#[test]
fn test_mutually_recursive_functions() {
    let code = r#"
        (label is-even (lambda (n)
            (cond ((= n 0) t) (t (is-odd (- n 1))))))
        
        (label is-odd (lambda (n)
            (cond ((= n 0) nil) (t (is-even (- n 1))))))
            
        (cons (is-even 4) (is-odd 4))
    "#;
    // Expect: (t . nil) or list (t nil) depending on implementation
    let result_str = format!("{}", run_code(code).unwrap());
    assert!(result_str.contains("t")); 
    assert!(result_str.contains("nil"));
}
```

## 4\. Checklist for LLM Agent

1.  [ ] **Modify Struct**: Change `Environment` to use `Arc<RwLock<EnvironmentState>>`.
2.  [ ] **Fix Imports**: Ensure `std::sync::{Arc, RwLock}` is imported.
3.  [ ] **Update Methods**: Update `new`, `extend`, `define`, and `lookup` to handle locking (`.read().unwrap()` / `.write().unwrap()`).
4.  [ ] **Update `label`**: Ensure it evaluates *before* defining, relying on the pointer capture.
5.  [ ] **Verify**: Run `cargo test` to ensure no deadlocks or borrowing errors.

<!-- end list -->
