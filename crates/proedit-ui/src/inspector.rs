//! Right-side inspector panel with collapsible sections.

use crate::theme::Theme;
use crate::widgets;
use egui::{self, Color32, Rounding, Stroke, Vec2};

// ── Data ───────────────────────────────────────────────────────

/// Actions emitted by the inspector when properties change.
#[derive(Debug)]
pub enum InspectorAction {
    PropertyChanged { clip_id: usize },
}

/// Represents a selected clip's inspectable properties.
pub struct InspectorClip {
    pub clip_id: Option<usize>,
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

impl InspectorClip {
    /// Create an inspector clip from a name, color and clip type with sensible defaults.
    pub fn new(
        clip_id: Option<usize>,
        name: String,
        color: Color32,
        clip_type: ClipType,
        out_point: f32,
    ) -> Self {
        Self {
            clip_id,
            name,
            color,
            clip_type,
            pos_x: 960.0,
            pos_y: 540.0,
            scale: 100.0,
            rotation: 0.0,
            opacity: 100.0,
            speed: 100.0,
            in_point: 0.0,
            out_point,
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

pub fn show_inspector(ui: &mut egui::Ui, state: &mut InspectorState) -> Vec<InspectorAction> {
    let mut actions = Vec::new();
    let Some(clip) = &mut state.clip else {
        // Empty state
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() * 0.3);
            ui.label(
                egui::RichText::new("\u{25C7}")
                    .size(28.0)
                    .color(Theme::white_15()),
            );
            ui.add_space(Theme::SPACE_SM);
            ui.label(
                egui::RichText::new("Select a clip to inspect")
                    .size(Theme::FONT_XS)
                    .color(Theme::t4()),
            );
        });
        return actions;
    };
    let prev = (
        clip.pos_x,
        clip.pos_y,
        clip.scale,
        clip.rotation,
        clip.opacity,
        clip.speed,
    );

    ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);

    // ── Header ─────────────────────────────────────────────
    let header_frame = egui::Frame::none()
        .stroke(Stroke::new(Theme::STROKE_SUBTLE, Theme::white_08()))
        .inner_margin(egui::Margin::symmetric(Theme::SPACE_MD, 9.0));

    header_frame.show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);
            // Color dot
            let (dot_resp, dot_painter) =
                ui.allocate_painter(Vec2::splat(10.0), egui::Sense::hover());
            dot_painter.rect_filled(dot_resp.rect, Rounding::same(3.0), clip.color);

            ui.label(
                egui::RichText::new(&clip.name)
                    .size(Theme::FONT_SM)
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
                widgets::themed_slider(ui, "Pos X", &mut clip.pos_x, 0.0..=1920.0, Theme::accent());
                widgets::themed_slider(ui, "Pos Y", &mut clip.pos_y, 0.0..=1080.0, Theme::accent());
                widgets::themed_slider(ui, "Scale", &mut clip.scale, 0.0..=400.0, Theme::accent());
                widgets::themed_slider(
                    ui,
                    "Rotation",
                    &mut clip.rotation,
                    -360.0..=360.0,
                    Theme::accent(),
                );
                widgets::themed_slider(
                    ui,
                    "Opacity",
                    &mut clip.opacity,
                    0.0..=100.0,
                    Theme::accent(),
                );
            });

            // ── Speed & Time section ───────────────────────
            collapsible_section(ui, "Speed & Time", &mut state.speed_open, |ui| {
                widgets::themed_slider(ui, "Speed", &mut clip.speed, 10.0..=400.0, Theme::accent());
                let max_point = clip.out_point.max(clip.in_point).max(1.0);
                widgets::themed_slider(
                    ui,
                    "In Point",
                    &mut clip.in_point,
                    0.0..=max_point,
                    Theme::accent(),
                );
                widgets::themed_slider(
                    ui,
                    "Out Point",
                    &mut clip.out_point,
                    0.0..=max_point,
                    Theme::accent(),
                );
            });

            // ── Audio section (only for audio clips) ───────
            if clip.clip_type == ClipType::Audio {
                collapsible_section(ui, "Audio", &mut state.audio_open, |ui| {
                    widgets::themed_slider(
                        ui,
                        "Volume",
                        &mut clip.volume,
                        0.0..=100.0,
                        Theme::green(),
                    );
                    widgets::themed_slider(ui, "Pan", &mut clip.pan, -100.0..=100.0, Theme::cyan());
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);
                        ui.label(
                            egui::RichText::new("EQ Enabled")
                                .size(Theme::FONT_XS)
                                .color(Theme::t3()),
                        );
                        if widgets::toggle_switch(ui, clip.eq_enabled) {
                            clip.eq_enabled = !clip.eq_enabled;
                        }
                    });
                });
            }

            // ── Effects section ────────────────────────────
            collapsible_section(ui, "Effects", &mut state.effects_open, |ui| {
                let add_btn = egui::Frame::none()
                    .stroke(Stroke::new(Theme::STROKE_EMPHASIS, Theme::t4()))
                    .rounding(Rounding::same(Theme::RADIUS))
                    .inner_margin(egui::Margin::symmetric(0.0, 10.0));
                add_btn.show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("+ Add Effect")
                                .size(Theme::FONT_XS)
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
                        .rounding(Rounding::same(Theme::RADIUS))
                        .inner_margin(egui::Margin::symmetric(Theme::SPACE_SM, 5.0));

                    let resp = item_frame
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
                                ui.label(
                                    egui::RichText::new("\u{2726}")
                                        .size(Theme::FONT_XS)
                                        .color(Theme::with_alpha(Theme::purple(), 100)),
                                );
                                ui.label(
                                    egui::RichText::new(*item)
                                        .size(Theme::FONT_XS)
                                        .color(Theme::with_alpha(Theme::purple(), 100)),
                                );
                            });
                        })
                        .response;

                    let hovered = resp.hovered();
                    let hover_rect = resp.rect;
                    resp.on_hover_text("Requires AI model (not loaded)");

                    if hovered {
                        ui.painter().rect_filled(
                            hover_rect,
                            Rounding::same(Theme::RADIUS),
                            Theme::with_alpha(Theme::purple(), 8),
                        );
                        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                    }
                }
            });
        });

    // Detect property changes
    let curr = (
        clip.pos_x,
        clip.pos_y,
        clip.scale,
        clip.rotation,
        clip.opacity,
        clip.speed,
    );
    if prev != curr {
        actions.push(InspectorAction::PropertyChanged {
            clip_id: clip.clip_id.unwrap_or(0),
        });
    }

    actions
}

// ── Helpers ────────────────────────────────────────────────────

fn collapsible_section(
    ui: &mut egui::Ui,
    title: &str,
    open: &mut bool,
    content: impl FnOnce(&mut egui::Ui),
) {
    let chevron = if *open { "\u{25BE}" } else { "\u{25B8}" };

    ui.add_space(Theme::SPACE_XS);
    Theme::draw_separator(ui);

    // Use a proper Button for reliable click detection
    let header_text = format!("{} {}", chevron, title);
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
        *open = !*open;
    }

    if *open {
        ui.indent(title, |ui| {
            ui.spacing_mut().item_spacing = Vec2::new(0.0, Theme::SPACE_XS);
            content(ui);
        });
    }
}
