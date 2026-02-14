//! Floating color wheels panel with 4 wheels and mini scopes.

use egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::theme::Theme;

const WHEEL_SIZE: f32 = 70.0;
const WHEEL_RADIUS: f32 = WHEEL_SIZE / 2.0;

const WHEEL_LABELS: &[&str] = &["Lift", "Gamma", "Gain", "Offset"];

// ── State ──────────────────────────────────────────────────────

pub struct ColorWheelsState {
    /// (x, y) offset of each wheel's indicator from center, normalized to [-1, 1].
    pub positions: [[f32; 2]; 4],
    pub dragging: Option<usize>,
}

impl Default for ColorWheelsState {
    fn default() -> Self {
        Self {
            positions: [[0.0, 0.0]; 4],
            dragging: None,
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_color_wheels(ctx: &egui::Context, state: &mut ColorWheelsState, time: f64) {
    egui::Area::new(egui::Id::new("color_wheels_panel"))
        .order(egui::Order::Foreground)
        .anchor(egui::Align2::CENTER_BOTTOM, Vec2::new(0.0, -20.0))
        .show(ctx, |ui| {
            Theme::glass_frame()
                .inner_margin(egui::Margin::symmetric(18.0, 14.0))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(14.0, 0.0);

                        // 4 color wheels
                        for (i, label) in WHEEL_LABELS.iter().enumerate() {
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);

                                // Label
                                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                                    ui.label(
                                        egui::RichText::new(label.to_uppercase())
                                            .size(8.0)
                                            .color(Theme::t3())
                                            .strong(),
                                    );
                                });

                                draw_wheel(ui, i, state, time);
                            });
                        }

                        // Separator
                        let (sep_resp, sep_painter) =
                            ui.allocate_painter(Vec2::new(0.5, 70.0), egui::Sense::hover());
                        // Gradient: transparent → white .05 → transparent
                        let sep_rect = sep_resp.rect;
                        sep_painter.rect_filled(
                            Rect::from_center_size(sep_rect.center(), Vec2::new(0.5, 50.0)),
                            0.0,
                            Color32::from_rgba_premultiplied(255, 255, 255, 13),
                        );

                        // Scopes section
                        ui.vertical(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);
                            ui.label(
                                egui::RichText::new("SCOPES")
                                    .size(8.0)
                                    .color(Theme::t3())
                                    .strong(),
                            );

                            draw_scope(ui, 120.0, 55.0, Theme::green(), time, 0);
                            draw_scope(ui, 120.0, 40.0, Theme::accent(), time, 1);
                        });
                    });
                });
        });
}

fn draw_wheel(ui: &mut egui::Ui, index: usize, state: &mut ColorWheelsState, _time: f64) {
    let (response, painter) =
        ui.allocate_painter(Vec2::splat(WHEEL_SIZE), egui::Sense::click_and_drag());
    let center = response.rect.center();
    let radius = WHEEL_RADIUS - 2.0;

    // Outer ring — approximate conic gradient with segments
    let segments = 36;
    for seg in 0..segments {
        let angle0 = (seg as f32 / segments as f32) * std::f32::consts::TAU;
        let angle1 = ((seg + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let hue_shift = (index as f32 * 90.0).to_radians();
        let hue = angle0 + hue_shift;
        let r = ((hue.cos() * 0.5 + 0.5) * 80.0 + 40.0) as u8;
        let g = (((hue + 2.094).cos() * 0.5 + 0.5) * 80.0 + 40.0) as u8;
        let b = (((hue + 4.189).cos() * 0.5 + 0.5) * 80.0 + 40.0) as u8;
        let color = Color32::from_rgba_premultiplied(r, g, b, 46);

        let p0 = Pos2::new(
            center.x + angle0.cos() * radius,
            center.y + angle0.sin() * radius,
        );
        let p1 = Pos2::new(
            center.x + angle1.cos() * radius,
            center.y + angle1.sin() * radius,
        );
        painter.line_segment([p0, p1], Stroke::new(3.0, color));
    }

    // Center disk
    painter.circle_filled(center, radius * 0.65, Theme::bg2());
    painter.circle_stroke(
        center,
        radius * 0.65,
        Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 15)),
    );

    // Indicator dot
    let pos = state.positions[index];
    let dot_x = center.x + pos[0] * radius * 0.6;
    let dot_y = center.y + pos[1] * radius * 0.6;
    painter.circle_filled(Pos2::new(dot_x, dot_y), 5.0, Color32::WHITE);
    painter.circle_stroke(
        Pos2::new(dot_x, dot_y),
        5.0,
        Stroke::new(2.0, Color32::from_rgba_premultiplied(0, 0, 0, 180)),
    );

    // Drag interaction
    if response.dragged() {
        state.dragging = Some(index);
        if let Some(pointer) = response.interact_pointer_pos() {
            let dx = (pointer.x - center.x) / (radius * 0.6);
            let dy = (pointer.y - center.y) / (radius * 0.6);
            let dist = (dx * dx + dy * dy).sqrt();
            let max_dist = 1.0;
            if dist > max_dist {
                state.positions[index] = [dx / dist * max_dist, dy / dist * max_dist];
            } else {
                state.positions[index] = [dx, dy];
            }
        }
    }
    if response.drag_stopped() && state.dragging == Some(index) {
        state.dragging = None;
    }
}

fn draw_scope(ui: &mut egui::Ui, width: f32, height: f32, color: Color32, time: f64, seed: u32) {
    let (response, painter) =
        ui.allocate_painter(Vec2::new(width, height), egui::Sense::hover());
    let rect = response.rect;

    // Background
    painter.rect_filled(rect, Rounding::same(4.0), Color32::from_rgb(7, 7, 13));

    // Grid lines
    for i in 1..=4 {
        let y = rect.top() + (i as f32 / 5.0) * height;
        painter.line_segment(
            [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
            Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 9)),
        );
    }

    // Waveform
    let steps = width as usize;
    let scope_color = Theme::with_alpha(color, 178);
    let mut prev = None;
    for s in 0..steps {
        let x = rect.left() + s as f32;
        let t = s as f64 / steps as f64;
        let wave = (t * 6.0 + time * 0.5 + seed as f64 * 1.7).sin() * 0.3
            + (t * 14.0 + time * 0.3).sin() * 0.15
            + (t * 31.0 + seed as f64 * 3.1).sin() * 0.08;
        let y = rect.center().y - wave as f32 * height * 0.4;

        if let Some(prev_pos) = prev {
            painter.line_segment([prev_pos, Pos2::new(x, y)], Stroke::new(0.8, scope_color));
        }
        prev = Some(Pos2::new(x, y));
    }
}
