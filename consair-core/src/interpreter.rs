use std::collections::HashMap;
use std::sync::{Arc, RwLock};

use crate::interner::InternedSymbol;
use crate::language::{
    AtomType, LambdaCell, MacroCell, SymbolType, Value, car, cdr, cons, eq, is_atom,
};
use crate::numeric::NumericType;

// ============================================================================
// Environment
// ============================================================================

// Internal state holding the data and parent pointer
struct EnvironmentState {
    data: HashMap<String, Value>,
    parent: Option<Arc<Environment>>,
}

// The public wrapper that is cheap to clone
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
    pub fn extend(&self, params: &[crate::interner::InternedSymbol], args: &[Value]) -> Self {
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

// ============================================================================
// Evaluator with Tail Call Optimization
// ============================================================================

/// Maximum recursion depth for non-tail calls
const MAX_DEPTH: usize = 10000;

pub fn eval(expr: Value, env: &mut Environment) -> Result<Value, String> {
    eval_loop(expr, env, 0)
}

fn eval_loop(mut expr: Value, env: &mut Environment, depth: usize) -> Result<Value, String> {
    // Track depth for non-tail recursive calls
    if depth >= MAX_DEPTH {
        return Err(format!(
            "Maximum recursion depth ({MAX_DEPTH}) exceeded. \
             This usually indicates very deep non-tail recursion."
        ));
    }

    // Start with the passed-in environment
    // For tail calls within the same scope, we keep this
    // For tail calls to lambdas, we'll replace it
    let mut current_env = env.clone();

    'outer: loop {
        match expr {
            // Self-evaluating forms - return immediately
            Value::Atom(AtomType::Number(_))
            | Value::Atom(AtomType::Bool(_))
            | Value::Atom(AtomType::String(_))
            | Value::Atom(AtomType::Char(_))
            | Value::Nil => return Ok(expr),

            // Symbol lookup
            Value::Atom(AtomType::Symbol(ref sym)) => {
                return match sym {
                    SymbolType::Symbol(name) => name.with_str(|s| {
                        current_env
                            .lookup(s)
                            .ok_or_else(|| format!("Unbound symbol: {name}"))
                    }),
                    SymbolType::Keyword { .. } => Ok(expr),
                };
            }

            // Self-evaluating forms
            Value::Lambda(_) | Value::Macro(_) | Value::Vector(_) | Value::NativeFn(_) => {
                return Ok(expr);
            }

            // List evaluation
            Value::Cons(ref _cell) => {
                // First, try to expand macros
                expr = expand_macros(expr.clone(), &mut current_env, depth)?;

                // After expansion, re-match to handle the expanded form
                if let Value::Cons(ref cell) = expr {
                    let operator = &cell.car;

                    // Special forms
                    if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = operator {
                        let sym_str = name.resolve();
                        match sym_str.as_str() {
                            "quote" => {
                                let arg = car(&cell.cdr)?;
                                return Ok(arg);
                            }
                            "quasiquote" => {
                                let arg = car(&cell.cdr)?;
                                return eval_quasiquote(arg, &mut current_env, depth, 0);
                            }
                            "defmacro" => {
                                let name_expr = car(&cell.cdr)?;
                                let rest = cdr(&cell.cdr)?;
                                let params_expr = car(&rest)?;
                                let body = car(&cdr(&rest)?)?;

                                // Extract macro name
                                let name = match name_expr {
                                    Value::Atom(AtomType::Symbol(SymbolType::Symbol(n))) => n,
                                    _ => {
                                        return Err(
                                            "defmacro: first argument must be a symbol".to_string()
                                        );
                                    }
                                };

                                // Extract parameter names
                                let mut params = Vec::new();
                                let mut current_param = params_expr;
                                while let Value::Cons(ref param_cell) = current_param {
                                    if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(
                                        param_name,
                                    ))) = &param_cell.car
                                    {
                                        params.push(*param_name);
                                    } else {
                                        return Err(
                                            "defmacro parameters must be symbols".to_string()
                                        );
                                    }
                                    current_param = param_cell.cdr.clone();
                                }

                                // Create macro
                                let macro_val = Value::Macro(Arc::new(MacroCell {
                                    params,
                                    body,
                                    env: current_env.clone(),
                                }));

                                // Define in environment
                                env.define(name.resolve(), macro_val.clone());
                                return Ok(macro_val);
                            }
                            "atom" => {
                                let arg = car(&cell.cdr)?;
                                let val = eval_loop(arg, &mut current_env, depth + 1)?;
                                return Ok(Value::Atom(AtomType::Bool(is_atom(&val))));
                            }
                            "eq" => {
                                let args = cell.cdr.clone();
                                let arg1 = car(&args)?;
                                let rest = cdr(&args)?;
                                let arg2 = car(&rest)?;

                                let val1 = eval_loop(arg1, &mut current_env, depth + 1)?;
                                let val2 = eval_loop(arg2, &mut current_env, depth + 1)?;

                                return Ok(Value::Atom(AtomType::Bool(eq(&val1, &val2))));
                            }
                            "car" => {
                                let arg = car(&cell.cdr)?;
                                let val = eval_loop(arg, &mut current_env, depth + 1)?;
                                return car(&val);
                            }
                            "cdr" => {
                                let arg = car(&cell.cdr)?;
                                let val = eval_loop(arg, &mut current_env, depth + 1)?;
                                return cdr(&val);
                            }
                            "cons" => {
                                let args = cell.cdr.clone();
                                let arg1 = car(&args)?;
                                let rest = cdr(&args)?;
                                let arg2 = car(&rest)?;

                                let val1 = eval_loop(arg1, &mut current_env, depth + 1)?;
                                let val2 = eval_loop(arg2, &mut current_env, depth + 1)?;

                                return Ok(cons(val1, val2));
                            }
                            "cond" => {
                                // TAIL POSITION: cond result expressions are in tail position
                                let mut clauses = cell.cdr.clone();
                                while let Value::Cons(ref clause_cell) = clauses {
                                    let clause = clause_cell.car.clone();
                                    let condition = car(&clause)?;
                                    let result_expr = car(&cdr(&clause)?)?;

                                    // Evaluate condition (NOT tail position)
                                    let cond_val =
                                        eval_loop(condition, &mut current_env, depth + 1)?;
                                    let is_true = !matches!(
                                        cond_val,
                                        Value::Nil | Value::Atom(AtomType::Bool(false))
                                    );

                                    if is_true {
                                        // TAIL CALL: update expr and continue loop
                                        expr = result_expr;
                                        continue 'outer;
                                    }

                                    clauses = clause_cell.cdr.clone();
                                }
                                return Ok(Value::Nil);
                            }
                            "lambda" => {
                                let params_expr = car(&cell.cdr)?;
                                let body = car(&cdr(&cell.cdr)?)?;

                                // Extract parameter names
                                let mut params = Vec::new();
                                let mut current = params_expr;
                                while let Value::Cons(ref param_cell) = current {
                                    if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) =
                                        &param_cell.car
                                    {
                                        params.push(*name);
                                    } else {
                                        return Err("lambda parameters must be symbols".to_string());
                                    }
                                    current = param_cell.cdr.clone();
                                }

                                return Ok(Value::Lambda(Arc::new(LambdaCell {
                                    params,
                                    body,
                                    env: current_env.clone(),
                                })));
                            }
                            "label" => {
                                let name_expr = car(&cell.cdr)?;
                                let fn_expr = car(&cdr(&cell.cdr)?)?;

                                if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) =
                                    name_expr
                                {
                                    let fn_val = eval_loop(fn_expr, &mut current_env, depth + 1)?;
                                    env.define(name.resolve(), fn_val.clone());
                                    return Ok(fn_val);
                                } else {
                                    return Err(
                                        "label: first argument must be a symbol".to_string()
                                    );
                                }
                            }
                            // Arithmetic operations (NOT tail position)
                            "+" => {
                                return eval_arithmetic(
                                    "+",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a.add(b),
                                );
                            }
                            "-" => {
                                return eval_arithmetic(
                                    "-",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a.sub(b),
                                );
                            }
                            "*" => {
                                return eval_arithmetic(
                                    "*",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a.mul(b),
                                );
                            }
                            "/" => {
                                return eval_arithmetic(
                                    "/",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a.div(b),
                                );
                            }
                            // Comparison operations (NOT tail position)
                            "<" => {
                                return eval_comparison(
                                    "<",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a < b,
                                );
                            }
                            ">" => {
                                return eval_comparison(
                                    ">",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a > b,
                                );
                            }
                            "<=" => {
                                return eval_comparison(
                                    "<=",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a <= b,
                                );
                            }
                            ">=" => {
                                return eval_comparison(
                                    ">=",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a >= b,
                                );
                            }
                            "=" => {
                                return eval_comparison(
                                    "=",
                                    &cell.cdr,
                                    &mut current_env,
                                    depth,
                                    |a, b| a == b,
                                );
                            }
                            // Vector operations (NOT tail position)
                            "vector-length" => {
                                let arg = car(&cell.cdr)?;
                                let val = eval_loop(arg, &mut current_env, depth + 1)?;
                                return match val {
                                    Value::Vector(vec) => Ok(Value::Atom(AtomType::Number(
                                        NumericType::Int(vec.elements.len() as i64),
                                    ))),
                                    _ => Err("vector-length: expected vector".to_string()),
                                };
                            }
                            "vector-ref" => {
                                let args = cell.cdr.clone();
                                let vec_expr = car(&args)?;
                                let rest = cdr(&args)?;
                                let idx_expr = car(&rest)?;

                                let vec_val = eval_loop(vec_expr, &mut current_env, depth + 1)?;
                                let idx_val = eval_loop(idx_expr, &mut current_env, depth + 1)?;

                                return match (vec_val, idx_val) {
                                    (
                                        Value::Vector(vec),
                                        Value::Atom(AtomType::Number(NumericType::Int(idx))),
                                    ) => {
                                        if idx < 0 || idx >= vec.elements.len() as i64 {
                                            Err(format!(
                                                "vector-ref: index {idx} out of bounds (length {})",
                                                vec.elements.len()
                                            ))
                                        } else {
                                            Ok(vec.elements[idx as usize].clone())
                                        }
                                    }
                                    (Value::Vector(_), _) => {
                                        Err("vector-ref: index must be an integer".to_string())
                                    }
                                    _ => {
                                        Err("vector-ref: first argument must be a vector"
                                            .to_string())
                                    }
                                };
                            }
                            _ => {}
                        }
                    }

                    // Function application
                    let func = eval_loop(operator.clone(), &mut current_env, depth + 1)?;

                    // Evaluate arguments (NOT tail position)
                    let mut args = Vec::new();
                    let mut current = cell.cdr.clone();
                    while let Value::Cons(ref arg_cell) = current {
                        let arg_val = eval_loop(arg_cell.car.clone(), &mut current_env, depth + 1)?;
                        args.push(arg_val);
                        current = arg_cell.cdr.clone();
                    }

                    // Apply function
                    match func {
                        Value::Lambda(ref lambda) => {
                            if args.len() != lambda.params.len() {
                                return Err(format!(
                                    "lambda: expected {} arguments, got {}",
                                    lambda.params.len(),
                                    args.len()
                                ));
                            }

                            // TAIL CALL OPTIMIZATION:
                            // Instead of recursing, update environment and expression
                            current_env = lambda.env.extend(&lambda.params, &args);
                            expr = lambda.body.clone();
                            // Continue the loop - this is tail call optimization!
                        }
                        Value::NativeFn(native_fn) => {
                            // Native functions can't be tail-optimized
                            return native_fn(&args, &mut current_env);
                        }
                        _ => return Err(format!("Cannot apply non-function: {func}")),
                    }
                } else {
                    // After macro expansion, result is not a list - just return it
                    return Ok(expr);
                }
            }
        }
    }
}

