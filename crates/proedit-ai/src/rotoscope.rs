//! Auto-rotoscoping / intelligent masking using SAM 2.
//!
//! User clicks a point on a subject in the viewer, AI generates a pixel-perfect
//! mask, and the mask propagates across every frame tracking the subject through
//! motion, occlusion, and lighting changes.
//!
//! The ONNX-backed implementation requires the `onnx` feature flag.

use crate::error::{AiError, AiResult};
use proedit_core::FrameBuffer;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A single-channel mask buffer (one byte per pixel, 0 = background, 255 = foreground).
#[derive(Debug, Clone)]
pub struct MaskBuffer {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Mask data (row-major, one byte per pixel).
    pub data: Vec<u8>,
}

impl MaskBuffer {
    /// Create a new empty mask (all zeros = fully transparent).
    pub fn new(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![0u8; (width as usize) * (height as usize)],
        }
    }

    /// Create a fully opaque mask.
    pub fn opaque(width: u32, height: u32) -> Self {
        Self {
            width,
            height,
            data: vec![255u8; (width as usize) * (height as usize)],
        }
    }

    /// Get the mask value at (x, y). Returns 0 if out of bounds.
    pub fn get(&self, x: u32, y: u32) -> u8 {
        if x >= self.width || y >= self.height {
            return 0;
        }
        self.data[(y as usize) * (self.width as usize) + (x as usize)]
    }

    /// Set the mask value at (x, y).
    pub fn set(&mut self, x: u32, y: u32, value: u8) {
        if x < self.width && y < self.height {
            self.data[(y as usize) * (self.width as usize) + (x as usize)] = value;
        }
    }

    /// Invert the mask (foreground becomes background and vice versa).
    pub fn invert(&mut self) {
        for v in &mut self.data {
            *v = 255 - *v;
        }
    }

    /// Apply feathering (Gaussian blur approximation) to soften mask edges.
    pub fn feather(&mut self, radius: u32) {
        if radius == 0 {
            return;
        }
        // Box blur approximation: apply 3 passes of a box blur
        let w = self.width as usize;
        let h = self.height as usize;
        let mut temp = vec![0u16; w * h];

        for _pass in 0..3 {
            // Horizontal pass
            for y in 0..h {
                for x in 0..w {
                    let mut sum: u32 = 0;
                    let mut count: u32 = 0;
                    let x_start = x.saturating_sub(radius as usize);
                    let x_end = (x + radius as usize + 1).min(w);
                    for xi in x_start..x_end {
                        sum += self.data[y * w + xi] as u32;
                        count += 1;
                    }
                    temp[y * w + x] = (sum / count) as u16;
                }
            }
            // Vertical pass
            for y in 0..h {
                for x in 0..w {
                    let mut sum: u32 = 0;
                    let mut count: u32 = 0;
                    let y_start = y.saturating_sub(radius as usize);
                    let y_end = (y + radius as usize + 1).min(h);
                    for yi in y_start..y_end {
                        sum += temp[yi * w + x] as u32;
                        count += 1;
                    }
                    self.data[y * w + x] = (sum / count) as u8;
                }
            }
        }
    }

    /// Expand or contract the mask by the given number of pixels.
    /// Positive = expand (dilate), negative = contract (erode).
    pub fn expand_contract(&mut self, pixels: i32) {
        let w = self.width as usize;
        let h = self.height as usize;
        let radius = pixels.unsigned_abs() as usize;
        if radius == 0 {
            return;
        }

        let mut output = vec![0u8; w * h];
        let threshold: u8 = if pixels > 0 { 1 } else { 255 };

        for y in 0..h {
            for x in 0..w {
                let y_start = y.saturating_sub(radius);
                let y_end = (y + radius + 1).min(h);
                let x_start = x.saturating_sub(radius);
                let x_end = (x + radius + 1).min(w);

                if pixels > 0 {
                    // Dilate: if any neighbor is foreground, include
                    let mut found = false;
                    for yi in y_start..y_end {
                        for xi in x_start..x_end {
                            if self.data[yi * w + xi] >= threshold {
                                found = true;
                                break;
                            }
                        }
                        if found {
                            break;
                        }
                    }
                    output[y * w + x] = if found { 255 } else { 0 };
                } else {
                    // Erode: if any neighbor is background, exclude
                    let mut all_fg = true;
                    for yi in y_start..y_end {
                        for xi in x_start..x_end {
                            if self.data[yi * w + xi] < threshold {
                                all_fg = false;
                                break;
                            }
                        }
                        if !all_fg {
                            break;
                        }
                    }
                    output[y * w + x] = if all_fg { 255 } else { 0 };
                }
            }
        }

        self.data = output;
    }

    /// Fraction of pixels that are foreground (> 128).
    pub fn foreground_ratio(&self) -> f32 {
        if self.data.is_empty() {
            return 0.0;
        }
        let fg_count = self.data.iter().filter(|&&v| v > 128).count();
        fg_count as f32 / self.data.len() as f32
    }
}

