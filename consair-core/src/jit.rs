//! JIT execution engine for Consair.
//!
//! This module provides the ability to compile and immediately execute
//! Consair expressions using LLVM's JIT compilation.

use std::collections::{HashMap, HashSet};
use std::sync::atomic::AtomicUsize;

use inkwell::OptimizationLevel;
use inkwell::context::Context;
use inkwell::execution_engine::ExecutionEngine;
use inkwell::values::FunctionValue;

use crate::codegen::Codegen;
use crate::interner::InternedSymbol;
use crate::language::{AtomType, SymbolType, Value};
use crate::numeric::NumericType;
use crate::runtime::RuntimeValue;

/// JIT compilation environment - maps symbols to their compiled values.
type JitEnv<'ctx> = HashMap<InternedSymbol, inkwell::values::StructValue<'ctx>>;

/// Stored lambda definitions for recursive functions.
type LambdaStore = HashMap<InternedSymbol, Value>;

/// Compiled LLVM functions - maps function names to LLVM function values.
type CompiledFns<'ctx> = HashMap<InternedSymbol, FunctionValue<'ctx>>;

/// Counter for generating unique function names
static EXPR_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Find all free variables in an expression.
/// A free variable is one that is used but not defined in the local scope.
fn find_free_variables(expr: &Value, bound: &HashSet<InternedSymbol>) -> HashSet<InternedSymbol> {
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
        Value::Lambda(_) | Value::Macro(_) | Value::NativeFn(_) => {}
    }
}

