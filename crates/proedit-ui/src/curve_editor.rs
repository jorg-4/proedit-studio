//! Curve editor for keyframe animation.

use egui::{self, Color32, Pos2, Rect, Stroke, Vec2};
use proedit_core::keyframe::{EasingCurve, KeyframeTrack};
use proedit_core::time::{FrameRate, RationalTime};

use crate::theme::Theme;

/// Whether to display value graph or speed graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurveView {
    ValueGraph,
    SpeedGraph,
}

/// State for the curve editor panel.
pub struct CurveEditorState {
    pub view: CurveView,
    pub zoom_x: f32,
    pub zoom_y: f32,
    pub pan: Vec2,
    pub selected_keyframes: Vec<usize>,
    pub drag_state: Option<KeyframeDragState>,
    pub property_name: String,
    pub visible: bool,
}

impl Default for CurveEditorState {
    fn default() -> Self {
        Self {
            view: CurveView::ValueGraph,
            zoom_x: 1.0,
            zoom_y: 1.0,
            pan: Vec2::ZERO,
            selected_keyframes: Vec::new(),
            drag_state: None,
            property_name: String::new(),
            visible: false,
        }
    }
}

/// What part of a keyframe is being dragged.
#[derive(Debug, Clone)]
pub struct KeyframeDragState {
    pub keyframe_index: usize,
    pub drag_type: DragType,
    pub start_pos: Pos2,
}

/// Type of drag on a keyframe.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DragType {
    Body,
    TangentIn,
    TangentOut,
}

/// Actions emitted by the curve editor.
#[derive(Debug)]
pub enum CurveEditorAction {
    MoveKeyframe {
        index: usize,
        new_time: RationalTime,
        new_value: f64,
    },
    SetTangent {
        index: usize,
        tangent_in: [f32; 2],
        tangent_out: [f32; 2],
    },
    AddKeyframe {
        time: RationalTime,
        value: f64,
    },
    DeleteKeyframes {
        indices: Vec<usize>,
    },
    SetInterpolation {
        indices: Vec<usize>,
        easing: EasingCurve,
    },
}

const DIAMOND_SIZE: f32 = 5.0;
const GRID_LINE_ALPHA: u8 = 25;
const CURVE_STROKE_WIDTH: f32 = 1.5;

