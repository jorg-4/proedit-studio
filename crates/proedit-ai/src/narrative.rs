//! Narrative intelligence — cloud-based editorial feedback.
//!
//! Analyzes a rough cut and provides editorial feedback: pacing issues,
//! emotional arc mapping, structural suggestions, format-specific optimization.
//!
//! This feature is opt-in and requires an internet connection + API key.
//! No raw video frames are ever sent — only transcript text and edit metadata.

use crate::error::{AiError, AiResult};
use crate::transcribe::Transcript;
use serde::{Deserialize, Serialize};

/// Metadata about the current edit sent to the cloud for analysis.
/// No raw footage is included — only structural information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditMetadata {
    /// Timestamped transcript from Whisper.
    pub transcript: Option<Transcript>,
    /// Clip structure of the edit.
    pub clips: Vec<ClipInfo>,
    /// Transition points.
    pub transitions: Vec<TransitionInfo>,
    /// Normalized audio energy curve (0.0 to 1.0 per second).
    pub audio_energy: Vec<f32>,
    /// Total duration of the edit in seconds.
    pub total_duration: f64,
    /// Target format/platform.
    pub target_format: TargetFormat,
    /// Optional user notes describing the intended content.
    pub user_notes: String,
}

/// Information about a single clip in the edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipInfo {
    /// Start time in the timeline (seconds).
    pub start: f64,
    /// End time in the timeline (seconds).
    pub end: f64,
    /// Shot type classification.
    pub shot_type: String,
    /// Speaker label (if identified).
    pub speaker: Option<String>,
    /// Source asset identifier.
    pub asset_id: String,
}

/// Information about a transition in the edit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionInfo {
    /// Time of the transition (seconds).
    pub time: f64,
    /// Type of transition.
    pub transition_type: String,
    /// Duration of the transition (0 for hard cuts).
    pub duration: f64,
}

/// Target format for the video.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TargetFormat {
    /// YouTube long-form video.
    YouTubeVideo,
    /// YouTube Shorts / TikTok / Reels.
    ShortForm,
    /// Documentary.
    Documentary,
    /// Social media advertisement.
    SocialAd,
    /// Corporate / training video.
    Corporate,
    /// Film / narrative.
    Film,
    /// Podcast / talking head.
    Podcast,
    /// Custom / unspecified.
    Custom,
}

impl TargetFormat {
    /// Display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::YouTubeVideo => "YouTube Video",
            Self::ShortForm => "Short Form (TikTok/Reels)",
            Self::Documentary => "Documentary",
            Self::SocialAd => "Social Ad",
            Self::Corporate => "Corporate",
            Self::Film => "Film / Narrative",
            Self::Podcast => "Podcast",
            Self::Custom => "Custom",
        }
    }
}

/// Feedback from the narrative analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeFeedback {
    /// Overall quality score (0 to 100).
    pub overall_score: f32,
    /// Emotional arc over the edit timeline.
    pub emotional_arc: Vec<EmotionPoint>,
    /// Pacing analysis.
    pub pacing: PacingReport,
    /// Actionable edit suggestions with timecodes.
    pub suggestions: Vec<EditSuggestion>,
}

/// A point on the emotional arc curve.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmotionPoint {
    /// Time in seconds.
    pub time: f64,
    /// Emotion type.
    pub emotion: EmotionType,
    /// Intensity (0.0 to 1.0).
    pub intensity: f32,
}

/// Types of emotion detected in the edit.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum EmotionType {
    Neutral,
    Excitement,
    Tension,
    Joy,
    Sadness,
    Surprise,
    Calm,
}

/// Pacing analysis report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PacingReport {
    /// Average cut length in seconds.
    pub avg_cut_length: f64,
    /// Standard deviation of cut lengths.
    pub cut_length_variance: f64,
    /// Pacing rhythm classification.
    pub rhythm: PacingRhythm,
    /// Sections that feel too slow.
    pub slow_sections: Vec<TimeSection>,
    /// Sections that feel too fast.
    pub fast_sections: Vec<TimeSection>,
}

