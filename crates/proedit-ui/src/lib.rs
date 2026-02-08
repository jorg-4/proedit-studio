//! ProEdit UI - egui widgets for video editing
//!
//! Provides UI components:
//! - Timeline widget
//! - Video viewport
//! - Properties inspector
//! - Media browser

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
            if ui.button("⏮").clicked() {
                *playhead = RationalTime::ZERO;
            }

            // Play/Pause button
            let play_text = if self.playing { "⏸" } else { "▶" };
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