/// Render the curve editor panel.
pub fn show_curve_editor(
    ui: &mut egui::Ui,
    state: &mut CurveEditorState,
    track: &KeyframeTrack,
    frame_rate: FrameRate,
) -> Vec<CurveEditorAction> {
    let mut actions = Vec::new();

    if !state.visible {
        return actions;
    }

    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
    let rect = response.rect;

    // Background — subtle gradient feel
    painter.rect_filled(rect, 0.0, Theme::bg());
    // Slight darkening at bottom for depth
    let bottom_grad = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - rect.height() * 0.3),
        rect.max,
    );
    painter.rect_filled(
        bottom_grad,
        0.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 6),
    );

    // Border with accent-tinted top line
    painter.rect_stroke(rect, 0.0, Stroke::new(0.5, Theme::white_06()));
    painter.line_segment(
        [rect.left_top(), Pos2::new(rect.right(), rect.top())],
        Stroke::new(1.0, Theme::with_alpha(Theme::accent(), 30)),
    );

    if track.is_empty() {
        painter.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "No keyframes",
            egui::FontId::proportional(Theme::FONT_SM),
            Theme::t4(),
        );
        return actions;
    }

    let keyframes = track.keyframes();
    let kf_count = keyframes.len();
    if kf_count == 0 {
        return actions;
    }

    // Compute value range
    let (min_val, max_val) = compute_value_range(keyframes);
    let val_range = (max_val - min_val).max(1.0);

    // Compute time range
    let first_time = keyframes[0].time.to_seconds_f64() as f32;
    let last_time = keyframes[kf_count - 1].time.to_seconds_f64() as f32;
    let time_range = (last_time - first_time).max(0.1);

    // Margins
    let margin = 40.0;
    let plot_rect = rect.shrink(margin);

    // Map helpers
    let time_to_x = |t: f32| -> f32 {
        let frac = (t - first_time) / time_range;
        plot_rect.left() + frac * plot_rect.width() * state.zoom_x + state.pan.x
    };
    let val_to_y = |v: f32| -> f32 {
        let frac = (v - min_val as f32) / val_range as f32;
        plot_rect.bottom() - frac * plot_rect.height() * state.zoom_y - state.pan.y
    };
    let x_to_time = |x: f32| -> f32 {
        let frac = (x - plot_rect.left() - state.pan.x) / (plot_rect.width() * state.zoom_x);
        first_time + frac * time_range
    };
    let y_to_val = |y: f32| -> f32 {
        let frac = (plot_rect.bottom() - y + state.pan.y) / (plot_rect.height() * state.zoom_y);
        min_val as f32 + frac * val_range as f32
    };

    // Draw grid
    draw_grid(
        &painter, plot_rect, state, first_time, time_range, min_val, val_range, frame_rate,
    );

    // Draw curve by sampling
    match state.view {
        CurveView::ValueGraph => {
            draw_value_curve(
                &painter, plot_rect, track, frame_rate, &time_to_x, &val_to_y, first_time,
                time_range,
            );
        }
        CurveView::SpeedGraph => {
            draw_speed_curve(
                &painter, plot_rect, track, frame_rate, &time_to_x, &val_to_y, first_time,
                time_range,
            );
        }
    }

    // Draw keyframe diamonds
    for (i, kf) in keyframes.iter().enumerate() {
        let t = kf.time.to_seconds_f64() as f32;
        let v = kf.value as f32;
        let pos = Pos2::new(time_to_x(t), val_to_y(v));

        let is_selected = state.selected_keyframes.contains(&i);
        let fill = if is_selected {
            Theme::accent()
        } else {
            Theme::t2()
        };

        // Glow behind selected diamonds
        if is_selected {
            painter.circle_filled(
                pos,
                DIAMOND_SIZE + 5.0,
                Theme::with_alpha(Theme::accent(), 18),
            );
        }

        // Diamond shape
        let diamond = egui::epaint::PathShape::convex_polygon(
            vec![
                Pos2::new(pos.x, pos.y - DIAMOND_SIZE),
                Pos2::new(pos.x + DIAMOND_SIZE, pos.y),
                Pos2::new(pos.x, pos.y + DIAMOND_SIZE),
                Pos2::new(pos.x - DIAMOND_SIZE, pos.y),
            ],
            fill,
            Stroke::new(1.0, Theme::white_25()),
        );
        painter.add(diamond);
    }

    // Property label
    if !state.property_name.is_empty() {
        painter.text(
            Pos2::new(rect.left() + 8.0, rect.top() + 4.0),
            egui::Align2::LEFT_TOP,
            &state.property_name,
            egui::FontId::proportional(Theme::FONT_XS),
            Theme::t3(),
        );
    }

    // View toggle label
    let view_label = match state.view {
        CurveView::ValueGraph => "Value",
        CurveView::SpeedGraph => "Speed",
    };
    painter.text(
        Pos2::new(rect.right() - 8.0, rect.top() + 4.0),
        egui::Align2::RIGHT_TOP,
        view_label,
        egui::FontId::proportional(Theme::FONT_XS),
        Theme::t4(),
    );

    // Click handling
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if plot_rect.contains(pos) {
                // Check if clicking on a keyframe
                let mut clicked_kf = None;
                for (i, kf) in keyframes.iter().enumerate() {
                    let t = kf.time.to_seconds_f64() as f32;
                    let v = kf.value as f32;
                    let kf_pos = Pos2::new(time_to_x(t), val_to_y(v));
                    if (kf_pos - pos).length() < DIAMOND_SIZE * 2.0 {
                        clicked_kf = Some(i);
                        break;
                    }
                }

                if let Some(idx) = clicked_kf {
                    state.selected_keyframes = vec![idx];
                } else {
                    state.selected_keyframes.clear();
                }
            }
        }
    }

    // Double-click to add keyframe
    if response.double_clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if plot_rect.contains(pos) {
                let t = x_to_time(pos.x);
                let v = y_to_val(pos.y);
                actions.push(CurveEditorAction::AddKeyframe {
                    time: RationalTime::from_seconds_f64(t as f64),
                    value: v as f64,
                });
            }
        }
    }

    // Context menu
    response.context_menu(|ui| {
        if ui.button("Add Keyframe").clicked() {
            if let Some(pos) = ui.input(|i| i.pointer.hover_pos()) {
                if plot_rect.contains(pos) {
                    let t = x_to_time(pos.x);
                    let v = y_to_val(pos.y);
                    actions.push(CurveEditorAction::AddKeyframe {
                        time: RationalTime::from_seconds_f64(t as f64),
                        value: v as f64,
                    });
                }
            }
            ui.close_menu();
        }
        if !state.selected_keyframes.is_empty() {
            ui.separator();
            if ui.button("Delete Selected").clicked() {
                actions.push(CurveEditorAction::DeleteKeyframes {
                    indices: state.selected_keyframes.clone(),
                });
                state.selected_keyframes.clear();
                ui.close_menu();
            }
            if ui.button("Set Linear").clicked() {
                actions.push(CurveEditorAction::SetInterpolation {
                    indices: state.selected_keyframes.clone(),
                    easing: EasingCurve::Linear,
                });
                ui.close_menu();
            }
            if ui.button("Set Hold").clicked() {
                actions.push(CurveEditorAction::SetInterpolation {
                    indices: state.selected_keyframes.clone(),
                    easing: EasingCurve::Hold,
                });
                ui.close_menu();
            }
        }
        ui.separator();
        if ui.button("Value Graph").clicked() {
            state.view = CurveView::ValueGraph;
            ui.close_menu();
        }
        if ui.button("Speed Graph").clicked() {
            state.view = CurveView::SpeedGraph;
            ui.close_menu();
        }
    });

    actions
}