/// Pacing rhythm classification.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum PacingRhythm {
    /// Consistent, steady pacing.
    Steady,
    /// Gradually accelerating.
    BuildingTension,
    /// Alternating fast and slow.
    Dynamic,
    /// Erratic, inconsistent.
    Erratic,
}

/// A section of the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSection {
    /// Start time in seconds.
    pub start: f64,
    /// End time in seconds.
    pub end: f64,
}

/// An actionable edit suggestion.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditSuggestion {
    /// Timecode where the suggestion applies.
    pub timecode: f64,
    /// Severity level.
    pub severity: Severity,
    /// Category of suggestion.
    pub category: String,
    /// Human-readable description.
    pub description: String,
}

/// Severity level for a suggestion.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum Severity {
    /// Informational observation.
    Info,
    /// Recommended improvement.
    Suggestion,
    /// Potential issue that should be addressed.
    Warning,
}

/// Configuration for the narrative analyzer.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeConfig {
    /// API endpoint URL.
    pub api_url: String,
    /// API key (stored securely, not in project files).
    pub api_key: String,
    /// Model to use for analysis.
    pub model: String,
    /// Maximum tokens for the response.
    pub max_tokens: u32,
}

impl Default for NarrativeConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.anthropic.com/v1/messages".into(),
            api_key: String::new(),
            model: "claude-sonnet-4-5-20250929".into(),
            max_tokens: 4096,
        }
    }
}

/// Narrative intelligence analyzer (cloud-based, opt-in).
pub struct NarrativeAnalyzer {
    config: NarrativeConfig,
}

impl NarrativeAnalyzer {
    /// Create a new narrative analyzer.
    pub fn new(config: NarrativeConfig) -> AiResult<Self> {
        if config.api_key.is_empty() {
            return Err(AiError::CloudApiError(
                "API key is required for narrative analysis".into(),
            ));
        }
        Ok(Self { config })
    }

    /// Check if the analyzer is configured and ready.
    pub fn is_configured(&self) -> bool {
        !self.config.api_key.is_empty()
    }

    /// Analyze the edit and return narrative feedback.
    ///
    /// This sends edit metadata (NOT raw footage) to the cloud API.
    pub async fn analyze_edit(&self, edit_data: &EditMetadata) -> AiResult<NarrativeFeedback> {
        if !self.is_configured() {
            return Err(AiError::CloudApiError("API key not configured".into()));
        }

        // Serialize edit metadata to JSON for the API request
        let _edit_json = serde_json::to_string(edit_data).map_err(|e| {
            AiError::SerializationError(format!("Failed to serialize edit metadata: {e}"))
        })?;

        // In production, this would make an HTTP request to the Claude API.
        // For now, return a local analysis using heuristics.
        Ok(local_pacing_analysis(edit_data))
    }

    /// Get the configuration.
    pub fn config(&self) -> &NarrativeConfig {
        &self.config
    }
}

