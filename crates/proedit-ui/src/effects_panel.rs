//! Effects panel with collapsible categories.

use egui::{self, Color32, Rounding, Vec2};
use crate::theme::Theme;

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
            EffectItem { name: "Gaussian Blur",        is_ai: false },
            EffectItem { name: "Chromatic Aberration",  is_ai: false },
            EffectItem { name: "Film Grain",            is_ai: false },
            EffectItem { name: "Vignette",              is_ai: false },
            EffectItem { name: "Glow",                  is_ai: false },
            EffectItem { name: "Sharpen",               is_ai: false },
            EffectItem { name: "Lens Distortion",       is_ai: false },
        ],
    },
    EffectCategory {
        name: "COLOR",
        items: &[
            EffectItem { name: "Curves",         is_ai: false },
            EffectItem { name: "Levels",         is_ai: false },
            EffectItem { name: "HSL Shift",      is_ai: false },
            EffectItem { name: "Color Balance",  is_ai: false },
            EffectItem { name: "LUT",            is_ai: false },
        ],
    },
    EffectCategory {
        name: "AI",
        items: &[
            EffectItem { name: "Smart Stabilize", is_ai: true },
            EffectItem { name: "Auto Color",      is_ai: true },
            EffectItem { name: "Remove BG",       is_ai: true },
            EffectItem { name: "Upscale 4K",      is_ai: true },
            EffectItem { name: "Denoise",         is_ai: true },
            EffectItem { name: "Scene Detect",    is_ai: true },
        ],
    },
    EffectCategory {
        name: "TRANSITIONS",
        items: &[
            EffectItem { name: "Cross Dissolve", is_ai: false },
            EffectItem { name: "Dip to Black",   is_ai: false },
            EffectItem { name: "Wipe",           is_ai: false },
            EffectItem { name: "Slide",          is_ai: false },
            EffectItem { name: "Iris",           is_ai: false },
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

        let header_resp = ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            ui.label(
                egui::RichText::new(chevron)
                    .size(8.0)
                    .color(Theme::t4()),
            );
            ui.label(
                egui::RichText::new(cat.name)
                    .size(9.5)
                    .color(Theme::t3())
                    .strong(),
            );

            // Count badge
            let badge_frame = egui::Frame::none()
                .fill(Color32::from_rgba_premultiplied(2, 2, 2, 8))
                .rounding(Rounding::same(6.0))
                .inner_margin(egui::Margin::symmetric(5.0, 1.0));
            badge_frame.show(ui, |ui| {
                ui.label(
                    egui::RichText::new(format!("{}", cat.items.len()))
                        .size(8.0)
                        .color(Theme::t4()),
                );
            });
        }).response;

        if header_resp.clicked() {
            state.expanded[cat_idx] = !is_expanded;
        }

        // Items
        if is_expanded {
            for item in cat.items {
                let icon = if item.is_ai { "\u{2726}" } else { "\u{25D1}" };
                let text_color = if item.is_ai {
                    Theme::with_alpha(Theme::purple(), 204) // purple+CC
                } else {
                    Theme::t2()
                };

                let item_frame = egui::Frame::none()
                    .rounding(Rounding::same(7.0))
                    .inner_margin(egui::Margin {
                        left: 16.0,
                        right: 8.0,
                        top: 4.0,
                        bottom: 4.0,
                    });

                let resp = item_frame.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                        ui.label(egui::RichText::new(icon).size(10.0).color(text_color));
                        ui.label(egui::RichText::new(item.name).size(10.5).color(text_color));
                    });
                }).response;

                if resp.hovered() {
                    let hover_bg = if item.is_ai {
                        Theme::with_alpha(Theme::purple(), 10)
                    } else {
                        Color32::from_rgba_premultiplied(2, 2, 2, 10)
                    };
                    ui.painter().rect_filled(resp.rect, Rounding::same(7.0), hover_bg);
                }
            }
        }

        ui.add_space(4.0);
    }
}
