# Epic: JIT Compilation for Consair

## Vision

Transform Consair from a tree-walking interpreter into a JIT-compiled Lisp capable of competing with Julia for scientific computing workloads. Users should experience native code performance while retaining the interactive REPL experience.

## Success Criteria

- All existing tests pass with JIT evaluation
- Benchmark showing ≥10x speedup on recursive fibonacci
- REPL supports seamless switching between interpreted and JIT modes
- No changes required to user-facing Lisp code

---

## Story 1: Add inkwell dependency and verify LLVM toolchain

### Description

As a developer, I need inkwell added to the project so that I can generate LLVM IR from Rust code. This story establishes that the LLVM toolchain builds correctly on all CI platforms.

### Acceptance Criteria

- [ ] `inkwell` added to `consair-core/Cargo.toml` as an optional dependency behind a `jit` feature flag
- [ ] Feature flag defaults to off so existing builds are unaffected
- [ ] CI matrix updated to test both `--features jit` and default features
- [ ] Simple test proves inkwell links correctly:

```rust
#[cfg(feature = "jit")]
#[test]
fn test_inkwell_links() {
    let context = inkwell::context::Context::create();
    let module = context.create_module("test");
    assert_eq!(module.get_name().to_str().unwrap(), "test");
}
```

- [ ] Documentation added explaining LLVM version requirements
- [ ] README updated with `--features jit` build instructions

### Technical Notes

- Use `inkwell = { version = "0.4", features = ["llvm17-0"], optional = true }`
- May need `llvm-sys` build dependencies documented for each platform
- Consider adding a CI job that tests JIT specifically

### Size: Small

---

## Story 2: Implement RuntimeValue representation

### Description

As a compiler developer, I need a C-compatible value representation so that compiled code can pass values to and from runtime functions. This representation must be efficient for numeric operations while supporting all Consair types.

### Acceptance Criteria

- [ ] New file `consair-core/src/runtime.rs` created
- [ ] `RuntimeValue` struct defined with `#[repr(C)]` for C ABI compatibility:

```rust
#[repr(C)]
#[derive(Clone, Copy)]
pub struct RuntimeValue {
    pub tag: u8,
    pub data: u64,
}
```

- [ ] Tag constants defined for all value types:

```rust
pub const TAG_NIL: u8 = 0;
pub const TAG_BOOL: u8 = 1;
pub const TAG_INT: u8 = 2;
pub const TAG_FLOAT: u8 = 3;
pub const TAG_CONS: u8 = 4;
pub const TAG_SYMBOL: u8 = 5;
pub const TAG_CLOSURE: u8 = 6;
pub const TAG_STRING: u8 = 7;
pub const TAG_VECTOR: u8 = 8;
```

- [ ] Constructor functions implemented:
  - `RuntimeValue::nil() -> RuntimeValue`
  - `RuntimeValue::from_bool(b: bool) -> RuntimeValue`
  - `RuntimeValue::from_int(n: i64) -> RuntimeValue`
  - `RuntimeValue::from_float(f: f64) -> RuntimeValue`

- [ ] Accessor functions implemented:
  - `RuntimeValue::is_nil(&self) -> bool`
  - `RuntimeValue::to_bool(&self) -> Option<bool>`
  - `RuntimeValue::to_int(&self) -> Option<i64>`
  - `RuntimeValue::to_float(&self) -> Option<f64>`
  - `RuntimeValue::is_truthy(&self) -> bool`

- [ ] Unit tests verify roundtrip for all scalar types
- [ ] `f64` bit representation preserved exactly through `u64` storage

### Technical Notes

- Use `transmute` or `to_bits`/`from_bits` for float storage
- Pointer types (cons, closure, string) store raw pointer as `u64`
- Consider NaN-boxing as future optimization but don't implement yet

### Size: Small

---

## Story 3: Implement RuntimeValue conversion to/from Value

### Description

As a compiler developer, I need to convert between the interpreter's `Value` type and the compiler's `RuntimeValue` type so that the JIT can interoperate with existing Consair infrastructure.

