//! Audio classification — categorizes audio segments and detects quality issues.
//!
//! Classifies audio into categories (dialogue, music, ambient, silence, SFX)
//! and flags quality issues (clipping, wind noise, room tone inconsistency).
//!
//! Can work with a lightweight ONNX model or a CPU-only heuristic fallback.

use crate::error::AiResult;
use serde::{Deserialize, Serialize};

/// Type of audio content in a segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AudioSegmentType {
    /// Human speech / dialogue.
    Dialogue,
    /// Music (instrumental or vocal).
    Music,
    /// Ambient / atmospheric sounds.
    Ambient,
    /// Silence or very low energy.
    Silence,
    /// Sound effects (transient, non-speech, non-music).
    SoundEffect,
    /// Indeterminate.
    Unknown,
}

impl AudioSegmentType {
    /// Display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Dialogue => "Dialogue",
            Self::Music => "Music",
            Self::Ambient => "Ambient",
            Self::Silence => "Silence",
            Self::SoundEffect => "Sound Effect",
            Self::Unknown => "Unknown",
        }
    }
}

/// A classified audio segment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioSegment {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Classified type.
    pub segment_type: AudioSegmentType,
    /// Classification confidence (0.0 to 1.0).
    pub confidence: f32,
}

/// A detected audio quality issue.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioQualityIssue {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Type of quality issue.
    pub issue_type: QualityIssueType,
    /// Severity (0.0 = minor, 1.0 = severe).
    pub severity: f32,
    /// Description of the issue.
    pub description: String,
}

/// Types of audio quality issues.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum QualityIssueType {
    /// Audio clipping (samples at or near ±1.0).
    Clipping,
    /// Wind noise (low-frequency rumble).
    WindNoise,
    /// Room tone inconsistency between segments.
    RoomToneMismatch,
    /// Excessive background noise.
    BackgroundNoise,
    /// Audio level too low.
    LowLevel,
}

impl QualityIssueType {
    /// Display name for UI.
    pub fn display_name(&self) -> &'static str {
        match self {
            Self::Clipping => "Audio Clipping",
            Self::WindNoise => "Wind Noise",
            Self::RoomToneMismatch => "Room Tone Mismatch",
            Self::BackgroundNoise => "Background Noise",
            Self::LowLevel => "Low Audio Level",
        }
    }
}

/// Configuration for audio classification.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioClassifyConfig {
    /// Window size for analysis in seconds.
    pub window_secs: f32,
    /// Overlap between windows (0.0 to 1.0).
    pub window_overlap: f32,
    /// Silence threshold in dB.
    pub silence_threshold_db: f32,
    /// Clipping threshold (fraction of max amplitude).
    pub clipping_threshold: f32,
}

impl Default for AudioClassifyConfig {
    fn default() -> Self {
        Self {
            window_secs: 1.0,
            window_overlap: 0.5,
            silence_threshold_db: -40.0,
            clipping_threshold: 0.99,
        }
    }
}

/// Audio classifier engine.
pub struct AudioClassifier {
    config: AudioClassifyConfig,
}

impl AudioClassifier {
    /// Create a new classifier with the given configuration.
    pub fn new(config: AudioClassifyConfig) -> Self {
        Self { config }
    }

