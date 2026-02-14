//! Filler word detection and silence detection.
//!
//! Operates on existing Whisper transcript data to identify hesitations,
//! verbal fillers, and word repetitions. Also detects silence regions
//! from raw audio samples using RMS energy analysis.

use crate::transcribe::Transcript;
use serde::{Deserialize, Serialize};

/// A region of audio containing a filler word.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FillerRegion {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// The detected filler word.
    pub word: String,
    /// Category of filler.
    pub category: FillerCategory,
}

/// Category of filler word.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum FillerCategory {
    /// Hesitation sounds: um, uh, er, ah.
    Hesitation,
    /// Verbal fillers: like, you know, basically, actually.
    Verbal,
    /// Word repetitions: "I I", "the the".
    Repetition,
}

/// A detected region of silence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SilenceRegion {
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Duration in seconds.
    pub duration: f64,
}

/// Configuration for filler word detection.
pub struct FillerDetectConfig {
    /// Detect hesitation sounds (um, uh, etc.).
    pub detect_hesitations: bool,
    /// Detect verbal fillers (like, basically, etc.).
    pub detect_verbal_fillers: bool,
    /// Detect word repetitions (I I, the the, etc.).
    pub detect_repetitions: bool,
    /// Additional words to flag as fillers.
    pub custom_filler_words: Vec<String>,
}

impl Default for FillerDetectConfig {
    fn default() -> Self {
        Self {
            detect_hesitations: true,
            detect_verbal_fillers: true,
            detect_repetitions: true,
            custom_filler_words: Vec::new(),
        }
    }
}

/// Known hesitation sounds.
const HESITATIONS: &[&str] = &["um", "uh", "er", "ah", "hmm", "hm", "erm"];

/// Known verbal filler words/phrases.
const VERBAL_FILLERS: &[&str] = &[
    "like",
    "you know",
    "basically",
    "actually",
    "sort of",
    "kind of",
    "right",
    "so",
    "well",
    "just",
    "literally",
    "i mean",
    "you see",
];

/// Detect filler words in a transcript.
///
/// Scans each word for hesitations, verbal fillers, and repetitions.
/// Returns regions sorted by start time.
pub fn detect_filler_words(
    transcript: &Transcript,
    config: &FillerDetectConfig,
) -> Vec<FillerRegion> {
    let mut fillers = Vec::new();

    for (i, word) in transcript.words.iter().enumerate() {
        let normalized = word.text.to_lowercase();
        let trimmed = normalized.trim();

        // Check hesitation sounds
        if config.detect_hesitations && HESITATIONS.contains(&trimmed) {
            fillers.push(FillerRegion {
                start_time: word.start_time,
                end_time: word.end_time,
                word: trimmed.to_string(),
                category: FillerCategory::Hesitation,
            });
            continue; // Don't double-flag
        }

        // Check verbal fillers
        if config.detect_verbal_fillers && VERBAL_FILLERS.contains(&trimmed) {
            fillers.push(FillerRegion {
                start_time: word.start_time,
                end_time: word.end_time,
                word: trimmed.to_string(),
                category: FillerCategory::Verbal,
            });
            continue;
        }

        // Check custom filler words
        if config
            .custom_filler_words
            .iter()
            .any(|w| w.to_lowercase() == trimmed)
        {
            fillers.push(FillerRegion {
                start_time: word.start_time,
                end_time: word.end_time,
                word: trimmed.to_string(),
                category: FillerCategory::Verbal,
            });
            continue;
        }

        // Check repetitions (same word as previous)
        if config.detect_repetitions && i > 0 {
            let prev_normalized = transcript.words[i - 1].text.to_lowercase();
            let prev_trimmed = prev_normalized.trim();
            if trimmed == prev_trimmed && !trimmed.is_empty() {
                fillers.push(FillerRegion {
                    start_time: word.start_time,
                    end_time: word.end_time,
                    word: trimmed.to_string(),
                    category: FillerCategory::Repetition,
                });
            }
        }
    }

    // Already sorted by start_time since we iterate in order
    fillers
}

