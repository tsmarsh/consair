//! Environment for variable bindings
//!
//! The Environment is a lexical scope that holds variable bindings.
//! It forms a chain of scopes, with child environments referencing their parents.

use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::interner::InternedSymbol;
use crate::language::Value;

// ============================================================================
// Environment
// ============================================================================

// Internal state holding the data and parent pointer
struct EnvironmentState {
    data: HashMap<String, Value>,
    parent: Option<Arc<Environment>>,
}

/// Environment for variable bindings.
///
/// The Environment is cheap to clone (just an Arc increment) and supports
/// concurrent access via RwLock.
#[derive(Clone)]
pub struct Environment {
    state: Arc<RwLock<EnvironmentState>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
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
    pub fn extend(&self, params: &[InternedSymbol], args: &[Value]) -> Self {
        let mut data = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            data.insert(param.resolve(), arg.clone());
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
