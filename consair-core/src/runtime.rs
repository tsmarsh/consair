//! Runtime value representation for JIT compilation.
//!
//! This module provides a C-compatible value representation that can be used
//! by compiled code to pass values to and from runtime functions.

use std::sync::Arc;
use std::sync::atomic::AtomicU32;

use crate::interner::InternedSymbol;
use crate::language::{AtomType, ConsCell, StringType, SymbolType, Value, VectorValue};
use crate::numeric::NumericType;

// ============================================================================
// Tag Constants
// ============================================================================

/// Tag for nil value
pub const TAG_NIL: u8 = 0;
/// Tag for boolean values
pub const TAG_BOOL: u8 = 1;
/// Tag for integer values (i64)
pub const TAG_INT: u8 = 2;
/// Tag for floating-point values (f64)
pub const TAG_FLOAT: u8 = 3;
/// Tag for cons cell pointers
pub const TAG_CONS: u8 = 4;
/// Tag for interned symbol keys
pub const TAG_SYMBOL: u8 = 5;
/// Tag for closure pointers
pub const TAG_CLOSURE: u8 = 6;
/// Tag for string pointers
pub const TAG_STRING: u8 = 7;
/// Tag for vector pointers
pub const TAG_VECTOR: u8 = 8;

// ============================================================================
// RuntimeValue
// ============================================================================

/// A C-compatible value representation for JIT-compiled code.
///
/// This struct uses a tagged union representation where:
/// - `tag` identifies the type of value
/// - `data` contains either the value directly (for scalars) or a pointer (for heap types)
///
/// The `#[repr(C)]` attribute ensures this has a predictable layout for FFI.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeValue {
    /// Type tag identifying what kind of value this is
    pub tag: u8,
    /// The actual data - either a scalar value or a pointer
    pub data: u64,
}

impl RuntimeValue {
    // ========================================================================
    // Constructors
    // ========================================================================

    /// Create a nil value.
    #[inline]
    pub fn nil() -> Self {
        RuntimeValue {
            tag: TAG_NIL,
            data: 0,
        }
    }

    /// Create a boolean value.
    #[inline]
    pub fn from_bool(b: bool) -> Self {
        RuntimeValue {
            tag: TAG_BOOL,
            data: b as u64,
        }
    }

    /// Create an integer value.
    #[inline]
    pub fn from_int(n: i64) -> Self {
        RuntimeValue {
            tag: TAG_INT,
            data: n as u64,
        }
    }

    /// Create a floating-point value.
    #[inline]
    pub fn from_float(f: f64) -> Self {
        RuntimeValue {
            tag: TAG_FLOAT,
            data: f.to_bits(),
        }
    }

    /// Create a symbol value from an interned symbol key.
    #[inline]
    pub fn from_symbol(key: u64) -> Self {
        RuntimeValue {
            tag: TAG_SYMBOL,
            data: key,
        }
    }

    /// Create a cons cell value from a pointer.
    ///
    /// # Safety
    /// The pointer must point to a valid `RuntimeConsCell`.
    #[inline]
    pub unsafe fn from_cons_ptr(ptr: *mut RuntimeConsCell) -> Self {
        RuntimeValue {
            tag: TAG_CONS,
            data: ptr as u64,
        }
    }

    /// Create a string value from a pointer.
    ///
    /// # Safety
    /// The pointer must point to a valid `RuntimeString`.
    #[inline]
    pub unsafe fn from_string_ptr(ptr: *mut RuntimeString) -> Self {
        RuntimeValue {
            tag: TAG_STRING,
            data: ptr as u64,
        }
    }

    /// Create a vector value from a pointer.
    ///
    /// # Safety
    /// The pointer must point to a valid `RuntimeVector`.
    #[inline]
    pub unsafe fn from_vector_ptr(ptr: *mut RuntimeVector) -> Self {
        RuntimeValue {
            tag: TAG_VECTOR,
            data: ptr as u64,
        }
    }

    /// Create a closure value from a pointer.
    ///
    /// # Safety
    /// The pointer must point to a valid `RuntimeClosure`.
    #[inline]
    pub unsafe fn from_closure_ptr(ptr: *mut RuntimeClosure) -> Self {
        RuntimeValue {
            tag: TAG_CLOSURE,
            data: ptr as u64,
        }
    }

    // ========================================================================
    // Type Predicates
    // ========================================================================

    /// Check if this value is nil.
    #[inline]
    pub fn is_nil(&self) -> bool {
        self.tag == TAG_NIL
    }

    /// Check if this value is a boolean.
    #[inline]
    pub fn is_bool(&self) -> bool {
        self.tag == TAG_BOOL
    }

    /// Check if this value is an integer.
    #[inline]
    pub fn is_int(&self) -> bool {
        self.tag == TAG_INT
    }

    /// Check if this value is a float.
    #[inline]
    pub fn is_float(&self) -> bool {
        self.tag == TAG_FLOAT
    }

    /// Check if this value is a number (int or float).
    #[inline]
    pub fn is_number(&self) -> bool {
        self.tag == TAG_INT || self.tag == TAG_FLOAT
    }

    /// Check if this value is a symbol.
    #[inline]
    pub fn is_symbol(&self) -> bool {
        self.tag == TAG_SYMBOL
    }

    /// Check if this value is a cons cell.
    #[inline]
    pub fn is_cons(&self) -> bool {
        self.tag == TAG_CONS
    }

    /// Check if this value is a string.
    #[inline]
    pub fn is_string(&self) -> bool {
        self.tag == TAG_STRING
    }

    /// Check if this value is a vector.
    #[inline]
    pub fn is_vector(&self) -> bool {
        self.tag == TAG_VECTOR
    }

    /// Check if this value is a closure.
    #[inline]
    pub fn is_closure(&self) -> bool {
        self.tag == TAG_CLOSURE
    }

    /// Check if this value is an atom (not a cons cell).
    #[inline]
    pub fn is_atom(&self) -> bool {
        self.tag != TAG_CONS
    }

    /// Check if this value is truthy (not nil and not false).
    #[inline]
    pub fn is_truthy(&self) -> bool {
        match self.tag {
            TAG_NIL => false,
            TAG_BOOL => self.data != 0,
            _ => true,
        }
    }

    // ========================================================================
    // Accessors
    // ========================================================================

    /// Get the boolean value if this is a bool.
    #[inline]
    pub fn to_bool(&self) -> Option<bool> {
        if self.tag == TAG_BOOL {
            Some(self.data != 0)
        } else {
            None
        }
    }

    /// Get the integer value if this is an int.
    #[inline]
    pub fn to_int(&self) -> Option<i64> {
        if self.tag == TAG_INT {
            Some(self.data as i64)
        } else {
            None
        }
    }

    /// Get the floating-point value if this is a float.
    #[inline]
    pub fn to_float(&self) -> Option<f64> {
        if self.tag == TAG_FLOAT {
            Some(f64::from_bits(self.data))
        } else {
            None
        }
    }

    /// Get the symbol key if this is a symbol.
    #[inline]
    pub fn to_symbol_key(&self) -> Option<u64> {
        if self.tag == TAG_SYMBOL {
            Some(self.data)
        } else {
            None
        }
    }

    /// Get the cons cell pointer if this is a cons.
    ///
    /// # Safety
    /// The caller must ensure the pointer is still valid.
    #[inline]
    pub unsafe fn to_cons_ptr(&self) -> Option<*mut RuntimeConsCell> {
        if self.tag == TAG_CONS {
            Some(self.data as *mut RuntimeConsCell)
        } else {
            None
        }
    }

    /// Get the string pointer if this is a string.
    ///
    /// # Safety
    /// The caller must ensure the pointer is still valid.
    #[inline]
    pub unsafe fn to_string_ptr(&self) -> Option<*mut RuntimeString> {
        if self.tag == TAG_STRING {
            Some(self.data as *mut RuntimeString)
        } else {
            None
        }
    }

