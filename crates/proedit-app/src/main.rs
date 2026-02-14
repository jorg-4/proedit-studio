//! ProEdit Studio - Professional Video Editor
//!
//! Entry point and main application loop.
//! Liquid Glass iOS 26 dark aesthetic.

use anyhow::Result;
use eframe::egui;
use proedit_core::{FrameBuffer, FrameRate, RationalTime};
use proedit_media::VideoDecoder;
use proedit_timeline::{Project, Sequence};
use proedit_ui::{
    show_audio_mixer, show_color_wheels, show_command_palette, show_effects_panel,
    show_inspector, show_media_browser, show_timeline, show_top_bar, show_viewer,
    AudioMixerState, ColorWheelsState, CommandPaletteState, EffectsPanelState, InspectorState,
    LeftTab, MediaBrowserState, Theme, TimelineState, TopBarState, ViewerState,
};
use std::path::PathBuf;
use tracing::{info, Level};
use tracing_subscriber::FmtSubscriber;

fn main() -> Result<()> {
    let subscriber = FmtSubscriber::builder()
        .with_max_level(Level::INFO)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;

    info!("ProEdit Studio starting...");
    proedit_media::init();

    let video_path = std::env::args().nth(1).map(PathBuf::from);

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1440.0, 900.0])
            .with_title("ProEdit Studio")
            .with_decorations(false),
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
    // Core
    #[allow(dead_code)]
    project: Project,
    decoder: Option<VideoDecoder>,
    current_frame: Option<FrameBuffer>,
    playhead: RationalTime,
    playing: bool,
    speed: f32,
    last_frame_time: std::time::Instant,
    start_time: std::time::Instant,
    frame_number: i64,

    // UI state
    top_bar: TopBarState,
    timeline: TimelineState,
    viewer: ViewerState,
    inspector: InspectorState,
    media_browser: MediaBrowserState,
    effects_panel: EffectsPanelState,
    command_palette: CommandPaletteState,
    color_wheels: ColorWheelsState,
    audio_mixer: AudioMixerState,
}

