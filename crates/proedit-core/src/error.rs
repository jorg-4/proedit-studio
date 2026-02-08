//! Error types for ProEdit.

use thiserror::Error;

/// Main error type for ProEdit operations.
#[derive(Error, Debug)]
pub enum ProEditError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Media error: {0}")]
    Media(String),

    #[error("Decoder error: {0}")]
    Decoder(String),

    #[error("Encoder error: {0}")]
    Encoder(String),

    #[error("GPU error: {0}")]
    Gpu(String),

    #[error("Shader compilation error: {0}")]
    Shader(String),

    #[error("Invalid parameter: {0}")]
    InvalidParameter(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Out of memory: {0}")]
    OutOfMemory(String),

    #[error("Unsupported format: {0}")]
    UnsupportedFormat(String),

    #[error("Timeline error: {0}")]
    Timeline(String),

    #[error("Effect error: {0}")]
    Effect(String),

    #[error("Audio error: {0}")]
    Audio(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

/// Result type alias for ProEdit operations.
pub type Result<T> = std::result::Result<T, ProEditError>;
