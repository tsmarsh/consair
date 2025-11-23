use std::collections::HashMap;
use std::sync::Arc;

use crate::language::{AtomType, LambdaCell, SymbolType, Value, car, cdr, cons, eq, is_atom};
use crate::numeric::NumericType;

// ============================================================================
// Environment
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    bindings: Arc<HashMap<String, Value>>,
    parent: Option<Arc<Environment>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            bindings: Arc::new(HashMap::new()),
            parent: None,
        }
    }

    fn extend(&self, params: &[String], args: &[Value]) -> Self {
        let mut bindings = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            bindings.insert(param.clone(), arg.clone());
        }
        Environment {
            bindings: Arc::new(bindings),
            parent: Some(Arc::new(self.clone())),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        // Copy-on-Write optimization using Arc::make_mut
        // Only clones if Arc has multiple strong references
        Arc::make_mut(&mut self.bindings).insert(name, value);
    }

    fn lookup(&self, name: &str) -> Option<Value> {
        if let Some(value) = self.bindings.get(name) {
            Some(value.clone())
        } else if let Some(ref parent) = self.parent {
            parent.lookup(name)
        } else {
            None
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
                    SymbolType::Symbol(name) => current_env
                        .lookup(name)
                        .ok_or_else(|| format!("Unbound symbol: {name}")),
                    SymbolType::Keyword { .. } => Ok(expr),
                };
            }

            // Self-evaluating forms
            Value::Lambda(_) | Value::Vector(_) | Value::NativeFn(_) => return Ok(expr),

            // List evaluation
            Value::Cons(ref cell) => {
                let operator = &cell.car;

                // Special forms
                if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = operator {
                    match name.as_str() {
                        "quote" => {
                            let arg = car(&cell.cdr)?;
                            return Ok(arg);
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
                                let cond_val = eval_loop(condition, &mut current_env, depth + 1)?;
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
                                    params.push(name.clone());
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
                                env.define(name.clone(), fn_val.clone());
                                return Ok(fn_val);
                            } else {
                                return Err("label: first argument must be a symbol".to_string());
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
                                _ => Err("vector-ref: first argument must be a vector".to_string()),
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
            }
        }
    }
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
