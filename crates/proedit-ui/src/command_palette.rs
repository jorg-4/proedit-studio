//! Modal command palette overlay (⌘K).

use crate::theme::Theme;
use egui::{self, Color32, Pos2, Rounding, Sense, Stroke, Vec2};

// ── Command data ───────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CommandCategory {
    File,
    Edit,
    Ai,
    View,
}

#[derive(Debug, Clone)]
pub struct Command {
    pub name: &'static str,
    pub shortcut: &'static str,
    pub category: CommandCategory,
    pub icon: &'static str,
}

/// Commands available in the palette — only entries with working implementations.
pub const COMMANDS: &[Command] = &[
    Command {
        name: "Import Media",
        shortcut: "\u{2318}I",
        category: CommandCategory::File,
        icon: "\u{2193}",
    },
    Command {
        name: "Export Project",
        shortcut: "\u{2318}\u{21E7}E",
        category: CommandCategory::File,
        icon: "\u{2197}",
    },
    Command {
        name: "Undo",
        shortcut: "\u{2318}Z",
        category: CommandCategory::Edit,
        icon: "\u{21BA}",
    },
    Command {
        name: "Razor at Playhead",
        shortcut: "C",
        category: CommandCategory::Edit,
        icon: "\u{2702}",
    },
    Command {
        name: "Ripple Delete",
        shortcut: "\u{232B}",
        category: CommandCategory::Edit,
        icon: "\u{2326}",
    },
    Command {
        name: "Add Marker",
        shortcut: "M",
        category: CommandCategory::Edit,
        icon: "\u{25C6}",
    },
    Command {
        name: "Speed Ramp",
        shortcut: "R",
        category: CommandCategory::Edit,
        icon: "\u{26A1}",
    },
    Command {
        name: "Toggle Audio Mixer",
        shortcut: "\u{2318}M",
        category: CommandCategory::View,
        icon: "\u{266A}",
    },
];

// ── State ──────────────────────────────────────────────────────

#[derive(Default)]
pub struct CommandPaletteState {
    pub open: bool,
    pub query: String,
    pub hovered_index: usize,
    /// Set to the name of any command that was executed this frame.
    pub executed: Option<&'static str>,
}

impl CommandPaletteState {
    pub fn toggle(&mut self) {
        self.open = !self.open;
        if self.open {
            self.query.clear();
            self.hovered_index = 0;
        }
    }
}

// ── Rendering ──────────────────────────────────────────────────