    /// Get the vector pointer if this is a vector.
    ///
    /// # Safety
    /// The caller must ensure the pointer is still valid.
    #[inline]
    pub unsafe fn to_vector_ptr(&self) -> Option<*mut RuntimeVector> {
        if self.tag == TAG_VECTOR {
            Some(self.data as *mut RuntimeVector)
        } else {
            None
        }
    }

    /// Get the closure pointer if this is a closure.
    ///
    /// # Safety
    /// The caller must ensure the pointer is still valid.
    #[inline]
    pub unsafe fn to_closure_ptr(&self) -> Option<*mut RuntimeClosure> {
        if self.tag == TAG_CLOSURE {
            Some(self.data as *mut RuntimeClosure)
        } else {
            None
        }
    }

    /// Convert to f64 for numeric operations (works for both int and float).
    #[inline]
    pub fn to_f64(&self) -> Option<f64> {
        match self.tag {
            TAG_INT => Some(self.data as i64 as f64),
            TAG_FLOAT => Some(f64::from_bits(self.data)),
            _ => None,
        }
    }

    // ========================================================================
    // Conversion from interpreter Value
    // ========================================================================

    /// Convert an interpreter Value to a RuntimeValue.
    ///
    /// This allocates heap memory for cons cells, strings, and vectors.
    /// The caller is responsible for managing the memory via reference counting.
    pub fn from_value(v: &Value) -> Result<Self, String> {
        match v {
            Value::Nil => Ok(RuntimeValue::nil()),

            Value::Atom(AtomType::Bool(b)) => Ok(RuntimeValue::from_bool(*b)),

            Value::Atom(AtomType::Number(num)) => match num {
                NumericType::Int(n) => Ok(RuntimeValue::from_int(*n)),
                NumericType::Float(f) => Ok(RuntimeValue::from_float(*f)),
                NumericType::Ratio(num, denom) => {
                    // Convert ratio to float for JIT
                    Ok(RuntimeValue::from_float(*num as f64 / *denom as f64))
                }
                NumericType::BigInt(_) => {
                    Err("JIT does not support BigInt; value too large".to_string())
                }
                NumericType::BigRatio(_) => {
                    Err("JIT does not support BigRatio; value too large".to_string())
                }
            },

            Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))) => {
                // Store the symbol's internal key as u64
                // We use pointer casting to get a stable representation
                // The InternedSymbol is Copy and small, so we store it directly
                let mut key: u64 = 0;
                let sym_bytes = unsafe {
                    std::slice::from_raw_parts(
                        sym as *const InternedSymbol as *const u8,
                        std::mem::size_of::<InternedSymbol>(),
                    )
                };
                // Copy the bytes into key
                for (i, &byte) in sym_bytes.iter().enumerate() {
                    key |= (byte as u64) << (i * 8);
                }
                Ok(RuntimeValue::from_symbol(key))
            }

            Value::Atom(AtomType::String(StringType::Basic(s))) => {
                // Clone the string data and allocate on heap
                let cloned = s.clone().into_bytes();
                let len = cloned.len();
                let ptr = Box::into_raw(cloned.into_boxed_slice()) as *mut u8;
                let rt_string = Box::new(RuntimeString {
                    data: ptr,
                    len: len as u64,
                    refcount: AtomicU32::new(1),
                });
                Ok(unsafe { RuntimeValue::from_string_ptr(Box::into_raw(rt_string)) })
            }

            Value::Cons(cell) => {
                // Recursively convert car and cdr
                let car = RuntimeValue::from_value(&cell.car)?;
                let cdr = RuntimeValue::from_value(&cell.cdr)?;
                let rt_cons = Box::new(RuntimeConsCell {
                    car,
                    cdr,
                    refcount: AtomicU32::new(1),
                });
                Ok(unsafe { RuntimeValue::from_cons_ptr(Box::into_raw(rt_cons)) })
            }

            Value::Vector(vec) => {
                // Convert all elements
                let mut elements: Vec<RuntimeValue> = Vec::with_capacity(vec.elements.len());
                for elem in &vec.elements {
                    elements.push(RuntimeValue::from_value(elem)?);
                }
                let len = elements.len();
                let ptr = Box::into_raw(elements.into_boxed_slice()) as *mut RuntimeValue;
                let rt_vec = Box::new(RuntimeVector {
                    elements: ptr,
                    len: len as u64,
                    refcount: AtomicU32::new(1),
                });
                Ok(unsafe { RuntimeValue::from_vector_ptr(Box::into_raw(rt_vec)) })
            }

            Value::Lambda(_) => {
                // Lambda conversion requires closure support - deferred to Story 18
                Err("JIT lambda conversion not yet implemented".to_string())
            }

            Value::Macro(_) => Err("Macros should be expanded before JIT compilation".to_string()),

            Value::NativeFn(_) => {
                Err("Native functions cannot be converted to RuntimeValue".to_string())
            }
        }
    }

    // ========================================================================
    // Conversion to interpreter Value
    // ========================================================================

    /// Convert a RuntimeValue back to an interpreter Value.
    ///
    /// # Safety
    /// For pointer types (cons, string, vector), this assumes the pointers are valid.
    pub fn to_value(&self) -> Result<Value, String> {
        match self.tag {
            TAG_NIL => Ok(Value::Nil),

            TAG_BOOL => Ok(Value::Atom(AtomType::Bool(self.data != 0))),

            TAG_INT => Ok(Value::Atom(AtomType::Number(NumericType::Int(
                self.data as i64,
            )))),

            TAG_FLOAT => Ok(Value::Atom(AtomType::Number(NumericType::Float(
                f64::from_bits(self.data),
            )))),

            TAG_SYMBOL => {
                // Reconstruct the InternedSymbol from its key
                let mut sym = std::mem::MaybeUninit::<InternedSymbol>::uninit();
                let sym_bytes = unsafe {
                    std::slice::from_raw_parts_mut(
                        sym.as_mut_ptr() as *mut u8,
                        std::mem::size_of::<InternedSymbol>(),
                    )
                };
                // Copy bytes from key
                for (i, byte) in sym_bytes.iter_mut().enumerate() {
                    *byte = ((self.data >> (i * 8)) & 0xFF) as u8;
                }
                let sym = unsafe { sym.assume_init() };
                Ok(Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym))))
            }

            TAG_CONS => {
                let ptr = self.data as *mut RuntimeConsCell;
                if ptr.is_null() {
                    return Err("Null cons cell pointer".to_string());
                }
                unsafe {
                    let cell = &*ptr;
                    let car = cell.car.to_value()?;
                    let cdr = cell.cdr.to_value()?;
                    Ok(Value::Cons(Arc::new(ConsCell { car, cdr })))
                }
            }

            TAG_STRING => {
                let ptr = self.data as *mut RuntimeString;
                if ptr.is_null() {
                    return Err("Null string pointer".to_string());
                }
                unsafe {
                    let rt_string = &*ptr;
                    let slice = std::slice::from_raw_parts(rt_string.data, rt_string.len as usize);
                    let s = String::from_utf8_lossy(slice).into_owned();
                    Ok(Value::Atom(AtomType::String(StringType::Basic(s))))
                }
            }

            TAG_VECTOR => {
                let ptr = self.data as *mut RuntimeVector;
                if ptr.is_null() {
                    return Err("Null vector pointer".to_string());
                }
                unsafe {
                    let rt_vec = &*ptr;
                    let slice = std::slice::from_raw_parts(rt_vec.elements, rt_vec.len as usize);
                    let mut elements = Vec::with_capacity(slice.len());
                    for elem in slice {
                        elements.push(elem.to_value()?);
                    }
                    Ok(Value::Vector(Arc::new(VectorValue { elements })))
                }
            }

            TAG_CLOSURE => {
                // Closure conversion requires additional context - deferred
                Err("Closure to Value conversion not yet implemented".to_string())
            }

            _ => Err(format!("Unknown RuntimeValue tag: {}", self.tag)),
        }
    }
}