### Acceptance Criteria

- [ ] `RuntimeValue::from_value(v: &Value) -> RuntimeValue` implemented
- [ ] `RuntimeValue::to_value(&self) -> Value` implemented
- [ ] Conversion handles all Value variants:
  - `Value::Nil`
  - `Value::Atom(AtomType::Bool(_))`
  - `Value::Atom(AtomType::Number(NumericType::Int(_)))`
  - `Value::Atom(AtomType::Number(NumericType::Float(_)))`
  - `Value::Atom(AtomType::Number(NumericType::Ratio(_, _)))` → converts to float
  - `Value::Atom(AtomType::Number(NumericType::BigInt(_)))` → error or special handling
  - `Value::Atom(AtomType::Symbol(_))`
  - `Value::Atom(AtomType::String(_))`
  - `Value::Cons(_)`
  - `Value::Vector(_)`
  - `Value::Lambda(_)` → requires closure representation
  
- [ ] Unit tests verify roundtrip: `Value → RuntimeValue → Value`
- [ ] Tests cover edge cases: empty list, nested cons, large integers

### Technical Notes

- Cons cells need heap allocation; store pointer in `data` field
- Symbols should use existing interner; store `InternedSymbol` key as u64
- BigInt/BigRatio need strategy: either error in JIT mode or box on heap
- Lambda conversion deferred to closure story

### Size: Medium

---

## Story 4: Implement runtime cons cell allocation

### Description

As a compiler developer, I need runtime functions for cons cell operations so that compiled code can construct and deconstruct lists.

### Acceptance Criteria

- [ ] `RuntimeConsCell` struct defined:

```rust
#[repr(C)]
pub struct RuntimeConsCell {
    pub car: RuntimeValue,
    pub cdr: RuntimeValue,
    pub refcount: std::sync::atomic::AtomicU32,
}
```

- [ ] Allocation function implemented:

```rust
#[no_mangle]
pub extern "C" fn rt_cons(car: RuntimeValue, cdr: RuntimeValue) -> RuntimeValue
```

- [ ] Access functions implemented:

```rust
#[no_mangle]
pub extern "C" fn rt_car(val: RuntimeValue) -> RuntimeValue

#[no_mangle]
pub extern "C" fn rt_cdr(val: RuntimeValue) -> RuntimeValue
```

- [ ] Runtime error handling for type mismatches (car/cdr on non-cons)
- [ ] Reference counting: `rt_incref(val: RuntimeValue)` and `rt_decref(val: RuntimeValue)`
- [ ] Tests verify:
  - Create cons, extract car and cdr
  - Nested cons cells
  - Reference counting increments/decrements correctly
  - Memory freed when refcount reaches zero (test with custom allocator or valgrind)

### Technical Notes

- Use `Box::into_raw` for allocation, `Box::from_raw` for deallocation
- `rt_car`/`rt_cdr` should call `rt_incref` on returned value
- Consider using `std::alloc` directly for better control
- Error handling: for now, panic on type error; later, return error value

### Size: Medium

---

## Story 5: Implement runtime arithmetic operations

### Description

As a compiler developer, I need runtime functions for arithmetic so that compiled code can perform numeric operations with proper type handling and promotion.

### Acceptance Criteria

- [ ] Arithmetic functions implemented with `#[no_mangle] extern "C"`:
  - `rt_add(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_sub(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_mul(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_div(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_neg(a: RuntimeValue) -> RuntimeValue`

- [ ] Type promotion rules match interpreter:
  - int + int → int (with overflow check)
  - int + float → float
  - float + float → float
  - Overflow promotes to... (decide: BigInt or float or error)

