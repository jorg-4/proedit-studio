//! Chroma keying (green/blue screen) with matte processing pipeline.

use serde::{Deserialize, Serialize};

/// Parameters for chroma key extraction.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChromaKeyParams {
    /// Key color in YCbCr space [Y, Cb, Cr]
    pub key_color: [f32; 3],
    /// Color distance tolerance (0.0-1.0)
    pub tolerance: f32,
    /// Edge softness (0.0-1.0)
    pub softness: f32,
    /// Spill suppression strength (0.0-1.0)
    pub spill_suppression: f32,
    /// Edge erosion amount (pixels)
    pub edge_thin: f32,
    /// Edge blur radius (pixels)
    pub edge_feather: f32,
    /// Light wrap intensity (0.0-1.0)
    pub light_wrap_intensity: f32,
}

impl Default for ChromaKeyParams {
    fn default() -> Self {
        Self {
            key_color: [0.0, 0.0, 0.0], // Will be set to green screen default
            tolerance: 0.3,
            softness: 0.1,
            spill_suppression: 0.5,
            edge_thin: 0.0,
            edge_feather: 1.0,
            light_wrap_intensity: 0.0,
        }
    }
}

impl ChromaKeyParams {
    /// Green screen default.
    pub fn green_screen() -> Self {
        Self {
            key_color: [0.587, -0.331, -0.419], // Pure green in YCbCr
            tolerance: 0.35,
            softness: 0.1,
            spill_suppression: 0.6,
            ..Default::default()
        }
    }

    /// Blue screen default.
    pub fn blue_screen() -> Self {
        Self {
            key_color: [0.114, 0.500, -0.081], // Pure blue in YCbCr approx
            tolerance: 0.35,
            softness: 0.1,
            spill_suppression: 0.6,
            ..Default::default()
        }
    }
}

/// Chroma key processing pipeline.
pub struct ChromaKeyProcessor;

impl ChromaKeyProcessor {
    /// Convert RGB pixel to YCbCr.
    fn rgb_to_ycbcr(r: f32, g: f32, b: f32) -> [f32; 3] {
        let y = 0.299 * r + 0.587 * g + 0.114 * b;
        let cb = -0.168_736 * r - 0.331_264 * g + 0.5 * b;
        let cr = 0.5 * r - 0.418_688 * g - 0.081_312 * b;
        [y, cb, cr]
    }

    /// Extract a matte (alpha mask) from a frame based on chroma key distance.
    /// Input frame is RGBA u8, output is f32 matte (w*h).
    #[allow(clippy::needless_range_loop)]
    pub fn extract_matte(frame: &[u8], w: u32, h: u32, params: &ChromaKeyParams) -> Vec<f32> {
        let size = (w * h) as usize;
        let mut matte = vec![1.0f32; size];
        let tol = params.tolerance.max(0.001);
        let soft = params.softness.max(0.001);

        for i in 0..size {
            let idx = i * 4;
            if idx + 2 >= frame.len() {
                break;
            }
            let r = frame[idx] as f32 / 255.0;
            let g = frame[idx + 1] as f32 / 255.0;
            let b = frame[idx + 2] as f32 / 255.0;

            let ycbcr = Self::rgb_to_ycbcr(r, g, b);
            let dcb = ycbcr[1] - params.key_color[1];
            let dcr = ycbcr[2] - params.key_color[2];
            let dist = (dcb * dcb + dcr * dcr).sqrt();

            if dist < tol {
                matte[i] = 0.0;
            } else if dist < tol + soft {
                matte[i] = (dist - tol) / soft;
            }
            // else matte stays 1.0 (fully opaque)
        }
        matte
    }

    /// Clip matte black/white points.
    pub fn clip_black_white(matte: &mut [f32], black: f32, white: f32) {
        let range = (white - black).max(0.001);
        for v in matte.iter_mut() {
            *v = ((*v - black) / range).clamp(0.0, 1.0);
        }
    }

