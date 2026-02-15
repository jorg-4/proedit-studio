//! Viewport / viewer with gradient background and transport overlay.

use crate::theme::Theme;
use egui::{self, Color32, Pos2, Rect, Rounding, Stroke, Vec2};

// ── Local domain constants ──────────────────────────────────────
const PLAY_ICON_SIZE: f32 = 40.0;

// ── State ──────────────────────────────────────────────────────

pub struct ViewerState {
    pub playing: bool,
    pub playhead_frames: f32,
    pub speed: f32,
    pub selected_clip: Option<usize>,
    pub fps: f32,
    pub has_media: bool,
}

impl Default for ViewerState {
    fn default() -> Self {
        Self {
            playing: false,
            playhead_frames: 0.0,
            speed: 1.0,
            selected_clip: None,
            fps: 24.0,
            has_media: false,
        }
    }
}

// ── Actions ────────────────────────────────────────────────────

#[derive(Debug)]
pub enum ViewerAction {
    TogglePlay,
    SetSpeed(f32),
}

// ── Rendering ──────────────────────────────────────────────────

pub fn show_viewer(ui: &mut egui::Ui, state: &ViewerState, time: f64) -> Vec<ViewerAction> {
    let mut actions = Vec::new();
    let available = ui.available_size();
    let (response, painter) = ui.allocate_painter(available, egui::Sense::click());
    let rect = response.rect;

    // ── Background — animated gradient ────────────────────────
    painter.rect_filled(rect, 0.0, Color32::from_rgb(11, 15, 23));

    // Animated ambient glow — drifting colored orbs
    let cx = rect.center().x;
    let cy = rect.center().y;
    let ph = state.playhead_frames as f64;

    // Blue glow (drifts with time)
    let bx = cx + (time * 0.3 + ph * 0.02).sin() as f32 * rect.width() * 0.18;
    let by = cy + (time * 0.2).cos() as f32 * rect.height() * 0.12;
    painter.circle_filled(
        Pos2::new(bx, by),
        rect.width() * 0.22,
        Color32::from_rgba_premultiplied(3, 5, 14, 9),
    );

    // Purple glow (counter-phase)
    let px = cx - (time * 0.25).cos() as f32 * rect.width() * 0.15;
    let py = cy + (time * 0.35 + 1.0).sin() as f32 * rect.height() * 0.10;
    painter.circle_filled(
        Pos2::new(px, py),
        rect.width() * 0.16,
        Color32::from_rgba_premultiplied(4, 2, 8, 6),
    );

    // Teal glow (smaller, faster)
    let tx = cx + (time * 0.5).cos() as f32 * rect.width() * 0.10;
    let ty = cy - (time * 0.4).sin() as f32 * rect.height() * 0.08;
    painter.circle_filled(
        Pos2::new(tx, ty),
        rect.width() * 0.10,
        Color32::from_rgba_premultiplied(1, 4, 6, 5),
    );

    // Safe area guides (10% inset)
    let safe_rect = rect.shrink2(rect.size() * 0.1);
    painter.rect_stroke(
        safe_rect,
        Rounding::same(2.0),
        Stroke::new(0.5, Color32::from_rgba_premultiplied(10, 10, 10, 10)),
    );

    // Request repaint for animation
    ui.ctx().request_repaint();

    // ── Idle / empty hint ─────────────────────────────────
    if !state.has_media {
        painter.text(
            Pos2::new(rect.center().x, rect.center().y - 10.0),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(PLAY_ICON_SIZE),
            Theme::white_10(),
        );
        painter.text(
            Pos2::new(rect.center().x, rect.center().y + 30.0),
            egui::Align2::CENTER_CENTER,
            "Open a video file to begin",
            egui::FontId::proportional(Theme::FONT_XS),
            Theme::t4(),
        );
    } else if !state.playing && state.selected_clip.is_none() {
        painter.text(
            Pos2::new(rect.center().x, rect.center().y - 10.0),
            egui::Align2::CENTER_CENTER,
            "\u{25B6}",
            egui::FontId::proportional(PLAY_ICON_SIZE),
            Theme::white_10(),
        );
        painter.text(
            Pos2::new(rect.center().x, rect.center().y + 30.0),
            egui::Align2::CENTER_CENTER,
            "SPACE TO PLAY \u{00B7} J K L SHUTTLE",
            egui::FontId::proportional(Theme::FONT_XS),
            Theme::t4(),
        );
    }

    // ── Transport overlay at bottom ────────────────────────
    let transport_height = 50.0;
    let transport_rect = Rect::from_min_max(
        Pos2::new(rect.left(), rect.bottom() - transport_height),
        rect.max,
    );

    // Gradient backdrop — darker scrim
    painter.rect_filled(
        transport_rect,
        0.0,
        Color32::from_rgba_premultiplied(0, 0, 0, 153), // rgba(0,0,0,.6)
    );
    // Glass top highlight line
    painter.line_segment(
        [
            Pos2::new(transport_rect.left(), transport_rect.top()),
            Pos2::new(transport_rect.right(), transport_rect.top()),
        ],
        Stroke::new(0.5, Color32::from_rgba_premultiplied(255, 255, 255, 12)),
    );

    let bar_y = transport_rect.center().y;
    let bar_left = transport_rect.left() + 14.0;

    // Play/pause button — semi-transparent glass-like fill
    let btn_size = 30.0;
    let btn_rect = Rect::from_center_size(
        Pos2::new(bar_left + btn_size * 0.5, bar_y),
        Vec2::splat(btn_size),
    );
    let (btn_bg, btn_color, btn_icon) = if state.playing {
        (
            Theme::with_alpha(Theme::red(), 38), // rgba(255,88,85,.15)
            Theme::t1(),
            "\u{23F8}",
        )
    } else {
        (
            Theme::with_alpha(Theme::accent(), 31), // rgba(78,133,255,.12)
            Theme::t1(),
            "\u{25B6}",
        )
    };
    painter.rect_filled(btn_rect, Rounding::same(8.0), btn_bg);
    painter.text(
        btn_rect.center(),
        egui::Align2::CENTER_CENTER,
        btn_icon,
        egui::FontId::proportional(Theme::FONT_SM),
        btn_color,
    );

    // Click play/pause (slightly enlarged hit area for usability)
    if response.clicked() {
        if let Some(pos) = response.interact_pointer_pos() {
            if btn_rect.expand(6.0).contains(pos) {
                actions.push(ViewerAction::TogglePlay);
            }
        }
    }
    // Hover cursor on button
    if response.hovered() {
        if let Some(pos) = response.hover_pos() {
            if btn_rect.expand(6.0).contains(pos) {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
        }
    }

    // Timecode
    let frames = state.playhead_frames as i32;
    let fps_int = state.fps.round() as i32;
    let secs = frames / fps_int;
    let remaining_frames = frames % fps_int;
    let mins = secs / 60;
    let secs_display = secs % 60;
    let timecode = format!("{:02}:{:02}:{:02}", mins, secs_display, remaining_frames);

    painter.text(
        Pos2::new(bar_left + btn_size + 12.0, bar_y),
        egui::Align2::LEFT_CENTER,
        &timecode,
        egui::FontId::monospace(Theme::FONT_SM),
        Theme::t1(),
    );

    // Speed badge — color changes for reverse playback
    if (state.speed - 1.0).abs() > 0.01 {
        let speed_text = format!("{:.1}x", state.speed);
        let speed_x = bar_left + btn_size + 90.0;
        let badge_rect = Rect::from_center_size(Pos2::new(speed_x, bar_y), Vec2::new(36.0, 18.0));
        let (badge_bg, badge_color) = if state.speed < 0.0 {
            (Theme::with_alpha(Theme::red(), 38), Theme::red())
        } else {
            (Theme::with_alpha(Theme::accent(), 31), Theme::accent())
        };
        painter.rect_filled(badge_rect, Rounding::same(Theme::RADIUS), badge_bg);
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            &speed_text,
            egui::FontId::proportional(Theme::FONT_XS),
            badge_color,
        );
    }

    // Recording dot (when playing) — pulsing
    if state.playing {
        let pulse = crate::anim::pulse(time, 1.5);
        let dot_x = transport_rect.right() - 60.0;
        painter.circle_filled(
            Pos2::new(dot_x, bar_y),
            3.0,
            Theme::with_alpha(Theme::red(), (pulse * 255.0) as u8),
        );
    }

    // FPS label
    painter.text(
        Pos2::new(transport_rect.right() - 14.0, bar_y),
        egui::Align2::RIGHT_CENTER,
        format!("{}fps", state.fps.round() as i32),
        egui::FontId::monospace(Theme::FONT_XS),
        Theme::t3(),
    );

    // Tool buttons when clip selected — glass pill
    if state.selected_clip.is_some() {
        let tool_icons = ["\u{2702}", "\u{25D1}", "fx", "\u{26A1}", "\u{2726}"];
        let tool_colors = [
            Theme::t2(),
            Theme::t2(),
            Theme::t2(),
            Theme::t2(),
            Theme::purple(),
        ];
        let tools_start_x = rect.center().x - 80.0;

        // Glass pill background
        let pill_rect = Rect::from_min_size(
            Pos2::new(tools_start_x - 10.0, bar_y - 14.0),
            Vec2::new(tool_icons.len() as f32 * 34.0 + 16.0, 28.0),
        );
        painter.rect_filled(
            pill_rect,
            Rounding::same(8.0),
            Color32::from_rgba_premultiplied(5, 5, 9, 90),
        );
        painter.rect_stroke(
            pill_rect,
            Rounding::same(8.0),
            Stroke::new(0.5, Theme::white_06()),
        );

        for (i, (icon, color)) in tool_icons.iter().zip(tool_colors.iter()).enumerate() {
            let tx = tools_start_x + i as f32 * 34.0;
            let tool_rect = Rect::from_center_size(Pos2::new(tx, bar_y), Vec2::new(28.0, 24.0));
            painter.text(
                tool_rect.center(),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(Theme::FONT_XS),
                *color,
            );
        }
    }

    actions
}