- [ ] Comparison functions implemented:
  - `rt_eq(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue` (returns bool)
  - `rt_lt(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_gt(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_lte(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - `rt_gte(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`

- [ ] Tests verify:
  - Basic arithmetic: `rt_add(2, 3) == 5`
  - Float promotion: `rt_add(1, 2.5) == 3.5`
  - Comparisons return proper bool RuntimeValues
  - Division by zero handling

### Technical Notes

- Can delegate to existing `NumericType` methods where possible
- For scientific computing, consider always using f64 in hot paths
- Overflow strategy affects later optimization opportunities

### Size: Medium

---

## Story 6: Implement runtime atom and eq operations

### Description

As a compiler developer, I need runtime functions for type predicates and equality so that compiled code can implement conditionals correctly.

### Acceptance Criteria

- [ ] Type predicate functions:
  - `rt_is_nil(val: RuntimeValue) -> RuntimeValue`
  - `rt_is_atom(val: RuntimeValue) -> RuntimeValue`
  - `rt_is_cons(val: RuntimeValue) -> RuntimeValue`
  - `rt_is_number(val: RuntimeValue) -> RuntimeValue`

- [ ] Equality function:
  - `rt_eq(a: RuntimeValue, b: RuntimeValue) -> RuntimeValue`
  - Follows same semantics as interpreter `eq`: atoms compared by value, cons cells by identity

- [ ] Boolean operations:
  - `rt_not(val: RuntimeValue) -> RuntimeValue`

- [ ] Tests verify all predicates and equality semantics

### Size: Small

---

## Story 7: Create codegen module scaffold

### Description

As a compiler developer, I need a codegen module structure so that I can systematically add compilation for each language construct.

### Acceptance Criteria

- [ ] New file `consair-core/src/codegen.rs` created
- [ ] `Codegen` struct defined:

```rust
pub struct Codegen<'ctx> {
    pub context: &'ctx Context,
    pub module: Module<'ctx>,
    pub builder: Builder<'ctx>,
    
    // LLVM type definitions
    value_type: StructType<'ctx>,
    
    // Runtime function declarations
    rt_cons: FunctionValue<'ctx>,
    rt_car: FunctionValue<'ctx>,
    rt_cdr: FunctionValue<'ctx>,
    rt_add: FunctionValue<'ctx>,
    rt_sub: FunctionValue<'ctx>,
    rt_mul: FunctionValue<'ctx>,
    rt_div: FunctionValue<'ctx>,
    rt_eq: FunctionValue<'ctx>,
    rt_lt: FunctionValue<'ctx>,
    rt_is_atom: FunctionValue<'ctx>,
    rt_is_nil: FunctionValue<'ctx>,
}
```

- [ ] Constructor initializes LLVM context, declares runtime functions:

```rust
impl<'ctx> Codegen<'ctx> {
    pub fn new(context: &'ctx Context, module_name: &str) -> Self
}
```

- [ ] Helper method to declare runtime functions with correct signatures
- [ ] `emit_ir(&self) -> String` method for debugging
- [ ] `verify(&self) -> Result<(), String>` method calls LLVM verifier
- [ ] Test verifies module creation and IR emission

### Technical Notes

- RuntimeValue in LLVM is `{ i8, i64 }` struct type
- All runtime functions take and return this struct by value
- Use `module.add_function` with `Linkage::External` for runtime functions

### Size: Medium

---

## Story 8: Compile integer and float literals

### Description

As a compiler developer, I need to compile numeric literals so that expressions like `42` and `3.14` produce correct LLVM IR.

### Acceptance Criteria

- [ ] Method `compile_int_literal(&self, n: i64) -> BasicValueEnum<'ctx>` implemented
- [ ] Method `compile_float_literal(&self, f: f64) -> BasicValueEnum<'ctx>` implemented
- [ ] Method `compile_nil(&self) -> BasicValueEnum<'ctx>` implemented
- [ ] Method `compile_bool(&self, b: bool) -> BasicValueEnum<'ctx>` implemented
- [ ] Generated IR creates RuntimeValue struct with correct tag and data
- [ ] Tests verify IR structure for each literal type
- [ ] Integration test: compile literal, JIT execute, verify result

### Technical Notes

- Use `context.const_struct` to create literal RuntimeValue
- Float bits stored via `f64::to_bits()`
- Example IR output:

```llvm
; RuntimeValue for integer 42
%val = insertvalue { i8, i64 } { i8 2, i64 undef }, i64 42, 1
```

### Size: Small

---

## Story 9: Create JIT execution engine

### Description

As a compiler developer, I need a JIT execution engine so that I can compile and immediately execute Consair expressions.

### Acceptance Criteria

- [ ] New file `consair-core/src/jit.rs` created
- [ ] `JitEngine` struct defined:

```rust
pub struct JitEngine {
    // Owns the execution engine and compiled modules
}
```

- [ ] Method to compile and execute a single expression:

```rust
impl JitEngine {
    pub fn new() -> Result<Self, String>
    pub fn eval(&mut self, expr: &Value) -> Result<RuntimeValue, String>
}
```

- [ ] Execution engine created with optimization level configurable
- [ ] Runtime functions linked correctly (engine can find `rt_add` etc.)
- [ ] Test: `eval(parse("42"))` returns `RuntimeValue::from_int(42)`
- [ ] Test: `eval(parse("3.14"))` returns correct float

### Technical Notes

- Use `module.create_jit_execution_engine(OptimizationLevel::Aggressive)`
- Runtime functions must be linked; use `execution_engine.add_global_mapping`
- Each eval creates a fresh function `__consair_expr_N` to avoid name collisions
- Memory management: execution engine owns modules

### Size: Medium

---

## Story 10: Compile arithmetic expressions

### Description

As a compiler developer, I need to compile arithmetic expressions so that `(+ 1 2)` produces working native code.

### Acceptance Criteria

- [ ] Method `compile_expr(&mut self, expr: &Value) -> BasicValueEnum<'ctx>` implemented
- [ ] Dispatches on expression type:
  - Literals → `compile_*_literal`
  - Cons cell → `compile_application`
  
- [ ] `compile_application` recognizes arithmetic operators:
  - `+` → calls `rt_add`
  - `-` → calls `rt_sub`
  - `*` → calls `rt_mul`
  - `/` → calls `rt_div`

- [ ] Arguments compiled recursively before runtime call
- [ ] Variadic arithmetic works: `(+ 1 2 3 4)` → nested calls
- [ ] Tests:
  - `(+ 1 2)` → 3
  - `(- 10 3)` → 7
  - `(* 6 7)` → 42
  - `(/ 10 2)` → 5
  - `(+ 1 2 3 4 5)` → 15
  - `(+ (* 2 3) (- 10 4))` → 12

### Technical Notes

- Use `builder.build_call` to invoke runtime functions
- Result of call is `CallSiteValue`; extract with `.try_as_basic_value()`
- Variadic: `(+ a b c)` compiles to `rt_add(rt_add(a, b), c)`

### Size: Medium

---

## Story 11: Compile comparison expressions

### Description

As a compiler developer, I need to compile comparison expressions so that conditionals can test numeric relationships.

### Acceptance Criteria

- [ ] Comparison operators compiled:
  - `<` → `rt_lt`
  - `>` → `rt_gt`
  - `<=` → `rt_lte`
  - `>=` → `rt_gte`
  - `=` → `rt_eq` (numeric equality)

- [ ] `eq` (atom equality) compiled → `rt_eq`
- [ ] `atom` predicate compiled → `rt_is_atom`
- [ ] Tests:
  - `(< 1 2)` → true
  - `(> 1 2)` → false
  - `(= 5 5)` → true
  - `(eq 'a 'a)` → true
  - `(atom 'x)` → true
  - `(atom '(1 2))` → false

### Size: Small

---

## Story 12: Compile quote

### Description

As a compiler developer, I need to compile `quote` so that literal data structures can be embedded in compiled code.

### Acceptance Criteria

- [ ] `(quote x)` and `'x` compile correctly
- [ ] Quoted symbols become runtime symbol values
- [ ] Quoted lists become runtime cons structures
- [ ] Quoted data constructed at compile time where possible, or via runtime calls
- [ ] Tests:
  - `'a` → symbol a
  - `'(1 2 3)` → list (1 2 3)
  - `(car '(1 2 3))` → 1
  - `(cdr '(a b))` → (b)

### Technical Notes

- Simple approach: emit runtime calls to construct quoted data each time
- Optimization (later): construct once, store as global constant
- Symbols need runtime representation; use interner key as u64

### Size: Medium

---

## Story 13: Compile cons, car, cdr

### Description

As a compiler developer, I need to compile list operations so that compiled code can construct and deconstruct lists.

### Acceptance Criteria

- [ ] `cons` compiles to `rt_cons` call
- [ ] `car` compiles to `rt_car` call
- [ ] `cdr` compiles to `rt_cdr` call
- [ ] Tests:
  - `(cons 1 2)` → (1 . 2)
  - `(cons 1 '(2 3))` → (1 2 3)
  - `(car (cons 1 2))` → 1
  - `(cdr (cons 1 2))` → 2
  - `(car (cdr '(1 2 3)))` → 2

### Size: Small

---

## Story 14: Compile cond expressions

### Description

As a compiler developer, I need to compile `cond` expressions so that compiled code can perform conditional branching.

### Acceptance Criteria

- [ ] `cond` compiles to LLVM basic blocks with conditional branches
- [ ] Each clause becomes: test condition → branch to result or next clause
- [ ] `t` recognized as always-true
- [ ] `nil` result if no clause matches
- [ ] Tests:
  - `(cond ((< 1 2) 'yes) (t 'no))` → yes
  - `(cond ((> 1 2) 'yes) (t 'no))` → no
  - `(cond ((= 1 2) 'a) ((= 2 2) 'b) (t 'c))` → b
  - `(cond ((= 1 2) 'a))` → nil (no match)

### Technical Notes

- Create basic blocks: `cond_test_0`, `cond_result_0`, `cond_test_1`, ..., `cond_end`
- Use `builder.build_conditional_branch`
- Use phi node or alloca for result value
- Check truthiness: `RuntimeValue.is_truthy()` → compare tag and data

### Size: Medium

---

## Story 15: Compile simple lambda (no captures)

### Description

As a compiler developer, I need to compile lambda expressions that don't capture variables so that simple function definitions work.

### Acceptance Criteria

- [ ] Lambda with no free variables compiles to LLVM function
- [ ] Function takes RuntimeValue arguments, returns RuntimeValue
- [ ] Function application compiles to call instruction
- [ ] Tests:
  - `((lambda (x) x) 42)` → 42
  - `((lambda (x y) (+ x y)) 10 20)` → 30
  - `((lambda (x) (+ x 1)) 5)` → 6
  - `((lambda (a b c) (+ a (+ b c))) 1 2 3)` → 6

### Technical Notes

- Generate unique function name: `__lambda_0`, `__lambda_1`, etc.
- Parameter access: `function.get_nth_param(i)`
- Store parameter → local variable mapping during body compilation
- Application: compile operator, compile args, emit call

### Size: Medium

---

## Story 16: Implement symbol environment for JIT

### Description

As a compiler developer, I need a compile-time environment so that variable references resolve correctly and `label` definitions persist across expressions.

### Acceptance Criteria

- [ ] `JitEnvironment` struct tracks:
  - Global bindings (from `label`)
  - Local bindings (lambda parameters)
  - Scope stack for nested lambdas

- [ ] Variable reference compiles to:
  - Local parameter access, OR
  - Global function call, OR
  - Runtime environment lookup (for closures - later)

- [ ] `label` adds binding to global environment
- [ ] Tests:
  - `(label x 42) x` → 42
  - `(label f (lambda (x) x)) (f 5)` → 5
  - `(label double (lambda (x) (+ x x))) (double 21)` → 42

### Technical Notes

- Globals can be LLVM global variables or functions
- For simple values, use global constant
- For functions, store function pointer

### Size: Medium

---

## Story 17: Compile recursive functions

### Description

As a compiler developer, I need recursive function calls to work so that functions like factorial can be compiled.

### Acceptance Criteria

- [ ] Function can call itself by name
- [ ] `label` creates forward declaration before compiling body
- [ ] Tests:
  - Factorial:
    ```lisp
    (label factorial (lambda (n)
      (cond ((= n 0) 1)
            (t (* n (factorial (- n 1)))))))
    (factorial 5)
    ```
    → 120
    
