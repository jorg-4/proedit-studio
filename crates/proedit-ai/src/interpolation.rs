//! Frame interpolation using RIFE model via ONNX Runtime.
//!
//! RIFE takes two frames and a timestep t ∈ [0,1] and produces an
//! intermediate frame. This enables AI-powered slow motion.
//!
//! Requires the `onnx` feature flag and a pre-downloaded RIFE model.

use crate::error::{AiError, AiResult};
use proedit_core::FrameBuffer;

/// Convert a FrameBuffer (RGBA8 with stride-aligned planes) to NCHW f32 tensor.
///
/// Output shape: `[1, 3, height, width]` with values in `[0.0, 1.0]`.
/// RGB channels only (alpha is discarded).
#[cfg(feature = "onnx")]
pub fn frame_to_nchw(frame: &FrameBuffer) -> AiResult<ndarray::ArrayD<f32>> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let mut arr = ndarray::Array4::<f32>::zeros((1, 3, h, w));

    let plane = frame.primary_plane();
    for y in 0..h {
        let row = plane.row(y as u32);
        for x in 0..w {
            let base = x * 4; // RGBA
            if base + 2 < row.len() {
                arr[[0, 0, y, x]] = row[base] as f32 / 255.0; // R
                arr[[0, 1, y, x]] = row[base + 1] as f32 / 255.0; // G
                arr[[0, 2, y, x]] = row[base + 2] as f32 / 255.0; // B
            }
        }
    }

    Ok(arr.into_dyn())
}

