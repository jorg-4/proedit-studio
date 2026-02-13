//! ProEdit UI - egui widgets for video editing
//!
//! Provides UI components:
//! - Timeline widget
//! - Video viewport
//! - Properties inspector
//! - Media browser
//! - Effects panel
//! - Command palette
//! - Color wheels
//! - Audio mixer

pub mod anim;
pub mod audio_mixer;
pub mod color_wheels;
pub mod command_palette;
pub mod effects_panel;
pub mod inspector;
pub mod media_browser;
pub mod theme;
pub mod timeline;
pub mod top_bar;
pub mod viewer;

// Re-exports for main app convenience
pub use audio_mixer::{show_audio_mixer, AudioMixerState};
pub use color_wheels::{show_color_wheels, ColorWheelsState};
pub use command_palette::{show_command_palette, CommandPaletteState};
pub use effects_panel::{show_effects_panel, EffectsPanelState};
pub use inspector::{show_inspector, InspectorClip, InspectorState};
pub use media_browser::{show_media_browser, MediaBrowserState};
pub use theme::Theme;
pub use timeline::{show_timeline, TimelineState};
pub use top_bar::{show_top_bar, LeftTab, Page, TopBarAction, TopBarState};
pub use viewer::{show_viewer, ViewerState};

// Keep the original types for backwards compatibility with existing code
use egui::Ui;
use proedit_core::{FrameRate, RationalTime};
use proedit_timeline::Sequence;

/// Timeline widget for displaying and editing sequences.
pub struct TimelineWidget {
    /// Zoom level (pixels per second)
    pub zoom: f32,
    /// Scroll position
    pub scroll: f32,
    /// Current playhead position
    pub playhead: RationalTime,
}

impl TimelineWidget {
    /// Create a new timeline widget.
    pub fn new() -> Self {
        Self {
            zoom: 100.0,
            scroll: 0.0,
            playhead: RationalTime::ZERO,
        }
    }

    /// Draw the timeline widget.
    pub fn show(&mut self, ui: &mut Ui, sequence: Option<&Sequence>) {
        let available = ui.available_size();

        // Timeline background
        ui.allocate_ui(available, |ui| {
            let (response, painter) = ui.allocate_painter(available, egui::Sense::click_and_drag());
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(30, 30, 30));

            // Draw track lanes
            if let Some(seq) = sequence {
                let track_height = 40.0;
                let mut y = rect.top() + 20.0;

                for track in &seq.video_tracks {
                    painter.rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(rect.left(), y),
                            egui::vec2(rect.width(), track_height),
                        ),
                        0.0,
                        egui::Color32::from_rgb(40, 40, 50),
                    );

                    painter.text(
                        egui::pos2(rect.left() + 5.0, y + 10.0),
                        egui::Align2::LEFT_TOP,
                        &track.name,
                        egui::FontId::proportional(12.0),
                        egui::Color32::WHITE,
                    );

                    y += track_height + 2.0;
                }
            }

            // Playhead
            let playhead_x = rect.left() + self.playhead.to_seconds_f64() as f32 * self.zoom - self.scroll;
            painter.line_segment(
                [egui::pos2(playhead_x, rect.top()), egui::pos2(playhead_x, rect.bottom())],
                egui::Stroke::new(2.0, egui::Color32::RED),
            );
        });
    }
}

impl Default for TimelineWidget {
    fn default() -> Self {
        Self::new()
    }
}

/// Transport controls (play/pause/seek).
pub struct TransportControls {
    pub playing: bool,
    pub frame_rate: FrameRate,
}

impl TransportControls {
    pub fn new(frame_rate: FrameRate) -> Self {
        Self {
            playing: false,
            frame_rate,
        }
    }

    pub fn show(&mut self, ui: &mut Ui, playhead: &mut RationalTime) {
        ui.horizontal(|ui| {
            // Rewind button
            if ui.button("\u{23EE}").clicked() {
                *playhead = RationalTime::ZERO;
            }

            // Play/Pause button
            let play_text = if self.playing { "\u{23F8}" } else { "\u{25B6}" };
            if ui.button(play_text).clicked() {
                self.playing = !self.playing;
            }

            // Current time display
            ui.label(format!(
                "{:.2}s | Frame {}",
                playhead.to_seconds_f64(),
                playhead.to_frames(self.frame_rate)
            ));
        });
    }
}

impl Default for TransportControls {
    fn default() -> Self {
        Self::new(FrameRate::FPS_24)
    }
}
