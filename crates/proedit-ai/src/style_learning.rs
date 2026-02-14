//! Style learning — observes editing decisions and builds a personal profile.
//!
//! Tracks the user's preferred cut rhythms, color palettes, transition choices,
//! text placement patterns, and pacing preferences. After enough observations,
//! AI suggestions adapt to match the user's personal aesthetic.
//!
//! All data is stored locally. No cloud connection needed.

use crate::error::{AiError, AiResult};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// A user's editing style profile, built from observed editing decisions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EditorProfile {
    /// Average cut duration in seconds.
    pub avg_cut_duration: f32,
    /// Standard deviation of cut durations.
    pub cut_duration_variance: f32,
    /// Preferred transition types and their usage frequency.
    pub preferred_transitions: HashMap<String, f32>,
    /// Color grading tendencies.
    pub color_tendencies: ColorProfile,
    /// Typical energy/pacing curve over video duration (normalized 0-1 for both axes).
    pub pacing_pattern: Vec<f32>,
    /// Text style preferences.
    pub text_style: TextPreferences,
    /// Total number of edits analyzed.
    pub total_edits_analyzed: u64,
    /// Running sum of cut durations (for incremental average).
    #[serde(default)]
    cut_duration_sum: f64,
    /// Running sum of squared cut durations (for incremental variance).
    #[serde(default)]
    cut_duration_sq_sum: f64,
    /// Number of cuts observed.
    #[serde(default)]
    cut_count: u64,
}

impl Default for EditorProfile {
    fn default() -> Self {
        Self {
            avg_cut_duration: 0.0,
            cut_duration_variance: 0.0,
            preferred_transitions: HashMap::new(),
            color_tendencies: ColorProfile::default(),
            pacing_pattern: Vec::new(),
            text_style: TextPreferences::default(),
            total_edits_analyzed: 0,
            cut_duration_sum: 0.0,
            cut_duration_sq_sum: 0.0,
            cut_count: 0,
        }
    }
}

/// Color grading tendencies observed from the user's edits.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorProfile {
    /// Average color temperature bias (-1.0 = cool, 0.0 = neutral, 1.0 = warm).
    pub temperature_bias: f32,
    /// Average contrast level (0.0 = low, 1.0 = high).
    pub contrast_level: f32,
    /// Average saturation level (0.0 = desaturated, 1.0 = vivid).
    pub saturation_level: f32,
    /// Number of color observations.
    pub observation_count: u32,
}

impl Default for ColorProfile {
    fn default() -> Self {
        Self {
            temperature_bias: 0.0,
            contrast_level: 0.5,
            saturation_level: 0.5,
            observation_count: 0,
        }
    }
}

/// Text style preferences observed from the user's title/caption choices.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextPreferences {
    /// Most frequently used font families.
    pub preferred_fonts: Vec<String>,
    /// Average font size (points).
    pub avg_font_size: f32,
    /// Preferred text position (normalized: 0.0 = top, 1.0 = bottom).
    pub preferred_y_position: f32,
    /// Preferred text alignment.
    pub preferred_alignment: String,
    /// Number of text observations.
    pub observation_count: u32,
}

impl Default for TextPreferences {
    fn default() -> Self {
        Self {
            preferred_fonts: Vec::new(),
            preferred_alignment: "center".into(),
            avg_font_size: 24.0,
            preferred_y_position: 0.85,
            observation_count: 0,
        }
    }
}

/// An observed edit operation that feeds into the style profile.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditOperation {
    /// A cut was made with the given clip duration.
    Cut { duration: f32 },
    /// A transition was applied.
    Transition { kind: String, duration: f32 },
    /// A color grade was applied.
    ColorGrade {
        temperature: f32,
        contrast: f32,
        saturation: f32,
    },
    /// A text element was placed.
    TextPlacement {
        font: String,
        size: f32,
        y_position: f32,
        alignment: String,
    },
}

