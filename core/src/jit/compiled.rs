//! Pre-compiled expression handling.

use inkwell::execution_engine::ExecutionEngine;

use crate::runtime::RuntimeValue;

/// Type alias for a compiled expression function.
pub type ExprFn = unsafe extern "C" fn() -> RuntimeValue;

/// A pre-compiled expression that can be executed multiple times efficiently.
///
/// This struct holds the compiled LLVM code and execution engine, allowing
/// the same expression to be executed many times without recompilation.
/// This is useful for benchmarking pure execution speed or when the same
/// expression needs to be evaluated repeatedly.
///
/// # Example
/// ```ignore
/// let engine = JitEngine::new()?;
/// let compiled = engine.compile(&expr)?;
///
/// // Execute multiple times without recompilation
/// for _ in 0..1000 {
///     let result = compiled.execute();
/// }
/// ```
pub struct CompiledExpr<'ctx> {
    /// The execution engine that owns the compiled code
    #[allow(dead_code)]
    pub(crate) execution_engine: ExecutionEngine<'ctx>,
    /// The raw function pointer to the compiled code
    pub(crate) func_ptr: ExprFn,
}

impl<'ctx> CompiledExpr<'ctx> {
    /// Execute the pre-compiled expression.
    ///
    /// This is very fast as no compilation occurs - it just calls the
    /// already-compiled native code.
    #[inline]
    pub fn execute(&self) -> RuntimeValue {
        unsafe { (self.func_ptr)() }
    }
}
