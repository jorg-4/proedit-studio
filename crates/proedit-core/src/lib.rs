//! ProEdit Core - Foundation types for video editing
//!
//! This crate provides the fundamental types used throughout ProEdit:
//! - Time representation (RationalTime, FrameRate, TimeRange)
//! - Color and color space management
//! - Frame buffers and pixel formats
//! - Geometric primitives

pub mod color;
pub mod error;
pub mod frame;
pub mod geometry;
pub mod time;

pub use color::{Color, ColorConfig, ColorSpace, TransferFunction};
pub use error::{ProEditError, Result};
pub use frame::{FrameBuffer, FrameId, FramePlane, PixelFormat, SharedFrameBuffer};
pub use geometry::{Rect, Transform2D, Vec2};
pub use time::{FrameRate, RationalTime, TimeRange};

/// Memory budget constants for 8GB M1 Mac
pub mod memory_budget {
    /// Total frame cache budget (for decoded frames in RAM)
    pub const FRAME_CACHE_SIZE: usize = 512 * 1024 * 1024; // 512 MB

    /// Maximum texture memory for GPU
    pub const GPU_TEXTURE_BUDGET: usize = 1024 * 1024 * 1024; // 1 GB

    /// Decode buffer pool size
    pub const DECODE_BUFFER_POOL: usize = 256 * 1024 * 1024; // 256 MB

    /// Number of frames to buffer ahead
    pub const LOOKAHEAD_FRAMES: usize = 8;

    /// Maximum 4K frames in cache (4K RGBA = 33MB each)
    pub const MAX_4K_CACHED_FRAMES: usize = 15;

    /// Maximum 1080p frames in cache (1080p RGBA = 8MB each)
    pub const MAX_1080P_CACHED_FRAMES: usize = 60;
}