/// Check if a symbol is a built-in operator.
fn is_builtin(name: &str) -> bool {
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
fn collect_list(val: &Value) -> Vec<Value> {
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

/// Type alias for a compiled expression function
type ExprFn = unsafe extern "C" fn() -> RuntimeValue;

/// JIT execution engine for compiling and running Consair expressions.
pub struct JitEngine {
    /// LLVM context - must be kept alive as long as execution engine exists
    context: Context,
}

impl JitEngine {
    /// Create a new JIT engine.
    pub fn new() -> Result<Self, String> {
        Ok(JitEngine {
            context: Context::create(),
        })
    }

    /// Compile and execute a single expression.
    pub fn eval(&self, expr: &Value) -> Result<RuntimeValue, String> {
        // Generate unique function name
        let counter = EXPR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let fn_name = format!("__consair_expr_{}", counter);

        // Create code generator
        let codegen = Codegen::new(&self.context, &fn_name);

        // Compile the expression into a function with empty environments
        let env = JitEnv::new();
        let lambdas = LambdaStore::new();
        let compiled_fns = CompiledFns::new();
        let _compiled =
            self.compile_expr(&codegen, expr, &fn_name, &env, &lambdas, &compiled_fns)?;

        // Verify the module
        codegen.verify()?;

        // Create execution engine
        let execution_engine = codegen
            .module
            .create_jit_execution_engine(OptimizationLevel::Default)
            .map_err(|e| e.to_string())?;

        // Link runtime functions
        self.link_runtime_functions(&codegen, &execution_engine);

        // Get the compiled function
        let func = unsafe {
            execution_engine
                .get_function::<ExprFn>(&fn_name)
                .map_err(|e| e.to_string())?
        };

        // Execute the function
        let result = unsafe { func.call() };

        Ok(result)
    }

    /// Compile an expression into LLVM IR.
    fn compile_expr<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        expr: &Value,
        fn_name: &str,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::FunctionValue<'ctx>, String> {
        // Create the expression function
        let fn_type = codegen.expr_fn_type();
        let function = codegen.add_function(fn_name, fn_type);

        // Create entry block
        let entry = self.context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(entry);

        // Compile the expression body (top-level is in tail position)
        let result = self.compile_value(codegen, expr, env, lambdas, compiled_fns, true)?;

        // Return the result
        codegen
            .builder
            .build_return(Some(&result))
            .map_err(|e| e.to_string())?;

        Ok(function)
    }

    /// Compile a Value into LLVM IR, returning the result as a struct value.
    ///
    /// `tail_position` indicates whether this expression is in tail position,
    /// which enables tail call optimization for function calls.
    fn compile_value<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        value: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        match value {
            Value::Nil => Ok(codegen.compile_nil()),

            Value::Atom(AtomType::Bool(b)) => Ok(codegen.compile_bool(*b)),

            Value::Atom(AtomType::Number(num)) => match num {
                NumericType::Int(n) => Ok(codegen.compile_int(*n)),
                NumericType::Float(f) => Ok(codegen.compile_float(*f)),
                NumericType::Ratio(num, denom) => {
                    // Convert ratio to float
                    Ok(codegen.compile_float(*num as f64 / *denom as f64))
                }
                NumericType::BigInt(_) => Err("JIT does not support BigInt".to_string()),
                NumericType::BigRatio(_) => Err("JIT does not support BigRatio".to_string()),
            },

            Value::Atom(AtomType::Symbol(sym)) => {
                let SymbolType::Symbol(interned) = sym;

                // Check if symbol is bound in environment
                if let Some(val) = env.get(interned) {
                    return Ok(*val);
                }

                // Special symbols that evaluate to themselves
                let sym_str = interned.resolve();
                if sym_str == "t" {
                    return Ok(codegen.compile_bool(true));
                }
                if sym_str == "nil" {
                    return Ok(codegen.compile_nil());
                }

                // Otherwise, compile as a symbol literal (for quote, etc.)
                let mut key: u64 = 0;
                let sym_bytes = unsafe {
                    std::slice::from_raw_parts(
                        interned as *const _ as *const u8,
                        std::mem::size_of_val(interned),
                    )
                };
                for (i, &byte) in sym_bytes.iter().enumerate() {
                    key |= (byte as u64) << (i * 8);
                }
                Ok(codegen.compile_symbol(key))
            }

            Value::Atom(AtomType::String(_)) => {
                // String compilation requires heap allocation - defer for now
                Err("JIT string literals not yet supported".to_string())
            }

            Value::Cons(cell) => {
                // Try to compile as a function call
                self.compile_call(
                    codegen,
                    &cell.car,
                    &cell.cdr,
                    env,
                    lambdas,
                    compiled_fns,
                    tail_position,
                )
            }

            Value::Vector(_) => Err("JIT vector literals not yet supported".to_string()),

            Value::Lambda(_) => Err("JIT lambda compilation not yet supported".to_string()),

            Value::Macro(_) => Err("Macros should be expanded before JIT compilation".to_string()),

            Value::NativeFn(_) => Err("Native functions cannot be JIT compiled".to_string()),
        }
    }

    /// Compile a function call expression.
    ///
    /// `tail_position` indicates if this call is in tail position for TCO.
    #[allow(clippy::too_many_arguments)]
    fn compile_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        operator: &Value,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // Check if operator is a symbol
        if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = operator {
            let sym_str = sym.resolve();
            match sym_str.as_str() {
                // Special forms
                "quote" => self.compile_quote(codegen, args),
                "cond" => {
                    self.compile_cond(codegen, args, env, lambdas, compiled_fns, tail_position)
                }
                "lambda" => self.compile_closure(codegen, args, env, lambdas, compiled_fns),
                "label" => self.compile_label(codegen, args, env, lambdas, compiled_fns),
                // List operations
                "cons" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_cons,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "car" => {
                    self.compile_unary_op(codegen, args, codegen.rt_car, env, lambdas, compiled_fns)
                }
                "cdr" => {
                    self.compile_unary_op(codegen, args, codegen.rt_cdr, env, lambdas, compiled_fns)
                }
                // Arithmetic operators
                "+" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_add,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "-" => self.compile_minus(codegen, args, env, lambdas, compiled_fns),
                "*" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_mul,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "/" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_div,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                // Comparison operators
                "=" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_num_eq,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "<" => {
                    self.compile_binary_op(codegen, args, codegen.rt_lt, env, lambdas, compiled_fns)
                }
                ">" => {
                    self.compile_binary_op(codegen, args, codegen.rt_gt, env, lambdas, compiled_fns)
                }
                "<=" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_lte,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                ">=" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_gte,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                // Equality and type predicates
                "eq" => {
                    self.compile_binary_op(codegen, args, codegen.rt_eq, env, lambdas, compiled_fns)
                }
                "atom" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_is_atom,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "nil?" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_is_nil,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "number?" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_is_number,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "cons?" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_is_cons,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "not" => {
                    self.compile_unary_op(codegen, args, codegen.rt_not, env, lambdas, compiled_fns)
                }
                // Standard library functions
                "now" => self.compile_nullary_op(codegen, args, codegen.rt_now),
                "length" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_length,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "append" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_append,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "reverse" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_reverse,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "nth" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_nth,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                // Vector operations
                "vector" => self.compile_vector(codegen, args, env, lambdas, compiled_fns),
                "vector-length" => self.compile_unary_op(
                    codegen,
                    args,
                    codegen.rt_vector_length,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                "vector-ref" => self.compile_binary_op(
                    codegen,
                    args,
                    codegen.rt_vector_ref,
                    env,
                    lambdas,
                    compiled_fns,
                ),
                _ => {
                    // Check if it's a compiled function call (recursive call)
                    if let Some(func) = compiled_fns.get(sym) {
                        return self.compile_recursive_call(
                            codegen,
                            *func,
                            args,
                            env,
                            lambdas,
                            compiled_fns,
                            tail_position,
                        );
                    }
                    // Check if it's a labeled function call (non-recursive case)
                    if let Some(Value::Cons(lambda_cell)) = lambdas.get(sym)
                        && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(lambda_sym))) =
                            &lambda_cell.car
                        && lambda_sym.resolve() == "lambda"
                    {
                        return self.compile_lambda_call(
                            codegen,
                            &lambda_cell.cdr,
                            args,
                            env,
                            lambdas,
                            compiled_fns,
                        );
                    }
                    Err(format!("JIT does not yet support operator: {}", sym_str))
                }
            }
        } else if let Value::Cons(cell) = operator {
            // Check if it's a lambda expression: ((lambda (params) body) args)
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = &cell.car {
                let sym_str = sym.resolve();
                if sym_str == "lambda" {
                    return self.compile_lambda_call(
                        codegen,
                        &cell.cdr,
                        args,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                if sym_str == "label" {
                    // It's a labeled lambda call: ((label name (lambda ...)) args)
                    return self.compile_labeled_lambda_call(
                        codegen,
                        &cell.cdr,
                        args,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
            }
            // The operator is some other expression - compile it and call the result as a closure
            // Operator is NOT in tail position - we need to call it, not just return it
            let closure_val =
                self.compile_value(codegen, operator, env, lambdas, compiled_fns, false)?;
            self.compile_closure_call(codegen, closure_val, args, env, lambdas, compiled_fns)
        } else {
            Err("JIT can only call named functions or lambda expressions".to_string())
        }
    }

    /// Compile a call to a recursive function.
    ///
    /// `tail_position` enables tail call optimization when true.
    #[allow(clippy::too_many_arguments)]
    fn compile_recursive_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        func: FunctionValue<'ctx>,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // Compile each argument (arguments are NOT in tail position)
        let arg_values = self.collect_args(args)?;
        let compiled_args: Vec<inkwell::values::BasicMetadataValueEnum> = arg_values
            .iter()
            .map(|arg| {
                self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)
                    .map(|v| v.into())
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Generate the call instruction
        let call_site = codegen
            .builder
            .build_call(func, &compiled_args, "recursive_call")
            .map_err(|e| e.to_string())?;

        // Mark as tail call if in tail position
        if tail_position {
            call_site.set_tail_call(true);
        }

        let result = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Recursive call did not return a value".to_string())?
            .into_struct_value();

        Ok(result)
    }

    /// Compile a call to a labeled lambda: ((label name (lambda ...)) args)
    /// This generates an actual LLVM function for the lambda, enabling recursion.
    fn compile_labeled_lambda_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        label_parts: &Value,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // label_parts should be (name (lambda ...))
        let parts = self.collect_args(label_parts)?;
        if parts.len() != 2 {
            return Err("label requires name and lambda".to_string());
        }

        // Get the name
        let name = match &parts[0] {
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => *sym,
            _ => return Err("label name must be a symbol".to_string()),
        };

        // Get the lambda expression
        let lambda_expr = &parts[1];

        // Parse the lambda to get parameters and body
        let (param_symbols, body) = if let Value::Cons(lambda_cell) = lambda_expr {
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(lambda_sym))) = &lambda_cell.car
            {
                if lambda_sym.resolve() == "lambda" {
                    let lambda_parts = self.collect_args(&lambda_cell.cdr)?;
                    if lambda_parts.len() < 2 {
                        return Err("lambda requires parameters and body".to_string());
                    }
                    let params = &lambda_parts[0];
                    let body = lambda_parts[1].clone();

                    let param_names = self.collect_args(params)?;
                    let param_symbols: Vec<InternedSymbol> = param_names
                        .iter()
                        .map(|p| {
                            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = p {
                                Ok(*sym)
                            } else {
                                Err("lambda parameters must be symbols".to_string())
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (param_symbols, body)
                } else {
                    return Err("label second argument must be a lambda".to_string());
                }
            } else {
                return Err("label second argument must be a lambda".to_string());
            }
        } else {
            return Err("label second argument must be a lambda".to_string());
        };

        // Generate a unique function name
        let counter = EXPR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let fn_name = format!("__consair_labeled_{}_{}", name.resolve(), counter);

        // Create the function type: (RuntimeValue, RuntimeValue, ...) -> RuntimeValue
        let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = (0..param_symbols.len())
            .map(|_| codegen.value_type.into())
            .collect();
        let fn_type = codegen.value_type.fn_type(&param_types, false);

        // Save the current insertion point so we can restore it later
        let saved_block = codegen.builder.get_insert_block();

        // Declare the function first (so recursive calls can reference it)
        let function = codegen.module.add_function(&fn_name, fn_type, None);

        // Add the function to compiled_fns for recursive calls
        let mut new_compiled_fns = compiled_fns.clone();
        new_compiled_fns.insert(name, function);

        // Create entry block for the function
        let entry = self.context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(entry);

        // Create new environment with parameters bound to function arguments
        let mut fn_env = env.clone();
        for (i, sym) in param_symbols.iter().enumerate() {
            let param = function
                .get_nth_param(i as u32)
                .ok_or_else(|| "Failed to get function parameter".to_string())?
                .into_struct_value();
            fn_env.insert(*sym, param);
        }

        // Compile the body with the new environment and compiled_fns (body is in tail position)
        let result =
            self.compile_value(codegen, &body, &fn_env, lambdas, &new_compiled_fns, true)?;

        // Return the result
        codegen
            .builder
            .build_return(Some(&result))
            .map_err(|e| e.to_string())?;

        // Restore the saved insertion point
        if let Some(block) = saved_block {
            codegen.builder.position_at_end(block);
        }

        // Now compile the initial call to the function with the provided arguments
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != param_symbols.len() {
            return Err(format!(
                "label lambda expects {} arguments, got {}",
                param_symbols.len(),
                arg_values.len()
            ));
        }

        // Compile each argument (arguments are NOT in tail position)
        let compiled_args: Vec<inkwell::values::BasicMetadataValueEnum> = arg_values
            .iter()
            .map(|arg| {
                self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)
                    .map(|v| v.into())
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Generate the call to the function
        let call_result = codegen
            .builder
            .build_call(function, &compiled_args, "label_call")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Label call did not return a value".to_string())?
            .into_struct_value();

        Ok(call_result)
    }

    /// Compile a label expression: (label name lambda-expr)
    fn compile_label<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        _env: &JitEnv<'ctx>,
        _lambdas: &LambdaStore,
        _compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != 2 {
            return Err("label requires exactly 2 arguments: name and lambda".to_string());
        }

        // Get the name
        let _name = match &arg_values[0] {
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => *sym,
            _ => return Err("label name must be a symbol".to_string()),
        };

        // The result of label is the lambda itself, which we compile
        // but we want to return a nil since label is typically used for its side effect
        // Actually, in Consair, (label name fn) evaluates to the fn value
        // For JIT, since we can't return lambdas as values, return nil
        Ok(codegen.compile_nil())
    }

    /// Compile a lambda call: ((lambda (params) body) args)
    fn compile_lambda_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        lambda_parts: &Value,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // lambda_parts should be ((params) body)
        let parts = self.collect_args(lambda_parts)?;
        if parts.len() < 2 {
            return Err("lambda requires parameters and body".to_string());
        }

        let params = &parts[0];
        let body = &parts[1];

        // Collect parameter names
        let param_names = self.collect_args(params)?;
        let param_symbols: Vec<InternedSymbol> = param_names
            .iter()
            .map(|p| {
                if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = p {
                    Ok(*sym)
                } else {
                    Err("lambda parameters must be symbols".to_string())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Compile arguments
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != param_symbols.len() {
            return Err(format!(
                "lambda expects {} arguments, got {}",
                param_symbols.len(),
                arg_values.len()
            ));
        }

        // Compile each argument (arguments are NOT in tail position)
        let compiled_args: Vec<inkwell::values::StructValue<'ctx>> = arg_values
            .iter()
            .map(|arg| self.compile_value(codegen, arg, env, lambdas, compiled_fns, false))
            .collect::<Result<Vec<_>, _>>()?;

        // Create new environment with parameter bindings
        let mut new_env = env.clone();
        for (sym, val) in param_symbols.iter().zip(compiled_args.iter()) {
            new_env.insert(*sym, *val);
        }

        // Compile the body with the new environment (body IS in tail position)
        self.compile_value(codegen, body, &new_env, lambdas, compiled_fns, true)
    }

    /// Compile a lambda expression into a closure value.
    /// This handles lambdas that capture free variables from their environment.
    ///
    /// Closure functions use a uniform calling convention:
    /// `(env_ptr: *RuntimeValue, args_ptr: *RuntimeValue, num_args: u32) -> RuntimeValue`
    /// This allows all closures to be called uniformly via indirect calls.
    fn compile_closure<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        lambda_parts: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // Parse lambda parts: ((params) body)
        let parts = self.collect_args(lambda_parts)?;
        if parts.len() < 2 {
            return Err("lambda requires parameters and body".to_string());
        }

        let params = &parts[0];
        let body = &parts[1];

        // Collect parameter names
        let param_names = self.collect_args(params)?;
        let param_symbols: Vec<InternedSymbol> = param_names
            .iter()
            .map(|p| {
                if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = p {
                    Ok(*sym)
                } else {
                    Err("lambda parameters must be symbols".to_string())
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Find free variables in the body
        let mut bound_vars: HashSet<InternedSymbol> = param_symbols.iter().cloned().collect();
        // Also add any recursively bound names from compiled_fns
        for key in compiled_fns.keys() {
            bound_vars.insert(*key);
        }
        let free_vars = find_free_variables(body, &bound_vars);
        let free_var_list: Vec<InternedSymbol> = free_vars.into_iter().collect();

        // Generate a unique function name for the closure
        let counter = EXPR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let fn_name = format!("__consair_closure_{}", counter);

        // Save the current insertion point
        let saved_block = codegen.builder.get_insert_block();

        // Create the closure function with uniform signature:
        // (env_ptr: *RuntimeValue, args_ptr: *RuntimeValue, num_args: u32) -> RuntimeValue
        let closure_fn = codegen
            .module
            .add_function(&fn_name, codegen.closure_fn_type(), None);

        // Create entry block for the closure function
        let entry = self.context.append_basic_block(closure_fn, "entry");
        codegen.builder.position_at_end(entry);

        // Get parameters: env_ptr, args_ptr, num_args
        let env_ptr = closure_fn
            .get_nth_param(0)
            .ok_or_else(|| "Failed to get env_ptr parameter".to_string())?
            .into_pointer_value();
        let args_ptr = closure_fn
            .get_nth_param(1)
            .ok_or_else(|| "Failed to get args_ptr parameter".to_string())?
            .into_pointer_value();
        let _num_args = closure_fn
            .get_nth_param(2)
            .ok_or_else(|| "Failed to get num_args parameter".to_string())?
            .into_int_value();

        // Create new environment with captured values and parameters bound
        let mut closure_env = JitEnv::new();

        // Load captured values from env_ptr and add to environment
        for (i, sym) in free_var_list.iter().enumerate() {
            let idx = codegen.i32_type().const_int(i as u64, false);
            let elem_ptr = unsafe {
                codegen.builder.build_gep(
                    codegen.value_type,
                    env_ptr,
                    &[idx],
                    &format!("env_{}", i),
                )
            }
            .map_err(|e| e.to_string())?;

            let val = codegen
                .builder
                .build_load(
                    codegen.value_type,
                    elem_ptr,
                    &format!("cap_{}", sym.resolve()),
                )
                .map_err(|e| e.to_string())?
                .into_struct_value();
            closure_env.insert(*sym, val);
        }

        // Load regular parameters from args_ptr and add to environment
        for (i, sym) in param_symbols.iter().enumerate() {
            let idx = codegen.i32_type().const_int(i as u64, false);
            let elem_ptr = unsafe {
                codegen.builder.build_gep(
                    codegen.value_type,
                    args_ptr,
                    &[idx],
                    &format!("arg_{}", i),
                )
            }
            .map_err(|e| e.to_string())?;

            let val = codegen
                .builder
                .build_load(
                    codegen.value_type,
                    elem_ptr,
                    &format!("param_{}", sym.resolve()),
                )
                .map_err(|e| e.to_string())?
                .into_struct_value();
            closure_env.insert(*sym, val);
        }

        // Compile the body with the closure environment (body IS in tail position)
        let result =
            self.compile_value(codegen, body, &closure_env, lambdas, compiled_fns, true)?;

        // Return the result
        codegen
            .builder
            .build_return(Some(&result))
            .map_err(|e| e.to_string())?;

        // Restore the saved insertion point
        if let Some(block) = saved_block {
            codegen.builder.position_at_end(block);
        }

        // Now generate code to create the closure at runtime:
        // 1. Allocate space for captured values on the stack
        // 2. Store captured values
        // 3. Call rt_make_closure

        // Get the function pointer
        let fn_ptr = closure_fn.as_global_value().as_pointer_value();

        if free_var_list.is_empty() {
            // No captures - create a simple closure with null env
            let null_ptr = codegen.ptr_type().const_null();
            let env_size = codegen.i32_type().const_int(0, false);

            let closure_val = codegen
                .builder
                .build_call(
                    codegen.rt_make_closure,
                    &[fn_ptr.into(), null_ptr.into(), env_size.into()],
                    "closure",
                )
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| "rt_make_closure did not return a value".to_string())?
                .into_struct_value();

            Ok(closure_val)
        } else {
            // Allocate space for captured values on the stack
            let array_type = codegen.value_type.array_type(free_var_list.len() as u32);
            let env_array = codegen
                .builder
                .build_alloca(array_type, "captured_env")
                .map_err(|e| e.to_string())?;

            // Store each captured value
            for (i, sym) in free_var_list.iter().enumerate() {
                let val = env
                    .get(sym)
                    .ok_or_else(|| format!("Undefined variable in closure: {}", sym.resolve()))?;

                let idx = codegen.i32_type().const_int(i as u64, false);
                let ptr = unsafe {
                    codegen.builder.build_gep(
                        array_type,
                        env_array,
                        &[codegen.i32_type().const_int(0, false), idx],
                        "env_ptr",
                    )
                }
                .map_err(|e| e.to_string())?;

                codegen
                    .builder
                    .build_store(ptr, *val)
                    .map_err(|e| e.to_string())?;
            }

            // Cast the array pointer to a generic pointer
            let env_ptr = codegen
                .builder
                .build_pointer_cast(env_array, codegen.ptr_type(), "env_cast")
                .map_err(|e| e.to_string())?;

            let env_size = codegen
                .i32_type()
                .const_int(free_var_list.len() as u64, false);

            let closure_val = codegen
                .builder
                .build_call(
                    codegen.rt_make_closure,
                    &[fn_ptr.into(), env_ptr.into(), env_size.into()],
                    "closure",
                )
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| "rt_make_closure did not return a value".to_string())?
                .into_struct_value();

            Ok(closure_val)
        }
    }

    /// Compile a call to a closure value.
    /// The closure was created by `compile_closure` and uses the uniform calling convention:
    /// `(env_ptr: *RuntimeValue, args_ptr: *RuntimeValue, num_args: u32) -> RuntimeValue`
    fn compile_closure_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        closure_val: inkwell::values::StructValue<'ctx>,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // Compile arguments (arguments are NOT in tail position)
        let arg_values = self.collect_args(args)?;
        let compiled_args: Vec<inkwell::values::StructValue<'ctx>> = arg_values
            .iter()
            .map(|arg| self.compile_value(codegen, arg, env, lambdas, compiled_fns, false))
            .collect::<Result<Vec<_>, _>>()?;

        // Allocate space for arguments on the stack
        let num_args = compiled_args.len();
        let args_array = if num_args > 0 {
            let array_type = codegen.value_type.array_type(num_args as u32);
            let args_arr = codegen
                .builder
                .build_alloca(array_type, "closure_args")
                .map_err(|e| e.to_string())?;

            // Store each argument
            for (i, arg_val) in compiled_args.iter().enumerate() {
                let idx = codegen.i32_type().const_int(i as u64, false);
                let ptr = unsafe {
                    codegen.builder.build_gep(
                        array_type,
                        args_arr,
                        &[codegen.i32_type().const_int(0, false), idx],
                        &format!("arg_ptr_{}", i),
                    )
                }
                .map_err(|e| e.to_string())?;

                codegen
                    .builder
                    .build_store(ptr, *arg_val)
                    .map_err(|e| e.to_string())?;
            }

            codegen
                .builder
                .build_pointer_cast(args_arr, codegen.ptr_type(), "args_cast")
                .map_err(|e| e.to_string())?
        } else {
            codegen.ptr_type().const_null()
        };

        // Get the function pointer from the closure
        let fn_ptr = codegen
            .builder
            .build_call(codegen.rt_closure_fn_ptr, &[closure_val.into()], "fn_ptr")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "rt_closure_fn_ptr did not return a value".to_string())?
            .into_pointer_value();

        // Get the environment pointer from the closure
        // The env pointer is stored in the data field of the closure RuntimeValue
        // We need to extract it - for now we'll pass the closure value itself
        // and let the closure function use rt_closure_env_get
        // Actually, we need to get the env_ptr that was passed to rt_make_closure

        // The closure stores: fn_ptr in one place, and the captured values as an array
        // We need a way to get the env_ptr back
        // For now, let's allocate an env array and fill it from rt_closure_env_get

        // Get the env size
        let env_size = codegen
            .builder
            .build_call(
                codegen.rt_closure_env_size,
                &[closure_val.into()],
                "env_size",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "rt_closure_env_size did not return a value".to_string())?
            .into_int_value();

        // For simplicity, we'll preallocate a max-size env array and fill it
        // A better approach would be to store the env_ptr directly in the closure
        // For now, allocate space for captured values and fill from rt_closure_env_get
        let max_env_size = 16u32; // Support up to 16 captures
        let env_array_type = codegen.value_type.array_type(max_env_size);
        let env_array = codegen
            .builder
            .build_alloca(env_array_type, "closure_env")
            .map_err(|e| e.to_string())?;

        // Get the current function for creating basic blocks
        let current_block = codegen
            .builder
            .get_insert_block()
            .ok_or("No current block")?;
        let function = current_block
            .get_parent()
            .ok_or("Block has no parent function")?;

        // Create blocks for the env loading loop
        let loop_header = self.context.append_basic_block(function, "env_loop_header");
        let loop_body = self.context.append_basic_block(function, "env_loop_body");
        let loop_end = self.context.append_basic_block(function, "env_loop_end");

        // Initialize loop counter
        let counter_ptr = codegen
            .builder
            .build_alloca(codegen.i32_type(), "env_counter")
            .map_err(|e| e.to_string())?;
        codegen
            .builder
            .build_store(counter_ptr, codegen.i32_type().const_int(0, false))
            .map_err(|e| e.to_string())?;

        codegen
            .builder
            .build_unconditional_branch(loop_header)
            .map_err(|e| e.to_string())?;

        // Loop header: check if counter < env_size
        codegen.builder.position_at_end(loop_header);
        let counter = codegen
            .builder
            .build_load(codegen.i32_type(), counter_ptr, "counter")
            .map_err(|e| e.to_string())?
            .into_int_value();
        let cond = codegen
            .builder
            .build_int_compare(inkwell::IntPredicate::ULT, counter, env_size, "cmp")
            .map_err(|e| e.to_string())?;
        codegen
            .builder
            .build_conditional_branch(cond, loop_body, loop_end)
            .map_err(|e| e.to_string())?;

        // Loop body: load env value and store in array
        codegen.builder.position_at_end(loop_body);
        let counter_val = codegen
            .builder
            .build_load(codegen.i32_type(), counter_ptr, "counter_val")
            .map_err(|e| e.to_string())?
            .into_int_value();

        let env_val = codegen
            .builder
            .build_call(
                codegen.rt_closure_env_get,
                &[closure_val.into(), counter_val.into()],
                "env_val",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "rt_closure_env_get did not return a value".to_string())?
            .into_struct_value();

        let env_elem_ptr = unsafe {
            codegen.builder.build_gep(
                env_array_type,
                env_array,
                &[codegen.i32_type().const_int(0, false), counter_val],
                "env_elem_ptr",
            )
        }
        .map_err(|e| e.to_string())?;

        codegen
            .builder
            .build_store(env_elem_ptr, env_val)
            .map_err(|e| e.to_string())?;

        // Increment counter
        let next_counter = codegen
            .builder
            .build_int_add(counter_val, codegen.i32_type().const_int(1, false), "next")
            .map_err(|e| e.to_string())?;
        codegen
            .builder
            .build_store(counter_ptr, next_counter)
            .map_err(|e| e.to_string())?;

        codegen
            .builder
            .build_unconditional_branch(loop_header)
            .map_err(|e| e.to_string())?;

        // After loop: call the closure function
        codegen.builder.position_at_end(loop_end);

        let env_ptr = codegen
            .builder
            .build_pointer_cast(env_array, codegen.ptr_type(), "env_ptr_cast")
            .map_err(|e| e.to_string())?;

        let num_args_val = codegen.i32_type().const_int(num_args as u64, false);

        // Build indirect call through function pointer
        let result = codegen
            .builder
            .build_indirect_call(
                codegen.closure_fn_type(),
                fn_ptr,
                &[env_ptr.into(), args_array.into(), num_args_val.into()],
                "closure_call",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Closure call did not return a value".to_string())?
            .into_struct_value();

        Ok(result)
    }

    /// Compile a binary operation (like +, *, /).
    fn compile_binary_op<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        func: inkwell::values::FunctionValue<'ctx>,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        // Collect arguments from the list
        let arg_values = self.collect_args(args)?;

        if arg_values.is_empty() {
            return Err("Binary operator requires at least one argument".to_string());
        }

        // Compile the first argument (arguments to binary ops are NOT in tail position)
        let mut result =
            self.compile_value(codegen, &arg_values[0], env, lambdas, compiled_fns, false)?;

        // Apply the operation left-to-right for remaining arguments
        for arg in &arg_values[1..] {
            let compiled_arg =
                self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)?;
            result = codegen
                .builder
                .build_call(func, &[result.into(), compiled_arg.into()], "binop")
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| "Binary op did not return a value".to_string())?
                .into_struct_value();
        }

        Ok(result)
    }

    /// Compile the minus operator, which can be unary or binary.
    fn compile_minus<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;

        match arg_values.len() {
            0 => Err("- requires at least one argument".to_string()),
            1 => {
                // Unary negation (argument is NOT in tail position)
                let compiled =
                    self.compile_value(codegen, &arg_values[0], env, lambdas, compiled_fns, false)?;
                let result = codegen
                    .builder
                    .build_call(codegen.rt_neg, &[compiled.into()], "neg")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| "Negation did not return a value".to_string())?
                    .into_struct_value();
                Ok(result)
            }
            _ => {
                // Binary subtraction
                self.compile_binary_op(codegen, args, codegen.rt_sub, env, lambdas, compiled_fns)
            }
        }
    }

    /// Compile a cond expression with branching.
    ///
    /// `tail_position` indicates whether the cond expression itself is in tail position,
    /// which propagates to the result expressions of each clause for TCO.
    fn compile_cond<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let clauses = self.collect_args(args)?;

        if clauses.is_empty() {
            // Empty cond returns nil
            return Ok(codegen.compile_nil());
        }

        // Get the current function
        let current_block = codegen
            .builder
            .get_insert_block()
            .ok_or("No current block")?;
        let function = current_block
            .get_parent()
            .ok_or("Block has no parent function")?;

        // Create a merge block where all branches will converge
        let merge_block = self.context.append_basic_block(function, "cond_merge");

        // We'll collect (value, from_block) pairs for the phi node
        let mut phi_incoming: Vec<(
            inkwell::values::BasicValueEnum<'_>,
            inkwell::basic_block::BasicBlock<'_>,
        )> = Vec::new();

        // Process each clause
        for (i, clause) in clauses.iter().enumerate() {
            // Each clause should be a list of (test result)
            let clause_parts = self.collect_args(clause)?;
            if clause_parts.len() < 2 {
                return Err("cond clause must have at least 2 elements".to_string());
            }

            let test_expr = &clause_parts[0];
            let result_expr = &clause_parts[1];

            // Check if this is the final 't' clause (always true)
            let is_final_t = matches!(
                test_expr,
                Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym)))
                    if sym.resolve() == "t"
            );

            if is_final_t || i == clauses.len() - 1 {
                // This is the final else clause - compile result and branch to merge
                // Result expression is in tail position if the cond is
                let result_val = self.compile_value(
                    codegen,
                    result_expr,
                    env,
                    lambdas,
                    compiled_fns,
                    tail_position,
                )?;
                let current = codegen
                    .builder
                    .get_insert_block()
                    .ok_or("No current block")?;
                phi_incoming.push((result_val.into(), current));
                codegen
                    .builder
                    .build_unconditional_branch(merge_block)
                    .map_err(|e| e.to_string())?;
                break;
            }

            // Compile the test expression (test is NOT in tail position)
            let test_val =
                self.compile_value(codegen, test_expr, env, lambdas, compiled_fns, false)?;

            // Check if test is truthy (not nil and not false)
            // We need to extract the tag and data from the struct
            let tag = codegen
                .builder
                .build_extract_value(test_val, 0, "tag")
                .map_err(|e| e.to_string())?
                .into_int_value();

            let data = codegen
                .builder
                .build_extract_value(test_val, 1, "data")
                .map_err(|e| e.to_string())?
                .into_int_value();

            // Check if tag == TAG_NIL (0)
            let is_nil = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    codegen
                        .i8_type()
                        .const_int(crate::runtime::TAG_NIL as u64, false),
                    "is_nil",
                )
                .map_err(|e| e.to_string())?;

            // Check if tag == TAG_BOOL (1) and data == 0 (false)
            let is_bool = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    codegen
                        .i8_type()
                        .const_int(crate::runtime::TAG_BOOL as u64, false),
                    "is_bool",
                )
                .map_err(|e| e.to_string())?;

            let is_false_data = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    data,
                    codegen.i64_type().const_int(0, false),
                    "is_false_data",
                )
                .map_err(|e| e.to_string())?;

            let is_false = codegen
                .builder
                .build_and(is_bool, is_false_data, "is_false")
                .map_err(|e| e.to_string())?;

            // Falsy if nil OR (bool AND data==0)
            let is_falsy = codegen
                .builder
                .build_or(is_nil, is_false, "is_falsy")
                .map_err(|e| e.to_string())?;

            // Create blocks for then and else
            let then_block = self
                .context
                .append_basic_block(function, &format!("cond_then_{}", i));
            let else_block = self
                .context
                .append_basic_block(function, &format!("cond_else_{}", i));

            // Branch based on truthiness (if falsy, go to else; if truthy, go to then)
            codegen
                .builder
                .build_conditional_branch(is_falsy, else_block, then_block)
                .map_err(|e| e.to_string())?;

            // Compile the then block (result is in tail position if cond is)
            codegen.builder.position_at_end(then_block);
            let result_val = self.compile_value(
                codegen,
                result_expr,
                env,
                lambdas,
                compiled_fns,
                tail_position,
            )?;
            let then_end = codegen
                .builder
                .get_insert_block()
                .ok_or("No current block")?;
            phi_incoming.push((result_val.into(), then_end));
            codegen
                .builder
                .build_unconditional_branch(merge_block)
                .map_err(|e| e.to_string())?;

            // Continue from the else block for the next clause
            codegen.builder.position_at_end(else_block);
        }

        // If we didn't hit a final 't' clause, we need to handle the fallthrough case
        // (return nil if no clause matched)
        let current = codegen
            .builder
            .get_insert_block()
            .ok_or("No current block")?;
        if current != merge_block && current.get_terminator().is_none() {
            let nil_val = codegen.compile_nil();
            phi_incoming.push((nil_val.into(), current));
            codegen
                .builder
                .build_unconditional_branch(merge_block)
                .map_err(|e| e.to_string())?;
        }

        // Position at merge block and create phi node
        codegen.builder.position_at_end(merge_block);

        if phi_incoming.is_empty() {
            // No clauses at all - return nil
            return Ok(codegen.compile_nil());
        }

        let phi = codegen
            .builder
            .build_phi(codegen.value_type, "cond_result")
            .map_err(|e| e.to_string())?;

        for (val, block) in &phi_incoming {
            phi.add_incoming(&[(val, *block)]);
        }

        Ok(phi.as_basic_value().into_struct_value())
    }

    /// Compile a quote expression - returns the argument unevaluated.
    fn compile_quote<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;

        if arg_values.len() != 1 {
            return Err("quote requires exactly one argument".to_string());
        }

        // Compile the quoted value as a literal (not as an expression)
        self.compile_quoted_value(codegen, &arg_values[0])
    }

    /// Compile a quoted value (builds data structures without evaluating).
    #[allow(clippy::only_used_in_recursion)]
    fn compile_quoted_value<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        value: &Value,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        match value {
            Value::Nil => Ok(codegen.compile_nil()),

            Value::Atom(AtomType::Bool(b)) => Ok(codegen.compile_bool(*b)),

            Value::Atom(AtomType::Number(num)) => match num {
                NumericType::Int(n) => Ok(codegen.compile_int(*n)),
                NumericType::Float(f) => Ok(codegen.compile_float(*f)),
                NumericType::Ratio(num, denom) => {
                    Ok(codegen.compile_float(*num as f64 / *denom as f64))
                }
                NumericType::BigInt(_) => Err("JIT does not support BigInt".to_string()),
                NumericType::BigRatio(_) => Err("JIT does not support BigRatio".to_string()),
            },

            Value::Atom(AtomType::Symbol(sym)) => {
                let SymbolType::Symbol(interned) = sym;
                let mut key: u64 = 0;
                let sym_bytes = unsafe {
                    std::slice::from_raw_parts(
                        interned as *const _ as *const u8,
                        std::mem::size_of_val(interned),
                    )
                };
                for (i, &byte) in sym_bytes.iter().enumerate() {
                    key |= (byte as u64) << (i * 8);
                }
                Ok(codegen.compile_symbol(key))
            }

            Value::Cons(cell) => {
                // Build the cons cell at runtime using rt_cons
                let car_val = self.compile_quoted_value(codegen, &cell.car)?;
                let cdr_val = self.compile_quoted_value(codegen, &cell.cdr)?;

                let result = codegen
                    .builder
                    .build_call(codegen.rt_cons, &[car_val.into(), cdr_val.into()], "cons")
                    .map_err(|e| e.to_string())?
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| "cons did not return a value".to_string())?
                    .into_struct_value();

                Ok(result)
            }

            Value::Atom(AtomType::String(_)) => {
                Err("JIT does not yet support quoted strings".to_string())
            }

            Value::Vector(_) => Err("JIT does not yet support quoted vectors".to_string()),

            Value::Lambda(_) => Err("Cannot quote lambdas".to_string()),

            Value::Macro(_) => Err("Cannot quote macros".to_string()),

            Value::NativeFn(_) => Err("Cannot quote native functions".to_string()),
        }
    }

    /// Compile a vector construction.
    fn compile_vector<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;
        let len = arg_values.len() as u32;

        // If no elements, call with null pointer
        if arg_values.is_empty() {
            let null_ptr = codegen.ptr_type().const_null();
            let len_val = codegen.i32_type().const_int(0, false);

            let result = codegen
                .builder
                .build_call(
                    codegen.rt_make_vector,
                    &[null_ptr.into(), len_val.into()],
                    "make_vector",
                )
                .map_err(|e| e.to_string())?
                .try_as_basic_value()
                .left()
                .ok_or_else(|| "rt_make_vector did not return a value".to_string())?
                .into_struct_value();

            return Ok(result);
        }

        // Compile all elements
        let mut compiled_elements = Vec::new();
        for arg in &arg_values {
            let compiled = self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)?;
            compiled_elements.push(compiled);
        }

        // Allocate stack space for the array
        let array_type = codegen.value_type.array_type(len);
        let array_ptr = codegen
            .builder
            .build_alloca(array_type, "vector_elements")
            .map_err(|e| e.to_string())?;

        // Store each element in the array
        for (i, elem) in compiled_elements.iter().enumerate() {
            let indices = [
                codegen.context.i32_type().const_int(0, false),
                codegen.context.i32_type().const_int(i as u64, false),
            ];
            let elem_ptr = unsafe {
                codegen
                    .builder
                    .build_gep(array_type, array_ptr, &indices, &format!("elem_ptr_{i}"))
                    .map_err(|e| e.to_string())?
            };
            codegen
                .builder
                .build_store(elem_ptr, *elem)
                .map_err(|e| e.to_string())?;
        }

        // Cast to *RuntimeValue and call rt_make_vector
        let elements_ptr = codegen
            .builder
            .build_pointer_cast(array_ptr, codegen.ptr_type(), "elements_ptr")
            .map_err(|e| e.to_string())?;
        let len_val = codegen.i32_type().const_int(len as u64, false);

        let result = codegen
            .builder
            .build_call(
                codegen.rt_make_vector,
                &[elements_ptr.into(), len_val.into()],
                "make_vector",
            )
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "rt_make_vector did not return a value".to_string())?
            .into_struct_value();

        Ok(result)
    }

    /// Compile a nullary operation (like now).
    fn compile_nullary_op<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        func: inkwell::values::FunctionValue<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;

        if !arg_values.is_empty() {
            return Err("Nullary operator takes no arguments".to_string());
        }

        let result = codegen
            .builder
            .build_call(func, &[], "nullary")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Nullary op did not return a value".to_string())?
            .into_struct_value();

        Ok(result)
    }

    /// Compile a unary operation (like not, atom, nil?, etc.).
    fn compile_unary_op<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        func: inkwell::values::FunctionValue<'ctx>,
        env: &JitEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<inkwell::values::StructValue<'ctx>, String> {
        let arg_values = self.collect_args(args)?;

        if arg_values.len() != 1 {
            return Err("Unary operator requires exactly one argument".to_string());
        }

        // Argument to unary op is NOT in tail position
        let compiled =
            self.compile_value(codegen, &arg_values[0], env, lambdas, compiled_fns, false)?;
        let result = codegen
            .builder
            .build_call(func, &[compiled.into()], "unary")
            .map_err(|e| e.to_string())?
            .try_as_basic_value()
            .left()
            .ok_or_else(|| "Unary op did not return a value".to_string())?
            .into_struct_value();

        Ok(result)
    }

    /// Collect arguments from a cons list into a Vec.
    fn collect_args(&self, args: &Value) -> Result<Vec<Value>, String> {
        let mut result = Vec::new();
        let mut current = args.clone();

        loop {
            match current {
                Value::Nil => break,
                Value::Cons(cell) => {
                    result.push(cell.car.clone());
                    current = cell.cdr.clone();
                }
                _ => return Err("Malformed argument list".to_string()),
            }
        }

        Ok(result)
    }

    /// Link runtime functions so the JIT can call them.
    fn link_runtime_functions<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        engine: &ExecutionEngine<'ctx>,
    ) {
        use crate::runtime::*;

        // Map declared functions to their actual addresses
        engine.add_global_mapping(&codegen.rt_cons, rt_cons as usize);
        engine.add_global_mapping(&codegen.rt_car, rt_car as usize);
        engine.add_global_mapping(&codegen.rt_cdr, rt_cdr as usize);
        engine.add_global_mapping(&codegen.rt_add, rt_add as usize);
        engine.add_global_mapping(&codegen.rt_sub, rt_sub as usize);
        engine.add_global_mapping(&codegen.rt_mul, rt_mul as usize);
        engine.add_global_mapping(&codegen.rt_div, rt_div as usize);
        engine.add_global_mapping(&codegen.rt_neg, rt_neg as usize);
        engine.add_global_mapping(&codegen.rt_num_eq, rt_num_eq as usize);
        engine.add_global_mapping(&codegen.rt_lt, rt_lt as usize);
        engine.add_global_mapping(&codegen.rt_gt, rt_gt as usize);
        engine.add_global_mapping(&codegen.rt_lte, rt_lte as usize);
        engine.add_global_mapping(&codegen.rt_gte, rt_gte as usize);
        engine.add_global_mapping(&codegen.rt_eq, rt_eq as usize);
        engine.add_global_mapping(&codegen.rt_is_nil, rt_is_nil as usize);
        engine.add_global_mapping(&codegen.rt_is_atom, rt_is_atom as usize);
        engine.add_global_mapping(&codegen.rt_is_cons, rt_is_cons as usize);
        engine.add_global_mapping(&codegen.rt_is_number, rt_is_number as usize);
        engine.add_global_mapping(&codegen.rt_not, rt_not as usize);
        engine.add_global_mapping(&codegen.rt_incref, rt_incref as usize);
        engine.add_global_mapping(&codegen.rt_decref, rt_decref as usize);
        // Closure functions
        engine.add_global_mapping(&codegen.rt_make_closure, rt_make_closure as usize);
        engine.add_global_mapping(&codegen.rt_closure_fn_ptr, rt_closure_fn_ptr as usize);
        engine.add_global_mapping(&codegen.rt_closure_env_get, rt_closure_env_get as usize);
        engine.add_global_mapping(&codegen.rt_closure_env_size, rt_closure_env_size as usize);
        // Standard library functions
        engine.add_global_mapping(&codegen.rt_now, rt_now as usize);
        engine.add_global_mapping(&codegen.rt_length, rt_length as usize);
        engine.add_global_mapping(&codegen.rt_append, rt_append as usize);
        engine.add_global_mapping(&codegen.rt_reverse, rt_reverse as usize);
        engine.add_global_mapping(&codegen.rt_nth, rt_nth as usize);
        // Vector functions
        engine.add_global_mapping(&codegen.rt_make_vector, rt_make_vector as usize);
        engine.add_global_mapping(&codegen.rt_vector_length, rt_vector_length as usize);
        engine.add_global_mapping(&codegen.rt_vector_ref, rt_vector_ref as usize);
    }
}