  - Fibonacci:
    ```lisp
    (label fib (lambda (n)
      (cond ((= n 0) 0)
            ((= n 1) 1)
            (t (+ (fib (- n 1)) (fib (- n 2)))))))
    (fib 10)
    ```
    → 55

### Technical Notes

- Declare function before defining body
- Self-reference resolves to the declared function
- Tail call optimization (later story)

### Size: Medium

---

## Story 18: Implement closure representation

### Description

As a compiler developer, I need a closure representation so that lambdas can capture variables from enclosing scopes.

### Acceptance Criteria

- [ ] `RuntimeClosure` struct defined:

```rust
#[repr(C)]
pub struct RuntimeClosure {
    pub fn_ptr: *const (),       // Pointer to compiled function
    pub env: *mut RuntimeValue,  // Array of captured values
    pub env_size: u32,
    pub refcount: AtomicU32,
}
```

- [ ] Runtime functions:
  - `rt_make_closure(fn_ptr, env_ptr, env_size) -> RuntimeValue`
  - `rt_closure_fn_ptr(closure: RuntimeValue) -> *const ()`
  - `rt_closure_env(closure: RuntimeValue) -> *mut RuntimeValue`

- [ ] Closure calling convention: first argument is environment pointer
- [ ] Tests verify closure allocation and field access

### Technical Notes

- Compiled closure function signature: `(env: *mut RuntimeValue, arg0: RuntimeValue, ...) -> RuntimeValue`
- Capture by value (clone RuntimeValue into env array)
- Reference counting for closure memory management

### Size: Medium

---

## Story 19: Compile closures (lambdas with captures)

### Description

As a compiler developer, I need to compile lambdas that capture variables so that higher-order functions work correctly.

### Acceptance Criteria

- [ ] Free variable analysis identifies captured variables
- [ ] Lambda compilation:
  1. Identify free variables
  2. Generate function with env parameter
  3. Access captured vars via env pointer
  4. Create closure struct at definition site

- [ ] Tests:
  - Basic capture:
    ```lisp
    (label make-adder (lambda (x) (lambda (y) (+ x y))))
    (label add-10 (make-adder 10))
    (add-10 5)
    ```
    → 15
    
