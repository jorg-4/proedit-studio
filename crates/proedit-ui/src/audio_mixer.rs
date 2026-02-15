//! Floating audio mixer panel.

use crate::theme::Theme;
use crate::widgets;
use egui::{self, Color32, Pos2, Rect, Rounding, Vec2};

// ── State ──────────────────────────────────────────────────────

pub struct AudioMixerState {
    pub master_volume: f32,
    pub levels: [f32; 3], // A1, A2, A3
    pub loudness_metering: bool,
    pub limiter: bool,
}

impl Default for AudioMixerState {
    fn default() -> Self {
        Self {
            master_volume: 80.0,
            levels: [0.0, 0.0, 0.0],
            loudness_metering: true,
            limiter: false,
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_audio_mixer(ctx: &egui::Context, state: &mut AudioMixerState) {
    egui::Area::new(egui::Id::new("audio_mixer_panel"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::RIGHT_BOTTOM, Vec2::new(-20.0, -20.0))
        .show(ctx, |ui| {
            Theme::glass_frame()
                .inner_margin(egui::Margin::symmetric(Theme::SPACE_MD, 14.0))
                .show(ui, |ui| {
                    ui.set_width(200.0);
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, Theme::SPACE_SM);

                    // Header
                    ui.label(
                        egui::RichText::new("AUDIO MIXER")
                            .size(Theme::FONT_XS)
                            .color(Theme::t3())
                            .strong(),
                    );

                    // Master slider
                    mixer_slider(ui, "Master", &mut state.master_volume, Theme::green());

                    // Level meters
                    let labels = ["A1", "A2", "A3"];
                    for (i, label) in labels.iter().enumerate() {
                        level_meter(ui, label, state.levels[i]);
                    }

                    ui.add_space(Theme::SPACE_XS);

                    // Toggles
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);
                        if widgets::toggle_switch(ui, state.loudness_metering) {
                            state.loudness_metering = !state.loudness_metering;
                        }
                        let text_color = if state.loudness_metering {
                            Theme::t1()
                        } else {
                            Theme::t3()
                        };
                        ui.label(
                            egui::RichText::new("Loudness Metering")
                                .size(Theme::FONT_XS)
                                .color(text_color),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);
                        if widgets::toggle_switch(ui, state.limiter) {
                            state.limiter = !state.limiter;
                        }
                        let text_color = if state.limiter {
                            Theme::t1()
                        } else {
                            Theme::t3()
                        };
                        ui.label(
                            egui::RichText::new("Limiter")
                                .size(Theme::FONT_XS)
                                .color(text_color),
                        );
                    });
                });
        });
}

fn mixer_slider(ui: &mut egui::Ui, label: &str, value: &mut f32, accent: Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

        // Label
        ui.allocate_ui(Vec2::new(42.0, 20.0), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .size(Theme::FONT_XS)
                        .color(Theme::t3()),
                );
            });
        });

        // Track
        let track_width = 110.0;
        let (track_resp, track_painter) =
            ui.allocate_painter(Vec2::new(track_width, 20.0), egui::Sense::click_and_drag());
        let track_rect = track_resp.rect;
        let bar_rect = Rect::from_center_size(track_rect.center(), Vec2::new(track_width, 4.0));

        track_painter.rect_filled(bar_rect, Rounding::same(2.0), Theme::white_10());

        let frac = (*value / 100.0).clamp(0.0, 1.0);
        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(bar_rect.width() * frac, 4.0));
        track_painter.rect_filled(fill_rect, Rounding::same(2.0), accent);

        let thumb_x = bar_rect.left() + frac * bar_rect.width();
        track_painter.circle_filled(Pos2::new(thumb_x, bar_rect.center().y), 5.0, Color32::WHITE);

        if track_resp.dragged() || track_resp.clicked() {
            if let Some(pos) = track_resp.interact_pointer_pos() {
                let rel = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                *value = rel * 100.0;
            }
        }

        // Value
        ui.label(
            egui::RichText::new(format!("{:.0}%", *value))
                .size(Theme::FONT_XS)
                .color(Theme::t2())
                .family(egui::FontFamily::Monospace),
        );
    });
}

fn level_meter(ui: &mut egui::Ui, label: &str, level: f32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

        // Label
        ui.allocate_ui(Vec2::new(20.0, 16.0), |ui| {
            ui.label(
                egui::RichText::new(label)
                    .size(Theme::FONT_XS)
                    .color(Theme::t4()),
            );
        });

        // Meter bar
        let bar_width = 120.0;
        let (bar_resp, bar_painter) =
            ui.allocate_painter(Vec2::new(bar_width, 4.0), egui::Sense::hover());
        let bar_rect = bar_resp.rect;

        bar_painter.rect_filled(bar_rect, Rounding::same(2.0), Theme::white_10());

        let fill_width = bar_rect.width() * level;
        let fill_color = if level > 0.9 {
            Theme::red()
        } else if level > 0.8 {
            Theme::amber()
        } else {
            Theme::green()
        };
        let fill_rect = Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, 4.0));
        bar_painter.rect_filled(fill_rect, Rounding::same(2.0), fill_color);

        // dB value
        let db = level * 100.0 - 100.0;
        ui.label(
            egui::RichText::new(format!("{:.0}dB", db))
                .size(Theme::FONT_XS)
                .color(Theme::t4())
                .family(egui::FontFamily::Monospace),
        );
    });
}