impl Default for JitEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create JIT engine")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;

    #[test]
    fn test_jit_engine_creation() {
        let engine = JitEngine::new();
        assert!(engine.is_ok());
    }

    #[test]
    fn test_eval_integer() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("42").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_negative_integer() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("-123").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(-123));
    }

    #[test]
    fn test_eval_float() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("3.15625").unwrap();
        let result = engine.eval(&expr).unwrap();
        let val = result.to_float().unwrap();
        assert!((val - 3.15625).abs() < 1e-10);
    }

    #[test]
    fn test_eval_nil() {
        let engine = JitEngine::new().unwrap();
        let expr = Value::Nil;
        let result = engine.eval(&expr).unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_eval_bool_true() {
        let engine = JitEngine::new().unwrap();
        let expr = Value::Atom(AtomType::Bool(true));
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_bool(), Some(true));
    }

    #[test]
    fn test_eval_bool_false() {
        let engine = JitEngine::new().unwrap();
        let expr = Value::Atom(AtomType::Bool(false));
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_multiple_expressions() {
        let engine = JitEngine::new().unwrap();

        // Each call should work independently
        let result1 = engine.eval(&parse("1").unwrap()).unwrap();
        let result2 = engine.eval(&parse("2").unwrap()).unwrap();
        let result3 = engine.eval(&parse("3").unwrap()).unwrap();

        assert_eq!(result1.to_int(), Some(1));
        assert_eq!(result2.to_int(), Some(2));
        assert_eq!(result3.to_int(), Some(3));
    }

    // ========================================================================
    // Arithmetic Expression Tests
    // ========================================================================

    #[test]
    fn test_eval_addition() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(+ 1 2)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eval_addition_multiple() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(+ 1 2 3 4)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(10));
    }

    #[test]
    fn test_eval_subtraction() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(- 10 3)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(7));
    }

    #[test]
    fn test_eval_subtraction_multiple() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(- 10 3 2)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(5));
    }

    #[test]
    fn test_eval_negation() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(- 42)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(-42));
    }

    #[test]
    fn test_eval_multiplication() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(* 6 7)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_multiplication_multiple() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(* 2 3 4)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(24));
    }

    #[test]
    fn test_eval_division() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(/ 20 4)").unwrap();
        let result = engine.eval(&expr).unwrap();
        assert_eq!(result.to_int(), Some(5));
    }

    #[test]
    fn test_eval_division_float() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(/ 7 2)").unwrap();
        let result = engine.eval(&expr).unwrap();
        let val = result.to_float().unwrap();
        assert!((val - 3.5).abs() < 1e-10);
    }

    #[test]
    fn test_eval_nested_arithmetic() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(+ (* 2 3) (- 10 5))").unwrap();
        let result = engine.eval(&expr).unwrap();
        // (2 * 3) + (10 - 5) = 6 + 5 = 11
        assert_eq!(result.to_int(), Some(11));
    }

    #[test]
    fn test_eval_deeply_nested() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(* (+ 1 2) (- 8 (/ 10 2)))").unwrap();
        let result = engine.eval(&expr).unwrap();
        // (1 + 2) * (8 - (10 / 2)) = 3 * (8 - 5) = 3 * 3 = 9
        assert_eq!(result.to_int(), Some(9));
    }

    #[test]
    fn test_eval_float_arithmetic() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(+ 1.5 2.5)").unwrap();
        let result = engine.eval(&expr).unwrap();
        // 1.5 + 2.5 = 4.0, which gets converted to int
        assert_eq!(result.to_int(), Some(4));
    }

    #[test]
    fn test_eval_mixed_int_float() {
        let engine = JitEngine::new().unwrap();
        let expr = parse("(+ 1 2.5)").unwrap();
        let result = engine.eval(&expr).unwrap();
        let val = result.to_float().unwrap();
        assert!((val - 3.5).abs() < 1e-10);
    }

    // ========================================================================
    // Comparison Expression Tests
    // ========================================================================

    #[test]
    fn test_eval_numeric_equals() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(= 5 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(= 5 6)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_less_than() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(< 3 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(< 5 3)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));

        let result = engine.eval(&parse("(< 5 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_greater_than() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(> 5 3)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(> 3 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_less_equal() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(<= 3 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(<= 5 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(<= 6 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_greater_equal() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(>= 5 3)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(>= 5 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine.eval(&parse("(>= 3 5)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_not() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(not nil)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));

        let result = engine
            .eval(&Value::Cons(std::sync::Arc::new(
                crate::language::ConsCell {
                    car: Value::Atom(AtomType::Symbol(SymbolType::Symbol(
                        crate::interner::InternedSymbol::new("not"),
                    ))),
                    cdr: Value::Cons(std::sync::Arc::new(crate::language::ConsCell {
                        car: Value::Atom(AtomType::Bool(true)),
                        cdr: Value::Nil,
                    })),
                },
            )))
            .unwrap();
        assert_eq!(result.to_bool(), Some(false));
    }

    #[test]
    fn test_eval_atom() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(atom 42)").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));
    }

    #[test]
    fn test_eval_comparison_with_arithmetic() {
        let engine = JitEngine::new().unwrap();
        // (> (+ 2 3) (* 2 2)) = (> 5 4) = true
        let result = engine.eval(&parse("(> (+ 2 3) (* 2 2))").unwrap()).unwrap();
        assert_eq!(result.to_bool(), Some(true));
    }

    // ========================================================================
    // Quote, Cons, Car, Cdr Tests
    // ========================================================================

    #[test]
    fn test_eval_quote_number() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(quote 42)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_quote_symbol() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(quote foo)").unwrap()).unwrap();
        assert!(result.is_symbol());
    }

    #[test]
    fn test_eval_quote_list() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(quote (1 2 3))").unwrap()).unwrap();
        assert!(result.is_cons());
    }

    #[test]
    fn test_eval_cons() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(cons 1 2)").unwrap()).unwrap();
        assert!(result.is_cons());
    }

    #[test]
    fn test_eval_car() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(car (cons 1 2))").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(1));
    }

    #[test]
    fn test_eval_cdr() {
        let engine = JitEngine::new().unwrap();
        let result = engine.eval(&parse("(cdr (cons 1 2))").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(2));
    }

    #[test]
    fn test_eval_car_of_quoted_list() {
        let engine = JitEngine::new().unwrap();
        let result = engine
            .eval(&parse("(car (quote (1 2 3)))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(1));
    }

    #[test]
    fn test_eval_cdr_of_quoted_list() {
        let engine = JitEngine::new().unwrap();
        let result = engine
            .eval(&parse("(cdr (quote (1 2 3)))").unwrap())
            .unwrap();
        // cdr of (1 2 3) is (2 3), which is a cons cell
        assert!(result.is_cons());
    }

    #[test]
    fn test_eval_nested_car_cdr() {
        let engine = JitEngine::new().unwrap();
        // (car (cdr (quote (1 2 3)))) should be 2
        let result = engine
            .eval(&parse("(car (cdr (quote (1 2 3))))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(2));
    }

    #[test]
    fn test_eval_cons_with_nil() {
        let engine = JitEngine::new().unwrap();
        // (cons 1 nil) should create a proper list (1)
        let result = engine.eval(&parse("(cons 1 nil)").unwrap()).unwrap();
        assert!(result.is_cons());

        // car should be 1
        let car_result = engine.eval(&parse("(car (cons 1 nil))").unwrap()).unwrap();
        assert_eq!(car_result.to_int(), Some(1));

        // cdr should be nil
        let cdr_result = engine.eval(&parse("(cdr (cons 1 nil))").unwrap()).unwrap();
        assert!(cdr_result.is_nil());
    }

    // ========================================================================
    // Cond Expression Tests
    // ========================================================================

    #[test]
    fn test_eval_cond_simple() {
        let engine = JitEngine::new().unwrap();
        // (cond (t 42)) should return 42
        let result = engine.eval(&parse("(cond (t 42))").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_cond_multiple_clauses() {
        let engine = JitEngine::new().unwrap();
        // (cond (nil 1) (t 2)) should return 2
        let result = engine
            .eval(&parse("(cond (nil 1) (t 2))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(2));
    }

    #[test]
    fn test_eval_cond_first_true() {
        let engine = JitEngine::new().unwrap();
        // (cond ((= 1 1) 100) (t 200)) should return 100
        let result = engine
            .eval(&parse("(cond ((= 1 1) 100) (t 200))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(100));
    }

    #[test]
    fn test_eval_cond_second_true() {
        let engine = JitEngine::new().unwrap();
        // (cond ((= 1 2) 100) ((= 2 2) 200) (t 300)) should return 200
        let result = engine
            .eval(&parse("(cond ((= 1 2) 100) ((= 2 2) 200) (t 300))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(200));
    }

    #[test]
    fn test_eval_cond_with_arithmetic() {
        let engine = JitEngine::new().unwrap();
        // (cond ((> 5 3) (+ 10 20)) (t 0)) should return 30
        let result = engine
            .eval(&parse("(cond ((> 5 3) (+ 10 20)) (t 0))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(30));
    }

    #[test]
    fn test_eval_cond_nested() {
        let engine = JitEngine::new().unwrap();
        // Nested cond expressions
        let result = engine
            .eval(&parse("(cond ((= 1 1) (cond ((= 2 2) 42) (t 0))) (t 99))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_cond_no_match_with_default() {
        let engine = JitEngine::new().unwrap();
        // All conditions false, return default
        let result = engine
            .eval(&parse("(cond ((= 1 2) 100) ((= 3 4) 200) (t 999))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(999));
    }

    // ========================================================================
    // Lambda Expression Tests
    // ========================================================================

    #[test]
    fn test_eval_lambda_identity() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x) x) 42)
        let result = engine.eval(&parse("((lambda (x) x) 42)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_eval_lambda_add_one() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x) (+ x 1)) 5)
        let result = engine
            .eval(&parse("((lambda (x) (+ x 1)) 5)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(6));
    }

    #[test]
    fn test_eval_lambda_two_params() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x y) (+ x y)) 3 4)
        let result = engine
            .eval(&parse("((lambda (x y) (+ x y)) 3 4)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(7));
    }

    #[test]
    fn test_eval_lambda_nested_body() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x) (* x (+ x 1))) 5) = 5 * 6 = 30
        let result = engine
            .eval(&parse("((lambda (x) (* x (+ x 1))) 5)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(30));
    }

    #[test]
    fn test_eval_lambda_with_cond() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x) (cond ((= x 0) 0) (t x))) 5)
        let result = engine
            .eval(&parse("((lambda (x) (cond ((= x 0) 0) (t x))) 5)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(5));

        // ((lambda (x) (cond ((= x 0) 0) (t x))) 0)
        let result = engine
            .eval(&parse("((lambda (x) (cond ((= x 0) 0) (t x))) 0)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(0));
    }

    #[test]
    fn test_eval_nested_lambda_call() {
        let engine = JitEngine::new().unwrap();
        // ((lambda (x) ((lambda (y) (+ x y)) 10)) 5) = 5 + 10 = 15
        let result = engine
            .eval(&parse("((lambda (x) ((lambda (y) (+ x y)) 10)) 5)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(15));
    }

    #[test]
    fn test_eval_lambda_shadow_var() {
        let engine = JitEngine::new().unwrap();
        // Inner x shadows outer x
        // ((lambda (x) ((lambda (x) x) 99)) 1) = 99
        let result = engine
            .eval(&parse("((lambda (x) ((lambda (x) x) 99)) 1)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(99));
    }

    // ========================================================================
    // Recursive Function Tests (using label)
    // ========================================================================

    #[test]
    fn test_eval_factorial_recursive() {
        let engine = JitEngine::new().unwrap();
        // Factorial using label for recursion
        let result = engine
            .eval(
                &parse("((label fac (lambda (n) (cond ((= n 0) 1) (t (* n (fac (- n 1))))))) 5)")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(result.to_int(), Some(120));
    }

    #[test]
    fn test_eval_factorial_zero() {
        let engine = JitEngine::new().unwrap();
        let result = engine
            .eval(
                &parse("((label fac (lambda (n) (cond ((= n 0) 1) (t (* n (fac (- n 1))))))) 0)")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(result.to_int(), Some(1));
    }

    #[test]
    fn test_eval_factorial_one() {
        let engine = JitEngine::new().unwrap();
        let result = engine
            .eval(
                &parse("((label fac (lambda (n) (cond ((= n 0) 1) (t (* n (fac (- n 1))))))) 1)")
                    .unwrap(),
            )
            .unwrap();
        assert_eq!(result.to_int(), Some(1));
    }

    #[test]
    fn test_eval_sum_to_n() {
        let engine = JitEngine::new().unwrap();
        // Sum from 1 to n
        let result = engine
            .eval(
                &parse("((label sum (lambda (n) (cond ((= n 0) 0) (t (+ n (sum (- n 1))))))) 10)")
                    .unwrap(),
            )
            .unwrap();
        // 1 + 2 + ... + 10 = 55
        assert_eq!(result.to_int(), Some(55));
    }

    #[test]
    fn test_eval_fibonacci() {
        let engine = JitEngine::new().unwrap();
        // Fibonacci (naive recursive implementation)
        let result = engine
            .eval(&parse(
                "((label fib (lambda (n) (cond ((= n 0) 0) ((= n 1) 1) (t (+ (fib (- n 1)) (fib (- n 2))))))) 10)",
            ).unwrap())
            .unwrap();
        // fib(10) = 55
        assert_eq!(result.to_int(), Some(55));
    }

    // ========================================================================
    // Closure Tests (lambdas with captured variables)
    // ========================================================================

    #[test]
    fn test_eval_closure_simple_capture() {
        let engine = JitEngine::new().unwrap();
        // A lambda that returns a closure capturing x
        // ((lambda (x) (lambda (y) (+ x y))) 5)
        // Returns a closure that captures x=5
        // We can't easily test the returned closure directly, so test the full application
        let result = engine
            .eval(&parse("(((lambda (x) (lambda (y) (+ x y))) 5) 10)").unwrap())
            .unwrap();
        // 5 + 10 = 15
        assert_eq!(result.to_int(), Some(15));
    }

    #[test]
    fn test_eval_closure_multiple_captures() {
        let engine = JitEngine::new().unwrap();
        // Capture multiple variables
        // ((lambda (a b) (lambda (c) (+ a (+ b c)))) 1 2) applied to 3
        let result = engine
            .eval(&parse("(((lambda (a b) (lambda (c) (+ a (+ b c)))) 1 2) 3)").unwrap())
            .unwrap();
        // 1 + 2 + 3 = 6
        assert_eq!(result.to_int(), Some(6));
    }

    #[test]
    fn test_eval_closure_nested() {
        let engine = JitEngine::new().unwrap();
        // Nested closures - currying
        // (((lambda (x) (lambda (y) (lambda (z) (+ x (+ y z))))) 1) 2) applied to 3
        let result = engine
            .eval(
                &parse("((((lambda (x) (lambda (y) (lambda (z) (+ x (+ y z))))) 1) 2) 3)").unwrap(),
            )
            .unwrap();
        // 1 + 2 + 3 = 6
        assert_eq!(result.to_int(), Some(6));
    }

    #[test]
    fn test_eval_closure_with_cond() {
        let engine = JitEngine::new().unwrap();
        // Closure with conditional
        let result = engine
            .eval(&parse("(((lambda (threshold) (lambda (x) (cond ((> x threshold) x) (t threshold)))) 10) 5)").unwrap())
            .unwrap();
        // 5 is not > 10, so return 10
        assert_eq!(result.to_int(), Some(10));

        let result = engine
            .eval(&parse("(((lambda (threshold) (lambda (x) (cond ((> x threshold) x) (t threshold)))) 10) 15)").unwrap())
            .unwrap();
        // 15 > 10, so return 15
        assert_eq!(result.to_int(), Some(15));
    }

    #[test]
    fn test_eval_closure_make_adder() {
        let engine = JitEngine::new().unwrap();
        // Classic make-adder example
        // make-adder returns a closure that adds n to its argument
        let result = engine
            .eval(&parse("(((lambda (n) (lambda (x) (+ n x))) 5) 3)").unwrap())
            .unwrap();
        // 5 + 3 = 8
        assert_eq!(result.to_int(), Some(8));
    }

    #[test]
    fn test_eval_closure_make_multiplier() {
        let engine = JitEngine::new().unwrap();
        // make-multiplier returns a closure that multiplies by n
        let result = engine
            .eval(&parse("(((lambda (n) (lambda (x) (* n x))) 3) 7)").unwrap())
            .unwrap();
        // 3 * 7 = 21
        assert_eq!(result.to_int(), Some(21));
    }

    #[test]
    fn test_eval_closure_compose() {
        let engine = JitEngine::new().unwrap();
        // Compose two operations: add 1 then multiply by 2
        // ((lambda (x) (* 2 (+ x 1))) 5) = (5 + 1) * 2 = 12
        // But with closures:
        // ((((lambda (add-n) (lambda (mul-n) (lambda (x) (* mul-n (+ x add-n))))) 1) 2) 5)
        let result = engine
            .eval(&parse("((((lambda (add_n) (lambda (mul_n) (lambda (x) (* mul_n (+ x add_n))))) 1) 2) 5)").unwrap())
            .unwrap();
        // (5 + 1) * 2 = 12
        assert_eq!(result.to_int(), Some(12));
    }

    #[test]
    fn test_eval_closure_no_captures() {
        let engine = JitEngine::new().unwrap();
        // A closure with no captures should still work
        let result = engine
            .eval(&parse("(((lambda () (lambda (x) (* x x)))) 5)").unwrap())
            .unwrap();
        // 5 * 5 = 25
        assert_eq!(result.to_int(), Some(25));
    }

    // ========================================================================
    // Tail Call Optimization Tests
    // ========================================================================

    #[test]
    fn test_eval_tail_recursive_countdown() {
        let engine = JitEngine::new().unwrap();
        // A tail-recursive countdown function
        // If TCO is working, this should not overflow the stack
        let result = engine
            .eval(
                &parse(
                    "((label countdown (lambda (n) (cond ((= n 0) 0) (t (countdown (- n 1)))))) 1000)",
                )
                .unwrap(),
            )
            .unwrap();
        assert_eq!(result.to_int(), Some(0));
    }

    #[test]
    fn test_eval_tail_recursive_sum() {
        let engine = JitEngine::new().unwrap();
        // A tail-recursive sum with accumulator
        // sum-acc(n, acc) = if n = 0 then acc else sum-acc(n-1, acc+n)
        let result = engine
            .eval(
                &parse(
                    "((label sum-acc (lambda (n acc) (cond ((= n 0) acc) (t (sum-acc (- n 1) (+ acc n)))))) 100 0)",
                )
                .unwrap(),
            )
            .unwrap();
        // Sum of 1 to 100 = 5050
        assert_eq!(result.to_int(), Some(5050));
    }

    #[test]
    fn test_eval_tail_call_in_cond() {
        let engine = JitEngine::new().unwrap();
        // Test that tail calls are properly recognized in cond branches
        let result = engine
            .eval(
                &parse(
                    "((label fact-acc (lambda (n acc) (cond ((= n 0) acc) (t (fact-acc (- n 1) (* acc n)))))) 10 1)",
                )
                .unwrap(),
            )
            .unwrap();
        // 10! = 3628800
        assert_eq!(result.to_int(), Some(3628800));
    }

    // ========================================================================
    // Standard Library Function Tests
    // ========================================================================

    #[test]
    fn test_eval_now() {
        let engine = JitEngine::new().unwrap();
        // (now) should return a reasonable Unix timestamp
        let result = engine.eval(&parse("(now)").unwrap()).unwrap();
        let timestamp = result.to_int().unwrap();
        // Timestamp should be reasonable (after 2020 = 1577836800)
        assert!(timestamp > 1577836800);
    }

    #[test]
    fn test_eval_length_empty() {
        let engine = JitEngine::new().unwrap();
        // (length nil) => 0
        let result = engine.eval(&parse("(length nil)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(0));
    }

    #[test]
    fn test_eval_length_list() {
        let engine = JitEngine::new().unwrap();
        // (length '(1 2 3)) => 3
        let result = engine.eval(&parse("(length '(1 2 3))").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eval_length_single() {
        let engine = JitEngine::new().unwrap();
        // (length '(42)) => 1
        let result = engine.eval(&parse("(length '(42))").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(1));
    }

    #[test]
    fn test_eval_append() {
        let engine = JitEngine::new().unwrap();
        // (length (append '(1 2) '(3 4))) => 4
        let result = engine
            .eval(&parse("(length (append '(1 2) '(3 4)))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(4));
    }

    #[test]
    fn test_eval_append_empty_first() {
        let engine = JitEngine::new().unwrap();
        // (length (append nil '(1 2 3))) => 3
        let result = engine
            .eval(&parse("(length (append nil '(1 2 3)))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eval_append_empty_second() {
        let engine = JitEngine::new().unwrap();
        // (length (append '(1 2) nil)) => 2
        let result = engine
            .eval(&parse("(length (append '(1 2) nil))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(2));
    }

    #[test]
    fn test_eval_reverse() {
        let engine = JitEngine::new().unwrap();
        // (car (reverse '(1 2 3))) => 3
        let result = engine
            .eval(&parse("(car (reverse '(1 2 3)))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eval_reverse_empty() {
        let engine = JitEngine::new().unwrap();
        // (reverse nil) => nil
        let result = engine.eval(&parse("(reverse nil)").unwrap()).unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_eval_nth_first() {
        let engine = JitEngine::new().unwrap();
        // (nth '(10 20 30) 0) => 10
        let result = engine.eval(&parse("(nth '(10 20 30) 0)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(10));
    }

    #[test]
    fn test_eval_nth_middle() {
        let engine = JitEngine::new().unwrap();
        // (nth '(10 20 30) 1) => 20
        let result = engine.eval(&parse("(nth '(10 20 30) 1)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(20));
    }

    #[test]
    fn test_eval_nth_last() {
        let engine = JitEngine::new().unwrap();
        // (nth '(10 20 30) 2) => 30
        let result = engine.eval(&parse("(nth '(10 20 30) 2)").unwrap()).unwrap();
        assert_eq!(result.to_int(), Some(30));
    }

    #[test]
    fn test_eval_nth_out_of_bounds() {
        let engine = JitEngine::new().unwrap();
        // (nth '(10 20 30) 5) => nil
        let result = engine.eval(&parse("(nth '(10 20 30) 5)").unwrap()).unwrap();
        assert!(result.is_nil());
    }

    // ========================================================================
    // Vector Operation Tests
    // ========================================================================

    #[test]
    fn test_eval_vector_empty() {
        let engine = JitEngine::new().unwrap();
        // (vector) => empty vector
        let result = engine.eval(&parse("(vector)").unwrap()).unwrap();
        assert!(result.is_vector());
    }

    #[test]
    fn test_eval_vector_with_elements() {
        let engine = JitEngine::new().unwrap();
        // (vector 1 2 3) => vector with 3 elements
        let result = engine.eval(&parse("(vector 1 2 3)").unwrap()).unwrap();
        assert!(result.is_vector());
    }

    #[test]
    fn test_eval_vector_length_empty() {
        let engine = JitEngine::new().unwrap();
        // (vector-length (vector)) => 0
        let result = engine
            .eval(&parse("(vector-length (vector))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(0));
    }

    #[test]
    fn test_eval_vector_length() {
        let engine = JitEngine::new().unwrap();
        // (vector-length (vector 1 2 3)) => 3
        let result = engine
            .eval(&parse("(vector-length (vector 1 2 3))").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(3));
    }

    #[test]
    fn test_eval_vector_ref_first() {
        let engine = JitEngine::new().unwrap();
        // (vector-ref (vector 10 20 30) 0) => 10
        let result = engine
            .eval(&parse("(vector-ref (vector 10 20 30) 0)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(10));
    }

    #[test]
    fn test_eval_vector_ref_middle() {
        let engine = JitEngine::new().unwrap();
        // (vector-ref (vector 10 20 30) 1) => 20
        let result = engine
            .eval(&parse("(vector-ref (vector 10 20 30) 1)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(20));
    }

    #[test]
    fn test_eval_vector_ref_last() {
        let engine = JitEngine::new().unwrap();
        // (vector-ref (vector 10 20 30) 2) => 30
        let result = engine
            .eval(&parse("(vector-ref (vector 10 20 30) 2)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(30));
    }

    #[test]
    fn test_eval_vector_ref_out_of_bounds() {
        let engine = JitEngine::new().unwrap();
        // (vector-ref (vector 10 20 30) 5) => nil
        let result = engine
            .eval(&parse("(vector-ref (vector 10 20 30) 5)").unwrap())
            .unwrap();
        assert!(result.is_nil());
    }

    #[test]
    fn test_eval_vector_with_arithmetic() {
        let engine = JitEngine::new().unwrap();
        // (vector (+ 1 2) (* 3 4) (- 10 5)) => (3, 12, 5)
        // (vector-ref ... 0) => 3
        let result = engine
            .eval(&parse("(vector-ref (vector (+ 1 2) (* 3 4) (- 10 5)) 0)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(3));

        let result = engine
            .eval(&parse("(vector-ref (vector (+ 1 2) (* 3 4) (- 10 5)) 1)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(12));

        let result = engine
            .eval(&parse("(vector-ref (vector (+ 1 2) (* 3 4) (- 10 5)) 2)").unwrap())
            .unwrap();
        assert_eq!(result.to_int(), Some(5));
    }
}