impl std::fmt::Debug for RuntimeValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.tag {
            TAG_NIL => write!(f, "RuntimeValue::Nil"),
            TAG_BOOL => write!(f, "RuntimeValue::Bool({})", self.data != 0),
            TAG_INT => write!(f, "RuntimeValue::Int({})", self.data as i64),
            TAG_FLOAT => write!(f, "RuntimeValue::Float({})", f64::from_bits(self.data)),
            TAG_SYMBOL => write!(f, "RuntimeValue::Symbol(key={})", self.data),
            TAG_CONS => write!(f, "RuntimeValue::Cons(ptr={:#x})", self.data),
            TAG_STRING => write!(f, "RuntimeValue::String(ptr={:#x})", self.data),
            TAG_VECTOR => write!(f, "RuntimeValue::Vector(ptr={:#x})", self.data),
            TAG_CLOSURE => write!(f, "RuntimeValue::Closure(ptr={:#x})", self.data),
            _ => write!(
                f,
                "RuntimeValue::Unknown(tag={}, data={})",
                self.tag, self.data
            ),
        }
    }
}

impl PartialEq for RuntimeValue {
    fn eq(&self, other: &Self) -> bool {
        if self.tag != other.tag {
            return false;
        }
        match self.tag {
            TAG_NIL => true,
            TAG_BOOL | TAG_INT | TAG_SYMBOL | TAG_CONS | TAG_STRING | TAG_VECTOR | TAG_CLOSURE => {
                self.data == other.data
            }
            TAG_FLOAT => {
                // Handle float comparison (NaN != NaN)
                let a = f64::from_bits(self.data);
                let b = f64::from_bits(other.data);
                a == b
            }
            _ => false,
        }
    }
}

// ============================================================================
// Runtime Heap Types
// ============================================================================

/// A cons cell allocated on the heap for runtime use.
#[repr(C)]
pub struct RuntimeConsCell {
    pub car: RuntimeValue,
    pub cdr: RuntimeValue,
    pub refcount: AtomicU32,
}

/// A string allocated on the heap for runtime use.
#[repr(C)]
pub struct RuntimeString {
    pub data: *mut u8,
    pub len: u64,
    pub refcount: AtomicU32,
}

/// A vector allocated on the heap for runtime use.
#[repr(C)]
pub struct RuntimeVector {
    pub elements: *mut RuntimeValue,
    pub len: u64,
    pub refcount: AtomicU32,
}

/// A closure allocated on the heap for runtime use.
#[repr(C)]
pub struct RuntimeClosure {
    /// Pointer to the compiled function
    pub fn_ptr: *const (),
    /// Array of captured values (environment)
    pub env: *mut RuntimeValue,
    /// Number of captured values
    pub env_size: u32,
    /// Reference count
    pub refcount: AtomicU32,
}

// ============================================================================
// Runtime FFI Functions
// ============================================================================
//
// These functions are callable from JIT-compiled code via the C ABI.
// They manage cons cell allocation, access, and reference counting.

/// Allocate a new cons cell and return it as a RuntimeValue.
///
/// # Safety
/// This function allocates memory that must be freed via reference counting.
#[unsafe(no_mangle)]
pub extern "C" fn rt_cons(car: RuntimeValue, cdr: RuntimeValue) -> RuntimeValue {
    let cell = Box::new(RuntimeConsCell {
        car,
        cdr,
        refcount: AtomicU32::new(1),
    });
    unsafe { RuntimeValue::from_cons_ptr(Box::into_raw(cell)) }
}

/// Get the car (first element) of a cons cell.
///
/// # Safety
/// The value must be a cons cell. Panics on type error.
#[unsafe(no_mangle)]
pub extern "C" fn rt_car(val: RuntimeValue) -> RuntimeValue {
    if val.tag != TAG_CONS {
        // Type error - panic for now (later: return error value)
        panic!("rt_car: expected cons cell, got tag {}", val.tag);
    }
    let ptr = val.data as *mut RuntimeConsCell;
    if ptr.is_null() {
        panic!("rt_car: null pointer");
    }
    unsafe {
        let cell = &*ptr;
        // Increment refcount on the returned value if it's a heap type
        let result = cell.car;
        rt_incref(result);
        result
    }
}

/// Get the cdr (rest) of a cons cell.
///
/// # Safety
/// The value must be a cons cell. Panics on type error.
#[unsafe(no_mangle)]
pub extern "C" fn rt_cdr(val: RuntimeValue) -> RuntimeValue {
    if val.tag != TAG_CONS {
        // Type error - panic for now (later: return error value)
        panic!("rt_cdr: expected cons cell, got tag {}", val.tag);
    }
    let ptr = val.data as *mut RuntimeConsCell;
    if ptr.is_null() {
        panic!("rt_cdr: null pointer");
    }
    unsafe {
        let cell = &*ptr;
        // Increment refcount on the returned value if it's a heap type
        let result = cell.cdr;
        rt_incref(result);
        result
    }
}

/// Increment the reference count of a heap-allocated value.
///
/// Does nothing for non-heap types (nil, bool, int, float, symbol).
#[unsafe(no_mangle)]
pub extern "C" fn rt_incref(val: RuntimeValue) {
    use std::sync::atomic::Ordering;

    match val.tag {
        TAG_CONS => {
            let ptr = val.data as *mut RuntimeConsCell;
            if !ptr.is_null() {
                unsafe {
                    (*ptr).refcount.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        TAG_STRING => {
            let ptr = val.data as *mut RuntimeString;
            if !ptr.is_null() {
                unsafe {
                    (*ptr).refcount.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        TAG_VECTOR => {
            let ptr = val.data as *mut RuntimeVector;
            if !ptr.is_null() {
                unsafe {
                    (*ptr).refcount.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        TAG_CLOSURE => {
            let ptr = val.data as *mut RuntimeClosure;
            if !ptr.is_null() {
                unsafe {
                    (*ptr).refcount.fetch_add(1, Ordering::Relaxed);
                }
            }
        }
        // Non-heap types: no-op
        _ => {}
    }
}

/// Decrement the reference count of a heap-allocated value.
///
/// Frees the memory when the count reaches zero.
/// Does nothing for non-heap types (nil, bool, int, float, symbol).
#[unsafe(no_mangle)]
pub extern "C" fn rt_decref(val: RuntimeValue) {
    use std::sync::atomic::Ordering;

    match val.tag {
        TAG_CONS => {
            let ptr = val.data as *mut RuntimeConsCell;
            if !ptr.is_null() {
                unsafe {
                    let prev = (*ptr).refcount.fetch_sub(1, Ordering::Release);
                    if prev == 1 {
                        // Memory fence before deallocation
                        std::sync::atomic::fence(Ordering::Acquire);
                        // Recursively decref car and cdr
                        rt_decref((*ptr).car);
                        rt_decref((*ptr).cdr);
                        // Free the cons cell
                        drop(Box::from_raw(ptr));
                    }
                }
            }
        }
        TAG_STRING => {
            let ptr = val.data as *mut RuntimeString;
            if !ptr.is_null() {
                unsafe {
                    let prev = (*ptr).refcount.fetch_sub(1, Ordering::Release);
                    if prev == 1 {
                        std::sync::atomic::fence(Ordering::Acquire);
                        // Free the string data
                        let data_ptr = (*ptr).data;
                        let len = (*ptr).len as usize;
                        if !data_ptr.is_null() {
                            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                data_ptr, len,
                            )));
                        }
                        // Free the RuntimeString
                        drop(Box::from_raw(ptr));
                    }
                }
            }
        }
        TAG_VECTOR => {
            let ptr = val.data as *mut RuntimeVector;
            if !ptr.is_null() {
                unsafe {
                    let prev = (*ptr).refcount.fetch_sub(1, Ordering::Release);
                    if prev == 1 {
                        std::sync::atomic::fence(Ordering::Acquire);
                        // Recursively decref all elements
                        let elements = (*ptr).elements;
                        let len = (*ptr).len as usize;
                        if !elements.is_null() {
                            for i in 0..len {
                                rt_decref(*elements.add(i));
                            }
                            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                elements, len,
                            )));
                        }
                        // Free the RuntimeVector
                        drop(Box::from_raw(ptr));
                    }
                }
            }
        }
        TAG_CLOSURE => {
            let ptr = val.data as *mut RuntimeClosure;
            if !ptr.is_null() {
                unsafe {
                    let prev = (*ptr).refcount.fetch_sub(1, Ordering::Release);
                    if prev == 1 {
                        std::sync::atomic::fence(Ordering::Acquire);
                        // Recursively decref captured values
                        let env = (*ptr).env;
                        let env_size = (*ptr).env_size as usize;
                        if !env.is_null() {
                            for i in 0..env_size {
                                rt_decref(*env.add(i));
                            }
                            drop(Box::from_raw(std::ptr::slice_from_raw_parts_mut(
                                env, env_size,
                            )));
                        }
                        // Free the RuntimeClosure
                        drop(Box::from_raw(ptr));
                    }
                }
            }
        }
        // Non-heap types: no-op
        _ => {}
    }
}

// ============================================================================
// Runtime Arithmetic Functions
// ============================================================================

/// Helper to extract a numeric value for arithmetic.
fn get_numeric(val: RuntimeValue) -> Result<f64, &'static str> {
    match val.tag {
        TAG_INT => Ok(val.data as i64 as f64),
        TAG_FLOAT => Ok(f64::from_bits(val.data)),
        _ => Err("expected number"),
    }
}

/// Helper to create result - returns int if whole, float otherwise.
fn make_numeric_result(val: f64) -> RuntimeValue {
    if val.fract() == 0.0 && val >= i64::MIN as f64 && val <= i64::MAX as f64 {
        RuntimeValue::from_int(val as i64)
    } else {
        RuntimeValue::from_float(val)
    }
}

/// Add two numbers.
#[unsafe(no_mangle)]
pub extern "C" fn rt_add(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(), // Error case
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };

    // If both are ints and no overflow, return int
    if a.tag == TAG_INT && b.tag == TAG_INT {
        let a_int = a.data as i64;
        let b_int = b.data as i64;
        if let Some(result) = a_int.checked_add(b_int) {
            return RuntimeValue::from_int(result);
        }
    }

    make_numeric_result(a_val + b_val)
}

/// Subtract two numbers.
#[unsafe(no_mangle)]
pub extern "C" fn rt_sub(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };

    if a.tag == TAG_INT && b.tag == TAG_INT {
        let a_int = a.data as i64;
        let b_int = b.data as i64;
        if let Some(result) = a_int.checked_sub(b_int) {
            return RuntimeValue::from_int(result);
        }
    }

    make_numeric_result(a_val - b_val)
}

