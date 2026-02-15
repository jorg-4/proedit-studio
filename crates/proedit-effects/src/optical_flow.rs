//! Horn-Schunck optical flow with multi-scale pyramid.

use serde::{Deserialize, Serialize};

/// A 2D flow field storing (dx, dy) per pixel.
#[derive(Debug, Clone)]
pub struct FlowField {
    pub width: u32,
    pub height: u32,
    /// Per-pixel displacement vectors [dx, dy].
    pub data: Vec<[f32; 2]>,
}

/// Parameters for optical flow computation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowParams {
    /// Number of Horn-Schunck iterations.
    pub iterations: u32,
    /// Smoothness weight (Lagrange multiplier).
    pub alpha: f32,
    /// Number of multi-scale pyramid levels.
    pub pyramid_levels: u32,
}

impl Default for FlowParams {
    fn default() -> Self {
        Self {
            iterations: 100,
            alpha: 1.0,
            pyramid_levels: 3,
        }
    }
}

impl FlowField {
    /// Create a zero flow field.
    pub fn zeros(w: u32, h: u32) -> Self {
        Self {
            width: w,
            height: h,
            data: vec![[0.0, 0.0]; (w * h) as usize],
        }
    }

    /// Compute optical flow between two grayscale frames using Horn-Schunck.
    /// Input frames are RGBA u8.
    pub fn compute(prev: &[u8], next: &[u8], w: u32, h: u32, params: &FlowParams) -> Self {
        // Convert to grayscale
        let prev_gray = Self::to_gray(prev, w, h);
        let next_gray = Self::to_gray(next, w, h);

        if params.pyramid_levels <= 1 {
            return Self::horn_schunck(&prev_gray, &next_gray, w, h, params);
        }

        // Multi-scale: build pyramids
        let mut prev_pyr = vec![prev_gray];
        let mut next_pyr = vec![next_gray];
        let mut pw = w;
        let mut ph = h;

        for _ in 1..params.pyramid_levels {
            pw = pw.div_ceil(2);
            ph = ph.div_ceil(2);
            prev_pyr.push(Self::downsample(prev_pyr.last().unwrap(), pw * 2, ph * 2));
            next_pyr.push(Self::downsample(next_pyr.last().unwrap(), pw * 2, ph * 2));
        }

        // Coarse to fine
        let mut flow = Self::zeros(pw, ph);

        for level in (0..prev_pyr.len()).rev() {
            let lw = if level == 0 { w } else { w >> level };
            let lh = if level == 0 { h } else { h >> level };

            if level < prev_pyr.len() - 1 {
                // Upsample flow from coarser level
                flow = Self::upsample_flow(&flow, lw, lh);
            }

            // Refine with Horn-Schunck
            let refined = Self::horn_schunck(&prev_pyr[level], &next_pyr[level], lw, lh, params);
            // Add refinement
            for i in 0..flow.data.len().min(refined.data.len()) {
                flow.data[i][0] += refined.data[i][0];
                flow.data[i][1] += refined.data[i][1];
            }
        }

        flow
    }

    /// Horn-Schunck optical flow on single scale.
    fn horn_schunck(prev: &[f32], next: &[f32], w: u32, h: u32, params: &FlowParams) -> Self {
        let size = (w * h) as usize;
        let mut u = vec![0.0f32; size]; // horizontal flow
        let mut v = vec![0.0f32; size]; // vertical flow

        // Compute spatial and temporal gradients
        let (ix, iy, it) = Self::compute_gradients(prev, next, w, h);

        let alpha_sq = params.alpha * params.alpha;

        for _ in 0..params.iterations {
            let u_prev = u.clone();
            let v_prev = v.clone();

            for y in 1..(h as i32 - 1) {
                for x in 1..(w as i32 - 1) {
                    let idx = (y * w as i32 + x) as usize;

                    // Laplacian (4-neighbor average)
                    let u_avg = (u_prev[idx - 1]
                        + u_prev[idx + 1]
                        + u_prev[idx - w as usize]
                        + u_prev[idx + w as usize])
                        * 0.25;
                    let v_avg = (v_prev[idx - 1]
                        + v_prev[idx + 1]
                        + v_prev[idx - w as usize]
                        + v_prev[idx + w as usize])
                        * 0.25;

                    let ix_val = ix[idx];
                    let iy_val = iy[idx];
                    let it_val = it[idx];

                    let denom = alpha_sq + ix_val * ix_val + iy_val * iy_val;
                    let common = (ix_val * u_avg + iy_val * v_avg + it_val) / denom;

                    u[idx] = u_avg - ix_val * common;
                    v[idx] = v_avg - iy_val * common;
                }
            }
        }

        let data: Vec<[f32; 2]> = u.into_iter().zip(v).map(|(dx, dy)| [dx, dy]).collect();
        Self {
            width: w,
            height: h,
            data,
        }
    }