  - Multiple captures:
    ```lisp
    (label make-linear (lambda (m b) (lambda (x) (+ (* m x) b))))
    (label f (make-linear 2 3))
    (f 5)
    ```
    → 13
    
  - Nested closures:
    ```lisp
    (label make-counter (lambda (start)
      (lambda (increment)
        (lambda () (+ start increment)))))
    (label counter ((make-counter 10) 5))
    (counter)
    ```
    → 15

### Technical Notes

- Free variable analysis: walk body, collect symbols not in parameter list or globals
- Env layout: captured vars in fixed order
- Closure application: extract fn_ptr and env, call with env as first arg

### Size: Large

---

## Story 20: Implement tail call optimization

### Description

As a compiler developer, I need tail call optimization so that recursive functions don't overflow the stack.

### Acceptance Criteria

- [ ] Tail position analysis identifies tail calls
- [ ] Tail calls use `musttail` LLVM attribute
- [ ] Self-recursive tail calls can use loop optimization
- [ ] Tests:
  - Deep recursion without stack overflow:
    ```lisp
    (label countdown (lambda (n)
      (cond ((= n 0) 0)
            (t (countdown (- n 1))))))
    (countdown 100000)
    ```
    → 0 (no stack overflow)
    
  - Tail-recursive accumulator:
    ```lisp
    (label sum-to (lambda (n acc)
      (cond ((= n 0) acc)
            (t (sum-to (- n 1) (+ n acc))))))
    (sum-to 10000 0)
    ```
    → 50005000

### Technical Notes

- LLVM `musttail` requires matching signatures
- Alternative: detect self-tail-calls, compile as loop
- Non-self tail calls harder; may need trampoline

### Size: Medium

---

## Story 21: Compile standard library functions

### Description

As a compiler developer, I need standard library functions available to compiled code so that I/O and other builtins work.

### Acceptance Criteria

- [ ] Runtime wrappers for stdlib:
  - `rt_print(val: RuntimeValue) -> RuntimeValue`
  - `rt_println(val: RuntimeValue) -> RuntimeValue`
  - `rt_slurp(path: RuntimeValue) -> RuntimeValue`
  - `rt_spit(path: RuntimeValue, content: RuntimeValue) -> RuntimeValue`
  - `rt_now() -> RuntimeValue`
  - `rt_gensym() -> RuntimeValue`

- [ ] Stdlib functions registered in JitEnvironment as globals
- [ ] Tests:
  - `(println 42)` prints and returns nil
  - `(now)` returns timestamp

### Technical Notes

- Wrappers convert RuntimeValue ↔ Rust types, call existing stdlib
- String handling needs RuntimeValue string representation

### Size: Medium

---

## Story 22: Compile vector operations

### Description

As a compiler developer, I need vector operations compiled so that the vector data type works in JIT mode.

### Acceptance Criteria

- [ ] Runtime functions:
  - `rt_vector(elements: *mut RuntimeValue, len: u32) -> RuntimeValue`
  - `rt_vector_length(vec: RuntimeValue) -> RuntimeValue`
  - `rt_vector_ref(vec: RuntimeValue, idx: RuntimeValue) -> RuntimeValue`

- [ ] `(vector ...)` syntax compiles correctly
- [ ] Tests:
  - `(vector 1 2 3)` → <<1 2 3>>
  - `(vector-length (vector 1 2 3 4 5))` → 5
  - `(vector-ref (vector 10 20 30) 1)` → 20

### Size: Medium

---

## Story 23: Compile macro expansion

### Description

As a compiler developer, I need macros to expand before compilation so that user-defined macros work in JIT mode.

### Acceptance Criteria

- [ ] Macro expansion happens before codegen
- [ ] `defmacro` registers macro in environment
- [ ] Macro application expands at compile time
- [ ] Tests:
  - `when` macro works:
    ```lisp
    (defmacro when (condition body) `(cond (,condition ,body) (t nil)))
    (when t 42)
    ```
    → 42

### Technical Notes

- Reuse existing macro expansion from interpreter
- Expand fully, then compile expanded form
- Macros don't need runtime representation

### Size: Small

---

## Story 24: Add JIT mode to REPL

### Description

As a user, I want to use JIT compilation in the REPL so that I get fast execution while maintaining interactivity.

### Acceptance Criteria

- [ ] REPL command `:jit on` enables JIT mode
- [ ] REPL command `:jit off` returns to interpreted mode
- [ ] Command-line flag `--jit` starts in JIT mode
- [ ] Status indicator shows current mode in prompt:
  - `consair>` (interpreted)
  - `consair[jit]>` (compiled)

- [ ] All existing REPL features work in JIT mode
- [ ] Graceful fallback: if JIT fails, show error and continue in interpreted mode

### Size: Small

---

## Story 25: JIT compilation caching

### Description

As a user, I want compiled functions cached so that I don't pay compilation cost on every call.

### Acceptance Criteria

- [ ] Functions compiled once, reused on subsequent calls
- [ ] `label` definitions persist in JIT module
- [ ] Cache invalidation if function redefined
- [ ] Benchmark shows second call faster than first

### Technical Notes

- Keep functions in persistent LLVM module
- Track function name → FunctionValue mapping
- Redefinition: remove old function, add new

### Size: Medium

---

## Story 26: Benchmark suite comparing interpreted vs JIT

### Description

As a developer, I want benchmarks comparing interpreted and JIT performance so that I can quantify improvements and catch regressions.

### Acceptance Criteria

- [ ] New benchmark file `consair-core/benches/jit_benchmarks.rs`
- [ ] Benchmarks for:
  - Arithmetic: `(+ 1 2 3 4 5 6 7 8 9 10)` 
  - Factorial(10)
  - Fibonacci(20)
  - List operations: build and traverse 1000-element list
  - Closure creation and invocation

- [ ] Each benchmark has interpreted and JIT variant
- [ ] Results added to BENCHMARK_ANALYSIS.md
- [ ] CI runs benchmarks and detects regressions

### Size: Medium

---

## Story 27: Documentation and examples

### Description

As a user, I want documentation explaining JIT compilation so that I understand how to use it and what to expect.

### Acceptance Criteria

- [ ] README updated with JIT section:
  - How to enable
  - Performance expectations
  - Limitations

- [ ] New example file `examples/jit_demo.lisp` showcasing JIT benefits
- [ ] REPL `:help` updated with JIT commands
- [ ] Doc comments on public JIT API

### Size: Small

---

## Story 28: Error handling and diagnostics

### Description

As a user, I want clear error messages when JIT compilation fails so that I can understand and fix problems.

### Acceptance Criteria

- [ ] Compilation errors include source location
- [ ] Unsupported features give clear message: "JIT does not yet support X, falling back to interpreter"
- [ ] LLVM verification errors translated to user-friendly messages
- [ ] Option to dump generated IR for debugging: `:dump-ir (+ 1 2)`

### Size: Small

---

## Dependency Graph

```
1 (inkwell) 
    ↓
