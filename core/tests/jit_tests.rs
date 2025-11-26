//! JIT compilation tests
//!
//! These tests verify the JIT compilation infrastructure works correctly.

use inkwell::context::Context;

/// Verify that inkwell links correctly and we can create basic LLVM structures.
#[test]
fn test_inkwell_links() {
    let context = Context::create();
    let module = context.create_module("test");
    assert_eq!(module.get_name().to_str().unwrap(), "test");
}

/// Verify we can create a simple function in LLVM IR.
#[test]
fn test_create_function() {
    let context = Context::create();
    let module = context.create_module("test");
    let builder = context.create_builder();

    // Create a simple function that returns i64
    let i64_type = context.i64_type();
    let fn_type = i64_type.fn_type(&[], false);
    let function = module.add_function("test_fn", fn_type, None);

    // Create entry block
    let entry = context.append_basic_block(function, "entry");
    builder.position_at_end(entry);

    // Return 42
    let ret_val = i64_type.const_int(42, false);
    builder.build_return(Some(&ret_val)).unwrap();

    // Verify the module
    assert!(module.verify().is_ok());
}

/// Verify we can create struct types (needed for RuntimeValue representation).
#[test]
fn test_create_struct_type() {
    let context = Context::create();

    // RuntimeValue will be { i8, i64 } - tag and data
    let i8_type = context.i8_type();
    let i64_type = context.i64_type();
    let runtime_value_type = context.struct_type(&[i8_type.into(), i64_type.into()], false);

    assert!(!runtime_value_type.is_packed());
    assert_eq!(runtime_value_type.count_fields(), 2);
}
