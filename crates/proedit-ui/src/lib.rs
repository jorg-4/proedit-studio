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
