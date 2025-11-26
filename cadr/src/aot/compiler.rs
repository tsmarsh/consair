//! AOT (Ahead-of-Time) compiler for Consair.
//!
//! This module provides compilation from Consair source code to LLVM IR,
//! which can then be compiled to native code using standard LLVM tools.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

use inkwell::context::Context;
use inkwell::values::{BasicValue, FunctionValue, StructValue};

use cons::codegen::Codegen;
use cons::jit::JitError;
use cons::jit::analysis::find_free_variables;
use cons::runtime::{TAG_BOOL, TAG_NIL};

use consair::interner::InternedSymbol;
use consair::language::{AtomType, StringType, SymbolType, Value};
use consair::lexer::Lexer;
use consair::numeric::NumericType;
use consair::parser::Parser;

use super::runtime_ir::generate_runtime_ir;

/// Counter for generating unique function names for labeled lambdas.
static EXPR_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Environment mapping bound symbols to their compiled values.
pub(crate) type AotEnv<'ctx> = HashMap<InternedSymbol, StructValue<'ctx>>;

/// Stored lambda definitions for recursive functions.
pub(crate) type LambdaStore = HashMap<InternedSymbol, Value>;

/// Compiled LLVM functions - maps function names to LLVM function values.
pub(crate) type CompiledFns<'ctx> = HashMap<InternedSymbol, FunctionValue<'ctx>>;

/// Error type for AOT compilation.
#[derive(Debug)]
pub enum AotError {
    /// Parse error in source code
    ParseError(String),
    /// Code generation error
    CodegenError(String),
    /// IO error
    IoError(io::Error),
    /// JIT compilation error (for reusing compile_value)
    JitError(JitError),
}

impl std::fmt::Display for AotError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AotError::ParseError(msg) => write!(f, "Parse error: {}", msg),
            AotError::CodegenError(msg) => write!(f, "Codegen error: {}", msg),
            AotError::IoError(err) => write!(f, "IO error: {}", err),
            AotError::JitError(err) => write!(f, "JIT error: {:?}", err),
        }
    }
}

impl std::error::Error for AotError {}

impl From<io::Error> for AotError {
    fn from(err: io::Error) -> Self {
        AotError::IoError(err)
    }
}

impl From<JitError> for AotError {
    fn from(err: JitError) -> Self {
        AotError::JitError(err)
    }
}

/// AOT compiler for Consair.
///
/// Compiles Consair source code to LLVM IR that can be compiled
/// to native code using clang or llc.
pub struct AotCompiler {
    /// Whether to include debug comments in the output
    pub debug: bool,
}

impl Default for AotCompiler {
    fn default() -> Self {
        Self::new()
    }
}

impl AotCompiler {
    /// Create a new AOT compiler.
    pub fn new() -> Self {
        AotCompiler { debug: false }
    }

    /// Compile a Lisp source file to LLVM IR.
    ///
    /// If `output` is None, writes to stdout.
    pub fn compile_file(&self, input: &Path, output: Option<&Path>) -> Result<(), AotError> {
        let source = fs::read_to_string(input)?;
        let ir = self.compile_source(&source)?;

        match output {
            Some(path) => {
                fs::write(path, ir)?;
            }
            None => {
                io::stdout().write_all(ir.as_bytes())?;
            }
        }

        Ok(())
    }

    /// Compile source code to LLVM IR.
    pub fn compile_source(&self, source: &str) -> Result<String, AotError> {
        // Parse all expressions from the source
        let exprs = self.parse_all(source)?;

        // Generate IR for each expression
        let context = Context::create();
        let codegen = Codegen::new(&context, "consair_aot");

        // First pass: collect top-level label definitions and pre-declare functions
        let mut compiled_fns: CompiledFns<'_> = HashMap::new();
        let mut label_lambdas: Vec<(InternedSymbol, Value)> = Vec::new();

        for expr in &exprs {
            if let Some((name, lambda_expr)) = extract_toplevel_label(expr) {
                // Parse the lambda to get parameter count
                let param_count = self.get_lambda_param_count(&lambda_expr)?;

                // Generate unique function name
                let counter = EXPR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                let fn_name = format!("__consair_labeled_{}_{}", name.resolve(), counter);

                // Create the function type based on parameter count
                let param_types: Vec<inkwell::types::BasicMetadataTypeEnum> = (0..param_count)
                    .map(|_| codegen.value_type.into())
                    .collect();
                let fn_type = codegen.value_type.fn_type(&param_types, false);

                // Declare the function
                let function = codegen.module.add_function(&fn_name, fn_type, None);
                compiled_fns.insert(name, function);
                label_lambdas.push((name, lambda_expr));
            }
        }

        // Second pass: compile all labeled lambda bodies
        for (name, lambda_expr) in &label_lambdas {
            self.compile_toplevel_label(&codegen, *name, lambda_expr, &compiled_fns)?;
        }

        // Third pass: compile all expressions with shared compiled_fns
        let mut expr_fns = Vec::new();
        for (i, expr) in exprs.iter().enumerate() {
            let fn_name = format!("__consair_expr_{}", i);
            let func = self.compile_expr_to_function(&codegen, &fn_name, expr, &compiled_fns)?;
            expr_fns.push(func);
        }

        // Generate main function that calls all expressions and prints the last result
        self.generate_main(&codegen, &expr_fns)?;

        // Get the generated IR (without runtime definitions - they're external)
        let user_ir = codegen.emit_ir();

        // Strip module header and duplicate declarations from user IR
        let user_ir_stripped: String = user_ir
            .lines()
            .filter(|line| {
                // Skip header lines and declare statements for runtime functions
                let is_header = line.starts_with("; ModuleID")
                    || line.starts_with("source_filename")
                    || line.starts_with("target datalayout")
                    || line.starts_with("target triple");

                let is_rt_declare = line.starts_with("declare")
                    && (line.contains("@rt_")
                        || line.contains("@print_value")
                        || line.contains("@print_list")
                        || line.contains("@printf")
                        || line.contains("@malloc")
                        || line.contains("@free")
                        || line.contains("@memcpy"));

                !is_header && !is_rt_declare
            })
            .collect::<Vec<_>>()
            .join("\n");

        // Get the runtime IR
        let runtime_ir = generate_runtime_ir();

        // Combine: runtime first, then user code
        let combined_ir = format!(
            "; Consair AOT Compiled Output\n\
             ; Generated by cadr\n\
             \n\
             {}\n\
             ; User code\n\
             {}\n",
            runtime_ir, user_ir_stripped
        );

        Ok(combined_ir)
    }

    /// Compile a single expression to a function.
    fn compile_expr_to_function<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        name: &str,
        expr: &Value,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<FunctionValue<'ctx>, AotError> {
        // Create the function
        let fn_type = codegen.expr_fn_type();
        let function = codegen.add_function(name, fn_type);

        // Create entry block
        let entry = codegen.context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(entry);

        // Initialize empty environments for top-level compilation
        let env: AotEnv<'ctx> = HashMap::new();
        let lambdas: LambdaStore = HashMap::new();

        // Compile the expression (top-level is in tail position)
        let result = self.compile_value(codegen, expr, &env, &lambdas, compiled_fns, true)?;

        // Return the result
        codegen.builder.build_return(Some(&result)).unwrap();