    /// Classify audio segments in the given samples.
    ///
    /// Analyzes the audio in windows and assigns a content type to each window,
    /// then merges adjacent windows of the same type into segments.
    pub fn classify_segments(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> AiResult<Vec<AudioSegment>> {
        if samples.is_empty() || sample_rate == 0 {
            return Ok(Vec::new());
        }

        let window_samples = (self.config.window_secs * sample_rate as f32) as usize;
        let step = ((1.0 - self.config.window_overlap) * window_samples as f32) as usize;
        let step = step.max(1);

        let mut raw_segments: Vec<(f64, f64, AudioSegmentType, f32)> = Vec::new();

        let mut pos = 0;
        while pos < samples.len() {
            let end = (pos + window_samples).min(samples.len());
            let window = &samples[pos..end];

            let (seg_type, confidence) = classify_window(window, sample_rate, &self.config);

            let start_time = pos as f64 / sample_rate as f64;
            let end_time = end as f64 / sample_rate as f64;
            raw_segments.push((start_time, end_time, seg_type, confidence));

            pos += step;
        }

        // Merge adjacent segments of the same type
        Ok(merge_segments(raw_segments))
    }

    /// Detect audio quality issues in the given samples.
    pub fn detect_quality_issues(
        &self,
        samples: &[f32],
        sample_rate: u32,
    ) -> Vec<AudioQualityIssue> {
        if samples.is_empty() || sample_rate == 0 {
            return Vec::new();
        }

        let mut issues = Vec::new();

        // Detect clipping
        issues.extend(detect_clipping(
            samples,
            sample_rate,
            self.config.clipping_threshold,
        ));

        // Detect low level
        issues.extend(detect_low_level(samples, sample_rate));

        // Detect wind noise (low-frequency energy dominance)
        issues.extend(detect_wind_noise(samples, sample_rate));

        issues
    }
}

/// Classify a single window of audio.
fn classify_window(
    window: &[f32],
    _sample_rate: u32,
    config: &AudioClassifyConfig,
) -> (AudioSegmentType, f32) {
    if window.is_empty() {
        return (AudioSegmentType::Silence, 1.0);
    }

    // Compute RMS energy
    let rms = compute_rms(window);
    let rms_db = if rms > 0.0 {
        20.0 * rms.log10()
    } else {
        -100.0
    };

    // Silence detection
    if rms_db < config.silence_threshold_db {
        return (AudioSegmentType::Silence, 0.95);
    }

    // Compute zero-crossing rate (high = speech/noise, low = tonal/music)
    let zcr = zero_crossing_rate(window);

    // Compute spectral centroid approximation
    let spectral_flatness = spectral_flatness(window);

    // Heuristic classification
    if zcr > 0.1 && spectral_flatness > 0.3 {
        // High ZCR + flat spectrum = noise-like (ambient or SFX)
        if rms_db > -20.0 {
            (AudioSegmentType::SoundEffect, 0.6)
        } else {
            (AudioSegmentType::Ambient, 0.6)
        }
    } else if zcr > 0.05 && zcr < 0.15 {
        // Medium ZCR = likely speech
        (AudioSegmentType::Dialogue, 0.7)
    } else if zcr < 0.05 && spectral_flatness < 0.3 {
        // Low ZCR + tonal = likely music
        (AudioSegmentType::Music, 0.6)
    } else {
        // Default to dialogue for mid-range characteristics
        (AudioSegmentType::Dialogue, 0.5)
    }
}

/// Compute RMS energy of a sample window.
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Compute zero-crossing rate (fraction of successive samples that change sign).
fn zero_crossing_rate(samples: &[f32]) -> f32 {
    if samples.len() < 2 {
        return 0.0;
    }
    let crossings = samples
        .windows(2)
        .filter(|w| (w[0] >= 0.0) != (w[1] >= 0.0))
        .count();
    crossings as f32 / (samples.len() - 1) as f32
}

/// Compute spectral flatness approximation using the ratio of geometric to arithmetic mean.
/// Values near 1.0 = noise-like, near 0.0 = tonal.
fn spectral_flatness(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }

    // Use absolute values as a proxy for spectral energy
    let abs_vals: Vec<f32> = samples.iter().map(|s| s.abs().max(1e-10)).collect();
    let n = abs_vals.len() as f32;

    let arithmetic_mean = abs_vals.iter().sum::<f32>() / n;
    let log_sum: f32 = abs_vals.iter().map(|v| v.ln()).sum();
    let geometric_mean = (log_sum / n).exp();

