//! Background analysis ingest pipeline.
//!
//! Orchestrates all analysis stages when footage is imported:
//! 1. Scene detection (TransNetV2 or MAD fallback)
//! 2. Audio transcription (Whisper)
//! 3. Speaker diarization
//! 4. Visual content indexing (CLIP embeddings)
//! 5. Audio classification
//!
//! Reports progress through a callback so the UI can display a progress bar.

use crate::analysis_store::{AnalysisStore, AssetAnalysis};
use crate::audio_classify::{AudioClassifier, AudioClassifyConfig};
use crate::content_index::{self, ContentIndex, FrameEmbedding, SceneVisualInfo};
use crate::error::AiResult;
use crate::scene_detect::{self, SceneDetectConfig};
use crate::speaker_diarize::{DiarizeConfig, SpeakerDiarizer};
use proedit_core::FrameBuffer;
use serde::{Deserialize, Serialize};
use tracing::{debug, info, warn};

/// Progress of the ingest pipeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestProgress {
    /// Current stage being processed.
    pub stage: IngestStage,
    /// Overall progress (0.0 to 1.0).
    pub overall_progress: f32,
    /// Stage-specific progress (0.0 to 1.0).
    pub stage_progress: f32,
    /// Human-readable status message.
    pub message: String,
}

/// Pipeline stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum IngestStage {
    /// Detecting scene boundaries.
    SceneDetection,
    /// Transcribing audio.
    Transcription,
    /// Identifying speakers.
    SpeakerDiarization,
    /// Indexing visual content.
    VisualIndexing,
    /// Classifying audio segments.
    AudioClassification,
    /// Saving results.
    Saving,
    /// All done.
    Complete,
}

impl IngestStage {
    /// Display name for the stage.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::SceneDetection => "Detecting scenes",
            Self::Transcription => "Transcribing audio",
            Self::SpeakerDiarization => "Identifying speakers",
            Self::VisualIndexing => "Indexing visual content",
            Self::AudioClassification => "Classifying audio",
            Self::Saving => "Saving results",
            Self::Complete => "Analysis complete",
        }
    }

    /// Weight of this stage in overall progress (out of 100).
    fn weight(&self) -> f32 {
        match self {
            Self::SceneDetection => 15.0,
            Self::Transcription => 35.0,
            Self::SpeakerDiarization => 15.0,
            Self::VisualIndexing => 20.0,
            Self::AudioClassification => 10.0,
            Self::Saving => 5.0,
            Self::Complete => 0.0,
        }
    }
}

/// Configuration for the ingest pipeline.
#[derive(Debug, Clone)]
pub struct IngestConfig {
    /// Scene detection configuration.
    pub scene_detect: SceneDetectConfig,
    /// Speaker diarization configuration.
    pub diarize: DiarizeConfig,
    /// Audio classification configuration.
    pub audio_classify: AudioClassifyConfig,
    /// Whether to skip transcription (e.g., no audio).
    pub skip_transcription: bool,
    /// Whether to skip visual indexing.
    pub skip_visual_indexing: bool,
    /// Frames per second for frame sampling during visual indexing.
    pub visual_sample_fps: f32,
}

impl Default for IngestConfig {
    fn default() -> Self {
        Self {
            scene_detect: SceneDetectConfig::default(),
            diarize: DiarizeConfig::default(),
            audio_classify: AudioClassifyConfig::default(),
            skip_transcription: false,
            skip_visual_indexing: false,
            visual_sample_fps: 1.0, // 1 frame per second
        }
    }
}

/// Input data for the ingest pipeline.
pub struct IngestInput {
    /// Asset UUID.
    pub asset_id: String,
    /// Source filename.
    pub filename: String,
    /// Video frames (sampled at a regular interval for analysis).
    pub frames: Vec<FrameBuffer>,
    /// Frame rate of the source video.
    pub fps: f64,
    /// Audio samples (16kHz mono f32 for analysis).
    pub audio_samples: Option<Vec<f32>>,
    /// Audio sample rate.
    pub audio_sample_rate: u32,
    /// Total duration of the media in seconds.
    pub duration_secs: f64,
}

/// Result of the ingest pipeline.
pub struct IngestResult {
    /// The complete analysis.
    pub analysis: AssetAnalysis,
    /// Content index with embeddings (not persisted in JSON, separate file).
    pub content_index: ContentIndex,
}

