//! ProEdit AI — AI-powered features for video editing.
//!
//! Provides:
//! - Scene/cut detection (fallback: frame differencing, optional: TransNetV2 ONNX)
//! - Audio transcription (whisper.cpp sidecar)
//! - Filler word and silence detection
//! - Frame interpolation for AI slow motion (RIFE via ONNX, requires `onnx` feature)
//! - Model download and cache management

pub mod error;
pub mod filler_detect;
pub mod interpolation;
pub mod model_manager;
pub mod scene_detect;
pub mod session;
pub mod transcribe;

pub use error::{AiError, AiResult};
pub use model_manager::{ModelId, ModelManager};

use std::path::PathBuf;
use tracing::info;

/// Main AI engine — manages models and provides access to AI features.
pub struct AIEngine {
    model_manager: ModelManager,
    initialized: bool,
}

impl AIEngine {
    /// Create a new AI engine with a custom cache directory.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        let cache_dir = cache_dir.into();
        info!(cache_dir = %cache_dir.display(), "AI engine created");
        Self {
            model_manager: ModelManager::new(cache_dir),
            initialized: false,
        }
    }

    /// Default cache directory for AI models.
    pub fn default_cache_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("proedit-studio")
            .join("models")
    }

    /// Get a reference to the model manager.
    pub fn model_manager(&self) -> &ModelManager {
        &self.model_manager
    }

    /// Check if the engine has been initialized.
    pub fn is_initialized(&self) -> bool {
        self.initialized
    }

    /// Mark the engine as initialized.
    pub fn set_initialized(&mut self) {
        self.initialized = true;
    }
}

impl Default for AIEngine {
    fn default() -> Self {
        Self::new(Self::default_cache_dir())
    }
}
