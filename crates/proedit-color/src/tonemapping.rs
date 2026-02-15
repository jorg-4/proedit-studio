//! Tone mapping operators for HDR→SDR conversion.

use serde::{Deserialize, Serialize};

/// Available tone mapping operators.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToneMapOperator {
    Reinhard,
    AcesFilmic,
    Hable,
    AgX,
}

impl ToneMapOperator {
    /// Apply the tone map operator to an HDR RGB triplet.
    /// Input is linear light, output is [0, 1] range.
    pub fn apply(&self, hdr: [f32; 3]) -> [f32; 3] {
        match self {
            Self::Reinhard => reinhard(hdr),
            Self::AcesFilmic => aces_filmic(hdr),
            Self::Hable => hable(hdr),
            Self::AgX => agx(hdr),
        }
    }

    /// Display name.
    pub fn name(&self) -> &str {
        match self {
            Self::Reinhard => "Reinhard",
            Self::AcesFilmic => "ACES Filmic",
            Self::Hable => "Hable (Uncharted 2)",
            Self::AgX => "AgX",
        }
    }
}

/// Reinhard tone mapping: rgb / (1 + rgb).
fn reinhard(hdr: [f32; 3]) -> [f32; 3] {
    [
        hdr[0] / (1.0 + hdr[0]),
        hdr[1] / (1.0 + hdr[1]),
        hdr[2] / (1.0 + hdr[2]),
    ]
}

/// ACES filmic approximation (Narkowicz 2015).
fn aces_filmic(hdr: [f32; 3]) -> [f32; 3] {
    fn aces_channel(x: f32) -> f32 {
        let a = 2.51;
        let b = 0.03;
        let c = 2.43;
        let d = 0.59;
        let e = 0.14;
        ((x * (a * x + b)) / (x * (c * x + d) + e)).clamp(0.0, 1.0)
    }
    [
        aces_channel(hdr[0]),
        aces_channel(hdr[1]),
        aces_channel(hdr[2]),
    ]
}

/// Hable (Uncharted 2) tone mapping.
fn hable(hdr: [f32; 3]) -> [f32; 3] {
    fn hable_partial(x: f32) -> f32 {
        let a = 0.15;
        let b = 0.50;
        let c = 0.10;
        let d = 0.20;
        let e = 0.02;
        let f = 0.30;
        ((x * (a * x + c * b) + d * e) / (x * (a * x + b) + d * f)) - e / f
    }

    let exposure_bias = 2.0;
    let white = 11.2;
    let w = hable_partial(white);

    [
        (hable_partial(hdr[0] * exposure_bias) / w).clamp(0.0, 1.0),
        (hable_partial(hdr[1] * exposure_bias) / w).clamp(0.0, 1.0),
        (hable_partial(hdr[2] * exposure_bias) / w).clamp(0.0, 1.0),
    ]
}

/// AgX tone mapping (simplified).
fn agx(hdr: [f32; 3]) -> [f32; 3] {
    // Simplified AgX: apply a log-domain curve
    fn agx_channel(x: f32) -> f32 {
        let x = x.max(1e-6);
        let log_x = x.log2().clamp(-10.0, 6.5);
        // Map log range [-10, 6.5] to [0, 1]
        let t = (log_x + 10.0) / 16.5;
        // Sigmoid curve
        let s = t * t * (3.0 - 2.0 * t);
        s.clamp(0.0, 1.0)
    }
    [
        agx_channel(hdr[0]),
        agx_channel(hdr[1]),
        agx_channel(hdr[2]),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reinhard_black() {
        let result = ToneMapOperator::Reinhard.apply([0.0, 0.0, 0.0]);
        assert_eq!(result, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn test_reinhard_maps_to_under_one() {
        let result = ToneMapOperator::Reinhard.apply([10.0, 10.0, 10.0]);
        assert!(result[0] < 1.0);
        assert!(result[0] > 0.9); // 10/(1+10) ≈ 0.909
    }

    #[test]
    fn test_aces_black() {
        let result = ToneMapOperator::AcesFilmic.apply([0.0, 0.0, 0.0]);
        assert!(result[0] < 0.01);
    }

    #[test]
    fn test_aces_bounded() {
        let result = ToneMapOperator::AcesFilmic.apply([100.0, 100.0, 100.0]);
        assert!(result[0] <= 1.0);
        assert!(result[0] >= 0.0);
    }

    #[test]
    fn test_hable_bounded() {
        let result = ToneMapOperator::Hable.apply([100.0, 100.0, 100.0]);
        assert!(result[0] <= 1.0);
        assert!(result[0] >= 0.0);
    }

    #[test]
    fn test_agx_bounded() {
        let result = ToneMapOperator::AgX.apply([100.0, 100.0, 100.0]);
        assert!(result[0] <= 1.0);
        assert!(result[0] >= 0.0);
    }

    #[test]
    fn test_all_operators_monotonic() {
        for op in [
            ToneMapOperator::Reinhard,
            ToneMapOperator::AcesFilmic,
            ToneMapOperator::Hable,
            ToneMapOperator::AgX,
        ] {
            let low = op.apply([0.1, 0.1, 0.1]);
            let high = op.apply([1.0, 1.0, 1.0]);
            assert!(
                high[0] > low[0],
                "{:?} is not monotonic: {:.4} vs {:.4}",
                op,
                low[0],
                high[0]
            );
        }
    }
}
