//! Audio transcription using whisper.cpp as a sidecar process.
//!
//! Extracts audio from video via FFmpeg, then runs whisper.cpp for
//! word-level speech-to-text with timestamps.

use crate::error::{AiError, AiResult};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::process::Command;
use tracing::{debug, info, warn};

/// A complete transcript with word-level timing.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transcript {
    /// Individual words with timestamps.
    pub words: Vec<TranscriptWord>,
    /// Detected or specified language code.
    pub language: String,
    /// Total audio duration in seconds.
    pub duration_secs: f64,
}

/// A single word in a transcript with timing information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranscriptWord {
    /// The transcribed text.
    pub text: String,
    /// Start time in seconds.
    pub start_time: f64,
    /// End time in seconds.
    pub end_time: f64,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
}

/// Configuration for the transcription engine.
pub struct TranscriberConfig {
    /// Path to the whisper.cpp binary.
    pub whisper_binary: PathBuf,
    /// Path to the whisper model file (.bin).
    pub model_path: PathBuf,
    /// Language code (None = auto-detect).
    pub language: Option<String>,
    /// Number of threads to use.
    pub threads: u32,
}

impl Default for TranscriberConfig {
    fn default() -> Self {
        Self {
            whisper_binary: Self::find_whisper_binary(),
            model_path: PathBuf::new(),
            language: None,
            threads: num_cpus::get() as u32,
        }
    }
}

impl TranscriberConfig {
    /// Search PATH for a whisper.cpp binary.
    pub fn find_whisper_binary() -> PathBuf {
        for name in &["whisper-cpp", "whisper", "main"] {
            if which::which(name).is_ok() {
                return PathBuf::from(name);
            }
        }
        PathBuf::from("whisper-cpp")
    }
}

/// Whisper.cpp-based transcription engine.
pub struct Transcriber {
    config: TranscriberConfig,
}

impl Transcriber {
    /// Create a new transcriber with the given configuration.
    pub fn new(config: TranscriberConfig) -> Self {
        Self { config }
    }

    /// Check if whisper.cpp is available on the system.
    pub fn is_available(&self) -> bool {
        Command::new(&self.config.whisper_binary)
            .arg("--help")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .is_ok()
    }

    /// Transcribe an audio file (WAV format, 16kHz mono preferred).
    ///
    /// Runs whisper.cpp as a subprocess and parses the JSON output.
    pub fn transcribe(&self, audio_path: &Path) -> AiResult<Transcript> {
        if !audio_path.exists() {
            return Err(AiError::Io(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!("Audio file not found: {}", audio_path.display()),
            )));
        }

        if self.config.model_path.as_os_str().is_empty() || !self.config.model_path.exists() {
            return Err(AiError::ModelNotLoaded);
        }

        info!(
            audio = %audio_path.display(),
            model = %self.config.model_path.display(),
            "Starting transcription"
        );

        // Build whisper.cpp command
        let mut cmd = Command::new(&self.config.whisper_binary);
        cmd.arg("-m")
            .arg(&self.config.model_path)
            .arg("-f")
            .arg(audio_path)
            .arg("--output-json")
            .arg("-pp") // print progress
            .arg("-t")
            .arg(self.config.threads.to_string());

        if let Some(ref lang) = self.config.language {
            cmd.arg("-l").arg(lang);
        } else {
            cmd.arg("-l").arg("auto");
        }

