use std::collections::HashMap;
use std::rc::Rc;

use crate::language::{AtomType, LambdaCell, Value, car, cdr, cons, eq, is_atom};
use crate::numeric::NumericType;

// ============================================================================
// Environment
// ============================================================================

#[derive(Clone, Debug, PartialEq)]
pub struct Environment {
    bindings: Rc<HashMap<String, Value>>,
    parent: Option<Rc<Environment>>,
}

impl Default for Environment {
    fn default() -> Self {
        Self::new()
    }
}

impl Environment {
    pub fn new() -> Self {
        Environment {
            bindings: Rc::new(HashMap::new()),
            parent: None,
        }
    }

    fn extend(&self, params: &[String], args: &[Value]) -> Self {
        let mut bindings = HashMap::new();
        for (param, arg) in params.iter().zip(args.iter()) {
            bindings.insert(param.clone(), arg.clone());
        }
        Environment {
            bindings: Rc::new(bindings),
            parent: Some(Rc::new(self.clone())),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        // Create a new HashMap with the existing bindings plus the new one
        let mut new_bindings = (*self.bindings).clone();
        new_bindings.insert(name, value);
        self.bindings = Rc::new(new_bindings);
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
// Evaluator
// ============================================================================

pub fn eval(expr: Value, env: &mut Environment) -> Result<Value, String> {
    match expr {
        // Self-evaluating forms
        Value::Atom(AtomType::Number(_)) | Value::Atom(AtomType::Bool(_)) | Value::Nil => Ok(expr),

        // Symbol lookup
        Value::Atom(AtomType::Symbol(ref name)) => env
            .lookup(name)
            .ok_or_else(|| format!("Unbound symbol: {name}")),

        // Lambda and Vector are self-evaluating
        Value::Lambda(_) | Value::Vector(_) => Ok(expr),

        // List evaluation
        Value::Cons(ref cell) => {
            let operator = &cell.car;

            // Special forms
            if let Value::Atom(AtomType::Symbol(name)) = operator {
                match name.as_str() {
                    "quote" => {
                        let arg = car(&cell.cdr)?;
                        return Ok(arg);
                    }
                    "atom" => {
                        let arg = car(&cell.cdr)?;
                        let val = eval(arg, env)?;
                        return Ok(Value::Atom(AtomType::Bool(is_atom(&val))));
                    }
                    "eq" => {
                        let args = cell.cdr.clone();
                        let arg1 = car(&args)?;
                        let rest = cdr(&args)?;
                        let arg2 = car(&rest)?;

                        let val1 = eval(arg1, env)?;
                        let val2 = eval(arg2, env)?;

                        return Ok(Value::Atom(AtomType::Bool(eq(&val1, &val2))));
                    }
                    "car" => {
                        let arg = car(&cell.cdr)?;
                        let val = eval(arg, env)?;
                        return car(&val);
                    }
                    "cdr" => {
                        let arg = car(&cell.cdr)?;
                        let val = eval(arg, env)?;
                        return cdr(&val);
                    }
                    "cons" => {
                        let args = cell.cdr.clone();
                        let arg1 = car(&args)?;
                        let rest = cdr(&args)?;
                        let arg2 = car(&rest)?;

                        let val1 = eval(arg1, env)?;
                        let val2 = eval(arg2, env)?;

                        return Ok(cons(val1, val2));
                    }
                    "cond" => {
                        let mut clauses = cell.cdr.clone();
                        while let Value::Cons(ref clause_cell) = clauses {
                            let clause = clause_cell.car.clone();
                            let condition = car(&clause)?;
                            let result_expr = car(&cdr(&clause)?)?;

                            let cond_val = eval(condition, env)?;
                            let is_true = !matches!(
                                cond_val,
                                Value::Nil | Value::Atom(AtomType::Bool(false))
                            );

                            if is_true {
                                return eval(result_expr, env);
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
                            if let Value::Atom(AtomType::Symbol(ref name)) = param_cell.car {
                                params.push(name.clone());
                            } else {
                                return Err("lambda parameters must be symbols".to_string());
                            }
                            current = param_cell.cdr.clone();
                        }

                        return Ok(Value::Lambda(Rc::new(LambdaCell {
                            params,
                            body,
                            env: env.clone(),
                        })));
                    }
                    "label" => {
                        let name_expr = car(&cell.cdr)?;
                        let fn_expr = car(&cdr(&cell.cdr)?)?;

                        if let Value::Atom(AtomType::Symbol(name)) = name_expr {
                            let fn_val = eval(fn_expr, env)?;
                            env.define(name.clone(), fn_val.clone());
                            return Ok(fn_val);
                        } else {
                            return Err("label: first argument must be a symbol".to_string());
                        }
                    }
                    // Arithmetic operations
                    "+" => {
                        return eval_arithmetic("+", &cell.cdr, env, |a, b| a.add(b));
                    }
                    "-" => {
                        return eval_arithmetic("-", &cell.cdr, env, |a, b| a.sub(b));
                    }
                    "*" => {
                        return eval_arithmetic("*", &cell.cdr, env, |a, b| a.mul(b));
                    }
                    "/" => {
                        return eval_arithmetic("/", &cell.cdr, env, |a, b| a.div(b));
                    }
                    // Comparison operations
                    "<" => {
                        return eval_comparison("<", &cell.cdr, env, |a, b| a < b);
                    }
                    ">" => {
                        return eval_comparison(">", &cell.cdr, env, |a, b| a > b);
                    }
                    "<=" => {
                        return eval_comparison("<=", &cell.cdr, env, |a, b| a <= b);
                    }
                    ">=" => {
                        return eval_comparison(">=", &cell.cdr, env, |a, b| a >= b);
                    }
                    "=" => {
                        return eval_comparison("=", &cell.cdr, env, |a, b| a == b);
                    }
                    // Vector operations
                    "vector-length" => {
                        let arg = car(&cell.cdr)?;
                        let val = eval(arg, env)?;
                        match val {
                            Value::Vector(vec) => {
                                return Ok(Value::Atom(AtomType::Number(NumericType::Int(
                                    vec.elements.len() as i64,
                                ))));
                            }
                            _ => return Err("vector-length: expected vector".to_string()),
                        }
                    }
                    "vector-ref" => {
                        let args = cell.cdr.clone();
                        let vec_expr = car(&args)?;
                        let rest = cdr(&args)?;
                        let idx_expr = car(&rest)?;

                        let vec_val = eval(vec_expr, env)?;
                        let idx_val = eval(idx_expr, env)?;

                        match (vec_val, idx_val) {
                            (
                                Value::Vector(vec),
                                Value::Atom(AtomType::Number(NumericType::Int(idx))),
                            ) => {
                                if idx < 0 || idx >= vec.elements.len() as i64 {
                                    return Err(format!(
                                        "vector-ref: index {idx} out of bounds (length {})",
                                        vec.elements.len()
                                    ));
                                }
                                return Ok(vec.elements[idx as usize].clone());
                            }
                            (Value::Vector(_), _) => {
                                return Err("vector-ref: index must be an integer".to_string());
                            }
                            _ => {
                                return Err(
                                    "vector-ref: first argument must be a vector".to_string()
                                );
                            }
                        }
                    }
                    _ => {}
                }
            }

            // Function application
            let func = eval(operator.clone(), env)?;

            // Evaluate arguments
            let mut args = Vec::new();
            let mut current = cell.cdr.clone();
            while let Value::Cons(ref arg_cell) = current {
                let arg_val = eval(arg_cell.car.clone(), env)?;
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

                    let mut new_env = lambda.env.extend(&lambda.params, &args);
                    eval(lambda.body.clone(), &mut new_env)
                }
                _ => Err(format!("Cannot apply non-function: {func}")),
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
    op: F,
) -> Result<Value, String>
where
    F: Fn(&NumericType, &NumericType) -> Result<NumericType, String>,
{
    let arg1 = car(args)?;
    let rest = cdr(args)?;
    let arg2 = car(&rest)?;

    let val1 = eval(arg1, env)?;
    let val2 = eval(arg2, env)?;

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
    op: F,
) -> Result<Value, String>
where
    F: Fn(&NumericType, &NumericType) -> bool,
{
    let arg1 = car(args)?;
    let rest = cdr(args)?;
    let arg2 = car(&rest)?;

    let val1 = eval(arg1, env)?;
    let val2 = eval(arg2, env)?;

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
