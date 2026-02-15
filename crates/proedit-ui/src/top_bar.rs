//! Custom 40 px top bar with traffic lights, tabs, page navigation and tool buttons.

use crate::theme::Theme;
use egui::{self, Color32, Rounding, Sense, Stroke, Ui, Vec2};

// ── Pages ──────────────────────────────────────────────────────

/// The six editor pages shown in the centre navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Cut,
    Edit,
    Motion,
    Color,
    Audio,
    Deliver,
}

impl Page {
    pub const ALL: [Page; 6] = [
        Page::Cut,
        Page::Edit,
        Page::Motion,
        Page::Color,
        Page::Audio,
        Page::Deliver,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Page::Cut => "Cut",
            Page::Edit => "Edit",
            Page::Motion => "Motion",
            Page::Color => "Color",
            Page::Audio => "Audio",
            Page::Deliver => "Deliver",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            Page::Cut => "\u{2702}",     // ✂
            Page::Edit => "\u{25AC}",    // ▬
            Page::Motion => "\u{25C7}",  // ◇
            Page::Color => "\u{25D0}",   // ◐
            Page::Audio => "\u{266A}",   // ♪
            Page::Deliver => "\u{2197}", // ↗
        }
    }
}

// ── Left tabs ──────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeftTab {
    Media,
    Effects,
}

impl LeftTab {
    pub const ALL: [LeftTab; 2] = [LeftTab::Media, LeftTab::Effects];

    pub fn label(self) -> &'static str {
        match self {
            LeftTab::Media => "Media",
            LeftTab::Effects => "Effects",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            LeftTab::Media => "\u{229E}",   // ⊞
            LeftTab::Effects => "\u{2726}", // ✦
        }
    }
}

// ── Actions returned from top bar ──────────────────────────────

#[derive(Debug, Clone)]
pub enum TopBarAction {
    PageChanged(Page),
    LeftTabChanged(LeftTab),
    ToggleInspector,
    ToggleColorWheels,
    ToggleAudioMixer,
    OpenCommandPalette,
}

// ── State ──────────────────────────────────────────────────────

pub struct TopBarState {
    pub active_page: Page,
    pub left_tab: LeftTab,
    pub inspector_open: bool,
    pub color_wheels_open: bool,
    pub audio_mixer_open: bool,
}

impl Default for TopBarState {
    fn default() -> Self {
        Self {
            active_page: Page::Edit,
            left_tab: LeftTab::Media,
            inspector_open: true,
            color_wheels_open: false,
            audio_mixer_open: false,
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

pub struct TopBarResponse {
    pub actions: Vec<TopBarAction>,
}

/// Show the top bar and return any actions.
pub fn show_top_bar(ui: &mut Ui, state: &mut TopBarState) -> TopBarResponse {
    let mut actions = Vec::new();
    let height = 40.0;

    ui.set_min_height(height);
    ui.set_max_height(height);

    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);

        // ── Traffic lights (macOS chrome — fully functional) ──
        let tl_data: &[(Color32, &str)] = &[
            (Color32::from_rgb(255, 95, 87), "\u{00D7}"),  // × close
            (Color32::from_rgb(255, 189, 46), "\u{2212}"), // − minimize
            (Color32::from_rgb(39, 201, 63), "+"),         // + maximize
        ];
        for (idx, (color, symbol)) in tl_data.iter().enumerate() {
            let (resp, painter) = ui.allocate_painter(Vec2::splat(12.0), Sense::click());
            let center = resp.rect.center();
            painter.circle_filled(center, 6.0, *color);
            painter.circle_stroke(
                center,
                6.0,
                Stroke::new(Theme::STROKE_SUBTLE, Theme::with_alpha(*color, 85)),
            );
            // Show symbol on hover
            if resp.hovered() {
                painter.text(
                    center,
                    egui::Align2::CENTER_CENTER,
                    *symbol,
                    egui::FontId::proportional(9.0),
                    Color32::from_rgba_premultiplied(60, 20, 20, 200),
                );
            }
            // Handle click
            if resp.clicked() {
                match idx {
                    0 => ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close),
                    1 => ui
                        .ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Minimized(true)),
                    2 => {
                        let maximized = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                        ui.ctx()
                            .send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                    }
                    _ => {}
                }
            }
        }

        ui.add_space(Theme::SPACE_SM);

        // ── Logo ───────────────────────────────────────────
        ui.label(
            egui::RichText::new("ProEdit")
                .color(Theme::t1())
                .size(Theme::FONT_MD)
                .strong(),
        );
        ui.label(
            egui::RichText::new("Studio")
                .color(Theme::t3())
                .size(Theme::FONT_MD),
        );

        ui.add_space(Theme::SPACE_MD);

        // ── Left panel tabs ────────────────────────────────
        let tab_frame = egui::Frame::none()
            .fill(Theme::input_bg())
            .rounding(Rounding::same(Theme::RADIUS))
            .inner_margin(egui::Margin::same(2.0));