    /// Compute image gradients Ix, Iy, It.
    fn compute_gradients(
        prev: &[f32],
        next: &[f32],
        w: u32,
        h: u32,
    ) -> (Vec<f32>, Vec<f32>, Vec<f32>) {
        let size = (w * h) as usize;
        let mut ix = vec![0.0f32; size];
        let mut iy = vec![0.0f32; size];
        let mut it = vec![0.0f32; size];

        for y in 0..(h as i32 - 1) {
            for x in 0..(w as i32 - 1) {
                let idx = (y * w as i32 + x) as usize;
                let idx_r = idx + 1;
                let idx_d = idx + w as usize;
                let idx_rd = idx_d + 1;

                // Average spatial gradients over 2x2 block
                ix[idx] = 0.25
                    * ((prev[idx_r] - prev[idx])
                        + (prev[idx_rd] - prev[idx_d])
                        + (next[idx_r] - next[idx])
                        + (next[idx_rd] - next[idx_d]));
                iy[idx] = 0.25
                    * ((prev[idx_d] - prev[idx])
                        + (prev[idx_rd] - prev[idx_r])
                        + (next[idx_d] - next[idx])
                        + (next[idx_rd] - next[idx_r]));
                it[idx] = 0.25
                    * ((next[idx] - prev[idx])
                        + (next[idx_r] - prev[idx_r])
                        + (next[idx_d] - prev[idx_d])
                        + (next[idx_rd] - prev[idx_rd]));
            }
        }

        (ix, iy, it)
    }

    /// Warp a frame using the flow field.
    /// Scale multiplies the flow vectors (e.g., 0.5 for half-step interpolation).
    pub fn warp_frame(&self, frame: &[u8], w: u32, h: u32, scale: f32) -> Vec<u8> {
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];

