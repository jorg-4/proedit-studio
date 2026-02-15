//! Transfer functions (OETF/EOTF) for various standards.
#![allow(clippy::excessive_precision)]

use serde::{Deserialize, Serialize};

/// Transfer function type.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum TransferFunction {
    SRGB,
    Rec709,
    PQ,
    HLG,
    Linear,
    Gamma(f32),
}

impl TransferFunction {
    /// Convert from non-linear (display/encoded) to linear light.
    pub fn to_linear(&self, v: f32) -> f32 {
        match self {
            Self::Linear => v,
            Self::SRGB => {
                if v <= 0.04045 {
                    v / 12.92
                } else {
                    ((v + 0.055) / 1.055).powf(2.4)
                }
            }
            Self::Rec709 => {
                if v < 0.081 {
                    v / 4.5
                } else {
                    ((v + 0.099) / 1.099).powf(1.0 / 0.45)
                }
            }
            Self::PQ => decode_pq(v),
            Self::HLG => decode_hlg(v),
            Self::Gamma(g) => {
                if v <= 0.0 {
                    0.0
                } else {
                    v.powf(*g)
                }
            }
        }
    }

    /// Convert from linear light to non-linear (display/encoded).
    pub fn from_linear(&self, v: f32) -> f32 {
        match self {
            Self::Linear => v,
            Self::SRGB => {
                if v <= 0.0031308 {
                    v * 12.92
                } else {
                    1.055 * v.powf(1.0 / 2.4) - 0.055
                }
            }
            Self::Rec709 => {
                if v < 0.018 {
                    v * 4.5
                } else {
                    1.099 * v.powf(0.45) - 0.099
                }
            }
            Self::PQ => encode_pq(v),
            Self::HLG => encode_hlg(v),
            Self::Gamma(g) => {
                if v <= 0.0 || *g == 0.0 {
                    0.0
                } else {
                    v.powf(1.0 / *g)
                }
            }
        }
    }

    /// Display name.
    pub fn name(&self) -> &str {
        match self {
            Self::SRGB => "sRGB",
            Self::Rec709 => "Rec. 709",
            Self::PQ => "PQ (ST.2084)",
            Self::HLG => "HLG (BT.2100)",
            Self::Linear => "Linear",
            Self::Gamma(_) => "Gamma",
        }
    }
}

// PQ (ST.2084) constants
const PQ_M1: f32 = 0.1593017578125;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.8359375;
const PQ_C2: f32 = 18.8515625;
const PQ_C3: f32 = 18.6875;

/// Encode linear luminance (in nits / 10000) to PQ [0, 1].
fn encode_pq(linear: f32) -> f32 {
    let y = linear.max(0.0);
    let ym1 = y.powf(PQ_M1);
    let num = PQ_C1 + PQ_C2 * ym1;
    let den = 1.0 + PQ_C3 * ym1;
    (num / den).powf(PQ_M2)
}

/// Decode PQ [0, 1] to linear luminance (in nits / 10000).
fn decode_pq(pq: f32) -> f32 {
    let pq = pq.max(0.0);
    let p = pq.powf(1.0 / PQ_M2);
    let num = (p - PQ_C1).max(0.0);
    let den = PQ_C2 - PQ_C3 * p;
    if den.abs() < 1e-10 {
        0.0
    } else {
        (num / den).powf(1.0 / PQ_M1)
    }
}

// HLG constants
const HLG_A: f32 = 0.17883277;
const HLG_B: f32 = 0.28466892;
const HLG_C: f32 = 0.55991073;

/// Encode scene-referred linear to HLG.
fn encode_hlg(linear: f32) -> f32 {
    let e = linear.max(0.0);
    if e <= 1.0 / 12.0 {
        (3.0 * e).sqrt()
    } else {
        HLG_A * (12.0 * e - HLG_B).ln() + HLG_C
    }
}

/// Decode HLG to scene-referred linear.
fn decode_hlg(hlg: f32) -> f32 {
    let e = hlg.max(0.0);
    if e <= 0.5 {
        e * e / 3.0
    } else {
        ((e - HLG_C) / HLG_A).exp() / 12.0 + HLG_B / 12.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_srgb_roundtrip() {
        let tf = TransferFunction::SRGB;
        for &v in &[0.0, 0.04, 0.1, 0.5, 0.9, 1.0] {
            let linear = tf.to_linear(v);
            let back = tf.from_linear(linear);
            assert!((back - v).abs() < 0.001, "sRGB roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_rec709_roundtrip() {
        let tf = TransferFunction::Rec709;
        for &v in &[0.0, 0.04, 0.5, 1.0] {
            let linear = tf.to_linear(v);
            let back = tf.from_linear(linear);
            assert!(
                (back - v).abs() < 0.001,
                "Rec709 roundtrip failed for {}",
                v
            );
        }
    }

    #[test]
    fn test_pq_roundtrip() {
        let tf = TransferFunction::PQ;
        for &v in &[0.0, 0.1, 0.5, 0.9] {
            let linear = tf.to_linear(v);
            let back = tf.from_linear(linear);
            assert!((back - v).abs() < 0.01, "PQ roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_hlg_roundtrip() {
        let tf = TransferFunction::HLG;
        for &v in &[0.0, 0.1, 0.3, 0.5, 0.8] {
            let linear = tf.to_linear(v);
            let back = tf.from_linear(linear);
            assert!((back - v).abs() < 0.01, "HLG roundtrip failed for {}", v);
        }
    }

    #[test]
    fn test_linear_passthrough() {
        let tf = TransferFunction::Linear;
        assert_eq!(tf.to_linear(0.5), 0.5);
        assert_eq!(tf.from_linear(0.5), 0.5);
    }

    #[test]
    fn test_gamma() {
        let tf = TransferFunction::Gamma(2.2);
        let linear = tf.to_linear(0.5);
        let back = tf.from_linear(linear);
        assert!((back - 0.5).abs() < 0.001);
    }
}