/// Run the full ingest pipeline on a media asset.
///
/// The `progress_callback` is called at each stage transition with the
/// current progress. This allows the UI to display a progress bar.
pub fn run_ingest_pipeline(
    input: &IngestInput,
    config: &IngestConfig,
    store: &AnalysisStore,
    mut progress_callback: impl FnMut(IngestProgress),
) -> AiResult<IngestResult> {
    let total_weight: f32 = 100.0;
    let mut completed_weight: f32 = 0.0;

    // ── Stage 1: Scene Detection ──────────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::SceneDetection,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Analyzing scenes...".into(),
    });

    info!(asset = %input.asset_id, "Starting scene detection");
    let scenes =
        scene_detect::detect_scenes_by_difference(&input.frames, input.fps, &config.scene_detect);
    info!(scenes = scenes.len(), "Scene detection complete");

    completed_weight += IngestStage::SceneDetection.weight();

    // ── Stage 2: Transcription ────────────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::Transcription,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Transcribing audio...".into(),
    });

    let transcript = if config.skip_transcription || input.audio_samples.is_none() {
        debug!("Skipping transcription (no audio or disabled)");
        None
    } else {
        // In production, this would call the Transcriber.
        // For now, return None since we can't run Whisper without the model.
        debug!("Transcription requires Whisper model — skipping for now");
        None
    };

    completed_weight += IngestStage::Transcription.weight();

    // ── Stage 3: Speaker Diarization ──────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::SpeakerDiarization,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Identifying speakers...".into(),
    });

    let speakers = if let Some(ref audio) = input.audio_samples {
        let diarizer = SpeakerDiarizer::new(config.diarize.clone());
        match diarizer.diarize(audio, input.audio_sample_rate) {
            Ok(segs) => {
                info!(speakers = segs.len(), "Speaker diarization complete");
                segs
            }
            Err(e) => {
                warn!(error = %e, "Speaker diarization failed, continuing");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    completed_weight += IngestStage::SpeakerDiarization.weight();

    // ── Stage 4: Visual Content Indexing ───────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::VisualIndexing,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Indexing visual content...".into(),
    });

    let mut content_index = ContentIndex::new();
    let mut visual_info = Vec::new();

    if !config.skip_visual_indexing && !input.frames.is_empty() {
        // Sample frames for embedding and visual analysis
        let sample_interval = (input.fps / config.visual_sample_fps as f64).max(1.0) as usize;

        for (i, frame) in input.frames.iter().enumerate() {
            if i % sample_interval != 0 {
                continue;
            }

            let frame_number = i as i64;
            let timestamp = frame_number as f64 / input.fps;

            // Placeholder embedding (in production: CLIP encoder)
            let embedding = FrameEmbedding {
                frame_number,
                timestamp_secs: timestamp,
                vector: vec![0.0; 512], // placeholder 512-dim
            };
            content_index.add_embedding(embedding);

            // Visual analysis for sampled frames
            let shot_type = content_index::classify_shot_type(frame);
            let colors = content_index::extract_dominant_colors(frame, 5);
            let brightness = content_index::average_brightness(frame);

            visual_info.push(SceneVisualInfo {
                start_frame: frame_number,
                end_frame: frame_number + sample_interval as i64,
                shot_type,
                camera_motion: content_index::CameraMotion::Unknown,
                dominant_colors: colors,
                avg_brightness: brightness,
            });

            // Report stage progress
            let stage_progress = i as f32 / input.frames.len() as f32;
            progress_callback(IngestProgress {
                stage: IngestStage::VisualIndexing,
                overall_progress: (completed_weight
                    + IngestStage::VisualIndexing.weight() * stage_progress)
                    / total_weight,
                stage_progress,
                message: format!(
                    "Indexing frame {}/{}...",
                    i / sample_interval + 1,
                    input.frames.len() / sample_interval
                ),
            });
        }
        info!(indexed = content_index.len(), "Visual indexing complete");
    }

    completed_weight += IngestStage::VisualIndexing.weight();

    // ── Stage 5: Audio Classification ─────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::AudioClassification,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Classifying audio...".into(),
    });

    let audio_segments = if let Some(ref audio) = input.audio_samples {
        let classifier = AudioClassifier::new(config.audio_classify.clone());
        match classifier.classify_segments(audio, input.audio_sample_rate) {
            Ok(segs) => {
                info!(segments = segs.len(), "Audio classification complete");
                segs
            }
            Err(e) => {
                warn!(error = %e, "Audio classification failed, continuing");
                Vec::new()
            }
        }
    } else {
        Vec::new()
    };

    completed_weight += IngestStage::AudioClassification.weight();

    // ── Stage 6: Save Results ─────────────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::Saving,
        overall_progress: completed_weight / total_weight,
        stage_progress: 0.0,
        message: "Saving analysis results...".into(),
    });

    let analysis = AssetAnalysis {
        asset_id: input.asset_id.clone(),
        filename: input.filename.clone(),
        duration_secs: input.duration_secs,
        analyzed_at: chrono_now_placeholder(),
        transcript,
        scenes,
        speakers,
        visual_info,
        audio_segments,
        embeddings_path: if content_index.is_empty() {
            None
        } else {
            Some(format!("{}.emb", input.asset_id))
        },
    };

    store.save_analysis(&analysis)?;

    if !content_index.is_empty() {
        store.save_embeddings(&input.asset_id, content_index.embeddings())?;
    }

    info!(asset = %input.asset_id, "Ingest pipeline complete");

    // ── Done ──────────────────────────────────────────────────────────
    progress_callback(IngestProgress {
        stage: IngestStage::Complete,
        overall_progress: 1.0,
        stage_progress: 1.0,
        message: "Analysis complete".into(),
    });

    Ok(IngestResult {
        analysis,
        content_index,
    })
}

