//! ProEdit Studio - Professional Video Editor
//!
//! Entry point and main application loop.

use anyhow::Result;
use eframe::egui;
use proedit_core::{FrameBuffer, FrameRate, RationalTime};
use proedit_media::VideoDecoder;
use proedit_timeline::{Project, Sequence};
use proedit_ui::{TimelineWidget, TransportControls};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    // Initialize logging
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("ProEdit Studio starting...");

    // Initialize media subsystem
    proedit_media::init();

    // Parse command line for video file
    let video_path = std::env::args().nth(1).map(PathBuf::from);

    // Run the application
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1280.0, 720.0])
            .with_title("ProEdit Studio"),
        renderer: eframe::Renderer::Wgpu,
        ..Default::default()
    };

    eframe::run_native(
        "ProEdit Studio",
        options,
        Box::new(move |cc| Ok(Box::new(ProEditApp::new(cc, video_path)))),
    )?;

    Ok(())
}

struct ProEditApp {
    project: Project,
    decoder: Option<VideoDecoder>,
    current_frame: Option<FrameBuffer>,
    playhead: RationalTime,
    playing: bool,
    last_frame_time: std::time::Instant,
    timeline_widget: TimelineWidget,
    transport: TransportControls,
    frame_number: i64,
}

impl ProEditApp {
    fn new(_cc: &eframe::CreationContext<'_>, video_path: Option<PathBuf>) -> Self {
        // Try to open the video file
        let decoder = video_path.and_then(|path| {
            if path.exists() {
                match VideoDecoder::open(&path) {
                    Ok(dec) => {
                        info!("Opened video: {:?}", path);
                        Some(dec)
                    }
                    Err(e) => {
                        eprintln!("Failed to open video: {}", e);
                        None
                    }
                }
            } else {
                eprintln!("Video file not found: {:?}", path);
                None
            }
        });

        // Create default project
        let mut project = Project::new("New Project");
        project.add_sequence(Sequence::default());

        Self {
            project,
            decoder,
            current_frame: None,
            playhead: RationalTime::ZERO,
            playing: false,
            last_frame_time: std::time::Instant::now(),
            timeline_widget: TimelineWidget::new(),
            transport: TransportControls::default(),
            frame_number: 0,
        }
    }

    fn decode_next_frame(&mut self) -> bool {
        if let Some(ref mut decoder) = self.decoder {
            match decoder.decode_frame() {
                Ok(Some(frame)) => {
                    self.current_frame = Some(frame.buffer);
                    self.frame_number = frame.frame_number;
                    true
                }
                Ok(None) => {
                    info!("End of video reached");
                    self.playing = false;
                    false
                }
                Err(e) => {
                    eprintln!("Decode error: {}", e);
                    false
                }
            }
        } else {
            // Generate test pattern
            self.current_frame = Some(FrameBuffer::test_pattern(1920, 1080));
            self.frame_number += 1;
            true
        }
    }

    fn frame_rate(&self) -> FrameRate {
        self.decoder
            .as_ref()
            .map(|d| d.frame_rate())
            .unwrap_or(FrameRate::FPS_24)
    }
}

