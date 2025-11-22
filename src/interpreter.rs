use std::collections::HashMap;
use std::rc::Rc;

use crate::language::{AtomType, LambdaCell, Value, car, cdr, cons, eq, is_atom};

// ============================================================================
// Environment
// ============================================================================

#[derive(Clone)]
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

        // Lambda is self-evaluating
        Value::Lambda(_) => Ok(expr),

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
