//! Effects panel with collapsible categories.

use crate::theme::Theme;
use egui::{self, Rounding, Stroke, Vec2};

// ── Data ───────────────────────────────────────────────────────

struct EffectCategory {
    name: &'static str,
    items: &'static [EffectItem],
}

struct EffectItem {
    name: &'static str,
    is_ai: bool,
}

const CATEGORIES: &[EffectCategory] = &[
    EffectCategory {
        name: "VIDEO",
        items: &[
            EffectItem {
                name: "Gaussian Blur",
                is_ai: false,
            },
            EffectItem {
                name: "Chromatic Aberration",
                is_ai: false,
            },
            EffectItem {
                name: "Film Grain",
                is_ai: false,
            },
            EffectItem {
                name: "Vignette",
                is_ai: false,
            },
            EffectItem {
                name: "Glow",
                is_ai: false,
            },
            EffectItem {
                name: "Sharpen",
                is_ai: false,
            },
            EffectItem {
                name: "Lens Distortion",
                is_ai: false,
            },
        ],
    },
    EffectCategory {
        name: "COLOR",
        items: &[
            EffectItem {
                name: "Curves",
                is_ai: false,
            },
            EffectItem {
                name: "Levels",
                is_ai: false,
            },
            EffectItem {
                name: "HSL Shift",
                is_ai: false,
            },
            EffectItem {
                name: "Color Balance",
                is_ai: false,
            },
            EffectItem {
                name: "LUT",
                is_ai: false,
            },
        ],
    },
    EffectCategory {
        name: "AI",
        items: &[
            EffectItem {
                name: "Smart Stabilize",
                is_ai: true,
            },
            EffectItem {
                name: "Auto Color",
                is_ai: true,
            },
            EffectItem {
                name: "Remove BG",
                is_ai: true,
            },
            EffectItem {
                name: "Upscale 4K",
                is_ai: true,
            },
            EffectItem {
                name: "Denoise",
                is_ai: true,
            },
            EffectItem {
                name: "Scene Detect",
                is_ai: true,
            },
        ],
    },
    EffectCategory {
        name: "TRANSITIONS",
        items: &[
            EffectItem {
                name: "Cross Dissolve",
                is_ai: false,
            },
            EffectItem {
                name: "Dip to Black",
                is_ai: false,
            },
            EffectItem {
                name: "Wipe",
                is_ai: false,
            },
            EffectItem {
                name: "Slide",
                is_ai: false,
            },
            EffectItem {
                name: "Iris",
                is_ai: false,
            },
        ],
    },
];

// ── State ──────────────────────────────────────────────────────

pub struct EffectsPanelState {
    /// Which categories are expanded (index into CATEGORIES).
    expanded: [bool; 4],
}

impl Default for EffectsPanelState {
    fn default() -> Self {
        Self {
            expanded: [true, false, true, false],
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_effects_panel(ui: &mut egui::Ui, state: &mut EffectsPanelState) {
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

    for (cat_idx, cat) in CATEGORIES.iter().enumerate() {
        // Category header
        let is_expanded = state.expanded[cat_idx];
        let chevron = if is_expanded { "\u{25BE}" } else { "\u{25B8}" };

        // Use a proper Button for reliable click detection
        let header_text = format!("{} {}  ({})", chevron, cat.name, cat.items.len());
        let header_btn = egui::Button::new(
            egui::RichText::new(header_text)
                .size(Theme::FONT_XS)
                .color(Theme::t3())
                .strong(),
        )
        .fill(egui::Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .rounding(Rounding::ZERO);

        if ui.add(header_btn).clicked() {
            state.expanded[cat_idx] = !is_expanded;
        }

        // Items
        if is_expanded {
            for item in cat.items {
                let icon = if item.is_ai { "\u{2726}" } else { "\u{25D1}" };
                let text_color = if item.is_ai {
                    Theme::with_alpha(Theme::purple(), 204)
                } else {
                    Theme::t2()
                };

                let item_text = format!("  {} {}", icon, item.name);
                let item_btn = egui::Button::new(
                    egui::RichText::new(item_text)
                        .size(Theme::FONT_XS)
                        .color(text_color),
                )
                .fill(egui::Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .rounding(Rounding::same(Theme::RADIUS));

                let _resp = ui.add(item_btn);
            }
        }

        ui.add_space(Theme::SPACE_XS);
    }
}
