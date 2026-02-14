//! Analysis result storage — sidecar JSON files per media asset.
//!
//! Persists all AI analysis results (transcript, scenes, speakers, embeddings)
//! as JSON files in the project's `.proedit/analysis/` directory.
//!
//! Format:
//! ```text
//! project/
//!   .proedit/
//!     analysis/
//!       {asset-uuid}.json    # transcript, scenes, speakers
//!       {asset-uuid}.emb     # binary vector embeddings
//! ```

use crate::audio_classify::AudioSegment;
use crate::content_index::{FrameEmbedding, SceneVisualInfo};
use crate::error::{AiError, AiResult};
use crate::scene_detect::SceneBoundary;
use crate::speaker_diarize::SpeakerSegment;
use crate::transcribe::Transcript;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Complete analysis results for a single media asset.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetAnalysis {
    /// Asset UUID.
    pub asset_id: String,
    /// Source filename.
    pub filename: String,
    /// Duration in seconds.
    pub duration_secs: f64,
    /// Timestamp of when the analysis was performed.
    pub analyzed_at: String,
    /// Speech transcript (if audio present).
    pub transcript: Option<Transcript>,
    /// Scene boundaries.
    pub scenes: Vec<SceneBoundary>,
    /// Speaker segments.
    pub speakers: Vec<SpeakerSegment>,
    /// Visual info per scene.
    pub visual_info: Vec<SceneVisualInfo>,
    /// Audio classification segments.
    pub audio_segments: Vec<AudioSegment>,
    /// Path to the binary embeddings file (relative to analysis dir).
    pub embeddings_path: Option<String>,
}

/// Manages analysis result storage.
pub struct AnalysisStore {
    analysis_dir: PathBuf,
}

impl AnalysisStore {
    /// Create a new analysis store for the given project directory.
    pub fn new(project_dir: &Path) -> Self {
        Self {
            analysis_dir: project_dir.join(".proedit").join("analysis"),
        }
    }

    /// Ensure the analysis directory exists.
    pub fn ensure_dir(&self) -> AiResult<()> {
        std::fs::create_dir_all(&self.analysis_dir)?;
        Ok(())
    }

    /// Save analysis results for an asset.
    pub fn save_analysis(&self, analysis: &AssetAnalysis) -> AiResult<()> {
        self.ensure_dir()?;

        let json_path = self.json_path(&analysis.asset_id);
        let json = serde_json::to_string_pretty(analysis).map_err(|e| {
            AiError::SerializationError(format!("Failed to serialize analysis: {e}"))
        })?;
        std::fs::write(&json_path, json)?;

        Ok(())
    }

    /// Load analysis results for an asset.
    pub fn load_analysis(&self, asset_id: &str) -> AiResult<AssetAnalysis> {
        let json_path = self.json_path(asset_id);
        if !json_path.exists() {
            return Err(AiError::ModelNotFound {
                model_id: format!("Analysis not found for asset: {asset_id}"),
            });
        }

        let json = std::fs::read_to_string(&json_path)?;
        serde_json::from_str(&json).map_err(|e| {
            AiError::SerializationError(format!("Failed to deserialize analysis: {e}"))
        })
    }

    /// Check if analysis exists for an asset.
    pub fn has_analysis(&self, asset_id: &str) -> bool {
        self.json_path(asset_id).exists()
    }

    /// Delete analysis for an asset.
    pub fn delete_analysis(&self, asset_id: &str) -> AiResult<()> {
        let json_path = self.json_path(asset_id);
        if json_path.exists() {
            std::fs::remove_file(&json_path)?;
        }

        let emb_path = self.embeddings_path(asset_id);
        if emb_path.exists() {
            std::fs::remove_file(&emb_path)?;
        }

        Ok(())
    }

    /// Save embeddings for an asset (binary format).
    pub fn save_embeddings(&self, asset_id: &str, embeddings: &[FrameEmbedding]) -> AiResult<()> {
        self.ensure_dir()?;
        let emb_path = self.embeddings_path(asset_id);
        let data = serde_json::to_vec(embeddings).map_err(|e| {
            AiError::SerializationError(format!("Failed to serialize embeddings: {e}"))
        })?;
        std::fs::write(&emb_path, data)?;
        Ok(())
    }