    /// Erode (negative amount) or dilate (positive amount) the matte.
    pub fn erode_dilate(matte: &mut [f32], w: u32, h: u32, amount: f32) {
        if amount.abs() < 0.01 {
            return;
        }
        let radius = amount.abs().ceil() as i32;
        let erode = amount < 0.0;
        let w = w as i32;
        let h = h as i32;
        let src = matte.to_vec();

        for y in 0..h {
            for x in 0..w {
                let mut val = if erode { 1.0f32 } else { 0.0f32 };
                for dy in -radius..=radius {
                    for dx in -radius..=radius {
                        let nx = x + dx;
                        let ny = y + dy;
                        if nx >= 0 && nx < w && ny >= 0 && ny < h {
                            let s = src[(ny * w + nx) as usize];
                            if erode {
                                val = val.min(s);
                            } else {
                                val = val.max(s);
                            }
                        }
                    }
                }
                matte[(y * w + x) as usize] = val;
            }
        }
    }

    /// Gaussian blur the matte.
    pub fn blur_matte(matte: &mut [f32], w: u32, h: u32, radius: f32) {
        if radius < 0.5 {
            return;
        }
        let r = radius.ceil() as i32;
        let sigma = radius / 3.0;
        let sigma2 = 2.0 * sigma * sigma;
        let w = w as i32;
        let h = h as i32;

        // Build 1D kernel
        let kernel_size = (2 * r + 1) as usize;
        let mut kernel = vec![0.0f32; kernel_size];
        let mut sum = 0.0f32;
        for i in 0..kernel_size as i32 {
            let d = (i - r) as f32;
            let v = (-d * d / sigma2).exp();
            kernel[i as usize] = v;
            sum += v;
        }
        for k in &mut kernel {
            *k /= sum;
        }

        // Horizontal pass
        let src = matte.to_vec();
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0f32;
                for k in 0..kernel_size as i32 {
                    let sx = (x + k - r).clamp(0, w - 1);
                    acc += src[(y * w + sx) as usize] * kernel[k as usize];
                }
                matte[(y * w + x) as usize] = acc;
            }
        }

        // Vertical pass
        let src = matte.to_vec();
        for y in 0..h {
            for x in 0..w {
                let mut acc = 0.0f32;
                for k in 0..kernel_size as i32 {
                    let sy = (y + k - r).clamp(0, h - 1);
                    acc += src[(sy * w + x) as usize] * kernel[k as usize];
                }
                matte[(y * w + x) as usize] = acc;
            }
        }
    }

    /// Remove color spill from the foreground.
    #[allow(clippy::needless_range_loop)]
    pub fn despill(frame: &mut [u8], matte: &[f32], w: u32, h: u32, params: &ChromaKeyParams) {
        let size = (w * h) as usize;
        let strength = params.spill_suppression;
        // Determine dominant key channel (green for green screen, blue for blue)
        let key_is_green = params.key_color[1].abs() > params.key_color[2].abs();

        for i in 0..size {
            let idx = i * 4;
            if idx + 2 >= frame.len() {
                break;
            }
            let spill_factor = 1.0 - matte[i]; // More spill removal where more transparent
            let r = frame[idx] as f32;
            let g = frame[idx + 1] as f32;
            let b = frame[idx + 2] as f32;

            if key_is_green {
                let avg = (r + b) * 0.5;
                let new_g = g - (g - avg).max(0.0) * strength * spill_factor;
                frame[idx + 1] = new_g.clamp(0.0, 255.0) as u8;
            } else {
                let avg = (r + g) * 0.5;
                let new_b = b - (b - avg).max(0.0) * strength * spill_factor;
                frame[idx + 2] = new_b.clamp(0.0, 255.0) as u8;
            }
        }
    }

    /// Composite foreground over background using the matte.
    /// All buffers are RGBA u8, matte is f32.
    #[allow(clippy::needless_range_loop)]
    pub fn composite(fg: &[u8], bg: &[u8], matte: &[f32], out: &mut [u8], w: u32, h: u32) {
        let size = (w * h) as usize;
        for i in 0..size {
            let idx = i * 4;
            if idx + 3 >= fg.len() || idx + 3 >= bg.len() || idx + 3 >= out.len() {
                break;
            }
            let alpha = matte[i];
            for c in 0..3 {
                out[idx + c] =
                    (fg[idx + c] as f32 * alpha + bg[idx + c] as f32 * (1.0 - alpha)) as u8;
            }
            out[idx + 3] = 255; // Full alpha in output
        }
    }

    /// Full chroma key pipeline: extract -> clip -> erode/dilate -> blur -> despill -> composite.
    pub fn process(fg: &[u8], bg: &[u8], w: u32, h: u32, params: &ChromaKeyParams) -> Vec<u8> {
        let mut matte = Self::extract_matte(fg, w, h, params);
        Self::clip_black_white(&mut matte, 0.0, 1.0);
        Self::erode_dilate(&mut matte, w, h, params.edge_thin);
        Self::blur_matte(&mut matte, w, h, params.edge_feather);

        let mut fg_copy = fg.to_vec();
        Self::despill(&mut fg_copy, &matte, w, h, params);

        let mut out = vec![0u8; fg.len()];
        Self::composite(&fg_copy, bg, &matte, &mut out, w, h);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solid_frame(w: u32, h: u32, r: u8, g: u8, b: u8) -> Vec<u8> {
        let size = (w * h) as usize;
        let mut frame = vec![0u8; size * 4];
        for i in 0..size {
            frame[i * 4] = r;
            frame[i * 4 + 1] = g;
            frame[i * 4 + 2] = b;
            frame[i * 4 + 3] = 255;
        }
        frame
    }

    #[test]
    fn test_green_screen_extraction() {
        let w = 4;
        let h = 4;
        let fg = make_solid_frame(w, h, 0, 255, 0); // Pure green
        let params = ChromaKeyParams::green_screen();
        let matte = ChromaKeyProcessor::extract_matte(&fg, w, h, &params);
        // Pure green should produce near-zero matte (transparent)
        for &v in &matte {
            assert!(v < 0.5, "Green pixel should be keyed out, got {v}");
        }
    }

    #[test]
    fn test_non_green_preserved() {
        let w = 4;
        let h = 4;
        let fg = make_solid_frame(w, h, 255, 0, 0); // Pure red
        let params = ChromaKeyParams::green_screen();
        let matte = ChromaKeyProcessor::extract_matte(&fg, w, h, &params);
        for &v in &matte {
            assert!(v > 0.5, "Red pixel should be preserved, got {v}");
        }
    }

    #[test]
    fn test_full_pipeline() {
        let w = 8;
        let h = 8;
        let fg = make_solid_frame(w, h, 0, 255, 0);
        let bg = make_solid_frame(w, h, 128, 64, 32);
        let params = ChromaKeyParams::green_screen();
        let result = ChromaKeyProcessor::process(&fg, &bg, w, h, &params);
        assert_eq!(result.len(), (w * h * 4) as usize);
        // Green keyed out -> should be close to background
        // (despill + composite on a pure green frame)
        assert!(result[3] == 255); // Alpha should be 255
    }

    #[test]
    fn test_clip_black_white() {
        let mut matte = vec![0.0, 0.25, 0.5, 0.75, 1.0];
        ChromaKeyProcessor::clip_black_white(&mut matte, 0.25, 0.75);
        assert!((matte[0] - 0.0).abs() < 0.01);
        assert!((matte[1] - 0.0).abs() < 0.01);
        assert!((matte[2] - 0.5).abs() < 0.01);
        assert!((matte[3] - 1.0).abs() < 0.01);
        assert!((matte[4] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_despill_no_crash() {
        let w = 4;
        let h = 4;
        let mut fg = make_solid_frame(w, h, 0, 255, 0);
        let matte = vec![0.5f32; (w * h) as usize];
        let params = ChromaKeyParams::green_screen();
        ChromaKeyProcessor::despill(&mut fg, &matte, w, h, &params);
        // Should not crash and green should be reduced
    }
}
