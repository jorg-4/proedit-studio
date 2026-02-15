//! ReelSmart Motion Blur (RSMB) - flow-based directional motion blur.

use super::optical_flow::{FlowField, FlowParams};
use serde::{Deserialize, Serialize};

/// Parameters for RSMB.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RSMBParams {
    /// Shutter angle in degrees (0-720). 180 = normal motion blur.
    pub shutter_angle: f32,
    /// Number of motion samples to accumulate.
    pub samples: u32,
}

impl Default for RSMBParams {
    fn default() -> Self {
        Self {
            shutter_angle: 180.0,
            samples: 16,
        }
    }
}

/// RSMB processor.
pub struct RSMBProcessor;

impl RSMBProcessor {
    /// Apply motion blur using optical flow between adjacent frames.
    /// Takes previous, current, and next frames (RGBA u8).
    pub fn apply(
        prev: &[u8],
        curr: &[u8],
        next: &[u8],
        w: u32,
        h: u32,
        params: &RSMBParams,
    ) -> Vec<u8> {
        let flow_params = FlowParams {
            iterations: 50,
            alpha: 1.0,
            pyramid_levels: 2,
        };

        // Compute forward and backward flow
        let flow_fwd = FlowField::compute(curr, next, w, h, &flow_params);
        let flow_bwd = FlowField::compute(curr, prev, w, h, &flow_params);

        let size = (w * h * 4) as usize;
        let mut accum = vec![0.0f32; size];
        let shutter_factor = params.shutter_angle / 360.0;
        let samples = params.samples.max(1);

        for s in 0..samples {
            let t = (s as f32 / samples as f32 - 0.5) * shutter_factor;
            let warped = if t >= 0.0 {
                flow_fwd.warp_frame(curr, w, h, t)
            } else {
                flow_bwd.warp_frame(curr, w, h, -t)
            };

            for i in 0..size {
                accum[i] += warped[i] as f32;
            }
        }

        // Average the accumulated samples
        let inv_samples = 1.0 / samples as f32;
        accum
            .iter()
            .map(|v| (v * inv_samples).clamp(0.0, 255.0) as u8)
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rsmb_output_size() {
        let w = 8;
        let h = 8;
        let frame = vec![128u8; (w * h * 4) as usize];
        let params = RSMBParams {
            shutter_angle: 180.0,
            samples: 4,
        };
        let result = RSMBProcessor::apply(&frame, &frame, &frame, w, h, &params);
        assert_eq!(result.len(), (w * h * 4) as usize);
    }

    #[test]
    fn test_rsmb_static_frame() {
        let w = 8;
        let h = 8;
        let frame = vec![100u8; (w * h * 4) as usize];
        let params = RSMBParams {
            shutter_angle: 180.0,
            samples: 4,
        };
        let result = RSMBProcessor::apply(&frame, &frame, &frame, w, h, &params);
        // Static frame -> no blur, output should be close to input
        for (a, b) in frame.iter().zip(result.iter()) {
            assert!((*a as i32 - *b as i32).abs() <= 2);
        }
    }
}
