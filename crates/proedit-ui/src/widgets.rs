//! Shared UI widgets — toggle switch, themed slider.

use crate::theme::Theme;
use egui::{self, Color32, Pos2, Rounding, Stroke, Vec2};

/// Toggle switch widget. Returns `true` if clicked (toggled).
pub fn toggle_switch(ui: &mut egui::Ui, on: bool) -> bool {
    let desired_size = Vec2::new(32.0, 18.0);
    let (resp, painter) = ui.allocate_painter(desired_size, egui::Sense::click());
    let rect = resp.rect;

    // Track — pill shape uses height/2 for rounding (NOT Theme::RADIUS)
    let pill_rounding = Rounding::same(rect.height() / 2.0);
    let (track_bg, track_border) = if on {
        (
            Theme::with_alpha(Theme::accent(), 102),
            Theme::with_alpha(Theme::accent(), 153),
        )
    } else {
        (Theme::white_06(), Theme::white_08())
    };
    painter.rect_filled(rect, pill_rounding, track_bg);
    painter.rect_stroke(
        rect,
        pill_rounding,
        Stroke::new(Theme::STROKE_SUBTLE, track_border),
    );

    // Thumb
    let thumb_x = if on {
        rect.right() - 9.0
    } else {
        rect.left() + 9.0
    };
    let thumb_color = if on {
        Theme::accent()
    } else {
        Theme::white_25()
    };
    painter.circle_filled(Pos2::new(thumb_x, rect.center().y), 7.0, thumb_color);

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
        track_painter.rect_filled(bar_rect, Rounding::same(2.0), Theme::white_04());

        // Fill
        let min = *range.start();
        let max = *range.end();
        let frac = if max > min {
            (*value - min) / (max - min)
        } else {
            0.0
        };
        let fill_width = bar_rect.width() * frac.clamp(0.0, 1.0);
        let fill_rect =
            egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, track_height));
        track_painter.rect_filled(fill_rect, Rounding::same(2.0), accent);

        // Thumb
        let thumb_x = bar_rect.left() + fill_width;
        let thumb_center = Pos2::new(thumb_x, bar_rect.center().y);
        track_painter.circle_filled(thumb_center, 5.0, Color32::WHITE);
        track_painter.circle_stroke(
            thumb_center,
            5.0,
            Stroke::new(1.5, Color32::from_rgba_premultiplied(0, 0, 0, 77)),
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
