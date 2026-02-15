//! Export pipeline for rendering timelines to video files.
//!
//! Uses FFmpeg via the sidecar process for encoding. Supports format presets,
//! progress reporting, and cancellation.

use proedit_core::{FrameRate, RationalTime};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

// ── Format presets ──────────────────────────────────────────────

/// Video codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoCodec {
    H264,
    H265,
    ProRes422,
    ProRes4444,
    Vp9,
    Av1,
}

impl VideoCodec {
    /// FFmpeg encoder name.
    pub fn ffmpeg_encoder(self) -> &'static str {
        match self {
            Self::H264 => "libx264",
            Self::H265 => "libx265",
            Self::ProRes422 => "prores_ks",
            Self::ProRes4444 => "prores_ks",
            Self::Vp9 => "libvpx-vp9",
            Self::Av1 => "libaom-av1",
        }
    }

    /// File extension for this codec.
    pub fn extension(self) -> &'static str {
        match self {
            Self::H264 | Self::H265 => "mp4",
            Self::ProRes422 | Self::ProRes4444 => "mov",
            Self::Vp9 => "webm",
            Self::Av1 => "mp4",
        }
    }
}

/// Audio codec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum AudioCodec {
    Aac,
    Pcm,
    Flac,
    Opus,
}

impl AudioCodec {
    /// FFmpeg encoder name.
    pub fn ffmpeg_encoder(self) -> &'static str {
        match self {
            Self::Aac => "aac",
            Self::Pcm => "pcm_s16le",
            Self::Flac => "flac",
            Self::Opus => "libopus",
        }
    }
}

/// Export quality preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum QualityPreset {
    Draft,
    Normal,
    High,
    Lossless,
}

/// Export format configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportFormat {
    pub video_codec: VideoCodec,
    pub audio_codec: AudioCodec,
    pub width: u32,
    pub height: u32,
    pub frame_rate: FrameRate,
    pub quality: QualityPreset,
    /// CRF value for H.264/H.265 (0-51, lower = better).
    pub crf: Option<u32>,
    /// Bitrate in kbps (overrides CRF if set).
    pub video_bitrate: Option<u32>,
    /// Audio bitrate in kbps.
    pub audio_bitrate: u32,
    /// Audio sample rate.
    pub audio_sample_rate: u32,
}

impl ExportFormat {
    /// H.264 HD preset.
    pub fn h264_hd() -> Self {
        Self {
            video_codec: VideoCodec::H264,
            audio_codec: AudioCodec::Aac,
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::FPS_24,
            quality: QualityPreset::Normal,
            crf: Some(18),
            video_bitrate: None,
            audio_bitrate: 192,
            audio_sample_rate: 48000,
        }
    }

    /// H.265 4K preset.
    pub fn h265_4k() -> Self {
        Self {
            video_codec: VideoCodec::H265,
            audio_codec: AudioCodec::Aac,
            width: 3840,
            height: 2160,
            frame_rate: FrameRate::FPS_24,
            quality: QualityPreset::High,
            crf: Some(20),
            video_bitrate: None,
            audio_bitrate: 256,
            audio_sample_rate: 48000,
        }
    }

    /// ProRes 422 for mastering.
    pub fn prores_422() -> Self {
        Self {
            video_codec: VideoCodec::ProRes422,
            audio_codec: AudioCodec::Pcm,
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::FPS_24,
            quality: QualityPreset::High,
            crf: None,
            video_bitrate: None,
            audio_bitrate: 1536,
            audio_sample_rate: 48000,
        }
    }

    /// Web-optimized VP9.
    pub fn vp9_web() -> Self {
        Self {
            video_codec: VideoCodec::Vp9,
            audio_codec: AudioCodec::Opus,
            width: 1920,
            height: 1080,
            frame_rate: FrameRate::FPS_30,
            quality: QualityPreset::Normal,
            crf: Some(30),
            video_bitrate: None,
            audio_bitrate: 128,
            audio_sample_rate: 48000,
        }
    }
}

// ── Export job ───────────────────────────────────────────────────

