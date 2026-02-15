//! Full-featured timeline with toolbar, ruler, track headers, clips, and playhead.

use crate::snapping::SnappingEngine;
use crate::theme::Theme;
use crate::trim::{apply_trim, hit_test_trim_handle, trim_cursor, ClipDragState, TrimState};
use crate::widgets;
use egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};
use std::collections::HashMap;

// ── Clip data ────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct TimelineClip {
    pub id: usize,
    pub name: String,
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

const TRACK_NAMES: &[&str] = &["V3", "V2", "V1", "A1", "A2", "A3"];
const TRACK_HEIGHT: f32 = 36.0;
const RULER_HEIGHT: f32 = 20.0;
const TOOLBAR_HEIGHT: f32 = 28.0;
const HEADER_WIDTH: f32 = 50.0;
const EMPTY_ICON_SIZE: f32 = 28.0;

// ── Marker ─────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Marker {
    pub frame: f32,
    pub color: Color32,
}

// ── State ──────────────────────────────────────────────────────

pub const TRACK_COUNT: usize = TRACK_NAMES.len();

pub struct TimelineState {
    pub zoom: f32,
    pub scroll_x: f32,
    pub playhead: f32,
    pub selected_clip: Option<usize>,
    pub snap_enabled: bool,
    pub ripple_enabled: bool,
    pub markers: Vec<Marker>,
    pub track_muted: [bool; TRACK_COUNT],
    pub hovered_clip: Option<usize>,
    pub fps: f32,
    pub clips: Vec<TimelineClip>,
    // Interactive features
    pub trim_state: Option<TrimState>,
    pub drag_state: Option<ClipDragState>,
    pub selection: Vec<usize>,
    pub rubber_band: Option<Rect>,
    pub razor_mode: bool,
    pub snapping: SnappingEngine,
    pub track_locked: [bool; TRACK_COUNT],
    pub track_solo: [bool; TRACK_COUNT],
    /// Cached waveform data per clip ID: Vec of [min, max] pairs for display.
    pub waveform_cache: HashMap<usize, Vec<[f32; 2]>>,
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
            markers: Vec::new(),
            track_muted: [false; TRACK_COUNT],
            hovered_clip: None,
            fps: 24.0,
            clips: Vec::new(),
            trim_state: None,
            drag_state: None,
            selection: Vec::new(),
            rubber_band: None,
            razor_mode: false,
            snapping: SnappingEngine::new(),
            track_locked: [false; TRACK_COUNT],
            track_solo: [false; TRACK_COUNT],
            waveform_cache: HashMap::new(),
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
    TrimClip {
        clip_id: usize,
        new_start: f32,
        new_dur: f32,
    },
    DragClip {
        clip_id: usize,
        new_start: f32,
        new_track: usize,
    },
    SplitClip {
        clip_id: usize,
        offset: f32,
    },
    MultiSelect(Vec<usize>),
    DeselectAll,
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_timeline(ui: &mut egui::Ui, state: &mut TimelineState) -> Vec<TimelineAction> {
    let mut actions = Vec::new();
    let available = ui.available_size();

    ui.vertical(|ui| {
        // ── Toolbar ────────────────────────────────────────
        draw_toolbar(ui, state, &mut actions);

        // ── Timeline body ──────────────────────────────────
        let body_height = available.y - TOOLBAR_HEIGHT - Theme::SPACE_XS;
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
                let ruler_rect =
                    Rect::from_min_size(rect.min, Vec2::new(clips_width, RULER_HEIGHT));
                draw_ruler(&painter, ruler_rect, state);

                // Track lanes
                let tracks_top = rect.top() + RULER_HEIGHT;
                for i in 0..TRACK_COUNT {
                    let lane_top = tracks_top + i as f32 * TRACK_HEIGHT;
                    let lane_rect = Rect::from_min_size(
                        Pos2::new(rect.left(), lane_top),
                        Vec2::new(clips_width, TRACK_HEIGHT),
                    );
                    let lane_bg = if i % 2 == 0 {
                        Theme::white_02()
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
                        Stroke::new(Theme::STROKE_SUBTLE, Theme::white_04()),
                    );
                }

                // Clips — empty state
                if state.clips.is_empty() {
                    let center = Pos2::new(
                        rect.center().x,
                        tracks_top + (TRACK_COUNT as f32 * TRACK_HEIGHT) * 0.5,
                    );
                    painter.text(
                        Pos2::new(center.x, center.y - Theme::SPACE_SM),
                        egui::Align2::CENTER_CENTER,
                        "\u{25AC}",
                        egui::FontId::proportional(EMPTY_ICON_SIZE),
                        Theme::white_10(),
                    );
                    painter.text(
                        Pos2::new(center.x, center.y + Theme::SPACE_LG - 6.0),
                        egui::Align2::CENTER_CENTER,
                        "Drag media here to start editing",
                        egui::FontId::proportional(Theme::FONT_XS),
                        Theme::t4(),
                    );
                }

                let mut new_hovered_clip: Option<usize> = None;
                for clip in &state.clips {
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
                        new_hovered_clip = Some(clip.id);
                    }

                    // Clip background
                    let (bg_alpha, border_alpha, border_width) = if is_selected {
                        (48, 128, 1.0)
                    } else if is_hovered {
                        (30, 64, 1.0)
                    } else {
                        (20, 30, Theme::STROKE_SUBTLE)
                    };

                    painter.rect_filled(
                        clip_rect,
                        Rounding::same(Theme::RADIUS),
                        Theme::with_alpha(clip.color, bg_alpha),
                    );
                    painter.rect_stroke(
                        clip_rect,
                        Rounding::same(Theme::RADIUS),
                        Stroke::new(border_width, Theme::with_alpha(clip.color, border_alpha)),
                    );

                    // Selected glow
                    if is_selected {
                        painter.rect_stroke(
                            clip_rect.expand(1.0),
                            Rounding::same(Theme::RADIUS + 1.0),
                            Stroke::new(1.0, Theme::with_alpha(clip.color, 20)),
                        );
                    }

                    // Audio waveform
                    if clip.clip_type == ClipKind::Audio {
                        if let Some(waveform) = state.waveform_cache.get(&clip.id) {
                            // Draw real waveform from cached data
                            let num_samples = waveform.len();
                            if num_samples > 0 {
                                let pixels = clip_width as usize;
                                let samples_per_px = (num_samples as f32 / pixels as f32).max(1.0);
                                let mid_y = clip_rect.center().y;
                                let half_h = clip_rect.height() * 0.4;
                                for px in 0..pixels.min(clip_rect.width() as usize) {
                                    let si = (px as f32 * samples_per_px) as usize;
                                    if si >= num_samples {
                                        break;
                                    }
                                    let [min_v, max_v] = waveform[si];
                                    let y_top = mid_y - max_v * half_h;
                                    let y_bot = mid_y - min_v * half_h;
                                    let x = clip_rect.left() + px as f32;
                                    painter.line_segment(
                                        [Pos2::new(x, y_top), Pos2::new(x, y_bot)],
                                        Stroke::new(1.0, Theme::with_alpha(clip.color, 80)),
                                    );
                                }
                            }
                        } else {
                            // Fallback: placeholder bars
                            let step = 3.0;
                            let mut x = clip_rect.left() + 2.0;
                            while x < clip_rect.right() - 2.0 {
                                painter.line_segment(
                                    [
                                        Pos2::new(x, clip_rect.top() + Theme::SPACE_XS),
                                        Pos2::new(x, clip_rect.bottom() - Theme::SPACE_XS),
                                    ],
                                    Stroke::new(1.0, Theme::with_alpha(clip.color, 30)),
                                );
                                x += step;
                            }
                        }
                    }

                    // Clip name
                    let text_rect = clip_rect.shrink2(Vec2::new(6.0, 0.0));
                    if text_rect.width() > 20.0 {
                        painter.text(
                            Pos2::new(text_rect.left(), text_rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &clip.name,
                            egui::FontId::proportional(Theme::FONT_XS),
                            Theme::t1(),
                        );
                    }

                    // Trim handles on hover/select
                    if is_selected || is_hovered {
                        // Left handle
                        let left_handle =
                            Rect::from_min_size(clip_rect.min, Vec2::new(4.0, clip_rect.height()));
                        painter.rect_filled(
                            left_handle,
                            Rounding {
                                nw: Theme::RADIUS,
                                sw: Theme::RADIUS,
                                ne: 0.0,
                                se: 0.0,
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
                                nw: 0.0,
                                sw: 0.0,
                                ne: Theme::RADIUS,
                                se: Theme::RADIUS,
                            },
                            Theme::with_alpha(clip.color, 60),
                        );
                    }
                }
                state.hovered_clip = new_hovered_clip;

                // Trim handle hover cursor
                if response.hovered() && state.trim_state.is_none() && state.drag_state.is_none() {
                    if let Some(hover_pos) = response.hover_pos() {
                        for clip in &state.clips {
                            let cr =
                                clip_rect_for(clip, tracks_top, &rect, state.zoom, state.scroll_x);
                            if let Some(edge) = hit_test_trim_handle(cr, hover_pos, 6.0) {
                                ui.ctx().set_cursor_icon(trim_cursor(edge));
                                break;
                            }
                        }
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
                                Pos2::new(mx, tracks_top + TRACK_COUNT as f32 * TRACK_HEIGHT),
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

                // Snap indicator line
                if let Some(ref drag) = state.drag_state {
                    if let Some(snap_frame) = drag.snap_indicator {
                        let sx = rect.left() + snap_frame * state.zoom - state.scroll_x;
                        if sx >= rect.left() && sx <= rect.right() {
                            painter.line_segment(
                                [Pos2::new(sx, tracks_top), Pos2::new(sx, rect.bottom())],
                                Stroke::new(1.0, Theme::cyan()),
                            );
                        }
                    }
                }

                // Click handling
                if response.clicked() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        // Check if clicking on a clip
                        let mut clicked_clip = None;
                        for clip in &state.clips {
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

                // --- Drag handling (trim / move / ruler scrub) ---
                if response.dragged() {
                    if let Some(pos) = response.interact_pointer_pos() {
                        // Initiate new interaction if none active
                        if state.trim_state.is_none()
                            && state.drag_state.is_none()
                            && pos.y >= tracks_top
                        {
                            for clip in &state.clips {
                                let cr = clip_rect_for(
                                    clip,
                                    tracks_top,
                                    &rect,
                                    state.zoom,
                                    state.scroll_x,
                                );
                                if let Some(edge) = hit_test_trim_handle(cr, pos, 6.0) {
                                    let frame = ((pos.x - rect.left() + state.scroll_x)
                                        / state.zoom)
                                        .max(0.0);
                                    state.trim_state = Some(TrimState::new(clip, edge, frame));
                                    break;
                                }
                                if cr.contains(pos) {
                                    let frame = ((pos.x - rect.left() + state.scroll_x)
                                        / state.zoom)
                                        .max(0.0);
                                    state.drag_state = Some(ClipDragState {
                                        clip_id: clip.id,
                                        offset_frame: frame - clip.start,
                                        original_track: clip.track,
                                        snap_indicator: None,
                                    });
                                    state.selected_clip = Some(clip.id);
                                    actions.push(TimelineAction::SelectClip(Some(clip.id)));
                                    break;
                                }
                            }
                        }

                        // Process active trim
                        if let Some((clip_id, edge)) =
                            state.trim_state.as_ref().map(|t| (t.clip_id, t.edge))
                        {
                            let frame =
                                ((pos.x - rect.left() + state.scroll_x) / state.zoom).max(0.0);
                            let snap_points = SnappingEngine::collect_snap_points(state);
                            let snapped = if state.snap_enabled {
                                state
                                    .snapping
                                    .find_snap(frame, &snap_points, state.zoom, Some(clip_id))
                                    .unwrap_or(frame)
                            } else {
                                frame
                            };
                            if let Some(trim) = state.trim_state.as_mut() {
                                trim.current_frame = snapped;
                            }
                            if let Some(trim_snap) = state.trim_state.clone() {
                                if let Some(clip) = state.clips.iter_mut().find(|c| c.id == clip_id)
                                {
                                    apply_trim(clip, &trim_snap, snapped);
                                }
                            }
                            ui.ctx().set_cursor_icon(trim_cursor(edge));
                        }
                        // Process active drag
                        else if let Some((clip_id, offset)) = state
                            .drag_state
                            .as_ref()
                            .map(|d| (d.clip_id, d.offset_frame))
                        {
                            let frame =
                                ((pos.x - rect.left() + state.scroll_x) / state.zoom).max(0.0);
                            let target_frame = frame - offset;
                            let track = ((pos.y - tracks_top) / TRACK_HEIGHT).floor() as usize;

                            let snapped = if state.snap_enabled {
                                if let Some(cc) =
                                    state.clips.iter().find(|c| c.id == clip_id).cloned()
                                {
                                    state.snapping.snap_clip(&cc, target_frame, state)
                                } else {
                                    target_frame
                                }
                            } else {
                                target_frame
                            };

                            if let Some(clip) = state.clips.iter_mut().find(|c| c.id == clip_id) {
                                clip.start = snapped.max(0.0);
                                clip.track = track.min(TRACK_COUNT - 1);
                            }

                            if let Some(drag) = state.drag_state.as_mut() {
                                drag.snap_indicator = if (snapped - target_frame).abs() > 0.01 {
                                    Some(snapped)
                                } else {
                                    None
                                };
                            }
                        }
                        // Ruler scrub (only if not trimming or dragging)
                        else if pos.y < tracks_top {
                            let frame = (pos.x - rect.left() + state.scroll_x) / state.zoom;
                            state.playhead = frame.max(0.0);
                            actions.push(TimelineAction::SeekTo(state.playhead));
                        }
                    }
                }

                // --- Drag release: emit final actions ---
                if response.drag_stopped() {
                    if let Some(trim) = state.trim_state.take() {
                        if let Some(clip) = state.clips.iter().find(|c| c.id == trim.clip_id) {
                            actions.push(TimelineAction::TrimClip {
                                clip_id: trim.clip_id,
                                new_start: clip.start,
                                new_dur: clip.dur,
                            });
                        }
                    }
                    if let Some(drag) = state.drag_state.take() {
                        if let Some(clip) = state.clips.iter().find(|c| c.id == drag.clip_id) {
                            actions.push(TimelineAction::DragClip {
                                clip_id: drag.clip_id,
                                new_start: clip.start,
                                new_track: clip.track,
                            });
                        }
                    }
                }
            });
        });
    });

