//! Model download and cache manager.
//!
//! Manages ONNX model files: checking cache, downloading, and verification.

use crate::error::{AiError, AiResult};
use std::path::{Path, PathBuf};
use tracing::info;

/// Specification for an AI model.
pub struct ModelSpec {
    /// Unique identifier.
    pub id: ModelId,
    /// Filename in cache directory.
    pub filename: &'static str,
    /// Download URL (placeholder until models are hosted).
    pub url: &'static str,
    /// Expected SHA-256 hash (placeholder until models are hosted).
    pub sha256: &'static str,
    /// Expected file size in bytes.
    pub size_bytes: u64,
}

/// Identifies a specific AI model.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ModelId {
    /// TransNetV2 scene detection model.
    TransNetV2,
    /// Whisper large-v3-turbo speech recognition.
    Whisper3Turbo,
    /// SAM 2 ViT-B segmentation model.
    SAM2ViTB,
    /// RIFE v4 frame interpolation model.
    RIFEv4,
    /// Real-ESRGAN 4x upscaling model.
    RealESRGAN4x,
    /// Demucs v4 stem separation model.
    DemucsV4,
    /// Speaker diarization model.
    SpeakerDiarize,
    /// CLIP ViT-B/32 embedding model.
    CLIPViTB32,
}

impl ModelId {
    /// Get the specification for this model.
    pub fn spec(&self) -> ModelSpec {
        match self {
            Self::TransNetV2 => ModelSpec {
                id: *self,
                filename: "transnetv2.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/transnetv2.onnx",
                sha256: "placeholder_hash",
                size_bytes: 4_000_000,
            },
            Self::Whisper3Turbo => ModelSpec {
                id: *self,
                filename: "whisper-large-v3-turbo.bin",
                url:
                    "https://huggingface.co/proedit/models/resolve/main/whisper-large-v3-turbo.bin",
                sha256: "placeholder_hash",
                size_bytes: 750_000_000,
            },
            Self::SAM2ViTB => ModelSpec {
                id: *self,
                filename: "sam2_vit_b.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/sam2_vit_b.onnx",
                sha256: "placeholder_hash",
                size_bytes: 350_000_000,
            },
            Self::RIFEv4 => ModelSpec {
                id: *self,
                filename: "rife_v4.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/rife_v4.onnx",
                sha256: "placeholder_hash",
                size_bytes: 120_000_000,
            },
            Self::RealESRGAN4x => ModelSpec {
                id: *self,
                filename: "realesrgan_4x.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/realesrgan_4x.onnx",
                sha256: "placeholder_hash",
                size_bytes: 64_000_000,
            },
            Self::DemucsV4 => ModelSpec {
                id: *self,
                filename: "demucs_v4.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/demucs_v4.onnx",
                sha256: "placeholder_hash",
                size_bytes: 300_000_000,
            },
            Self::SpeakerDiarize => ModelSpec {
                id: *self,
                filename: "speaker_diarize.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/speaker_diarize.onnx",
                sha256: "placeholder_hash",
                size_bytes: 150_000_000,
            },
            Self::CLIPViTB32 => ModelSpec {
                id: *self,
                filename: "clip_vit_b32.onnx",
                url: "https://huggingface.co/proedit/models/resolve/main/clip_vit_b32.onnx",
                sha256: "placeholder_hash",
                size_bytes: 400_000_000,
            },
        }
    }

    /// Human-readable model size.
    pub fn size_human(&self) -> &'static str {
        match self {
            Self::TransNetV2 => "4 MB",
            Self::Whisper3Turbo => "750 MB",
            Self::SAM2ViTB => "350 MB",
            Self::RIFEv4 => "120 MB",
            Self::RealESRGAN4x => "64 MB",
            Self::DemucsV4 => "300 MB",
            Self::SpeakerDiarize => "150 MB",
            Self::CLIPViTB32 => "400 MB",
        }
    }
}

/// Manages AI model downloads and caching.
pub struct ModelManager {
    cache_dir: PathBuf,
}

impl ModelManager {
    /// Create a new model manager with the given cache directory.
    pub fn new(cache_dir: impl Into<PathBuf>) -> Self {
        Self {
            cache_dir: cache_dir.into(),
        }
    }

    /// Returns path to cached model. Returns error if not yet downloaded.
    pub async fn ensure_model(&self, model: ModelId) -> AiResult<PathBuf> {
        let spec = model.spec();
        let local_path = self.cache_dir.join(spec.filename);

        if local_path.exists() {
            info!(model = ?model, path = %local_path.display(), "Model already cached");
            return Ok(local_path);
        }

        // Create cache dir if needed
        std::fs::create_dir_all(&self.cache_dir)?;

        info!(model = ?model, url = spec.url, "Model not cached — download required");
        // Real download will be implemented when model hosting is set up.
        // For now, return an error indicating the model needs to be manually placed.
        Err(AiError::ModelNotFound {
            model_id: format!("{:?}", model),
        })
    }

    /// Check if a model is already cached locally.
    pub fn is_cached(&self, model: ModelId) -> bool {
        let spec = model.spec();
        self.cache_dir.join(spec.filename).exists()
    }

    /// Get the local path for a model (may not exist yet).
    pub fn model_path(&self, model: ModelId) -> PathBuf {
        let spec = model.spec();
        self.cache_dir.join(spec.filename)
    }

    /// Get the cache directory path.
    pub fn cache_dir(&self) -> &Path {
        &self.cache_dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_model_specs_all_defined() {
        let models = [
            ModelId::TransNetV2,
            ModelId::Whisper3Turbo,
            ModelId::SAM2ViTB,
            ModelId::RIFEv4,
            ModelId::RealESRGAN4x,
            ModelId::DemucsV4,
            ModelId::SpeakerDiarize,
            ModelId::CLIPViTB32,
        ];
        for model in models {
            let spec = model.spec();
            assert!(!spec.filename.is_empty());
            assert!(!spec.url.is_empty());
            assert!(spec.size_bytes > 0);
        }
    }

    #[test]
    fn test_model_manager_cache_dir() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let mgr = ModelManager::new(tmp.path());
        assert!(!mgr.is_cached(ModelId::TransNetV2));
    }

    #[test]
    fn test_model_path_construction() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let mgr = ModelManager::new(tmp.path());
        let path = mgr.model_path(ModelId::TransNetV2);
        assert!(path.ends_with("transnetv2.onnx"));
    }

    #[test]
    fn test_size_human_all_defined() {
        let models = [
            ModelId::TransNetV2,
            ModelId::Whisper3Turbo,
            ModelId::SAM2ViTB,
            ModelId::RIFEv4,
            ModelId::RealESRGAN4x,
            ModelId::DemucsV4,
            ModelId::SpeakerDiarize,
            ModelId::CLIPViTB32,
        ];
        for model in models {
            let size = model.size_human();
            assert!(size.contains("MB"), "Size should contain 'MB', got: {size}");
        }
    }
}