/// Export progress information.
#[derive(Debug, Clone)]
pub struct ExportProgress {
    /// Current frame being rendered.
    pub current_frame: u64,
    /// Total frames to render.
    pub total_frames: u64,
    /// Estimated time remaining in seconds.
    pub eta_seconds: f64,
    /// Frames per second (encoding speed).
    pub fps: f64,
}

impl ExportProgress {
    /// Completion percentage (0.0 to 1.0).
    pub fn fraction(&self) -> f64 {
        if self.total_frames == 0 {
            return 0.0;
        }
        self.current_frame as f64 / self.total_frames as f64
    }
}

/// An export job configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExportJob {
    /// Output file path.
    pub output_path: PathBuf,
    /// Export format.
    pub format: ExportFormat,
    /// Time range to export (None = entire sequence).
    pub range: Option<(RationalTime, RationalTime)>,
}

impl ExportJob {
    /// Create a new export job.
    pub fn new(output_path: impl Into<PathBuf>, format: ExportFormat) -> Self {
        Self {
            output_path: output_path.into(),
            format,
            range: None,
        }
    }

    /// Set the export range.
    pub fn with_range(mut self, start: RationalTime, end: RationalTime) -> Self {
        self.range = Some((start, end));
        self
    }

    /// Compute total frames for this job.
    pub fn total_frames(&self, sequence_duration: RationalTime) -> u64 {
        let duration = if let Some((start, end)) = self.range {
            end - start
        } else {
            sequence_duration
        };
        duration.to_frames(self.format.frame_rate).unsigned_abs()
    }

    /// Build the FFmpeg command arguments.
    pub fn ffmpeg_args(&self) -> Vec<String> {
        let mut args = Vec::new();

        // Input from pipe (raw frames)
        args.extend_from_slice(&[
            "-y".into(),
            "-f".into(),
            "rawvideo".into(),
            "-pixel_format".into(),
            "rgba".into(),
            "-video_size".into(),
            format!("{}x{}", self.format.width, self.format.height),
            "-framerate".into(),
            format!(
                "{}/{}",
                self.format.frame_rate.numerator, self.format.frame_rate.denominator
            ),
            "-i".into(),
            "pipe:0".into(),
        ]);

        // Video codec
        args.extend_from_slice(&[
            "-c:v".into(),
            self.format.video_codec.ffmpeg_encoder().into(),
        ]);

        // Quality settings
        if let Some(crf) = self.format.crf {
            args.extend_from_slice(&["-crf".into(), crf.to_string()]);
        }
        if let Some(bitrate) = self.format.video_bitrate {
            args.extend_from_slice(&["-b:v".into(), format!("{}k", bitrate)]);
        }

        // ProRes profile
        if self.format.video_codec == VideoCodec::ProRes422 {
            args.extend_from_slice(&["-profile:v".into(), "2".into()]);
        } else if self.format.video_codec == VideoCodec::ProRes4444 {
            args.extend_from_slice(&["-profile:v".into(), "4".into()]);
        }

        // Pixel format for output
        args.extend_from_slice(&["-pix_fmt".into(), "yuv420p".into()]);

        // Output
        args.push(self.output_path.to_string_lossy().into_owned());

        args
    }
}

impl ExportJob {
    /// Run the export, piping placeholder black RGBA frames into FFmpeg.
    ///
    /// * `sequence_duration` – total duration of the sequence (used to compute frame count).
    /// * `on_progress` – called periodically with progress info.
    /// * `cancel` – checked every frame; if cancelled, the export aborts early.
    ///
    /// Real compositor integration will replace the black-frame writer later.
    pub fn run(
        &self,
        sequence_duration: RationalTime,
        on_progress: impl Fn(ExportProgress),
        cancel: &ExportCancel,
    ) -> proedit_core::Result<()> {
        use std::io::Write;
        use std::process::{Command, Stdio};
        use std::time::Instant;

        let total_frames = self.total_frames(sequence_duration);
        if total_frames == 0 {
            return Ok(());
        }

        let args = self.ffmpeg_args();
        let mut child = Command::new("ffmpeg")
            .args(&args)
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| {
                proedit_core::ProEditError::Encoder(format!("Failed to spawn ffmpeg: {e}"))
            })?;

