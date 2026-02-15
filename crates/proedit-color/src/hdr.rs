//! HDR metadata and encoding utilities.
#![allow(clippy::excessive_precision)]

use serde::{Deserialize, Serialize};

/// HDR metadata for a video stream.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HdrMetadata {
    /// Maximum Content Light Level in nits.
    pub max_content_light_level: u32,
    /// Maximum Frame Average Light Level in nits.
    pub max_frame_avg_light_level: u32,
    /// Mastering display luminance range (min, max) in nits.
    pub mastering_display_luminance: (f32, f32),
    /// Mastering display color primaries (RGB) as CIE xy coordinates.
    pub mastering_display_primaries: [[f32; 2]; 3],
    /// White point as CIE xy coordinates.
    pub white_point: [f32; 2],
}

impl Default for HdrMetadata {
    fn default() -> Self {
        Self {
            max_content_light_level: 1000,
            max_frame_avg_light_level: 400,
            mastering_display_luminance: (0.005, 1000.0),
            mastering_display_primaries: [
                [0.708, 0.292], // R (Rec.2020)
                [0.170, 0.797], // G
                [0.131, 0.046], // B
            ],
            white_point: [0.3127, 0.3290], // D65
        }
    }
}

// PQ constants (duplicated here for standalone encode/decode)
const PQ_M1: f32 = 0.1593017578125;
const PQ_M2: f32 = 78.84375;
const PQ_C1: f32 = 0.8359375;
const PQ_C2: f32 = 18.8515625;
const PQ_C3: f32 = 18.6875;

/// Encode linear luminance (0-10000 nits) to PQ [0, 1].
pub fn encode_pq(linear_nits: f32) -> f32 {
    let y = (linear_nits / 10000.0).max(0.0);
    let ym1 = y.powf(PQ_M1);
    let num = PQ_C1 + PQ_C2 * ym1;
    let den = 1.0 + PQ_C3 * ym1;
    (num / den).powf(PQ_M2)
}

/// Decode PQ [0, 1] to linear luminance (0-10000 nits).
pub fn decode_pq(pq: f32) -> f32 {
    let pq = pq.max(0.0);
    let p = pq.powf(1.0 / PQ_M2);
    let num = (p - PQ_C1).max(0.0);
    let den = PQ_C2 - PQ_C3 * p;
    if den.abs() < 1e-10 {
        0.0
    } else {
        (num / den).powf(1.0 / PQ_M1) * 10000.0
    }
}

// HLG constants
const HLG_A: f32 = 0.17883277;
const HLG_B: f32 = 0.28466892;
const HLG_C: f32 = 0.55991073;

/// Encode scene-referred linear to HLG.
pub fn encode_hlg(linear: f32) -> f32 {
    let e = linear.max(0.0);
    if e <= 1.0 / 12.0 {
        (3.0 * e).sqrt()
    } else {
        HLG_A * (12.0 * e - HLG_B).ln() + HLG_C
    }
}

/// Decode HLG to scene-referred linear.
pub fn decode_hlg(hlg: f32) -> f32 {
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
    fn test_pq_roundtrip() {
        for &nits in &[0.0, 100.0, 1000.0, 4000.0, 10000.0] {
            let encoded = encode_pq(nits);
            let decoded = decode_pq(encoded);
            assert!(
                (decoded - nits).abs() < 1.0,
                "PQ roundtrip failed for {} nits: got {}",
                nits,
                decoded
            );
        }
    }

    #[test]
    fn test_pq_range() {
        assert!(encode_pq(0.0) >= 0.0);
        assert!(encode_pq(10000.0) <= 1.01);
    }

    #[test]
    fn test_hlg_roundtrip() {
        for &v in &[0.0, 0.05, 0.1, 0.5, 1.0] {
            let encoded = encode_hlg(v);
            let decoded = decode_hlg(encoded);
            assert!(
                (decoded - v).abs() < 0.01,
                "HLG roundtrip failed for {}: got {}",
                v,
                decoded
            );
        }
    }

    #[test]
    fn test_hdr_metadata_default() {
        let meta = HdrMetadata::default();
        assert_eq!(meta.max_content_light_level, 1000);
        assert!((meta.white_point[0] - 0.3127).abs() < 0.001);
    }
}
