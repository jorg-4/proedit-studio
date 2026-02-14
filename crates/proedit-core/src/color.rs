//! Color types and color space management.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// RGBA color with 32-bit float components.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize, Pod, Zeroable)]
#[repr(C)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Color {
    /// Create a new color from RGBA components.
    #[inline]
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Create a color from RGB with alpha = 1.0.
    #[inline]
    pub const fn rgb(r: f32, g: f32, b: f32) -> Self {
        Self { r, g, b, a: 1.0 }
    }

    /// Create a color from 8-bit RGBA values.
    #[inline]
    pub fn from_rgba8(r: u8, g: u8, b: u8, a: u8) -> Self {
        Self {
            r: r as f32 / 255.0,
            g: g as f32 / 255.0,
            b: b as f32 / 255.0,
            a: a as f32 / 255.0,
        }
    }

    /// Convert to 8-bit RGBA.
    #[inline]
    pub fn to_rgba8(self) -> [u8; 4] {
        [
            (self.r.clamp(0.0, 1.0) * 255.0) as u8,
            (self.g.clamp(0.0, 1.0) * 255.0) as u8,
            (self.b.clamp(0.0, 1.0) * 255.0) as u8,
            (self.a.clamp(0.0, 1.0) * 255.0) as u8,
        ]
    }

    /// Premultiply alpha.
    #[inline]
    pub fn premultiply(self) -> Self {
        Self {
            r: self.r * self.a,
            g: self.g * self.a,
            b: self.b * self.a,
            a: self.a,
        }
    }

    /// Luminance (perceived brightness).
    #[inline]
    pub fn luminance(self) -> f32 {
        0.2126 * self.r + 0.7152 * self.g + 0.0722 * self.b
    }

    /// Linear interpolation between two colors.
    #[inline]
    pub fn lerp(self, other: Self, t: f32) -> Self {
        Self {
            r: self.r + (other.r - self.r) * t,
            g: self.g + (other.g - self.g) * t,
            b: self.b + (other.b - self.b) * t,
            a: self.a + (other.a - self.a) * t,
        }
    }

    // Common colors
    pub const BLACK: Self = Self::new(0.0, 0.0, 0.0, 1.0);
    pub const WHITE: Self = Self::new(1.0, 1.0, 1.0, 1.0);
    pub const TRANSPARENT: Self = Self::new(0.0, 0.0, 0.0, 0.0);
    pub const RED: Self = Self::new(1.0, 0.0, 0.0, 1.0);
    pub const GREEN: Self = Self::new(0.0, 1.0, 0.0, 1.0);
    pub const BLUE: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const YELLOW: Self = Self::new(1.0, 1.0, 0.0, 1.0);
    pub const CYAN: Self = Self::new(0.0, 1.0, 1.0, 1.0);
    pub const MAGENTA: Self = Self::new(1.0, 0.0, 1.0, 1.0);
}

/// Color space enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ColorSpace {
    /// sRGB (standard display)
    #[default]
    Srgb,
    /// Linear sRGB (for compositing)
    LinearSrgb,
    /// Rec. 709 (HD video)
    Rec709,
    /// Rec. 2020 (HDR video)
    Rec2020,
    /// DCI-P3 (cinema)
    DciP3,
    /// Display P3 (Apple displays)
    DisplayP3,
    /// ACEScg (VFX working space)
    AcesCg,
    /// ACES 2065-1 (interchange)
    Aces2065,
}

/// Transfer function (gamma/OETF/EOTF).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum TransferFunction {
    /// Linear (gamma 1.0)
    Linear,
    /// sRGB transfer function
    #[default]
    Srgb,
    /// Rec. 709 transfer function
    Rec709,
    /// PQ (Perceptual Quantizer) for HDR
    Pq,
    /// HLG (Hybrid Log-Gamma) for HDR
    Hlg,
}

impl TransferFunction {
    /// Apply the transfer function (linear to display).
    pub fn apply(self, linear: f32) -> f32 {
        match self {
            Self::Linear => linear,
            Self::Srgb => {
                if linear <= 0.0031308 {
                    linear * 12.92
                } else {
                    1.055 * linear.powf(1.0 / 2.4) - 0.055
                }
            }
            Self::Rec709 => {
                if linear < 0.018 {
                    linear * 4.5
                } else {
                    1.099 * linear.powf(0.45) - 0.099
                }
            }
            Self::Pq => {
                // Simplified PQ EOTF
                let m1 = 0.159_301_76_f32;
                let m2 = 78.84375;
                let c1 = 0.8359375;
                let c2 = 18.851_563_f32;
                let c3 = 18.6875;
                let y = linear.max(0.0);
                let num = c1 + c2 * y.powf(m1);
                let den = 1.0 + c3 * y.powf(m1);
                (num / den).powf(m2)
            }
            Self::Hlg => {
                // Simplified HLG OETF
                let a = 0.17883277;
                let b = 0.28466892;
                let c = 0.559_910_7_f32;
                if linear <= 1.0 / 12.0 {
                    (3.0 * linear).sqrt()
                } else {
                    a * (12.0 * linear - b).ln() + c
                }
            }
        }
    }

    /// Invert the transfer function (display to linear).
    pub fn invert(self, display: f32) -> f32 {
        match self {
            Self::Linear => display,
            Self::Srgb => {
                if display <= 0.04045 {
                    display / 12.92
                } else {
                    ((display + 0.055) / 1.055).powf(2.4)
                }
            }
            Self::Rec709 => {
                if display < 0.081 {
                    display / 4.5
                } else {
                    ((display + 0.099) / 1.099).powf(1.0 / 0.45)
                }
            }
            Self::Pq | Self::Hlg => {
                // Inverse operations are complex, approximate with sRGB for now
                Self::Srgb.invert(display)
            }
        }
    }
}

/// Combined color space and transfer function.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct ColorConfig {
    pub space: ColorSpace,
    pub transfer: TransferFunction,
}

impl ColorConfig {
    /// Create a new color configuration.
    pub const fn new(space: ColorSpace, transfer: TransferFunction) -> Self {
        Self { space, transfer }
    }

    /// Standard sRGB configuration.
    pub const SRGB: Self = Self::new(ColorSpace::Srgb, TransferFunction::Srgb);

    /// Linear sRGB (for compositing).
    pub const LINEAR_SRGB: Self = Self::new(ColorSpace::LinearSrgb, TransferFunction::Linear);

    /// Rec. 709 HD video.
    pub const REC709: Self = Self::new(ColorSpace::Rec709, TransferFunction::Rec709);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_color_rgba8_conversion() {
        let color = Color::from_rgba8(255, 128, 0, 255);
        assert!((color.r - 1.0).abs() < 0.01);
        assert!((color.g - 0.5).abs() < 0.01);
        assert_eq!(color.b, 0.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_color_luminance() {
        assert!((Color::WHITE.luminance() - 1.0).abs() < 0.001);
        assert!(Color::BLACK.luminance().abs() < 0.001);
    }

    #[test]
    fn test_srgb_transfer() {
        let tf = TransferFunction::Srgb;
        // Round trip test
        let linear = 0.5;
        let display = tf.apply(linear);
        let back = tf.invert(display);
        assert!((linear - back).abs() < 0.001);
    }
}