/// Show the command palette overlay. Call this after all other panels.
pub fn show_command_palette(ctx: &egui::Context, state: &mut CommandPaletteState) {
    if !state.open {
        return;
    }

    state.executed = None;

    // Filter commands
    let query_lower = state.query.to_ascii_lowercase();
    let filtered: Vec<&Command> = COMMANDS
        .iter()
        .filter(|c| query_lower.is_empty() || c.name.to_ascii_lowercase().contains(&query_lower))
        .collect();

    if state.hovered_index >= filtered.len() {
        state.hovered_index = filtered.len().saturating_sub(1);
    }

    // Full-screen backdrop
    let screen = ctx.screen_rect();
    let backdrop_layer = egui::LayerId::new(egui::Order::Foreground, egui::Id::new("cmd_backdrop"));
    let painter = ctx.layer_painter(backdrop_layer);
    painter.rect_filled(screen, 0.0, Theme::scrim());

    // Command palette window
    let palette_width = 460.0_f32.min(screen.width() - 40.0);
    let max_height = 400.0_f32.min(screen.height() - 100.0);

    let area_resp = egui::Area::new(egui::Id::new("command_palette"))
        .order(egui::Order::Foreground)
        .anchor(
            egui::Align2::CENTER_TOP,
            Vec2::new(0.0, screen.height() * 0.15),
        )
        .show(ctx, |ui| {
            Theme::glass_frame()
                .rounding(Rounding::same(Theme::RADIUS_LG))
                .show(ui, |ui| {
                    ui.set_width(palette_width);
                    ui.set_max_height(max_height);

                    // ── Search area ────────────────────────
                    ui.horizontal(|ui| {
                        // Command icon
                        let icon_rect = ui.allocate_space(Vec2::splat(20.0));
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(icon_rect.1.min, Vec2::splat(20.0)),
                            Rounding::same(5.0),
                            Theme::accent_subtle(),
                        );
                        ui.painter().text(
                            icon_rect.1.center(),
                            egui::Align2::CENTER_CENTER,
                            "\u{2318}",
                            egui::FontId::proportional(Theme::FONT_SM),
                            Theme::accent(),
                        );

                        // Search input
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut state.query)
                                .hint_text("Search commands\u{2026}")
                                .desired_width(palette_width - 60.0)
                                .font(egui::FontId::proportional(Theme::FONT_MD))
                                .frame(false),
                        );
                        resp.request_focus();
                    });

                    ui.add_space(Theme::SPACE_XS);
                    Theme::draw_separator(ui);
                    ui.add_space(Theme::SPACE_XS);

                    // ── Command list ───────────────────────
                    egui::ScrollArea::vertical()
                        .max_height(max_height - 90.0)
                        .show(ui, |ui| {
                            for (i, cmd) in filtered.iter().enumerate() {
                                let is_hovered = i == state.hovered_index;
                                let is_ai = cmd.category == CommandCategory::Ai;

                                let bg = if is_hovered {
                                    Theme::accent_subtle()
                                } else {
                                    Color32::TRANSPARENT
                                };

                                // Use allocate_ui with click sense for reliable hit detection
                                let row_width = ui.available_width();
                                let (row_rect, row_resp) = ui.allocate_exact_size(
                                    Vec2::new(row_width, 38.0),
                                    Sense::click(),
                                );

                                // Draw background
                                ui.painter().rect_filled(
                                    row_rect,
                                    Rounding::same(Theme::RADIUS),
                                    bg,
                                );

                                // Draw content inside the allocated rect
                                let mut child_ui = ui.new_child(
                                    egui::UiBuilder::new()
                                        .max_rect(row_rect.shrink2(Vec2::new(10.0, 7.0)))
                                        .layout(egui::Layout::left_to_right(egui::Align::Center)),
                                );
                                child_ui.spacing_mut().item_spacing = Vec2::new(9.0, 0.0);

                                // Icon box
                                let icon_bg = if is_ai {
                                    Theme::with_alpha(Theme::purple(), 30)
                                } else {
                                    Theme::input_bg()
                                };
                                let icon_color = if is_ai { Theme::purple() } else { Theme::t2() };

                                let (icon_resp, icon_painter) =
                                    child_ui.allocate_painter(Vec2::splat(24.0), Sense::hover());
                                icon_painter.rect_filled(
                                    icon_resp.rect,
                                    Rounding::same(Theme::RADIUS),
                                    icon_bg,
                                );
                                icon_painter.text(
                                    icon_resp.rect.center(),
                                    egui::Align2::CENTER_CENTER,
                                    cmd.icon,
                                    egui::FontId::proportional(Theme::FONT_SM),
                                    icon_color,
                                );

                                // Name
                                child_ui.label(
                                    egui::RichText::new(cmd.name)
                                        .size(Theme::FONT_SM)
                                        .color(Theme::t1()),
                                );

                                // AI badge
                                if is_ai {
                                    let badge_frame = egui::Frame::none()
                                        .fill(Theme::with_alpha(Theme::purple(), 30))
                                        .rounding(Rounding::same(Theme::RADIUS))
                                        .inner_margin(egui::Margin::symmetric(6.0, 2.0));
                                    badge_frame.show(&mut child_ui, |ui| {
                                        ui.label(
                                            egui::RichText::new("AI")
                                                .size(Theme::FONT_XS)
                                                .color(Theme::purple())
                                                .strong(),
                                        );
                                    });
                                }

                                // Shortcut (right-aligned) — draw directly with painter
                                if cmd.shortcut != "\u{2014}" {
                                    let shortcut_text = cmd.shortcut;
                                    let font = egui::FontId::monospace(Theme::FONT_XS);
                                    let text_galley = ui.painter().layout_no_wrap(
                                        shortcut_text.to_string(),
                                        font,
                                        Theme::t3(),
                                    );
                                    let text_w = text_galley.rect.width();
                                    let key_rect = egui::Rect::from_min_size(
                                        Pos2::new(
                                            row_rect.right() - 10.0 - text_w - 10.0,
                                            row_rect.center().y - 10.0,
                                        ),
                                        Vec2::new(text_w + 10.0, 20.0),
                                    );
                                    ui.painter().rect_filled(
                                        key_rect,
                                        Rounding::same(4.0),
                                        Theme::input_bg(),
                                    );
                                    ui.painter().rect_stroke(
                                        key_rect,
                                        Rounding::same(4.0),
                                        Stroke::new(Theme::STROKE_SUBTLE, Theme::white_10()),
                                    );
                                    ui.painter().text(
                                        key_rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        shortcut_text,
                                        egui::FontId::monospace(Theme::FONT_XS),
                                        Theme::t3(),
                                    );
                                }

                                if row_resp.clicked() {
                                    state.executed = Some(cmd.name);
                                    state.open = false;
                                }
                                if row_resp.hovered() {
                                    state.hovered_index = i;
                                    ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
                                }
                            }
                        });

                    ui.add_space(Theme::SPACE_XS);
                    Theme::draw_separator(ui);
                    ui.add_space(Theme::SPACE_XS);

                    // Footer hints
                    ui.horizontal(|ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "\u{2191}\u{2193} Navigate    \u{21B5} Run    esc Close",
                                    )
                                    .size(Theme::FONT_XS)
                                    .color(Theme::t4()),
                                );
                            },
                        );
                    });
                });
        });

    // ── Click outside to dismiss ────────────────────────────────
    let palette_rect = area_resp.response.rect;
    if ctx.input(|i| i.pointer.any_pressed()) {
        if let Some(pos) = ctx.input(|i| i.pointer.latest_pos()) {
            if !palette_rect.contains(pos) {
                state.open = false;
            }
        }
    }

    // ── Keyboard navigation ────────────────────────────────────
    ctx.input(|inp| {
        if inp.key_pressed(egui::Key::Escape) {
            state.open = false;
        }
        if inp.key_pressed(egui::Key::ArrowDown) && state.hovered_index + 1 < filtered.len() {
            state.hovered_index += 1;
        }
        if inp.key_pressed(egui::Key::ArrowUp) {
            state.hovered_index = state.hovered_index.saturating_sub(1);
        }
        if inp.key_pressed(egui::Key::Enter) && !filtered.is_empty() {
            state.executed = Some(filtered[state.hovered_index].name);
            state.open = false;
        }
    });
}