2 (RuntimeValue) → 3 (Value conversion)
    ↓
4 (cons/car/cdr) → 5 (arithmetic) → 6 (predicates)
    ↓
7 (codegen scaffold)
    ↓
8 (literals) → 9 (JIT engine)
    ↓
10 (arithmetic exprs) → 11 (comparisons)
    ↓
12 (quote) → 13 (cons/car/cdr exprs)
    ↓
14 (cond)
    ↓
15 (simple lambda) → 16 (environment) → 17 (recursion)
    ↓
18 (closure repr) → 19 (closures) → 20 (TCO)
    ↓
21 (stdlib) → 22 (vectors) → 23 (macros)
    ↓
24 (REPL) → 25 (caching) → 26 (benchmarks) → 27 (docs) → 28 (errors)
```

---

## Milestones

### Milestone 1: Proof of Concept (Stories 1-9)
- JIT can compile and execute numeric literals
- Foundation proven, no user-facing changes yet

### Milestone 2: Arithmetic (Stories 10-14)
- JIT can compile arithmetic and conditionals
- Feature-flag release: `--features jit` for early adopters

### Milestone 3: Functions (Stories 15-20)
- Full function support including closures and TCO
- JIT passes all interpreter tests
- Beta release

### Milestone 4: Complete (Stories 21-28)
- Full feature parity with interpreter
- Documentation and benchmarks
- Stable release

---

## Risks

| Risk | Mitigation |
|------|------------|
| LLVM version compatibility | Pin to specific LLVM version, test in CI |
| Closure complexity | Start with non-capturing lambdas, iterate |
| Memory leaks in runtime | Extensive testing, valgrind in CI |
| Platform differences | CI matrix covers Linux, macOS, Windows |
| Performance not meeting goals | Benchmark early and often |

---

## Open Questions

1. **BigInt handling**: Error in JIT mode, or implement heap-boxed BigInt?
2. **Ratio handling**: Convert to float, or implement rational arithmetic in runtime?
3. **Error recovery**: Panic in runtime functions, or return error value?
4. **Garbage collection**: Start with refcounting, add tracing GC later?

---

## Notes for AI Assistants

When implementing these stories:

1. **Always write tests first** - each story has specific test cases
2. **Keep existing interpreter working** - JIT is additive, not replacement
3. **Use feature flags** - `#[cfg(feature = "jit")]` guards all new code
4. **Follow existing code style** - match consair-core patterns
5. **Document public APIs** - rustdoc on all public items
6. **Prefer simple over clever** - optimize later with benchmarks to guide
