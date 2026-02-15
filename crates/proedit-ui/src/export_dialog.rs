//! Export dialog — format selection, output path, progress bar, and cancel.

use crate::theme::Theme;
use egui::{self, Vec2};
use proedit_media::export::ExportFormat;
use std::path::PathBuf;

// ── Format preset labels ────────────────────────────────────────

const FORMAT_PRESETS: &[&str] = &["H.264 HD", "H.265 4K", "ProRes 422", "VP9 Web"];

fn format_from_index(index: usize) -> ExportFormat {
    match index {
        0 => ExportFormat::h264_hd(),
        1 => ExportFormat::h265_4k(),
        2 => ExportFormat::prores_422(),
        3 => ExportFormat::vp9_web(),
        _ => ExportFormat::h264_hd(),
    }
}

// ── State ───────────────────────────────────────────────────────

/// Persistent state for the export dialog.
#[derive(Default)]
pub struct ExportDialogState {
    /// Whether the dialog window is visible.
    pub open: bool,
    /// Currently selected format preset index.
    pub format_index: usize,
    /// Output file path string.
    pub output_path: String,
    /// Export progress (0.0 .. 1.0), `None` when idle.
    pub progress: Option<f32>,
    /// Whether an export is currently running.
    pub exporting: bool,
}

// ── Actions ─────────────────────────────────────────────────────

/// Actions that the export dialog can produce.
#[derive(Debug)]
pub enum ExportDialogAction {
    /// User clicked "Export" with these settings.
    StartExport {
        format: ExportFormat,
        output_path: PathBuf,
    },
    /// User clicked "Cancel" during an active export.
    Cancel,
}

// ── Rendering ───────────────────────────────────────────────────

/// Show the export dialog as a floating egui window.
///
/// Returns any actions the caller should handle.
pub fn show_export_dialog(
    ctx: &egui::Context,
    state: &mut ExportDialogState,
) -> Vec<ExportDialogAction> {
    let mut actions = Vec::new();

    if !state.open {
        return actions;
    }

    let mut still_open = state.open;

    egui::Window::new("Export")
        .open(&mut still_open)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
        .frame(Theme::glass_frame())
        .show(ctx, |ui| {
            ui.set_width(340.0);
            ui.spacing_mut().item_spacing = Vec2::new(0.0, Theme::SPACE_SM);

            // ── Format preset ────────────────────────────
            ui.label(
                egui::RichText::new("FORMAT PRESET")
                    .size(Theme::FONT_XS)
                    .color(Theme::t3())
                    .strong(),
            );

            egui::ComboBox::from_id_salt("export_format_combo")
                .selected_text(FORMAT_PRESETS[state.format_index])
                .width(ui.available_width())
                .show_ui(ui, |ui| {
                    for (i, label) in FORMAT_PRESETS.iter().enumerate() {
                        ui.selectable_value(&mut state.format_index, i, *label);
                    }
                });

            ui.add_space(Theme::SPACE_XS);

            // ── Output path ──────────────────────────────
            ui.label(
                egui::RichText::new("OUTPUT PATH")
                    .size(Theme::FONT_XS)
                    .color(Theme::t3())
                    .strong(),
            );

            ui.horizontal(|ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut state.output_path)
                        .desired_width(ui.available_width() - 70.0)
                        .hint_text("Select output file..."),
                );
                if ui
                    .add_enabled(!state.exporting, egui::Button::new("Browse"))
                    .clicked()
                {
                    // Placeholder — actual rfd dialog handled by the app layer
                }
            });

            ui.add_space(Theme::SPACE_SM);
            Theme::draw_separator(ui);
            ui.add_space(Theme::SPACE_SM);

            // ── Progress bar (visible when exporting) ────
            if let Some(progress) = state.progress {
                ui.label(
                    egui::RichText::new(format!("Exporting... {:.0}%", progress * 100.0))
                        .size(Theme::FONT_SM)
                        .color(Theme::t1()),
                );
                let bar = egui::ProgressBar::new(progress).desired_width(ui.available_width());
                ui.add(bar);

                ui.add_space(Theme::SPACE_SM);
            }

            // ── Buttons ──────────────────────────────────
            ui.horizontal(|ui| {
                if state.exporting {
                    if ui.button("Cancel").clicked() {
                        actions.push(ExportDialogAction::Cancel);
                    }
                } else {
                    let can_export = !state.output_path.is_empty();
                    if ui
                        .add_enabled(can_export, egui::Button::new("Export"))
                        .clicked()
                    {
                        let format = format_from_index(state.format_index);
                        let output_path = PathBuf::from(&state.output_path);
                        actions.push(ExportDialogAction::StartExport {
                            format,
                            output_path,
                        });
                    }
                }
            });
        });

    state.open = still_open;

    actions
}
