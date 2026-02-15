//! Viewport / viewer with animated gradient background and transport overlay.

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

    // ── Animated gradient background ───────────────────────
    let ph = state.playhead_frames;
    let _angle = 135.0 + ph * 0.2;

    painter.rect_filled(rect, 0.0, Theme::bg());

    // Subtle mid-tone overlay in the center region
    let mid_rect = rect.shrink2(Vec2::new(rect.width() * 0.2, rect.height() * 0.2));
    painter.rect_filled(
        mid_rect,
        0.0,
        Color32::from_rgba_premultiplied(20, 26, 38, 40),
    );

    // Floating radial highlight
    let cx = rect.center().x + (time * 0.3).sin() as f32 * rect.width() * 0.15;
    let cy = rect.center().y + (time * 0.2).cos() as f32 * rect.height() * 0.15;
    let glow_radius = rect.width().min(rect.height()) * 0.4;
    // Approximate radial glow with a translucent circle
    painter.circle_filled(
        Pos2::new(cx, cy),
        glow_radius,
        Theme::with_alpha(Theme::accent(), 8),
    );

    // ── Safe area guides ───────────────────────────────────
    let inset = rect.shrink2(Vec2::new(rect.width() * 0.1, rect.height() * 0.1));
    painter.rect_stroke(
        inset,
        0.0,
        Stroke::new(Theme::STROKE_EMPHASIS, Theme::white_04()),
    );

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

    // Gradient backdrop
    // Top transparent → bottom dark
    painter.rect_filled(transport_rect, 0.0, Theme::scrim());

    let bar_y = transport_rect.center().y;
    let bar_left = transport_rect.left() + 14.0;

    // Play/pause button
    let btn_size = 30.0;
    let btn_rect = Rect::from_center_size(
        Pos2::new(bar_left + btn_size * 0.5, bar_y),
        Vec2::splat(btn_size),
    );
    let (btn_bg, btn_color, btn_icon) = if state.playing {
        (Theme::red(), Theme::t1(), "\u{23F8}") // ⏸
    } else {
        (Theme::accent(), Theme::t1(), "\u{25B6}") // ▶
    };
    painter.rect_filled(btn_rect, Rounding::same(Theme::RADIUS), btn_bg);
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

    // Speed badge
    if (state.speed - 1.0).abs() > 0.01 {
        let speed_text = format!("{:.1}x", state.speed);
        let speed_x = bar_left + btn_size + 90.0;
        let badge_rect = Rect::from_center_size(Pos2::new(speed_x, bar_y), Vec2::new(36.0, 18.0));
        painter.rect_filled(
            badge_rect,
            Rounding::same(Theme::RADIUS),
            Theme::accent_subtle(),
        );
        painter.text(
            badge_rect.center(),
            egui::Align2::CENTER_CENTER,
            &speed_text,
            egui::FontId::proportional(Theme::FONT_XS),
            Theme::accent(),
        );
    }

    // Recording dot (when playing)
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

    // Tool buttons when clip selected
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
        for (i, (icon, color)) in tool_icons.iter().zip(tool_colors.iter()).enumerate() {
            let tx = tools_start_x + i as f32 * 34.0;
            let tool_rect = Rect::from_center_size(Pos2::new(tx, bar_y), Vec2::new(28.0, 24.0));
            painter.rect_filled(tool_rect, Rounding::same(Theme::RADIUS), Theme::white_06());
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
