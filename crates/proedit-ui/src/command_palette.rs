//! Modal command palette overlay (⌘K).

use crate::theme::Theme;
use egui::{self, Color32, Rounding, Sense, Stroke, Vec2};

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
    painter.rect_filled(screen, 0.0, Color32::from_rgba_premultiplied(0, 0, 0, 102));

    // Command palette window
    let palette_width = 460.0_f32.min(screen.width() - 40.0);
    let max_height = 400.0_f32.min(screen.height() - 100.0);

    egui::Area::new(egui::Id::new("command_palette"))
        .order(egui::Order::Foreground)
        .anchor(
            egui::Align2::CENTER_TOP,
            Vec2::new(0.0, screen.height() * 0.15),
        )
        .show(ctx, |ui| {
            Theme::glass_frame()
                .rounding(Rounding::same(16.0))
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
                            egui::FontId::proportional(12.0),
                            Theme::accent(),
                        );

                        // Search input
                        let resp = ui.add(
                            egui::TextEdit::singleline(&mut state.query)
                                .hint_text("Search commands\u{2026}")
                                .desired_width(palette_width - 60.0)
                                .font(egui::FontId::proportional(14.0))
                                .frame(false),
                        );
                        resp.request_focus();
                    });

                    ui.add_space(4.0);
                    // Separator
                    let sep_rect = ui.allocate_space(Vec2::new(ui.available_width(), 0.5));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            sep_rect.1.min,
                            Vec2::new(sep_rect.1.width(), 0.5),
                        ),
                        0.0,
                        Theme::with_alpha(Color32::WHITE, 10),
                    );
                    ui.add_space(4.0);

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

                                let frame = egui::Frame::none()
                                    .fill(bg)
                                    .rounding(Rounding::same(9.0))
                                    .inner_margin(egui::Margin::symmetric(10.0, 7.0));

                                let resp = frame
                                    .show(ui, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.spacing_mut().item_spacing = Vec2::new(9.0, 0.0);

                                            // Icon box
                                            let icon_bg = if is_ai {
                                                Theme::with_alpha(Theme::purple(), 30)
                                            } else {
                                                Color32::from_rgba_premultiplied(2, 2, 2, 8)
                                            };
                                            let icon_color =
                                                if is_ai { Theme::purple() } else { Theme::t2() };

                                            let (icon_resp, icon_painter) = ui.allocate_painter(
                                                Vec2::splat(24.0),
                                                Sense::hover(),
                                            );
                                            icon_painter.rect_filled(
                                                icon_resp.rect,
                                                Rounding::same(6.0),
                                                icon_bg,
                                            );
                                            icon_painter.text(
                                                icon_resp.rect.center(),
                                                egui::Align2::CENTER_CENTER,
                                                cmd.icon,
                                                egui::FontId::proportional(12.0),
                                                icon_color,
                                            );

                                            // Name
                                            ui.label(
                                                egui::RichText::new(cmd.name)
                                                    .size(12.5)
                                                    .color(Theme::t1()),
                                            );

                                            // AI badge
                                            if is_ai {
                                                let badge_frame = egui::Frame::none()
                                                    .fill(Theme::with_alpha(Theme::purple(), 30))
                                                    .rounding(Rounding::same(8.0))
                                                    .inner_margin(egui::Margin::symmetric(
                                                        6.0, 2.0,
                                                    ));
                                                badge_frame.show(ui, |ui| {
                                                    ui.label(
                                                        egui::RichText::new("AI")
                                                            .size(8.0)
                                                            .color(Theme::purple())
                                                            .strong(),
                                                    );
                                                });
                                            }

                                            // Spacer
                                            ui.with_layout(
                                                egui::Layout::right_to_left(egui::Align::Center),
                                                |ui| {
                                                    // Shortcut
                                                    if cmd.shortcut != "\u{2014}" {
                                                        let key_frame = egui::Frame::none()
                                                            .fill(Color32::from_rgba_premultiplied(
                                                                2, 2, 2, 10,
                                                            ))
                                                            .stroke(Stroke::new(
                                                                0.5,
                                                                Theme::with_alpha(
                                                                    Color32::WHITE,
                                                                    10,
                                                                ),
                                                            ))
                                                            .rounding(Rounding::same(4.0))
                                                            .inner_margin(egui::Margin::symmetric(
                                                                5.0, 2.0,
                                                            ));
                                                        key_frame.show(ui, |ui| {
                                                            ui.label(
                                                                egui::RichText::new(cmd.shortcut)
                                                                    .size(9.5)
                                                                    .color(Theme::t3())
                                                                    .family(
                                                                        egui::FontFamily::Monospace,
                                                                    ),
                                                            );
                                                        });
                                                    }
                                                },
                                            );
                                        });
                                    })
                                    .response;

                                if resp.clicked() {
                                    state.executed = Some(cmd.name);
                                    state.open = false;
                                }
                                if resp.hovered() {
                                    state.hovered_index = i;
                                }
                            }
                        });

                    ui.add_space(4.0);
                    // Footer separator
                    let sep_rect2 = ui.allocate_space(Vec2::new(ui.available_width(), 0.5));
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            sep_rect2.1.min,
                            Vec2::new(sep_rect2.1.width(), 0.5),
                        ),
                        0.0,
                        Theme::with_alpha(Color32::WHITE, 10),
                    );
                    ui.add_space(4.0);

                    // Footer hints
                    ui.horizontal(|ui| {
                        ui.with_layout(
                            egui::Layout::centered_and_justified(egui::Direction::LeftToRight),
                            |ui| {
                                ui.label(
                                    egui::RichText::new(
                                        "\u{2191}\u{2193} Navigate    \u{21B5} Run    esc Close",
                                    )
                                    .size(9.0)
                                    .color(Theme::t4()),
                                );
                            },
                        );
                    });
                });
        });

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
