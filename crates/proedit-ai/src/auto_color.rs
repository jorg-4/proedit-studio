//! Automatic color correction using CPU-based analysis.
//!
//! Provides auto white balance (gray-world assumption), auto levels
//! (histogram percentile stretch), and auto contrast.

use proedit_core::FrameBuffer;
use serde::{Deserialize, Serialize};

/// Per-channel levels adjustment.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LevelsAdjust {
    /// Black point per channel (R, G, B) in [0, 1].
    pub black_point: [f32; 3],
    /// White point per channel (R, G, B) in [0, 1].
    pub white_point: [f32; 3],
    /// Gamma per channel (R, G, B).
    pub gamma: [f32; 3],
}

impl Default for LevelsAdjust {
    fn default() -> Self {
        Self {
            black_point: [0.0; 3],
            white_point: [1.0; 3],
            gamma: [1.0; 3],
        }
    }
}

/// Combined color correction result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColorCorrection {
    /// RGB multipliers for white balance.
    pub white_balance_shift: [f32; 3],
    /// Levels adjustment.
    pub levels: LevelsAdjust,
    /// Contrast adjustment factor.
    pub contrast: f32,
}

impl Default for ColorCorrection {
    fn default() -> Self {
        Self {
            white_balance_shift: [1.0, 1.0, 1.0],
            levels: LevelsAdjust::default(),
            contrast: 1.0,
        }
    }
}

/// Auto white balance using gray-world assumption.
///
/// The gray-world assumption states that the average color of a scene
/// should be neutral gray. Returns RGB multipliers to shift the average
/// toward neutral.
pub fn auto_white_balance(frame: &FrameBuffer) -> [f32; 3] {
    let plane = frame.primary_plane();
    let w = frame.width as usize;
    let h = frame.height as usize;
    let total = (w * h) as f64;
    if total == 0.0 {
        return [1.0, 1.0, 1.0];
    }

    let mut sum_r = 0.0f64;
    let mut sum_g = 0.0f64;
    let mut sum_b = 0.0f64;

    for y in 0..h {
        let row = plane.row(y as u32);
        for x in 0..w {
            let i = x * 4;
            if i + 2 < row.len() {
                sum_r += row[i] as f64;
                sum_g += row[i + 1] as f64;
                sum_b += row[i + 2] as f64;
            }
        }
    }

    let avg_r = sum_r / total;
    let avg_g = sum_g / total;
    let avg_b = sum_b / total;

    // Target: overall average luminance
    let avg_lum = (avg_r + avg_g + avg_b) / 3.0;

    if avg_lum < 1.0 {
        return [1.0, 1.0, 1.0];
    }

    [
        (avg_lum / avg_r).clamp(0.5, 2.0) as f32,
        (avg_lum / avg_g).clamp(0.5, 2.0) as f32,
        (avg_lum / avg_b).clamp(0.5, 2.0) as f32,
    ]
}

/// Auto levels using histogram percentile stretch.
///
/// Finds the 0.1% and 99.9% histogram percentiles per channel and
/// maps the range to [0, 1].
pub fn auto_levels(frame: &FrameBuffer) -> LevelsAdjust {
    let plane = frame.primary_plane();
    let w = frame.width as usize;
    let h = frame.height as usize;
    let total = w * h;
    if total == 0 {
        return LevelsAdjust::default();
    }

    let mut hist_r = [0u32; 256];
    let mut hist_g = [0u32; 256];
    let mut hist_b = [0u32; 256];

    for y in 0..h {
        let row = plane.row(y as u32);
        for x in 0..w {
            let i = x * 4;
            if i + 2 < row.len() {
                hist_r[row[i] as usize] += 1;
                hist_g[row[i + 1] as usize] += 1;
                hist_b[row[i + 2] as usize] += 1;
            }
        }
    }

    let low_pct = (total as f64 * 0.001) as u32;
    let high_pct = (total as f64 * 0.999) as u32;

    let find_percentile = |hist: &[u32; 256], target: u32| -> f32 {
        let mut cum = 0u32;
        for (i, &count) in hist.iter().enumerate() {
            cum += count;
            if cum >= target {
                return i as f32 / 255.0;
            }
        }
        1.0
    };

    let black_point = [
        find_percentile(&hist_r, low_pct),
        find_percentile(&hist_g, low_pct),
        find_percentile(&hist_b, low_pct),
    ];

    let white_point = [
        find_percentile(&hist_r, high_pct),
        find_percentile(&hist_g, high_pct),
        find_percentile(&hist_b, high_pct),
    ];

    LevelsAdjust {
        black_point,
        white_point,
        gamma: [1.0; 3],
    }
}

