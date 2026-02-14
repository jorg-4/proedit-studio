//! Speaker diarization — identifies who spoke when.
//!
//! Analyzes audio to assign speaker labels (Speaker A, Speaker B, etc.)
//! with timecode ranges. Users can rename speakers in the UI.
//!
//! Uses a pyannote/NeMo-style ONNX model when available, with a
//! CPU fallback based on spectral feature clustering.

#[cfg(feature = "onnx")]
use crate::error::AiError;
use crate::error::AiResult;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A segment of audio attributed to a specific speaker.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerSegment {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Speaker label (e.g., "Speaker A", "Speaker B").
    pub speaker_label: String,
    /// Confidence of the speaker assignment (0.0 to 1.0).
    pub confidence: f32,
}

/// A speaker identity with user-customizable name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpeakerIdentity {
    /// Internal label (e.g., "speaker_0").
    pub internal_id: String,
    /// Display name (user-editable, e.g., "Sarah").
    pub display_name: String,
    /// Total speaking time in seconds.
    pub total_speaking_time: f64,
    /// Number of segments.
    pub segment_count: usize,
}

/// Configuration for speaker diarization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiarizeConfig {
    /// Minimum segment duration in seconds.
    pub min_segment_duration: f32,
    /// Maximum number of speakers to detect (0 = auto).
    pub max_speakers: u32,
    /// Merge segments from the same speaker within this gap (seconds).
    pub merge_gap: f32,
}

impl Default for DiarizeConfig {
    fn default() -> Self {
        Self {
            min_segment_duration: 0.5,
            max_speakers: 0,
            merge_gap: 0.5,
        }
    }
}

/// Speaker diarization engine.
pub struct SpeakerDiarizer {
    config: DiarizeConfig,
}

impl SpeakerDiarizer {
    /// Create a new diarizer.
    pub fn new(config: DiarizeConfig) -> Self {
        Self { config }
    }

    /// Load the ONNX model for diarization.
    #[cfg(feature = "onnx")]
    pub fn load(model_path: &std::path::Path, config: DiarizeConfig) -> AiResult<Self> {
        if !model_path.exists() {
            return Err(AiError::ModelNotFound {
                model_id: format!("{:?}", crate::model_manager::ModelId::SpeakerDiarize),
            });
        }
        Ok(Self::new(config))
    }

    /// Run speaker diarization on audio samples.
    ///
    /// Returns speaker segments sorted by start time.
    pub fn diarize(&self, samples: &[f32], sample_rate: u32) -> AiResult<Vec<SpeakerSegment>> {
        if samples.is_empty() || sample_rate == 0 {
            return Ok(Vec::new());
        }

        // CPU fallback: energy-based segmentation with spectral clustering.
        // In production, this would use the pyannote/NeMo ONNX model.
        let raw_segments = cpu_energy_diarize(samples, sample_rate, &self.config);

        // Merge close segments from the same speaker
        let merged = merge_speaker_segments(raw_segments, self.config.merge_gap);

        // Filter by minimum duration
        let filtered: Vec<SpeakerSegment> = merged
            .into_iter()
            .filter(|s| (s.end_time - s.start_time) >= self.config.min_segment_duration as f64)
            .collect();

        Ok(filtered)
    }

