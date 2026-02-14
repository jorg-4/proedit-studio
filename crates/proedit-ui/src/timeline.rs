//! Full-featured timeline with toolbar, ruler, track headers, clips, and playhead.

use egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use crate::theme::Theme;

// ── Clip data (the 9 clips from React reference) ───────────────

#[derive(Debug, Clone)]
pub struct TimelineClip {
    pub id: usize,
    pub name: &'static str,
    pub color: Color32,
    pub start: f32,   // frame offset
    pub dur: f32,     // duration in frames
    pub track: usize, // 0-5 (V3,V2,V1,A1,A2,A3)
    pub clip_type: ClipKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipKind {
    Video,
    Audio,
    Gfx,
}

pub const CLIPS: &[TimelineClip] = &[
    TimelineClip { id: 1, name: "Hero_Shot_01",  color: Color32::from_rgb(78, 133, 255),  start: 0.0,   dur: 160.0, track: 0, clip_type: ClipKind::Video },
    TimelineClip { id: 2, name: "B-Roll_City",   color: Color32::from_rgb(167, 139, 250), start: 160.0, dur: 100.0, track: 0, clip_type: ClipKind::Video },
    TimelineClip { id: 3, name: "Interview_A",   color: Color32::from_rgb(48, 213, 160),  start: 260.0, dur: 180.0, track: 0, clip_type: ClipKind::Video },
    TimelineClip { id: 4, name: "Title_Card",    color: Color32::from_rgb(244, 114, 182), start: 40.0,  dur: 80.0,  track: 1, clip_type: ClipKind::Gfx },
    TimelineClip { id: 5, name: "Lower_Third",   color: Color32::from_rgb(255, 184, 48),  start: 270.0, dur: 70.0,  track: 1, clip_type: ClipKind::Gfx },
    TimelineClip { id: 6, name: "Overlay_Lens",  color: Color32::from_rgb(34, 211, 238),  start: 140.0, dur: 120.0, track: 2, clip_type: ClipKind::Video },
    TimelineClip { id: 7, name: "VO_Take3",      color: Color32::from_rgb(48, 213, 160),  start: 0.0,   dur: 220.0, track: 3, clip_type: ClipKind::Audio },
    TimelineClip { id: 8, name: "SFX_Whoosh",    color: Color32::from_rgb(244, 114, 182), start: 155.0, dur: 30.0,  track: 4, clip_type: ClipKind::Audio },
    TimelineClip { id: 9, name: "Music_Bed",     color: Color32::from_rgb(129, 140, 248), start: 0.0,   dur: 440.0, track: 5, clip_type: ClipKind::Audio },
];

const TRACK_NAMES: &[&str] = &["V3", "V2", "V1", "A1", "A2", "A3"];
const TRACK_HEIGHT: f32 = 36.0;
const RULER_HEIGHT: f32 = 20.0;
const TOOLBAR_HEIGHT: f32 = 28.0;
const HEADER_WIDTH: f32 = 50.0;

// ── Marker ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Marker {
    pub frame: f32,
    pub color: Color32,
}

// ── State ──────────────────────────────────────────────────────

pub struct TimelineState {
    pub zoom: f32,
    pub scroll_x: f32,
    pub playhead: f32,
    pub selected_clip: Option<usize>,
    pub snap_enabled: bool,
    pub ripple_enabled: bool,
    pub markers: Vec<Marker>,
    pub track_muted: [bool; 6],
    pub hovered_clip: Option<usize>,
}

impl Default for TimelineState {
    fn default() -> Self {
        Self {
            zoom: 1.2,
            scroll_x: 0.0,
            playhead: 0.0,
            selected_clip: None,
            snap_enabled: true,
            ripple_enabled: false,
            markers: vec![
                Marker { frame: 80.0,  color: Theme::amber() },
                Marker { frame: 200.0, color: Theme::accent() },
                Marker { frame: 350.0, color: Theme::green() },
            ],
            track_muted: [false; 6],
            hovered_clip: None,
        }
    }
}

// ── Actions ────────────────────────────────────────────────────