impl eframe::App for ProEditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle playback
        if self.playing {
            let frame_duration =
                std::time::Duration::from_secs_f64(1.0 / self.frame_rate().to_fps_f64());

            if self.last_frame_time.elapsed() >= frame_duration {
                self.decode_next_frame();
                self.playhead = self.playhead + self.frame_rate().frame_duration();
                self.last_frame_time = std::time::Instant::now();
            }
            ctx.request_repaint();
        }

        // Decode first frame if we haven't yet
        if self.current_frame.is_none() {
            self.decode_next_frame();
        }

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("New Project").clicked() {
                        self.project = Project::new("New Project");
                        self.project.add_sequence(Sequence::default());
                        ui.close_menu();
                    }
                    if ui.button("Open...").clicked() {
                        info!("Open clicked");
                        ui.close_menu();
                    }
                    if ui.button("Save").clicked() {
                        info!("Save clicked");
                        ui.close_menu();
                    }
                    ui.separator();
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });
                ui.menu_button("Edit", |ui| {
                    if ui.button("Undo").clicked() {
                        info!("Undo clicked");
                        ui.close_menu();
                    }
                    if ui.button("Redo").clicked() {
                        info!("Redo clicked");
                        ui.close_menu();
                    }
                });
                ui.menu_button("Help", |ui| {
                    if ui.button("About").clicked() {
                        info!("ProEdit Studio v0.1.0");
                        ui.close_menu();
                    }
                });
            });
        });

        // Timeline at bottom
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(100.0)
            .default_height(200.0)
            .show(ctx, |ui| {
                ui.heading("Timeline");
                let sequence = self.project.active_sequence();
                self.timeline_widget.playhead = self.playhead;
                self.timeline_widget.show(ui, sequence);
            });

        // Media browser on left
        egui::SidePanel::left("media_panel")
            .resizable(true)
            .default_width(200.0)
            .show(ctx, |ui| {
                ui.heading("Media Browser");
                ui.separator();
                ui.label("Drag media files here to import");
                if ui.button("Import Media...").clicked() {
                    info!("Import media clicked");
                }
            });

        // Inspector on right
        egui::SidePanel::right("inspector_panel")
            .resizable(true)
            .default_width(250.0)
            .show(ctx, |ui| {
                ui.heading("Inspector");
                ui.separator();
                if let Some(ref decoder) = self.decoder {
                    ui.label(format!(
                        "Resolution: {}x{}",
                        decoder.dimensions().0,
                        decoder.dimensions().1
                    ));
                    ui.label(format!("Frame Rate: {}", decoder.frame_rate()));
                    ui.label(format!("Duration: {:.2}s", decoder.duration()));
                } else {
                    ui.label("No media selected");
                }

                ui.separator();
                ui.heading("Project");
                ui.label(format!("Name: {}", self.project.name));
                if let Some(seq) = self.project.active_sequence() {
                    ui.label(format!("Sequence: {}", seq.name));
                    ui.label(format!("Resolution: {}x{}", seq.width, seq.height));
                }
            });

        // Central viewport
        egui::CentralPanel::default().show(ctx, |ui| {
            // Transport controls
            self.transport.playing = self.playing;
            self.transport.show(ui, &mut self.playhead);
            self.playing = self.transport.playing;

            ui.separator();

            // Video frame display
            let (response, painter) = ui.allocate_painter(ui.available_size(), egui::Sense::click());
            let rect = response.rect;

            // Background
            painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(20, 20, 20));

            // Frame info and preview
            if let Some(ref frame) = self.current_frame {
                // Draw simplified color bars preview
                let preview_size = egui::vec2(
                    (rect.width() * 0.6).min(640.0),
                    (rect.height() * 0.6).min(360.0),
                );
                let preview_rect = egui::Rect::from_center_size(rect.center(), preview_size);

                // Color bars
                let bar_width = preview_rect.width() / 8.0;
                let colors = [
                    egui::Color32::WHITE,
                    egui::Color32::YELLOW,
                    egui::Color32::from_rgb(0, 255, 255),
                    egui::Color32::GREEN,
                    egui::Color32::from_rgb(255, 0, 255),
                    egui::Color32::RED,
                    egui::Color32::BLUE,
                    egui::Color32::BLACK,
                ];
                for (i, color) in colors.iter().enumerate() {
                    let bar_rect = egui::Rect::from_min_size(
                        egui::pos2(preview_rect.left() + i as f32 * bar_width, preview_rect.top()),
                        egui::vec2(bar_width, preview_rect.height()),
                    );
                    painter.rect_filled(bar_rect, 0.0, *color);
                }

                // Frame info below preview
                let info = format!(
                    "Frame {} | {}x{} | {}",
                    self.frame_number,
                    frame.width,
                    frame.height,
                    self.frame_rate()
                );
                painter.text(
                    egui::pos2(rect.center().x, preview_rect.bottom() + 20.0),
                    egui::Align2::CENTER_TOP,
                    info,
                    egui::FontId::proportional(14.0),
                    egui::Color32::WHITE,
                );
            } else {
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "No video loaded\nUse File > Open to load a video",
                    egui::FontId::proportional(16.0),
                    egui::Color32::GRAY,
                );
            }
        });
    }
}