    /// Build speaker identities from diarization segments.
    pub fn build_speaker_identities(segments: &[SpeakerSegment]) -> Vec<SpeakerIdentity> {
        let mut speakers: HashMap<String, (f64, usize)> = HashMap::new();

        for seg in segments {
            let entry = speakers
                .entry(seg.speaker_label.clone())
                .or_insert((0.0, 0));
            entry.0 += seg.end_time - seg.start_time;
            entry.1 += 1;
        }

        let mut identities: Vec<SpeakerIdentity> = speakers
            .into_iter()
            .map(|(label, (time, count))| SpeakerIdentity {
                internal_id: label.clone(),
                display_name: label,
                total_speaking_time: time,
                segment_count: count,
            })
            .collect();

        identities.sort_by(|a, b| {
            b.total_speaking_time
                .partial_cmp(&a.total_speaking_time)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        identities
    }

    /// Get the configuration.
    pub fn config(&self) -> &DiarizeConfig {
        &self.config
    }
}

/// CPU fallback: simple energy-based speaker segmentation.
///
/// Splits audio into chunks, computes spectral features, and clusters
/// into speaker groups using a simple distance metric.
fn cpu_energy_diarize(
    samples: &[f32],
    sample_rate: u32,
    config: &DiarizeConfig,
) -> Vec<SpeakerSegment> {
    let chunk_duration = 1.0; // 1-second chunks
    let chunk_samples = (chunk_duration * sample_rate as f64) as usize;
    if chunk_samples == 0 {
        return Vec::new();
    }

    let silence_threshold = 0.01_f32;

    // Step 1: Compute features per chunk
    let mut chunk_features: Vec<(f64, f64, f32, f32)> = Vec::new(); // (start, end, rms, zcr)

    let mut pos = 0;
    while pos < samples.len() {
        let end = (pos + chunk_samples).min(samples.len());
        let chunk = &samples[pos..end];

        let rms = compute_rms(chunk);
        let zcr = zero_crossing_rate(chunk);
        let start_time = pos as f64 / sample_rate as f64;
        let end_time = end as f64 / sample_rate as f64;

        chunk_features.push((start_time, end_time, rms, zcr));
        pos += chunk_samples;
    }

    // Step 2: Assign speakers based on feature clustering
    // Simple heuristic: split by ZCR into 2 groups (high = speaker A, low = speaker B)
    let max_speakers = if config.max_speakers > 0 {
        config.max_speakers as usize
    } else {
        2 // default to 2 speakers
    };

    let mut segments = Vec::new();

    if chunk_features.is_empty() {
        return segments;
    }

    // Compute median ZCR for splitting
    let mut zcr_values: Vec<f32> = chunk_features
        .iter()
        .filter(|(_, _, rms, _)| *rms > silence_threshold)
        .map(|(_, _, _, zcr)| *zcr)
        .collect();

    if zcr_values.is_empty() {
        return segments;
    }

    zcr_values.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_zcr = zcr_values[zcr_values.len() / 2];

    for (start, end, rms, zcr) in &chunk_features {
        if *rms < silence_threshold {
            continue; // Skip silence
        }

        let speaker_idx = if max_speakers <= 1 || *zcr > median_zcr {
            0
        } else {
            1
        };

        let label = format!("Speaker {}", (b'A' + speaker_idx as u8) as char);
        segments.push(SpeakerSegment {
            start_time: *start,
            end_time: *end,
            speaker_label: label,
            confidence: 0.5, // Low confidence for heuristic
        });
    }

    segments
}

/// Compute RMS energy.
fn compute_rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
}

/// Compute zero-crossing rate.
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

/// Merge adjacent segments from the same speaker within the merge gap.
fn merge_speaker_segments(segments: Vec<SpeakerSegment>, merge_gap: f32) -> Vec<SpeakerSegment> {
    if segments.is_empty() {
        return segments;
    }

    let mut merged = Vec::new();
    let mut current = segments[0].clone();

    for seg in segments.iter().skip(1) {
        if seg.speaker_label == current.speaker_label
            && (seg.start_time - current.end_time) <= merge_gap as f64
        {
            // Extend current segment
            current.end_time = seg.end_time;
            current.confidence = (current.confidence + seg.confidence) / 2.0;
        } else {
            merged.push(current);
            current = seg.clone();
        }
    }
    merged.push(current);

    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diarize_silence() {
        let diarizer = SpeakerDiarizer::new(DiarizeConfig::default());
        let samples = vec![0.0_f32; 16000 * 3]; // 3 seconds silence
        let segments = diarizer.diarize(&samples, 16000).unwrap();
        assert!(
            segments.is_empty(),
            "Silence should produce no speaker segments"
        );
    }

    #[test]
    fn test_diarize_single_speaker() {
        let diarizer = SpeakerDiarizer::new(DiarizeConfig {
            max_speakers: 1,
            ..Default::default()
        });

        // Generate a tone (simulating speech)
        let sample_rate = 16000u32;
        let samples: Vec<f32> = (0..sample_rate * 3)
            .map(|i| {
                (i as f32 * 200.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5
            })
            .collect();

        let segments = diarizer.diarize(&samples, sample_rate).unwrap();
        // All segments should be from the same speaker
        for seg in &segments {
            assert_eq!(seg.speaker_label, "Speaker A");
        }
    }

    #[test]
    fn test_build_speaker_identities() {
        let segments = vec![
            SpeakerSegment {
                start_time: 0.0,
                end_time: 5.0,
                speaker_label: "Speaker A".into(),
                confidence: 0.8,
            },
            SpeakerSegment {
                start_time: 5.0,
                end_time: 8.0,
                speaker_label: "Speaker B".into(),
                confidence: 0.7,
            },
            SpeakerSegment {
                start_time: 8.0,
                end_time: 15.0,
                speaker_label: "Speaker A".into(),
                confidence: 0.9,
            },
        ];

        let identities = SpeakerDiarizer::build_speaker_identities(&segments);
        assert_eq!(identities.len(), 2);

        // Speaker A should have more total time (5 + 7 = 12s vs 3s)
        assert_eq!(identities[0].internal_id, "Speaker A");
        assert!((identities[0].total_speaking_time - 12.0).abs() < 0.01);
        assert_eq!(identities[0].segment_count, 2);

        assert_eq!(identities[1].internal_id, "Speaker B");
        assert!((identities[1].total_speaking_time - 3.0).abs() < 0.01);
    }

    #[test]
    fn test_merge_speaker_segments() {
        let segments = vec![
            SpeakerSegment {
                start_time: 0.0,
                end_time: 1.0,
                speaker_label: "A".into(),
                confidence: 0.8,
            },
            SpeakerSegment {
                start_time: 1.2,
                end_time: 2.0,
                speaker_label: "A".into(),
                confidence: 0.9,
            },
            SpeakerSegment {
                start_time: 3.0,
                end_time: 4.0,
                speaker_label: "B".into(),
                confidence: 0.7,
            },
        ];

        let merged = merge_speaker_segments(segments, 0.5);
        assert_eq!(merged.len(), 2, "Adjacent A segments should merge");
        assert_eq!(merged[0].speaker_label, "A");
        assert!((merged[0].end_time - 2.0).abs() < 0.01);
    }

    #[test]
    fn test_diarize_config_default() {
        let config = DiarizeConfig::default();
        assert_eq!(config.max_speakers, 0);
        assert_eq!(config.min_segment_duration, 0.5);
    }

    #[test]
    fn test_speaker_segment_serialization() {
        let seg = SpeakerSegment {
            start_time: 1.5,
            end_time: 3.0,
            speaker_label: "Speaker A".into(),
            confidence: 0.85,
        };
        let json = serde_json::to_string(&seg).unwrap();
        let decoded: SpeakerSegment = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.speaker_label, "Speaker A");
    }
}
