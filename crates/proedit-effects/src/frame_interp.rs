//! Optical flow-based frame interpolation (Twixtor-style).

use super::optical_flow::{FlowField, FlowParams};

/// Frame interpolator using bidirectional optical flow.
pub struct FrameInterpolator;

impl FrameInterpolator {
    /// Interpolate between two frames at parameter t (0.0 = frame_a, 1.0 = frame_b).
    pub fn interpolate(frame_a: &[u8], frame_b: &[u8], w: u32, h: u32, t: f32) -> Vec<u8> {
        let t = t.clamp(0.0, 1.0);

        // Shortcut for endpoints
        if t < 0.001 {
            return frame_a.to_vec();
        }
        if t > 0.999 {
            return frame_b.to_vec();
        }

        let flow_params = FlowParams {
            iterations: 50,
            alpha: 1.0,
            pyramid_levels: 2,
        };

        // Compute bidirectional flow
        let flow_ab = FlowField::compute(frame_a, frame_b, w, h, &flow_params);
        let flow_ba = FlowField::compute(frame_b, frame_a, w, h, &flow_params);

        // Warp frame_a forward by t and frame_b backward by (1-t)
        let warped_a = flow_ab.warp_frame(frame_a, w, h, t);
        let warped_b = flow_ba.warp_frame(frame_b, w, h, 1.0 - t);

        // Blend with cross-fade weighted by t
        let size = (w * h * 4) as usize;
        let mut out = vec![0u8; size];
        for i in 0..size {
            out[i] =
                (warped_a[i] as f32 * (1.0 - t) + warped_b[i] as f32 * t).clamp(0.0, 255.0) as u8;
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_frame(w: u32, h: u32, val: u8) -> Vec<u8> {
        vec![val; (w * h * 4) as usize]
    }

    #[test]
    fn test_interpolate_endpoints() {
        let w = 8;
        let h = 8;
        let a = make_frame(w, h, 50);
        let b = make_frame(w, h, 200);

        let at_zero = FrameInterpolator::interpolate(&a, &b, w, h, 0.0);
        assert_eq!(at_zero, a);

        let at_one = FrameInterpolator::interpolate(&a, &b, w, h, 1.0);
        assert_eq!(at_one, b);
    }

    #[test]
    fn test_interpolate_midpoint() {
        let w = 8;
        let h = 8;
        let a = make_frame(w, h, 50);
        let b = make_frame(w, h, 200);

        let mid = FrameInterpolator::interpolate(&a, &b, w, h, 0.5);
        assert_eq!(mid.len(), (w * h * 4) as usize);
        // Should be somewhere between a and b
        // (exact value depends on flow computation)
    }
}