impl EditorProfile {
    /// Observe an edit operation and update the profile incrementally.
    pub fn observe_edit(&mut self, edit: &EditOperation) {
        match edit {
            EditOperation::Cut { duration } => {
                self.update_cut_stats(*duration);
            }
            EditOperation::Transition { kind, duration: _ } => {
                *self.preferred_transitions.entry(kind.clone()).or_insert(0.0) += 1.0;
            }
            EditOperation::ColorGrade {
                temperature,
                contrast,
                saturation,
            } => {
                self.color_tendencies.observe(*temperature, *contrast, *saturation);
            }
            EditOperation::TextPlacement {
                font,
                size,
                y_position,
                alignment,
            } => {
                self.text_style
                    .observe(font, *size, *y_position, alignment);
            }
        }
        self.total_edits_analyzed += 1;
    }

    /// Update running cut statistics.
    fn update_cut_stats(&mut self, duration: f32) {
        let d = duration as f64;
        self.cut_duration_sum += d;
        self.cut_duration_sq_sum += d * d;
        self.cut_count += 1;

        let n = self.cut_count as f64;
        self.avg_cut_duration = (self.cut_duration_sum / n) as f32;

        if self.cut_count > 1 {
            let mean = self.cut_duration_sum / n;
            let variance = (self.cut_duration_sq_sum / n) - (mean * mean);
            self.cut_duration_variance = variance.max(0.0).sqrt() as f32;
        }
    }

    /// Generate a text summary of the editing style for use in AI prompts.
    pub fn style_summary(&self) -> String {
        if self.total_edits_analyzed < 10 {
            return "Insufficient editing data to determine style preferences.".into();
        }

        let mut parts = Vec::new();

        if self.cut_count > 0 {
            parts.push(format!(
                "Average cut duration: {:.1}s (variance: {:.1}s)",
                self.avg_cut_duration, self.cut_duration_variance
            ));
        }

        // Top transition preferences
        if !self.preferred_transitions.is_empty() {
            let mut transitions: Vec<_> = self.preferred_transitions.iter().collect();
            transitions.sort_by(|a, b| b.1.partial_cmp(a.1).unwrap_or(std::cmp::Ordering::Equal));
            let top: Vec<_> = transitions
                .iter()
                .take(3)
                .map(|(k, v)| format!("{k}: {v:.0}"))
                .collect();
            parts.push(format!("Preferred transitions: {}", top.join(", ")));
        }

        let cp = &self.color_tendencies;
        if cp.observation_count > 0 {
            let temp_desc = if cp.temperature_bias > 0.2 {
                "warm"
            } else if cp.temperature_bias < -0.2 {
                "cool"
            } else {
                "neutral"
            };
            parts.push(format!(
                "Color style: {temp_desc} tones, contrast {:.0}%, saturation {:.0}%",
                cp.contrast_level * 100.0,
                cp.saturation_level * 100.0
            ));
        }

        parts.join(". ")
    }

    /// Save the profile to a JSON file.
    pub fn save(&self, path: &Path) -> AiResult<()> {
        let json = serde_json::to_string_pretty(self).map_err(|e| {
            AiError::SerializationError(format!("Failed to serialize profile: {e}"))
        })?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load a profile from a JSON file.
    pub fn load(path: &Path) -> AiResult<Self> {
        let json = std::fs::read_to_string(path)?;
        serde_json::from_str(&json).map_err(|e| {
            AiError::SerializationError(format!("Failed to deserialize profile: {e}"))
        })
    }

    /// Default profile file path.
    pub fn default_path() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("proedit-studio")
            .join("editor_profile.json")
    }

    /// Whether enough data has been collected for meaningful style suggestions.
    pub fn is_mature(&self) -> bool {
        self.total_edits_analyzed >= 50
    }
}

impl ColorProfile {
    /// Observe a color grade operation.
    fn observe(&mut self, temperature: f32, contrast: f32, saturation: f32) {
        let n = self.observation_count as f32;
        let new_n = n + 1.0;

        // Incremental running average
        self.temperature_bias = (self.temperature_bias * n + temperature) / new_n;
        self.contrast_level = (self.contrast_level * n + contrast) / new_n;
        self.saturation_level = (self.saturation_level * n + saturation) / new_n;
        self.observation_count += 1;
    }
}