/// Multiply two numbers.
#[unsafe(no_mangle)]
pub extern "C" fn rt_mul(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };

    if a.tag == TAG_INT && b.tag == TAG_INT {
        let a_int = a.data as i64;
        let b_int = b.data as i64;
        if let Some(result) = a_int.checked_mul(b_int) {
            return RuntimeValue::from_int(result);
        }
    }

    make_numeric_result(a_val * b_val)
}

/// Divide two numbers.
#[unsafe(no_mangle)]
pub extern "C" fn rt_div(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::nil(),
    };

    if b_val == 0.0 {
        // Division by zero - return float infinity or NaN
        return RuntimeValue::from_float(a_val / b_val);
    }

    // If both are ints and divides evenly, return int
    if a.tag == TAG_INT && b.tag == TAG_INT {
        let a_int = a.data as i64;
        let b_int = b.data as i64;
        if a_int % b_int == 0 {
            return RuntimeValue::from_int(a_int / b_int);
        }
    }

    RuntimeValue::from_float(a_val / b_val)
}

/// Negate a number.
#[unsafe(no_mangle)]
pub extern "C" fn rt_neg(a: RuntimeValue) -> RuntimeValue {
    match a.tag {
        TAG_INT => {
            let val = a.data as i64;
            if let Some(result) = val.checked_neg() {
                RuntimeValue::from_int(result)
            } else {
                RuntimeValue::from_float(-(val as f64))
            }
        }
        TAG_FLOAT => RuntimeValue::from_float(-f64::from_bits(a.data)),
        _ => RuntimeValue::nil(),
    }
}

// ============================================================================
// Runtime Comparison Functions
// ============================================================================

/// Numeric equality.
#[unsafe(no_mangle)]
pub extern "C" fn rt_num_eq(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    RuntimeValue::from_bool(a_val == b_val)
}

/// Less than comparison.
#[unsafe(no_mangle)]
pub extern "C" fn rt_lt(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    RuntimeValue::from_bool(a_val < b_val)
}

/// Greater than comparison.
#[unsafe(no_mangle)]
pub extern "C" fn rt_gt(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    RuntimeValue::from_bool(a_val > b_val)
}

/// Less than or equal comparison.
#[unsafe(no_mangle)]
pub extern "C" fn rt_lte(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    RuntimeValue::from_bool(a_val <= b_val)
}

/// Greater than or equal comparison.
#[unsafe(no_mangle)]
pub extern "C" fn rt_gte(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    let a_val = match get_numeric(a) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    let b_val = match get_numeric(b) {
        Ok(v) => v,
        Err(_) => return RuntimeValue::from_bool(false),
    };
    RuntimeValue::from_bool(a_val >= b_val)
}

// ============================================================================
// Runtime Type Predicate and Equality Functions
// ============================================================================

/// Check if value is nil.
#[unsafe(no_mangle)]
pub extern "C" fn rt_is_nil(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::from_bool(val.tag == TAG_NIL)
}

/// Check if value is an atom (not a cons cell).
#[unsafe(no_mangle)]
pub extern "C" fn rt_is_atom(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::from_bool(val.tag != TAG_CONS)
}

/// Check if value is a cons cell.
#[unsafe(no_mangle)]
pub extern "C" fn rt_is_cons(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::from_bool(val.tag == TAG_CONS)
}

/// Check if value is a number.
#[unsafe(no_mangle)]
pub extern "C" fn rt_is_number(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::from_bool(val.tag == TAG_INT || val.tag == TAG_FLOAT)
}

/// Atom equality - compares atoms by value, cons cells by identity.
#[unsafe(no_mangle)]
pub extern "C" fn rt_eq(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue {
    if a.tag != b.tag {
        return RuntimeValue::from_bool(false);
    }

    match a.tag {
        TAG_NIL => RuntimeValue::from_bool(true),
        TAG_BOOL | TAG_INT | TAG_SYMBOL => RuntimeValue::from_bool(a.data == b.data),
        TAG_FLOAT => {
            let a_f = f64::from_bits(a.data);
            let b_f = f64::from_bits(b.data);
            RuntimeValue::from_bool(a_f == b_f)
        }
        TAG_CONS | TAG_STRING | TAG_VECTOR | TAG_CLOSURE => {
            // For cons cells, compare by identity (pointer equality)
            RuntimeValue::from_bool(a.data == b.data)
        }
        _ => RuntimeValue::from_bool(false),
    }
}

/// Boolean not.
#[unsafe(no_mangle)]
pub extern "C" fn rt_not(val: RuntimeValue) -> RuntimeValue {
    RuntimeValue::from_bool(!val.is_truthy())
}

// ============================================================================
// Runtime Closure Functions
// ============================================================================

/// Allocate a new closure with a function pointer and captured environment.
///
/// # Arguments
/// * `fn_ptr` - Pointer to the compiled function
/// * `env_values` - Array of captured RuntimeValues
/// * `env_size` - Number of values in the environment
///
/// # Safety
/// The function pointer must be valid and the env_values must point to
/// a valid array of `env_size` RuntimeValues.
#[unsafe(no_mangle)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub extern "C" fn rt_make_closure(
    fn_ptr: *const (),
    env_values: *const RuntimeValue,
    env_size: u32,
) -> RuntimeValue {
    // Copy the environment values
    let mut env: Vec<RuntimeValue> = Vec::with_capacity(env_size as usize);
    if !env_values.is_null() && env_size > 0 {
        unsafe {
            for i in 0..env_size as usize {
                let val = *env_values.add(i);
                // Increment refcount for heap types in the captured environment
                rt_incref(val);
                env.push(val);
            }
        }
    }

    let env_ptr = if env.is_empty() {
        std::ptr::null_mut()
    } else {
        Box::into_raw(env.into_boxed_slice()) as *mut RuntimeValue
    };

    let closure = Box::new(RuntimeClosure {
        fn_ptr,
        env: env_ptr,
        env_size,
        refcount: AtomicU32::new(1),
    });

    unsafe { RuntimeValue::from_closure_ptr(Box::into_raw(closure)) }
}

