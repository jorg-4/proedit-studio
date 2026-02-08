//! Project and sequence types.

use proedit_core::{FrameRate, RationalTime, TimeRange};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::track::Track;

/// A project containing media references and sequences.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Project {
    /// Unique project ID
    pub id: Uuid,
    /// Project name
    pub name: String,
    /// Default frame rate
    pub frame_rate: FrameRate,
    /// Sequences in this project
    pub sequences: Vec<Sequence>,
}

impl Project {
    /// Create a new empty project.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            frame_rate: FrameRate::FPS_24,
            sequences: Vec::new(),
        }
    }

    /// Add a new sequence to the project.
    pub fn add_sequence(&mut self, sequence: Sequence) {
        self.sequences.push(sequence);
    }

    /// Get the active sequence (first one for now).
    pub fn active_sequence(&self) -> Option<&Sequence> {
        self.sequences.first()
    }

    /// Get the active sequence mutably.
    pub fn active_sequence_mut(&mut self) -> Option<&mut Sequence> {
        self.sequences.first_mut()
    }
}

impl Default for Project {
    fn default() -> Self {
        Self::new("Untitled Project")
    }
}

/// A sequence (timeline) containing tracks.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Sequence {
    /// Unique sequence ID
    pub id: Uuid,
    /// Sequence name
    pub name: String,
    /// Frame rate
    pub frame_rate: FrameRate,
    /// Resolution width
    pub width: u32,
    /// Resolution height
    pub height: u32,
    /// Video tracks
    pub video_tracks: Vec<Track>,
    /// Audio tracks
    pub audio_tracks: Vec<Track>,
}

impl Sequence {
    /// Create a new sequence.
    pub fn new(name: impl Into<String>, width: u32, height: u32, frame_rate: FrameRate) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            frame_rate,
            width,
            height,
            video_tracks: vec![Track::new_video("V1")],
            audio_tracks: vec![Track::new_audio("A1")],
        }
    }

    /// Get the total duration of the sequence.
    pub fn duration(&self) -> RationalTime {
        let video_duration = self
            .video_tracks
            .iter()
            .map(|t| t.duration())
            .max()
            .unwrap_or(RationalTime::ZERO);

        let audio_duration = self
            .audio_tracks
            .iter()
            .map(|t| t.duration())
            .max()
            .unwrap_or(RationalTime::ZERO);

        if video_duration > audio_duration {
            video_duration
        } else {
            audio_duration
        }
    }

    /// Get the time range of the sequence.
    pub fn time_range(&self) -> TimeRange {
        TimeRange::new(RationalTime::ZERO, self.duration())
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new("Sequence 1", 1920, 1080, FrameRate::FPS_24)
    }
}
