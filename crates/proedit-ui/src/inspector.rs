//! Right-side inspector panel with collapsible sections.

use egui::{self, Color32, Rounding, Stroke, Vec2};
use crate::theme::Theme;

// ── Data ───────────────────────────────────────────────────────

/// Represents a selected clip's inspectable properties.
pub struct InspectorClip {
    pub name: String,
    pub color: Color32,
    pub clip_type: ClipType,
    // Transform
    pub pos_x: f32,
    pub pos_y: f32,
    pub scale: f32,
    pub rotation: f32,
    pub opacity: f32,
    // Speed & Time
    pub speed: f32,
    pub in_point: f32,
    pub out_point: f32,
    // Audio (only for audio clips)
    pub volume: f32,
    pub pan: f32,
    pub eq_enabled: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipType {
    Video,
    Audio,
    Gfx,
}

impl Default for InspectorClip {
    fn default() -> Self {
        Self {
            name: "Hero_Shot_01".into(),
            color: Theme::accent(),
            clip_type: ClipType::Video,
            pos_x: 960.0,
            pos_y: 540.0,
            scale: 100.0,
            rotation: 0.0,
            opacity: 100.0,
            speed: 100.0,
            in_point: 0.0,
            out_point: 440.0,
            volume: 80.0,
            pan: 0.0,
            eq_enabled: false,
        }
    }
}

// ── State ──────────────────────────────────────────────────────

pub struct InspectorState {
    pub clip: Option<InspectorClip>,
    pub transform_open: bool,
    pub speed_open: bool,
    pub audio_open: bool,
    pub effects_open: bool,
    pub ai_open: bool,
}

impl Default for InspectorState {
    fn default() -> Self {
        Self {
            clip: None,
            transform_open: true,
            speed_open: false,
            audio_open: false,
            effects_open: false,
            ai_open: false,
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_inspector(ui: &mut egui::Ui, state: &mut InspectorState) {
    let Some(clip) = &mut state.clip else {
        // Empty state
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.label(
                egui::RichText::new("\u{25C7}")
                    .size(28.0)
                    .color(Color32::from_rgba_premultiplied(255, 255, 255, 38)),
            );
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Select a clip to inspect")
                    .size(11.0)
                    .color(Theme::t4()),
            );
        });
        return;
    };

    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

    // ── Header ─────────────────────────────────────────────
    let header_frame = egui::Frame::none()
        .stroke(Stroke::new(0.5, Theme::with_alpha(Color32::WHITE, 8)))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0));

    header_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
            // Color dot
            let (dot_resp, dot_painter) = ui.allocate_painter(Vec2::splat(10.0), egui::Sense::hover());
            dot_painter.rect_filled(dot_resp.rect, Rounding::same(3.0), clip.color);

            ui.label(
                egui::RichText::new(&clip.name)
                    .size(12.0)
                    .color(Theme::t1())
                    .strong(),
            );
        });
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // ── Transform section ──────────────────────────
            collapsible_section(ui, "Transform", &mut state.transform_open, |ui| {
                themed_slider(ui, "Pos X", &mut clip.pos_x, 0.0..=1920.0, Theme::accent());
                themed_slider(ui, "Pos Y", &mut clip.pos_y, 0.0..=1080.0, Theme::accent());
                themed_slider(ui, "Scale", &mut clip.scale, 0.0..=400.0, Theme::accent());
                themed_slider(ui, "Rotation", &mut clip.rotation, -360.0..=360.0, Theme::accent());
                themed_slider(ui, "Opacity", &mut clip.opacity, 0.0..=100.0, Theme::accent());
            });

            // ── Speed & Time section ───────────────────────
            collapsible_section(ui, "Speed & Time", &mut state.speed_open, |ui| {
                themed_slider(ui, "Speed", &mut clip.speed, 10.0..=400.0, Theme::accent());
                themed_slider(ui, "In Point", &mut clip.in_point, 0.0..=440.0, Theme::accent());
                themed_slider(ui, "Out Point", &mut clip.out_point, 0.0..=440.0, Theme::accent());
            });

            // ── Audio section (only for audio clips) ───────
            if clip.clip_type == ClipType::Audio {
                collapsible_section(ui, "Audio", &mut state.audio_open, |ui| {
                    themed_slider(ui, "Volume", &mut clip.volume, 0.0..=100.0, Theme::green());
                    themed_slider(ui, "Pan", &mut clip.pan, -100.0..=100.0, Theme::cyan());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);
                        ui.label(egui::RichText::new("EQ Enabled").size(10.0).color(Theme::t3()));
                        themed_toggle(ui, &mut clip.eq_enabled);
                    });
                });
            }

            // ── Effects section ────────────────────────────
            collapsible_section(ui, "Effects", &mut state.effects_open, |ui| {
                let add_btn = egui::Frame::none()
                    .stroke(Stroke::new(1.0, Theme::t4()))
                    .rounding(Rounding::same(8.0))
                    .inner_margin(egui::Margin::symmetric(0.0, 10.0));
                add_btn.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("+ Add Effect")
                                .size(10.0)
                                .color(Theme::t4()),
                        );
                    });
                });
            });

            // ── AI Actions section ─────────────────────────
            collapsible_section(ui, "AI Actions", &mut state.ai_open, |ui| {
                let ai_items = [
                    "Auto Color Match",
                    "Remove Background",
                    "Smart Stabilize",
                    "Enhance Audio",
                    "Upscale 4K",
                    "Style Transfer",
                ];
                for item in &ai_items {
                    let item_frame = egui::Frame::none()
                        .rounding(Rounding::same(7.0))
                        .inner_margin(egui::Margin::symmetric(8.0, 5.0));

                    let resp = item_frame.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                            ui.label(
                                egui::RichText::new("\u{2726}")
                                    .size(10.0)
                                    .color(Theme::purple()),
                            );
                            ui.label(
                                egui::RichText::new(*item)
                                    .size(10.5)
                                    .color(Theme::purple()),
                            );
                        });
                    }).response;

                    if resp.hovered() {
                        ui.painter().rect_filled(
                            resp.rect,
                            Rounding::same(7.0),
                            Theme::with_alpha(Theme::purple(), 12),
                        );
                    }
                }
            });
        });
}