        tab_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                for tab in LeftTab::ALL {
                    let is_active = state.left_tab == tab;
                    let text_color = if is_active {
                        Theme::accent()
                    } else {
                        Theme::t3()
                    };
                    let bg = if is_active {
                        Theme::accent_subtle()
                    } else {
                        Color32::TRANSPARENT
                    };

                    let btn = egui::Frame::none()
                        .fill(bg)
                        .rounding(Rounding::same(Theme::RADIUS))
                        .inner_margin(egui::Margin::symmetric(Theme::SPACE_SM, 3.0));

                    let resp = btn
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_XS, 0.0);
                                ui.label(
                                    egui::RichText::new(tab.icon())
                                        .size(Theme::FONT_XS)
                                        .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(tab.label())
                                        .size(Theme::FONT_XS)
                                        .color(text_color),
                                );
                            });
                        })
                        .response;

                    if resp.clicked() && !is_active {
                        state.left_tab = tab;
                        actions.push(TopBarAction::LeftTabChanged(tab));
                    }
                }
            });
        });

        // ── Drag zone spacer (enables window movement) ────
        let spacer_w = (ui.available_width() * 0.1).max(10.0);
        let (drag_resp, _) = ui.allocate_painter(Vec2::new(spacer_w, height), Sense::drag());
        if drag_resp.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // ── Page navigation ────────────────────────────────
        let nav_frame = egui::Frame::none()
            .fill(Theme::input_bg())
            .rounding(Rounding::same(Theme::RADIUS))
            .inner_margin(egui::Margin::same(2.0));

        nav_frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                for page in Page::ALL {
                    let is_active = state.active_page == page;
                    let text_color = if is_active {
                        Theme::accent()
                    } else {
                        Theme::t3()
                    };
                    let bg = if is_active {
                        Theme::accent_subtle()
                    } else {
                        Color32::TRANSPARENT
                    };

                    let btn = egui::Frame::none()
                        .fill(bg)
                        .rounding(Rounding::same(Theme::RADIUS))
                        .inner_margin(egui::Margin::symmetric(Theme::SPACE_MD, Theme::SPACE_XS));

                    let resp = btn
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_XS, 0.0);
                                ui.label(
                                    egui::RichText::new(page.icon())
                                        .size(Theme::FONT_XS)
                                        .color(text_color),
                                );
                                ui.label(
                                    egui::RichText::new(page.label())
                                        .size(Theme::FONT_XS)
                                        .color(text_color),
                                );
                            });
                        })
                        .response;

                    if resp.clicked() && !is_active {
                        state.active_page = page;
                        actions.push(TopBarAction::PageChanged(page));
                    }
                }
            });
        });

        // ── Drag zone spacer (enables window movement) ────
        let right_spacer = (ui.available_width() - 160.0).max(10.0);
        let (drag_resp2, _) = ui.allocate_painter(Vec2::new(right_spacer, height), Sense::drag());
        if drag_resp2.drag_started() {
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
        }

        // ── Right tool buttons ─────────────────────────────
        struct ToolBtn {
            icon: &'static str,
            active: bool,
        }

        let tools = [
            ToolBtn {
                icon: "\u{2318}K",
                active: false,
            }, // ⌘K
            ToolBtn {
                icon: "\u{25D0}",
                active: state.color_wheels_open,
            }, // ◐
            ToolBtn {
                icon: "\u{266A}",
                active: state.audio_mixer_open,
            }, // ♪
            ToolBtn {
                icon: "\u{25A4}",
                active: state.inspector_open,
            }, // ▤
        ];

        for (i, tool) in tools.iter().enumerate() {
            let text_color = if tool.active {
                Theme::accent()
            } else {
                Theme::t3()
            };
            let bg = if tool.active {
                Theme::accent_subtle()
            } else {
                Color32::TRANSPARENT
            };

            let btn = egui::Frame::none()
                .fill(bg)
                .rounding(Rounding::same(Theme::RADIUS))
                .inner_margin(egui::Margin::symmetric(6.0, Theme::SPACE_XS));

            let resp = btn
                .show(ui, |ui| {
                    let size = Vec2::new(30.0, 28.0);
                    let (r, _p) = ui.allocate_painter(size, Sense::click());
                    ui.painter().text(
                        r.rect.center(),
                        egui::Align2::CENTER_CENTER,
                        tool.icon,
                        egui::FontId::proportional(Theme::FONT_XS),
                        text_color,
                    );
                })
                .response;

            if resp.clicked() {
                match i {
                    0 => actions.push(TopBarAction::OpenCommandPalette),
                    1 => {
                        state.color_wheels_open = !state.color_wheels_open;
                        actions.push(TopBarAction::ToggleColorWheels);
                    }
                    2 => {
                        state.audio_mixer_open = !state.audio_mixer_open;
                        actions.push(TopBarAction::ToggleAudioMixer);
                    }
                    3 => {
                        state.inspector_open = !state.inspector_open;
                        actions.push(TopBarAction::ToggleInspector);
                    }
                    _ => {}
                }
            }
        }
    });

    TopBarResponse { actions }
}