#[derive(Debug)]
pub enum TimelineAction {
    SelectClip(Option<usize>),
    SeekTo(f32),
    AddMarker(f32),
    ToggleSnap,
    ToggleRipple,
    ZoomIn,
    ZoomOut,
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_timeline(ui: &mut egui::Ui, state: &mut TimelineState) -> Vec<TimelineAction> {
    let mut actions = Vec::new();
    let available = ui.available_size();

    ui.vertical(|ui| {
        // ── Toolbar ────────────────────────────────────────
        draw_toolbar(ui, state, &mut actions);

        // ── Timeline body ──────────────────────────────────
        let body_height = available.y - TOOLBAR_HEIGHT - 4.0;
        let body_width = available.x;

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::ZERO;

            // Track headers
            ui.allocate_ui(Vec2::new(HEADER_WIDTH, body_height), |ui| {
                ui.add_space(RULER_HEIGHT); // offset for ruler
                draw_track_headers(ui, state);
            });

            // Ruler + clips area
            let clips_width = body_width - HEADER_WIDTH;
            ui.allocate_ui(Vec2::new(clips_width, body_height), |ui| {
                let (response, painter) = ui.allocate_painter(
                    Vec2::new(clips_width, body_height),
                    egui::Sense::click_and_drag(),
                );
                let rect = response.rect;

                // Background
                painter.rect_filled(rect, 0.0, Theme::bg());

                // Ruler
                let ruler_rect = Rect::from_min_size(rect.min, Vec2::new(clips_width, RULER_HEIGHT));
                draw_ruler(&painter, ruler_rect, state);

                // Track lanes
                let tracks_top = rect.top() + RULER_HEIGHT;
                for i in 0..6 {
                    let lane_top = tracks_top + i as f32 * TRACK_HEIGHT;
                    let lane_rect = Rect::from_min_size(
                        Pos2::new(rect.left(), lane_top),
                        Vec2::new(clips_width, TRACK_HEIGHT),
                    );
                    let lane_bg = if i % 2 == 0 {
                        Color32::from_rgba_premultiplied(255, 255, 255, 2)
                    } else {
                        Color32::TRANSPARENT
                    };
                    painter.rect_filled(lane_rect, 0.0, lane_bg);

                    // Lane bottom border
                    painter.line_segment(
                        [
                            Pos2::new(lane_rect.left(), lane_rect.bottom()),
                            Pos2::new(lane_rect.right(), lane_rect.bottom()),
                        ],
                        Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 4)),
                    );
                }

                // Clips
                state.hovered_clip = None;
                for clip in CLIPS {
                    let clip_left = rect.left() + clip.start * state.zoom - state.scroll_x;
                    let clip_width = (clip.dur * state.zoom).max(20.0);
                    let clip_top = tracks_top + clip.track as f32 * TRACK_HEIGHT + 3.0;

                    let clip_rect = Rect::from_min_size(
                        Pos2::new(clip_left, clip_top),
                        Vec2::new(clip_width, 30.0),
                    );

                    // Skip if out of view
                    if clip_rect.right() < rect.left() || clip_rect.left() > rect.right() {
                        continue;
                    }

                    let is_selected = state.selected_clip == Some(clip.id);
                    let is_hovered = response.hovered()
                        && response.hover_pos().is_some_and(|p| clip_rect.contains(p));

                    if is_hovered {
                        state.hovered_clip = Some(clip.id);
                    }

                    // Clip background
                    let (bg_alpha, border_alpha, border_width) = if is_selected {
                        (48, 128, 1.0)
                    } else if is_hovered {
                        (30, 64, 1.0)
                    } else {
                        (20, 30, 0.5)
                    };

                    painter.rect_filled(
                        clip_rect,
                        Rounding::same(7.0),
                        Theme::with_alpha(clip.color, bg_alpha),
                    );
                    painter.rect_stroke(
                        clip_rect,
                        Rounding::same(7.0),
                        Stroke::new(border_width, Theme::with_alpha(clip.color, border_alpha)),
                    );

                    // Selected glow
                    if is_selected {
                        painter.rect_stroke(
                            clip_rect.expand(1.0),
                            Rounding::same(8.0),
                            Stroke::new(1.0, Theme::with_alpha(clip.color, 20)),
                        );
                    }

                    // Audio waveform pattern
                    if clip.clip_type == ClipKind::Audio {
                        let step = 3.0;
                        let mut x = clip_rect.left() + 2.0;
                        while x < clip_rect.right() - 2.0 {
                            painter.line_segment(
                                [
                                    Pos2::new(x, clip_rect.top() + 4.0),
                                    Pos2::new(x, clip_rect.bottom() - 4.0),
                                ],
                                Stroke::new(1.0, Theme::with_alpha(clip.color, 30)),
                            );
                            x += step;
                        }
                    }

                    // Clip name
                    let text_rect = clip_rect.shrink2(Vec2::new(6.0, 0.0));
                    if text_rect.width() > 20.0 {
                        painter.text(
                            Pos2::new(text_rect.left(), text_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            clip.name,
                            egui::FontId::proportional(9.0),
                            Theme::t1(),
                        );
                    }

                    // Trim handles on hover/select
                    if is_selected || is_hovered {
                        // Left handle
                        let left_handle = Rect::from_min_size(
                            clip_rect.min,
                            Vec2::new(4.0, clip_rect.height()),
                        );
                        painter.rect_filled(
                            left_handle,
                            Rounding {
                                nw: 7.0, sw: 7.0, ne: 0.0, se: 0.0,
                            },
                            Theme::with_alpha(clip.color, 60),
                        );
                        // Right handle
                        let right_handle = Rect::from_min_size(
                            Pos2::new(clip_rect.right() - 4.0, clip_rect.top()),
                            Vec2::new(4.0, clip_rect.height()),
                        );
                        painter.rect_filled(
                            right_handle,
                            Rounding {
                                nw: 0.0, sw: 0.0, ne: 7.0, se: 7.0,
                            },
                            Theme::with_alpha(clip.color, 60),
                        );
                    }
                }

                // Marker lines
                for marker in &state.markers {
                    let mx = rect.left() + marker.frame * state.zoom - state.scroll_x;
                    if mx >= rect.left() && mx <= rect.right() {
                        // Diamond on ruler
                        let diamond_y = ruler_rect.bottom() - 4.0;
                        let diamond_size = 3.0;
                        let diamond = egui::epaint::PathShape::convex_polygon(
                            vec![
                                Pos2::new(mx, diamond_y - diamond_size),
                                Pos2::new(mx + diamond_size, diamond_y),
                                Pos2::new(mx, diamond_y + diamond_size),
                                Pos2::new(mx - diamond_size, diamond_y),
                            ],
                            marker.color,
                            Stroke::NONE,
                        );
                        painter.add(diamond);

                        // Vertical line through tracks
                        painter.line_segment(
                            [
                                Pos2::new(mx, tracks_top),
                                Pos2::new(mx, tracks_top + 6.0 * TRACK_HEIGHT),
                            ],
                            Stroke::new(1.0, Theme::with_alpha(marker.color, 51)),
                        );
                    }
                }

                // Playhead
                let ph_x = rect.left() + state.playhead * state.zoom - state.scroll_x;
                if ph_x >= rect.left() && ph_x <= rect.right() {
                    // Playhead indicator triangle on ruler
                    let tri_w = 5.0;
                    let tri_h = 6.0;
                    let tri_y = ruler_rect.bottom();
                    let tri = egui::epaint::PathShape::convex_polygon(
                        vec![
                            Pos2::new(ph_x - tri_w, tri_y - tri_h),
                            Pos2::new(ph_x + tri_w, tri_y - tri_h),
                            Pos2::new(ph_x, tri_y),
                        ],
                        Theme::red(),
                        Stroke::NONE,
                    );
                    painter.add(tri);

                    // Playhead line
                    painter.line_segment(
                        [
                            Pos2::new(ph_x, ruler_rect.bottom()),
                            Pos2::new(ph_x, rect.bottom()),
                        ],
                        Stroke::new(1.5, Theme::red()),
                    );
                    // Glow
                    painter.line_segment(
                        [
                            Pos2::new(ph_x, ruler_rect.bottom()),
                            Pos2::new(ph_x, rect.bottom()),
                        ],
                        Stroke::new(4.0, Theme::with_alpha(Theme::red(), 30)),
                    );
                }

                // Click handling
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        // Check if clicking on a clip
                        let mut clicked_clip = None;
                        for clip in CLIPS {
                            let clip_left = rect.left() + clip.start * state.zoom - state.scroll_x;
                            let clip_width = (clip.dur * state.zoom).max(20.0);
                            let clip_top = tracks_top + clip.track as f32 * TRACK_HEIGHT + 3.0;
                            let clip_rect = Rect::from_min_size(
                                Pos2::new(clip_left, clip_top),
                                Vec2::new(clip_width, 30.0),
                            );
                            if clip_rect.contains(pos) {
                                clicked_clip = Some(clip.id);
                                break;
                            }
                        }

                        if let Some(id) = clicked_clip {
                            state.selected_clip = Some(id);
                            actions.push(TimelineAction::SelectClip(Some(id)));
                        } else if pos.y < tracks_top {
                            // Clicked on ruler → seek
                            let frame = (pos.x - rect.left() + state.scroll_x) / state.zoom;
                            state.playhead = frame.max(0.0);
                            actions.push(TimelineAction::SeekTo(state.playhead));
                        } else {
                            state.selected_clip = None;
                            actions.push(TimelineAction::SelectClip(None));
                        }
                    }
                }

                // Drag on ruler to scrub
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        if pos.y < tracks_top {
                            let frame = (pos.x - rect.left() + state.scroll_x) / state.zoom;
                            state.playhead = frame.max(0.0);
                            actions.push(TimelineAction::SeekTo(state.playhead));
                        }
                    }
                }
            });
        });
    });

    actions
}