// ============================================================================
// Macro Support - Quasiquote Evaluation
// ============================================================================

/// Evaluate quasiquote - construct templates with unquote/unquote-splicing
fn eval_quasiquote(
    expr: Value,
    env: &mut Environment,
    depth: usize,
    level: usize,
) -> Result<Value, String> {
    match expr {
        // Check for unquote at this level
        Value::Cons(ref cell) => {
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &cell.car {
                match name.resolve().as_str() {
                    "unquote" if level == 0 => {
                        // Evaluate the unquoted expression
                        let arg = car(&cell.cdr)?;
                        return eval_loop(arg, env, depth + 1);
                    }
                    "unquote-splicing" if level == 0 => {
                        return Err("unquote-splicing not in list context".to_string());
                    }
                    "quasiquote" => {
                        // Nested quasiquote - increase level
                        let arg = car(&cell.cdr)?;
                        let result = eval_quasiquote(arg, env, depth, level + 1)?;
                        return Ok(cons(
                            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                                "quasiquote",
                            )))),
                            cons(result, Value::Nil),
                        ));
                    }
                    "unquote" => {
                        // Nested unquote - decrease level
                        let arg = car(&cell.cdr)?;
                        let result = eval_quasiquote(arg, env, depth, level - 1)?;
                        return Ok(cons(
                            Value::Atom(AtomType::Symbol(SymbolType::Symbol(InternedSymbol::new(
                                "unquote",
                            )))),
                            cons(result, Value::Nil),
                        ));
                    }
                    _ => {}
                }
            }

            // Process list elements, handling unquote-splicing
            let mut result_elements = Vec::new();
            let mut current = expr;

            while let Value::Cons(ref element_cell) = current {
                // Check if this element is unquote-splicing
                if let Value::Cons(ref inner) = element_cell.car
                    && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &inner.car
                    && name.resolve().as_str() == "unquote-splicing"
                    && level == 0
                {
                    // Evaluate and splice the result
                    let splice_expr = car(&inner.cdr)?;
                    let splice_result = eval_loop(splice_expr, env, depth + 1)?;

                    // Splice the list into result
                    let mut splice_current = splice_result;
                    while let Value::Cons(ref splice_cell) = splice_current {
                        result_elements.push(splice_cell.car.clone());
                        splice_current = splice_cell.cdr.clone();
                    }

                    current = element_cell.cdr.clone();
                    continue;
                }

                // Not unquote-splicing, process normally
                let processed = eval_quasiquote(element_cell.car.clone(), env, depth, level)?;
                result_elements.push(processed);
                current = element_cell.cdr.clone();
            }

            // Handle improper list (dotted pair)
            if !matches!(current, Value::Nil) {
                return Err("quasiquote: improper list not fully supported".to_string());
            }

            // Build result list
            let result = result_elements
                .into_iter()
                .rev()
                .fold(Value::Nil, |acc, elem| cons(elem, acc));
            Ok(result)
        }
        // Atoms and other values quote themselves
        _ => Ok(expr),
    }
}