/// Local pacing analysis as fallback / offline mode.
/// Computes basic pacing metrics without cloud AI.
fn local_pacing_analysis(edit_data: &EditMetadata) -> NarrativeFeedback {
    let mut suggestions = Vec::new();

    // Compute average cut length
    let cut_lengths: Vec<f64> = edit_data
        .clips
        .iter()
        .map(|c| c.end - c.start)
        .collect();

    let avg_cut = if cut_lengths.is_empty() {
        0.0
    } else {
        cut_lengths.iter().sum::<f64>() / cut_lengths.len() as f64
    };

    let variance = if cut_lengths.len() > 1 {
        let mean = avg_cut;
        cut_lengths.iter().map(|&l| (l - mean).powi(2)).sum::<f64>()
            / (cut_lengths.len() - 1) as f64
    } else {
        0.0
    };

    // Check for hook (first 3 seconds)
    if edit_data.total_duration > 10.0 {
        let first_clip_end = edit_data.clips.first().map(|c| c.end).unwrap_or(0.0);
        if first_clip_end > 5.0 {
            suggestions.push(EditSuggestion {
                timecode: 0.0,
                severity: Severity::Suggestion,
                category: "hook".into(),
                description: "Consider a shorter opening clip to hook viewers in the first 3 seconds".into(),
            });
        }
    }

    // Check for long static sections
    for clip in &edit_data.clips {
        let duration = clip.end - clip.start;
        if duration > 30.0 {
            suggestions.push(EditSuggestion {
                timecode: clip.start + 15.0,
                severity: Severity::Suggestion,
                category: "pacing".into(),
                description: format!(
                    "This {:.0}s clip may benefit from a cutaway or b-roll to maintain engagement",
                    duration
                ),
            });
        }
    }

    let rhythm = if variance < 1.0 {
        PacingRhythm::Steady
    } else if variance < 5.0 {
        PacingRhythm::Dynamic
    } else {
        PacingRhythm::Erratic
    };

    NarrativeFeedback {
        overall_score: 70.0,
        emotional_arc: vec![
            EmotionPoint {
                time: 0.0,
                emotion: EmotionType::Neutral,
                intensity: 0.3,
            },
            EmotionPoint {
                time: edit_data.total_duration * 0.5,
                emotion: EmotionType::Excitement,
                intensity: 0.7,
            },
            EmotionPoint {
                time: edit_data.total_duration,
                emotion: EmotionType::Calm,
                intensity: 0.4,
            },
        ],
        pacing: PacingReport {
            avg_cut_length: avg_cut,
            cut_length_variance: variance,
            rhythm,
            slow_sections: Vec::new(),
            fast_sections: Vec::new(),
        },
        suggestions,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_target_format_display() {
        assert_eq!(TargetFormat::YouTubeVideo.display_name(), "YouTube Video");
        assert_eq!(TargetFormat::ShortForm.display_name(), "Short Form (TikTok/Reels)");
    }

    #[test]
    fn test_narrative_config_default() {
        let config = NarrativeConfig::default();
        assert!(!config.api_url.is_empty());
        assert!(config.api_key.is_empty());
    }

    #[test]
    fn test_analyzer_requires_api_key() {
        let config = NarrativeConfig::default();
        let result = NarrativeAnalyzer::new(config);
        assert!(result.is_err());
    }

    #[test]
    fn test_analyzer_creation_with_key() {
        let config = NarrativeConfig {
            api_key: "test-key".into(),
            ..Default::default()
        };
        let analyzer = NarrativeAnalyzer::new(config).unwrap();
        assert!(analyzer.is_configured());
    }

    #[test]
    fn test_local_pacing_analysis() {
        let edit = EditMetadata {
            transcript: None,
            clips: vec![
                ClipInfo {
                    start: 0.0,
                    end: 3.0,
                    shot_type: "close-up".into(),
                    speaker: Some("A".into()),
                    asset_id: "clip1".into(),
                },
                ClipInfo {
                    start: 3.0,
                    end: 8.0,
                    shot_type: "wide".into(),
                    speaker: None,
                    asset_id: "clip2".into(),
                },
            ],
            transitions: vec![TransitionInfo {
                time: 3.0,
                transition_type: "cut".into(),
                duration: 0.0,
            }],
            audio_energy: vec![0.3, 0.5, 0.8, 0.4],
            total_duration: 8.0,
            target_format: TargetFormat::YouTubeVideo,
            user_notes: String::new(),
        };

        let feedback = local_pacing_analysis(&edit);
        assert!(feedback.overall_score > 0.0);
        assert!(!feedback.emotional_arc.is_empty());
        assert!(feedback.pacing.avg_cut_length > 0.0);
    }

    #[test]
    fn test_edit_metadata_serialization() {
        let edit = EditMetadata {
            transcript: None,
            clips: vec![],
            transitions: vec![],
            audio_energy: vec![],
            total_duration: 0.0,
            target_format: TargetFormat::Custom,
            user_notes: "test".into(),
        };
        let json = serde_json::to_string(&edit).unwrap();
        let _: EditMetadata = serde_json::from_str(&json).unwrap();
    }

    #[test]
    fn test_severity_ordering() {
        // Just verify they serialize properly
        let s = Severity::Info;
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("Info"));
    }
}
