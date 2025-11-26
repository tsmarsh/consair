//! LLVM IR definitions for runtime functions.
//!
//! This module contains hand-written LLVM IR for the runtime functions
//! that need to be embedded in AOT-compiled output.

use cons::runtime::{
    TAG_BOOL, TAG_CLOSURE, TAG_CONS, TAG_FLOAT, TAG_INT, TAG_NIL, TAG_STRING, TAG_SYMBOL,
    TAG_VECTOR,
};

/// Generate the complete runtime LLVM IR as a string.
///
/// This includes type definitions, constants, and all runtime function definitions
/// needed for standalone AOT-compiled executables.
pub fn generate_runtime_ir() -> String {
    let mut ir = String::new();

    // Type definitions
    ir.push_str(&generate_type_definitions());

    // External declarations (libc)
    ir.push_str(&generate_external_declarations());

    // Runtime function definitions
    ir.push_str(&generate_runtime_functions());

    // Print function for main (print_value, print_list)
    ir.push_str(&generate_print_result());

    // I/O functions (depend on print_value, so must come after)
    ir.push_str(&generate_io_functions());

    ir
}

fn generate_type_definitions() -> String {
    format!(
        r#"
; Type definitions
%RuntimeValue = type {{ i8, i64 }}
%RuntimeConsCell = type {{ %RuntimeValue, %RuntimeValue, i32 }}
%RuntimeClosure = type {{ ptr, ptr, i32, i32 }}
%RuntimeString = type {{ ptr, i64, i32 }}
%RuntimeVector = type {{ ptr, i64, i32 }}

; Tag constants
@TAG_NIL = private constant i8 {TAG_NIL}
@TAG_BOOL = private constant i8 {TAG_BOOL}
@TAG_INT = private constant i8 {TAG_INT}
@TAG_FLOAT = private constant i8 {TAG_FLOAT}
@TAG_CONS = private constant i8 {TAG_CONS}
@TAG_SYMBOL = private constant i8 {TAG_SYMBOL}
@TAG_CLOSURE = private constant i8 {TAG_CLOSURE}
@TAG_STRING = private constant i8 {TAG_STRING}
@TAG_VECTOR = private constant i8 {TAG_VECTOR}

; Format strings for printing
@fmt_nil = private constant [4 x i8] c"nil\00"
@fmt_true = private constant [5 x i8] c"true\00"
@fmt_false = private constant [6 x i8] c"false\00"
@fmt_int = private constant [5 x i8] c"%lld\00"
@fmt_float = private constant [3 x i8] c"%g\00"
@fmt_cons_open = private constant [2 x i8] c"(\00"
@fmt_cons_close = private constant [2 x i8] c")\00"
@fmt_space = private constant [2 x i8] c" \00"
@fmt_dot = private constant [4 x i8] c" . \00"
@fmt_newline = private constant [2 x i8] c"\0A\00"
@fmt_string = private constant [5 x i8] c"%.*s\00"
"#
    )
}

fn generate_external_declarations() -> String {
    r#"
; External declarations (libc)
declare ptr @malloc(i64)
declare void @free(ptr)
declare i32 @printf(ptr, ...)
declare ptr @memcpy(ptr, ptr, i64)
"#
    .to_string()
}

fn generate_runtime_functions() -> String {
    let mut ir = String::new();

    // rt_cons - allocate a new cons cell
    ir.push_str(&generate_rt_cons());

    // rt_car - get car of cons cell
    ir.push_str(&generate_rt_car());

    // rt_cdr - get cdr of cons cell
    ir.push_str(&generate_rt_cdr());

    // Arithmetic functions
    ir.push_str(&generate_rt_add());
    ir.push_str(&generate_rt_sub());
    ir.push_str(&generate_rt_mul());
    ir.push_str(&generate_rt_div());
    ir.push_str(&generate_rt_neg());

    // Comparison functions
    ir.push_str(&generate_rt_num_eq());
    ir.push_str(&generate_rt_lt());
    ir.push_str(&generate_rt_gt());
    ir.push_str(&generate_rt_lte());
    ir.push_str(&generate_rt_gte());
    ir.push_str(&generate_rt_eq());

    // Type predicates
    ir.push_str(&generate_rt_is_nil());
    ir.push_str(&generate_rt_is_atom());
    ir.push_str(&generate_rt_is_cons());
    ir.push_str(&generate_rt_is_number());
    ir.push_str(&generate_rt_not());

    // Reference counting (simplified - no actual refcounting for AOT)
    ir.push_str(&generate_rt_incref());
    ir.push_str(&generate_rt_decref());

    // Closure functions
    ir.push_str(&generate_rt_make_closure());
    ir.push_str(&generate_rt_closure_fn_ptr());
    ir.push_str(&generate_rt_closure_env_get());
    ir.push_str(&generate_rt_closure_env_size());

    // List functions
    ir.push_str(&generate_rt_length());
    ir.push_str(&generate_rt_append());
    ir.push_str(&generate_rt_reverse());
    ir.push_str(&generate_rt_nth());

    // Vector functions
    ir.push_str(&generate_rt_make_vector());
    ir.push_str(&generate_rt_vector_length());
    ir.push_str(&generate_rt_vector_ref());

    // String functions
    ir.push_str(&generate_rt_make_string());

    // Utility
    ir.push_str(&generate_rt_now());

    ir
}

