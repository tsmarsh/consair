//! JIT error types and handling.

use std::fmt;

use crate::language::Value;

/// Categories of JIT compilation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum JitErrorKind {
    /// Expression type not yet supported by the JIT compiler
    UnsupportedExpression,
    /// Data type not supported (e.g., BigInt, BigRatio)
    UnsupportedType,
    /// Malformed expression structure
    InvalidSyntax,
    /// Reference to undefined variable
    UnboundVariable,
    /// LLVM compilation failure
    CompilationError,
    /// Runtime execution failure
    ExecutionError,
}

/// A JIT compilation or execution error with context.
#[derive(Debug, Clone)]
pub struct JitError {
    /// The category of error
    pub kind: JitErrorKind,
    /// Human-readable error message
    pub message: String,
    /// The expression that caused the error (if available)
    pub expression: Option<String>,
    /// Suggestion for how to fix or work around the error
    pub suggestion: Option<String>,
}

impl JitError {
    /// Create a new JIT error.
    pub fn new(kind: JitErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            expression: None,
            suggestion: None,
        }
    }

    /// Add expression context to the error.
    pub fn with_expression(mut self, expr: &Value) -> Self {
        self.expression = Some(format!("{}", expr));
        self
    }

    /// Add a suggestion for fixing the error.
    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    /// Create an unsupported expression error.
    pub fn unsupported(what: impl Into<String>) -> Self {
        Self::new(JitErrorKind::UnsupportedExpression, what)
    }

    /// Create an unsupported type error.
    pub fn unsupported_type(what: impl Into<String>) -> Self {
        Self::new(JitErrorKind::UnsupportedType, what)
    }

    /// Create an invalid syntax error.
    pub fn syntax(what: impl Into<String>) -> Self {
        Self::new(JitErrorKind::InvalidSyntax, what)
    }

    /// Create an unbound variable error.
    pub fn unbound(name: impl Into<String>) -> Self {
        Self::new(
            JitErrorKind::UnboundVariable,
            format!("Unbound symbol: {}", name.into()),
        )
    }

    /// Create a compilation error.
    pub fn compilation(what: impl Into<String>) -> Self {
        Self::new(JitErrorKind::CompilationError, what)
    }

    /// Create an execution error.
    pub fn execution(what: impl Into<String>) -> Self {
        Self::new(JitErrorKind::ExecutionError, what)
    }
}

impl fmt::Display for JitError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)?;
        if let Some(ref expr) = self.expression {
            // Truncate long expressions
            let truncated = if expr.len() > 60 {
                format!("{}...", &expr[..57])
            } else {
                expr.clone()
            };
            write!(f, " in: {}", truncated)?;
        }
        if let Some(ref suggestion) = self.suggestion {
            write!(f, " ({})", suggestion)?;
        }
        Ok(())
    }
}

impl std::error::Error for JitError {}

impl From<JitError> for String {
    fn from(err: JitError) -> String {
        err.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jit_error_display() {
        let err = JitError::unsupported("test feature");
        assert_eq!(err.to_string(), "test feature");
    }

    #[test]
    fn test_jit_error_with_suggestion() {
        let err = JitError::unsupported("BigInt").with_suggestion("use integers within i64 range");
        assert!(err.to_string().contains("use integers"));
    }

    #[test]
    fn test_jit_error_kind() {
        let err = JitError::unbound("x");
        assert_eq!(err.kind, JitErrorKind::UnboundVariable);
    }

    #[test]
    fn test_jit_error_into_string() {
        let err = JitError::syntax("missing parenthesis");
        let s: String = err.into();
        assert!(s.contains("missing parenthesis"));
    }

    #[test]
    fn test_jit_error_unsupported() {
        let err = JitError::unsupported("macros");
        assert_eq!(err.kind, JitErrorKind::UnsupportedExpression);
        assert_eq!(err.message, "macros");
    }

    #[test]
    fn test_jit_error_unsupported_type() {
        let err = JitError::unsupported_type("BigInt");
        assert_eq!(err.kind, JitErrorKind::UnsupportedType);
    }

    #[test]
    fn test_jit_error_compilation() {
        let err = JitError::compilation("LLVM verification failed");
        assert_eq!(err.kind, JitErrorKind::CompilationError);
    }

    #[test]
    fn test_jit_error_execution() {
        let err = JitError::execution("segmentation fault");
        assert_eq!(err.kind, JitErrorKind::ExecutionError);
    }
}