        let output = cmd
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .map_err(|e| {
                AiError::PreprocessError(format!(
                    "Failed to run whisper-cpp ({}): {e}",
                    self.config.whisper_binary.display()
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            warn!(stderr = %stderr, "whisper-cpp failed");
            return Err(AiError::PreprocessError(format!(
                "whisper-cpp exited with status {}: {}",
                output.status,
                stderr.chars().take(500).collect::<String>()
            )));
        }

        // whisper.cpp writes JSON output next to the input file
        let json_path = audio_path.with_extension("wav.json");
        let alt_json_path = audio_path.with_extension("json");

        let json_content = if json_path.exists() {
            std::fs::read_to_string(&json_path)?
        } else if alt_json_path.exists() {
            std::fs::read_to_string(&alt_json_path)?
        } else {
            // Try parsing stdout as JSON
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().starts_with('{') || stdout.trim().starts_with('[') {
                stdout.to_string()
            } else {
                return Err(AiError::PreprocessError(
                    "whisper-cpp produced no JSON output".to_string(),
                ));
            }
        };

        debug!(json_len = json_content.len(), "Parsing whisper output");
        parse_whisper_json(&json_content)
    }
}

/// Parse whisper.cpp JSON output into a Transcript.
fn parse_whisper_json(json_str: &str) -> AiResult<Transcript> {
    // whisper.cpp JSON format:
    // { "transcription": [ { "timestamps": { "from": "00:00:00,000", "to": "00:00:02,000" },
    //                        "offsets": { "from": 0, "to": 2000 },
    //                        "text": " Hello world" } ] }
    let value: serde_json::Value = serde_json::from_str(json_str)
        .map_err(|e| AiError::PreprocessError(format!("Failed to parse whisper JSON: {e}")))?;

    let mut words = Vec::new();
    let mut max_end_time: f64 = 0.0;

    if let Some(transcription) = value.get("transcription").and_then(|v| v.as_array()) {
        for segment in transcription {
            let text = segment
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim()
                .to_string();

            if text.is_empty() {
                continue;
            }

            let from_ms = segment
                .get("offsets")
                .and_then(|o| o.get("from"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);
            let to_ms = segment
                .get("offsets")
                .and_then(|o| o.get("to"))
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0);

            let start_time = from_ms / 1000.0;
            let end_time = to_ms / 1000.0;

            if end_time > max_end_time {
                max_end_time = end_time;
            }

            // Split segment text into individual words and distribute time evenly
            let segment_words: Vec<&str> = text.split_whitespace().collect();
            let word_count = segment_words.len();
            if word_count == 0 {
                continue;
            }

            let duration_per_word = if word_count > 1 {
                (end_time - start_time) / word_count as f64
            } else {
                end_time - start_time
            };

            for (i, word_text) in segment_words.iter().enumerate() {
                let word_start = start_time + i as f64 * duration_per_word;
                let word_end = word_start + duration_per_word;
                words.push(TranscriptWord {
                    text: word_text.to_string(),
                    start_time: word_start,
                    end_time: word_end,
                    confidence: 1.0,
                });
            }
        }
    }

    let language = value
        .get("result")
        .and_then(|r| r.get("language"))
        .and_then(|l| l.as_str())
        .unwrap_or("en")
        .to_string();

    Ok(Transcript {
        words,
        language,
        duration_secs: max_end_time,
    })
}

/// Extract audio from a video file as 16kHz mono WAV using FFmpeg.
pub fn extract_audio(video_path: &Path, output_dir: &Path) -> AiResult<PathBuf> {
    let wav_path = output_dir.join("audio_16k.wav");

    let status = Command::new("ffmpeg")
        .args([
            "-i",
            &video_path.to_string_lossy(),
            "-ar",
            "16000",
            "-ac",
            "1",
            "-vn",
            &wav_path.to_string_lossy(),
            "-y",
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .status()
        .map_err(|e| AiError::PreprocessError(format!("FFmpeg not found: {e}")))?;

    if !status.success() {
        return Err(AiError::PreprocessError(
            "FFmpeg audio extraction failed".to_string(),
        ));
    }

    Ok(wav_path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_whisper_binary_doesnt_panic() {
        let path = TranscriberConfig::find_whisper_binary();
        assert!(!path.to_string_lossy().is_empty());
    }

    #[test]
    fn test_transcript_serialization_roundtrip() {
        let transcript = Transcript {
            words: vec![
                TranscriptWord {
                    text: "hello".into(),
                    start_time: 0.0,
                    end_time: 0.5,
                    confidence: 0.95,
                },
                TranscriptWord {
                    text: "world".into(),
                    start_time: 0.6,
                    end_time: 1.0,
                    confidence: 0.88,
                },
            ],
            language: "en".into(),
            duration_secs: 1.0,
        };
        let json = serde_json::to_string(&transcript).expect("serialize");
        let parsed: Transcript = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed.words.len(), 2);
        assert_eq!(parsed.words[0].text, "hello");
        assert!((parsed.words[1].end_time - 1.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_whisper_json_valid() {
        let json = r#"{
            "transcription": [
                {
                    "timestamps": { "from": "00:00:00,000", "to": "00:00:02,000" },
                    "offsets": { "from": 0, "to": 2000 },
                    "text": " Hello world"
                },
                {
                    "timestamps": { "from": "00:00:02,000", "to": "00:00:04,000" },
                    "offsets": { "from": 2000, "to": 4000 },
                    "text": " This is a test"
                }
            ]
        }"#;

        let transcript = parse_whisper_json(json).expect("should parse");
        assert_eq!(transcript.words.len(), 6); // "Hello", "world", "This", "is", "a", "test"
        assert_eq!(transcript.words[0].text, "Hello");
        assert!((transcript.words[0].start_time - 0.0).abs() < 1e-6);
        assert!((transcript.duration_secs - 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_parse_whisper_json_empty() {
        let json = r#"{ "transcription": [] }"#;
        let transcript = parse_whisper_json(json).expect("should parse empty");
        assert!(transcript.words.is_empty());
    }

    #[test]
    #[ignore] // Run with: cargo test -p proedit-ai -- --ignored
    fn test_transcribe_real_audio() {
        // Requires whisper-cpp installed and a model + test audio
    }
}
