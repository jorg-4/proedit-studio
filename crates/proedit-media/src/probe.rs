//! Media file probing to get metadata without full decode.

use proedit_core::{FrameRate, ProEditError, RationalTime, Result};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Information about a media file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaProbe {
    /// File path
    pub path: String,
    /// Duration
    pub duration: RationalTime,
    /// Video streams
    pub video_streams: Vec<VideoStreamInfo>,
    /// Audio streams
    pub audio_streams: Vec<AudioStreamInfo>,
    /// Container format
    pub format: String,
}

/// Information about a video stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoStreamInfo {
    pub index: usize,
    pub codec: String,
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub pixel_format: String,
    pub bit_rate: Option<u64>,
}

/// Information about an audio stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioStreamInfo {
    pub index: usize,
    pub codec: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub bit_rate: Option<u64>,
}

impl MediaProbe {
    /// Probe a media file.
    pub fn probe<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        // Check if file exists
        if !path.exists() {
            return Err(ProEditError::NotFound(format!(
                "File not found: {}",
                path_str
            )));
        }

        // For now, return placeholder data
        // In a real implementation, we would use ffprobe via ffmpeg-sidecar
        Ok(Self {
            path: path_str,
            duration: RationalTime::from_seconds_f64(10.0),
            video_streams: vec![VideoStreamInfo {
                index: 0,
                codec: "h264".to_string(),
                width: 1920,
                height: 1080,
                frame_rate: FrameRate::FPS_24,
                pixel_format: "yuv420p".to_string(),
                bit_rate: Some(10_000_000),
            }],
            audio_streams: vec![AudioStreamInfo {
                index: 1,
                codec: "aac".to_string(),
                sample_rate: 48000,
                channels: 2,
                bit_rate: Some(192_000),
            }],
            format: "mp4".to_string(),
        })
    }

    /// Check if the file has video.
    pub fn has_video(&self) -> bool {
        !self.video_streams.is_empty()
    }

    /// Check if the file has audio.
    pub fn has_audio(&self) -> bool {
        !self.audio_streams.is_empty()
    }

    /// Get the primary video stream info.
    pub fn primary_video(&self) -> Option<&VideoStreamInfo> {
        self.video_streams.first()
    }

    /// Get the primary audio stream info.
    pub fn primary_audio(&self) -> Option<&AudioStreamInfo> {
        self.audio_streams.first()
    }
}