    /// Load embeddings for an asset.
    pub fn load_embeddings(&self, asset_id: &str) -> AiResult<Vec<FrameEmbedding>> {
        let emb_path = self.embeddings_path(asset_id);
        if !emb_path.exists() {
            return Err(AiError::ModelNotFound {
                model_id: format!("Embeddings not found for asset: {asset_id}"),
            });
        }
        let data = std::fs::read(&emb_path)?;
        serde_json::from_slice(&data).map_err(|e| {
            AiError::SerializationError(format!("Failed to deserialize embeddings: {e}"))
        })
    }

    /// List all analyzed asset IDs.
    pub fn list_analyzed_assets(&self) -> AiResult<Vec<String>> {
        if !self.analysis_dir.exists() {
            return Ok(Vec::new());
        }

        let mut ids = Vec::new();
        for entry in std::fs::read_dir(&self.analysis_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("json") {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    ids.push(stem.to_string());
                }
            }
        }

        Ok(ids)
    }

    /// Get the path to the analysis directory.
    pub fn analysis_dir(&self) -> &Path {
        &self.analysis_dir
    }

    /// JSON file path for an asset.
    fn json_path(&self, asset_id: &str) -> PathBuf {
        self.analysis_dir.join(format!("{asset_id}.json"))
    }

    /// Embeddings file path for an asset.
    fn embeddings_path(&self, asset_id: &str) -> PathBuf {
        self.analysis_dir.join(format!("{asset_id}.emb"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_test_analysis() -> AssetAnalysis {
        AssetAnalysis {
            asset_id: "test-asset-001".into(),
            filename: "interview.mp4".into(),
            duration_secs: 120.0,
            analyzed_at: "2025-01-15T10:30:00Z".into(),
            transcript: Some(Transcript {
                words: vec![],
                language: "en".into(),
                duration_secs: 120.0,
            }),
            scenes: vec![],
            speakers: vec![],
            visual_info: vec![],
            audio_segments: vec![],
            embeddings_path: Some("test-asset-001.emb".into()),
        }
    }

    #[test]
    fn test_save_and_load_analysis() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        let analysis = make_test_analysis();
        store.save_analysis(&analysis).unwrap();

        assert!(store.has_analysis("test-asset-001"));

        let loaded = store.load_analysis("test-asset-001").unwrap();
        assert_eq!(loaded.asset_id, "test-asset-001");
        assert_eq!(loaded.filename, "interview.mp4");
        assert!((loaded.duration_secs - 120.0).abs() < 0.01);
    }

    #[test]
    fn test_load_nonexistent_fails() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());
        assert!(store.load_analysis("nonexistent").is_err());
    }

    #[test]
    fn test_delete_analysis() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        let analysis = make_test_analysis();
        store.save_analysis(&analysis).unwrap();
        assert!(store.has_analysis("test-asset-001"));

        store.delete_analysis("test-asset-001").unwrap();
        assert!(!store.has_analysis("test-asset-001"));
    }

    #[test]
    fn test_save_and_load_embeddings() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        let embeddings = vec![
            FrameEmbedding {
                frame_number: 0,
                timestamp_secs: 0.0,
                vector: vec![0.1, 0.2, 0.3],
            },
            FrameEmbedding {
                frame_number: 30,
                timestamp_secs: 1.0,
                vector: vec![0.4, 0.5, 0.6],
            },
        ];

        store.save_embeddings("test-asset", &embeddings).unwrap();
        let loaded = store.load_embeddings("test-asset").unwrap();
        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded[0].frame_number, 0);
    }

    #[test]
    fn test_list_analyzed_assets() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        // Empty initially
        let ids = store.list_analyzed_assets().unwrap();
        assert!(ids.is_empty());

        // Save two analyses
        let mut a1 = make_test_analysis();
        a1.asset_id = "asset-aaa".into();
        store.save_analysis(&a1).unwrap();

        let mut a2 = make_test_analysis();
        a2.asset_id = "asset-bbb".into();
        store.save_analysis(&a2).unwrap();

        let ids = store.list_analyzed_assets().unwrap();
        assert_eq!(ids.len(), 2);
        assert!(ids.contains(&"asset-aaa".to_string()));
        assert!(ids.contains(&"asset-bbb".to_string()));
    }

    #[test]
    fn test_analysis_serialization_roundtrip() {
        let analysis = make_test_analysis();
        let json = serde_json::to_string(&analysis).unwrap();
        let decoded: AssetAnalysis = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.asset_id, analysis.asset_id);
    }
}
