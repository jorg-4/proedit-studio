//! Liquid Glass dark theme — iOS 26 inspired color palette and styling.

use egui::{Color32, Rounding, Stroke, Vec2};

/// Central theme with every color matching the React reference.
pub struct Theme;

impl Theme {
    // ── Backgrounds ──────────────────────────────────────────────
    pub const fn bg() -> Color32 { Color32::from_rgb(5, 5, 8) }
    pub const fn bg1() -> Color32 { Color32::from_rgb(11, 11, 18) }
    pub const fn bg2() -> Color32 { Color32::from_rgb(18, 18, 30) }
    pub const fn bg3() -> Color32 { Color32::from_rgb(26, 26, 43) }
    pub const fn bg4() -> Color32 { Color32::from_rgb(34, 34, 58) }

    // ── Text ─────────────────────────────────────────────────────
    pub const fn t1() -> Color32 { Color32::from_rgb(244, 244, 252) }
    pub const fn t2() -> Color32 { Color32::from_rgb(152, 152, 180) }
    pub const fn t3() -> Color32 { Color32::from_rgb(94, 94, 120) }
    pub const fn t4() -> Color32 { Color32::from_rgb(62, 62, 88) }

    // ── Accent ───────────────────────────────────────────────────
    pub const fn accent() -> Color32 { Color32::from_rgb(78, 133, 255) }
    pub const fn accent_subtle() -> Color32 { Color32::from_rgba_premultiplied(11, 19, 36, 36) }
    pub const fn accent_glow() -> Color32 { Color32::from_rgba_premultiplied(22, 37, 71, 71) }

    // ── Semantic colors ──────────────────────────────────────────
    pub const fn red() -> Color32 { Color32::from_rgb(255, 88, 85) }
    pub const fn green() -> Color32 { Color32::from_rgb(48, 213, 160) }
    pub const fn amber() -> Color32 { Color32::from_rgb(255, 184, 48) }
    pub const fn purple() -> Color32 { Color32::from_rgb(167, 139, 250) }
    pub const fn pink() -> Color32 { Color32::from_rgb(244, 114, 182) }
    pub const fn cyan() -> Color32 { Color32::from_rgb(34, 211, 238) }

    // ── Helpers ──────────────────────────────────────────────────

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

    /// Apply the Liquid Glass theme to an egui context.
    pub fn apply(ctx: &egui::Context) {
        let mut visuals = egui::Visuals::dark();

        visuals.panel_fill = Self::bg1();
        visuals.window_fill = Self::bg2();
        visuals.extreme_bg_color = Self::bg();
        visuals.faint_bg_color = Self::bg2();

        // Widgets
        visuals.widgets.noninteractive.bg_fill = Self::bg2();
        visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, Self::t3());
        visuals.widgets.noninteractive.bg_stroke = Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 10));
        visuals.widgets.noninteractive.rounding = Rounding::same(8.0);

        visuals.widgets.inactive.bg_fill = Self::bg3();
        visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, Self::t2());
        visuals.widgets.inactive.bg_stroke = Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 10));
        visuals.widgets.inactive.rounding = Rounding::same(8.0);

        visuals.widgets.hovered.bg_fill = Self::bg4();
        visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, Self::t1());
        visuals.widgets.hovered.bg_stroke = Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 18));
        visuals.widgets.hovered.rounding = Rounding::same(8.0);

        visuals.widgets.active.bg_fill = Self::accent_subtle();
        visuals.widgets.active.fg_stroke = Stroke::new(1.0, Self::accent());
        visuals.widgets.active.bg_stroke = Stroke::new(1.0, Self::accent());
        visuals.widgets.active.rounding = Rounding::same(8.0);

        visuals.widgets.open.bg_fill = Self::bg3();
        visuals.widgets.open.fg_stroke = Stroke::new(1.0, Self::t1());
        visuals.widgets.open.rounding = Rounding::same(8.0);

        visuals.selection.bg_fill = Self::accent_subtle();
        visuals.selection.stroke = Stroke::new(1.0, Self::accent());

        visuals.window_rounding = Rounding::same(12.0);
        visuals.window_stroke = Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 10));
        visuals.window_shadow = egui::epaint::Shadow {
            offset: Vec2::new(0.0, 4.0),
            blur: 20.0,
            spread: 0.0,
            color: Color32::from_rgba_premultiplied(0, 0, 0, 80),
        };

        visuals.resize_corner_size = 0.0;

        ctx.set_visuals(visuals);
    }

    /// Glass-effect frame for floating panels.
    pub fn glass_frame() -> egui::Frame {
        egui::Frame::none()
            .fill(Color32::from_rgba_premultiplied(8, 8, 14, 148))
            .stroke(Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 18)))
            .rounding(Rounding::same(16.0))
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
            .stroke(Stroke::new(0.5, Self::with_alpha(Color32::WHITE, 6)))
            .inner_margin(egui::Margin::symmetric(12.0, 0.0))
    }
}