// ── Helpers ────────────────────────────────────────────────────

fn collapsible_section(
    ui: &mut egui::Ui,
    title: &str,
    open: &mut bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let chevron = if *open { "\u{25BE}" } else { "\u{25B8}" };

    ui.add_space(4.0);
    // Separator
    let sep_rect = ui.allocate_space(Vec2::new(ui.available_width(), 0.5));
    ui.painter().rect_filled(
        egui::Rect::from_min_size(sep_rect.1.min, Vec2::new(sep_rect.1.width(), 0.5)),
        0.0,
        Theme::with_alpha(Color32::WHITE, 8),
    );

    let header_resp = ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
        ui.add_space(4.0);
        ui.label(egui::RichText::new(chevron).size(8.0).color(Theme::t4()));
        ui.label(
            egui::RichText::new(title)
                .size(9.5)
                .color(Theme::t3())
                .strong(),
        );
    }).response;

    if header_resp.clicked() {
        *open = !*open;
    }

    if *open {
        ui.indent(title, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, 4.0);
            content(ui);
        });
    }
}

/// Custom themed slider matching the React reference.
fn themed_slider(
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
                        .size(10.0)
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

        let bar_rect = egui::Rect::from_center_size(
            track_rect.center(),
            Vec2::new(track_width, track_height),
        );

        // Background track
        track_painter.rect_filled(
            bar_rect,
            Rounding::same(2.0),
            Color32::from_rgba_premultiplied(2, 2, 2, 10),
        );

        // Fill
        let min = *range.start();
        let max = *range.end();
        let frac = if max > min {
            (*value - min) / (max - min)
        } else {
            0.0
        };
        let fill_width = bar_rect.width() * frac.clamp(0.0, 1.0);
        let fill_rect = egui::Rect::from_min_size(bar_rect.min, Vec2::new(fill_width, track_height));
        track_painter.rect_filled(fill_rect, Rounding::same(2.0), accent);

        // Thumb
        let thumb_x = bar_rect.left() + fill_width;
        let thumb_center = egui::Pos2::new(thumb_x, bar_rect.center().y);
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
                        .size(10.0)
                        .color(Theme::t2())
                        .family(egui::FontFamily::Monospace),
                );
            });
        });
    });
}

/// Custom themed toggle switch.
fn themed_toggle(ui: &mut egui::Ui, on: &mut bool) {
    let desired_size = Vec2::new(32.0, 18.0);
    let (resp, painter) = ui.allocate_painter(desired_size, egui::Sense::click());

    if resp.clicked() {
        *on = !*on;
    }

    let rect = resp.rect;

    // Track
    let (track_bg, track_border) = if *on {
        (Theme::with_alpha(Theme::accent(), 102), Theme::with_alpha(Theme::accent(), 153))
    } else {
        (
            Color32::from_rgba_premultiplied(2, 2, 2, 15),
            Color32::from_rgba_premultiplied(2, 2, 2, 20),
        )
    };
    painter.rect_filled(rect, Rounding::same(9.0), track_bg);
    painter.rect_stroke(rect, Rounding::same(9.0), Stroke::new(0.5, track_border));

    // Thumb
    let thumb_x = if *on {
        rect.right() - 9.0
    } else {
        rect.left() + 9.0
    };
    let thumb_color = if *on { Theme::accent() } else { Color32::from_rgba_premultiplied(64, 64, 64, 255) };
    painter.circle_filled(egui::Pos2::new(thumb_x, rect.center().y), 7.0, thumb_color);
}
