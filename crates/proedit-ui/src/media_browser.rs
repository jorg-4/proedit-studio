//! Media browser panel with search, filter chips and media items.

use crate::theme::Theme;
use egui::{self, Color32, Rounding, Stroke, Vec2};

// ── Media item data ────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MediaKind {
    Video,
    Audio,
    Image,
    Gfx,
}

impl MediaKind {
    pub fn icon(self) -> &'static str {
        match self {
            MediaKind::Video => "\u{25B6}", // ▶
            MediaKind::Audio => "\u{266A}", // ♪
            MediaKind::Image => "\u{25FB}", // ◻
            MediaKind::Gfx => "\u{25C7}",   // ◇
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            MediaKind::Video => "video",
            MediaKind::Audio => "audio",
            MediaKind::Image => "image",
            MediaKind::Gfx => "gfx",
        }
    }
}

#[derive(Debug, Clone)]
pub struct MediaItem {
    pub name: String,
    pub kind: MediaKind,
    pub duration: String,
    pub size: String,
    pub color: Color32,
}

const FILTERS: &[&str] = &["all", "video", "audio", "image", "gfx"];

// ── State ──────────────────────────────────────────────────────

#[derive(Default)]
pub struct MediaBrowserState {
    pub search_query: String,
    pub active_filter: usize,
    pub items: Vec<MediaItem>,
}

// ── Rendering ──────────────────────────────────────────────────

/// Actions emitted by the media browser.
#[derive(Debug)]
pub enum MediaBrowserAction {
    ImportMedia,
}

pub fn show_media_browser(
    ui: &mut egui::Ui,
    state: &mut MediaBrowserState,
) -> Vec<MediaBrowserAction> {
    let mut actions = Vec::new();
    show_media_browser_inner(ui, state, &mut actions);
    actions
}

