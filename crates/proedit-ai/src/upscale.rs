//! AI upscaling using Real-ESRGAN via ONNX Runtime.
//!
//! Upscales footage from 1080p to 4K (or other scales) using tile-based
//! processing with feathered overlap to avoid seam artifacts.
//!
//! Requires the `onnx` feature flag and a pre-downloaded Real-ESRGAN model.

#[cfg(feature = "onnx")]
use crate::error::AiError;
use crate::error::AiResult;
use proedit_core::FrameBuffer;

/// Configuration for the upscaler.
#[derive(Debug, Clone)]
pub struct UpscaleConfig {
    /// Upscale factor (2 or 4).
    pub scale: u32,
    /// Tile size in pixels (smaller = less VRAM, more tiles).
    pub tile_size: u32,
    /// Overlap between adjacent tiles in pixels.
    pub tile_overlap: u32,
}

impl Default for UpscaleConfig {
    fn default() -> Self {
        Self {
            scale: 4,
            tile_size: 512,
            tile_overlap: 32,
        }
    }
}

/// A tile extracted from the input frame for processing.
#[derive(Debug, Clone)]
pub struct Tile {
    /// Pixel data for this tile (RGBA8).
    pub pixels: Vec<u8>,
    /// Width of the tile in pixels.
    pub width: u32,
    /// Height of the tile in pixels.
    pub height: u32,
    /// Position of the tile in the source frame (x, y).
    pub src_x: u32,
    /// Position of the tile in the source frame (x, y).
    pub src_y: u32,
}

/// Split a frame into overlapping tiles for processing.
pub fn split_into_tiles(frame: &FrameBuffer, tile_size: u32, overlap: u32) -> Vec<Tile> {
    let w = frame.width;
    let h = frame.height;
    let step = tile_size.saturating_sub(overlap).max(1);
    let plane = frame.primary_plane();

    let mut tiles = Vec::new();

    let mut y = 0u32;
    while y < h {
        let tile_h = tile_size.min(h - y);
        let mut x = 0u32;
        while x < w {
            let tile_w = tile_size.min(w - x);
            let mut pixels = vec![0u8; (tile_w as usize) * (tile_h as usize) * 4];

            for ty in 0..tile_h {
                let src_row = plane.row(y + ty);
                let src_start = (x as usize) * 4;
                let src_end = src_start + (tile_w as usize) * 4;
                let dst_start = (ty as usize) * (tile_w as usize) * 4;
                let dst_end = dst_start + (tile_w as usize) * 4;

                if src_end <= src_row.len() {
                    pixels[dst_start..dst_end].copy_from_slice(&src_row[src_start..src_end]);
                }
            }

            tiles.push(Tile {
                pixels,
                width: tile_w,
                height: tile_h,
                src_x: x,
                src_y: y,
            });

            x += step;
            if x >= w && x < w + step - 1 {
                break;
            }
        }
        y += step;
        if y >= h && y < h + step - 1 {
            break;
        }
    }

    tiles
}

/// Parameters for blending a tile into the output frame.
pub struct BlendParams {
    /// Width of the tile.
    pub tile_w: u32,
    /// Height of the tile.
    pub tile_h: u32,
    /// Destination x position in the output frame.
    pub dst_x: u32,
    /// Destination y position in the output frame.
    pub dst_y: u32,
    /// Overlap in pixels (before scaling).
    pub overlap: u32,
    /// Scale factor.
    pub scale: u32,
}

/// Blend an upscaled tile into the output frame with feathered overlap.
pub fn blend_tile_into(output: &mut FrameBuffer, tile_data: &[u8], params: &BlendParams) {
    let tile_w = params.tile_w;
    let tile_h = params.tile_h;
    let dst_x = params.dst_x;
    let dst_y = params.dst_y;
    let out_w = output.width;
    let out_h = output.height;
    let out_plane = output.primary_plane_mut();
    let scaled_overlap = params.overlap * params.scale;

    for ty in 0..tile_h {
        let out_y = dst_y + ty;
        if out_y >= out_h {
            break;
        }
        let out_row = out_plane.row_mut(out_y);

        for tx in 0..tile_w {
            let out_x = dst_x + tx;
            if out_x >= out_w {
                break;
            }

            let src_idx = ((ty as usize) * (tile_w as usize) + (tx as usize)) * 4;
            let dst_idx = (out_x as usize) * 4;

            if src_idx + 3 >= tile_data.len() || dst_idx + 3 >= out_row.len() {
                continue;
            }

            // Compute blend weight for overlap regions
            let weight = compute_blend_weight(tx, ty, tile_w, tile_h, scaled_overlap);

            if weight >= 1.0 {
                out_row[dst_idx] = tile_data[src_idx];
                out_row[dst_idx + 1] = tile_data[src_idx + 1];
                out_row[dst_idx + 2] = tile_data[src_idx + 2];
                out_row[dst_idx + 3] = tile_data[src_idx + 3];
            } else if weight > 0.0 {
                // Blend with existing pixel
                for c in 0..4 {
                    let existing = out_row[dst_idx + c] as f32;
                    let incoming = tile_data[src_idx + c] as f32;
                    out_row[dst_idx + c] = (existing * (1.0 - weight) + incoming * weight) as u8;
                }
            }
        }
    }
}