/// Generate I/O functions (these depend on print_value, so must come after generate_print_result)
fn generate_io_functions() -> String {
    let mut ir = String::new();
    ir.push_str(&generate_rt_println());
    ir.push_str(&generate_rt_print());
    ir.push_str(&generate_rt_print_space());
    ir.push_str(&generate_rt_print_newline());
    ir
}

fn generate_rt_cons() -> String {
    format!(
        r#"
; rt_cons: Allocate a new cons cell
define %RuntimeValue @rt_cons(%RuntimeValue %car, %RuntimeValue %cdr) {{
entry:
  ; Allocate cons cell (2 RuntimeValues + refcount = 16 + 16 + 4 = 36 bytes, round to 40)
  %cell_ptr = call ptr @malloc(i64 40)

  ; Store car
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  store %RuntimeValue %car, ptr %car_ptr

  ; Store cdr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  store %RuntimeValue %cdr, ptr %cdr_ptr

  ; Initialize refcount to 1
  %refcount_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 2
  store i32 1, ptr %refcount_ptr

  ; Create result RuntimeValue
  %ptr_int = ptrtoint ptr %cell_ptr to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_CONS}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %ptr_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_car() -> String {
    format!(
        r#"
; rt_car: Get the car of a cons cell
define %RuntimeValue @rt_car(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %extract, label %error

extract:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  %car = load %RuntimeValue, ptr %car_ptr
  ret %RuntimeValue %car

error:
  ; Return nil on error
  %nil = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_cdr() -> String {
    format!(
        r#"
; rt_cdr: Get the cdr of a cons cell
define %RuntimeValue @rt_cdr(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %extract, label %error

extract:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr
  ret %RuntimeValue %cdr

error:
  %nil = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_add() -> String {
    format!(
        r#"
; rt_add: Add two numbers
define %RuntimeValue @rt_add(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  ; Check if both are integers
  %a_is_int = icmp eq i8 %a_tag, {TAG_INT}
  %b_is_int = icmp eq i8 %b_tag, {TAG_INT}
  %both_int = and i1 %a_is_int, %b_is_int
  br i1 %both_int, label %int_add, label %float_add

int_add:
  ; Integer addition
  %a_int = bitcast i64 %a_data to i64
  %b_int = bitcast i64 %b_data to i64
  %sum_int = add i64 %a_int, %b_int
  %result_int1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result_int2 = insertvalue %RuntimeValue %result_int1, i64 %sum_int, 1
  ret %RuntimeValue %result_int2

float_add:
  ; Convert to float and add
  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float = select i1 %a_is_float, double bitcast (i64 0 to double), double 0.0
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %sum_float = fadd double %a_final, %b_final
  %sum_bits = bitcast double %sum_float to i64
  %result_float1 = insertvalue %RuntimeValue undef, i8 {TAG_FLOAT}, 0
  %result_float2 = insertvalue %RuntimeValue %result_float1, i64 %sum_bits, 1
  ret %RuntimeValue %result_float2
}}
"#
    )
}

fn generate_rt_sub() -> String {
    format!(
        r#"
; rt_sub: Subtract two numbers
define %RuntimeValue @rt_sub(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_int = icmp eq i8 %a_tag, {TAG_INT}
  %b_is_int = icmp eq i8 %b_tag, {TAG_INT}
  %both_int = and i1 %a_is_int, %b_is_int
  br i1 %both_int, label %int_sub, label %float_sub

int_sub:
  %diff_int = sub i64 %a_data, %b_data
  %result_int1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result_int2 = insertvalue %RuntimeValue %result_int1, i64 %diff_int, 1
  ret %RuntimeValue %result_int2

float_sub:
  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %diff_float = fsub double %a_final, %b_final
  %diff_bits = bitcast double %diff_float to i64
  %result_float1 = insertvalue %RuntimeValue undef, i8 {TAG_FLOAT}, 0
  %result_float2 = insertvalue %RuntimeValue %result_float1, i64 %diff_bits, 1
  ret %RuntimeValue %result_float2
}}
"#
    )
}

fn generate_rt_mul() -> String {
    format!(
        r#"
; rt_mul: Multiply two numbers
define %RuntimeValue @rt_mul(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_int = icmp eq i8 %a_tag, {TAG_INT}
  %b_is_int = icmp eq i8 %b_tag, {TAG_INT}
  %both_int = and i1 %a_is_int, %b_is_int
  br i1 %both_int, label %int_mul, label %float_mul

int_mul:
  %prod_int = mul i64 %a_data, %b_data
  %result_int1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result_int2 = insertvalue %RuntimeValue %result_int1, i64 %prod_int, 1
  ret %RuntimeValue %result_int2

float_mul:
  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %prod_float = fmul double %a_final, %b_final
  %prod_bits = bitcast double %prod_float to i64
  %result_float1 = insertvalue %RuntimeValue undef, i8 {TAG_FLOAT}, 0
  %result_float2 = insertvalue %RuntimeValue %result_float1, i64 %prod_bits, 1
  ret %RuntimeValue %result_float2
}}
"#
    )
}

fn generate_rt_div() -> String {
    format!(
        r#"
; rt_div: Divide two numbers
define %RuntimeValue @rt_div(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  ; Always do float division for simplicity
  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %quot_float = fdiv double %a_final, %b_final
  %quot_bits = bitcast double %quot_float to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_FLOAT}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %quot_bits, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_neg() -> String {
    format!(
        r#"
; rt_neg: Negate a number
define %RuntimeValue @rt_neg(%RuntimeValue %a) {{
entry:
  %tag = extractvalue %RuntimeValue %a, 0
  %data = extractvalue %RuntimeValue %a, 1

  %is_int = icmp eq i8 %tag, {TAG_INT}
  br i1 %is_int, label %neg_int, label %neg_float

neg_int:
  %neg_val = sub i64 0, %data
  %result_int1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result_int2 = insertvalue %RuntimeValue %result_int1, i64 %neg_val, 1
  ret %RuntimeValue %result_int2

neg_float:
  %float_val = bitcast i64 %data to double
  %neg_float_val = fneg double %float_val
  %neg_bits = bitcast double %neg_float_val to i64
  %result_float1 = insertvalue %RuntimeValue undef, i8 {TAG_FLOAT}, 0
  %result_float2 = insertvalue %RuntimeValue %result_float1, i64 %neg_bits, 1
  ret %RuntimeValue %result_float2
}}
"#
    )
}

fn generate_rt_num_eq() -> String {
    format!(
        r#"
; rt_num_eq: Numeric equality
define %RuntimeValue @rt_num_eq(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  ; Convert to float for comparison
  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %eq = fcmp oeq double %a_final, %b_final
  %eq_int = zext i1 %eq to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %eq_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_lt() -> String {
    format!(
        r#"
; rt_lt: Less than comparison
define %RuntimeValue @rt_lt(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %lt = fcmp olt double %a_final, %b_final
  %lt_int = zext i1 %lt to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %lt_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_gt() -> String {
    format!(
        r#"
; rt_gt: Greater than comparison
define %RuntimeValue @rt_gt(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %gt = fcmp ogt double %a_final, %b_final
  %gt_int = zext i1 %gt to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %gt_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_lte() -> String {
    format!(
        r#"
; rt_lte: Less than or equal comparison
define %RuntimeValue @rt_lte(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %lte = fcmp ole double %a_final, %b_final
  %lte_int = zext i1 %lte to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %lte_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_gte() -> String {
    format!(
        r#"
; rt_gte: Greater than or equal comparison
define %RuntimeValue @rt_gte(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1

  %a_is_float = icmp eq i8 %a_tag, {TAG_FLOAT}
  %a_float_bits = select i1 %a_is_float, i64 %a_data, i64 0
  %a_float_val = bitcast i64 %a_float_bits to double
  %a_int_val = sitofp i64 %a_data to double
  %a_final = select i1 %a_is_float, double %a_float_val, double %a_int_val

  %b_is_float = icmp eq i8 %b_tag, {TAG_FLOAT}
  %b_float_bits = select i1 %b_is_float, i64 %b_data, i64 0
  %b_float_val = bitcast i64 %b_float_bits to double
  %b_int_val = sitofp i64 %b_data to double
  %b_final = select i1 %b_is_float, double %b_float_val, double %b_int_val

  %gte = fcmp oge double %a_final, %b_final
  %gte_int = zext i1 %gte to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %gte_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_eq() -> String {
    format!(
        r#"
; rt_eq: Value equality
define %RuntimeValue @rt_eq(%RuntimeValue %a, %RuntimeValue %b) {{
entry:
  %a_tag = extractvalue %RuntimeValue %a, 0
  %b_tag = extractvalue %RuntimeValue %b, 0
  %tags_equal = icmp eq i8 %a_tag, %b_tag
  br i1 %tags_equal, label %check_data, label %not_equal

check_data:
  %a_data = extractvalue %RuntimeValue %a, 1
  %b_data = extractvalue %RuntimeValue %b, 1
  %data_equal = icmp eq i64 %a_data, %b_data
  br i1 %data_equal, label %equal, label %not_equal

equal:
  %result_true1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result_true2 = insertvalue %RuntimeValue %result_true1, i64 1, 1
  ret %RuntimeValue %result_true2

not_equal:
  %result_false1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result_false2 = insertvalue %RuntimeValue %result_false1, i64 0, 1
  ret %RuntimeValue %result_false2
}}
"#
    )
}

fn generate_rt_is_nil() -> String {
    format!(
        r#"
; rt_is_nil: Check if value is nil
define %RuntimeValue @rt_is_nil(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_nil = icmp eq i8 %tag, {TAG_NIL}
  %is_nil_int = zext i1 %is_nil to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %is_nil_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_is_atom() -> String {
    format!(
        r#"
; rt_is_atom: Check if value is an atom (not a cons cell)
define %RuntimeValue @rt_is_atom(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  %is_atom = xor i1 %is_cons, true
  %is_atom_int = zext i1 %is_atom to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %is_atom_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_is_cons() -> String {
    format!(
        r#"
; rt_is_cons: Check if value is a cons cell
define %RuntimeValue @rt_is_cons(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  %is_cons_int = zext i1 %is_cons to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %is_cons_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_is_number() -> String {
    format!(
        r#"
; rt_is_number: Check if value is a number
define %RuntimeValue @rt_is_number(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_int = icmp eq i8 %tag, {TAG_INT}
  %is_float = icmp eq i8 %tag, {TAG_FLOAT}
  %is_number = or i1 %is_int, %is_float
  %is_number_int = zext i1 %is_number to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %is_number_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_not() -> String {
    format!(
        r#"
; rt_not: Boolean not
define %RuntimeValue @rt_not(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %data = extractvalue %RuntimeValue %val, 1

  ; Check if nil
  %is_nil = icmp eq i8 %tag, {TAG_NIL}
  br i1 %is_nil, label %return_true, label %check_bool

check_bool:
  ; Check if false
  %is_bool = icmp eq i8 %tag, {TAG_BOOL}
  %is_false = icmp eq i64 %data, 0
  %is_bool_false = and i1 %is_bool, %is_false
  br i1 %is_bool_false, label %return_true, label %return_false

return_true:
  %result_true1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result_true2 = insertvalue %RuntimeValue %result_true1, i64 1, 1
  ret %RuntimeValue %result_true2

return_false:
  %result_false1 = insertvalue %RuntimeValue undef, i8 {TAG_BOOL}, 0
  %result_false2 = insertvalue %RuntimeValue %result_false1, i64 0, 1
  ret %RuntimeValue %result_false2
}}
"#
    )
}

fn generate_rt_incref() -> String {
    r#"
; rt_incref: Increment reference count (no-op for AOT)
define void @rt_incref(%RuntimeValue %val) {
entry:
  ret void
}
"#
    .to_string()
}

fn generate_rt_decref() -> String {
    r#"
; rt_decref: Decrement reference count (no-op for AOT - rely on OS cleanup)
define void @rt_decref(%RuntimeValue %val) {
entry:
  ret void
}
"#
    .to_string()
}

fn generate_rt_make_closure() -> String {
    format!(
        r#"
; rt_make_closure: Create a closure
define %RuntimeValue @rt_make_closure(ptr %fn_ptr, ptr %env_values, i32 %env_size) {{
entry:
  ; Allocate closure struct
  %closure_ptr = call ptr @malloc(i64 32)

  ; Store function pointer
  %fn_ptr_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 0
  store ptr %fn_ptr, ptr %fn_ptr_slot

  ; Allocate and copy environment if non-empty
  %env_empty = icmp eq i32 %env_size, 0
  br i1 %env_empty, label %store_null_env, label %copy_env

copy_env:
  %env_bytes = mul i32 %env_size, 16  ; sizeof(RuntimeValue) = 16
  %env_bytes_64 = zext i32 %env_bytes to i64
  %new_env = call ptr @malloc(i64 %env_bytes_64)
  call ptr @memcpy(ptr %new_env, ptr %env_values, i64 %env_bytes_64)
  br label %store_env

store_null_env:
  br label %store_env

store_env:
  %env_to_store = phi ptr [ null, %store_null_env ], [ %new_env, %copy_env ]
  %env_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 1
  store ptr %env_to_store, ptr %env_slot

  ; Store env size
  %size_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 2
  store i32 %env_size, ptr %size_slot

  ; Store refcount
  %refcount_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 3
  store i32 1, ptr %refcount_slot

  ; Create result RuntimeValue
  %ptr_int = ptrtoint ptr %closure_ptr to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_CLOSURE}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %ptr_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_closure_fn_ptr() -> String {
    format!(
        r#"
; rt_closure_fn_ptr: Get function pointer from closure
define ptr @rt_closure_fn_ptr(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_closure = icmp eq i8 %tag, {TAG_CLOSURE}
  br i1 %is_closure, label %extract, label %error

extract:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %closure_ptr = inttoptr i64 %ptr_int to ptr
  %fn_ptr_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 0
  %fn_ptr = load ptr, ptr %fn_ptr_slot
  ret ptr %fn_ptr

error:
  ret ptr null
}}
"#
    )
}

fn generate_rt_closure_env_get() -> String {
    format!(
        r#"
; rt_closure_env_get: Get captured value from closure environment
define %RuntimeValue @rt_closure_env_get(%RuntimeValue %val, i32 %index) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_closure = icmp eq i8 %tag, {TAG_CLOSURE}
  br i1 %is_closure, label %check_bounds, label %return_nil

check_bounds:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %closure_ptr = inttoptr i64 %ptr_int to ptr
  %size_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 2
  %env_size = load i32, ptr %size_slot
  %in_bounds = icmp ult i32 %index, %env_size
  br i1 %in_bounds, label %extract, label %return_nil

extract:
  %env_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 1
  %env = load ptr, ptr %env_slot
  %val_ptr = getelementptr %RuntimeValue, ptr %env, i32 %index
  %result = load %RuntimeValue, ptr %val_ptr
  ret %RuntimeValue %result

return_nil:
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_closure_env_size() -> String {
    format!(
        r#"
; rt_closure_env_size: Get size of closure environment
define i32 @rt_closure_env_size(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_closure = icmp eq i8 %tag, {TAG_CLOSURE}
  br i1 %is_closure, label %extract, label %return_zero

extract:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %closure_ptr = inttoptr i64 %ptr_int to ptr
  %size_slot = getelementptr %RuntimeClosure, ptr %closure_ptr, i32 0, i32 2
  %env_size = load i32, ptr %size_slot
  ret i32 %env_size

return_zero:
  ret i32 0
}}
"#
    )
}

fn generate_rt_length() -> String {
    format!(
        r#"
; rt_length: Get length of a list
define %RuntimeValue @rt_length(%RuntimeValue %val) {{
entry:
  br label %loop

loop:
  %count = phi i64 [ 0, %entry ], [ %next_count, %next ]
  %current = phi %RuntimeValue [ %val, %entry ], [ %cdr, %next ]

  %tag = extractvalue %RuntimeValue %current, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %next, label %done

next:
  %next_count = add i64 %count, 1
  %ptr_int = extractvalue %RuntimeValue %current, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr
  br label %loop

done:
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %count, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_append() -> String {
    format!(
        r#"
; rt_append: Append two lists
define %RuntimeValue @rt_append(%RuntimeValue %list1, %RuntimeValue %list2) {{
entry:
  %tag1 = extractvalue %RuntimeValue %list1, 0
  %is_nil = icmp eq i8 %tag1, {TAG_NIL}
  br i1 %is_nil, label %return_list2, label %check_cons

check_cons:
  %is_cons = icmp eq i8 %tag1, {TAG_CONS}
  br i1 %is_cons, label %do_append, label %return_list2

return_list2:
  ret %RuntimeValue %list2

do_append:
  ; Get car and cdr of list1
  %ptr_int = extractvalue %RuntimeValue %list1, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  %car = load %RuntimeValue, ptr %car_ptr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr

  ; Recursively append cdr with list2
  %rest = call %RuntimeValue @rt_append(%RuntimeValue %cdr, %RuntimeValue %list2)

  ; Cons car with the result
  %result = call %RuntimeValue @rt_cons(%RuntimeValue %car, %RuntimeValue %rest)
  ret %RuntimeValue %result
}}
"#
    )
}

fn generate_rt_reverse() -> String {
    format!(
        r#"
; rt_reverse: Reverse a list
define %RuntimeValue @rt_reverse(%RuntimeValue %list) {{
entry:
  ; Start with nil as accumulator
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  br label %loop

loop:
  %acc = phi %RuntimeValue [ %nil2, %entry ], [ %new_acc, %next ]
  %current = phi %RuntimeValue [ %list, %entry ], [ %cdr, %next ]

  %tag = extractvalue %RuntimeValue %current, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %next, label %done

next:
  %ptr_int = extractvalue %RuntimeValue %current, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  %car = load %RuntimeValue, ptr %car_ptr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr

  ; Cons car onto accumulator
  %new_acc = call %RuntimeValue @rt_cons(%RuntimeValue %car, %RuntimeValue %acc)
  br label %loop

done:
  ret %RuntimeValue %acc
}}
"#
    )
}

fn generate_rt_nth() -> String {
    format!(
        r#"
; rt_nth: Get nth element of a list
define %RuntimeValue @rt_nth(%RuntimeValue %list, %RuntimeValue %index) {{
entry:
  %idx_tag = extractvalue %RuntimeValue %index, 0
  %is_int = icmp eq i8 %idx_tag, {TAG_INT}
  br i1 %is_int, label %start_loop, label %return_nil

start_loop:
  %n = extractvalue %RuntimeValue %index, 1
  br label %loop

loop:
  %i = phi i64 [ 0, %start_loop ], [ %next_i, %next ]
  %current = phi %RuntimeValue [ %list, %start_loop ], [ %cdr, %next ]

  %tag = extractvalue %RuntimeValue %current, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %check_index, label %return_nil

check_index:
  %found = icmp eq i64 %i, %n
  br i1 %found, label %return_car, label %next

return_car:
  %ptr_int = extractvalue %RuntimeValue %current, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  %car = load %RuntimeValue, ptr %car_ptr
  ret %RuntimeValue %car

next:
  %next_i = add i64 %i, 1
  %ptr_int2 = extractvalue %RuntimeValue %current, 1
  %cell_ptr2 = inttoptr i64 %ptr_int2 to ptr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr2, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr
  br label %loop

return_nil:
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_make_vector() -> String {
    format!(
        r#"
; rt_make_vector: Create a vector from elements
define %RuntimeValue @rt_make_vector(ptr %elements, i32 %len) {{
entry:
  ; Allocate vector struct
  %vec_ptr = call ptr @malloc(i64 24)

  ; Allocate and copy elements if non-empty
  %empty = icmp eq i32 %len, 0
  br i1 %empty, label %store_null, label %copy_elements

copy_elements:
  %bytes = mul i32 %len, 16
  %bytes_64 = zext i32 %bytes to i64
  %new_elements = call ptr @malloc(i64 %bytes_64)
  call ptr @memcpy(ptr %new_elements, ptr %elements, i64 %bytes_64)
  br label %store_elements

store_null:
  br label %store_elements

store_elements:
  %elements_to_store = phi ptr [ null, %store_null ], [ %new_elements, %copy_elements ]
  %elements_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 0
  store ptr %elements_to_store, ptr %elements_slot

  ; Store length
  %len_64 = zext i32 %len to i64
  %len_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 1
  store i64 %len_64, ptr %len_slot

  ; Store refcount
  %refcount_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 2
  store i32 1, ptr %refcount_slot

  ; Create result
  %ptr_int = ptrtoint ptr %vec_ptr to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_VECTOR}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %ptr_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_vector_length() -> String {
    format!(
        r#"
; rt_vector_length: Get length of a vector
define %RuntimeValue @rt_vector_length(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %is_vector = icmp eq i8 %tag, {TAG_VECTOR}
  br i1 %is_vector, label %extract, label %return_zero

extract:
  %ptr_int = extractvalue %RuntimeValue %val, 1
  %vec_ptr = inttoptr i64 %ptr_int to ptr
  %len_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 1
  %len = load i64, ptr %len_slot
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %len, 1
  ret %RuntimeValue %result2

return_zero:
  %zero1 = insertvalue %RuntimeValue undef, i8 {TAG_INT}, 0
  %zero2 = insertvalue %RuntimeValue %zero1, i64 0, 1
  ret %RuntimeValue %zero2
}}
"#
    )
}

fn generate_rt_vector_ref() -> String {
    format!(
        r#"
; rt_vector_ref: Get element from vector by index
define %RuntimeValue @rt_vector_ref(%RuntimeValue %vec, %RuntimeValue %index) {{
entry:
  %vec_tag = extractvalue %RuntimeValue %vec, 0
  %is_vector = icmp eq i8 %vec_tag, {TAG_VECTOR}
  br i1 %is_vector, label %check_index, label %return_nil

check_index:
  %idx_tag = extractvalue %RuntimeValue %index, 0
  %is_int = icmp eq i8 %idx_tag, {TAG_INT}
  br i1 %is_int, label %bounds_check, label %return_nil

bounds_check:
  %idx = extractvalue %RuntimeValue %index, 1
  %ptr_int = extractvalue %RuntimeValue %vec, 1
  %vec_ptr = inttoptr i64 %ptr_int to ptr
  %len_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 1
  %len = load i64, ptr %len_slot
  %in_bounds = icmp ult i64 %idx, %len
  br i1 %in_bounds, label %extract, label %return_nil

extract:
  %elements_slot = getelementptr %RuntimeVector, ptr %vec_ptr, i32 0, i32 0
  %elements = load ptr, ptr %elements_slot
  %idx_32 = trunc i64 %idx to i32
  %elem_ptr = getelementptr %RuntimeValue, ptr %elements, i32 %idx_32
  %result = load %RuntimeValue, ptr %elem_ptr
  ret %RuntimeValue %result

return_nil:
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_make_string() -> String {
    format!(
        r#"
; rt_make_string: Create a string from a pointer and length
; For string literals, the data pointer points to constant data (no allocation needed)
define %RuntimeValue @rt_make_string(ptr %data, i64 %len) {{
entry:
  ; Allocate RuntimeString struct (ptr + i64 + i32 = 8 + 8 + 4 = 20, round to 24)
  %str_ptr = call ptr @malloc(i64 24)

  ; Store data pointer
  %data_slot = getelementptr %RuntimeString, ptr %str_ptr, i32 0, i32 0
  store ptr %data, ptr %data_slot

  ; Store length
  %len_slot = getelementptr %RuntimeString, ptr %str_ptr, i32 0, i32 1
  store i64 %len, ptr %len_slot

  ; Store refcount (1 for new allocation)
  %refcount_slot = getelementptr %RuntimeString, ptr %str_ptr, i32 0, i32 2
  store i32 1, ptr %refcount_slot

  ; Create result RuntimeValue
  %ptr_int = ptrtoint ptr %str_ptr to i64
  %result1 = insertvalue %RuntimeValue undef, i8 {TAG_STRING}, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 %ptr_int, 1
  ret %RuntimeValue %result2
}}
"#
    )
}

fn generate_rt_now() -> String {
    r#"
; rt_now: Get current Unix timestamp (stub - returns 0)
; Note: Would need platform-specific syscall for real implementation
define %RuntimeValue @rt_now() {
entry:
  %result1 = insertvalue %RuntimeValue undef, i8 2, 0
  %result2 = insertvalue %RuntimeValue %result1, i64 0, 1
  ret %RuntimeValue %result2
}
"#
    .to_string()
}

fn generate_rt_println() -> String {
    format!(
        r#"
; rt_println: Print a RuntimeValue followed by newline, return nil
define %RuntimeValue @rt_println(%RuntimeValue %val) {{
entry:
  call void @print_value(%RuntimeValue %val)
  %newline_fmt = getelementptr [2 x i8], ptr @fmt_newline, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %newline_fmt)

  ; Return nil
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_print() -> String {
    format!(
        r#"
; rt_print: Print a RuntimeValue without newline, return nil
define %RuntimeValue @rt_print(%RuntimeValue %val) {{
entry:
  call void @print_value(%RuntimeValue %val)

  ; Return nil
  %nil1 = insertvalue %RuntimeValue undef, i8 {TAG_NIL}, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}}
"#
    )
}

fn generate_rt_print_space() -> String {
    r#"
; rt_print_space: Print a space character, return nil
define %RuntimeValue @rt_print_space() {
entry:
  %space_fmt = getelementptr [2 x i8], ptr @fmt_space, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %space_fmt)

  ; Return nil
  %nil1 = insertvalue %RuntimeValue undef, i8 0, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}
"#
    .to_string()
}

fn generate_rt_print_newline() -> String {
    r#"
; rt_print_newline: Print a newline character, return nil
define %RuntimeValue @rt_print_newline() {
entry:
  %newline_fmt = getelementptr [2 x i8], ptr @fmt_newline, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %newline_fmt)

  ; Return nil
  %nil1 = insertvalue %RuntimeValue undef, i8 0, 0
  %nil2 = insertvalue %RuntimeValue %nil1, i64 0, 1
  ret %RuntimeValue %nil2
}
"#
    .to_string()
}

fn generate_print_result() -> String {
    format!(
        r#"
; print_value: Print a RuntimeValue
define void @print_value(%RuntimeValue %val) {{
entry:
  %tag = extractvalue %RuntimeValue %val, 0
  %data = extractvalue %RuntimeValue %val, 1

  switch i8 %tag, label %print_unknown [
    i8 {TAG_NIL}, label %print_nil
    i8 {TAG_BOOL}, label %print_bool
    i8 {TAG_INT}, label %print_int
    i8 {TAG_FLOAT}, label %print_float
    i8 {TAG_CONS}, label %print_cons
    i8 {TAG_STRING}, label %print_string
  ]

print_nil:
  %nil_fmt = getelementptr [4 x i8], ptr @fmt_nil, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %nil_fmt)
  br label %done

print_bool:
  %is_true = icmp ne i64 %data, 0
  br i1 %is_true, label %print_true, label %print_false

print_true:
  %true_fmt = getelementptr [5 x i8], ptr @fmt_true, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %true_fmt)
  br label %done

print_false:
  %false_fmt = getelementptr [6 x i8], ptr @fmt_false, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %false_fmt)
  br label %done

print_int:
  %int_fmt = getelementptr [5 x i8], ptr @fmt_int, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %int_fmt, i64 %data)
  br label %done

print_float:
  %float_val = bitcast i64 %data to double
  %float_fmt = getelementptr [3 x i8], ptr @fmt_float, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %float_fmt, double %float_val)
  br label %done

print_cons:
  ; Print opening paren
  %open_fmt = getelementptr [2 x i8], ptr @fmt_cons_open, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %open_fmt)

  ; Print list elements
  call void @print_list(%RuntimeValue %val)

  ; Print closing paren
  %close_fmt = getelementptr [2 x i8], ptr @fmt_cons_close, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %close_fmt)
  br label %done

print_string:
  ; Get RuntimeString pointer
  %str_ptr = inttoptr i64 %data to ptr
  ; Get data pointer
  %str_data_slot = getelementptr %RuntimeString, ptr %str_ptr, i32 0, i32 0
  %str_data = load ptr, ptr %str_data_slot
  ; Get length
  %str_len_slot = getelementptr %RuntimeString, ptr %str_ptr, i32 0, i32 1
  %str_len = load i64, ptr %str_len_slot
  ; Truncate length to i32 for printf precision
  %str_len_32 = trunc i64 %str_len to i32
  ; Print using %.*s format (precision, pointer)
  %string_fmt = getelementptr [5 x i8], ptr @fmt_string, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %string_fmt, i32 %str_len_32, ptr %str_data)
  br label %done

print_unknown:
  br label %done

done:
  ret void
}}

; print_list: Print elements of a list (without parens)
define void @print_list(%RuntimeValue %val) {{
entry:
  br label %loop

loop:
  %current = phi %RuntimeValue [ %val, %entry ], [ %cdr, %print_space ]
  %first = phi i1 [ true, %entry ], [ false, %print_space ]

  %tag = extractvalue %RuntimeValue %current, 0
  %is_cons = icmp eq i8 %tag, {TAG_CONS}
  br i1 %is_cons, label %process_cons, label %check_nil

process_cons:
  ; Print space if not first element
  br i1 %first, label %get_car, label %print_sep

print_sep:
  %space_fmt = getelementptr [2 x i8], ptr @fmt_space, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %space_fmt)
  br label %get_car

get_car:
  %ptr_int = extractvalue %RuntimeValue %current, 1
  %cell_ptr = inttoptr i64 %ptr_int to ptr
  %car_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 0
  %car = load %RuntimeValue, ptr %car_ptr

  ; Print car
  call void @print_value(%RuntimeValue %car)

  ; Get cdr
  %cdr_ptr = getelementptr %RuntimeConsCell, ptr %cell_ptr, i32 0, i32 1
  %cdr = load %RuntimeValue, ptr %cdr_ptr
  br label %print_space

print_space:
  br label %loop

check_nil:
  %is_nil = icmp eq i8 %tag, {TAG_NIL}
  br i1 %is_nil, label %done, label %print_dot

print_dot:
  ; Improper list - print " . value"
  %dot_fmt = getelementptr [4 x i8], ptr @fmt_dot, i32 0, i32 0
  call i32 (ptr, ...) @printf(ptr %dot_fmt)
  call void @print_value(%RuntimeValue %current)
  br label %done

done:
  ret void
}}
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_runtime_ir() {
        let ir = generate_runtime_ir();

        // Check that all expected definitions are present
        assert!(ir.contains("%RuntimeValue = type"));
        assert!(ir.contains("%RuntimeConsCell = type"));
        assert!(ir.contains("define %RuntimeValue @rt_cons"));
        assert!(ir.contains("define %RuntimeValue @rt_car"));
        assert!(ir.contains("define %RuntimeValue @rt_cdr"));
        assert!(ir.contains("define %RuntimeValue @rt_add"));
        assert!(ir.contains("define %RuntimeValue @rt_sub"));
        assert!(ir.contains("define %RuntimeValue @rt_mul"));
        assert!(ir.contains("define %RuntimeValue @rt_div"));
        assert!(ir.contains("define %RuntimeValue @rt_eq"));
        assert!(ir.contains("define %RuntimeValue @rt_lt"));
        assert!(ir.contains("define %RuntimeValue @rt_is_nil"));
        assert!(ir.contains("define void @print_value"));
    }

    #[test]
    fn test_tag_constants_correct() {
        let ir = generate_runtime_ir();

        // Verify tag values match runtime.rs
        assert!(ir.contains(&format!("@TAG_NIL = private constant i8 {TAG_NIL}")));
        assert!(ir.contains(&format!("@TAG_BOOL = private constant i8 {TAG_BOOL}")));
        assert!(ir.contains(&format!("@TAG_INT = private constant i8 {TAG_INT}")));
        assert!(ir.contains(&format!("@TAG_FLOAT = private constant i8 {TAG_FLOAT}")));
        assert!(ir.contains(&format!("@TAG_CONS = private constant i8 {TAG_CONS}")));
    }
}
