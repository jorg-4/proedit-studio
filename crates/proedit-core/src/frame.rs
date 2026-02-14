//! Frame buffer types for video frames in CPU memory.
//!
//! Designed for efficient memory usage on systems with limited RAM.

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::Arc;

/// Unique identifier for a frame in the cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameId(pub u64);

/// Pixel format enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum PixelFormat {
    /// 8-bit RGBA (32 bits per pixel)
    #[default]
    Rgba8,
    /// 16-bit RGBA half-float (64 bits per pixel)
    Rgba16F,
    /// 32-bit RGBA float (128 bits per pixel)
    Rgba32F,
    /// 8-bit grayscale
    Gray8,
    /// 16-bit grayscale half-float
    Gray16F,
    /// NV12 YUV format (VideoToolbox native)
    Nv12,
    /// YUV 4:2:0 planar
    Yuv420P,
    /// YUV 4:2:0 planar 10-bit
    Yuv420P10,
}

impl PixelFormat {
    /// Bytes per pixel for packed formats, or 0 for planar.
    pub fn bytes_per_pixel(self) -> usize {
        match self {
            Self::Rgba8 => 4,
            Self::Rgba16F => 8,
            Self::Rgba32F => 16,
            Self::Gray8 => 1,
            Self::Gray16F => 2,
            Self::Nv12 | Self::Yuv420P | Self::Yuv420P10 => 0, // Planar
        }
    }

    /// Number of planes for this format.
    pub fn plane_count(self) -> usize {
        match self {
            Self::Rgba8 | Self::Rgba16F | Self::Rgba32F | Self::Gray8 | Self::Gray16F => 1,
            Self::Nv12 => 2,
            Self::Yuv420P | Self::Yuv420P10 => 3,
        }
    }

    /// Calculate total bytes needed for a frame of this format.
    pub fn frame_size(self, width: u32, height: u32) -> usize {
        match self {
            Self::Rgba8 => (width * height * 4) as usize,
            Self::Rgba16F => (width * height * 8) as usize,
            Self::Rgba32F => (width * height * 16) as usize,
            Self::Gray8 => (width * height) as usize,
            Self::Gray16F => (width * height * 2) as usize,
            Self::Nv12 => {
                // Y plane + UV interleaved (half resolution)
                let y_size = (width * height) as usize;
                let uv_size = (width * height / 2) as usize;
                y_size + uv_size
            }
            Self::Yuv420P => {
                // Y + U + V planes (U/V at half resolution)
                let y_size = (width * height) as usize;
                let uv_size = (width / 2 * height / 2) as usize;
                y_size + uv_size * 2
            }
            Self::Yuv420P10 => {
                // Same as YUV420P but 2 bytes per sample
                let y_size = (width * height * 2) as usize;
                let uv_size = (width / 2 * height / 2 * 2) as usize;
                y_size + uv_size * 2
            }
        }
    }
}


/// A plane of pixel data with stride information.
#[derive(Debug, Clone)]
pub struct FramePlane {
    /// Raw pixel data
    pub data: Vec<u8>,
    /// Bytes per row (may include padding)
    pub stride: usize,
    /// Width in pixels
    pub width: u32,
    /// Height in pixels
    pub height: u32,
}

impl FramePlane {
    /// Create a new frame plane with the given dimensions.
    pub fn new(width: u32, height: u32, bytes_per_pixel: usize) -> Self {
        // Align stride to 64 bytes for SIMD and GPU compatibility
        let min_stride = (width as usize) * bytes_per_pixel;
        let stride = (min_stride + 63) & !63;
        let data = vec![0u8; stride * height as usize];
        Self {
            data,
            stride,
            width,
            height,
        }
    }

    /// Get a row of pixel data.
    #[inline]
    pub fn row(&self, y: u32) -> &[u8] {
        let start = y as usize * self.stride;
        let bpp = self.bytes_per_row_pixel();
        let end = start + (self.width as usize * bpp);
        &self.data[start..end]
    }

    /// Get a mutable row of pixel data.
    #[inline]
    pub fn row_mut(&mut self, y: u32) -> &mut [u8] {
        let start = y as usize * self.stride;
        let bpp = self.bytes_per_row_pixel();
        let end = start + (self.width as usize * bpp);
        &mut self.data[start..end]
    }

    fn bytes_per_row_pixel(&self) -> usize {
        if self.width == 0 || self.stride == 0 {
            return 1;
        }
        // Estimate bytes per pixel from stride
        let min_bpp = self.stride / self.width as usize;
        if min_bpp == 0 {
            1
        } else {
            min_bpp
        }
    }
}

