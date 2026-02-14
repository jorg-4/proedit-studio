//! Video decoder using FFmpeg via ffmpeg-sidecar.

use proedit_core::{FrameBuffer, FrameRate, ProEditError, Result};
use std::path::Path;
use tracing::info;

/// A decoded video frame with metadata.
pub struct VideoFrame {
    /// Frame data in RGBA8 format
    pub buffer: FrameBuffer,
    /// Presentation timestamp in seconds
    pub pts: f64,
    /// Frame number
    pub frame_number: i64,
}

/// Video decoder using FFmpeg.
///
/// Uses ffmpeg-sidecar to spawn FFmpeg as a subprocess for decoding.
/// This approach works without system FFmpeg development headers.
pub struct VideoDecoder {
    path: String,
    width: u32,
    height: u32,
    frame_rate: FrameRate,
    duration: f64,
    frame_count: i64,
    current_frame: i64,
}

impl VideoDecoder {
    /// Open a video file for decoding.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();
        let path_str = path.to_string_lossy().to_string();

        info!("Opening video file: {}", path_str);

        // For now, create a placeholder decoder
        // In a real implementation, we would probe the file with FFmpeg
        Ok(Self {
            path: path_str,
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::FPS_24,
            duration: 10.0,
            frame_count: 240,
            current_frame: 0,
        })
    }

    /// Get the file path.
    pub fn path(&self) -> &str {
        &self.path
    }

    /// Get the video dimensions.
    pub fn dimensions(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// Get the frame rate.
    pub fn frame_rate(&self) -> FrameRate {
        self.frame_rate
    }

    /// Get video duration in seconds.
    pub fn duration(&self) -> f64 {
        self.duration
    }

    /// Get total frame count.
    pub fn frame_count(&self) -> i64 {
        self.frame_count
    }

    /// Get current frame number.
    pub fn current_frame(&self) -> i64 {
        self.current_frame
    }

    /// Decode the next frame, returning it as an RGBA8 FrameBuffer.
    pub fn decode_frame(&mut self) -> Result<Option<VideoFrame>> {
        if self.current_frame >= self.frame_count {
            return Ok(None);
        }

        // Generate a test pattern frame for now
        // In a real implementation, this would decode from the video file
        let buffer = FrameBuffer::test_pattern(self.width, self.height);
        let pts = self.current_frame as f64 / self.frame_rate.to_fps_f64();
        let frame_number = self.current_frame;

        self.current_frame += 1;

        Ok(Some(VideoFrame {
            buffer,
            pts,
            frame_number,
        }))
    }

    /// Seek to a specific frame number.
    pub fn seek_to_frame(&mut self, frame_number: i64) -> Result<()> {
        if frame_number < 0 || frame_number >= self.frame_count {
            return Err(ProEditError::InvalidParameter(format!(
                "Frame {} out of range (0-{})",
                frame_number,
                self.frame_count - 1
            )));
        }

        self.current_frame = frame_number;
        info!("Seeked to frame {}", frame_number);
        Ok(())
    }

    /// Seek to a specific time in seconds.
    pub fn seek_to_time(&mut self, time: f64) -> Result<()> {
        let frame = (time * self.frame_rate.to_fps_f64()).floor() as i64;
        self.seek_to_frame(frame)
    }
}