/// Auto contrast estimation.
///
/// Computes a contrast factor based on histogram equalization target.
/// Returns a multiplier (1.0 = no change, > 1.0 = increase contrast).
pub fn auto_contrast(frame: &FrameBuffer) -> f32 {
    let plane = frame.primary_plane();
    let w = frame.width as usize;
    let h = frame.height as usize;
    let total = w * h;
    if total == 0 {
        return 1.0;
    }

    let mut hist = [0u32; 256];
    for y in 0..h {
        let row = plane.row(y as u32);
        for x in 0..w {
            let i = x * 4;
            if i + 2 < row.len() {
                // Compute approximate luminance
                let lum = ((row[i] as u32 * 77 + row[i + 1] as u32 * 150 + row[i + 2] as u32 * 29)
                    >> 8) as usize;
                hist[lum.min(255)] += 1;
            }
        }
    }

    // Compute current standard deviation of luminance
    let mean: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| i as f64 * c as f64)
        .sum::<f64>()
        / total as f64;

    let variance: f64 = hist
        .iter()
        .enumerate()
        .map(|(i, &c)| {
            let diff = i as f64 - mean;
            diff * diff * c as f64
        })
        .sum::<f64>()
        / total as f64;

    let std_dev = variance.sqrt();

    // Target std dev for "good" contrast is roughly 64 (for 8-bit)
    let target_std = 64.0;
    if std_dev < 1.0 {
        return 1.0;
    }

    (target_std / std_dev).clamp(0.5, 3.0) as f32
}

/// Analyze a frame and produce a combined color correction.
pub fn analyze_and_correct(frame: &FrameBuffer) -> ColorCorrection {
    ColorCorrection {
        white_balance_shift: auto_white_balance(frame),
        levels: auto_levels(frame),
        contrast: auto_contrast(frame),
    }
}

/// Detect subject center using brightness/edge centroid fallback.
///
/// Used by SmartReframer when no face is detected.
pub fn detect_subject_center(frame: &FrameBuffer) -> Option<[f32; 2]> {
    let plane = frame.primary_plane();
    let w = frame.width as usize;
    let h = frame.height as usize;
    if w < 3 || h < 3 {
        return None;
    }

    let mut weight_sum = 0.0f64;
    let mut cx = 0.0f64;
    let mut cy = 0.0f64;

    // Use gradient magnitude as a proxy for "interesting" regions
    for y in 1..(h - 1) {
        let row_prev = plane.row((y - 1) as u32);
        let row_curr = plane.row(y as u32);
        let row_next = plane.row((y + 1) as u32);

        for x in 1..(w - 1) {
            let i = x * 4;
            if i + 6 < row_curr.len() {
                // Luminance at neighbors (approx)
                let lum = |r: &[u8], idx: usize| -> f64 {
                    (r[idx] as f64 * 0.299 + r[idx + 1] as f64 * 0.587 + r[idx + 2] as f64 * 0.114)
                        / 255.0
                };

                let gx = lum(row_curr, i + 4) - lum(row_curr, i - 4);
                let gy = lum(row_next, i) - lum(row_prev, i);
                let mag = (gx * gx + gy * gy).sqrt();

                if mag > 0.02 {
                    weight_sum += mag;
                    cx += x as f64 * mag;
                    cy += y as f64 * mag;
                }
            }
        }
    }

    if weight_sum < 1e-6 {
        return None;
    }

    Some([(cx / weight_sum) as f32, (cy / weight_sum) as f32])
}

#[cfg(test)]
mod tests {
    use super::*;
    use proedit_core::PixelFormat;

