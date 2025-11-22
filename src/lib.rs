use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

// ============================================================================
// Core Type System
// ============================================================================

#[derive(Debug, Clone, PartialEq)]
pub enum AtomType {
    Symbol(String),
    Number(i64),
    Bool(bool),
}

#[derive(Clone)]
pub struct ConsCell {
    pub car: Value,
    pub cdr: Value,
}

#[derive(Clone)]
pub struct LambdaCell {
    pub params: Vec<String>,
    pub body: Value,
    pub env: Environment,
}

#[derive(Clone)]
pub enum Value {
    Atom(AtomType),
    Cons(Rc<ConsCell>),
    Nil,
    Lambda(Rc<LambdaCell>),
}

// ============================================================================
// Display Implementation
// ============================================================================

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Atom(AtomType::Symbol(s)) => write!(f, "{}", s),
            Value::Atom(AtomType::Number(n)) => write!(f, "{}", n),
            Value::Atom(AtomType::Bool(b)) => write!(f, "{}", if *b { "t" } else { "nil" }),
            Value::Nil => write!(f, "nil"),
            Value::Cons(_) => {
                write!(f, "(")?;
                let mut current = self.clone();
                loop {
                    match current {
                        Value::Cons(ref cell) => {
                            write!(f, "{}", cell.car)?;
                            match cell.cdr {
                                Value::Nil => break,
                                Value::Cons(_) => {
                                    write!(f, " ")?;
                                    current = cell.cdr.clone();
                                }
                                ref other => {
                                    write!(f, " . {}", other)?;
                                    break;
                                }
                            }
                        }
                        _ => break,
                    }
                }
                write!(f, ")")
            }
            Value::Lambda(_) => write!(f, "<lambda>"),
        }
    }
}

// ============================================================================
// Environment
// ============================================================================

#[derive(Clone)]
pub struct Environment {
    bindings: Rc<HashMap<String, Value>>,
    parent: Option<Rc<Environment>>,
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
// Parser
// ============================================================================

#[derive(Debug)]
enum Token {
    LParen,
    RParen,
    Quote,
    Symbol(String),
    Number(i64),
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut tokens = Vec::new();
    let mut chars = input.chars().peekable();

    while let Some(&ch) = chars.peek() {
        match ch {
            '(' => {
                tokens.push(Token::LParen);
                chars.next();
            }
            ')' => {
                tokens.push(Token::RParen);
                chars.next();
            }
            '\'' => {
                tokens.push(Token::Quote);
                chars.next();
            }
            ch if ch.is_whitespace() => {
                chars.next();
            }
            ch if ch.is_numeric() || ch == '-' => {
                let mut num = String::new();
                if ch == '-' {
                    num.push(ch);
                    chars.next();
                }
                while let Some(&ch) = chars.peek() {
                    if ch.is_numeric() {
                        num.push(ch);
                        chars.next();
                    } else {
                        break;
                    }
                }
                if let Ok(n) = num.parse::<i64>() {
                    tokens.push(Token::Number(n));
                } else {
                    tokens.push(Token::Symbol(num));
                }
            }
            _ => {
                let mut symbol = String::new();
                while let Some(&ch) = chars.peek() {
                    if ch.is_whitespace() || ch == '(' || ch == ')' || ch == '\'' {
                        break;
                    }
                    symbol.push(ch);
                    chars.next();
                }
                tokens.push(Token::Symbol(symbol));
            }
        }
    }

    tokens
}

fn parse_tokens(tokens: &[Token]) -> Result<(Value, usize), String> {
    if tokens.is_empty() {
        return Err("Unexpected end of input".to_string());
    }

    match &tokens[0] {
        Token::Number(n) => Ok((Value::Atom(AtomType::Number(*n)), 1)),
        Token::Symbol(s) => {
            if s == "nil" {
                Ok((Value::Nil, 1))
            } else if s == "t" {
                Ok((Value::Atom(AtomType::Bool(true)), 1))
            } else {
                Ok((Value::Atom(AtomType::Symbol(s.clone())), 1))
            }
        }
        Token::Quote => {
            let (quoted, consumed) = parse_tokens(&tokens[1..])?;
            let quote_list = cons(
                Value::Atom(AtomType::Symbol("quote".to_string())),
                cons(quoted, Value::Nil),
            );
            Ok((quote_list, consumed + 1))
        }
        Token::LParen => {
            let mut values = Vec::new();
            let mut i = 1;

            while i < tokens.len() {
                if matches!(tokens[i], Token::RParen) {
                    let list = values
                        .into_iter()
                        .rev()
                        .fold(Value::Nil, |acc, val| cons(val, acc));
                    return Ok((list, i + 1));
                }

                let (value, consumed) = parse_tokens(&tokens[i..])?;
                values.push(value);
                i += consumed;
            }

            Err("Unclosed parenthesis".to_string())
        }
        Token::RParen => Err("Unexpected )".to_string()),
    }
}

pub fn parse(input: &str) -> Result<Value, String> {
    let tokens = tokenize(input);
    let (value, _) = parse_tokens(&tokens)?;
    Ok(value)
}

// ============================================================================
// Primitive Operations
// ============================================================================

pub fn cons(car: Value, cdr: Value) -> Value {
    Value::Cons(Rc::new(ConsCell { car, cdr }))
}

fn car(value: &Value) -> Result<Value, String> {
    match value {
        Value::Cons(cell) => Ok(cell.car.clone()),
        _ => Err(format!("car: expected cons cell, got {}", value)),
    }
}

fn cdr(value: &Value) -> Result<Value, String> {
    match value {
        Value::Cons(cell) => Ok(cell.cdr.clone()),
        _ => Err(format!("cdr: expected cons cell, got {}", value)),
    }
}

fn is_atom(value: &Value) -> bool {
    matches!(value, Value::Atom(_) | Value::Nil)
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Atom(a1), Value::Atom(a2)) => a1 == a2,
        (Value::Nil, Value::Nil) => true,
        _ => false,
    }
}

// ============================================================================
// Evaluator
// ============================================================================

pub fn eval(expr: Value, env: &mut Environment) -> Result<Value, String> {
    match expr {
        // Self-evaluating forms
        Value::Atom(AtomType::Number(_)) | Value::Atom(AtomType::Bool(_)) | Value::Nil => {
            Ok(expr)
        }

        // Symbol lookup
        Value::Atom(AtomType::Symbol(ref name)) => env
            .lookup(name)
            .ok_or_else(|| format!("Unbound symbol: {}", name)),

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
                            let is_true = match cond_val {
                                Value::Nil | Value::Atom(AtomType::Bool(false)) => false,
                                _ => true,
                            };

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
                _ => Err(format!("Cannot apply non-function: {}", func)),
            }
        }
    }
}