        for y in 0..h {
            for x in 0..w {
                let idx = (y * w + x) as usize;
                let flow = self.data.get(idx).copied().unwrap_or([0.0, 0.0]);
                let src_x = x as f32 + flow[0] * scale;
                let src_y = y as f32 + flow[1] * scale;

                // Bilinear interpolation
                let sx = src_x.floor() as i32;
                let sy = src_y.floor() as i32;
                let fx = src_x - sx as f32;
                let fy = src_y - sy as f32;

                let dst_idx = idx * 4;
                for c in 0..4 {
                    let sample = |px: i32, py: i32| -> f32 {
                        let px = px.clamp(0, w as i32 - 1) as u32;
                        let py = py.clamp(0, h as i32 - 1) as u32;
                        let si = ((py * w + px) * 4 + c as u32) as usize;
                        frame.get(si).copied().unwrap_or(0) as f32
                    };

                    let v00 = sample(sx, sy);
                    let v10 = sample(sx + 1, sy);
                    let v01 = sample(sx, sy + 1);
                    let v11 = sample(sx + 1, sy + 1);

                    let v = v00 * (1.0 - fx) * (1.0 - fy)
                        + v10 * fx * (1.0 - fy)
                        + v01 * (1.0 - fx) * fy
                        + v11 * fx * fy;

                    out[dst_idx + c] = v.clamp(0.0, 255.0) as u8;
                }
            }
        }
        out
    }

    /// Get the flow magnitude at a pixel.
    pub fn magnitude_at(&self, x: u32, y: u32) -> f32 {
        let idx = (y * self.width + x) as usize;
        let flow = self.data.get(idx).copied().unwrap_or([0.0, 0.0]);
        (flow[0] * flow[0] + flow[1] * flow[1]).sqrt()
    }

    /// Visualize the flow field as an HSV color-coded RGBA image.
    pub fn visualize(&self) -> Vec<u8> {
        let size = (self.width * self.height) as usize;
        let mut out = vec![0u8; size * 4];

        // Find max magnitude for normalization
        let max_mag = self
            .data
            .iter()
            .map(|f| (f[0] * f[0] + f[1] * f[1]).sqrt())
            .fold(0.0f32, f32::max)
            .max(0.001);

        for i in 0..size {
            let [dx, dy] = self.data[i];
            let mag = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx); // -PI to PI

            // HSV to RGB: H = angle, S = 1, V = mag/max_mag
            let h = (angle + std::f32::consts::PI) / (2.0 * std::f32::consts::PI) * 6.0;
            let v = (mag / max_mag).min(1.0);

            let hi = h.floor() as i32 % 6;
            let f = h - h.floor();
            let q = v * (1.0 - f);
            let t = v * f;

            let (r, g, b) = match hi {
                0 => (v, t, 0.0),
                1 => (q, v, 0.0),
                2 => (0.0, v, t),
                3 => (0.0, q, v),
                4 => (t, 0.0, v),
                _ => (v, 0.0, q),
            };

            let idx = i * 4;
            out[idx] = (r * 255.0) as u8;
            out[idx + 1] = (g * 255.0) as u8;
            out[idx + 2] = (b * 255.0) as u8;
            out[idx + 3] = 255;
        }
        out
    }

    // Helper: convert RGBA to grayscale f32
    #[allow(clippy::needless_range_loop)]
    fn to_gray(rgba: &[u8], w: u32, h: u32) -> Vec<f32> {
        let size = (w * h) as usize;
        let mut gray = vec![0.0f32; size];
        for i in 0..size {
            let idx = i * 4;
            if idx + 2 < rgba.len() {
                gray[i] = (0.299 * rgba[idx] as f32
                    + 0.587 * rgba[idx + 1] as f32
                    + 0.114 * rgba[idx + 2] as f32)
                    / 255.0;
            }
        }
        gray
    }

    // Helper: downsample by 2x with averaging
    fn downsample(img: &[f32], src_w: u32, src_h: u32) -> Vec<f32> {
        let dw = src_w.div_ceil(2);
        let dh = src_h.div_ceil(2);
        let mut out = vec![0.0f32; (dw * dh) as usize];

        for y in 0..dh {
            for x in 0..dw {
                let sx = (x * 2) as usize;
                let sy = (y * 2) as usize;
                let sw = src_w as usize;

                let mut sum = 0.0f32;
                let mut count = 0;
                for dy in 0..2 {
                    for dx in 0..2 {
                        let px = sx + dx;
                        let py = sy + dy;
                        if px < src_w as usize && py < src_h as usize {
                            sum += img[py * sw + px];
                            count += 1;
                        }
                    }
                }
                out[(y * dw + x) as usize] = if count > 0 { sum / count as f32 } else { 0.0 };
            }
        }
        out
    }

    // Helper: upsample flow by 2x with bilinear interpolation
    fn upsample_flow(flow: &FlowField, new_w: u32, new_h: u32) -> FlowField {
        let mut out = FlowField::zeros(new_w, new_h);
        let scale_x = flow.width as f32 / new_w as f32;
        let scale_y = flow.height as f32 / new_h as f32;

        for y in 0..new_h {
            for x in 0..new_w {
                let src_x = x as f32 * scale_x;
                let src_y = y as f32 * scale_y;
                let sx = (src_x as u32).min(flow.width.saturating_sub(1));
                let sy = (src_y as u32).min(flow.height.saturating_sub(1));
                let idx = (sy * flow.width + sx) as usize;
                let f = flow.data.get(idx).copied().unwrap_or([0.0, 0.0]);
                out.data[(y * new_w + x) as usize] = [f[0] * 2.0, f[1] * 2.0]; // Scale flow vectors
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_solid(w: u32, h: u32, val: u8) -> Vec<u8> {
        vec![val; (w * h * 4) as usize]
    }

    #[test]
    fn test_zero_flow_on_identical_frames() {
        let w = 16;
        let h = 16;
        let frame = make_solid(w, h, 128);
        let params = FlowParams {
            iterations: 10,
            alpha: 1.0,
            pyramid_levels: 1,
        };
        let flow = FlowField::compute(&frame, &frame, w, h, &params);
        assert_eq!(flow.width, w);
        assert_eq!(flow.height, h);
        // Flow should be near zero for identical frames
        for f in &flow.data {
            assert!(f[0].abs() < 0.1 && f[1].abs() < 0.1);
        }
    }

    #[test]
    fn test_warp_identity() {
        let w = 8;
        let h = 8;
        let frame = make_solid(w, h, 100);
        let flow = FlowField::zeros(w, h);
        let warped = flow.warp_frame(&frame, w, h, 1.0);
        // Zero flow -> output should match input
        assert_eq!(warped.len(), frame.len());
        for (a, b) in frame.iter().zip(warped.iter()) {
            assert!(((*a as i32) - (*b as i32)).abs() <= 1);
        }
    }

    #[test]
    fn test_visualize_size() {
        let flow = FlowField::zeros(10, 10);
        let vis = flow.visualize();
        assert_eq!(vis.len(), 10 * 10 * 4);
    }

    #[test]
    fn test_magnitude() {
        let mut flow = FlowField::zeros(4, 4);
        flow.data[0] = [3.0, 4.0];
        assert!((flow.magnitude_at(0, 0) - 5.0).abs() < 0.01);
    }
}