    fn make_solid_frame(r: u8, g: u8, b: u8, w: u32, h: u32) -> FrameBuffer {
        let mut frame = FrameBuffer::new(w, h, PixelFormat::Rgba8);
        let plane = frame.primary_plane_mut();
        for y in 0..h {
            let row = plane.row_mut(y);
            for x in 0..w as usize {
                let i = x * 4;
                row[i] = r;
                row[i + 1] = g;
                row[i + 2] = b;
                row[i + 3] = 255;
            }
        }
        frame
    }

    #[test]
    fn test_auto_white_balance_neutral() {
        let frame = make_solid_frame(128, 128, 128, 64, 64);
        let wb = auto_white_balance(&frame);
        // Neutral gray should yield multipliers near 1.0
        assert!((wb[0] - 1.0).abs() < 0.01);
        assert!((wb[1] - 1.0).abs() < 0.01);
        assert!((wb[2] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_auto_white_balance_color_cast() {
        // Blue-heavy image → expect red/green multipliers > 1.0
        let frame = make_solid_frame(100, 100, 200, 64, 64);
        let wb = auto_white_balance(&frame);
        assert!(wb[0] > 1.0, "Red multiplier should be > 1.0, got {}", wb[0]);
        assert!(
            wb[1] > 1.0,
            "Green multiplier should be > 1.0, got {}",
            wb[1]
        );
        assert!(
            wb[2] < 1.0,
            "Blue multiplier should be < 1.0, got {}",
            wb[2]
        );
    }

    #[test]
    fn test_auto_levels_full_range() {
        // Frame with pixel values spread across full range
        let mut frame = FrameBuffer::new(256, 1, PixelFormat::Rgba8);
        let plane = frame.primary_plane_mut();
        let row = plane.row_mut(0);
        for x in 0..256 {
            let i = x * 4;
            row[i] = x as u8;
            row[i + 1] = x as u8;
            row[i + 2] = x as u8;
            row[i + 3] = 255;
        }
        let levels = auto_levels(&frame);
        assert!(levels.black_point[0] < 0.01);
        assert!(levels.white_point[0] > 0.99);
    }

    #[test]
    fn test_auto_contrast() {
        // Solid color → zero std dev → returns 1.0 (no adjustment possible)
        let frame = make_solid_frame(128, 128, 128, 64, 64);
        let c = auto_contrast(&frame);
        assert!(
            (c - 1.0).abs() < 0.01,
            "Solid frame should return 1.0, got {}",
            c
        );
    }

    #[test]
    fn test_analyze_and_correct() {
        let frame = make_solid_frame(128, 128, 128, 32, 32);
        let correction = analyze_and_correct(&frame);
        assert!((correction.white_balance_shift[0] - 1.0).abs() < 0.01);
        assert_eq!(correction.levels.gamma, [1.0; 3]);
    }

    #[test]
    fn test_detect_subject_center_uniform() {
        // Uniform image → no edges → None
        let frame = make_solid_frame(128, 128, 128, 64, 64);
        let center = detect_subject_center(&frame);
        assert!(center.is_none());
    }

    #[test]
    fn test_detect_subject_center_bright_spot() {
        let mut frame = FrameBuffer::new(64, 64, PixelFormat::Rgba8);
        let plane = frame.primary_plane_mut();
        // Dark background
        for y in 0..64u32 {
            let row = plane.row_mut(y);
            for x in 0..64usize {
                let i = x * 4;
                row[i] = 10;
                row[i + 1] = 10;
                row[i + 2] = 10;
                row[i + 3] = 255;
            }
        }
        // Bright spot in upper-right quadrant
        for y in 15..25u32 {
            let row = plane.row_mut(y);
            for x in 40..50usize {
                let i = x * 4;
                row[i] = 255;
                row[i + 1] = 255;
                row[i + 2] = 255;
            }
        }
        let center = detect_subject_center(&frame);
        assert!(center.is_some());
        let [cx, cy] = center.unwrap();
        // Should be roughly in the upper-right quadrant area
        assert!(cx > 32.0, "Expected cx > 32, got {}", cx);
        assert!(cy < 32.0, "Expected cy < 32, got {}", cy);
    }
}