/// Convert an NCHW f32 tensor back to a FrameBuffer (RGBA8).
///
/// Expects input shape `[1, 3, height, width]` with values in `[0.0, 1.0]`.
/// Alpha channel is set to 255 (fully opaque).
#[cfg(feature = "onnx")]
pub fn nchw_to_frame(
    tensor: ndarray::ArrayViewD<'_, f32>,
    width: u32,
    height: u32,
) -> AiResult<FrameBuffer> {
    let w = width as usize;
    let h = height as usize;

    let mut frame = FrameBuffer::new(width, height, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();

    for y in 0..h {
        let row = plane.row_mut(y as u32);
        for x in 0..w {
            let base = x * 4;
            if base + 3 < row.len() {
                row[base] = (tensor[[0, 0, y, x]].clamp(0.0, 1.0) * 255.0) as u8; // R
                row[base + 1] = (tensor[[0, 1, y, x]].clamp(0.0, 1.0) * 255.0) as u8; // G
                row[base + 2] = (tensor[[0, 2, y, x]].clamp(0.0, 1.0) * 255.0) as u8; // B
                row[base + 3] = 255; // A
            }
        }
    }

    Ok(frame)
}

/// RIFE frame interpolation model wrapper.
#[cfg(feature = "onnx")]
pub struct RIFEInterpolator {
    session: crate::session::OnnxSession,
}

#[cfg(feature = "onnx")]
impl RIFEInterpolator {
    /// Load a RIFE model from an ONNX file.
    pub fn load(model_path: &std::path::Path) -> AiResult<Self> {
        let session =
            crate::session::OnnxSession::load(model_path, crate::model_manager::ModelId::RIFEv4)?;
        Ok(Self { session })
    }

    /// Interpolate between two frames at time `t ∈ [0.0, 1.0]`.
    ///
    /// `t = 0.0` returns a frame close to `frame_a`,
    /// `t = 1.0` returns a frame close to `frame_b`.
    pub fn interpolate(
        &self,
        frame_a: &FrameBuffer,
        frame_b: &FrameBuffer,
        t: f32,
    ) -> AiResult<FrameBuffer> {
        if frame_a.width != frame_b.width || frame_a.height != frame_b.height {
            return Err(AiError::PreprocessError(format!(
                "Frame size mismatch: {}x{} vs {}x{}",
                frame_a.width, frame_a.height, frame_b.width, frame_b.height
            )));
        }

        let w = frame_a.width;
        let h = frame_a.height;

        let tensor_a = frame_to_nchw(frame_a)?;
        let tensor_b = frame_to_nchw(frame_b)?;

        let timestep = ndarray::Array::from_shape_vec(ndarray::IxDyn(&[1, 1, 1, 1]), vec![t])
            .map_err(|e| AiError::PreprocessError(e.to_string()))?;

        let inputs = ort::inputs![
            "img0" => tensor_a.view(),
            "img1" => tensor_b.view(),
            "timestep" => timestep.view(),
        ]
        .map_err(AiError::OnnxError)?;

        let outputs = self.session.inner().run(inputs)?;

        let output_tensor = outputs[0]
            .try_extract_tensor::<f32>()
            .map_err(AiError::OnnxError)?;

        nchw_to_frame(output_tensor.view(), w, h)
    }
}

// ── Non-feature-gated test helpers ─────────────────────────────────

/// Convert a FrameBuffer to NCHW f32 tensor (non-onnx version for testing).
/// Returns a flat Vec<f32> in NCHW order.
pub fn frame_to_nchw_vec(frame: &FrameBuffer) -> Vec<f32> {
    let w = frame.width as usize;
    let h = frame.height as usize;
    let plane_size = h * w;
    let mut result = vec![0.0_f32; 3 * plane_size];

    let plane = frame.primary_plane();
    for y in 0..h {
        let row = plane.row(y as u32);
        for x in 0..w {
            let base = x * 4;
            let pixel = y * w + x;
            if base + 2 < row.len() {
                result[pixel] = row[base] as f32 / 255.0; // R
                result[plane_size + pixel] = row[base + 1] as f32 / 255.0; // G
                result[2 * plane_size + pixel] = row[base + 2] as f32 / 255.0; // B
            }
        }
    }

    result
}

/// Convert NCHW f32 vec back to a FrameBuffer (non-onnx version for testing).
pub fn nchw_vec_to_frame(data: &[f32], width: u32, height: u32) -> AiResult<FrameBuffer> {
    let w = width as usize;
    let h = height as usize;
    let plane_size = h * w;
    if data.len() < 3 * plane_size {
        return Err(AiError::PreprocessError(format!(
            "Tensor data too short: {} < {}",
            data.len(),
            3 * plane_size
        )));
    }

    let mut frame = FrameBuffer::new(width, height, proedit_core::PixelFormat::Rgba8);
    let plane = frame.primary_plane_mut();

    for y in 0..h {
        let row = plane.row_mut(y as u32);
        for x in 0..w {
            let base = x * 4;
            let pixel = y * w + x;
            if base + 3 < row.len() {
                row[base] = (data[pixel].clamp(0.0, 1.0) * 255.0) as u8; // R
                row[base + 1] = (data[plane_size + pixel].clamp(0.0, 1.0) * 255.0) as u8; // G
                row[base + 2] = (data[2 * plane_size + pixel].clamp(0.0, 1.0) * 255.0) as u8; // B
                row[base + 3] = 255; // A
            }
        }
    }

    Ok(frame)
}

/// Helper: create a solid RGBA8 frame for testing.
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
    fn test_frame_to_nchw_vec_dimensions() {
        let frame = make_solid_frame(64, 48, 128, 64, 32);
        let tensor = frame_to_nchw_vec(&frame);
        assert_eq!(tensor.len(), 3 * 48 * 64);
    }

    #[test]
    fn test_frame_to_nchw_vec_values() {
        let frame = make_solid_frame(2, 2, 255, 0, 128);
        let tensor = frame_to_nchw_vec(&frame);
        let plane_size = 2 * 2; // 4 pixels per channel plane
                                // R channel, first pixel
        assert!((tensor[0] - 1.0).abs() < 1e-3); // R = 255 → 1.0
                                                 // G channel, first pixel
        assert!((tensor[plane_size] - 0.0).abs() < 1e-3); // G = 0 → 0.0
                                                          // B channel, first pixel
        assert!((tensor[2 * plane_size] - 128.0 / 255.0).abs() < 1e-2); // B = 128
    }

    #[test]
    fn test_nchw_vec_roundtrip() {
        let frame = make_solid_frame(4, 4, 100, 150, 200);
        let tensor = frame_to_nchw_vec(&frame);
        let recovered = nchw_vec_to_frame(&tensor, 4, 4).expect("should convert back");

        let orig_plane = frame.primary_plane();
        let rec_plane = recovered.primary_plane();
        for y in 0..4 {
            let orig_row = orig_plane.row(y);
            let rec_row = rec_plane.row(y);
            for x in 0..4usize {
                let base = x * 4;
                for c in 0..4 {
                    let diff = (orig_row[base + c] as i16 - rec_row[base + c] as i16).abs();
                    assert!(
                        diff <= 1,
                        "Pixel mismatch at ({x},{y}) channel {c}: {} vs {}",
                        orig_row[base + c],
                        rec_row[base + c]
                    );
                }
            }
        }
    }

    #[test]
    fn test_various_frame_sizes() {
        for (w, h) in &[(64, 64), (32, 32), (128, 96), (1, 1)] {
            let frame = make_solid_frame(*w, *h, 42, 84, 126);
            let tensor = frame_to_nchw_vec(&frame);
            assert_eq!(tensor.len(), 3 * (*h as usize) * (*w as usize));
            let recovered = nchw_vec_to_frame(&tensor, *w, *h).expect("should convert back");
            assert_eq!(recovered.width, *w);
            assert_eq!(recovered.height, *h);
        }
    }

    #[test]
    #[ignore]
    fn test_interpolate_real_model() {
        // Requires RIFE ONNX model and onnx feature
    }
}
