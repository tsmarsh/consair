//! Code generation module for JIT compilation.
//!
//! This module provides LLVM IR code generation for Consair expressions.

use inkwell::builder::Builder;
use inkwell::context::Context;
use inkwell::module::Module;
use inkwell::types::{FunctionType, StructType};
use inkwell::values::FunctionValue;

/// Code generator for Consair expressions.
///
/// This struct holds the LLVM context, module, and builder needed to generate
/// LLVM IR from Consair expressions.
pub struct Codegen<'ctx> {
    /// The LLVM context - owns all LLVM data structures
    pub context: &'ctx Context,
    /// The LLVM module - contains functions and global variables
    pub module: Module<'ctx>,
    /// The IR builder - used to create instructions
    pub builder: Builder<'ctx>,

    // Type definitions
    /// The RuntimeValue type: { i8, i64 }
    pub value_type: StructType<'ctx>,

    // Runtime function declarations
    pub rt_cons: FunctionValue<'ctx>,
    pub rt_car: FunctionValue<'ctx>,
    pub rt_cdr: FunctionValue<'ctx>,
    pub rt_add: FunctionValue<'ctx>,
    pub rt_sub: FunctionValue<'ctx>,
    pub rt_mul: FunctionValue<'ctx>,
    pub rt_div: FunctionValue<'ctx>,
    pub rt_neg: FunctionValue<'ctx>,
    pub rt_num_eq: FunctionValue<'ctx>,
    pub rt_lt: FunctionValue<'ctx>,
    pub rt_gt: FunctionValue<'ctx>,
    pub rt_lte: FunctionValue<'ctx>,
    pub rt_gte: FunctionValue<'ctx>,
    pub rt_eq: FunctionValue<'ctx>,
    pub rt_is_nil: FunctionValue<'ctx>,
    pub rt_is_atom: FunctionValue<'ctx>,
    pub rt_is_cons: FunctionValue<'ctx>,
    pub rt_is_number: FunctionValue<'ctx>,
    pub rt_not: FunctionValue<'ctx>,
    pub rt_incref: FunctionValue<'ctx>,
    pub rt_decref: FunctionValue<'ctx>,
    // Closure functions
    pub rt_make_closure: FunctionValue<'ctx>,
    pub rt_closure_fn_ptr: FunctionValue<'ctx>,
    pub rt_closure_env_get: FunctionValue<'ctx>,
    pub rt_closure_env_size: FunctionValue<'ctx>,
    // Standard library functions
    pub rt_now: FunctionValue<'ctx>,
    pub rt_length: FunctionValue<'ctx>,
    pub rt_append: FunctionValue<'ctx>,
    pub rt_reverse: FunctionValue<'ctx>,
    pub rt_nth: FunctionValue<'ctx>,
    // Vector functions
    pub rt_make_vector: FunctionValue<'ctx>,
    pub rt_vector_length: FunctionValue<'ctx>,
    pub rt_vector_ref: FunctionValue<'ctx>,
    // I/O functions
    pub rt_println: FunctionValue<'ctx>,
    pub rt_print: FunctionValue<'ctx>,
}