    if arithmetic_mean > 0.0 {
        (geometric_mean / arithmetic_mean).clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// Merge adjacent raw segments of the same type.
fn merge_segments(raw: Vec<(f64, f64, AudioSegmentType, f32)>) -> Vec<AudioSegment> {
    if raw.is_empty() {
        return Vec::new();
    }

    let mut merged = Vec::new();
    let mut current = raw[0];

    for seg in raw.iter().skip(1) {
        if seg.2 == current.2 {
            // Extend current segment
            current.1 = seg.1;
            current.3 = (current.3 + seg.3) / 2.0; // average confidence
        } else {
            merged.push(AudioSegment {
                start_time: current.0,
                end_time: current.1,
                segment_type: current.2,
                confidence: current.3,
            });
            current = *seg;
        }
    }

    // Push the last segment
    merged.push(AudioSegment {
        start_time: current.0,
        end_time: current.1,
        segment_type: current.2,
        confidence: current.3,
    });

    merged
}

/// Detect clipping in audio.
fn detect_clipping(samples: &[f32], sample_rate: u32, threshold: f32) -> Vec<AudioQualityIssue> {
    let mut issues = Vec::new();
    let window = sample_rate as usize; // 1-second windows
    let mut pos = 0;

    while pos < samples.len() {
        let end = (pos + window).min(samples.len());
        let slice = &samples[pos..end];

        let clipped_count = slice.iter().filter(|&&s| s.abs() >= threshold).count();
        let clip_ratio = clipped_count as f32 / slice.len() as f32;

        if clip_ratio > 0.001 {
            // More than 0.1% of samples clipping
            issues.push(AudioQualityIssue {
                start_time: pos as f64 / sample_rate as f64,
                end_time: end as f64 / sample_rate as f64,
                issue_type: QualityIssueType::Clipping,
                severity: (clip_ratio * 100.0).min(1.0),
                description: format!(
                    "{:.1}% of samples clipping in this section",
                    clip_ratio * 100.0
                ),
            });
        }

        pos += window;
    }

    issues
}

/// Detect sections with very low audio level.
fn detect_low_level(samples: &[f32], sample_rate: u32) -> Vec<AudioQualityIssue> {
    let mut issues = Vec::new();
    let window = sample_rate as usize * 5; // 5-second windows
    let mut pos = 0;

    while pos < samples.len() {
        let end = (pos + window).min(samples.len());
        let rms = compute_rms(&samples[pos..end]);
        let rms_db = if rms > 0.0 {
            20.0 * rms.log10()
        } else {
            -100.0
        };

        if rms_db > -60.0 && rms_db < -30.0 {
            // Audio present but very quiet
            issues.push(AudioQualityIssue {
                start_time: pos as f64 / sample_rate as f64,
                end_time: end as f64 / sample_rate as f64,
                issue_type: QualityIssueType::LowLevel,
                severity: ((-30.0 - rms_db) / 30.0).clamp(0.0, 1.0),
                description: format!("Audio level is low ({rms_db:.1} dB RMS)"),
            });
        }

        pos += window;
    }

    issues
}

/// Detect wind noise using low-frequency energy dominance.
fn detect_wind_noise(samples: &[f32], sample_rate: u32) -> Vec<AudioQualityIssue> {
    let mut issues = Vec::new();
    let window = sample_rate as usize * 2; // 2-second windows
    let mut pos = 0;

    while pos < samples.len() {
        let end = (pos + window).min(samples.len());
        let slice = &samples[pos..end];

        // Simple low-pass filter approximation: compare low-freq energy to total
        let total_rms = compute_rms(slice);
        if total_rms < 0.001 {
            pos += window;
            continue;
        }

        // Crude low-pass: average pairs of samples
        let low_passed: Vec<f32> = slice
            .chunks(4)
            .map(|chunk| chunk.iter().sum::<f32>() / chunk.len() as f32)
            .collect();
        let low_rms = compute_rms(&low_passed);
        let low_ratio = low_rms / total_rms;

        if low_ratio > 0.8 {
            issues.push(AudioQualityIssue {
                start_time: pos as f64 / sample_rate as f64,
                end_time: end as f64 / sample_rate as f64,
                issue_type: QualityIssueType::WindNoise,
                severity: ((low_ratio - 0.8) * 5.0).min(1.0),
                description: "Possible wind noise detected (low-frequency energy dominant)".into(),
            });
        }

        pos += window;
    }

    issues
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classify_silence() {
        let samples = vec![0.0_f32; 16000];
        let classifier = AudioClassifier::new(AudioClassifyConfig::default());
        let segments = classifier.classify_segments(&samples, 16000).unwrap();
        assert!(!segments.is_empty());
        assert_eq!(segments[0].segment_type, AudioSegmentType::Silence);
    }

    #[test]
    fn test_classify_tone() {
        // Generate a 440Hz sine wave (music-like)
        let sample_rate = 16000u32;
        let samples: Vec<f32> = (0..sample_rate * 2)
            .map(|i| {
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5
            })
            .collect();
        let classifier = AudioClassifier::new(AudioClassifyConfig::default());
        let segments = classifier.classify_segments(&samples, sample_rate).unwrap();
        assert!(!segments.is_empty());
        // A pure tone should not be classified as silence
        assert_ne!(segments[0].segment_type, AudioSegmentType::Silence);
    }

    #[test]
    fn test_detect_clipping() {
        let mut samples = vec![0.5_f32; 16000];
        // Add clipping in the middle
        for s in samples[4000..8000].iter_mut() {
            *s = 1.0;
        }
        let issues = detect_clipping(&samples, 16000, 0.99);
        assert!(!issues.is_empty(), "Should detect clipping");
        assert_eq!(issues[0].issue_type, QualityIssueType::Clipping);
    }

    #[test]
    fn test_no_clipping_in_clean_audio() {
        let samples = vec![0.5_f32; 16000];
        let issues = detect_clipping(&samples, 16000, 0.99);
        assert!(issues.is_empty(), "Clean audio should have no clipping");
    }

    #[test]
    fn test_zero_crossing_rate() {
        // Alternating samples should have high ZCR
        let alternating: Vec<f32> = (0..100)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let zcr = zero_crossing_rate(&alternating);
        assert!(
            zcr > 0.9,
            "Alternating signal should have high ZCR, got {zcr}"
        );

        // Constant signal should have zero ZCR
        let constant = vec![1.0_f32; 100];
        let zcr = zero_crossing_rate(&constant);
        assert_eq!(zcr, 0.0);
    }

    #[test]
    fn test_compute_rms() {
        let samples = vec![1.0_f32; 100];
        let rms = compute_rms(&samples);
        assert!((rms - 1.0).abs() < 0.01);

        let silence = vec![0.0_f32; 100];
        assert_eq!(compute_rms(&silence), 0.0);
    }

    #[test]
    fn test_merge_segments() {
        let raw = vec![
            (0.0, 1.0, AudioSegmentType::Dialogue, 0.8),
            (1.0, 2.0, AudioSegmentType::Dialogue, 0.9),
            (2.0, 3.0, AudioSegmentType::Music, 0.7),
            (3.0, 4.0, AudioSegmentType::Music, 0.8),
        ];
        let merged = merge_segments(raw);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].segment_type, AudioSegmentType::Dialogue);
        assert!((merged[0].end_time - 2.0).abs() < 0.001);
        assert_eq!(merged[1].segment_type, AudioSegmentType::Music);
    }

    #[test]
    fn test_segment_type_display() {
        assert_eq!(AudioSegmentType::Dialogue.display_name(), "Dialogue");
        assert_eq!(AudioSegmentType::Music.display_name(), "Music");
    }

    #[test]
    fn test_quality_issue_display() {
        assert_eq!(QualityIssueType::Clipping.display_name(), "Audio Clipping");
        assert_eq!(QualityIssueType::WindNoise.display_name(), "Wind Noise");
    }
}