        Ok(function)
    }

    /// Get the parameter count from a lambda expression.
    fn get_lambda_param_count(&self, lambda_expr: &Value) -> Result<usize, AotError> {
        if let Value::Cons(lambda_cell) = lambda_expr
            && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(lambda_sym))) = &lambda_cell.car
            && lambda_sym.resolve() == "lambda"
        {
            let lambda_parts = self.collect_args(&lambda_cell.cdr)?;
            if lambda_parts.is_empty() {
                return Err(AotError::CodegenError(
                    "lambda requires parameters".to_string(),
                ));
            }
            let params = &lambda_parts[0];
            let param_names = self.collect_args(params)?;
            return Ok(param_names.len());
        }
        Err(AotError::CodegenError(
            "Expected lambda expression".to_string(),
        ))
    }

    /// Compile a top-level label definition.
    /// This compiles the body of a pre-declared labeled function.
    fn compile_toplevel_label<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        name: InternedSymbol,
        lambda_expr: &Value,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<(), AotError> {
        // Get the function we declared earlier
        let function = compiled_fns.get(&name).ok_or_else(|| {
            AotError::CodegenError(format!("Function {} not pre-declared", name.resolve()))
        })?;

        // Parse the lambda to get parameters and body
        let (param_symbols, body) = if let Value::Cons(lambda_cell) = lambda_expr {
            if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(lambda_sym))) = &lambda_cell.car
            {
                if lambda_sym.resolve() == "lambda" {
                    let lambda_parts = self.collect_args(&lambda_cell.cdr)?;
                    if lambda_parts.len() < 2 {
                        return Err(AotError::CodegenError(
                            "lambda requires parameters and body".to_string(),
                        ));
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
                                Err(AotError::CodegenError(
                                    "lambda parameters must be symbols".to_string(),
                                ))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (param_symbols, body)
                } else {
                    return Err(AotError::CodegenError(
                        "Expected lambda expression".to_string(),
                    ));
                }
            } else {
                return Err(AotError::CodegenError(
                    "Expected lambda expression".to_string(),
                ));
            }
        } else {
            return Err(AotError::CodegenError(
                "Expected lambda expression".to_string(),
            ));
        };

        // Create entry block for the function
        let entry = codegen.context.append_basic_block(*function, "entry");
        codegen.builder.position_at_end(entry);

        // Create environment with parameters bound to function arguments
        let mut fn_env: AotEnv<'ctx> = HashMap::new();
        for (i, sym) in param_symbols.iter().enumerate() {
            let param = function
                .get_nth_param(i as u32)
                .ok_or_else(|| {
                    AotError::CodegenError("Failed to get function parameter".to_string())
                })?
                .into_struct_value();
            fn_env.insert(*sym, param);
        }

        let lambdas: LambdaStore = HashMap::new();

        // Compile the body with the environment and compiled_fns (body is in tail position)
        let result = self.compile_value(codegen, &body, &fn_env, &lambdas, compiled_fns, true)?;

        // Return the result
        codegen.builder.build_return(Some(&result)).unwrap();

        Ok(())
    }

    /// Compile a Value to LLVM IR.
    ///
    /// This is a simplified version of JitEngine::compile_value that generates
    /// IR for AOT compilation.
    ///
    /// # Parameters
    /// - `codegen`: The code generator context
    /// - `value`: The value to compile
    /// - `env`: Environment mapping bound symbols to their compiled values
    /// - `lambdas`: Stored lambda definitions for recursive functions
    /// - `compiled_fns`: Already-compiled LLVM functions
    /// - `tail_position`: Whether this expression is in tail position (for TCO)
    fn compile_value<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        value: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        match value {
            Value::Nil => Ok(codegen.compile_nil()),

            Value::Atom(AtomType::Bool(b)) => Ok(codegen.compile_bool(*b)),

            Value::Atom(AtomType::Number(NumericType::Int(n))) => Ok(codegen.compile_int(*n)),

            Value::Atom(AtomType::Number(NumericType::Float(f))) => Ok(codegen.compile_float(*f)),

            Value::Atom(AtomType::Number(NumericType::Ratio(num, denom))) => {
                // Convert ratio to float
                let float_val = *num as f64 / *denom as f64;
                Ok(codegen.compile_float(float_val))
            }

            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => {
                // Check environment first (for bound variables)
                if let Some(val) = env.get(sym) {
                    return Ok(*val);
                }

                // Otherwise, convert symbol to key by copying its bytes
                let key = symbol_to_key(sym);
                Ok(codegen.compile_symbol(key))
            }

            Value::Atom(AtomType::String(StringType::Basic(s))) => {
                // Generate a unique ID for this string literal
                let unique_id = EXPR_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                Ok(codegen.compile_string_literal(s, unique_id))
            }

            Value::Cons(cell) => {
                // Handle special forms and function calls
                self.compile_cons(
                    codegen,
                    &cell.car,
                    &cell.cdr,
                    env,
                    lambdas,
                    compiled_fns,
                    tail_position,
                )
            }

            Value::Lambda(_) => Err(AotError::CodegenError(
                "Lambda expressions should be compiled to closures".to_string(),
            )),

            _ => Err(AotError::CodegenError(format!(
                "Unsupported value type for AOT: {:?}",
                value
            ))),
        }
    }

    /// Compile a cons cell (function call or special form).
    #[allow(clippy::too_many_arguments)]
    fn compile_cons<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        car: &Value,
        cdr: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Check for special forms
        if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = car {
            let name = sym.resolve();
            match name.as_str() {
                "quote" => return self.compile_quote(codegen, cdr),
                "label" => {
                    // Check if this is a top-level label that was pre-compiled
                    // If so, the function already exists in compiled_fns; just return nil
                    // cdr is (name (lambda ...))
                    if let Value::Cons(name_cell) = cdr
                        && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(label_name))) =
                            &name_cell.car
                        && compiled_fns.contains_key(label_name)
                    {
                        // Already compiled, return nil
                        return Ok(codegen.compile_nil());
                    }
                    // Otherwise fall through to handle inline label calls
                }
                "if" => {
                    return self.compile_if(
                        codegen,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                        tail_position,
                    );
                }
                "cond" => {
                    return self.compile_cond(
                        codegen,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                        tail_position,
                    );
                }
                "+" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_add,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "-" => return self.compile_minus(codegen, cdr, env, lambdas, compiled_fns),
                "*" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_mul,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "/" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_div,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "=" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_num_eq,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "<" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_lt,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                ">" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_gt,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "<=" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_lte,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                ">=" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_gte,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "eq" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_eq,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "nil?" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_is_nil,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "atom" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_is_atom,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "cons?" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_is_cons,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "number?" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_is_number,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "not" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_not,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "cons" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_cons,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "car" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_car,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "cdr" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_cdr,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "length" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_length,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "reverse" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_reverse,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "println" => {
                    return self.compile_variadic_print(
                        codegen,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                        true, // add newline
                    );
                }
                "print" => {
                    return self.compile_variadic_print(
                        codegen,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                        false, // no newline
                    );
                }
                "append" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_append,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "nth" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_nth,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "list" => return self.compile_list(codegen, cdr, env, lambdas, compiled_fns),
                "lambda" => {
                    // Lambda not immediately applied - compile to a closure
                    return self.compile_closure(codegen, cdr, env, lambdas, compiled_fns);
                }
                "vector" => return self.compile_vector(codegen, cdr, env, lambdas, compiled_fns),
                "vector-length" => {
                    return self.compile_unary_op(
                        codegen,
                        codegen.rt_vector_length,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                "vector-ref" => {
                    return self.compile_binary_op(
                        codegen,
                        codegen.rt_vector_ref,
                        cdr,
                        env,
                        lambdas,
                        compiled_fns,
                    );
                }
                _ => {}
            }
        }

        // Check if operator is a lambda expression: ((lambda (params) body) args...)
        if let Value::Cons(op_cell) = car {
            if is_lambda(&op_cell.car) {
                return self.compile_lambda_call(
                    codegen,
                    &op_cell.cdr,
                    cdr,
                    env,
                    lambdas,
                    compiled_fns,
                );
            }
            // Check if operator is a label expression: ((label name (lambda ...)) args...)
            if is_label(&op_cell.car) {
                return self.compile_labeled_lambda_call(
                    codegen,
                    &op_cell.cdr,
                    cdr,
                    env,
                    lambdas,
                    compiled_fns,
                    tail_position,
                );
            }
        }

        // Check if operator is a compiled recursive function
        if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) = car {
            if let Some(func) = compiled_fns.get(sym) {
                return self.compile_recursive_call(
                    codegen,
                    *func,
                    cdr,
                    env,
                    lambdas,
                    compiled_fns,
                    tail_position,
                );
            }
            // Check if operator is a bound variable (might be a closure)
            if let Some(val) = env.get(sym) {
                return self.compile_closure_call(codegen, *val, cdr, env, lambdas, compiled_fns);
            }
        }

        // If operator is a complex expression (like ((lambda ...) args) returning a closure),
        // we need to compile the operator and call the result as a closure
        if matches!(car, Value::Cons(_)) {
            // Compile the operator expression (it might return a closure)
            let closure_val =
                self.compile_value(codegen, car, env, lambdas, compiled_fns, false)?;
            return self.compile_closure_call(
                codegen,
                closure_val,
                cdr,
                env,
                lambdas,
                compiled_fns,
            );
        }

        // Unknown form - try to compile as data
        Err(AotError::CodegenError(format!(
            "Unknown form or function call not yet supported: {:?}",
            car
        )))
    }

    /// Compile a quote form.
    fn compile_quote<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Get the quoted value
        let quoted = self.get_first_arg(args)?;
        self.compile_quoted_value(codegen, quoted)
    }

    /// Compile a quoted value (data).
    #[allow(clippy::only_used_in_recursion)]
    fn compile_quoted_value<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        value: &Value,
    ) -> Result<StructValue<'ctx>, AotError> {
        match value {
            Value::Nil => Ok(codegen.compile_nil()),
            Value::Atom(AtomType::Bool(b)) => Ok(codegen.compile_bool(*b)),
            Value::Atom(AtomType::Number(NumericType::Int(n))) => Ok(codegen.compile_int(*n)),
            Value::Atom(AtomType::Number(NumericType::Float(f))) => Ok(codegen.compile_float(*f)),
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => {
                let key = symbol_to_key(sym);
                Ok(codegen.compile_symbol(key))
            }
            Value::Cons(cell) => {
                // Build cons cell at runtime
                let car = self.compile_quoted_value(codegen, &cell.car)?;
                let cdr = self.compile_quoted_value(codegen, &cell.cdr)?;

                let result = codegen
                    .builder
                    .build_call(codegen.rt_cons, &[car.into(), cdr.into()], "cons")
                    .unwrap()
                    .try_as_basic_value()
                    .left()
                    .ok_or_else(|| AotError::CodegenError("rt_cons didn't return value".into()))?;

                Ok(result.into_struct_value())
            }
            _ => Err(AotError::CodegenError(format!(
                "Cannot quote value: {:?}",
                value
            ))),
        }
    }

    /// Compile an if form.
    #[allow(clippy::too_many_arguments)]
    fn compile_if<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        let (cond_expr, rest) = self.get_car_cdr(args)?;
        let (then_expr, rest2) = self.get_car_cdr(rest)?;
        let else_expr = self.get_first_arg(rest2).unwrap_or(&Value::Nil);

        // Compile condition (not in tail position)
        let cond_val = self.compile_value(codegen, cond_expr, env, lambdas, compiled_fns, false)?;

        // Extract tag and data to check truthiness
        let tag = codegen
            .builder
            .build_extract_value(cond_val, 0, "tag")
            .unwrap()
            .into_int_value();
        let data = codegen
            .builder
            .build_extract_value(cond_val, 1, "data")
            .unwrap()
            .into_int_value();

        // Check if nil (tag == 0)
        let is_nil = codegen
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                codegen.i8_type().const_int(TAG_NIL as u64, false),
                "is_nil",
            )
            .unwrap();

        // Check if false (tag == 1 && data == 0)
        let is_bool = codegen
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                tag,
                codegen.i8_type().const_int(TAG_BOOL as u64, false),
                "is_bool",
            )
            .unwrap();
        let is_false_data = codegen
            .builder
            .build_int_compare(
                inkwell::IntPredicate::EQ,
                data,
                codegen.i64_type().const_int(0, false),
                "is_false_data",
            )
            .unwrap();
        let is_false = codegen
            .builder
            .build_and(is_bool, is_false_data, "is_false")
            .unwrap();

        // Falsy = nil or false
        let is_falsy = codegen
            .builder
            .build_or(is_nil, is_false, "is_falsy")
            .unwrap();

        // Get current function
        let func = codegen
            .builder
            .get_insert_block()
            .unwrap()
            .get_parent()
            .unwrap();

        // Create blocks
        let then_bb = codegen.context.append_basic_block(func, "then");
        let else_bb = codegen.context.append_basic_block(func, "else");
        let merge_bb = codegen.context.append_basic_block(func, "merge");

        // Branch based on condition
        codegen
            .builder
            .build_conditional_branch(is_falsy, else_bb, then_bb)
            .unwrap();

        // Then block (inherits tail_position)
        codegen.builder.position_at_end(then_bb);
        let then_val = self.compile_value(
            codegen,
            then_expr,
            env,
            lambdas,
            compiled_fns,
            tail_position,
        )?;
        codegen
            .builder
            .build_unconditional_branch(merge_bb)
            .unwrap();
        let then_bb = codegen.builder.get_insert_block().unwrap();

        // Else block (inherits tail_position)
        codegen.builder.position_at_end(else_bb);
        let else_val = self.compile_value(
            codegen,
            else_expr,
            env,
            lambdas,
            compiled_fns,
            tail_position,
        )?;
        codegen
            .builder
            .build_unconditional_branch(merge_bb)
            .unwrap();
        let else_bb = codegen.builder.get_insert_block().unwrap();

        // Merge block with phi
        codegen.builder.position_at_end(merge_bb);
        let phi = codegen
            .builder
            .build_phi(codegen.value_type, "if_result")
            .unwrap();
        phi.add_incoming(&[
            (&then_val.as_basic_value_enum(), then_bb),
            (&else_val.as_basic_value_enum(), else_bb),
        ]);

        Ok(phi.as_basic_value().into_struct_value())
    }

    /// Compile a cond expression (multi-branch conditional).
    #[allow(clippy::too_many_arguments)]
    fn compile_cond<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        let clauses = self.collect_args(args)?;

        if clauses.is_empty() {
            // Empty cond returns nil
            return Ok(codegen.compile_nil());
        }

        // Get the current function
        let current_block = codegen
            .builder
            .get_insert_block()
            .ok_or_else(|| AotError::CodegenError("No current block".to_string()))?;
        let function = current_block
            .get_parent()
            .ok_or_else(|| AotError::CodegenError("Block has no parent function".to_string()))?;

        // Create a merge block where all branches will converge
        let merge_block = codegen.context.append_basic_block(function, "cond_merge");

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
                return Err(AotError::CodegenError(
                    "cond clause must have at least 2 elements".to_string(),
                ));
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
                    .ok_or_else(|| AotError::CodegenError("No current block".to_string()))?;
                phi_incoming.push((result_val.into(), current));
                codegen.builder.build_unconditional_branch(merge_block).ok();
                break;
            }

            // Compile the test expression (test is NOT in tail position)
            let test_val =
                self.compile_value(codegen, test_expr, env, lambdas, compiled_fns, false)?;

            // Check if test is truthy (not nil and not false)
            let tag = codegen
                .builder
                .build_extract_value(test_val, 0, "tag")
                .unwrap()
                .into_int_value();

            let data = codegen
                .builder
                .build_extract_value(test_val, 1, "data")
                .unwrap()
                .into_int_value();

            // Check if tag == TAG_NIL (0)
            let is_nil = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    codegen.i8_type().const_int(TAG_NIL as u64, false),
                    "is_nil",
                )
                .unwrap();

            // Check if tag == TAG_BOOL (1) and data == 0 (false)
            let is_bool = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    tag,
                    codegen.i8_type().const_int(TAG_BOOL as u64, false),
                    "is_bool",
                )
                .unwrap();

            let is_false_data = codegen
                .builder
                .build_int_compare(
                    inkwell::IntPredicate::EQ,
                    data,
                    codegen.i64_type().const_int(0, false),
                    "is_false_data",
                )
                .unwrap();

            let is_false = codegen
                .builder
                .build_and(is_bool, is_false_data, "is_false")
                .unwrap();

            // Falsy if nil OR (bool AND data==0)
            let is_falsy = codegen
                .builder
                .build_or(is_nil, is_false, "is_falsy")
                .unwrap();

            // Create blocks for then and else
            let then_block = codegen
                .context
                .append_basic_block(function, &format!("cond_then_{}", i));
            let else_block = codegen
                .context
                .append_basic_block(function, &format!("cond_else_{}", i));

            // Branch based on truthiness (if falsy, go to else; if truthy, go to then)
            codegen
                .builder
                .build_conditional_branch(is_falsy, else_block, then_block)
                .ok();

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
                .ok_or_else(|| AotError::CodegenError("No current block".to_string()))?;
            phi_incoming.push((result_val.into(), then_end));
            codegen.builder.build_unconditional_branch(merge_block).ok();

            // Continue from the else block for the next clause
            codegen.builder.position_at_end(else_block);
        }

        // If we didn't hit a final 't' clause, we need to handle the fallthrough case
        // (return nil if no clause matched)
        let current = codegen
            .builder
            .get_insert_block()
            .ok_or_else(|| AotError::CodegenError("No current block".to_string()))?;
        if current != merge_block && current.get_terminator().is_none() {
            let nil_val = codegen.compile_nil();
            phi_incoming.push((nil_val.into(), current));
            codegen.builder.build_unconditional_branch(merge_block).ok();
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
            .unwrap();

        for (val, block) in &phi_incoming {
            phi.add_incoming(&[(val, *block)]);
        }

        Ok(phi.as_basic_value().into_struct_value())
    }

    /// Compile an immediately-applied lambda: ((lambda (params) body) args...)
    ///
    /// This handles cases where a lambda is directly called with arguments.
    /// It binds the arguments to the parameters and compiles the body.
    #[allow(clippy::too_many_arguments)]
    fn compile_lambda_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        lambda_parts: &Value,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        // lambda_parts should be ((params) body)
        let parts = self.collect_args(lambda_parts)?;
        if parts.len() < 2 {
            return Err(AotError::CodegenError(
                "lambda requires parameters and body".to_string(),
            ));
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
                    Err(AotError::CodegenError(
                        "lambda parameters must be symbols".to_string(),
                    ))
                }
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Compile arguments
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != param_symbols.len() {
            return Err(AotError::CodegenError(format!(
                "lambda expects {} arguments, got {}",
                param_symbols.len(),
                arg_values.len()
            )));
        }

        // Compile each argument (arguments are NOT in tail position)
        let compiled_args: Vec<StructValue<'ctx>> = arg_values
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

    /// Compile a labeled lambda call: ((label name (lambda (params) body)) args...)
    ///
    /// This creates a named recursive function and immediately calls it.
    #[allow(clippy::too_many_arguments)]
    fn compile_labeled_lambda_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        label_parts: &Value,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        // label_parts should be (name (lambda ...))
        let parts = self.collect_args(label_parts)?;
        if parts.len() != 2 {
            return Err(AotError::CodegenError(
                "label requires name and lambda".to_string(),
            ));
        }

        // Get the name
        let name = match &parts[0] {
            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => *sym,
            _ => {
                return Err(AotError::CodegenError(
                    "label name must be a symbol".to_string(),
                ));
            }
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
                        return Err(AotError::CodegenError(
                            "lambda requires parameters and body".to_string(),
                        ));
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
                                Err(AotError::CodegenError(
                                    "lambda parameters must be symbols".to_string(),
                                ))
                            }
                        })
                        .collect::<Result<Vec<_>, _>>()?;
                    (param_symbols, body)
                } else {
                    return Err(AotError::CodegenError(
                        "label second argument must be a lambda".to_string(),
                    ));
                }
            } else {
                return Err(AotError::CodegenError(
                    "label second argument must be a lambda".to_string(),
                ));
            }
        } else {
            return Err(AotError::CodegenError(
                "label second argument must be a lambda".to_string(),
            ));
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
        let entry = codegen.context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(entry);

        // Create new environment with parameters bound to function arguments
        let mut fn_env = env.clone();
        for (i, sym) in param_symbols.iter().enumerate() {
            let param = function
                .get_nth_param(i as u32)
                .ok_or_else(|| {
                    AotError::CodegenError("Failed to get function parameter".to_string())
                })?
                .into_struct_value();
            fn_env.insert(*sym, param);
        }

        // Compile the body with the new environment and compiled_fns (body is in tail position)
        let result =
            self.compile_value(codegen, &body, &fn_env, lambdas, &new_compiled_fns, true)?;

        // Return the result
        codegen.builder.build_return(Some(&result)).unwrap();

        // Restore the saved insertion point
        if let Some(block) = saved_block {
            codegen.builder.position_at_end(block);
        }

        // Now compile the initial call to the function with the provided arguments
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != param_symbols.len() {
            return Err(AotError::CodegenError(format!(
                "label lambda expects {} arguments, got {}",
                param_symbols.len(),
                arg_values.len()
            )));
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
        let call_site = codegen
            .builder
            .build_call(function, &compiled_args, "label_call")
            .unwrap();

        // Mark as tail call if in tail position
        if tail_position {
            call_site.set_tail_call(true);
        }

        let call_result = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| AotError::CodegenError("Label call did not return a value".to_string()))?
            .into_struct_value();

        Ok(call_result)
    }

    /// Compile a call to a recursively-defined function.
    #[allow(clippy::too_many_arguments)]
    fn compile_recursive_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        function: FunctionValue<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        tail_position: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Get expected parameter count
        let expected_params = function.count_params() as usize;

        // Compile arguments
        let arg_values = self.collect_args(args)?;
        if arg_values.len() != expected_params {
            return Err(AotError::CodegenError(format!(
                "recursive function expects {} arguments, got {}",
                expected_params,
                arg_values.len()
            )));
        }

        // Compile each argument (arguments are NOT in tail position)
        let compiled_args: Vec<inkwell::values::BasicMetadataValueEnum> = arg_values
            .iter()
            .map(|arg| {
                self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)
                    .map(|v| v.into())
            })
            .collect::<Result<Vec<_>, _>>()?;

        // Generate the call
        let call_site = codegen
            .builder
            .build_call(function, &compiled_args, "recursive_call")
            .unwrap();

        // Mark as tail call if in tail position
        if tail_position {
            call_site.set_tail_call(true);
        }

        let call_result = call_site
            .try_as_basic_value()
            .left()
            .ok_or_else(|| {
                AotError::CodegenError("Recursive call did not return a value".to_string())
            })?
            .into_struct_value();

        Ok(call_result)
    }

    /// Compile a lambda into a closure value.
    ///
    /// This creates a closure that captures free variables from the environment.
    fn compile_closure<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        lambda_parts: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Parse lambda parts: ((params) body)
        let parts = self.collect_args(lambda_parts)?;
        if parts.len() < 2 {
            return Err(AotError::CodegenError(
                "lambda requires parameters and body".to_string(),
            ));
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
                    Err(AotError::CodegenError(
                        "lambda parameters must be symbols".to_string(),
                    ))
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
        let entry = codegen.context.append_basic_block(closure_fn, "entry");
        codegen.builder.position_at_end(entry);

        // Get parameters: env_ptr, args_ptr, num_args
        let env_ptr = closure_fn
            .get_nth_param(0)
            .ok_or_else(|| AotError::CodegenError("Failed to get env_ptr parameter".to_string()))?
            .into_pointer_value();
        let args_ptr = closure_fn
            .get_nth_param(1)
            .ok_or_else(|| AotError::CodegenError("Failed to get args_ptr parameter".to_string()))?
            .into_pointer_value();
        let _num_args = closure_fn
            .get_nth_param(2)
            .ok_or_else(|| AotError::CodegenError("Failed to get num_args parameter".to_string()))?
            .into_int_value();

        // Create new environment with captured values and parameters bound
        let mut closure_env = AotEnv::new();

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
            .unwrap();

            let val = codegen
                .builder
                .build_load(
                    codegen.value_type,
                    elem_ptr,
                    &format!("cap_{}", sym.resolve()),
                )
                .unwrap()
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
            .unwrap();

            let val = codegen
                .builder
                .build_load(
                    codegen.value_type,
                    elem_ptr,
                    &format!("param_{}", sym.resolve()),
                )
                .unwrap()
                .into_struct_value();
            closure_env.insert(*sym, val);
        }

        // Compile the body with the closure environment (body IS in tail position)
        let result =
            self.compile_value(codegen, body, &closure_env, lambdas, compiled_fns, true)?;

        // Return the result
        codegen.builder.build_return(Some(&result)).unwrap();

        // Restore the saved insertion point
        if let Some(block) = saved_block {
            codegen.builder.position_at_end(block);
        }

        // Now generate code to create the closure at runtime:
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
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| {
                    AotError::CodegenError("rt_make_closure did not return a value".to_string())
                })?
                .into_struct_value();

            Ok(closure_val)
        } else {
            // Allocate space for captured values on the stack
            let array_type = codegen.value_type.array_type(free_var_list.len() as u32);
            let env_array = codegen
                .builder
                .build_alloca(array_type, "captured_env")
                .unwrap();

            // Store each captured value
            for (i, sym) in free_var_list.iter().enumerate() {
                let val = env.get(sym).ok_or_else(|| {
                    AotError::CodegenError(format!(
                        "Undefined variable in closure: {}",
                        sym.resolve()
                    ))
                })?;

                let idx = codegen.i32_type().const_int(i as u64, false);
                let ptr = unsafe {
                    codegen.builder.build_gep(
                        array_type,
                        env_array,
                        &[codegen.i32_type().const_int(0, false), idx],
                        "env_ptr",
                    )
                }
                .unwrap();

                codegen.builder.build_store(ptr, *val).unwrap();
            }

            // Cast the array pointer to a generic pointer
            let env_ptr = codegen
                .builder
                .build_pointer_cast(env_array, codegen.ptr_type(), "env_cast")
                .unwrap();

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
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| {
                    AotError::CodegenError("rt_make_closure did not return a value".to_string())
                })?
                .into_struct_value();

            Ok(closure_val)
        }
    }

    /// Compile a call to a closure value.
    fn compile_closure_call<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        closure_val: StructValue<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Compile arguments (arguments are NOT in tail position)
        let arg_values = self.collect_args(args)?;
        let compiled_args: Vec<StructValue<'ctx>> = arg_values
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
                .unwrap();

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
                .unwrap();

                codegen.builder.build_store(ptr, *arg_val).unwrap();
            }

            codegen
                .builder
                .build_pointer_cast(args_arr, codegen.ptr_type(), "args_cast")
                .unwrap()
        } else {
            codegen.ptr_type().const_null()
        };

        // Get the function pointer from the closure
        let fn_ptr = codegen
            .builder
            .build_call(codegen.rt_closure_fn_ptr, &[closure_val.into()], "fn_ptr")
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| {
                AotError::CodegenError("rt_closure_fn_ptr did not return a value".to_string())
            })?
            .into_pointer_value();

        // Get the env size
        let env_size = codegen
            .builder
            .build_call(
                codegen.rt_closure_env_size,
                &[closure_val.into()],
                "env_size",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| {
                AotError::CodegenError("rt_closure_env_size did not return a value".to_string())
            })?
            .into_int_value();

        // Allocate space for captured values and fill from rt_closure_env_get
        let max_env_size = 16u32; // Support up to 16 captures
        let env_array_type = codegen.value_type.array_type(max_env_size);
        let env_array = codegen
            .builder
            .build_alloca(env_array_type, "closure_env")
            .unwrap();

        // Get the current function for creating basic blocks
        let current_block = codegen
            .builder
            .get_insert_block()
            .ok_or_else(|| AotError::CodegenError("No current block".to_string()))?;
        let function = current_block
            .get_parent()
            .ok_or_else(|| AotError::CodegenError("Block has no parent function".to_string()))?;

        // Create blocks for the env loading loop
        let loop_header = codegen
            .context
            .append_basic_block(function, "env_loop_header");
        let loop_body = codegen
            .context
            .append_basic_block(function, "env_loop_body");
        let loop_end = codegen.context.append_basic_block(function, "env_loop_end");

        // Initialize loop counter
        let counter_ptr = codegen
            .builder
            .build_alloca(codegen.i32_type(), "env_counter")
            .unwrap();
        codegen
            .builder
            .build_store(counter_ptr, codegen.i32_type().const_int(0, false))
            .unwrap();

        codegen
            .builder
            .build_unconditional_branch(loop_header)
            .unwrap();

        // Loop header: check if counter < env_size
        codegen.builder.position_at_end(loop_header);
        let counter = codegen
            .builder
            .build_load(codegen.i32_type(), counter_ptr, "counter")
            .unwrap()
            .into_int_value();
        let cond = codegen
            .builder
            .build_int_compare(inkwell::IntPredicate::ULT, counter, env_size, "cmp")
            .unwrap();
        codegen
            .builder
            .build_conditional_branch(cond, loop_body, loop_end)
            .unwrap();

        // Loop body: load env value and store in array
        codegen.builder.position_at_end(loop_body);
        let counter_val = codegen
            .builder
            .build_load(codegen.i32_type(), counter_ptr, "counter_val")
            .unwrap()
            .into_int_value();

        let env_val = codegen
            .builder
            .build_call(
                codegen.rt_closure_env_get,
                &[closure_val.into(), counter_val.into()],
                "env_val",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| {
                AotError::CodegenError("rt_closure_env_get did not return a value".to_string())
            })?
            .into_struct_value();

        let env_elem_ptr = unsafe {
            codegen.builder.build_gep(
                env_array_type,
                env_array,
                &[codegen.i32_type().const_int(0, false), counter_val],
                "env_elem_ptr",
            )
        }
        .unwrap();

        codegen.builder.build_store(env_elem_ptr, env_val).unwrap();

        // Increment counter
        let next_counter = codegen
            .builder
            .build_int_add(counter_val, codegen.i32_type().const_int(1, false), "next")
            .unwrap();
        codegen
            .builder
            .build_store(counter_ptr, next_counter)
            .unwrap();

        codegen
            .builder
            .build_unconditional_branch(loop_header)
            .unwrap();

        // After loop: call the closure function
        codegen.builder.position_at_end(loop_end);

        let env_ptr = codegen
            .builder
            .build_pointer_cast(env_array, codegen.ptr_type(), "env_ptr_cast")
            .unwrap();

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
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| {
                AotError::CodegenError("Closure call did not return a value".to_string())
            })?
            .into_struct_value();

        Ok(result)
    }

    /// Compile a binary operation.
    #[allow(clippy::too_many_arguments)]
    fn compile_binary_op<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        func: FunctionValue<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        let (arg1, rest) = self.get_car_cdr(args)?;
        let arg2 = self.get_first_arg(rest)?;

        // Arguments are never in tail position
        let val1 = self.compile_value(codegen, arg1, env, lambdas, compiled_fns, false)?;
        let val2 = self.compile_value(codegen, arg2, env, lambdas, compiled_fns, false)?;

        let result = codegen
            .builder
            .build_call(func, &[val1.into(), val2.into()], "binop")
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| AotError::CodegenError("Binary op didn't return value".into()))?;

        Ok(result.into_struct_value())
    }

    /// Compile a unary operation.
    #[allow(clippy::too_many_arguments)]
    fn compile_unary_op<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        func: FunctionValue<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        let arg = self.get_first_arg(args)?;
        // Arguments are never in tail position
        let val = self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)?;

        let result = codegen
            .builder
            .build_call(func, &[val.into()], "unop")
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| AotError::CodegenError("Unary op didn't return value".into()))?;

        Ok(result.into_struct_value())
    }

    /// Compile variadic print/println.
    ///
    /// Prints all arguments with spaces between them.
    /// If `newline` is true, prints a newline at the end.
    #[allow(clippy::too_many_arguments)]
    fn compile_variadic_print<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
        newline: bool,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Collect all arguments
        let arg_values = self.collect_args(args)?;

        // Print each argument, with spaces between them
        for (i, arg) in arg_values.iter().enumerate() {
            // Compile and print the argument
            let val = self.compile_value(codegen, arg, env, lambdas, compiled_fns, false)?;
            codegen
                .builder
                .build_call(codegen.rt_print, &[val.into()], "print_arg")
                .unwrap();

            // Print space between arguments (but not after the last one)
            if i < arg_values.len() - 1 {
                codegen
                    .builder
                    .build_call(codegen.rt_print_space, &[], "print_space")
                    .unwrap();
            }
        }

        // Print newline if requested
        if newline {
            codegen
                .builder
                .build_call(codegen.rt_print_newline, &[], "print_newline")
                .unwrap();
        }

        // Return nil
        Ok(codegen.compile_nil())
    }

    /// Compile minus (handles both negation and subtraction).
    fn compile_minus<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        let (first, rest) = self.get_car_cdr(args)?;

        if matches!(rest, Value::Nil) {
            // Single argument - negation
            let val = self.compile_value(codegen, first, env, lambdas, compiled_fns, false)?;
            let result = codegen
                .builder
                .build_call(codegen.rt_neg, &[val.into()], "neg")
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| AotError::CodegenError("rt_neg didn't return value".into()))?;
            Ok(result.into_struct_value())
        } else {
            // Two arguments - subtraction
            let second = self.get_first_arg(rest)?;
            let val1 = self.compile_value(codegen, first, env, lambdas, compiled_fns, false)?;
            let val2 = self.compile_value(codegen, second, env, lambdas, compiled_fns, false)?;
            let result = codegen
                .builder
                .build_call(codegen.rt_sub, &[val1.into(), val2.into()], "sub")
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| AotError::CodegenError("rt_sub didn't return value".into()))?;
            Ok(result.into_struct_value())
        }
    }

    /// Compile a vector form.
    fn compile_vector<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Compile all elements
        let elements = self.collect_args(args)?;

        if elements.is_empty() {
            // Empty vector
            let null_ptr = codegen.ptr_type().const_null();
            let zero = codegen.i32_type().const_int(0, false);

            let result = codegen
                .builder
                .build_call(
                    codegen.rt_make_vector,
                    &[null_ptr.into(), zero.into()],
                    "empty_vector",
                )
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| {
                    AotError::CodegenError("rt_make_vector didn't return value".into())
                })?;

            return Ok(result.into_struct_value());
        }

        // Compile each element (not in tail position)
        let compiled_elements: Vec<StructValue<'ctx>> = elements
            .iter()
            .map(|elem| self.compile_value(codegen, elem, env, lambdas, compiled_fns, false))
            .collect::<Result<Vec<_>, _>>()?;

        // Allocate stack space for elements
        let array_type = codegen
            .value_type
            .array_type(compiled_elements.len() as u32);
        let elements_array = codegen
            .builder
            .build_alloca(array_type, "vector_elements")
            .unwrap();

        // Store each element
        for (i, elem) in compiled_elements.iter().enumerate() {
            let idx = codegen.i32_type().const_int(i as u64, false);
            let ptr = unsafe {
                codegen.builder.build_gep(
                    array_type,
                    elements_array,
                    &[codegen.i32_type().const_int(0, false), idx],
                    &format!("elem_ptr_{}", i),
                )
            }
            .unwrap();

            codegen.builder.build_store(ptr, *elem).unwrap();
        }

        // Cast to generic pointer
        let elements_ptr = codegen
            .builder
            .build_pointer_cast(elements_array, codegen.ptr_type(), "elements_cast")
            .unwrap();

        let len = codegen
            .i32_type()
            .const_int(compiled_elements.len() as u64, false);

        let result = codegen
            .builder
            .build_call(
                codegen.rt_make_vector,
                &[elements_ptr.into(), len.into()],
                "vector",
            )
            .unwrap()
            .try_as_basic_value()
            .left()
            .ok_or_else(|| AotError::CodegenError("rt_make_vector didn't return value".into()))?;

        Ok(result.into_struct_value())
    }

    /// Compile a list form.
    fn compile_list<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        args: &Value,
        env: &AotEnv<'ctx>,
        lambdas: &LambdaStore,
        compiled_fns: &CompiledFns<'ctx>,
    ) -> Result<StructValue<'ctx>, AotError> {
        // Build list from right to left
        let mut result: StructValue<'ctx> = codegen.compile_nil();

        // Collect args into vec first
        let mut arg_values = Vec::new();
        let mut current = args;
        while let Value::Cons(cell) = current {
            // Arguments are never in tail position
            let val = self.compile_value(codegen, &cell.car, env, lambdas, compiled_fns, false)?;
            arg_values.push(val);
            current = &cell.cdr;
        }

        // Build list from right to left
        for val in arg_values.into_iter().rev() {
            result = codegen
                .builder
                .build_call(codegen.rt_cons, &[val.into(), result.into()], "list_cons")
                .unwrap()
                .try_as_basic_value()
                .left()
                .ok_or_else(|| AotError::CodegenError("rt_cons didn't return value".into()))?
                .into_struct_value();
        }

        Ok(result)
    }

    /// Generate the main function.
    fn generate_main<'ctx>(
        &self,
        codegen: &Codegen<'ctx>,
        expr_fns: &[FunctionValue<'ctx>],
    ) -> Result<(), AotError> {
        // Create main: () -> i32
        let i32_type = codegen.i32_type();
        let main_type = i32_type.fn_type(&[], false);
        let main_fn = codegen.add_function("main", main_type);

        let entry = codegen.context.append_basic_block(main_fn, "entry");
        codegen.builder.position_at_end(entry);

        // Call each expression function, keeping the last result
        let mut last_result = None;
        for func in expr_fns {
            let result = codegen
                .builder
                .build_call(*func, &[], "expr_result")
                .unwrap()
                .try_as_basic_value()
                .left();
            last_result = result;
        }

        // Print the last result if we have one
        if let Some(result) = last_result {
            // Call print_value
            let print_value = codegen
                .module
                .get_function("print_value")
                .unwrap_or_else(|| {
                    // Declare print_value if not already declared
                    let void_type = codegen.context.void_type();
                    let print_type = void_type.fn_type(&[codegen.value_type.into()], false);
                    codegen.module.add_function(
                        "print_value",
                        print_type,
                        Some(inkwell::module::Linkage::External),
                    )
                });

            codegen
                .builder
                .build_call(print_value, &[result.into()], "")
                .unwrap();

            // Print newline
            let printf = codegen.module.get_function("printf").unwrap_or_else(|| {
                let i32_type = codegen.i32_type();
                let ptr_type = codegen.ptr_type();
                let printf_type = i32_type.fn_type(&[ptr_type.into()], true);
                codegen.module.add_function(
                    "printf",
                    printf_type,
                    Some(inkwell::module::Linkage::External),
                )
            });

            // Create newline string
            let newline = codegen
                .builder
                .build_global_string_ptr("\n", "newline")
                .unwrap();
            codegen
                .builder
                .build_call(printf, &[newline.as_pointer_value().into()], "")
                .unwrap();
        }

        // Return 0
        codegen
            .builder
            .build_return(Some(&i32_type.const_int(0, false)))
            .unwrap();

        Ok(())
    }

    // Helper functions

    fn get_first_arg<'a>(&self, args: &'a Value) -> Result<&'a Value, AotError> {
        match args {
            Value::Cons(cell) => Ok(&cell.car),
            Value::Nil => Err(AotError::CodegenError("Expected argument".into())),
            _ => Err(AotError::CodegenError("Invalid argument list".into())),
        }
    }

    fn get_car_cdr<'a>(&self, args: &'a Value) -> Result<(&'a Value, &'a Value), AotError> {
        match args {
            Value::Cons(cell) => Ok((&cell.car, &cell.cdr)),
            Value::Nil => Err(AotError::CodegenError("Expected argument".into())),
            _ => Err(AotError::CodegenError("Invalid argument list".into())),
        }
    }

    /// Collect all arguments from a list into a Vec.
    fn collect_args(&self, args: &Value) -> Result<Vec<Value>, AotError> {
        let mut result = Vec::new();
        let mut current = args.clone();

        loop {
            match current {
                Value::Nil => break,
                Value::Cons(cell) => {
                    result.push(cell.car.clone());
                    current = cell.cdr.clone();
                }
                _ => {
                    return Err(AotError::CodegenError(
                        "Malformed argument list".to_string(),
                    ));
                }
            }
        }

        Ok(result)
    }

    /// Parse all expressions from source code.
    fn parse_all(&self, source: &str) -> Result<Vec<Value>, AotError> {
        let mut lexer = Lexer::new(source);
        let mut parser = Parser::new(&mut lexer);
        let mut exprs = Vec::new();

        loop {
            // Try to parse an expression
            match parser.parse_expression() {
                Ok(expr) => exprs.push(expr),
                Err(e) => {
                    // Check if we're at end of input
                    if e.contains("Unexpected end of input") || e.contains("end of input") {
                        break;
                    }
                    return Err(AotError::ParseError(e));
                }
            }
        }

        if exprs.is_empty() {
            return Err(AotError::ParseError("No expressions to compile".into()));
        }

        Ok(exprs)
    }
}