impl<'ctx> Codegen<'ctx> {
    /// Create a new code generator with the given context and module name.
    pub fn new(context: &'ctx Context, module_name: &str) -> Self {
        let module = context.create_module(module_name);
        let builder = context.create_builder();

        // Define the RuntimeValue type: { i8, i64 }
        let i8_type = context.i8_type();
        let i64_type = context.i64_type();
        let value_type = context.struct_type(&[i8_type.into(), i64_type.into()], false);

        // Create a temporary codegen to get access to helper methods
        let mut codegen = Codegen {
            context,
            module,
            builder,
            value_type,
            // Initialize with dummy values - we'll set them properly below
            rt_cons: unsafe { std::mem::zeroed() },
            rt_car: unsafe { std::mem::zeroed() },
            rt_cdr: unsafe { std::mem::zeroed() },
            rt_add: unsafe { std::mem::zeroed() },
            rt_sub: unsafe { std::mem::zeroed() },
            rt_mul: unsafe { std::mem::zeroed() },
            rt_div: unsafe { std::mem::zeroed() },
            rt_neg: unsafe { std::mem::zeroed() },
            rt_num_eq: unsafe { std::mem::zeroed() },
            rt_lt: unsafe { std::mem::zeroed() },
            rt_gt: unsafe { std::mem::zeroed() },
            rt_lte: unsafe { std::mem::zeroed() },
            rt_gte: unsafe { std::mem::zeroed() },
            rt_eq: unsafe { std::mem::zeroed() },
            rt_is_nil: unsafe { std::mem::zeroed() },
            rt_is_atom: unsafe { std::mem::zeroed() },
            rt_is_cons: unsafe { std::mem::zeroed() },
            rt_is_number: unsafe { std::mem::zeroed() },
            rt_not: unsafe { std::mem::zeroed() },
            rt_incref: unsafe { std::mem::zeroed() },
            rt_decref: unsafe { std::mem::zeroed() },
            rt_make_closure: unsafe { std::mem::zeroed() },
            rt_closure_fn_ptr: unsafe { std::mem::zeroed() },
            rt_closure_env_get: unsafe { std::mem::zeroed() },
            rt_closure_env_size: unsafe { std::mem::zeroed() },
            // Standard library functions
            rt_now: unsafe { std::mem::zeroed() },
            rt_length: unsafe { std::mem::zeroed() },
            rt_append: unsafe { std::mem::zeroed() },
            rt_reverse: unsafe { std::mem::zeroed() },
            rt_nth: unsafe { std::mem::zeroed() },
            // Vector functions
            rt_make_vector: unsafe { std::mem::zeroed() },
            rt_vector_length: unsafe { std::mem::zeroed() },
            rt_vector_ref: unsafe { std::mem::zeroed() },
            // I/O functions
            rt_println: unsafe { std::mem::zeroed() },
            rt_print: unsafe { std::mem::zeroed() },
        };

        // Declare all runtime functions
        codegen.rt_cons = codegen.declare_binary_fn("rt_cons");
        codegen.rt_car = codegen.declare_unary_fn("rt_car");
        codegen.rt_cdr = codegen.declare_unary_fn("rt_cdr");
        codegen.rt_add = codegen.declare_binary_fn("rt_add");
        codegen.rt_sub = codegen.declare_binary_fn("rt_sub");
        codegen.rt_mul = codegen.declare_binary_fn("rt_mul");
        codegen.rt_div = codegen.declare_binary_fn("rt_div");
        codegen.rt_neg = codegen.declare_unary_fn("rt_neg");
        codegen.rt_num_eq = codegen.declare_binary_fn("rt_num_eq");
        codegen.rt_lt = codegen.declare_binary_fn("rt_lt");
        codegen.rt_gt = codegen.declare_binary_fn("rt_gt");
        codegen.rt_lte = codegen.declare_binary_fn("rt_lte");
        codegen.rt_gte = codegen.declare_binary_fn("rt_gte");
        codegen.rt_eq = codegen.declare_binary_fn("rt_eq");
        codegen.rt_is_nil = codegen.declare_unary_fn("rt_is_nil");
        codegen.rt_is_atom = codegen.declare_unary_fn("rt_is_atom");
        codegen.rt_is_cons = codegen.declare_unary_fn("rt_is_cons");
        codegen.rt_is_number = codegen.declare_unary_fn("rt_is_number");
        codegen.rt_not = codegen.declare_unary_fn("rt_not");
        codegen.rt_incref = codegen.declare_void_unary_fn("rt_incref");
        codegen.rt_decref = codegen.declare_void_unary_fn("rt_decref");

        // Closure functions
        codegen.rt_make_closure = codegen.declare_make_closure_fn();
        codegen.rt_closure_fn_ptr = codegen.declare_closure_fn_ptr_fn();
        codegen.rt_closure_env_get = codegen.declare_closure_env_get_fn();
        codegen.rt_closure_env_size = codegen.declare_closure_env_size_fn();

        // Standard library functions
        codegen.rt_now = codegen.declare_nullary_fn("rt_now");
        codegen.rt_length = codegen.declare_unary_fn("rt_length");
        codegen.rt_append = codegen.declare_binary_fn("rt_append");
        codegen.rt_reverse = codegen.declare_unary_fn("rt_reverse");
        codegen.rt_nth = codegen.declare_binary_fn("rt_nth");

        // Vector functions
        codegen.rt_make_vector = codegen.declare_make_vector_fn();
        codegen.rt_vector_length = codegen.declare_unary_fn("rt_vector_length");
        codegen.rt_vector_ref = codegen.declare_binary_fn("rt_vector_ref");

        // I/O functions
        codegen.rt_println = codegen.declare_unary_fn("rt_println");
        codegen.rt_print = codegen.declare_unary_fn("rt_print");

        codegen
    }