impl ProEditApp {
    fn new(cc: &eframe::CreationContext<'_>, video_path: Option<PathBuf>) -> Self {
        // Apply the Liquid Glass theme
        Theme::apply(&cc.egui_ctx);

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

        let mut project = Project::new("New Project");
        project.add_sequence(Sequence::default());

        Self {
            project,
            decoder,
            current_frame: None,
            playhead: RationalTime::ZERO,
            playing: false,
            speed: 1.0,
            last_frame_time: std::time::Instant::now(),
            start_time: std::time::Instant::now(),
            frame_number: 0,
            top_bar: TopBarState::default(),
            timeline: TimelineState::default(),
            viewer: ViewerState::default(),
            inspector: InspectorState::default(),
            media_browser: MediaBrowserState::default(),
            effects_panel: EffectsPanelState::default(),
            command_palette: CommandPaletteState::default(),
            color_wheels: ColorWheelsState::default(),
            audio_mixer: AudioMixerState::default(),
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

    fn handle_keyboard(&mut self, ctx: &egui::Context) {
        // Don't handle keys if command palette is open (it handles its own)
        if self.command_palette.open {
            return;
        }

        ctx.input(|inp| {
            // Space — toggle play/pause
            if inp.key_pressed(egui::Key::Space) {
                self.playing = !self.playing;
            }
            // J — play reverse (speed -= 1)
            if inp.key_pressed(egui::Key::J) {
                self.speed -= 1.0;
                self.playing = true;
            }
            // K — stop, reset speed
            if inp.key_pressed(egui::Key::K) {
                self.playing = false;
                self.speed = 1.0;
            }
            // L — play forward (speed += 1)
            if inp.key_pressed(egui::Key::L) {
                self.speed += 1.0;
                self.playing = true;
            }
            // ArrowLeft — step back 1 frame
            if inp.key_pressed(egui::Key::ArrowLeft) {
                self.timeline.playhead = (self.timeline.playhead - 1.0).max(0.0);
            }
            // ArrowRight — step forward 1 frame
            if inp.key_pressed(egui::Key::ArrowRight) {
                self.timeline.playhead += 1.0;
            }
            // M — add marker at playhead
            if inp.key_pressed(egui::Key::M) {
                self.timeline.markers.push(proedit_ui::timeline::Marker {
                    frame: self.timeline.playhead,
                    color: Theme::amber(),
                });
            }
            // I — toggle inspector
            if inp.key_pressed(egui::Key::I) {
                self.top_bar.inspector_open = !self.top_bar.inspector_open;
            }
            // +/= — zoom in
            if inp.key_pressed(egui::Key::Plus) || inp.key_pressed(egui::Key::Equals) {
                self.timeline.zoom = (self.timeline.zoom + 0.2).min(3.0);
            }
            // - — zoom out
            if inp.key_pressed(egui::Key::Minus) {
                self.timeline.zoom = (self.timeline.zoom - 0.2).max(0.4);
            }
            // Escape — close overlays
            if inp.key_pressed(egui::Key::Escape) {
                self.command_palette.open = false;
            }
        });

        // ⌘K — command palette (check modifiers separately)
        ctx.input(|inp| {
            if inp.modifiers.command && inp.key_pressed(egui::Key::K) {
                self.command_palette.toggle();
            }
        });
    }
}

impl eframe::App for ProEditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = self.start_time.elapsed().as_secs_f64();

        // ── Playback ───────────────────────────────────────────
        if self.playing {
            let frame_duration =
                std::time::Duration::from_secs_f64(1.0 / (self.frame_rate().to_fps_f64() * self.speed.abs() as f64));

            if self.last_frame_time.elapsed() >= frame_duration {
                self.decode_next_frame();
                self.playhead = self.playhead + self.frame_rate().frame_duration();
                self.timeline.playhead += self.speed;
                self.last_frame_time = std::time::Instant::now();
            }
            ctx.request_repaint();
        }

        if self.current_frame.is_none() {
            self.decode_next_frame();
        }

        // ── Keyboard shortcuts ─────────────────────────────────
        self.handle_keyboard(ctx);

        // ── Sync viewer state ──────────────────────────────────
        self.viewer.playing = self.playing;
        self.viewer.playhead_frames = self.timeline.playhead;
        self.viewer.speed = self.speed;
        self.viewer.selected_clip = self.timeline.selected_clip;

        // ── Top bar ────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar")
            .exact_height(40.0)
            .frame(Theme::top_bar_frame())
            .show(ctx, |ui| {
                let response = show_top_bar(ui, &mut self.top_bar);
                for action in response.actions {
                    if let proedit_ui::TopBarAction::OpenCommandPalette = action {
                        self.command_palette.toggle();
                    }
                }
            });

        // ── Timeline at bottom ─────────────────────────────────
        egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(120.0)
            .default_height(260.0)
            .frame(egui::Frame::none().fill(Theme::bg()))
            .show(ctx, |ui| {
                let _actions = show_timeline(ui, &mut self.timeline);
            });

        // ── Left panel (Media / Effects) ───────────────────────
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .min_width(180.0)
            .frame(egui::Frame::none().fill(Theme::bg1()).inner_margin(egui::Margin::same(8.0)))
            .show(ctx, |ui| {
                match self.top_bar.left_tab {
                    LeftTab::Media => show_media_browser(ui, &mut self.media_browser),
                    LeftTab::Effects => show_effects_panel(ui, &mut self.effects_panel),
                }
            });

        // ── Right panel (Inspector) ────────────────────────────
        if self.top_bar.inspector_open {
            egui::SidePanel::right("inspector_panel")
                .resizable(true)
                .default_width(240.0)
                .min_width(200.0)
                .frame(egui::Frame::none().fill(Theme::bg1()).inner_margin(egui::Margin::same(8.0)))
                .show(ctx, |ui| {
                    show_inspector(ui, &mut self.inspector);
                });
        }

        // ── Central viewport ───────────────────────────────────
        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Theme::bg()))
            .show(ctx, |ui| {
                let viewer_actions = show_viewer(ui, &self.viewer, time);
                for action in viewer_actions {
                    match action {
                        proedit_ui::viewer::ViewerAction::TogglePlay => {
                            self.playing = !self.playing;
                        }
                        proedit_ui::viewer::ViewerAction::SetSpeed(s) => {
                            self.speed = s;
                        }
                    }
                }
            });

        // ── Floating panels ────────────────────────────────────
        if self.top_bar.color_wheels_open {
            show_color_wheels(ctx, &mut self.color_wheels, time);
        }
        if self.top_bar.audio_mixer_open {
            show_audio_mixer(ctx, &mut self.audio_mixer);
        }

        // ── Command palette (must be last — topmost layer) ─────
        show_command_palette(ctx, &mut self.command_palette);
    }
}
