//! ProEdit Studio - Professional Video Editor
//!
//! Entry point and main application loop.
//! Liquid Glass iOS 26 dark aesthetic.

mod ai_bridge;
mod compositor;

use anyhow::Result;
use eframe::egui;
use proedit_core::{FrameBuffer, FrameRate};
use proedit_media::VideoDecoder;
use proedit_timeline::{Project, ProjectFile, Sequence};
use proedit_ui::timeline::{TimelineAction, TimelineClip};
use proedit_ui::{
    show_audio_mixer, show_color_wheels, show_command_palette, show_effects_panel,
    show_export_dialog, show_inspector, show_media_browser, show_timeline, show_top_bar,
    show_viewer, AudioMixerState, ColorWheelsState, CommandPaletteState, CommandRegistry,
    CurveEditorState, EffectsPanelState, ExportDialogAction, ExportDialogState, InspectorState,
    LeftTab, MediaBrowserState, Page, Theme, TimelineState, TopBarAction, TopBarState, ViewerState,
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

#[allow(dead_code)]
struct ProEditApp {
    // AI
    ai_engine: Option<proedit_ai::AIEngine>,

    // Core
    project: Project,
    decoder: Option<VideoDecoder>,
    current_frame: Option<FrameBuffer>,
    playing: bool,
    speed: f32,
    last_frame_time: std::time::Instant,
    start_time: std::time::Instant,
    frame_number: i64,

    // Audio
    audio_engine: Option<proedit_audio::AudioEngine>,

    // Undo/redo
    undo_snapshots: Vec<Vec<TimelineClip>>,
    redo_snapshots: Vec<Vec<TimelineClip>>,
    dirty: bool,
    project_path: Option<PathBuf>,

    // Command system
    command_registry: CommandRegistry,

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
    curve_editor: CurveEditorState,
    export_dialog: ExportDialogState,
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

        let audio_engine = match proedit_audio::AudioEngine::new() {
            Ok(engine) => {
                info!("Audio engine initialized");
                Some(engine)
            }
            Err(e) => {
                eprintln!("Audio engine init failed: {}", e);
                None
            }
        };

        Self {
            ai_engine: Some(ai_bridge::init_ai_engine()),
            project,
            decoder,
            current_frame: None,
            playing: false,
            speed: 1.0,
            last_frame_time: std::time::Instant::now(),
            start_time: std::time::Instant::now(),
            frame_number: 0,
            audio_engine,
            undo_snapshots: Vec::new(),
            redo_snapshots: Vec::new(),
            dirty: false,
            project_path: None,
            command_registry: CommandRegistry::new(),
            top_bar: TopBarState::default(),
            timeline: TimelineState::default(),
            viewer: ViewerState::default(),
            inspector: InspectorState::default(),
            media_browser: MediaBrowserState::default(),
            effects_panel: EffectsPanelState::default(),
            command_palette: CommandPaletteState::default(),
            color_wheels: ColorWheelsState::default(),
            audio_mixer: AudioMixerState::default(),
            curve_editor: CurveEditorState::default(),
            export_dialog: ExportDialogState::default(),
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
            false
        }
    }

    fn frame_rate(&self) -> FrameRate {
        self.decoder
            .as_ref()
            .map(|d| d.frame_rate())
            .unwrap_or(self.project.frame_rate)
    }

    // ── Undo/Redo ────────────────────────────────────────────

    fn push_undo(&mut self) {
        self.undo_snapshots.push(self.timeline.clips.clone());
        self.redo_snapshots.clear();
        self.dirty = true;
        // Limit history depth
        if self.undo_snapshots.len() > 200 {
            self.undo_snapshots.remove(0);
        }
    }

    fn undo(&mut self) {
        if let Some(snapshot) = self.undo_snapshots.pop() {
            self.redo_snapshots.push(self.timeline.clips.clone());
            self.timeline.clips = snapshot;
            self.dirty = true;
            info!("Undo");
        }
    }

    fn redo(&mut self) {
        if let Some(snapshot) = self.redo_snapshots.pop() {
            self.undo_snapshots.push(self.timeline.clips.clone());
            self.timeline.clips = snapshot;
            self.dirty = true;
            info!("Redo");
        }
    }

    // ── Save/Load ──────────────────────────────────────────

    fn save_project(&mut self) {
        let path = if let Some(ref p) = self.project_path {
            Some(p.clone())
        } else {
            rfd::FileDialog::new()
                .set_title("Save Project")
                .add_filter("ProEdit Project", &["pep"])
                .save_file()
        };
        if let Some(path) = path {
            let file = ProjectFile::new(self.project.clone());
            match file.save_to_file(&path) {
                Ok(()) => {
                    self.project_path = Some(path);
                    self.dirty = false;
                    info!("Project saved");
                }
                Err(e) => eprintln!("Save failed: {}", e),
            }
        }
    }

    fn load_project(&mut self) {
        let path = rfd::FileDialog::new()
            .set_title("Open Project")
            .add_filter("ProEdit Project", &["pep"])
            .pick_file();
        if let Some(path) = path {
            match ProjectFile::load_from_file(&path) {
                Ok(file) => {
                    self.project = file.project;
                    self.project_path = Some(path);
                    self.dirty = false;
                    self.undo_snapshots.clear();
                    self.redo_snapshots.clear();
                    info!("Project loaded");
                }
                Err(e) => eprintln!("Load failed: {}", e),
            }
        }
    }

    // ── Command dispatch ────────────────────────────────────

    fn execute_command(&mut self, name: &str) {
        match name {
            "Undo" => self.undo(),
            "Redo" => self.redo(),
            "Save Project" => self.save_project(),
            "Open Project" => self.load_project(),
            "Import Media" => self.import_media(),
            "Razor at Playhead" | "Split at Playhead" => {
                self.push_undo();
                self.razor_at_playhead();
            }
            "Ripple Delete" | "Delete" => {
                self.push_undo();
                self.delete_selected_clip();
            }
            "Add Marker" => {
                self.timeline.markers.push(proedit_ui::timeline::Marker {
                    frame: self.timeline.playhead,
                    color: Theme::amber(),
                });
            }
            "Toggle Audio Mixer" => {
                self.top_bar.audio_mixer_open = !self.top_bar.audio_mixer_open;
            }
            "Toggle Inspector" => {
                self.top_bar.inspector_open = !self.top_bar.inspector_open;
            }
            "Toggle Color Wheels" => {
                self.top_bar.color_wheels_open = !self.top_bar.color_wheels_open;
            }
            "Export" | "Export Project" => {
                self.export_dialog.open = !self.export_dialog.open;
            }
            "Play/Pause" => {
                self.playing = !self.playing;
                if let Some(ref mut engine) = self.audio_engine {
                    if self.playing {
                        engine.play();
                    } else {
                        engine.stop();
                    }
                }
            }
            "Zoom In" => {
                self.timeline.zoom = (self.timeline.zoom + 0.2).min(3.0);
            }
            "Zoom Out" => {
                self.timeline.zoom = (self.timeline.zoom - 0.2).max(0.4);
            }
            "Select All" => {
                self.timeline.selection = self.timeline.clips.iter().map(|c| c.id).collect();
            }
            "Command Palette" => {
                self.command_palette.toggle();
            }
            "Toggle Fullscreen" => {
                info!("Fullscreen toggle requested");
            }
            "New Project" => {
                self.project = Project::new("New Project");
                self.project.add_sequence(Sequence::default());
                self.timeline.clips.clear();
                self.undo_snapshots.clear();
                self.redo_snapshots.clear();
                self.dirty = false;
                self.project_path = None;
                info!("New project created");
            }
            "Detect Scenes" | "Scene Detect" => {
                info!("Scene detection requested (requires decoded frames)");
            }
            "Auto Color" | "Auto Color Match" => {
                info!("Auto color correction requested");
            }
            "Remove Background" => {
                info!("Background removal requested (requires AI model)");
            }
            "Enhance Audio" => {
                info!("Audio enhancement requested (requires AI model)");
            }
            "Upscale 4K" => {
                info!("4K upscale requested (requires AI model)");
            }
            "Style Transfer" => {
                info!("Style transfer requested (requires AI model)");
            }
            "Smart Stabilize" => {
                info!("Smart stabilization requested (requires AI model)");
            }
            _ => info!("Command '{}' not yet implemented", name),
        }
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
                if let Some(ref mut engine) = self.audio_engine {
                    if self.playing {
                        engine.play();
                    } else {
                        engine.stop();
                    }
                }
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
                if let Some(ref mut engine) = self.audio_engine {
                    engine.stop();
                }
            }
            // L — play forward (speed += 1)
            if inp.key_pressed(egui::Key::L) {
                self.speed += 1.0;
                self.playing = true;
            }
            // ArrowLeft — step back 1 frame
            if inp.key_pressed(egui::Key::ArrowLeft) {
                self.playing = false;
                self.timeline.playhead = (self.timeline.playhead - 1.0).max(0.0);
            }
            // ArrowRight — step forward 1 frame
            if inp.key_pressed(egui::Key::ArrowRight) {
                self.playing = false;
                self.timeline.playhead += 1.0;
            }
            // Home — jump to start
            if inp.key_pressed(egui::Key::Home) {
                self.timeline.playhead = 0.0;
            }
            // End — jump to end of last clip
            if inp.key_pressed(egui::Key::End) {
                let end = self
                    .timeline
                    .clips
                    .iter()
                    .map(|c| c.start + c.dur)
                    .fold(0.0_f32, f32::max);
                self.timeline.playhead = end;
            }
            // M — add marker at playhead
            if inp.key_pressed(egui::Key::M) && !inp.modifiers.command {
                self.timeline.markers.push(proedit_ui::timeline::Marker {
                    frame: self.timeline.playhead,
                    color: Theme::amber(),
                });
            }
            // I — toggle inspector (without ⌘)
            if inp.key_pressed(egui::Key::I) && !inp.modifiers.command {
                self.top_bar.inspector_open = !self.top_bar.inspector_open;
            }
            // C — razor at playhead (split selected clip)
            if inp.key_pressed(egui::Key::C) {
                self.push_undo();
                self.razor_at_playhead();
            }
            // Delete/Backspace — delete selected clip
            if inp.key_pressed(egui::Key::Delete) || inp.key_pressed(egui::Key::Backspace) {
                self.push_undo();
                self.delete_selected_clip();
            }
            // G — toggle curve editor
            if inp.key_pressed(egui::Key::G) {
                self.curve_editor.visible = !self.curve_editor.visible;
            }
            // +/= — zoom in
            if inp.key_pressed(egui::Key::Plus) || inp.key_pressed(egui::Key::Equals) {
                self.timeline.zoom = (self.timeline.zoom + 0.2).min(3.0);
            }
            // - — zoom out
            if inp.key_pressed(egui::Key::Minus) {
                self.timeline.zoom = (self.timeline.zoom - 0.2).max(0.4);
            }
            // Escape — close overlays, deselect
            if inp.key_pressed(egui::Key::Escape) {
                self.command_palette.open = false;
                self.timeline.selected_clip = None;
            }
        });

        // Modifier-key shortcuts (check separately to avoid conflicts)
        ctx.input(|inp| {
            // ⌘K — command palette
            if inp.modifiers.command && inp.key_pressed(egui::Key::K) {
                self.command_palette.toggle();
            }
            // ⌘Z — undo, ⌘⇧Z — redo
            if inp.modifiers.command && inp.key_pressed(egui::Key::Z) {
                if inp.modifiers.shift {
                    self.redo();
                } else {
                    self.undo();
                }
            }
            // ⌘S — save project
            if inp.modifiers.command && inp.key_pressed(egui::Key::S) {
                self.save_project();
            }
            // ⌘O — open project
            if inp.modifiers.command && inp.key_pressed(egui::Key::O) {
                self.load_project();
            }
            // ⌘I — import media
            if inp.modifiers.command && inp.key_pressed(egui::Key::I) {
                self.import_media();
            }
            // ⌘M — toggle audio mixer
            if inp.modifiers.command && inp.key_pressed(egui::Key::M) {
                self.top_bar.audio_mixer_open = !self.top_bar.audio_mixer_open;
            }
            // ⌘⇧E — toggle export dialog
            if inp.modifiers.command && inp.modifiers.shift && inp.key_pressed(egui::Key::E) {
                self.export_dialog.open = !self.export_dialog.open;
            }
        });
    }

    /// Razor tool: split the selected clip at the playhead position.
    fn razor_at_playhead(&mut self) {
        let Some(selected_id) = self.timeline.selected_clip else {
            return;
        };
        let playhead = self.timeline.playhead;

        let clip_idx = self.timeline.clips.iter().position(|c| c.id == selected_id);
        let Some(idx) = clip_idx else { return };

        let clip = &self.timeline.clips[idx];
        // Only split if playhead is within the clip bounds
        if playhead <= clip.start || playhead >= clip.start + clip.dur {
            return;
        }

        let split_offset = playhead - clip.start;
        let next_id = self.timeline.clips.iter().map(|c| c.id).max().unwrap_or(0) + 1;

        // Create the right half
        let right_half = proedit_ui::timeline::TimelineClip {
            id: next_id,
            name: format!("{} (split)", clip.name),
            color: clip.color,
            start: playhead,
            dur: clip.dur - split_offset,
            track: clip.track,
            clip_type: clip.clip_type,
        };

        // Trim the left half
        self.timeline.clips[idx].dur = split_offset;

        // Insert right half after the original
        self.timeline.clips.insert(idx + 1, right_half);
        info!("Razor split clip {} at frame {}", selected_id, playhead);
    }

    /// Delete the currently selected clip from the timeline.
    fn delete_selected_clip(&mut self) {
        let Some(selected_id) = self.timeline.selected_clip else {
            return;
        };
        self.timeline.clips.retain(|c| c.id != selected_id);
        self.timeline.selected_clip = None;
        info!("Deleted clip {}", selected_id);
    }

    /// Sync the inspector panel to the currently selected timeline clip.
    fn sync_inspector(&mut self) {
        match self.timeline.selected_clip {
            Some(id) => {
                let clip = self.timeline.clips.iter().find(|c| c.id == id);
                if let Some(clip) = clip {
                    // Only update if the inspector doesn't already show this clip
                    let needs_update = self
                        .inspector
                        .clip
                        .as_ref()
                        .map_or(true, |ic| ic.name != clip.name);
                    if needs_update {
                        let clip_type = match clip.clip_type {
                            proedit_ui::timeline::ClipKind::Video => {
                                proedit_ui::inspector::ClipType::Video
                            }
                            proedit_ui::timeline::ClipKind::Audio => {
                                proedit_ui::inspector::ClipType::Audio
                            }
                            proedit_ui::timeline::ClipKind::Gfx => {
                                proedit_ui::inspector::ClipType::Gfx
                            }
                        };
                        self.inspector.clip = Some(proedit_ui::InspectorClip::new(
                            Some(clip.id),
                            clip.name.clone(),
                            clip.color,
                            clip_type,
                            clip.dur,
                        ));
                    }
                } else {
                    self.inspector.clip = None;
                }
            }
            None => {
                self.inspector.clip = None;
            }
        }
    }

    // ── Media Import ────────────────────────────────────────────

    fn import_media(&mut self) {
        let paths = rfd::FileDialog::new()
            .set_title("Import Media")
            .add_filter(
                "Media Files",
                &[
                    "mp4", "mov", "avi", "mkv", "wav", "mp3", "aac", "png", "jpg",
                ],
            )
            .pick_files();
        let Some(paths) = paths else { return };

        for path in paths {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "unknown".into());

            // Probe the file to determine kind and duration
            let (kind, duration_str) = match VideoDecoder::open(&path) {
                Ok(dec) => {
                    let secs = dec.duration();
                    let dur = format!("{:.1}s", secs);
                    (proedit_ui::media_browser::MediaKind::Video, dur)
                }
                Err(_) => {
                    // Treat as audio if video probe fails
                    (proedit_ui::media_browser::MediaKind::Audio, "—".into())
                }
            };

            let size = std::fs::metadata(&path)
                .map(|m| {
                    let mb = m.len() as f64 / (1024.0 * 1024.0);
                    format!("{:.1} MB", mb)
                })
                .unwrap_or_else(|_| "—".into());

            let color = match kind {
                proedit_ui::media_browser::MediaKind::Video => Theme::accent(),
                proedit_ui::media_browser::MediaKind::Audio => Theme::green(),
                proedit_ui::media_browser::MediaKind::Image => Theme::amber(),
                proedit_ui::media_browser::MediaKind::Gfx => Theme::purple(),
            };

            self.media_browser
                .items
                .push(proedit_ui::media_browser::MediaItem {
                    name,
                    kind,
                    duration: duration_str,
                    size,
                    color,
                });
            info!("Imported: {:?}", path);
        }
    }

    // ── Page Switching ──────────────────────────────────────────

    fn apply_page_layout(&mut self, page: Page) {
        match page {
            Page::Cut => {
                // Simplified timeline-focused layout
                self.top_bar.inspector_open = false;
                self.top_bar.color_wheels_open = false;
                self.top_bar.audio_mixer_open = false;
                self.curve_editor.visible = false;
            }
            Page::Edit => {
                // Full NLE layout — inspector open
                self.top_bar.inspector_open = true;
                self.top_bar.color_wheels_open = false;
                self.top_bar.audio_mixer_open = false;
            }
            Page::Motion => {
                // Motion/keyframe focus — show curve editor
                self.top_bar.inspector_open = true;
                self.top_bar.color_wheels_open = false;
                self.top_bar.audio_mixer_open = false;
                self.curve_editor.visible = true;
            }
            Page::Color => {
                // Color grading focus
                self.top_bar.inspector_open = false;
                self.top_bar.color_wheels_open = true;
                self.top_bar.audio_mixer_open = false;
                self.curve_editor.visible = false;
            }
            Page::Audio => {
                // Audio mixing focus
                self.top_bar.inspector_open = false;
                self.top_bar.color_wheels_open = false;
                self.top_bar.audio_mixer_open = true;
                self.curve_editor.visible = false;
            }
            Page::Deliver => {
                // Export focus
                self.top_bar.inspector_open = false;
                self.top_bar.color_wheels_open = false;
                self.top_bar.audio_mixer_open = false;
                self.curve_editor.visible = false;
                self.export_dialog.open = true;
            }
        }
    }
}