/// A video frame in CPU memory.
///
/// Memory layout is optimized for:
/// - Zero-copy upload to GPU textures
/// - Efficient FFmpeg interop
/// - Cache-friendly access patterns
#[derive(Debug, Clone)]
pub struct FrameBuffer {
    /// Pixel format
    pub format: PixelFormat,
    /// Frame width in pixels
    pub width: u32,
    /// Frame height in pixels
    pub height: u32,
    /// Pixel data planes (1-3 depending on format)
    pub planes: SmallVec<[FramePlane; 3]>,
}

impl FrameBuffer {
    /// Create a new frame buffer with the given dimensions and format.
    pub fn new(width: u32, height: u32, format: PixelFormat) -> Self {
        let planes = match format {
            PixelFormat::Rgba8 => {
                smallvec::smallvec![FramePlane::new(width, height, 4)]
            }
            PixelFormat::Rgba16F => {
                smallvec::smallvec![FramePlane::new(width, height, 8)]
            }
            PixelFormat::Rgba32F => {
                smallvec::smallvec![FramePlane::new(width, height, 16)]
            }
            PixelFormat::Gray8 => {
                smallvec::smallvec![FramePlane::new(width, height, 1)]
            }
            PixelFormat::Gray16F => {
                smallvec::smallvec![FramePlane::new(width, height, 2)]
            }
            PixelFormat::Nv12 => {
                smallvec::smallvec![
                    FramePlane::new(width, height, 1),           // Y
                    FramePlane::new(width / 2, height / 2, 2),   // UV interleaved
                ]
            }
            PixelFormat::Yuv420P => {
                smallvec::smallvec![
                    FramePlane::new(width, height, 1),           // Y
                    FramePlane::new(width / 2, height / 2, 1),   // U
                    FramePlane::new(width / 2, height / 2, 1),   // V
                ]
            }
            PixelFormat::Yuv420P10 => {
                smallvec::smallvec![
                    FramePlane::new(width, height, 2),           // Y
                    FramePlane::new(width / 2, height / 2, 2),   // U
                    FramePlane::new(width / 2, height / 2, 2),   // V
                ]
            }
        };

        Self {
            format,
            width,
            height,
            planes,
        }
    }

    /// Total memory usage of this frame in bytes.
    pub fn memory_size(&self) -> usize {
        self.planes.iter().map(|p| p.data.len()).sum()
    }

    /// Get the primary plane (plane 0).
    #[inline]
    pub fn primary_plane(&self) -> &FramePlane {
        &self.planes[0]
    }

    /// Get the primary plane mutably.
    #[inline]
    pub fn primary_plane_mut(&mut self) -> &mut FramePlane {
        &mut self.planes[0]
    }

    /// Create a test pattern frame (color bars).
    pub fn test_pattern(width: u32, height: u32) -> Self {
        let mut frame = Self::new(width, height, PixelFormat::Rgba8);
        let plane = frame.primary_plane_mut();

        for y in 0..height {
            let row = plane.row_mut(y);
            for x in 0..width {
                let i = (x * 4) as usize;
                // Color bars pattern (8 bars)
                let bar = (x * 8 / width) as u8;
                let colors: [[u8; 4]; 8] = [
                    [255, 255, 255, 255], // White
                    [255, 255, 0, 255],   // Yellow
                    [0, 255, 255, 255],   // Cyan
                    [0, 255, 0, 255],     // Green
                    [255, 0, 255, 255],   // Magenta
                    [255, 0, 0, 255],     // Red
                    [0, 0, 255, 255],     // Blue
                    [0, 0, 0, 255],       // Black
                ];
                let color = colors[bar as usize];
                row[i..i + 4].copy_from_slice(&color);
            }
        }

        frame
    }
}

/// Arc-wrapped frame buffer for shared ownership.
pub type SharedFrameBuffer = Arc<FrameBuffer>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rgba8_frame_size() {
        let frame = FrameBuffer::new(1920, 1080, PixelFormat::Rgba8);
        // With 64-byte alignment, stride is 1920*4 = 7680, aligned to 7680
        assert!(frame.memory_size() >= 1920 * 1080 * 4);
    }

    #[test]
    fn test_yuv420p_planes() {
        let frame = FrameBuffer::new(1920, 1080, PixelFormat::Yuv420P);
        assert_eq!(frame.planes.len(), 3);
        assert_eq!(frame.planes[0].width, 1920);
        assert_eq!(frame.planes[1].width, 960);
        assert_eq!(frame.planes[2].width, 960);
    }

    #[test]
    fn test_test_pattern() {
        let frame = FrameBuffer::test_pattern(1920, 1080);
        assert_eq!(frame.width, 1920);
        assert_eq!(frame.height, 1080);

        // Check first pixel is white
        let row = frame.primary_plane().row(0);
        assert_eq!(row[0..4], [255, 255, 255, 255]);
    }
}
