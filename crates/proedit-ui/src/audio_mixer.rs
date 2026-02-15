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
                    Theme::draw_accent_top_line(ui);
                    ui.add_space(Theme::SPACE_XS);

                    // Header with icon
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                        let (icon_resp, icon_painter) =
                            ui.allocate_painter(Vec2::splat(16.0), egui::Sense::hover());
                        icon_painter.rect_filled(
                            icon_resp.rect,
                            Rounding::same(4.0),
                            Theme::with_alpha(Theme::green(), 25),
                        );
                        icon_painter.text(
                            icon_resp.rect.center(),
                            egui::Align2::CENTER_CENTER,
                            "\u{266A}",
                            egui::FontId::proportional(10.0),
                            Theme::green(),
                        );
                        ui.label(
                            egui::RichText::new("AUDIO MIXER")
                                .size(Theme::FONT_XS)
                                .color(Theme::t3())
                                .strong(),
                        );
                    });

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
        let thumb_center = Pos2::new(thumb_x, bar_rect.center().y);
        track_painter.circle_filled(thumb_center, 5.0, Color32::WHITE);
        track_painter.circle_stroke(
            thumb_center,
            5.0,
            egui::Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 60)),
        );
        // Thumb glow
        track_painter.circle_stroke(
            thumb_center,
            7.0,
            egui::Stroke::new(1.0, Theme::with_alpha(accent, 20)),
        );

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

        // Segmented LED meter — green/amber/red zones
        let bar_width = 120.0;
        let bar_height = 6.0;
        let (bar_resp, bar_painter) =
            ui.allocate_painter(Vec2::new(bar_width, bar_height), egui::Sense::hover());
        let bar_rect = bar_resp.rect;

        let segments = 24;
        let gap = 1.0;
        let seg_width = (bar_width - gap * (segments - 1) as f32) / segments as f32;

        for s in 0..segments {
            let frac = s as f32 / segments as f32;
            let x = bar_rect.left() + s as f32 * (seg_width + gap);
            let seg_rect = Rect::from_min_size(
                Pos2::new(x, bar_rect.top()),
                Vec2::new(seg_width, bar_height),
            );

            let zone_color = if frac > 0.9 {
                Theme::red()
            } else if frac > 0.75 {
                Theme::amber()
            } else {
                Theme::green()
            };

            if frac < level {
                bar_painter.rect_filled(seg_rect, Rounding::same(0.5), zone_color);
            } else {
                bar_painter.rect_filled(seg_rect, Rounding::same(0.5), Theme::white_04());
            }
        }

        // Peak indicator (bright line at peak position)
        if level > 0.01 {
            let peak_x = bar_rect.left() + level.clamp(0.0, 1.0) * bar_width;
            let peak_color = if level > 0.9 {
                Theme::red()
            } else if level > 0.75 {
                Theme::amber()
            } else {
                Theme::green()
            };
            bar_painter.rect_filled(
                Rect::from_min_size(
                    Pos2::new(peak_x - 1.0, bar_rect.top()),
                    Vec2::new(2.0, bar_height),
                ),
                Rounding::ZERO,
                peak_color,
            );
        }

        // dB value (proper logarithmic scale)
        let db = if level > 0.001 {
            20.0 * level.log10()
        } else {
            -60.0
        };
        ui.label(
            egui::RichText::new(format!("{:+.0}", db))
                .size(Theme::FONT_XS)
                .color(Theme::t4())
                .family(egui::FontFamily::Monospace),
        );
    });
}