// ============================================================================
// Macro Expansion
// ============================================================================

/// Check if a value is a macro call and expand it
fn expand_macro_once(
    expr: Value,
    env: &mut Environment,
    depth: usize,
) -> Result<(Value, bool), String> {
    if let Value::Cons(cell) = &expr
        && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &cell.car
        && let Some(Value::Macro(macro_cell)) = env.lookup(&name.resolve())
    {
        // Collect unevaluated arguments
        let mut args = Vec::new();
        let mut current = cell.cdr.clone();
        while let Value::Cons(ref arg_cell) = current {
            args.push(arg_cell.car.clone());
            current = arg_cell.cdr.clone();
        }

        // Check argument count
        if args.len() != macro_cell.params.len() {
            return Err(format!(
                "macro: expected {} arguments, got {}",
                macro_cell.params.len(),
                args.len()
            ));
        }

        // Create environment for macro expansion
        let mut macro_env = macro_cell.env.extend(&macro_cell.params, &args);

        // Evaluate macro body to get expanded code
        let expanded = eval_loop(macro_cell.body.clone(), &mut macro_env, depth + 1)?;
        return Ok((expanded, true));
    }

    Ok((expr, false))
}

/// Recursively expand all macros in an expression
fn expand_macros(expr: Value, env: &mut Environment, depth: usize) -> Result<Value, String> {
    let (mut result, mut expanded) = expand_macro_once(expr, env, depth)?;

    // Keep expanding until no more macros
    while expanded {
        let (new_result, new_expanded) = expand_macro_once(result, env, depth)?;
        result = new_result;
        expanded = new_expanded;
    }

    Ok(result)
}