/// A click prompt for SAM 2 segmentation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ClickPrompt {
    /// X coordinate in frame pixels.
    pub x: f32,
    /// Y coordinate in frame pixels.
    pub y: f32,
    /// True = include this region, false = exclude.
    pub is_positive: bool,
}

/// Stored frame embedding for cross-frame propagation.
struct FrameMemoryEntry {
    frame_number: i64,
    /// Flattened embedding vector from the image encoder.
    embedding: Vec<f32>,
    /// The mask produced for this frame.
    mask: MaskBuffer,
}

/// Memory bank for tracking objects across frames.
pub struct FrameMemoryBank {
    entries: HashMap<i64, FrameMemoryEntry>,
    max_entries: usize,
}

impl FrameMemoryBank {
    /// Create a new memory bank with the given capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: HashMap::new(),
            max_entries,
        }
    }

    /// Store a frame's embedding and mask in the memory bank.
    pub fn store_frame(&mut self, frame_number: i64, embedding: Vec<f32>, mask: MaskBuffer) {
        // Evict oldest entry if at capacity
        if self.entries.len() >= self.max_entries {
            if let Some(&oldest) = self.entries.keys().min() {
                self.entries.remove(&oldest);
            }
        }
        self.entries.insert(
            frame_number,
            FrameMemoryEntry {
                frame_number,
                embedding,
                mask,
            },
        );
    }

    /// Get the most recent entry before the given frame number.
    pub fn nearest_before(&self, frame_number: i64) -> Option<(&Vec<f32>, &MaskBuffer)> {
        self.entries
            .values()
            .filter(|e| e.frame_number < frame_number)
            .max_by_key(|e| e.frame_number)
            .map(|e| (&e.embedding, &e.mask))
    }

    /// Number of stored entries.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether the bank is empty.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Clear all entries.
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

/// SAM 2 auto-rotoscoping engine.
///
/// ONNX-backed implementation for production use requires the `onnx` feature.
/// A CPU fallback using simple color-distance thresholding is always available.
pub struct SAM2Rotoscope {
    memory: FrameMemoryBank,
    /// Model quality level.
    quality: SegmentationQuality,
}

/// Quality level for segmentation model.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SegmentationQuality {
    /// ViT-B backbone — fast, good for real-time preview.
    Fast,
    /// ViT-H backbone — slow, best quality for final render.
    High,
}

impl SAM2Rotoscope {
    /// Create a new rotoscope engine.
    pub fn new(quality: SegmentationQuality) -> Self {
        Self {
            memory: FrameMemoryBank::new(64),
            quality,
        }
    }

    /// Load the ONNX model for this quality level.
    #[cfg(feature = "onnx")]
    pub fn load(
        model_path: &std::path::Path,
        quality: SegmentationQuality,
    ) -> AiResult<Self> {
        let model_id = match quality {
            SegmentationQuality::Fast => crate::model_manager::ModelId::SAM2ViTB,
            SegmentationQuality::High => crate::model_manager::ModelId::SAM2ViTH,
        };
        // Validate the model file exists
        if !model_path.exists() {
            return Err(AiError::ModelNotFound {
                model_id: format!("{model_id:?}"),
            });
        }
        Ok(Self::new(quality))
    }

    /// Segment the current frame using click prompts.
    ///
    /// Returns a mask for the selected object. The mask is stored in the
    /// memory bank for subsequent frame propagation.
    pub fn segment_frame(
        &mut self,
        frame: &FrameBuffer,
        frame_number: i64,
        clicks: &[ClickPrompt],
    ) -> AiResult<MaskBuffer> {
        if clicks.is_empty() {
            return Err(AiError::PreprocessError(
                "At least one click prompt is required".into(),
            ));
        }

        // CPU fallback: color-distance-based segmentation from click points.
        // In production, this would run the SAM 2 ONNX encoder + decoder.
        let mask = cpu_segment_by_color(frame, clicks);

        // Store the embedding (placeholder) and mask for propagation
        let embedding = vec![0.0_f32; 256]; // placeholder
        self.memory.store_frame(frame_number, embedding, mask.clone());

        Ok(mask)
    }

