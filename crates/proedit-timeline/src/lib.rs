//! ProEdit Timeline - Timeline data model
//!
//! Implements the timeline structure for video editing:
//! - Projects containing sequences
//! - Tracks containing clips
//! - Edit operations with undo/redo
//! - Professional trim modes (ripple, roll, slip, slide)

pub mod clip;
pub mod edit;
pub mod project;
pub mod serialization;
pub mod track;

pub use clip::{Clip, ClipRef};
pub use edit::{EditCommand, TrimMode, UndoStack};
pub use project::{Project, Sequence};
pub use serialization::{ProjectFile, RecentProjects};
pub use track::{Track, TrackItem, TrackKind};