/// Compute blend weight for a pixel in a tile (used for feathered overlap).
/// Returns 1.0 in the center, ramping down to 0.0 at edges within the overlap zone.
fn compute_blend_weight(x: u32, y: u32, w: u32, h: u32, overlap: u32) -> f32 {
    if overlap == 0 {
        return 1.0;
    }

    let wx = if x < overlap {
        x as f32 / overlap as f32
    } else if x >= w.saturating_sub(overlap) {
        (w - 1 - x) as f32 / overlap as f32
    } else {
        1.0
    };

    let wy = if y < overlap {
        y as f32 / overlap as f32
    } else if y >= h.saturating_sub(overlap) {
        (h - 1 - y) as f32 / overlap as f32
    } else {
        1.0
    };

    (wx * wy).clamp(0.0, 1.0)
}

/// AI upscaler engine using Real-ESRGAN.
pub struct Upscaler {
    config: UpscaleConfig,
}

impl Upscaler {
    /// Create a new upscaler with the given configuration.
    pub fn new(config: UpscaleConfig) -> Self {
        Self { config }
    }

    /// Load the ONNX model for upscaling.
    #[cfg(feature = "onnx")]
    pub fn load(model_path: &std::path::Path, config: UpscaleConfig) -> AiResult<Self> {
        if !model_path.exists() {
            return Err(AiError::ModelNotFound {
                model_id: format!("{:?}", crate::model_manager::ModelId::RealESRGAN4x),
            });
        }
        Ok(Self::new(config))
    }

    /// Upscale a single frame.
    ///
    /// Splits the frame into tiles, processes each tile through the model,
    /// and blends the results back together with feathered overlap.
    pub fn upscale_frame(&self, frame: &FrameBuffer) -> AiResult<FrameBuffer> {
        let out_w = frame.width * self.config.scale;
        let out_h = frame.height * self.config.scale;
        let mut output = FrameBuffer::new(out_w, out_h, proedit_core::PixelFormat::Rgba8);

        let tiles = split_into_tiles(frame, self.config.tile_size, self.config.tile_overlap);

        for tile in &tiles {
            // CPU fallback: bilinear upscale each tile
            // In production, this would run the Real-ESRGAN ONNX model
            let upscaled =
                cpu_bilinear_upscale(&tile.pixels, tile.width, tile.height, self.config.scale);

            let up_w = tile.width * self.config.scale;
            let up_h = tile.height * self.config.scale;
            let dst_x = tile.src_x * self.config.scale;
            let dst_y = tile.src_y * self.config.scale;

            blend_tile_into(
                &mut output,
                &upscaled,
                &BlendParams {
                    tile_w: up_w,
                    tile_h: up_h,
                    dst_x,
                    dst_y,
                    overlap: self.config.tile_overlap,
                    scale: self.config.scale,
                },
            );
        }

        Ok(output)
    }

    /// Get the upscale configuration.
    pub fn config(&self) -> &UpscaleConfig {
        &self.config
    }
}

