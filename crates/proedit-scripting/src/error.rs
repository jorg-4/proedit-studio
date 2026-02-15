//! Scripting subsystem errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ExpressionError {
    #[error("parse error: {0}")]
    Parse(String),
    #[error("evaluation error: {0}")]
    Eval(String),
    #[error("type error: expected {expected}, got {got}")]
    Type { expected: String, got: String },
    #[error("undefined variable: {0}")]
    Undefined(String),
    #[error("scripting runtime not available")]
    RuntimeNotAvailable,
}