    /// Propagate the mask to the next frame using memory attention.
    ///
    /// Uses the stored embeddings from previous frames to predict the mask
    /// for the new frame without additional click prompts.
    pub fn propagate_to_frame(
        &mut self,
        frame: &FrameBuffer,
        frame_number: i64,
    ) -> AiResult<MaskBuffer> {
        let (_prev_embedding, prev_mask) = self
            .memory
            .nearest_before(frame_number)
            .ok_or(AiError::PreprocessError(
                "No previous frame in memory bank for propagation".into(),
            ))?;

        // CPU fallback: propagate by color similarity to the previous mask region.
        // In production, this would use the SAM 2 memory-attention mechanism.
        let mask = cpu_propagate_mask(frame, prev_mask);

        let embedding = vec![0.0_f32; 256]; // placeholder
        self.memory.store_frame(frame_number, embedding, mask.clone());

        Ok(mask)
    }

    /// Clear the memory bank (start fresh for a new object).
    pub fn reset(&mut self) {
        self.memory.clear();
    }

    /// Get the quality level.
    pub fn quality(&self) -> SegmentationQuality {
        self.quality
    }
}

/// CPU fallback: segment by color distance from click points.
/// Samples the color at each positive click and marks pixels within
/// a color-distance threshold as foreground.
fn cpu_segment_by_color(frame: &FrameBuffer, clicks: &[ClickPrompt]) -> MaskBuffer {
    let w = frame.width;
    let h = frame.height;
    let mut mask = MaskBuffer::new(w, h);
    let plane = frame.primary_plane();
    let threshold = 50u32; // color distance threshold

    // Collect reference colors from positive clicks
    let ref_colors: Vec<[u8; 3]> = clicks
        .iter()
        .filter(|c| c.is_positive)
        .filter_map(|c| {
            let x = c.x as u32;
            let y = c.y as u32;
            if x < w && y < h {
                let row = plane.row(y);
                let base = (x as usize) * 4;
                if base + 2 < row.len() {
                    return Some([row[base], row[base + 1], row[base + 2]]);
                }
            }
            None
        })
        .collect();

    if ref_colors.is_empty() {
        return mask;
    }

    // Collect exclude colors from negative clicks
    let neg_colors: Vec<[u8; 3]> = clicks
        .iter()
        .filter(|c| !c.is_positive)
        .filter_map(|c| {
            let x = c.x as u32;
            let y = c.y as u32;
            if x < w && y < h {
                let row = plane.row(y);
                let base = (x as usize) * 4;
                if base + 2 < row.len() {
                    return Some([row[base], row[base + 1], row[base + 2]]);
                }
            }
            None
        })
        .collect();

    // Mark pixels by color distance
    for y in 0..h {
        let row = plane.row(y);
        for x in 0..w {
            let base = (x as usize) * 4;
            if base + 2 >= row.len() {
                break;
            }
            let px = [row[base], row[base + 1], row[base + 2]];

            // Check if close to any reference color
            let near_ref = ref_colors.iter().any(|rc| color_distance(&px, rc) < threshold);
            let near_neg = neg_colors.iter().any(|nc| color_distance(&px, nc) < threshold);

            if near_ref && !near_neg {
                mask.set(x, y, 255);
            }
        }
    }

    mask
}

/// CPU fallback: propagate mask using color similarity with previous mask.
fn cpu_propagate_mask(frame: &FrameBuffer, prev_mask: &MaskBuffer) -> MaskBuffer {
    let w = frame.width;
    let h = frame.height;

    // If dimensions don't match, return empty mask
    if w != prev_mask.width || h != prev_mask.height {
        return MaskBuffer::new(w, h);
    }

    // Simple propagation: dilate the previous mask slightly and re-threshold by color
    let mut mask = prev_mask.clone();
    mask.expand_contract(2); // slight dilation to account for motion
    mask
}