fn show_media_browser_inner(
    ui: &mut egui::Ui,
    state: &mut MediaBrowserState,
    actions: &mut Vec<MediaBrowserAction>,
) {
    ui.spacing_mut().item_spacing = Vec2::new(0.0, 6.0);

    // ── Search bar ─────────────────────────────────────────
    Theme::input_frame().show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing = Vec2::new(6.0, 0.0);
            ui.label(
                egui::RichText::new("\u{1F50D}")
                    .size(Theme::FONT_XS)
                    .color(Theme::t4()),
            );
            let resp = ui.add(
                egui::TextEdit::singleline(&mut state.search_query)
                    .hint_text("Search media\u{2026}")
                    .desired_width(ui.available_width() - 20.0)
                    .font(egui::FontId::proportional(Theme::FONT_XS))
                    .frame(false),
            );
            if !state.search_query.is_empty() && ui.small_button("\u{00D7}").clicked() {
                state.search_query.clear();
                resp.request_focus();
            }
        });
    });

    // ── Filter chips ───────────────────────────────────────
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_XS, Theme::SPACE_XS);
        for (i, filter) in FILTERS.iter().enumerate() {
            let is_active = state.active_filter == i;
            let text_color = if is_active {
                Theme::accent()
            } else {
                Theme::t3()
            };
            let bg = if is_active {
                Theme::accent_subtle()
            } else {
                Theme::input_bg()
            };
            let border = if is_active {
                Stroke::new(Theme::STROKE_SUBTLE, Theme::with_alpha(Theme::accent(), 60))
            } else {
                Stroke::new(Theme::STROKE_SUBTLE, Theme::white_04())
            };

            let chip_btn = egui::Button::new(
                egui::RichText::new(*filter)
                    .size(Theme::FONT_XS)
                    .color(text_color),
            )
            .fill(bg)
            .stroke(border)
            .rounding(Rounding::same(Theme::RADIUS));

            if ui.add(chip_btn).clicked() {
                state.active_filter = i;
            }
        }
    });

    ui.add_space(2.0);

    // ── Media items ────────────────────────────────────────
    let query_lower = state.search_query.to_ascii_lowercase();
    let active_filter_str = FILTERS[state.active_filter];

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            // Empty state
            if state.items.is_empty() {
                ui.add_space(ui.available_height() * 0.15);
                ui.vertical_centered(|ui| {
                    ui.spacing_mut().item_spacing = Vec2::new(0.0, Theme::SPACE_SM);
                    // Dashed outline box
                    let avail = ui.available_width() * 0.75;
                    let (resp, painter) =
                        ui.allocate_painter(Vec2::new(avail, 70.0), egui::Sense::hover());
                    painter.rect_stroke(
                        resp.rect,
                        Rounding::same(8.0),
                        Stroke::new(1.0, Theme::white_06()),
                    );
                    painter.text(
                        egui::Pos2::new(resp.rect.center().x, resp.rect.center().y - 10.0),
                        egui::Align2::CENTER_CENTER,
                        "+",
                        egui::FontId::proportional(22.0),
                        Theme::white_10(),
                    );
                    painter.text(
                        egui::Pos2::new(resp.rect.center().x, resp.rect.center().y + 12.0),
                        egui::Align2::CENTER_CENTER,
                        "Import Media  \u{2318}I",
                        egui::FontId::proportional(Theme::FONT_XS),
                        Theme::t4(),
                    );
                    ui.label(
                        egui::RichText::new("or drag files here")
                            .size(Theme::FONT_XS)
                            .color(Theme::t4()),
                    );
                });
            }

            for item in &state.items {
                // Filter by search
                if !query_lower.is_empty() && !item.name.to_ascii_lowercase().contains(&query_lower)
                {
                    continue;
                }
                // Filter by type
                if active_filter_str != "all" && item.kind.label() != active_filter_str {
                    continue;
                }

                let item_frame = egui::Frame::none()
                    .rounding(Rounding::same(Theme::RADIUS))
                    .inner_margin(egui::Margin::symmetric(Theme::SPACE_SM, 5.0));

                let resp = item_frame
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing = Vec2::new(Theme::SPACE_SM, 0.0);

                            // Thumbnail
                            let (thumb_resp, thumb_painter) =
                                ui.allocate_painter(Vec2::new(34.0, 22.0), egui::Sense::hover());
                            let thumb_rect = thumb_resp.rect;
                            thumb_painter.rect_filled(
                                thumb_rect,
                                Rounding::same(4.0),
                                Theme::with_alpha(item.color, 60),
                            );
                            thumb_painter.text(
                                thumb_rect.center(),
                                egui::Align2::CENTER_CENTER,
                                item.kind.icon(),
                                egui::FontId::proportional(Theme::FONT_XS),
                                Theme::with_alpha(item.color, 200),
                            );

                            // Name + meta
                            ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = Vec2::new(0.0, 2.0);
                                ui.label(
                                    egui::RichText::new(item.name.as_str())
                                        .size(Theme::FONT_XS)
                                        .color(Theme::t1()),
                                );
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} \u{00B7} {}",
                                        item.duration, item.size
                                    ))
                                    .size(Theme::FONT_XS)
                                    .color(Theme::t4())
                                    .family(egui::FontFamily::Monospace),
                                );
                            });
                        });
                    })
                    .response;

                if resp.hovered() {
                    ui.painter().rect_filled(
                        resp.rect,
                        Rounding::same(Theme::RADIUS),
                        Theme::white_04(),
                    );
                }
            }

            ui.add_space(Theme::SPACE_SM);

            // ── Import button ──────────────────────────────
            let import_btn = egui::Button::new(
                egui::RichText::new("+ Import Media")
                    .size(Theme::FONT_XS)
                    .color(Theme::t3()),
            )
            .fill(egui::Color32::TRANSPARENT)
            .stroke(Stroke::new(Theme::STROKE_EMPHASIS, Theme::t4()))
            .rounding(Rounding::same(Theme::RADIUS))
            .min_size(Vec2::new(ui.available_width(), 44.0));

            if ui.add(import_btn).clicked() {
                actions.push(MediaBrowserAction::ImportMedia);
            }
        });
}
