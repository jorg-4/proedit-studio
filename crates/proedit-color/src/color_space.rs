//! Color space definitions and RGB↔XYZ transforms.
#![allow(clippy::excessive_precision)]

use serde::{Deserialize, Serialize};

/// Supported color spaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ColorSpace {
    SRGB,
    Rec709,
    Rec2020,
    ACEScg,
    ACEScct,
    DciP3,
    LinearSRGB,
}

impl ColorSpace {
    /// RGB-to-XYZ 3x3 matrix for this color space.
    pub fn to_xyz_matrix(&self) -> [[f32; 3]; 3] {
        match self {
            Self::SRGB | Self::LinearSRGB | Self::Rec709 => [
                [0.4124564, 0.3575761, 0.1804375],
                [0.2126729, 0.7151522, 0.0721750],
                [0.0193339, 0.1191920, 0.9503041],
            ],
            Self::Rec2020 => [
                [0.6369580, 0.1446169, 0.1688810],
                [0.2627002, 0.6779981, 0.0593017],
                [0.0000000, 0.0280727, 1.0609851],
            ],
            Self::ACEScg => [
                [0.6624542, 0.1340042, 0.1561877],
                [0.2722287, 0.6740818, 0.0536895],
                [-0.0055746, 0.0040607, 1.0103391],
            ],
            Self::ACEScct => Self::ACEScg.to_xyz_matrix(),
            Self::DciP3 => [
                [0.4865709, 0.2656677, 0.1982173],
                [0.2289746, 0.6917385, 0.0792869],
                [0.0000000, 0.0451134, 1.0439444],
            ],
        }
    }

    /// XYZ-to-RGB 3x3 matrix for this color space (inverse of to_xyz).
    pub fn from_xyz_matrix(&self) -> [[f32; 3]; 3] {
        match self {
            Self::SRGB | Self::LinearSRGB | Self::Rec709 => [
                [3.2404542, -1.5371385, -0.4985314],
                [-0.9692660, 1.8760108, 0.0415560],
                [0.0556434, -0.2040259, 1.0572252],
            ],
            Self::Rec2020 => [
                [1.7166512, -0.3556708, -0.2533663],
                [-0.6666844, 1.6164812, 0.0157685],
                [0.0176399, -0.0427706, 0.9421031],
            ],
            Self::ACEScg => [
                [1.6410234, -0.3248033, -0.2364247],
                [-0.6636629, 1.6153316, 0.0167563],
                [0.0117219, -0.0082844, 0.9883949],
            ],
            Self::ACEScct => Self::ACEScg.from_xyz_matrix(),
            Self::DciP3 => [
                [2.4934969, -0.9313836, -0.4027108],
                [-0.8294890, 1.7626641, 0.0236247],
                [0.0358458, -0.0761724, 0.9568845],
            ],
        }
    }

    /// Whether this space uses linear light.
    pub fn is_linear(&self) -> bool {
        matches!(self, Self::LinearSRGB | Self::ACEScg)
    }

    /// Display name.
    pub fn name(&self) -> &str {
        match self {
            Self::SRGB => "sRGB",
            Self::Rec709 => "Rec. 709",
            Self::Rec2020 => "Rec. 2020",
            Self::ACEScg => "ACEScg",
            Self::ACEScct => "ACEScct",
            Self::DciP3 => "DCI-P3",
            Self::LinearSRGB => "Linear sRGB",
        }
    }

    /// CIE xy white point.
    pub fn white_point(&self) -> [f32; 2] {
        match self {
            Self::SRGB | Self::LinearSRGB | Self::Rec709 | Self::Rec2020 => [0.3127, 0.3290],
            Self::ACEScg | Self::ACEScct => [0.32168, 0.33767],
            Self::DciP3 => [0.3140, 0.3510],
        }
    }
}

/// Apply a 3x3 matrix to an RGB triplet.
fn mat3_mul(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Convert an RGB pixel from one color space to another via XYZ.
pub fn convert_3x3(pixel: [f32; 3], from: &ColorSpace, to: &ColorSpace) -> [f32; 3] {
    if from == to {
        return pixel;
    }
    let xyz = mat3_mul(&from.to_xyz_matrix(), pixel);
    mat3_mul(&to.from_xyz_matrix(), xyz)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_conversion() {
        let pixel = [0.5, 0.3, 0.8];
        let result = convert_3x3(pixel, &ColorSpace::SRGB, &ColorSpace::SRGB);
        assert!((result[0] - pixel[0]).abs() < 0.001);
        assert!((result[1] - pixel[1]).abs() < 0.001);
        assert!((result[2] - pixel[2]).abs() < 0.001);
    }

    #[test]
    fn test_srgb_to_rec2020_roundtrip() {
        let pixel = [0.5, 0.3, 0.8];
        let rec2020 = convert_3x3(pixel, &ColorSpace::SRGB, &ColorSpace::Rec2020);
        let back = convert_3x3(rec2020, &ColorSpace::Rec2020, &ColorSpace::SRGB);
        assert!((back[0] - pixel[0]).abs() < 0.02);
        assert!((back[1] - pixel[1]).abs() < 0.02);
        assert!((back[2] - pixel[2]).abs() < 0.02);
    }

    #[test]
    fn test_white_stays_white() {
        let white = [1.0, 1.0, 1.0];
        let xyz = mat3_mul(&ColorSpace::SRGB.to_xyz_matrix(), white);
        // XYZ of D65 white should be approximately [0.95, 1.0, 1.09]
        assert!((xyz[1] - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_space_names() {
        assert_eq!(ColorSpace::SRGB.name(), "sRGB");
        assert_eq!(ColorSpace::ACEScg.name(), "ACEScg");
    }

    #[test]
    fn test_is_linear() {
        assert!(ColorSpace::LinearSRGB.is_linear());
        assert!(ColorSpace::ACEScg.is_linear());
        assert!(!ColorSpace::SRGB.is_linear());
    }
}