/// Get the function pointer from a closure.
///
/// # Safety
/// The value must be a closure. Returns null pointer on type error.
#[unsafe(no_mangle)]
pub extern "C" fn rt_closure_fn_ptr(val: RuntimeValue) -> *const () {
    if val.tag != TAG_CLOSURE {
        return std::ptr::null();
    }
    let ptr = val.data as *mut RuntimeClosure;
    if ptr.is_null() {
        return std::ptr::null();
    }
    unsafe { (*ptr).fn_ptr }
}

/// Get a captured value from a closure's environment.
///
/// # Arguments
/// * `val` - The closure value
/// * `index` - Index into the captured environment
///
/// # Safety
/// Returns nil if the value is not a closure or the index is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn rt_closure_env_get(val: RuntimeValue, index: u32) -> RuntimeValue {
    if val.tag != TAG_CLOSURE {
        return RuntimeValue::nil();
    }
    let ptr = val.data as *mut RuntimeClosure;
    if ptr.is_null() {
        return RuntimeValue::nil();
    }
    unsafe {
        let closure = &*ptr;
        if index >= closure.env_size || closure.env.is_null() {
            return RuntimeValue::nil();
        }
        let result = *closure.env.add(index as usize);
        rt_incref(result);
        result
    }
}

/// Get the size of a closure's captured environment.
///
/// # Safety
/// Returns 0 if the value is not a closure.
#[unsafe(no_mangle)]
pub extern "C" fn rt_closure_env_size(val: RuntimeValue) -> u32 {
    if val.tag != TAG_CLOSURE {
        return 0;
    }
    let ptr = val.data as *mut RuntimeClosure;
    if ptr.is_null() {
        return 0;
    }
    unsafe { (*ptr).env_size }
}

// ============================================================================
// Standard Library Runtime Functions
// ============================================================================

/// Get current Unix timestamp (seconds since epoch).
#[unsafe(no_mangle)]
pub extern "C" fn rt_now() -> RuntimeValue {
    use std::time::{SystemTime, UNIX_EPOCH};
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(duration) => RuntimeValue::from_int(duration.as_secs() as i64),
        Err(_) => RuntimeValue::from_int(0),
    }
}

/// Get the length of a list.
/// Returns 0 for non-list values.
#[unsafe(no_mangle)]
pub extern "C" fn rt_length(val: RuntimeValue) -> RuntimeValue {
    let mut count: i64 = 0;
    let mut current = val;

    while current.tag == TAG_CONS {
        count += 1;
        let ptr = current.data as *const RuntimeConsCell;
        if ptr.is_null() {
            break;
        }
        current = unsafe { (*ptr).cdr };
    }

    RuntimeValue::from_int(count)
}

/// Append two lists.
/// (append '(1 2) '(3 4)) => (1 2 3 4)
#[unsafe(no_mangle)]
pub extern "C" fn rt_append(list1: RuntimeValue, list2: RuntimeValue) -> RuntimeValue {
    // If first list is nil, return second list
    if list1.tag == TAG_NIL {
        return list2;
    }

    // If first list is not a cons, return second list
    if list1.tag != TAG_CONS {
        return list2;
    }

    // Collect elements of first list
    let mut elements = Vec::new();
    let mut current = list1;
    while current.tag == TAG_CONS {
        let ptr = current.data as *const RuntimeConsCell;
        if ptr.is_null() {
            break;
        }
        unsafe {
            elements.push((*ptr).car);
            current = (*ptr).cdr;
        }
    }

    // Build result in reverse, starting from list2
    let mut result = list2;
    for elem in elements.into_iter().rev() {
        result = rt_cons(elem, result);
    }

    result
}

/// Reverse a list.
#[unsafe(no_mangle)]
pub extern "C" fn rt_reverse(list: RuntimeValue) -> RuntimeValue {
    let mut result = RuntimeValue::nil();
    let mut current = list;

    while current.tag == TAG_CONS {
        let ptr = current.data as *const RuntimeConsCell;
        if ptr.is_null() {
            break;
        }
        unsafe {
            result = rt_cons((*ptr).car, result);
            current = (*ptr).cdr;
        }
    }

    result
}

