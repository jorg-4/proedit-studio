//! Blend mode definitions for compositing.
//!
//! Lists all supported blend modes. The actual GPU implementation
//! uses these enums to select the correct blend operation in the
//! compositing shader.

use serde::{Deserialize, Serialize};

/// Blend mode for compositing layers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[repr(u32)]
pub enum BlendMode {
    // ── Normal modes ────────────────────────────
    #[default]
    Normal = 0,
    Dissolve = 1,

    // ── Darken group ────────────────────────────
    Darken = 2,
    Multiply = 3,
    ColorBurn = 4,
    LinearBurn = 5,

    // ── Lighten group ───────────────────────────
    Lighten = 6,
    Screen = 7,
    ColorDodge = 8,
    LinearDodge = 9,

    // ── Contrast group ──────────────────────────
    Overlay = 10,
    SoftLight = 11,
    HardLight = 12,
    VividLight = 13,
    LinearLight = 14,
    PinLight = 15,
    HardMix = 16,

    // ── Inversion group ─────────────────────────
    Difference = 17,
    Exclusion = 18,
    Subtract = 19,
    Divide = 20,

    // ── Component group ─────────────────────────
    Hue = 21,
    Saturation = 22,
    Color = 23,
    Luminosity = 24,

    // ── Video-specific ──────────────────────────
    Add = 25,
    Stencil = 26,
    Silhouette = 27,
}

impl BlendMode {
    /// All blend modes in display order.
    pub const ALL: [BlendMode; 28] = [
        Self::Normal,
        Self::Dissolve,
        Self::Darken,
        Self::Multiply,
        Self::ColorBurn,
        Self::LinearBurn,
        Self::Lighten,
        Self::Screen,
        Self::ColorDodge,
        Self::LinearDodge,
        Self::Overlay,
        Self::SoftLight,
        Self::HardLight,
        Self::VividLight,
        Self::LinearLight,
        Self::PinLight,
        Self::HardMix,
        Self::Difference,
        Self::Exclusion,
        Self::Subtract,
        Self::Divide,
        Self::Hue,
        Self::Saturation,
        Self::Color,
        Self::Luminosity,
        Self::Add,
        Self::Stencil,
        Self::Silhouette,
    ];

    /// Human-readable name.
    pub fn name(self) -> &'static str {
        match self {
            Self::Normal => "Normal",
            Self::Dissolve => "Dissolve",
            Self::Darken => "Darken",
            Self::Multiply => "Multiply",
            Self::ColorBurn => "Color Burn",
            Self::LinearBurn => "Linear Burn",
            Self::Lighten => "Lighten",
            Self::Screen => "Screen",
            Self::ColorDodge => "Color Dodge",
            Self::LinearDodge => "Linear Dodge (Add)",
            Self::Overlay => "Overlay",
            Self::SoftLight => "Soft Light",
            Self::HardLight => "Hard Light",
            Self::VividLight => "Vivid Light",
            Self::LinearLight => "Linear Light",
            Self::PinLight => "Pin Light",
            Self::HardMix => "Hard Mix",
            Self::Difference => "Difference",
            Self::Exclusion => "Exclusion",
            Self::Subtract => "Subtract",
            Self::Divide => "Divide",
            Self::Hue => "Hue",
            Self::Saturation => "Saturation",
            Self::Color => "Color",
            Self::Luminosity => "Luminosity",
            Self::Add => "Add",
            Self::Stencil => "Stencil Alpha",
            Self::Silhouette => "Silhouette Alpha",
        }
    }

    /// Category for UI grouping.
    pub fn category(self) -> &'static str {
        match self {
            Self::Normal | Self::Dissolve => "Normal",
            Self::Darken | Self::Multiply | Self::ColorBurn | Self::LinearBurn => "Darken",
            Self::Lighten | Self::Screen | Self::ColorDodge | Self::LinearDodge => "Lighten",
            Self::Overlay
            | Self::SoftLight
            | Self::HardLight
            | Self::VividLight
            | Self::LinearLight
            | Self::PinLight
            | Self::HardMix => "Contrast",
            Self::Difference | Self::Exclusion | Self::Subtract | Self::Divide => "Inversion",
            Self::Hue | Self::Saturation | Self::Color | Self::Luminosity => "Component",
            Self::Add | Self::Stencil | Self::Silhouette => "Video",
        }
    }
}
