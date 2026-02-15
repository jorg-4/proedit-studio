//! Shared UI widgets — toggle switch, themed slider.

use crate::theme::Theme;
use egui::{self, Color32, Pos2, Rounding, Stroke, Vec2};

/// Toggle switch widget. Returns `true` if clicked (toggled).
pub fn toggle_switch(ui: &mut egui::Ui, on: bool) -> bool {
    let desired_size = Vec2::new(30.0, 16.0);
    let (resp, painter) = ui.allocate_painter(desired_size, egui::Sense::click());
    let rect = resp.rect;

    // Track — pill shape
    let pill_rounding = Rounding::same(rect.height() / 2.0);
    let (track_bg, track_border) = if on {
        (
            Theme::with_alpha(Theme::accent(), 90),
            Theme::with_alpha(Theme::accent(), 130),
        )
    } else {
        (Theme::white_04(), Theme::white_08())
    };
    painter.rect_filled(rect, pill_rounding, track_bg);
    painter.rect_stroke(rect, pill_rounding, Stroke::new(0.5, track_border));

    // Thumb — smooth animated position
    let thumb_radius = 6.0;
    let anim_t = ui
        .ctx()
        .animate_bool_with_time(resp.id.with("toggle_anim"), on, 0.15);
    let thumb_x = egui::lerp(
        rect.left() + thumb_radius + 2.0..=rect.right() - thumb_radius - 2.0,
        anim_t,
    );
    let thumb_color = if on {
        Theme::accent()
    } else {
        Theme::white_25()
    };
    painter.circle_filled(
        Pos2::new(thumb_x, rect.center().y),
        thumb_radius,
        thumb_color,
    );
    // Thumb shadow
    if on {
        painter.circle_stroke(
            Pos2::new(thumb_x, rect.center().y),
            thumb_radius + 1.0,
            Stroke::new(1.0, Theme::with_alpha(Theme::accent(), 30)),
        );
    }

    resp.clicked()
}

/// Custom themed slider matching the design reference.
pub fn themed_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    accent: Color32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);

        // Label
        let label_width = 54.0;
        ui.allocate_ui(Vec2::new(label_width, 26.0), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(label)
                        .size(Theme::FONT_XS)
                        .color(Theme::t3()),
                );
            });
        });

        // Slider track
        let track_width = ui.available_width() - 44.0;
        let track_height = 4.0;
        let (track_resp, track_painter) =
            ui.allocate_painter(Vec2::new(track_width, 26.0), egui::Sense::click_and_drag());
        let track_rect = track_resp.rect;

        let bar_rect =
            egui::Rect::from_center_size(track_rect.center(), Vec2::new(track_width, track_height));

        // Background track
        track_painter.rect_filled(bar_rect, Rounding::same(2.0), Theme::white_06());

        // Fill — gradient from dark to bright
        let min = *range.start();
        let max = *range.end();
        let frac = if max > min {
            (*value - min) / (max - min)
        } else {
            0.0
        };
        let fill_width = bar_rect.width() * frac.clamp(0.0, 1.0);
        if fill_width > 2.0 {
            // Multi-segment gradient fill
            let segments = 3;
            let seg_w = fill_width / segments as f32;
            for s in 0..segments {
                let seg_frac = s as f32 / segments as f32;
                let alpha = (100.0 + seg_frac * 155.0) as u8; // 100→255 gradient
                let seg_rect = egui::Rect::from_min_size(
                    egui::Pos2::new(bar_rect.left() + s as f32 * seg_w, bar_rect.top()),
                    Vec2::new(seg_w + 0.5, track_height), // +0.5 overlap to avoid gaps
                );
                let seg_rounding = if s == 0 {
                    Rounding {
                        nw: 2.0,
                        sw: 2.0,
                        ne: 0.0,
                        se: 0.0,
                    }
                } else if s == segments - 1 {
                    Rounding {
                        nw: 0.0,
                        sw: 0.0,
                        ne: 2.0,
                        se: 2.0,
                    }
                } else {
                    Rounding::ZERO
                };
                track_painter.rect_filled(seg_rect, seg_rounding, Theme::with_alpha(accent, alpha));
            }
        }

        // Thumb — glass effect
        let thumb_x = bar_rect.left() + fill_width;
        let thumb_center = Pos2::new(thumb_x, bar_rect.center().y);
        track_painter.circle_filled(thumb_center, 5.0, Color32::WHITE);
        track_painter.circle_stroke(
            thumb_center,
            5.0,
            Stroke::new(1.0, Color32::from_rgba_premultiplied(0, 0, 0, 60)),
        );
        // Glow around thumb
        track_painter.circle_stroke(
            thumb_center,
            7.0,
            Stroke::new(1.0, Theme::with_alpha(accent, 25)),
        );

        // Interaction
        if track_resp.dragged() || track_resp.clicked() {
            if let Some(pos) = track_resp.interact_pointer_pos() {
                let rel = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                *value = min + rel * (max - min);
            }
        }

        // Value display
        ui.allocate_ui(Vec2::new(38.0, 26.0), |ui| {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!("{:.0}", *value))
                        .size(Theme::FONT_XS)
                        .color(Theme::t2())
                        .family(egui::FontFamily::Monospace),
                );
            });
        });
    });
}