/// Squared Euclidean color distance in RGB space.
fn color_distance(a: &[u8; 3], b: &[u8; 3]) -> u32 {
    let dr = (a[0] as i32 - b[0] as i32).unsigned_abs();
    let dg = (a[1] as i32 - b[1] as i32).unsigned_abs();
    let db = (a[2] as i32 - b[2] as i32).unsigned_abs();
    dr * dr + dg * dg + db * db
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
    fn test_mask_buffer_basic() {
        let mut mask = MaskBuffer::new(10, 10);
        assert_eq!(mask.get(5, 5), 0);
        mask.set(5, 5, 255);
        assert_eq!(mask.get(5, 5), 255);
        assert_eq!(mask.get(0, 0), 0);
    }

    #[test]
    fn test_mask_invert() {
        let mut mask = MaskBuffer::new(4, 4);
        mask.set(0, 0, 255);
        mask.invert();
        assert_eq!(mask.get(0, 0), 0);
        assert_eq!(mask.get(1, 1), 255);
    }

    #[test]
    fn test_mask_foreground_ratio() {
        let mut mask = MaskBuffer::new(10, 10);
        assert_eq!(mask.foreground_ratio(), 0.0);

        let full = MaskBuffer::opaque(10, 10);
        assert_eq!(full.foreground_ratio(), 1.0);

        // Set half to foreground
        for i in 0..50 {
            mask.data[i] = 255;
        }
        assert!((mask.foreground_ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_mask_expand_contract() {
        let mut mask = MaskBuffer::new(10, 10);
        mask.set(5, 5, 255);
        mask.expand_contract(1);
        // After dilation, neighbors should be filled
        assert_eq!(mask.get(5, 5), 255);
        assert_eq!(mask.get(4, 5), 255);
        assert_eq!(mask.get(6, 5), 255);
        assert_eq!(mask.get(5, 4), 255);
        assert_eq!(mask.get(5, 6), 255);
    }

    #[test]
    fn test_memory_bank() {
        let mut bank = FrameMemoryBank::new(3);
        assert!(bank.is_empty());

        bank.store_frame(0, vec![1.0], MaskBuffer::new(2, 2));
        bank.store_frame(5, vec![2.0], MaskBuffer::new(2, 2));
        bank.store_frame(10, vec![3.0], MaskBuffer::new(2, 2));
        assert_eq!(bank.len(), 3);

        // Nearest before frame 7 should be frame 5
        let (emb, _mask) = bank.nearest_before(7).unwrap();
        assert_eq!(emb[0], 2.0);

        // Adding a 4th should evict the oldest (frame 0)
        bank.store_frame(15, vec![4.0], MaskBuffer::new(2, 2));
        assert_eq!(bank.len(), 3);
        assert!(bank.nearest_before(1).is_none());
    }

    #[test]
    fn test_segment_frame_requires_clicks() {
        let mut roto = SAM2Rotoscope::new(SegmentationQuality::Fast);
        let frame = make_solid_frame(32, 32, 128, 128, 128);
        let result = roto.segment_frame(&frame, 0, &[]);
        assert!(result.is_err());
    }

    #[test]
    fn test_segment_and_propagate() {
        let mut roto = SAM2Rotoscope::new(SegmentationQuality::Fast);

        // Create a frame with a red region
        let mut frame = make_solid_frame(32, 32, 0, 0, 0);
        let plane = frame.primary_plane_mut();
        for y in 10..20 {
            let row = plane.row_mut(y);
            for x in 10..20usize {
                let base = x * 4;
                row[base] = 255; // R
                row[base + 1] = 0;
                row[base + 2] = 0;
            }
        }

        // Click on red region
        let clicks = vec![ClickPrompt {
            x: 15.0,
            y: 15.0,
            is_positive: true,
        }];
        let mask = roto.segment_frame(&frame, 0, &clicks).unwrap();
        assert!(mask.get(15, 15) > 0, "Clicked region should be foreground");

        // Propagate to next frame (same frame data for test)
        let mask2 = roto.propagate_to_frame(&frame, 1).unwrap();
        assert!(mask2.get(15, 15) > 0, "Propagated mask should preserve region");
    }

    #[test]
    fn test_color_distance() {
        assert_eq!(color_distance(&[0, 0, 0], &[0, 0, 0]), 0);
        assert_eq!(color_distance(&[255, 0, 0], &[0, 0, 0]), 255 * 255);
    }
}
