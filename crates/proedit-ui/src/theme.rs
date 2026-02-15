//! Liquid Glass dark theme — iOS 26 inspired color palette and styling.

use egui::{Color32, Rounding, Stroke, Vec2};

/// Central theme with every color matching the React reference.
pub struct Theme;

impl Theme {
    // ── Typography ─────────────────────────────────────────────
    pub const FONT_XS: f32 = 11.0; // meta, timestamps, badges
    pub const FONT_SM: f32 = 13.0; // body, labels, buttons
    pub const FONT_MD: f32 = 15.0; // section headers
    pub const FONT_LG: f32 = 18.0; // panel titles

    // ── Spacing (4px base) ─────────────────────────────────────
    pub const SPACE_XS: f32 = 4.0; // internal padding
    pub const SPACE_SM: f32 = 8.0; // related items
    pub const SPACE_MD: f32 = 16.0; // section separation
    pub const SPACE_LG: f32 = 24.0; // panel padding
    pub const SPACE_XL: f32 = 32.0; // major breaks

    // ── Border radius ──────────────────────────────────────────
    pub const RADIUS: f32 = 6.0; // all interactive elements
    pub const RADIUS_LG: f32 = 12.0; // floating panels, modals

    // ── Stroke widths ──────────────────────────────────────────
    pub const STROKE_SUBTLE: f32 = 0.5;
    pub const STROKE_EMPHASIS: f32 = 1.0;
    pub const DIVIDER_WIDTH: f32 = 1.0;

    // ── Backgrounds ────────────────────────────────────────────
    pub const fn bg() -> Color32 {
        Color32::from_rgb(18, 18, 22)
    }
    pub const fn bg1() -> Color32 {
        Color32::from_rgb(28, 28, 34)
    }
    pub const fn bg2() -> Color32 {
        Color32::from_rgb(35, 35, 42)
    }
    pub const fn bg3() -> Color32 {
        Color32::from_rgb(45, 45, 55)
    }
    pub const fn bg4() -> Color32 {
        Color32::from_rgb(55, 55, 68)
    }
    /// Text inputs, search fields.
    pub const fn input_bg() -> Color32 {
        Color32::from_rgb(22, 22, 28)
    }

    // ── Text (opacity-based white) ─────────────────────────────
    pub const fn t1() -> Color32 {
        Color32::from_rgba_premultiplied(235, 235, 235, 235)
    }
    pub const fn t2() -> Color32 {
        Color32::from_rgba_premultiplied(153, 153, 153, 153)
    }
    pub const fn t3() -> Color32 {
        Color32::from_rgba_premultiplied(89, 89, 89, 89)
    }
    pub const fn t4() -> Color32 {
        Color32::from_rgba_premultiplied(38, 38, 38, 38)
    }

    // ── Accent ─────────────────────────────────────────────────
    pub const fn accent() -> Color32 {
        Color32::from_rgb(86, 130, 255)
    }
    /// Accent @ 8% — subtle active fill.
    pub const fn accent_subtle() -> Color32 {
        Color32::from_rgba_premultiplied(7, 10, 20, 20)
    }
    /// Accent @ 15% — hovered widget stroke.
    pub const fn accent_hover() -> Color32 {
        Color32::from_rgba_premultiplied(13, 20, 38, 38)
    }

    // ── White-alpha overlay helpers ────────────────────────────
    pub const fn white_02() -> Color32 {
        Color32::from_rgba_premultiplied(5, 5, 5, 5)
    }
    pub const fn white_04() -> Color32 {
        Color32::from_rgba_premultiplied(10, 10, 10, 10)
    }
    pub const fn white_06() -> Color32 {
        Color32::from_rgba_premultiplied(15, 15, 15, 15)
    }
    pub const fn white_08() -> Color32 {
        Color32::from_rgba_premultiplied(20, 20, 20, 20)
    }
    pub const fn white_10() -> Color32 {
        Color32::from_rgba_premultiplied(26, 26, 26, 26)
    }
    pub const fn white_15() -> Color32 {
        Color32::from_rgba_premultiplied(38, 38, 38, 38)
    }
    pub const fn white_25() -> Color32 {
        Color32::from_rgba_premultiplied(64, 64, 64, 64)
    }

    // ── Other helpers ──────────────────────────────────────────
    /// Backdrop overlay.
    pub const fn scrim() -> Color32 {
        Color32::from_rgba_premultiplied(0, 0, 0, 102)
    }
    /// Section separators — white @ 6%.
    pub const fn divider() -> Color32 {
        Color32::from_rgba_premultiplied(15, 15, 15, 15)
    }

    // ── Semantic colors ────────────────────────────────────────
    pub const fn red() -> Color32 {
        Color32::from_rgb(255, 88, 85)
    }
    pub const fn green() -> Color32 {
        Color32::from_rgb(48, 213, 160)
    }
    pub const fn amber() -> Color32 {
        Color32::from_rgb(255, 184, 48)
    }
    pub const fn purple() -> Color32 {
        Color32::from_rgb(167, 139, 250)
    }
    pub const fn pink() -> Color32 {
        Color32::from_rgb(244, 114, 182)
    }
    pub const fn cyan() -> Color32 {
        Color32::from_rgb(34, 211, 238)
    }

