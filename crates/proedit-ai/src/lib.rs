//! ProEdit AI — AI-powered features for video editing.
//!
//! Provides:
//! - Scene/cut detection (fallback: frame differencing, optional: TransNetV2 ONNX)
//! - Audio transcription (whisper.cpp sidecar)
//! - Filler word and silence detection
//! - Frame interpolation for AI slow motion (RIFE via ONNX, requires `onnx` feature)
//! - AI upscaling (Real-ESRGAN via ONNX, requires `onnx` feature)
//! - Auto-rotoscoping / intelligent masking (SAM 2)
//! - Audio stem separation (Demucs v4)
//! - Speaker diarization
//! - Visual content indexing (CLIP embeddings) and natural language search
//! - Audio classification and quality detection
//! - Smart reframing for multi-platform export
//! - Narrative intelligence (cloud, opt-in)
//! - Auto-edit from text prompt (cloud, opt-in)
//! - Style learning (local editor profile)
//! - Background analysis ingest pipeline
//! - Model download and cache management

pub mod analysis_store;
pub mod audio_classify;
pub mod auto_color;
pub mod auto_edit;
pub mod content_index;
pub mod error;
pub mod filler_detect;
pub mod ingest_pipeline;
pub mod interpolation;
pub mod model_manager;
pub mod narrative;
pub mod reframe;
pub mod rotoscope;
pub mod scene_detect;
pub mod session;
pub mod speaker_diarize;
pub mod stem_separation;
pub mod style_learning;
pub mod transcribe;
pub mod upscale;

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