/// Placeholder for current timestamp (avoids adding chrono dependency).
fn chrono_now_placeholder() -> String {
    "2025-01-01T00:00:00Z".into()
}

#[cfg(test)]
fn make_solid_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> FrameBuffer {
    let mut frame = FrameBuffer::new(w, h, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();
    for y in 0..h {
        let row = plane.row_mut(y);
        for x in 0..w as usize {
            let base = x * 4;
            if base + 3 < row.len() {
                row[base] = r;
                row[base + 1] = g;
                row[base + 2] = b;
                row[base + 3] = 255;
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ingest_stage_display() {
        assert_eq!(
            IngestStage::SceneDetection.display_name(),
            "Detecting scenes"
        );
        assert_eq!(IngestStage::Complete.display_name(), "Analysis complete");
    }

    #[test]
    fn test_ingest_stage_weights_sum_to_100() {
        let stages = [
            IngestStage::SceneDetection,
            IngestStage::Transcription,
            IngestStage::SpeakerDiarization,
            IngestStage::VisualIndexing,
            IngestStage::AudioClassification,
            IngestStage::Saving,
        ];
        let total: f32 = stages.iter().map(|s| s.weight()).sum();
        assert!(
            (total - 100.0).abs() < 0.01,
            "Stage weights should sum to 100, got {total}"
        );
    }

    #[test]
    fn test_pipeline_with_video_only() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        let frames = vec![
            make_solid_frame(64, 64, 100, 100, 100),
            make_solid_frame(64, 64, 100, 100, 100),
            make_solid_frame(64, 64, 200, 200, 200), // scene change
            make_solid_frame(64, 64, 200, 200, 200),
        ];

        let input = IngestInput {
            asset_id: "test-video-001".into(),
            filename: "test.mp4".into(),
            frames,
            fps: 30.0,
            audio_samples: None,
            audio_sample_rate: 16000,
            duration_secs: 4.0 / 30.0,
        };

        let config = IngestConfig {
            skip_transcription: true,
            ..Default::default()
        };

        let mut stages_seen = Vec::new();
        let result = run_ingest_pipeline(&input, &config, &store, |progress| {
            if !stages_seen.contains(&progress.stage) {
                stages_seen.push(progress.stage);
            }
        })
        .unwrap();

        // Should have gone through all stages
        assert!(stages_seen.contains(&IngestStage::SceneDetection));
        assert!(stages_seen.contains(&IngestStage::Complete));

        // Analysis should be saved
        assert!(store.has_analysis("test-video-001"));
        assert_eq!(result.analysis.asset_id, "test-video-001");
    }

    #[test]
    fn test_pipeline_with_audio() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        // Generate a tone for audio
        let sample_rate = 16000u32;
        let audio: Vec<f32> = (0..sample_rate * 2)
            .map(|i| {
                (i as f32 * 200.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5
            })
            .collect();

        let input = IngestInput {
            asset_id: "test-audio-001".into(),
            filename: "test_with_audio.mp4".into(),
            frames: vec![make_solid_frame(64, 64, 128, 128, 128)],
            fps: 30.0,
            audio_samples: Some(audio),
            audio_sample_rate: sample_rate,
            duration_secs: 2.0,
        };

        let config = IngestConfig {
            skip_transcription: true,
            ..Default::default()
        };

        let result = run_ingest_pipeline(&input, &config, &store, |_| {}).unwrap();

        // Should have speaker segments and audio classification
        assert!(store.has_analysis("test-audio-001"));
        // Audio classification should produce segments
        assert!(
            !result.analysis.audio_segments.is_empty(),
            "Should have audio segments"
        );
    }

    #[test]
    fn test_progress_callback_monotonic() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let store = AnalysisStore::new(tmp.path());

        let input = IngestInput {
            asset_id: "test-progress".into(),
            filename: "test.mp4".into(),
            frames: vec![make_solid_frame(32, 32, 100, 100, 100)],
            fps: 30.0,
            audio_samples: None,
            audio_sample_rate: 16000,
            duration_secs: 1.0,
        };

        let config = IngestConfig {
            skip_transcription: true,
            ..Default::default()
        };

        let mut last_progress = -1.0_f32;
        run_ingest_pipeline(&input, &config, &store, |progress| {
            assert!(
                progress.overall_progress >= last_progress,
                "Progress should be monotonically increasing: {} -> {}",
                last_progress,
                progress.overall_progress
            );
            last_progress = progress.overall_progress;
        })
        .unwrap();

        // Final progress should be 1.0
        assert!(
            (last_progress - 1.0).abs() < 0.01,
            "Final progress should be 1.0, got {last_progress}"
        );
    }
}
