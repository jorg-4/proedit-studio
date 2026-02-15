//! ProEdit Media - FFmpeg integration for video/audio I/O
//!
//! This crate handles:
//! - Video decoding with hardware acceleration
//! - Audio decoding
//! - Media file probing
//! - Encoding and muxing

pub mod decoder;
pub mod export;
pub mod probe;

pub use decoder::{VideoDecoder, VideoFrame};
pub use export::{ExportCancel, ExportFormat, ExportJob, ExportProgress, VideoCodec};
pub use probe::MediaProbe;

/// Initialize FFmpeg (call once at startup).
pub fn init() {
    tracing::info!("ProEdit Media initialized");
}