// ── Sub-components ─────────────────────────────────────────────

fn draw_toolbar(ui: &mut egui::Ui, state: &mut TimelineState, actions: &mut Vec<TimelineAction>) {
    let toolbar_frame = egui::Frame::none()
        .fill(Theme::bg1())
        .stroke(Stroke::new(0.5, Theme::with_alpha(Color32::WHITE, 6)))
        .inner_margin(egui::Margin::symmetric(8.0, 0.0));

    toolbar_frame.show(ui, |ui| {
        ui.set_height(TOOLBAR_HEIGHT);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(8.0, 0.0);

            // Snap toggle
            let snap_label_color = if state.snap_enabled { Theme::t1() } else { Theme::t3() };
            if draw_mini_toggle(ui, state.snap_enabled) {
                state.snap_enabled = !state.snap_enabled;
                actions.push(TimelineAction::ToggleSnap);
            }
            ui.label(egui::RichText::new("Snap").size(10.5).color(snap_label_color));

            ui.add_space(8.0);

            // Ripple toggle
            let ripple_label_color = if state.ripple_enabled { Theme::t1() } else { Theme::t3() };
            if draw_mini_toggle(ui, state.ripple_enabled) {
                state.ripple_enabled = !state.ripple_enabled;
                actions.push(TimelineAction::ToggleRipple);
            }
            ui.label(egui::RichText::new("Ripple").size(10.5).color(ripple_label_color));

            ui.add_space(12.0);

            // Timecode
            let ph = state.playhead as i32;
            let total = 440;
            let fmt = |f: i32| -> String {
                let s = f / 24;
                let ff = f % 24;
                let m = s / 60;
                let ss = s % 60;
                format!("{:02}:{:02}:{:02}", m, ss, ff)
            };
            ui.label(
                egui::RichText::new(format!("{} / {}", fmt(ph), fmt(total)))
                    .size(8.5)
                    .color(Theme::t4())
                    .family(egui::FontFamily::Monospace),
            );

            // Separator
            ui.add_space(4.0);
            let (sep_resp, sep_painter) = ui.allocate_painter(Vec2::new(1.0, 12.0), egui::Sense::hover());
            sep_painter.rect_filled(
                sep_resp.rect,
                0.0,
                Color32::from_rgba_premultiplied(255, 255, 255, 10),
            );
            ui.add_space(4.0);

            // Zoom controls
            if ui.small_button(
                egui::RichText::new("\u{2212}").size(12.0).color(Theme::t3()),
            ).clicked() {
                state.zoom = (state.zoom - 0.2).max(0.4);
                actions.push(TimelineAction::ZoomOut);
            }

            // Zoom slider bar
            let (zoom_resp, zoom_painter) =
                ui.allocate_painter(Vec2::new(50.0, 14.0), egui::Sense::click_and_drag());
            let zoom_rect = zoom_resp.rect;
            let bar_rect = Rect::from_center_size(
                zoom_rect.center(),
                Vec2::new(50.0, 3.0),
            );
            zoom_painter.rect_filled(
                bar_rect,
                Rounding::same(1.5),
                Color32::from_rgba_premultiplied(255, 255, 255, 10),
            );
            let zoom_frac = ((state.zoom - 0.4) / 2.6).clamp(0.0, 1.0);
            let thumb_x = bar_rect.left() + zoom_frac * bar_rect.width();
            zoom_painter.circle_filled(
                Pos2::new(thumb_x, bar_rect.center().y),
                3.5,
                Theme::accent(),
            );

            if zoom_resp.dragged() || zoom_resp.clicked() {
                if let Some(pos) = zoom_resp.interact_pointer_pos() {
                    let frac = ((pos.x - bar_rect.left()) / bar_rect.width()).clamp(0.0, 1.0);
                    state.zoom = 0.4 + frac * 2.6;
                }
            }

            if ui.small_button(
                egui::RichText::new("+").size(12.0).color(Theme::t3()),
            ).clicked() {
                state.zoom = (state.zoom + 0.2).min(3.0);
                actions.push(TimelineAction::ZoomIn);
            }
        });
    });
}