    // ── Color helpers ──────────────────────────────────────────

    /// Return a color with replaced alpha.
    pub const fn with_alpha(c: Color32, a: u8) -> Color32 {
        Color32::from_rgba_premultiplied(
            (c.r() as u16 * a as u16 / 255) as u8,
            (c.g() as u16 * a as u16 / 255) as u8,
            (c.b() as u16 * a as u16 / 255) as u8,
            a,
        )
    }

    /// Blend a color toward another by `t` (0..1).
    pub fn lerp(a: Color32, b: Color32, t: f32) -> Color32 {
        let t = t.clamp(0.0, 1.0);
        let inv = 1.0 - t;
        Color32::from_rgba_premultiplied(
            (a.r() as f32 * inv + b.r() as f32 * t) as u8,
            (a.g() as f32 * inv + b.g() as f32 * t) as u8,
            (a.b() as f32 * inv + b.b() as f32 * t) as u8,
            (a.a() as f32 * inv + b.a() as f32 * t) as u8,
        )
    }

    // ── Frame builders ─────────────────────────────────────────

    /// Standard side panel frame.
    pub fn panel_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Self::bg1())
            .inner_margin(egui::Margin::same(Self::SPACE_SM))
    }

    /// Text input background with subtle border.
    pub fn input_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Self::input_bg())
            .stroke(Stroke::new(Self::STROKE_SUBTLE, Self::white_10()))
            .rounding(Rounding::same(Self::RADIUS))
            .inner_margin(egui::Margin::symmetric(Self::SPACE_SM, 5.0))
    }

    /// Draw a reusable 1px horizontal divider.
    pub fn draw_separator(ui: &mut egui::Ui) {
        let width = ui.available_width();
        let (resp, painter) =
            ui.allocate_painter(Vec2::new(width, Self::DIVIDER_WIDTH), egui::Sense::hover());
        painter.rect_filled(resp.rect, 0.0, Self::divider());
    }

    // ── Theme application ──────────────────────────────────────

    /// Apply the Liquid Glass theme to an egui context.
    pub fn apply(ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        let visuals = &mut style.visuals;
        *visuals = egui::Visuals::dark();

        visuals.panel_fill = Self::bg1();
        visuals.window_fill = Self::bg2();
        visuals.extreme_bg_color = Self::bg();
        visuals.faint_bg_color = Self::bg2();

        // Widgets
        visuals.widgets.noninteractive.bg_fill = Self::bg2();
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::t3());
        visuals.widgets.noninteractive.bg_stroke =
            Stroke::new(Self::STROKE_SUBTLE, Self::white_04());
        visuals.widgets.noninteractive.rounding = Rounding::same(Self::RADIUS);

        visuals.widgets.inactive.bg_fill = Self::bg3();
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::t2());
        visuals.widgets.inactive.bg_stroke = Stroke::new(Self::STROKE_SUBTLE, Self::white_04());
        visuals.widgets.inactive.rounding = Rounding::same(Self::RADIUS);

        visuals.widgets.hovered.bg_fill = Self::bg4();
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::t1());
        visuals.widgets.hovered.bg_stroke = Stroke::new(Self::STROKE_SUBTLE, Self::accent_hover());
        visuals.widgets.hovered.rounding = Rounding::same(Self::RADIUS);

        visuals.widgets.active.bg_fill = Self::accent_subtle();
        visuals.widgets.active.fg_stroke = Stroke::new(Self::STROKE_EMPHASIS, Self::accent());
        visuals.widgets.active.bg_stroke = Stroke::new(Self::STROKE_EMPHASIS, Self::accent());
        visuals.widgets.active.rounding = Rounding::same(Self::RADIUS);

        visuals.widgets.open.bg_fill = Self::bg3();
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, Self::t1());
        visuals.widgets.open.rounding = Rounding::same(Self::RADIUS);

        visuals.selection.bg_fill = Self::accent_subtle();
        visuals.selection.stroke = Stroke::new(1.0, Self::accent());

        visuals.window_rounding = Rounding::same(Self::RADIUS_LG);
        visuals.window_stroke = Stroke::new(Self::STROKE_SUBTLE, Self::white_04());
        visuals.window_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 4.0),
            blur: 20.0,
            spread: 0.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 80),
        };

        visuals.resize_corner_size = 8.0;

        // Tooltip delay
        style.interaction.tooltip_delay = 0.4;

        ctx.set_style(style);
    }

    /// Glass-effect frame for floating panels.
    pub fn glass_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Color32::from_rgba_premultiplied(8, 8, 14, 148))
            .stroke(Stroke::new(Self::STROKE_SUBTLE, Self::white_08()))
            .rounding(Rounding::same(Self::RADIUS_LG))
            .inner_margin(egui::Margin::same(14.0))
            .shadow(egui::epaint::Shadow {
                offset: Vec2::new(0.0, 8.0),
                blur: 32.0,
                spread: 0.0,
                color: Color32::from_rgba_premultiplied(0, 0, 0, 100),
            })
    }

    /// Frame for top bar.
    pub fn top_bar_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Self::bg1())
            .stroke(Stroke::new(Self::STROKE_SUBTLE, Self::white_06()))
            .inner_margin(egui::Margin::symmetric(12.0, 0.0))
    }
}