        let mut stdin = child.stdin.take().ok_or_else(|| {
            proedit_core::ProEditError::Encoder("Failed to open ffmpeg stdin".into())
        })?;

        // Black RGBA frame (placeholder until real compositor is wired in)
        let frame_size = (self.format.width as usize) * (self.format.height as usize) * 4;
        let black_frame = vec![0u8; frame_size];

        let start_time = Instant::now();

        for frame_number in 0..total_frames {
            if cancel.is_cancelled() {
                // Drop stdin to signal EOF, then kill
                drop(stdin);
                let _ = child.kill();
                let _ = child.wait();
                return Err(proedit_core::ProEditError::Encoder(
                    "Export cancelled".into(),
                ));
            }

            stdin.write_all(&black_frame).map_err(|e| {
                proedit_core::ProEditError::Encoder(format!("Failed to write frame: {e}"))
            })?;

            // Report progress every 10 frames
            if frame_number % 10 == 0 || frame_number == total_frames - 1 {
                let elapsed = start_time.elapsed().as_secs_f64();
                let fps = if elapsed > 0.0 {
                    (frame_number + 1) as f64 / elapsed
                } else {
                    0.0
                };
                let remaining = if fps > 0.0 {
                    (total_frames - frame_number - 1) as f64 / fps
                } else {
                    0.0
                };
                on_progress(ExportProgress {
                    current_frame: frame_number,
                    total_frames,
                    eta_seconds: remaining,
                    fps,
                });
            }
        }

        // Close stdin to signal end-of-stream
        drop(stdin);

        let status = child.wait().map_err(|e| {
            proedit_core::ProEditError::Encoder(format!("Failed to wait for ffmpeg: {e}"))
        })?;

        if !status.success() {
            return Err(proedit_core::ProEditError::Encoder(format!(
                "ffmpeg exited with status: {}",
                status
            )));
        }

        Ok(())
    }
}

/// Handle for cancelling an in-progress export.
#[derive(Debug, Clone)]
pub struct ExportCancel(Arc<AtomicBool>);

impl ExportCancel {
    /// Create a new cancel handle.
    pub fn new() -> Self {
        Self(Arc::new(AtomicBool::new(false)))
    }

    /// Signal cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }

    /// Check if cancelled.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Relaxed)
    }
}

impl Default for ExportCancel {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_h264_preset() {
        let fmt = ExportFormat::h264_hd();
        assert_eq!(fmt.video_codec.ffmpeg_encoder(), "libx264");
        assert_eq!(fmt.video_codec.extension(), "mp4");
        assert_eq!(fmt.width, 1920);
        assert_eq!(fmt.height, 1080);
    }

    #[test]
    fn test_export_job_total_frames() {
        let job = ExportJob::new("/tmp/out.mp4", ExportFormat::h264_hd());
        let duration = RationalTime::new(10, 1); // 10 seconds
        assert_eq!(job.total_frames(duration), 240); // 10s * 24fps
    }

    #[test]
    fn test_export_job_range() {
        let job = ExportJob::new("/tmp/out.mp4", ExportFormat::h264_hd())
            .with_range(RationalTime::new(5, 1), RationalTime::new(10, 1));
        let duration = RationalTime::new(100, 1);
        assert_eq!(job.total_frames(duration), 120); // 5s * 24fps
    }

    #[test]
    fn test_ffmpeg_args() {
        let job = ExportJob::new("/tmp/out.mp4", ExportFormat::h264_hd());
        let args = job.ffmpeg_args();
        assert!(args.contains(&"-c:v".to_string()));
        assert!(args.contains(&"libx264".to_string()));
        assert!(args.contains(&"-crf".to_string()));
    }

    #[test]
    fn test_progress_fraction() {
        let progress = ExportProgress {
            current_frame: 50,
            total_frames: 200,
            eta_seconds: 10.0,
            fps: 30.0,
        };
        assert!((progress.fraction() - 0.25).abs() < 0.001);
    }

    #[test]
    fn test_cancel_handle() {
        let cancel = ExportCancel::new();
        assert!(!cancel.is_cancelled());
        cancel.cancel();
        assert!(cancel.is_cancelled());
    }
}
