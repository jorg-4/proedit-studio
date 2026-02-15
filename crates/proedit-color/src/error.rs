//! Color subsystem errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ColorError {
    #[error("invalid LUT format: {0}")]
    InvalidLut(String),
    #[error("unsupported color space: {0}")]
    UnsupportedSpace(String),
    #[error("dimension mismatch: expected {expected}, got {got}")]
    DimensionMismatch { expected: usize, got: usize },
    #[error("parse error: {0}")]
    Parse(String),
}