// ============================================================================
// Helper Functions for Arithmetic and Comparison
// ============================================================================

fn eval_arithmetic<F>(
    op_name: &str,
    args: &Value,
    env: &mut Environment,
    depth: usize,
    op: F,
) -> Result<Value, String>
where
    F: Fn(&NumericType, &NumericType) -> Result<NumericType, String>,
{
    let arg1 = car(args)?;
    let rest = cdr(args)?;
    let arg2 = car(&rest)?;

    let val1 = eval_loop(arg1, env, depth + 1)?;
    let val2 = eval_loop(arg2, env, depth + 1)?;

    // Extract numeric values
    let num1 = match val1 {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("{op_name}: expected number, got {val1}")),
    };

    let num2 = match val2 {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("{op_name}: expected number, got {val2}")),
    };

    // Perform operation
    let result = op(&num1, &num2)?;
    Ok(Value::Atom(AtomType::Number(result)))
}

fn eval_comparison<F>(
    op_name: &str,
    args: &Value,
    env: &mut Environment,
    depth: usize,
    op: F,
) -> Result<Value, String>
where
    F: Fn(&NumericType, &NumericType) -> bool,
{
    let arg1 = car(args)?;
    let rest = cdr(args)?;
    let arg2 = car(&rest)?;

    let val1 = eval_loop(arg1, env, depth + 1)?;
    let val2 = eval_loop(arg2, env, depth + 1)?;

    // Extract numeric values
    let num1 = match val1 {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("{op_name}: expected number, got {val1}")),
    };

    let num2 = match val2 {
        Value::Atom(AtomType::Number(n)) => n,
        _ => return Err(format!("{op_name}: expected number, got {val2}")),
    };

    // Perform comparison
    let result = op(&num1, &num2);
    Ok(Value::Atom(AtomType::Bool(result)))
}
