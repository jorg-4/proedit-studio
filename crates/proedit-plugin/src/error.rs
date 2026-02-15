//! Plugin subsystem errors.

use thiserror::Error;

#[derive(Debug, Error)]
pub enum PluginError {
    #[error("plugin not found: {0}")]
    NotFound(String),
    #[error("OFX error: {0}")]
    Ofx(String),
    #[error("WASM runtime not available")]
    WasmNotAvailable,
    #[error("WASM error: {0}")]
    Wasm(String),
    #[error("invalid plugin: {0}")]
    Invalid(String),
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