fn draw_track_headers(ui: &mut egui::Ui, state: &mut TimelineState) {
    for (i, name) in TRACK_NAMES.iter().enumerate() {
        let is_video = i < 3;
        let text_color = if is_video {
            Theme::with_alpha(Theme::accent(), 144)
        } else {
            Theme::with_alpha(Theme::green(), 144)
        };

        let header_frame = egui::Frame::none()
            .fill(Theme::bg1())
            .stroke(Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 4)))
            .inner_margin(egui::Margin::symmetric(4.0, 0.0));

        header_frame.show(ui, |ui| {
            ui.set_height(TRACK_HEIGHT);
            ui.set_width(HEADER_WIDTH - 8.0);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(4.0, 0.0);
                ui.label(
                    egui::RichText::new(*name)
                        .size(8.0)
                        .color(text_color)
                        .strong(),
                );
                // Mute button
                let mute_color = if state.track_muted[i] {
                    Theme::with_alpha(Theme::red(), 230)
                } else {
                    Theme::with_alpha(Theme::t3(), 102)
                };
                let mute_resp = ui.label(
                    egui::RichText::new("M")
                        .size(7.0)
                        .color(mute_color),
                );
                if mute_resp.clicked() {
                    state.track_muted[i] = !state.track_muted[i];
                }
            });
        });
    }
}