impl eframe::App for ProEditApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let time = self.start_time.elapsed().as_secs_f64();

        // ── Sync fps to UI components ────────────────────────────
        let fps = self.frame_rate().to_fps_f64() as f32;
        self.timeline.fps = fps;
        self.viewer.fps = fps;

        // ── Playback ───────────────────────────────────────────
        if self.playing {
            let frame_duration = std::time::Duration::from_secs_f64(
                1.0 / (self.frame_rate().to_fps_f64() * self.speed.abs() as f64),
            );

            if self.last_frame_time.elapsed() >= frame_duration {
                self.decode_next_frame();
                self.timeline.playhead += self.speed;
                self.last_frame_time = std::time::Instant::now();
            }
            ctx.request_repaint();
        }

        if self.current_frame.is_none() && self.decoder.is_some() {
            self.decode_next_frame();
        }

        // ── Keyboard shortcuts ─────────────────────────────────
        self.handle_keyboard(ctx);

        // ── Sync viewer state ──────────────────────────────────
        self.viewer.playing = self.playing;
        self.viewer.playhead_frames = self.timeline.playhead;
        self.viewer.speed = self.speed;
        self.viewer.selected_clip = self.timeline.selected_clip;
        self.viewer.has_media = self.decoder.is_some();

        // ── Sync inspector to selected clip ─────────────────────
        self.sync_inspector();

        // ── Top bar ────────────────────────────────────────────
        egui::TopBottomPanel::top("top_bar")
            .exact_height(40.0)
            .frame(Theme::top_bar_frame())
            .show(ctx, |ui| {
                let response = show_top_bar(ui, &mut self.top_bar);
                for action in response.actions {
                    match action {
                        TopBarAction::OpenCommandPalette => self.command_palette.toggle(),
                        TopBarAction::PageChanged(page) => self.apply_page_layout(page),
                        _ => {}
                    }
                }
            });

        // ── Title bar dirty indicator ─────────────────────────────
        if self.dirty {
            ctx.send_viewport_cmd(egui::ViewportCommand::Title(
                "ProEdit Studio [unsaved]".into(),
            ));
        }

        // ── Timeline at bottom ─────────────────────────────────
        let timeline_actions = egui::TopBottomPanel::bottom("timeline_panel")
            .resizable(true)
            .min_height(120.0)
            .default_height(260.0)
            .frame(egui::Frame::none().fill(Theme::bg()))
            .show(ctx, |ui| show_timeline(ui, &mut self.timeline))
            .inner;

        // Handle timeline actions
        for action in timeline_actions {
            match action {
                TimelineAction::TrimClip { .. } => {
                    self.push_undo();
                }
                TimelineAction::DragClip { .. } => {
                    self.push_undo();
                }
                TimelineAction::SplitClip { clip_id, offset } => {
                    self.push_undo();
                    self.timeline.selected_clip = Some(clip_id);
                    self.razor_at_playhead();
                    let _ = (clip_id, offset);
                }
                TimelineAction::SelectClip(id) => {
                    self.timeline.selected_clip = id;
                    self.sync_inspector();
                }
                TimelineAction::SeekTo(f) => {
                    self.timeline.playhead = f;
                }
                _ => {}
            }
        }

        // ── Left panel (Media / Effects) ───────────────────────
        egui::SidePanel::left("left_panel")
            .resizable(true)
            .default_width(220.0)
            .min_width(180.0)
            .frame(Theme::panel_frame())
            .show(ctx, |ui| match self.top_bar.left_tab {
                LeftTab::Media => show_media_browser(ui, &mut self.media_browser),
                LeftTab::Effects => show_effects_panel(ui, &mut self.effects_panel),
            });

        // ── Right panel (Inspector) ────────────────────────────
        if self.top_bar.inspector_open {
            let inspector_actions = egui::SidePanel::right("inspector_panel")
                .resizable(true)
                .default_width(240.0)
                .min_width(200.0)
                .frame(Theme::panel_frame())
                .show(ctx, |ui| show_inspector(ui, &mut self.inspector))
                .inner;

            for action in inspector_actions {
                match action {
                    proedit_ui::InspectorAction::PropertyChanged { .. } => {
                        self.push_undo();
                    }
                }
            }
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
                            if let Some(ref mut engine) = self.audio_engine {
                                if self.playing {
                                    engine.play();
                                } else {
                                    engine.stop();
                                }
                            }
                        }
                        proedit_ui::viewer::ViewerAction::SetSpeed(s) => {
                            self.speed = s;
                        }
                    }
                }
            });

        // ── Curve editor (resizable bottom panel when visible) ──
        if self.curve_editor.visible {
            let fr = self.frame_rate();
            egui::TopBottomPanel::bottom("curve_editor_panel")
                .resizable(true)
                .min_height(100.0)
                .default_height(200.0)
                .frame(egui::Frame::none().fill(Theme::bg()))
                .show(ctx, |ui| {
                    let empty_track = proedit_core::keyframe::KeyframeTrack::new("(none)");
                    let _curve_actions =
                        proedit_ui::show_curve_editor(ui, &mut self.curve_editor, &empty_track, fr);
                });
        }

        // ── Floating panels ────────────────────────────────────
        if self.top_bar.color_wheels_open {
            show_color_wheels(ctx, &mut self.color_wheels, time);
        }
        if self.top_bar.audio_mixer_open {
            show_audio_mixer(ctx, &mut self.audio_mixer);
        }

        // ── Export dialog ─────────────────────────────────────
        let export_actions = show_export_dialog(ctx, &mut self.export_dialog);
        for action in export_actions {
            match action {
                ExportDialogAction::StartExport {
                    format,
                    output_path,
                } => {
                    info!(
                        "Export requested: {:?} -> {:?}",
                        format.video_codec, output_path
                    );
                }
                ExportDialogAction::Cancel => {
                    info!("Export cancelled by user");
                }
            }
        }

        // ── Command palette (must be last — topmost layer) ─────
        show_command_palette(ctx, &mut self.command_palette);

        // Handle command palette execution
        if let Some(cmd) = self.command_palette.executed.take() {
            self.execute_command(cmd);
        }
    }
}