/// Detect silence regions in audio using RMS energy.
///
/// Analyzes audio in ~50ms windows and identifies regions where the
/// RMS energy falls below the given dB threshold for at least
/// `min_duration_secs`.
pub fn detect_silence(
    audio_samples: &[f32],
    sample_rate: u32,
    threshold_db: f32,
    min_duration_secs: f32,
) -> Vec<SilenceRegion> {
    if audio_samples.is_empty() || sample_rate == 0 {
        return Vec::new();
    }

    // Convert threshold from dB to linear amplitude
    let threshold_linear = 10.0_f32.powf(threshold_db / 20.0);

    // Window size: ~50ms
    let window_size = (sample_rate as usize) / 20;
    if window_size == 0 {
        return Vec::new();
    }

    // Classify each window as silent or not
    let mut silent_start: Option<f64> = None;
    let mut regions = Vec::new();

    let total_windows = audio_samples.len() / window_size;

    for window_idx in 0..=total_windows {
        let start_sample = window_idx * window_size;
        let end_sample = (start_sample + window_size).min(audio_samples.len());
        let window = &audio_samples[start_sample..end_sample];

        if window.is_empty() {
            continue;
        }

        // Compute RMS
        let sum_sq: f32 = window.iter().map(|s| s * s).sum();
        let rms = (sum_sq / window.len() as f32).sqrt();

        let is_silent = rms < threshold_linear;
        let current_time = start_sample as f64 / sample_rate as f64;

        if is_silent {
            if silent_start.is_none() {
                silent_start = Some(current_time);
            }
        } else if let Some(start) = silent_start {
            let duration = current_time - start;
            if duration >= min_duration_secs as f64 {
                regions.push(SilenceRegion {
                    start_time: start,
                    end_time: current_time,
                    duration,
                });
            }
            silent_start = None;
        }
    }

    // Handle silence extending to end of audio
    if let Some(start) = silent_start {
        let end_time = audio_samples.len() as f64 / sample_rate as f64;
        let duration = end_time - start;
        if duration >= min_duration_secs as f64 {
            regions.push(SilenceRegion {
                start_time: start,
                end_time,
                duration,
            });
        }
    }

    regions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transcribe::TranscriptWord;

    fn make_transcript(words: &[(&str, f64, f64)]) -> Transcript {
        Transcript {
            words: words
                .iter()
                .map(|(text, start, end)| TranscriptWord {
                    text: text.to_string(),
                    start_time: *start,
                    end_time: *end,
                    confidence: 1.0,
                })
                .collect(),
            language: "en".into(),
            duration_secs: words.last().map(|w| w.2).unwrap_or(0.0),
        }
    }

    #[test]
    fn test_detect_hesitation_fillers() {
        let t = make_transcript(&[
            ("so", 0.0, 0.2),
            ("um", 0.3, 0.5),
            ("I", 0.6, 0.7),
            ("think", 0.8, 1.0),
            ("uh", 1.1, 1.3),
            ("yes", 1.4, 1.6),
        ]);
        let config = FillerDetectConfig::default();
        let fillers = detect_filler_words(&t, &config);

        let hesitations: Vec<_> = fillers
            .iter()
            .filter(|f| f.category == FillerCategory::Hesitation)
            .collect();
        assert_eq!(hesitations.len(), 2);
        assert_eq!(hesitations[0].word, "um");
        assert_eq!(hesitations[1].word, "uh");
    }

    #[test]
    fn test_detect_verbal_fillers() {
        let t = make_transcript(&[
            ("I", 0.0, 0.2),
            ("like", 0.3, 0.5),
            ("think", 0.6, 0.8),
            ("basically", 0.9, 1.3),
            ("yes", 1.4, 1.6),
        ]);
        let config = FillerDetectConfig::default();
        let fillers = detect_filler_words(&t, &config);

        let verbal: Vec<_> = fillers
            .iter()
            .filter(|f| f.category == FillerCategory::Verbal)
            .collect();
        assert!(verbal.len() >= 2);
    }

    #[test]
    fn test_detect_repetitions() {
        let t = make_transcript(&[
            ("I", 0.0, 0.2),
            ("I", 0.3, 0.5),
            ("think", 0.6, 0.8),
            ("the", 0.9, 1.0),
            ("the", 1.1, 1.2),
            ("answer", 1.3, 1.6),
        ]);
        let config = FillerDetectConfig::default();
        let fillers = detect_filler_words(&t, &config);

        let reps: Vec<_> = fillers
            .iter()
            .filter(|f| f.category == FillerCategory::Repetition)
            .collect();
        assert_eq!(reps.len(), 2);
    }

    #[test]
    fn test_no_fillers_in_clean_speech() {
        let t = make_transcript(&[
            ("the", 0.0, 0.2),
            ("quick", 0.3, 0.5),
            ("brown", 0.6, 0.8),
            ("fox", 0.9, 1.0),
        ]);
        let config = FillerDetectConfig::default();
        let fillers = detect_filler_words(&t, &config);
        assert!(fillers.is_empty(), "Clean speech should have no fillers");
    }

    #[test]
    fn test_detect_silence_in_audio() {
        let sample_rate = 16000u32;
        let mut samples = Vec::new();

        // 1 second of 440Hz sine wave
        for i in 0..sample_rate {
            samples.push(
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5,
            );
        }
        // 1 second of silence
        samples.resize(samples.len() + sample_rate as usize, 0.0);
        // 1 second of tone
        for i in 0..sample_rate {
            samples.push(
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5,
            );
        }

        let silences = detect_silence(&samples, sample_rate, -40.0, 0.5);
        assert_eq!(silences.len(), 1, "Should detect exactly 1 silence region");
        assert!(
            (silences[0].start_time - 1.0).abs() < 0.1,
            "Silence should start near 1.0s, got {}",
            silences[0].start_time
        );
        assert!(
            (silences[0].duration - 1.0).abs() < 0.1,
            "Silence should be ~1.0s long, got {}",
            silences[0].duration
        );
    }

    #[test]
    fn test_no_silence_in_continuous_audio() {
        let sample_rate = 16000u32;
        let samples: Vec<f32> = (0..sample_rate * 3)
            .map(|i| {
                (i as f32 * 440.0 * 2.0 * std::f32::consts::PI / sample_rate as f32).sin() * 0.5
            })
            .collect();

        let silences = detect_silence(&samples, sample_rate, -40.0, 0.5);
        assert!(
            silences.is_empty(),
            "Continuous tone should have no silence"
        );
    }

    #[test]
    fn test_custom_filler_words() {
        let t = make_transcript(&[("and", 0.0, 0.2), ("stuff", 0.3, 0.5), ("things", 0.6, 0.8)]);
        let config = FillerDetectConfig {
            custom_filler_words: vec!["stuff".to_string()],
            ..Default::default()
        };
        let fillers = detect_filler_words(&t, &config);
        assert_eq!(fillers.len(), 1);
        assert_eq!(fillers[0].word, "stuff");
    }
}