fn compute_value_range(keyframes: &[proedit_core::keyframe::Keyframe]) -> (f64, f64) {
    let mut min_val = f64::MAX;
    let mut max_val = f64::MIN;
    for kf in keyframes {
        min_val = min_val.min(kf.value);
        max_val = max_val.max(kf.value);
    }
    // Add some padding
    let pad = (max_val - min_val) * 0.1;
    (min_val - pad.max(0.5), max_val + pad.max(0.5))
}

#[allow(clippy::too_many_arguments)]
fn draw_grid(
    painter: &egui::Painter,
    plot_rect: Rect,
    state: &CurveEditorState,
    first_time: f32,
    time_range: f32,
    min_val: f64,
    val_range: f64,
    frame_rate: FrameRate,
) {
    let grid_color = Color32::from_rgba_premultiplied(
        GRID_LINE_ALPHA,
        GRID_LINE_ALPHA,
        GRID_LINE_ALPHA,
        GRID_LINE_ALPHA,
    );

    // Vertical grid lines (time)
    let frame_dur = frame_rate.frame_duration().to_seconds_f64() as f32;
    let time_step = compute_time_grid_step(time_range / state.zoom_x, frame_dur);
    if time_step > 0.0 {
        let mut t = (first_time / time_step).floor() * time_step;
        while t <= first_time + time_range {
            let frac = (t - first_time) / time_range;
            let x = plot_rect.left() + frac * plot_rect.width() * state.zoom_x + state.pan.x;
            if x >= plot_rect.left() && x <= plot_rect.right() {
                painter.line_segment(
                    [
                        Pos2::new(x, plot_rect.top()),
                        Pos2::new(x, plot_rect.bottom()),
                    ],
                    Stroke::new(0.5, grid_color),
                );
            }
            t += time_step;
        }
    }

    // Horizontal grid lines (value)
    let val_step = compute_value_grid_step(val_range as f32 / state.zoom_y);
    if val_step > 0.0 {
        let mut v = ((min_val as f32) / val_step).floor() * val_step;
        while (v as f64) <= min_val + val_range {
            let frac = (v - min_val as f32) / val_range as f32;
            let y = plot_rect.bottom() - frac * plot_rect.height() * state.zoom_y - state.pan.y;
            if y >= plot_rect.top() && y <= plot_rect.bottom() {
                painter.line_segment(
                    [
                        Pos2::new(plot_rect.left(), y),
                        Pos2::new(plot_rect.right(), y),
                    ],
                    Stroke::new(0.5, grid_color),
                );
                // Value label
                painter.text(
                    Pos2::new(plot_rect.left() - 4.0, y),
                    egui::Align2::RIGHT_CENTER,
                    format!("{:.0}", v),
                    egui::FontId::monospace(9.0),
                    Theme::t4(),
                );
            }
            v += val_step;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_value_curve(
    painter: &egui::Painter,
    plot_rect: Rect,
    track: &KeyframeTrack,
    _frame_rate: FrameRate,
    time_to_x: &dyn Fn(f32) -> f32,
    val_to_y: &dyn Fn(f32) -> f32,
    first_time: f32,
    time_range: f32,
) {
    let samples = (plot_rect.width() as usize).max(10);
    let mut points = Vec::with_capacity(samples);

    for i in 0..samples {
        let frac = i as f32 / (samples - 1) as f32;
        let t = first_time + frac * time_range;
        let time = RationalTime::from_seconds_f64(t as f64);
        let v = track.evaluate(time) as f32;
        let x = time_to_x(t);
        let y = val_to_y(v);
        if x >= plot_rect.left() && x <= plot_rect.right() {
            points.push(Pos2::new(x, y.clamp(plot_rect.top(), plot_rect.bottom())));
        }
    }

    if points.len() >= 2 {
        // Glow pass
        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(
                    CURVE_STROKE_WIDTH + 4.0,
                    Theme::with_alpha(Theme::accent(), 15),
                ),
            );
        }
        // Core pass
        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(CURVE_STROKE_WIDTH, Theme::accent()),
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_speed_curve(
    painter: &egui::Painter,
    plot_rect: Rect,
    track: &KeyframeTrack,
    _frame_rate: FrameRate,
    time_to_x: &dyn Fn(f32) -> f32,
    val_to_y: &dyn Fn(f32) -> f32,
    first_time: f32,
    time_range: f32,
) {
    let samples = (plot_rect.width() as usize).max(10);
    let dt = time_range / samples as f32;
    let mut points = Vec::with_capacity(samples);

    for i in 0..samples {
        let frac = i as f32 / (samples - 1) as f32;
        let t = first_time + frac * time_range;
        // Finite difference for speed
        let t0 = RationalTime::from_seconds_f64((t - dt * 0.5) as f64);
        let t1 = RationalTime::from_seconds_f64((t + dt * 0.5) as f64);
        let speed = if dt.abs() > 1e-6 {
            ((track.evaluate(t1) - track.evaluate(t0)) / dt as f64) as f32
        } else {
            0.0
        };
        let x = time_to_x(t);
        let y = val_to_y(speed);
        if x >= plot_rect.left() && x <= plot_rect.right() {
            points.push(Pos2::new(x, y.clamp(plot_rect.top(), plot_rect.bottom())));
        }
    }

    if points.len() >= 2 {
        let speed_color = Theme::green();
        // Glow pass
        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(CURVE_STROKE_WIDTH + 4.0, Theme::with_alpha(speed_color, 15)),
            );
        }
        // Core pass
        for pair in points.windows(2) {
            painter.line_segment(
                [pair[0], pair[1]],
                Stroke::new(CURVE_STROKE_WIDTH, speed_color),
            );
        }
    }
}

fn compute_time_grid_step(visible_range: f32, _frame_dur: f32) -> f32 {
    let target_lines = 8.0;
    let raw_step = visible_range / target_lines;
    // Snap to nice intervals
    let steps = [
        0.01, 0.02, 0.05, 0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 30.0, 60.0,
    ];
    for &s in &steps {
        if s >= raw_step {
            return s;
        }
    }
    60.0
}

fn compute_value_grid_step(visible_range: f32) -> f32 {
    let target_lines = 6.0;
    let raw_step = visible_range / target_lines;
    let steps = [
        0.1, 0.2, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0, 200.0, 500.0, 1000.0,
    ];
    for &s in &steps {
        if s >= raw_step {
            return s;
        }
    }
    1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_curve_editor_state_default() {
        let state = CurveEditorState::default();
        assert!(!state.visible);
        assert_eq!(state.view, CurveView::ValueGraph);
        assert!(state.selected_keyframes.is_empty());
    }

    #[test]
    fn test_compute_value_range() {
        let mut track = KeyframeTrack::new("test");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Linear);
        track.set(RationalTime::new(1, 1), 100.0, EasingCurve::Linear);
        let (min, max) = compute_value_range(track.keyframes());
        assert!(min < 0.0); // Has padding
        assert!(max > 100.0); // Has padding
    }

    #[test]
    fn test_time_grid_step() {
        let step = compute_time_grid_step(10.0, 1.0 / 24.0);
        assert!(step > 0.0);
        assert!(step <= 10.0);
    }

    #[test]
    fn test_value_grid_step() {
        let step = compute_value_grid_step(100.0);
        assert!(step > 0.0);
        assert!(step <= 100.0);
    }
}