fn draw_ruler(painter: &egui::Painter, rect: Rect, state: &TimelineState) {
    // Background
    painter.rect_filled(rect, 0.0, Color32::from_rgba_premultiplied(255, 255, 255, 2));
    // Bottom border
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 8)),
    );

    // Tick marks
    let tick_spacing = 12.0 * state.zoom;
    let start_tick = (state.scroll_x / tick_spacing) as i32;
    let end_tick = start_tick + (rect.width() / tick_spacing) as i32 + 2;

    for i in start_tick..end_tick {
        let x = rect.left() + i as f32 * tick_spacing - state.scroll_x;
        if x < rect.left() || x > rect.right() {
            continue;
        }

        let is_major = i % 5 == 0;
        let is_label = i % 10 == 0;

        let tick_height = if is_major { 12.0 } else { 5.0 };
        let tick_alpha: u8 = if is_major { 26 } else { 8 };

        painter.line_segment(
            [
                Pos2::new(x, rect.bottom() - tick_height),
                Pos2::new(x, rect.bottom()),
            ],
            Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, tick_alpha)),
        );

        if is_label && i >= 0 {
            let frame = i as f32 * 12.0;
            let secs = frame / 24.0;
            let label = format!("{:.0}s", secs);
            painter.text(
                Pos2::new(x + 2.0, rect.top() + 3.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::monospace(7.0),
                Theme::t4(),
            );
        }
    }
}

/// Draw a small toggle (returns true if clicked).
fn draw_mini_toggle(ui: &mut egui::Ui, on: bool) -> bool {
    let desired_size = Vec2::new(32.0, 18.0);
    let (resp, painter) = ui.allocate_painter(desired_size, egui::Sense::click());
    let rect = resp.rect;

    let (track_bg, track_border) = if on {
        (Theme::with_alpha(Theme::accent(), 102), Theme::with_alpha(Theme::accent(), 153))
    } else {
        (
            Color32::from_rgba_premultiplied(255, 255, 255, 15),
            Color32::from_rgba_premultiplied(255, 255, 255, 20),
        )
    };
    painter.rect_filled(rect, Rounding::same(9.0), track_bg);
    painter.rect_stroke(rect, Rounding::same(9.0), Stroke::new(0.5, track_border));

    let thumb_x = if on { rect.right() - 9.0 } else { rect.left() + 9.0 };
    let thumb_color = if on { Theme::accent() } else { Color32::from_rgba_premultiplied(255, 255, 255, 64) };
    painter.circle_filled(Pos2::new(thumb_x, rect.center().y), 7.0, thumb_color);

    resp.clicked()
}
