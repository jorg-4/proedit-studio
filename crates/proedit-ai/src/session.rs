//! ONNX Runtime session wrapper.
//!
//! Provides a thin wrapper around `ort::Session` with proper error handling.
//! Gated behind the `onnx` feature flag.

#[cfg(feature = "onnx")]
use crate::error::AiResult;
#[cfg(feature = "onnx")]
use crate::model_manager::ModelId;
#[cfg(feature = "onnx")]
use std::path::Path;
#[cfg(feature = "onnx")]
use tracing::info;

/// A loaded ONNX model session.
#[cfg(feature = "onnx")]
pub struct OnnxSession {
    session: ort::Session,
    model_id: ModelId,
}

#[cfg(feature = "onnx")]
impl OnnxSession {
    /// Load an ONNX model from a file path.
    pub fn load(model_path: &Path, model_id: ModelId) -> AiResult<Self> {
        info!(model = ?model_id, path = %model_path.display(), "Loading ONNX session");

        let session = ort::Session::builder()?
            .with_optimization_level(ort::GraphOptimizationLevel::Level3)?
            .commit_from_file(model_path)?;

        info!(model = ?model_id, "ONNX session loaded successfully");
        Ok(Self { session, model_id })
    }

    /// Get a reference to the inner ort::Session.
    pub fn inner(&self) -> &ort::Session {
        &self.session
    }

    /// Get the model ID this session was loaded for.
    pub fn model_id(&self) -> ModelId {
        self.model_id
    }
}