    /// Declare a nullary runtime function: () -> RuntimeValue
    fn declare_nullary_fn(&self, name: &str) -> FunctionValue<'ctx> {
        let fn_type = self.expr_fn_type();
        self.module
            .add_function(name, fn_type, Some(inkwell::module::Linkage::External))
    }

    /// Declare a unary runtime function: RuntimeValue -> RuntimeValue
    fn declare_unary_fn(&self, name: &str) -> FunctionValue<'ctx> {
        let fn_type = self.unary_fn_type();
        self.module
            .add_function(name, fn_type, Some(inkwell::module::Linkage::External))
    }

    /// Declare a binary runtime function: (RuntimeValue, RuntimeValue) -> RuntimeValue
    fn declare_binary_fn(&self, name: &str) -> FunctionValue<'ctx> {
        let fn_type = self.binary_fn_type();
        self.module
            .add_function(name, fn_type, Some(inkwell::module::Linkage::External))
    }

    /// Declare a void unary runtime function: RuntimeValue -> void
    fn declare_void_unary_fn(&self, name: &str) -> FunctionValue<'ctx> {
        let fn_type = self.void_unary_fn_type();
        self.module
            .add_function(name, fn_type, Some(inkwell::module::Linkage::External))
    }

    /// Get the function type for unary functions: RuntimeValue -> RuntimeValue
    fn unary_fn_type(&self) -> FunctionType<'ctx> {
        self.value_type.fn_type(&[self.value_type.into()], false)
    }

    /// Get the function type for binary functions: (RuntimeValue, RuntimeValue) -> RuntimeValue
    fn binary_fn_type(&self) -> FunctionType<'ctx> {
        self.value_type
            .fn_type(&[self.value_type.into(), self.value_type.into()], false)
    }

    /// Get the function type for void unary functions: RuntimeValue -> void
    fn void_unary_fn_type(&self) -> FunctionType<'ctx> {
        self.context
            .void_type()
            .fn_type(&[self.value_type.into()], false)
    }

    /// Get the function type for expression functions: () -> RuntimeValue
    pub fn expr_fn_type(&self) -> FunctionType<'ctx> {
        self.value_type.fn_type(&[], false)
    }

    // ========================================================================
    // Closure Function Declarations
    // ========================================================================

    /// Declare rt_make_closure: (ptr, *RuntimeValue, u32) -> RuntimeValue
    fn declare_make_closure_fn(&self) -> FunctionValue<'ctx> {
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let fn_type = self
            .value_type
            .fn_type(&[ptr_type.into(), ptr_type.into(), i32_type.into()], false);
        self.module.add_function(
            "rt_make_closure",
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    /// Declare rt_closure_fn_ptr: RuntimeValue -> ptr
    fn declare_closure_fn_ptr_fn(&self) -> FunctionValue<'ctx> {
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default());
        let fn_type = ptr_type.fn_type(&[self.value_type.into()], false);
        self.module.add_function(
            "rt_closure_fn_ptr",
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    /// Declare rt_closure_env_get: (RuntimeValue, u32) -> RuntimeValue
    fn declare_closure_env_get_fn(&self) -> FunctionValue<'ctx> {
        let i32_type = self.context.i32_type();
        let fn_type = self
            .value_type
            .fn_type(&[self.value_type.into(), i32_type.into()], false);
        self.module.add_function(
            "rt_closure_env_get",
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    /// Declare rt_closure_env_size: RuntimeValue -> u32
    fn declare_closure_env_size_fn(&self) -> FunctionValue<'ctx> {
        let i32_type = self.context.i32_type();
        let fn_type = i32_type.fn_type(&[self.value_type.into()], false);
        self.module.add_function(
            "rt_closure_env_size",
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    /// Declare rt_make_vector: (*RuntimeValue, u32) -> RuntimeValue
    fn declare_make_vector_fn(&self) -> FunctionValue<'ctx> {
        let ptr_type = self
            .context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default());
        let i32_type = self.context.i32_type();
        let fn_type = self
            .value_type
            .fn_type(&[ptr_type.into(), i32_type.into()], false);
        self.module.add_function(
            "rt_make_vector",
            fn_type,
            Some(inkwell::module::Linkage::External),
        )
    }

    /// Get pointer type (opaque pointer in LLVM 17+)
    pub fn ptr_type(&self) -> inkwell::types::PointerType<'ctx> {
        self.context
            .i8_type()
            .ptr_type(inkwell::AddressSpace::default())
    }

    /// Get the uniform closure function type: (env_ptr, args_ptr, num_args) -> RuntimeValue
    /// This allows all closures to be called uniformly via indirect calls.
    pub fn closure_fn_type(&self) -> FunctionType<'ctx> {
        let ptr_type = self.ptr_type();
        let i32_type = self.context.i32_type();
        self.value_type
            .fn_type(&[ptr_type.into(), ptr_type.into(), i32_type.into()], false)
    }

    /// Get i32 type
    pub fn i32_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i32_type()
    }

    /// Get the i8 type
    pub fn i8_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i8_type()
    }

    /// Get the i64 type
    pub fn i64_type(&self) -> inkwell::types::IntType<'ctx> {
        self.context.i64_type()
    }

    /// Get the f64 type
    pub fn f64_type(&self) -> inkwell::types::FloatType<'ctx> {
        self.context.f64_type()
    }

    /// Create a RuntimeValue constant from tag and data
    pub fn const_runtime_value(&self, tag: u8, data: u64) -> inkwell::values::StructValue<'ctx> {
        let tag_val = self.i8_type().const_int(tag as u64, false);
        let data_val = self.i64_type().const_int(data, false);
        self.value_type
            .const_named_struct(&[tag_val.into(), data_val.into()])
    }

    /// Emit the generated LLVM IR as a string for debugging.
    pub fn emit_ir(&self) -> String {
        self.module.print_to_string().to_string()
    }

    /// Verify the module, returning an error message if verification fails.
    pub fn verify(&self) -> Result<(), String> {
        self.module.verify().map_err(|e| e.to_string())
    }

    /// Create a new function in the module
    pub fn add_function(&self, name: &str, fn_type: FunctionType<'ctx>) -> FunctionValue<'ctx> {
        self.module.add_function(name, fn_type, None)
    }

    // ========================================================================
    // Literal Compilation
    // ========================================================================

    /// Compile a nil literal.
    pub fn compile_nil(&self) -> inkwell::values::StructValue<'ctx> {
        self.const_runtime_value(crate::runtime::TAG_NIL, 0)
    }

    /// Compile a boolean literal.
    pub fn compile_bool(&self, b: bool) -> inkwell::values::StructValue<'ctx> {
        self.const_runtime_value(crate::runtime::TAG_BOOL, b as u64)
    }

    /// Compile an integer literal.
    pub fn compile_int(&self, n: i64) -> inkwell::values::StructValue<'ctx> {
        self.const_runtime_value(crate::runtime::TAG_INT, n as u64)
    }

    /// Compile a floating-point literal.
    pub fn compile_float(&self, f: f64) -> inkwell::values::StructValue<'ctx> {
        self.const_runtime_value(crate::runtime::TAG_FLOAT, f.to_bits())
    }

    /// Compile a symbol literal from an interned symbol key.
    pub fn compile_symbol(&self, key: u64) -> inkwell::values::StructValue<'ctx> {
        self.const_runtime_value(crate::runtime::TAG_SYMBOL, key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_codegen_creation() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test_module");

        assert_eq!(codegen.module.get_name().to_str().unwrap(), "test_module");
    }

    #[test]
    fn test_value_type_structure() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        // RuntimeValue should be { i8, i64 }
        assert_eq!(codegen.value_type.count_fields(), 2);
    }

    #[test]
    fn test_runtime_functions_declared() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        // Check that all runtime functions are declared
        assert!(codegen.module.get_function("rt_cons").is_some());
        assert!(codegen.module.get_function("rt_car").is_some());
        assert!(codegen.module.get_function("rt_cdr").is_some());
        assert!(codegen.module.get_function("rt_add").is_some());
        assert!(codegen.module.get_function("rt_sub").is_some());
        assert!(codegen.module.get_function("rt_mul").is_some());
        assert!(codegen.module.get_function("rt_div").is_some());
        assert!(codegen.module.get_function("rt_eq").is_some());
        assert!(codegen.module.get_function("rt_lt").is_some());
        assert!(codegen.module.get_function("rt_is_atom").is_some());
        assert!(codegen.module.get_function("rt_is_nil").is_some());
    }

    #[test]
    fn test_emit_ir() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        let ir = codegen.emit_ir();
        assert!(ir.contains("declare"));
        assert!(ir.contains("rt_cons"));
        assert!(ir.contains("rt_add"));
    }

    #[test]
    fn test_verify_empty_module() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        // Empty module should verify successfully
        assert!(codegen.verify().is_ok());
    }

    #[test]
    fn test_const_runtime_value() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        // Create a constant nil value (tag=0, data=0)
        let nil_val = codegen.const_runtime_value(0, 0);
        // Verify it's the right type
        assert_eq!(nil_val.get_type(), codegen.value_type);

        // Create a constant int value (tag=2, data=42)
        let int_val = codegen.const_runtime_value(2, 42);
        assert_eq!(int_val.get_type(), codegen.value_type);
    }

    #[test]
    fn test_create_simple_function() {
        let context = Context::create();
        let codegen = Codegen::new(&context, "test");

        // Create a function that returns nil
        let fn_type = codegen.expr_fn_type();
        let function = codegen.add_function("return_nil", fn_type);

        // Create entry block
        let entry = context.append_basic_block(function, "entry");
        codegen.builder.position_at_end(entry);

        // Return nil: { i8 0, i64 0 }
        let nil_val = codegen.const_runtime_value(0, 0);
        codegen.builder.build_return(Some(&nil_val)).unwrap();

        // Module should verify
        assert!(codegen.verify().is_ok());

        // Check IR contains our function
        let ir = codegen.emit_ir();
        assert!(ir.contains("return_nil"));
        assert!(ir.contains("ret"));
    }
}