/// Get the nth element of a list (0-indexed).
/// Returns nil if index is out of bounds.
#[unsafe(no_mangle)]
pub extern "C" fn rt_nth(list: RuntimeValue, index: RuntimeValue) -> RuntimeValue {
    let n = match index.to_int() {
        Some(i) if i >= 0 => i as usize,
        _ => return RuntimeValue::nil(),
    };

    let mut current = list;
    let mut i = 0;

    while current.tag == TAG_CONS {
        if i == n {
            let ptr = current.data as *const RuntimeConsCell;
            if ptr.is_null() {
                return RuntimeValue::nil();
            }
            return unsafe { (*ptr).car };
        }
        let ptr = current.data as *const RuntimeConsCell;
        if ptr.is_null() {
            break;
        }
        current = unsafe { (*ptr).cdr };
        i += 1;
    }

    RuntimeValue::nil()
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nil() {
        let v = RuntimeValue::nil();
        assert!(v.is_nil());
        assert!(!v.is_truthy());
        assert!(v.is_atom());
    }

    #[test]
    fn test_bool_true() {
        let v = RuntimeValue::from_bool(true);
        assert!(v.is_bool());
        assert_eq!(v.to_bool(), Some(true));
        assert!(v.is_truthy());
        assert!(v.is_atom());
    }

    #[test]
    fn test_bool_false() {
        let v = RuntimeValue::from_bool(false);
        assert!(v.is_bool());
        assert_eq!(v.to_bool(), Some(false));
        assert!(!v.is_truthy());
        assert!(v.is_atom());
    }

    #[test]
    fn test_int_positive() {
        let v = RuntimeValue::from_int(42);
        assert!(v.is_int());
        assert!(v.is_number());
        assert_eq!(v.to_int(), Some(42));
        assert!(v.is_truthy());
        assert!(v.is_atom());
    }

    #[test]
    fn test_int_negative() {
        let v = RuntimeValue::from_int(-123);
        assert!(v.is_int());
        assert_eq!(v.to_int(), Some(-123));
    }

    #[test]
    fn test_int_zero() {
        let v = RuntimeValue::from_int(0);
        assert!(v.is_int());
        assert_eq!(v.to_int(), Some(0));
        assert!(v.is_truthy()); // 0 is truthy in Lisp!
    }

    #[test]
    fn test_int_max() {
        let v = RuntimeValue::from_int(i64::MAX);
        assert_eq!(v.to_int(), Some(i64::MAX));
    }

    #[test]
    fn test_int_min() {
        let v = RuntimeValue::from_int(i64::MIN);
        assert_eq!(v.to_int(), Some(i64::MIN));
    }

    #[test]
    fn test_float_positive() {
        let v = RuntimeValue::from_float(3.15625);
        assert!(v.is_float());
        assert!(v.is_number());
        assert_eq!(v.to_float(), Some(3.15625));
        assert!(v.is_truthy());
        assert!(v.is_atom());
    }

    #[test]
    fn test_float_negative() {
        let v = RuntimeValue::from_float(-2.625);
        assert!(v.is_float());
        assert_eq!(v.to_float(), Some(-2.625));
    }

    #[test]
    fn test_float_zero() {
        let v = RuntimeValue::from_float(0.0);
        assert!(v.is_float());
        assert_eq!(v.to_float(), Some(0.0));
    }

    #[test]
    fn test_float_infinity() {
        let v = RuntimeValue::from_float(f64::INFINITY);
        assert_eq!(v.to_float(), Some(f64::INFINITY));
    }

    #[test]
    fn test_float_neg_infinity() {
        let v = RuntimeValue::from_float(f64::NEG_INFINITY);
        assert_eq!(v.to_float(), Some(f64::NEG_INFINITY));
    }

    #[test]
    fn test_float_nan() {
        let v = RuntimeValue::from_float(f64::NAN);
        assert!(v.is_float());
        // NaN should roundtrip correctly
        assert!(v.to_float().unwrap().is_nan());
    }

    #[test]
    fn test_to_f64_from_int() {
        let v = RuntimeValue::from_int(42);
        assert_eq!(v.to_f64(), Some(42.0));
    }

    #[test]
    fn test_to_f64_from_float() {
        let v = RuntimeValue::from_float(3.125);
        assert_eq!(v.to_f64(), Some(3.125));
    }

    #[test]
    fn test_to_f64_from_nil() {
        let v = RuntimeValue::nil();
        assert_eq!(v.to_f64(), None);
    }

    #[test]
    fn test_symbol() {
        let v = RuntimeValue::from_symbol(12345);
        assert!(v.is_symbol());
        assert_eq!(v.to_symbol_key(), Some(12345));
        assert!(v.is_atom());
    }

    #[test]
    fn test_wrong_accessor() {
        let int_val = RuntimeValue::from_int(42);
        assert_eq!(int_val.to_bool(), None);
        assert_eq!(int_val.to_float(), None);

        let bool_val = RuntimeValue::from_bool(true);
        assert_eq!(bool_val.to_int(), None);
        assert_eq!(bool_val.to_float(), None);
    }

    #[test]
    fn test_equality() {
        assert_eq!(RuntimeValue::nil(), RuntimeValue::nil());
        assert_eq!(RuntimeValue::from_bool(true), RuntimeValue::from_bool(true));
        assert_eq!(RuntimeValue::from_int(42), RuntimeValue::from_int(42));
        assert_eq!(
            RuntimeValue::from_float(3.125),
            RuntimeValue::from_float(3.125)
        );

        assert_ne!(RuntimeValue::nil(), RuntimeValue::from_bool(false));
        assert_ne!(RuntimeValue::from_int(42), RuntimeValue::from_float(42.0));
        assert_ne!(RuntimeValue::from_int(1), RuntimeValue::from_int(2));
    }

    #[test]
    fn test_debug_format() {
        assert!(format!("{:?}", RuntimeValue::nil()).contains("Nil"));
        assert!(format!("{:?}", RuntimeValue::from_bool(true)).contains("Bool(true)"));
        assert!(format!("{:?}", RuntimeValue::from_int(42)).contains("Int(42)"));
        assert!(format!("{:?}", RuntimeValue::from_float(3.125)).contains("Float(3.125)"));
    }

    #[test]
    fn test_struct_size() {
        // RuntimeValue should be 16 bytes (1 byte tag + 7 padding + 8 byte data)
        // or 9 bytes packed. Let's just verify it's reasonable.
        assert!(std::mem::size_of::<RuntimeValue>() <= 16);
    }

    // ========================================================================
    // Conversion Tests
    // ========================================================================

    #[test]
    fn test_convert_nil() {
        let v = Value::Nil;
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert!(rt.is_nil());
        let back = rt.to_value().unwrap();
        assert_eq!(back, Value::Nil);
    }

    #[test]
    fn test_convert_bool_true() {
        let v = Value::Atom(AtomType::Bool(true));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert_eq!(rt.to_bool(), Some(true));
        let back = rt.to_value().unwrap();
        assert_eq!(back, Value::Atom(AtomType::Bool(true)));
    }

    #[test]
    fn test_convert_bool_false() {
        let v = Value::Atom(AtomType::Bool(false));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert_eq!(rt.to_bool(), Some(false));
        let back = rt.to_value().unwrap();
        assert_eq!(back, Value::Atom(AtomType::Bool(false)));
    }

    #[test]
    fn test_convert_int() {
        let v = Value::Atom(AtomType::Number(NumericType::Int(42)));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert_eq!(rt.to_int(), Some(42));
        let back = rt.to_value().unwrap();
        assert_eq!(back, Value::Atom(AtomType::Number(NumericType::Int(42))));
    }

    #[test]
    fn test_convert_int_negative() {
        let v = Value::Atom(AtomType::Number(NumericType::Int(-999)));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert_eq!(rt.to_int(), Some(-999));
        let back = rt.to_value().unwrap();
        assert_eq!(back, Value::Atom(AtomType::Number(NumericType::Int(-999))));
    }

    #[test]
    fn test_convert_float() {
        let v = Value::Atom(AtomType::Number(NumericType::Float(3.15625)));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert_eq!(rt.to_float(), Some(3.15625));
        let back = rt.to_value().unwrap();
        assert_eq!(
            back,
            Value::Atom(AtomType::Number(NumericType::Float(3.15625)))
        );
    }

    #[test]
    fn test_convert_ratio_to_float() {
        // Ratios are converted to floats in JIT
        let v = Value::Atom(AtomType::Number(NumericType::Ratio(1, 2)));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert!(rt.is_float());
        assert_eq!(rt.to_float(), Some(0.5));
    }

    #[test]
    fn test_convert_symbol() {
        let sym = InternedSymbol::new("test-symbol");
        let v = Value::Atom(AtomType::Symbol(SymbolType::Symbol(sym)));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert!(rt.is_symbol());
        let back = rt.to_value().unwrap();
        if let Value::Atom(AtomType::Symbol(SymbolType::Symbol(s))) = back {
            assert_eq!(s.resolve(), "test-symbol");
        } else {
            panic!("Expected symbol");
        }
    }

    #[test]
    fn test_convert_string() {
        let v = Value::Atom(AtomType::String(StringType::Basic(
            "hello world".to_string(),
        )));
        let rt = RuntimeValue::from_value(&v).unwrap();
        assert!(rt.is_string());
        let back = rt.to_value().unwrap();
        assert_eq!(
            back,
            Value::Atom(AtomType::String(StringType::Basic(
                "hello world".to_string()
            )))
        );
    }

    #[test]
    fn test_convert_cons() {
        use crate::language::cons;
        let list = cons(
            Value::Atom(AtomType::Number(NumericType::Int(1))),
            cons(
                Value::Atom(AtomType::Number(NumericType::Int(2))),
                Value::Nil,
            ),
        );
        let rt = RuntimeValue::from_value(&list).unwrap();
        assert!(rt.is_cons());
        let back = rt.to_value().unwrap();
        // Verify it's a list (1 2)
        if let Value::Cons(cell) = &back {
            assert_eq!(cell.car, Value::Atom(AtomType::Number(NumericType::Int(1))));
            if let Value::Cons(cell2) = &cell.cdr {
                assert_eq!(
                    cell2.car,
                    Value::Atom(AtomType::Number(NumericType::Int(2)))
                );
                assert_eq!(cell2.cdr, Value::Nil);
            } else {
                panic!("Expected cons");
            }
        } else {
            panic!("Expected cons");
        }
    }

    #[test]
    fn test_convert_nested_cons() {
        use crate::language::cons;
        // ((1 2) 3)
        let inner = cons(
            Value::Atom(AtomType::Number(NumericType::Int(1))),
            cons(
                Value::Atom(AtomType::Number(NumericType::Int(2))),
                Value::Nil,
            ),
        );
        let outer = cons(
            inner,
            cons(
                Value::Atom(AtomType::Number(NumericType::Int(3))),
                Value::Nil,
            ),
        );
        let rt = RuntimeValue::from_value(&outer).unwrap();
        let back = rt.to_value().unwrap();
        // Should be able to convert back successfully
        assert!(matches!(back, Value::Cons(_)));
    }

    #[test]
    fn test_convert_vector() {
        let vec = Value::Vector(Arc::new(VectorValue {
            elements: vec![
                Value::Atom(AtomType::Number(NumericType::Int(1))),
                Value::Atom(AtomType::Number(NumericType::Int(2))),
                Value::Atom(AtomType::Number(NumericType::Int(3))),
            ],
        }));
        let rt = RuntimeValue::from_value(&vec).unwrap();
        assert!(rt.is_vector());
        let back = rt.to_value().unwrap();
        if let Value::Vector(v) = back {
            assert_eq!(v.elements.len(), 3);
            assert_eq!(
                v.elements[0],
                Value::Atom(AtomType::Number(NumericType::Int(1)))
            );
            assert_eq!(
                v.elements[1],
                Value::Atom(AtomType::Number(NumericType::Int(2)))
            );
            assert_eq!(
                v.elements[2],
                Value::Atom(AtomType::Number(NumericType::Int(3)))
            );
        } else {
            panic!("Expected vector");
        }
    }

    #[test]
    fn test_convert_empty_vector() {
        let vec = Value::Vector(Arc::new(VectorValue { elements: vec![] }));
        let rt = RuntimeValue::from_value(&vec).unwrap();
        let back = rt.to_value().unwrap();
        if let Value::Vector(v) = back {
            assert_eq!(v.elements.len(), 0);
        } else {
            panic!("Expected vector");
        }
    }

    // ========================================================================
    // Runtime FFI Function Tests
    // ========================================================================

    #[test]
    fn test_rt_cons() {
        let car = RuntimeValue::from_int(1);
        let cdr = RuntimeValue::from_int(2);
        let cons = rt_cons(car, cdr);

        assert!(cons.is_cons());

        // Extract and verify
        let car_out = rt_car(cons);
        let cdr_out = rt_cdr(cons);

        assert_eq!(car_out.to_int(), Some(1));
        assert_eq!(cdr_out.to_int(), Some(2));
    }

    #[test]
    fn test_rt_cons_nested() {
        // Build (1 2 3) as (1 . (2 . (3 . nil)))
        let nil = RuntimeValue::nil();
        let three = rt_cons(RuntimeValue::from_int(3), nil);
        let two = rt_cons(RuntimeValue::from_int(2), three);
        let one = rt_cons(RuntimeValue::from_int(1), two);

        assert!(one.is_cons());

        // Traverse the list
        let first = rt_car(one);
        assert_eq!(first.to_int(), Some(1));

        let rest = rt_cdr(one);
        let second = rt_car(rest);
        assert_eq!(second.to_int(), Some(2));

        let rest2 = rt_cdr(rest);
        let third = rt_car(rest2);
        assert_eq!(third.to_int(), Some(3));

        let rest3 = rt_cdr(rest2);
        assert!(rest3.is_nil());
    }

    #[test]
    fn test_rt_refcount() {
        use std::sync::atomic::Ordering;

        let car = RuntimeValue::from_int(42);
        let cdr = RuntimeValue::nil();
        let cons = rt_cons(car, cdr);

        // Initial refcount should be 1
        let ptr = cons.data as *mut RuntimeConsCell;
        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 1);
        }

        // Increment refcount
        rt_incref(cons);
        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 2);
        }

        // Decrement refcount
        rt_decref(cons);
        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 1);
        }

        // Final decref will free - can't check after this
        rt_decref(cons);
    }

    #[test]
    fn test_rt_incref_decref_noop_on_scalars() {
        // These should be no-ops and not crash
        let nil = RuntimeValue::nil();
        let int = RuntimeValue::from_int(42);
        let float = RuntimeValue::from_float(3.125);
        let bool_val = RuntimeValue::from_bool(true);

        rt_incref(nil);
        rt_decref(nil);
        rt_incref(int);
        rt_decref(int);
        rt_incref(float);
        rt_decref(float);
        rt_incref(bool_val);
        rt_decref(bool_val);
    }

    // Note: We can't test panic behavior for extern "C" functions as they can't unwind.
    // Type errors in rt_car/rt_cdr will abort the process.
    // In the future, we should return error values instead of panicking.

    // ========================================================================
    // Arithmetic Function Tests
    // ========================================================================

    #[test]
    fn test_rt_add_ints() {
        let a = RuntimeValue::from_int(2);
        let b = RuntimeValue::from_int(3);
        let result = rt_add(a, b);
        assert_eq!(result.to_int(), Some(5));
    }

    #[test]
    fn test_rt_add_floats() {
        let a = RuntimeValue::from_float(1.5);
        let b = RuntimeValue::from_float(2.5);
        let result = rt_add(a, b);
        // 1.5 + 2.5 = 4.0, which is a whole number so returned as int
        assert_eq!(result.to_int(), Some(4));

        // Test with non-whole result
        let c = RuntimeValue::from_float(1.1);
        let d = RuntimeValue::from_float(2.2);
        let result2 = rt_add(c, d);
        assert!(result2.is_float());
        let val = result2.to_float().unwrap();
        assert!((val - 3.3).abs() < 1e-10);
    }

    #[test]
    fn test_rt_add_mixed() {
        let a = RuntimeValue::from_int(1);
        let b = RuntimeValue::from_float(2.5);
        let result = rt_add(a, b);
        assert_eq!(result.to_float(), Some(3.5));
    }

    #[test]
    fn test_rt_sub() {
        let a = RuntimeValue::from_int(10);
        let b = RuntimeValue::from_int(3);
        let result = rt_sub(a, b);
        assert_eq!(result.to_int(), Some(7));
    }

    #[test]
    fn test_rt_mul() {
        let a = RuntimeValue::from_int(6);
        let b = RuntimeValue::from_int(7);
        let result = rt_mul(a, b);
        assert_eq!(result.to_int(), Some(42));
    }

    #[test]
    fn test_rt_div_exact() {
        let a = RuntimeValue::from_int(10);
        let b = RuntimeValue::from_int(2);
        let result = rt_div(a, b);
        assert_eq!(result.to_int(), Some(5));
    }

    #[test]
    fn test_rt_div_inexact() {
        let a = RuntimeValue::from_int(10);
        let b = RuntimeValue::from_int(3);
        let result = rt_div(a, b);
        // Should return float for inexact division
        assert!(result.is_float());
        let val = result.to_float().unwrap();
        assert!((val - 3.333333333333333).abs() < 1e-10);
    }

    #[test]
    fn test_rt_neg() {
        let a = RuntimeValue::from_int(42);
        let result = rt_neg(a);
        assert_eq!(result.to_int(), Some(-42));

        let b = RuntimeValue::from_float(3.125);
        let result2 = rt_neg(b);
        assert_eq!(result2.to_float(), Some(-3.125));
    }

    // ========================================================================
    // Comparison Function Tests
    // ========================================================================

    #[test]
    fn test_rt_lt() {
        assert_eq!(
            rt_lt(RuntimeValue::from_int(1), RuntimeValue::from_int(2)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_lt(RuntimeValue::from_int(2), RuntimeValue::from_int(1)).to_bool(),
            Some(false)
        );
        assert_eq!(
            rt_lt(RuntimeValue::from_int(1), RuntimeValue::from_int(1)).to_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_rt_gt() {
        assert_eq!(
            rt_gt(RuntimeValue::from_int(2), RuntimeValue::from_int(1)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_gt(RuntimeValue::from_int(1), RuntimeValue::from_int(2)).to_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_rt_lte() {
        assert_eq!(
            rt_lte(RuntimeValue::from_int(1), RuntimeValue::from_int(2)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_lte(RuntimeValue::from_int(1), RuntimeValue::from_int(1)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_lte(RuntimeValue::from_int(2), RuntimeValue::from_int(1)).to_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_rt_gte() {
        assert_eq!(
            rt_gte(RuntimeValue::from_int(2), RuntimeValue::from_int(1)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_gte(RuntimeValue::from_int(1), RuntimeValue::from_int(1)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_gte(RuntimeValue::from_int(1), RuntimeValue::from_int(2)).to_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_rt_num_eq() {
        assert_eq!(
            rt_num_eq(RuntimeValue::from_int(5), RuntimeValue::from_int(5)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_num_eq(RuntimeValue::from_int(5), RuntimeValue::from_int(6)).to_bool(),
            Some(false)
        );
        assert_eq!(
            rt_num_eq(RuntimeValue::from_int(5), RuntimeValue::from_float(5.0)).to_bool(),
            Some(true)
        );
    }

    // ========================================================================
    // Type Predicate Tests
    // ========================================================================

    #[test]
    fn test_rt_is_nil() {
        assert_eq!(rt_is_nil(RuntimeValue::nil()).to_bool(), Some(true));
        assert_eq!(rt_is_nil(RuntimeValue::from_int(0)).to_bool(), Some(false));
    }

    #[test]
    fn test_rt_is_atom() {
        assert_eq!(rt_is_atom(RuntimeValue::nil()).to_bool(), Some(true));
        assert_eq!(rt_is_atom(RuntimeValue::from_int(42)).to_bool(), Some(true));
        assert_eq!(
            rt_is_atom(RuntimeValue::from_symbol(123)).to_bool(),
            Some(true)
        );

        let cons = rt_cons(RuntimeValue::from_int(1), RuntimeValue::nil());
        assert_eq!(rt_is_atom(cons).to_bool(), Some(false));
    }

    #[test]
    fn test_rt_is_cons() {
        let cons = rt_cons(RuntimeValue::from_int(1), RuntimeValue::nil());
        assert_eq!(rt_is_cons(cons).to_bool(), Some(true));
        assert_eq!(rt_is_cons(RuntimeValue::nil()).to_bool(), Some(false));
    }

    #[test]
    fn test_rt_is_number() {
        assert_eq!(
            rt_is_number(RuntimeValue::from_int(42)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_is_number(RuntimeValue::from_float(3.125)).to_bool(),
            Some(true)
        );
        assert_eq!(rt_is_number(RuntimeValue::nil()).to_bool(), Some(false));
        assert_eq!(
            rt_is_number(RuntimeValue::from_bool(true)).to_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_rt_eq() {
        // Same atoms
        assert_eq!(
            rt_eq(RuntimeValue::nil(), RuntimeValue::nil()).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_eq(RuntimeValue::from_int(42), RuntimeValue::from_int(42)).to_bool(),
            Some(true)
        );
        assert_eq!(
            rt_eq(RuntimeValue::from_bool(true), RuntimeValue::from_bool(true)).to_bool(),
            Some(true)
        );

        // Different atoms
        assert_eq!(
            rt_eq(RuntimeValue::from_int(1), RuntimeValue::from_int(2)).to_bool(),
            Some(false)
        );
        assert_eq!(
            rt_eq(RuntimeValue::nil(), RuntimeValue::from_int(0)).to_bool(),
            Some(false)
        );

        // Same cons (by identity)
        let cons = rt_cons(RuntimeValue::from_int(1), RuntimeValue::nil());
        assert_eq!(rt_eq(cons, cons).to_bool(), Some(true));

        // Different cons cells with same values
        let cons1 = rt_cons(RuntimeValue::from_int(1), RuntimeValue::nil());
        let cons2 = rt_cons(RuntimeValue::from_int(1), RuntimeValue::nil());
        assert_eq!(rt_eq(cons1, cons2).to_bool(), Some(false)); // Different pointers
    }

    #[test]
    fn test_rt_not() {
        assert_eq!(rt_not(RuntimeValue::nil()).to_bool(), Some(true));
        assert_eq!(rt_not(RuntimeValue::from_bool(false)).to_bool(), Some(true));
        assert_eq!(rt_not(RuntimeValue::from_bool(true)).to_bool(), Some(false));
        assert_eq!(rt_not(RuntimeValue::from_int(0)).to_bool(), Some(false)); // 0 is truthy in Lisp
        assert_eq!(rt_not(RuntimeValue::from_int(42)).to_bool(), Some(false));
    }

    // ========================================================================
    // Closure Function Tests
    // ========================================================================

    // A dummy function pointer for testing
    extern "C" fn dummy_closure_fn(_env: *const RuntimeValue, _arg: RuntimeValue) -> RuntimeValue {
        RuntimeValue::from_int(42)
    }

    #[test]
    fn test_rt_make_closure_empty_env() {
        let closure = rt_make_closure(dummy_closure_fn as *const (), std::ptr::null(), 0);
        assert!(closure.is_closure());
        assert_eq!(rt_closure_env_size(closure), 0);
        assert!(!rt_closure_fn_ptr(closure).is_null());
        rt_decref(closure);
    }

    #[test]
    fn test_rt_make_closure_with_env() {
        let env_values = [
            RuntimeValue::from_int(10),
            RuntimeValue::from_int(20),
            RuntimeValue::from_int(30),
        ];
        let closure = rt_make_closure(dummy_closure_fn as *const (), env_values.as_ptr(), 3);

        assert!(closure.is_closure());
        assert_eq!(rt_closure_env_size(closure), 3);

        // Check captured values
        let val0 = rt_closure_env_get(closure, 0);
        assert_eq!(val0.to_int(), Some(10));
        rt_decref(val0);

        let val1 = rt_closure_env_get(closure, 1);
        assert_eq!(val1.to_int(), Some(20));
        rt_decref(val1);

        let val2 = rt_closure_env_get(closure, 2);
        assert_eq!(val2.to_int(), Some(30));
        rt_decref(val2);

        rt_decref(closure);
    }

    #[test]
    fn test_rt_closure_env_get_out_of_bounds() {
        let env_values = [RuntimeValue::from_int(42)];
        let closure = rt_make_closure(dummy_closure_fn as *const (), env_values.as_ptr(), 1);

        // Index 0 should work
        let val0 = rt_closure_env_get(closure, 0);
        assert_eq!(val0.to_int(), Some(42));
        rt_decref(val0);

        // Index 1 should return nil (out of bounds)
        let val1 = rt_closure_env_get(closure, 1);
        assert!(val1.is_nil());

        rt_decref(closure);
    }

    #[test]
    fn test_rt_closure_fn_ptr() {
        let closure = rt_make_closure(dummy_closure_fn as *const (), std::ptr::null(), 0);
        let fn_ptr = rt_closure_fn_ptr(closure);
        assert_eq!(fn_ptr, dummy_closure_fn as *const ());
        rt_decref(closure);
    }

    #[test]
    fn test_rt_closure_on_non_closure() {
        // These should return safe defaults
        let int_val = RuntimeValue::from_int(42);
        assert_eq!(rt_closure_env_size(int_val), 0);
        assert!(rt_closure_fn_ptr(int_val).is_null());
        assert!(rt_closure_env_get(int_val, 0).is_nil());
    }

    #[test]
    fn test_rt_closure_refcount() {
        use std::sync::atomic::Ordering;

        let closure = rt_make_closure(dummy_closure_fn as *const (), std::ptr::null(), 0);
        let ptr = closure.data as *mut RuntimeClosure;

        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 1);
        }

        rt_incref(closure);
        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 2);
        }

        rt_decref(closure);
        unsafe {
            assert_eq!((*ptr).refcount.load(Ordering::Relaxed), 1);
        }

        // Final decref frees
        rt_decref(closure);
    }

    #[test]
    fn test_rt_closure_with_heap_values_in_env() {
        // Test that captured heap values have their refcounts managed correctly
        let cons = rt_cons(RuntimeValue::from_int(1), RuntimeValue::from_int(2));
        let cons_ptr = cons.data as *mut RuntimeConsCell;

        unsafe {
            // Initial refcount is 1
            assert_eq!(
                (*cons_ptr)
                    .refcount
                    .load(std::sync::atomic::Ordering::Relaxed),
                1
            );
        }

        let env_values = [cons];
        let closure = rt_make_closure(dummy_closure_fn as *const (), env_values.as_ptr(), 1);

        // After capturing, cons refcount should be 2
        unsafe {
            assert_eq!(
                (*cons_ptr)
                    .refcount
                    .load(std::sync::atomic::Ordering::Relaxed),
                2
            );
        }

        // Clean up - decref the original and the closure
        rt_decref(cons);
        unsafe {
            assert_eq!(
                (*cons_ptr)
                    .refcount
                    .load(std::sync::atomic::Ordering::Relaxed),
                1
            );
        }

        rt_decref(closure);
        // After decref closure, the cons should also be freed (can't check after free)
    }
}