    actions
}

// ── Helpers ──────────────────────────────────────────────────────

fn clip_rect_for(
    clip: &TimelineClip,
    tracks_top: f32,
    rect: &Rect,
    zoom: f32,
    scroll_x: f32,
) -> Rect {
    let x = rect.left() + clip.start * zoom - scroll_x;
    let w = (clip.dur * zoom).max(20.0);
    let y = tracks_top + clip.track as f32 * TRACK_HEIGHT + 3.0;
    Rect::from_min_size(Pos2::new(x, y), Vec2::new(w, 30.0))
}

// ── Sub-components ─────────────────────────────────────────────

fn draw_toolbar(ui: &mut egui::Ui, state: &mut TimelineState, actions: &mut Vec<TimelineAction>) {
    let toolbar_frame = egui::Frame::none()
        .fill(Theme::bg1())
        .stroke(Stroke::new(Theme::STROKE_SUBTLE, Theme::white_06()))
        .inner_margin(egui::Margin::symmetric(Theme::SPACE_SM, 0.0));

    toolbar_frame.show(ui, |ui| {
        ui.set_height(TOOLBAR_HEIGHT);
        ui.horizontal_centered(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);

            // Snap toggle
            let snap_label_color = if state.snap_enabled {
                Theme::t1()
            } else {
                Theme::t3()
            };
            if widgets::toggle_switch(ui, state.snap_enabled) {
                state.snap_enabled = !state.snap_enabled;
                actions.push(TimelineAction::ToggleSnap);
            }
            ui.label(
                egui::RichText::new("Snap")
                    .size(Theme::FONT_XS)
                    .color(snap_label_color),
            );

            ui.add_space(Theme::SPACE_SM);

            // Ripple toggle
            let ripple_label_color = if state.ripple_enabled {
                Theme::t1()
            } else {
                Theme::t3()
            };
            if widgets::toggle_switch(ui, state.ripple_enabled) {
                state.ripple_enabled = !state.ripple_enabled;
                actions.push(TimelineAction::ToggleRipple);
            }
            ui.label(
                egui::RichText::new("Ripple")
                    .size(Theme::FONT_XS)
                    .color(ripple_label_color),
            );

            ui.add_space(Theme::SPACE_MD);

            // Timecode
            let ph = state.playhead as i32;
            let total = state
                .clips
                .iter()
                .map(|c| (c.start + c.dur) as i32)
                .max()
                .unwrap_or(0);
            let fps_int = (state.fps.round() as i32).max(1);
            let fmt = |f: i32| -> String {
                let s = f / fps_int;
                let ff = f % fps_int;
                let m = s / 60;
                let ss = s % 60;
                format!("{:02}:{:02}:{:02}", m, ss, ff)
            };
            ui.label(
                egui::RichText::new(format!("{} / {}", fmt(ph), fmt(total)))
                    .size(Theme::FONT_XS)
                    .color(Theme::t4())
                    .family(egui::FontFamily::Monospace),
            );

            // Separator
            ui.add_space(Theme::SPACE_XS);
            let (sep_resp, sep_painter) =
                ui.allocate_painter(Vec2::new(1.0, 12.0), egui::Sense::hover());
            sep_painter.rect_filled(sep_resp.rect, 0.0, Theme::white_10());
            ui.add_space(Theme::SPACE_XS);

            // Zoom controls
            if ui
                .small_button(
                    egui::RichText::new("\u{2212}")
                        .size(Theme::FONT_SM)
                        .color(Theme::t3()),
                )
                .clicked()
            {
                state.zoom = (state.zoom - 0.2).max(0.4);
                actions.push(TimelineAction::ZoomOut);
            }

            // Zoom slider bar
            let (zoom_resp, zoom_painter) =
                ui.allocate_painter(Vec2::new(50.0, 14.0), egui::Sense::click_and_drag());
            let zoom_rect = zoom_resp.rect;
            let bar_rect = Rect::from_center_size(zoom_rect.center(), Vec2::new(50.0, 3.0));
            zoom_painter.rect_filled(bar_rect, Rounding::same(1.5), Theme::white_10());
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

            if ui
                .small_button(
                    egui::RichText::new("+")
                        .size(Theme::FONT_SM)
                        .color(Theme::t3()),
                )
                .clicked()
            {
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
            .stroke(Stroke::new(Theme::STROKE_SUBTLE, Theme::white_04()))
            .inner_margin(egui::Margin::symmetric(Theme::SPACE_XS, 0.0));

        header_frame.show(ui, |ui| {
            ui.set_height(TRACK_HEIGHT);
            ui.set_width(HEADER_WIDTH - Theme::SPACE_SM);
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_XS, 0.0);
                ui.label(
                    egui::RichText::new(*name)
                        .size(Theme::FONT_XS)
                        .color(text_color)
                        .strong(),
                );
                // Mute button
                let mute_color = if state.track_muted[i] {
                    Theme::with_alpha(Theme::red(), 230)
                } else {
                    Theme::with_alpha(Theme::t3(), 102)
                };
                let mute_btn = egui::Button::new(
                    egui::RichText::new("M")
                        .size(Theme::FONT_XS)
                        .color(mute_color),
                )
                .fill(Color32::TRANSPARENT)
                .stroke(Stroke::NONE)
                .rounding(Rounding::same(2.0));

                if ui.add(mute_btn).clicked() {
                    state.track_muted[i] = !state.track_muted[i];
                }
            });
        });
    }
}

fn draw_ruler(painter: &egui::Painter, rect: Rect, state: &TimelineState) {
    let fps = state.fps;
    // Background
    painter.rect_filled(rect, 0.0, Theme::white_02());
    // Bottom border
    painter.line_segment(
        [
            Pos2::new(rect.left(), rect.bottom()),
            Pos2::new(rect.right(), rect.bottom()),
        ],
        Stroke::new(Theme::STROKE_SUBTLE, Theme::white_08()),
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
        let tick_color = if is_major {
            Theme::white_10()
        } else {
            Theme::white_04()
        };

        painter.line_segment(
            [
                Pos2::new(x, rect.bottom() - tick_height),
                Pos2::new(x, rect.bottom()),
            ],
            Stroke::new(Theme::STROKE_SUBTLE, tick_color),
        );

        if is_label && i >= 0 {
            let frame = i as f32 * 12.0;
            let secs = frame / fps;
            let label = format!("{:.0}s", secs);
            painter.text(
                Pos2::new(x + 2.0, rect.top() + 3.0),
                egui::Align2::LEFT_TOP,
                &label,
                egui::FontId::monospace(Theme::FONT_XS),
                Theme::t4(),
            );
        }
    }
}
