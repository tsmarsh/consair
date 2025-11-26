//! Analysis functions for JIT compilation.
//!
//! This module provides free variable analysis and other static analysis
//! utilities used during JIT compilation.

use std::collections::HashSet;

use consair::interner::InternedSymbol;
use consair::language::{AtomType, SymbolType, Value};

/// Find all free variables in an expression.
/// A free variable is one that is used but not defined in the local scope.
pub fn find_free_variables(
    expr: &Value,
    bound: &HashSet<InternedSymbol>,
) -> HashSet<InternedSymbol> {
    let mut free = HashSet::new();
    find_free_vars_helper(expr, bound, &mut free);
    free
}

/// Helper function to recursively find free variables.
fn find_free_vars_helper(
    expr: &Value,
    bound: &HashSet<InternedSymbol>,
    free: &mut HashSet<InternedSymbol>,
) {
    match expr {
        Value::Nil => {}
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => {
            let name = sym.resolve();
            // Skip built-in operators and special forms
            if !is_builtin(&name) && !bound.contains(sym) {
                free.insert(*sym);
            }
        }
        Value::Atom(_) => {}
        Value::Cons(cell) => {
            // Check if this is a special form
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = &cell.car {
                let name = sym.resolve();
                match name.as_str() {
                    "quote" => {
                        // Don't look for free variables in quoted expressions
                    }
                    "lambda" => {
                        // Lambda binds its parameters
                        let args = collect_list(&cell.cdr);
                        if args.len() >= 2 {
                            let params = &args[0];
                            let body = &args[1];
                            let param_list = collect_list(params);
                            let mut new_bound = bound.clone();
                            for p in param_list {
                                if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) = p {
                                    new_bound.insert(s);
                                }
                            }
                            find_free_vars_helper(body, &new_bound, free);
                        }
                    }
                    "label" => {
                        // Label binds the name for recursive calls
                        let args = collect_list(&cell.cdr);
                        if args.len() >= 2 {
                            let name_val = &args[0];
                            let lambda_val = &args[1];
                            let mut new_bound = bound.clone();
                            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) = name_val {
                                new_bound.insert(*s);
                            }
                            find_free_vars_helper(lambda_val, &new_bound, free);
                        }
                    }
                    "cond" => {
                        // Check all condition clauses
                        let clauses = collect_list(&cell.cdr);
                        for clause in clauses {
                            let parts = collect_list(&clause);
                            for part in parts {
                                find_free_vars_helper(&part, bound, free);
                            }
                        }
                    }
                    _ => {
                        // Regular function call - check operator and all arguments
                        find_free_vars_helper(&cell.car, bound, free);
                        let args = collect_list(&cell.cdr);
                        for arg in args {
                            find_free_vars_helper(&arg, bound, free);
                        }
                    }
                }
            } else {
                // Not a symbol in operator position - check both car and cdr
                find_free_vars_helper(&cell.car, bound, free);
                let args = collect_list(&cell.cdr);
                for arg in args {
                    find_free_vars_helper(&arg, bound, free);
                }
            }
        }
        Value::Vector(vec) => {
            for elem in &vec.elements {
                find_free_vars_helper(elem, bound, free);
            }
        }
        Value::PersistentVector(vec) => {
            for elem in vec.elements.iter() {
                find_free_vars_helper(elem, bound, free);
            }
        }
        Value::Map(m) => {
            for (k, v) in &m.entries {
                find_free_vars_helper(k, bound, free);
                find_free_vars_helper(v, bound, free);
            }
        }
        Value::PersistentMap(m) => {
            for (k, v) in m.entries.iter() {
                find_free_vars_helper(k, bound, free);
                find_free_vars_helper(v, bound, free);
            }
        }
        Value::Set(s) => {
            for elem in &s.elements {
                find_free_vars_helper(elem, bound, free);
            }
        }
        Value::PersistentSet(s) => {
            for elem in s.elements.iter() {
                find_free_vars_helper(elem, bound, free);
            }
        }
        Value::Lambda(_) | Value::Macro(_) | Value::Reduced(_) | Value::NativeFn(_) => {}
    }
}

/// Check if a symbol is a built-in operator.
pub fn is_builtin(name: &str) -> bool {
    matches!(
        name,
        "quote"
            | "lambda"
            | "label"
            | "cond"
            | "cons"
            | "car"
            | "cdr"
            | "+"
            | "-"
            | "*"
            | "/"
            | "="
            | "<"
            | ">"
            | "<="
            | ">="
            | "eq"
            | "atom"
            | "nil?"
            | "number?"
            | "cons?"
            | "not"
            | "t"
            | "nil"
    )
}

/// Collect a cons list into a Vec.
pub fn collect_list(val: &Value) -> Vec<Value> {
    let mut result = Vec::new();
    let mut current = val.clone();
    loop {
        match current {
            Value::Nil => break,
            Value::Cons(cell) => {
                result.push(cell.car.clone());
                current = cell.cdr.clone();
            }
            _ => break,
        }
    }
    result
}