impl TextPreferences {
    /// Observe a text placement operation.
    fn observe(&mut self, font: &str, size: f32, y_position: f32, alignment: &str) {
        let n = self.observation_count as f32;
        let new_n = n + 1.0;

        self.avg_font_size = (self.avg_font_size * n + size) / new_n;
        self.preferred_y_position = (self.preferred_y_position * n + y_position) / new_n;
        self.observation_count += 1;

        // Track font usage
        if !self.preferred_fonts.contains(&font.to_string()) {
            self.preferred_fonts.push(font.to_string());
        }

        self.preferred_alignment = alignment.to_string();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        let profile = EditorProfile::default();
        assert_eq!(profile.total_edits_analyzed, 0);
        assert_eq!(profile.avg_cut_duration, 0.0);
        assert!(!profile.is_mature());
    }

    #[test]
    fn test_observe_cuts() {
        let mut profile = EditorProfile::default();
        profile.observe_edit(&EditOperation::Cut { duration: 2.0 });
        profile.observe_edit(&EditOperation::Cut { duration: 4.0 });
        profile.observe_edit(&EditOperation::Cut { duration: 3.0 });

        assert_eq!(profile.cut_count, 3);
        assert!((profile.avg_cut_duration - 3.0).abs() < 0.01);
        assert!(profile.cut_duration_variance > 0.0);
    }

    #[test]
    fn test_observe_transitions() {
        let mut profile = EditorProfile::default();
        profile.observe_edit(&EditOperation::Transition {
            kind: "cut".into(),
            duration: 0.0,
        });
        profile.observe_edit(&EditOperation::Transition {
            kind: "cut".into(),
            duration: 0.0,
        });
        profile.observe_edit(&EditOperation::Transition {
            kind: "dissolve".into(),
            duration: 0.5,
        });

        assert_eq!(profile.preferred_transitions["cut"], 2.0);
        assert_eq!(profile.preferred_transitions["dissolve"], 1.0);
    }

    #[test]
    fn test_observe_color_grade() {
        let mut profile = EditorProfile::default();
        profile.observe_edit(&EditOperation::ColorGrade {
            temperature: 0.5,
            contrast: 0.7,
            saturation: 0.8,
        });
        profile.observe_edit(&EditOperation::ColorGrade {
            temperature: 0.3,
            contrast: 0.5,
            saturation: 0.6,
        });

        let cp = &profile.color_tendencies;
        assert_eq!(cp.observation_count, 2);
        assert!((cp.temperature_bias - 0.4).abs() < 0.01);
        assert!((cp.contrast_level - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_style_summary_insufficient_data() {
        let profile = EditorProfile::default();
        let summary = profile.style_summary();
        assert!(summary.contains("Insufficient"));
    }

    #[test]
    fn test_style_summary_with_data() {
        let mut profile = EditorProfile::default();
        for i in 0..60 {
            profile.observe_edit(&EditOperation::Cut {
                duration: 2.0 + (i as f32 * 0.1),
            });
        }
        let summary = profile.style_summary();
        assert!(summary.contains("Average cut duration"));
    }

    #[test]
    fn test_profile_save_load() {
        let tmp = tempfile::tempdir().expect("failed to create tempdir");
        let path = tmp.path().join("test_profile.json");

        let mut profile = EditorProfile::default();
        profile.observe_edit(&EditOperation::Cut { duration: 3.0 });
        profile.observe_edit(&EditOperation::Transition {
            kind: "dissolve".into(),
            duration: 0.5,
        });

        profile.save(&path).unwrap();
        let loaded = EditorProfile::load(&path).unwrap();

        assert_eq!(loaded.total_edits_analyzed, 2);
        assert_eq!(loaded.cut_count, 1);
        assert!((loaded.avg_cut_duration - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_is_mature() {
        let mut profile = EditorProfile::default();
        assert!(!profile.is_mature());

        for _ in 0..50 {
            profile.observe_edit(&EditOperation::Cut { duration: 2.0 });
        }
        assert!(profile.is_mature());
    }

    #[test]
    fn test_text_preferences() {
        let mut profile = EditorProfile::default();
        profile.observe_edit(&EditOperation::TextPlacement {
            font: "Helvetica".into(),
            size: 32.0,
            y_position: 0.9,
            alignment: "center".into(),
        });

        assert_eq!(profile.text_style.observation_count, 1);
        assert!(profile.text_style.preferred_fonts.contains(&"Helvetica".to_string()));
    }
}