/// Check if a value is the 'lambda' symbol.
fn is_lambda(value: &Value) -> bool {
    matches!(
        value,
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym)))
            if sym.resolve() == "lambda"
    )
}

/// Check if a value is the 'label' symbol.
fn is_label(value: &Value) -> bool {
    matches!(
        value,
        Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym)))
            if sym.resolve() == "label"
    )
}

/// Check if an expression is a top-level label definition: (label name (lambda ...))
/// Returns Some((name, lambda_expr)) if it is, None otherwise.
fn extract_toplevel_label(expr: &Value) -> Option<(InternedSymbol, Value)> {
    if let Value::Cons(cell) = expr
        && is_label(&cell.car)
    {
        // Get (name (lambda ...))
        if let Value::Cons(name_cell) = &cell.cdr
            && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(name))) = &name_cell.car
        {
            // Get ((lambda ...))
            if let Value::Cons(lambda_cell) = &name_cell.cdr
                && let Value::Cons(lambda_inner) = &lambda_cell.car
                && let Value::Atom(AtomType::Symbol(SymbolType::Symbol(lambda_kw))) =
                    &lambda_inner.car
                && lambda_kw.resolve() == "lambda"
            {
                return Some((*name, lambda_cell.car.clone()));
            }
        }
    }
    None
}