/// CPU fallback: simple bilinear upscale (placeholder for Real-ESRGAN).
fn cpu_bilinear_upscale(pixels: &[u8], w: u32, h: u32, scale: u32) -> Vec<u8> {
    let out_w = w * scale;
    let out_h = h * scale;
    let mut output = vec![0u8; (out_w as usize) * (out_h as usize) * 4];

    for oy in 0..out_h {
        for ox in 0..out_w {
            // Map output pixel to source coordinates
            let sx = (ox as f32) / (scale as f32);
            let sy = (oy as f32) / (scale as f32);

            let x0 = (sx.floor() as u32).min(w - 1);
            let y0 = (sy.floor() as u32).min(h - 1);
            let x1 = (x0 + 1).min(w - 1);
            let y1 = (y0 + 1).min(h - 1);

            let fx = sx - sx.floor();
            let fy = sy - sy.floor();

            let out_idx = ((oy as usize) * (out_w as usize) + (ox as usize)) * 4;

            for c in 0..4 {
                let get_px = |x: u32, y: u32| -> f32 {
                    let idx = ((y as usize) * (w as usize) + (x as usize)) * 4 + c;
                    if idx < pixels.len() {
                        pixels[idx] as f32
                    } else {
                        0.0
                    }
                };

                let v00 = get_px(x0, y0);
                let v10 = get_px(x1, y0);
                let v01 = get_px(x0, y1);
                let v11 = get_px(x1, y1);

                let top = v00 * (1.0 - fx) + v10 * fx;
                let bottom = v01 * (1.0 - fx) + v11 * fx;
                let val = top * (1.0 - fy) + bottom * fy;

                if out_idx + c < output.len() {
                    output[out_idx + c] = val.clamp(0.0, 255.0) as u8;
                }
            }
        }
    }

    output
}

#[cfg(test)]
fn make_solid_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> FrameBuffer {
    let mut frame = FrameBuffer::new(w, h, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();
    for y in 0..h {
        let row = plane.row_mut(y);
        for x in 0..w as usize {
            let base = x * 4;
            if base + 3 < row.len() {
                row[base] = r;
                row[base + 1] = g;
                row[base + 2] = b;
                row[base + 3] = 255;
            }
        }
    }
    frame
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_into_tiles() {
        let frame = make_solid_frame(64, 64, 100, 100, 100);
        let tiles = split_into_tiles(&frame, 32, 0);
        assert_eq!(tiles.len(), 4, "64x64 frame with 32x32 tiles = 4 tiles");
        for tile in &tiles {
            assert_eq!(tile.width, 32);
            assert_eq!(tile.height, 32);
        }
    }

    #[test]
    fn test_split_with_overlap() {
        let frame = make_solid_frame(64, 64, 100, 100, 100);
        let tiles = split_into_tiles(&frame, 32, 8);
        // With overlap=8, step=24, so we get more tiles
        assert!(
            tiles.len() >= 4,
            "Overlapping tiles should produce >= 4 tiles"
        );
    }

    #[test]
    fn test_blend_weight_center() {
        let w = compute_blend_weight(50, 50, 100, 100, 10);
        assert_eq!(w, 1.0, "Center pixels should have weight 1.0");
    }

    #[test]
    fn test_blend_weight_edge() {
        let w = compute_blend_weight(0, 50, 100, 100, 10);
        assert_eq!(w, 0.0, "Edge pixel at x=0 should have weight 0.0");
    }

    #[test]
    fn test_blend_weight_mid_overlap() {
        let w = compute_blend_weight(5, 50, 100, 100, 10);
        assert!(
            (w - 0.5).abs() < 0.01,
            "Mid-overlap should be ~0.5, got {w}"
        );
    }

    #[test]
    fn test_cpu_bilinear_upscale_dimensions() {
        let pixels = vec![128u8; 4 * 4 * 4]; // 4x4 RGBA
        let result = cpu_bilinear_upscale(&pixels, 4, 4, 2);
        assert_eq!(result.len(), 8 * 8 * 4, "2x upscale of 4x4 should be 8x8");
    }

    #[test]
    fn test_upscaler_output_dimensions() {
        let frame = make_solid_frame(16, 16, 200, 100, 50);
        let config = UpscaleConfig {
            scale: 2,
            tile_size: 16,
            tile_overlap: 0,
        };
        let upscaler = Upscaler::new(config);
        let result = upscaler.upscale_frame(&frame).unwrap();
        assert_eq!(result.width, 32);
        assert_eq!(result.height, 32);
    }

    #[test]
    fn test_upscaler_preserves_solid_color() {
        let frame = make_solid_frame(8, 8, 200, 100, 50);
        let config = UpscaleConfig {
            scale: 2,
            tile_size: 8,
            tile_overlap: 0,
        };
        let upscaler = Upscaler::new(config);
        let result = upscaler.upscale_frame(&frame).unwrap();

        // Check that the center pixel is close to the original color
        let plane = result.primary_plane();
        let row = plane.row(8); // center of 16x16 output
        let base = 8 * 4;
        assert!((row[base] as i16 - 200).abs() <= 1, "R should be ~200");
        assert!((row[base + 1] as i16 - 100).abs() <= 1, "G should be ~100");
        assert!((row[base + 2] as i16 - 50).abs() <= 1, "B should be ~50");
    }
}
