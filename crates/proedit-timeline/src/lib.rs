//! ProEdit Timeline - Timeline data model
//!
//! Implements the timeline structure for video editing:
//! - Projects containing sequences
//! - Tracks containing clips
//! - Edit operations with undo/redo

pub mod clip;
pub mod project;
pub mod track;

pub use clip::{Clip, ClipRef};
pub use project::{Project, Sequence};
pub use track::{Track, TrackItem, TrackKind};