/// Convert an InternedSymbol to a u64 key by copying its bytes.
fn symbol_to_key(sym: &InternedSymbol) -> u64 {
    let mut key: u64 = 0;
    let sym_bytes = unsafe {
        std::slice::from_raw_parts(sym as *const _ as *const u8, std::mem::size_of_val(sym))
    };
    for (i, &byte) in sym_bytes.iter().enumerate() {
        key |= (byte as u64) << (i * 8);
    }
    key
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compiler_new() {
        let compiler = AotCompiler::new();
        assert!(!compiler.debug);
    }

    #[test]
    fn test_compile_simple_int() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("42").unwrap();

        // Should contain the integer constant
        assert!(ir.contains("i8 2")); // TAG_INT
        assert!(ir.contains("i64 42"));
        assert!(ir.contains("define i32 @main"));
    }

    #[test]
    fn test_compile_simple_float() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("3.14").unwrap();

        assert!(ir.contains("define i32 @main"));
    }

    #[test]
    fn test_compile_nil() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("nil").unwrap();

        assert!(ir.contains("i8 0")); // TAG_NIL
        assert!(ir.contains("i64 0"));
    }

    #[test]
    fn test_compile_bool_true() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("true").unwrap();

        assert!(ir.contains("i8 1")); // TAG_BOOL
        assert!(ir.contains("i64 1"));
    }

    #[test]
    fn test_compile_bool_false() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("false").unwrap();

        assert!(ir.contains("i8 1")); // TAG_BOOL
    }

    #[test]
    fn test_compile_addition() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(+ 1 2)").unwrap();

        assert!(ir.contains("@rt_add"));
    }

    #[test]
    fn test_compile_subtraction() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(- 5 3)").unwrap();

        assert!(ir.contains("@rt_sub"));
    }

    #[test]
    fn test_compile_negation() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(- 5)").unwrap();

        assert!(ir.contains("@rt_neg"));
    }

    #[test]
    fn test_compile_multiplication() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(* 3 4)").unwrap();

        assert!(ir.contains("@rt_mul"));
    }

    #[test]
    fn test_compile_division() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(/ 10 2)").unwrap();

        assert!(ir.contains("@rt_div"));
    }

    #[test]
    fn test_compile_comparison() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(< 1 2)").unwrap();

        assert!(ir.contains("@rt_lt"));
    }

    #[test]
    fn test_compile_if() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(if true 1 2)").unwrap();

        assert!(ir.contains("then:"));
        assert!(ir.contains("else:"));
        assert!(ir.contains("merge:"));
    }

    #[test]
    fn test_compile_cond() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source("(cond ((= 1 1) 42) (t 0))")
            .unwrap();

        assert!(ir.contains("cond_then_"));
        assert!(ir.contains("cond_merge"));
    }

    #[test]
    fn test_compile_quote() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(quote 42)").unwrap();

        assert!(ir.contains("i64 42"));
    }

    #[test]
    fn test_compile_quoted_list() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(quote (1 2 3))").unwrap();

        assert!(ir.contains("@rt_cons"));
    }

    #[test]
    fn test_compile_cons() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(cons 1 2)").unwrap();

        assert!(ir.contains("@rt_cons"));
    }

    #[test]
    fn test_compile_car() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(car (quote (1 2)))").unwrap();

        assert!(ir.contains("@rt_car"));
    }

    #[test]
    fn test_compile_cdr() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(cdr (quote (1 2)))").unwrap();

        assert!(ir.contains("@rt_cdr"));
    }

    #[test]
    fn test_compile_list() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(list 1 2 3)").unwrap();

        // Should build list with multiple cons calls
        assert!(ir.contains("@rt_cons"));
    }

    #[test]
    fn test_compile_type_predicates() {
        let compiler = AotCompiler::new();

        let ir = compiler.compile_source("(nil? nil)").unwrap();
        assert!(ir.contains("@rt_is_nil"));

        // Note: atom is the McCarthy primitive name (not atom?)
        let ir = compiler.compile_source("(atom 42)").unwrap();
        assert!(ir.contains("@rt_is_atom"));

        let ir = compiler.compile_source("(cons? (quote (1)))").unwrap();
        assert!(ir.contains("@rt_is_cons"));

        let ir = compiler.compile_source("(number? 42)").unwrap();
        assert!(ir.contains("@rt_is_number"));
    }

    #[test]
    fn test_compile_has_runtime_definitions() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("42").unwrap();

        // Should include runtime IR
        assert!(ir.contains("define %RuntimeValue @rt_cons"));
        assert!(ir.contains("define %RuntimeValue @rt_car"));
        assert!(ir.contains("define %RuntimeValue @rt_add"));
        assert!(ir.contains("define void @print_value"));
    }

    #[test]
    fn test_compile_multiple_expressions() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("1\n2\n3").unwrap();

        // Should have multiple expression functions
        assert!(ir.contains("__consair_expr_0"));
        assert!(ir.contains("__consair_expr_1"));
        assert!(ir.contains("__consair_expr_2"));
    }

    #[test]
    fn test_compile_lambda_call() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("((lambda (x) (+ x 1)) 5)").unwrap();

        // Should call rt_add (the lambda body)
        assert!(ir.contains("@rt_add"));
    }

    #[test]
    fn test_compile_lambda_multi_param() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source("((lambda (x y) (+ x y)) 3 4)")
            .unwrap();

        assert!(ir.contains("@rt_add"));
    }

    #[test]
    fn test_compile_nested_lambda() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source("((lambda (x) ((lambda (y) (+ x y)) 10)) 5)")
            .unwrap();

        assert!(ir.contains("@rt_add"));
    }

    #[test]
    fn test_compile_label_factorial() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source(
                "((label fact (lambda (n) (cond ((= n 0) 1) (t (* n (fact (- n 1))))))) 5)",
            )
            .unwrap();

        // Should have a labeled function
        assert!(ir.contains("__consair_labeled_fact_"));
        // Should call rt_mul
        assert!(ir.contains("@rt_mul"));
    }

    #[test]
    fn test_compile_label_sum() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source(
                "((label sum (lambda (n) (cond ((= n 0) 0) (t (+ n (sum (- n 1))))))) 5)",
            )
            .unwrap();

        assert!(ir.contains("__consair_labeled_sum_"));
        assert!(ir.contains("@rt_add"));
    }

    #[test]
    fn test_compile_closure_simple() {
        let compiler = AotCompiler::new();
        // Closure that captures y and returns a closure
        let ir = compiler
            .compile_source("((lambda (f) (f 10)) ((lambda (y) (lambda (x) (+ x y))) 5))")
            .unwrap();

        // Should have closure creation and indirect call
        assert!(ir.contains("@rt_make_closure"));
        assert!(ir.contains("__consair_closure_"));
    }

    #[test]
    fn test_compile_closure_no_capture() {
        let compiler = AotCompiler::new();
        // Simple closure with no captures
        let ir = compiler
            .compile_source("((lambda (f) (f 3 4)) (lambda (a b) (+ a b)))")
            .unwrap();

        assert!(ir.contains("@rt_make_closure"));
    }

    #[test]
    fn test_compile_vector() {
        let compiler = AotCompiler::new();
        let ir = compiler.compile_source("(vector 1 2 3)").unwrap();

        assert!(ir.contains("@rt_make_vector"));
    }

    #[test]
    fn test_compile_vector_ref() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source("(vector-ref (vector 10 20 30) 1)")
            .unwrap();

        assert!(ir.contains("@rt_make_vector"));
        assert!(ir.contains("@rt_vector_ref"));
    }

    #[test]
    fn test_compile_vector_length() {
        let compiler = AotCompiler::new();
        let ir = compiler
            .compile_source("(vector-length (vector 1 2 3 4))")
            .unwrap();

        assert!(ir.contains("@rt_vector_length"));
    }
}
