//! Error types for the AI subsystem.

use thiserror::Error;

/// Errors that can occur in AI operations.
#[derive(Debug, Error)]
pub enum AiError {
    /// The requested model was not found in the cache.
    #[error("Model not found: {model_id}")]
    ModelNotFound { model_id: String },

    /// Model download failed.
    #[error("Model download failed: {url} — {source}")]
    DownloadFailed {
        url: String,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    /// Model checksum mismatch after download.
    #[error("Model checksum mismatch: expected {expected}, got {actual}")]
    ChecksumMismatch { expected: String, actual: String },

    /// ONNX Runtime error.
    #[cfg(feature = "onnx")]
    #[error("ONNX Runtime error: {0}")]
    OnnxError(#[from] ort::Error),

    /// Preprocessing error (frame conversion, format issues, etc.).
    #[error("Preprocessing error: {0}")]
    PreprocessError(String),

    /// IO error.
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Model not loaded — call load_model() first.
    #[error("Model not loaded — call load_model() first")]
    ModelNotLoaded,
}

/// Result type alias for AI operations.
pub type AiResult<T> = std::result::Result<T, AiError>;
