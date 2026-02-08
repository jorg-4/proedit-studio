//! Track types for the timeline.

use proedit_core::RationalTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::clip::Clip;

/// Kind of track.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TrackKind {
    Video,
    Audio,
}

/// An item in a track (clip or gap).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackItem {
    Clip(Clip),
    Gap { duration: RationalTime },
}

impl TrackItem {
    /// Get the duration of this item.
    pub fn duration(&self) -> RationalTime {
        match self {
            TrackItem::Clip(clip) => clip.duration,
            TrackItem::Gap { duration } => *duration,
        }
    }
}

/// A track containing clips and gaps.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Track {
    /// Unique track ID
    pub id: Uuid,
    /// Track name
    pub name: String,
    /// Track kind
    pub kind: TrackKind,
    /// Items in this track
    pub items: Vec<TrackItem>,
    /// Is track muted
    pub muted: bool,
    /// Is track locked (prevent edits)
    pub locked: bool,
}

impl Track {
    /// Create a new video track.
    pub fn new_video(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind: TrackKind::Video,
            items: Vec::new(),
            muted: false,
            locked: false,
        }
    }

    /// Create a new audio track.
    pub fn new_audio(name: impl Into<String>) -> Self {
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            kind: TrackKind::Audio,
            items: Vec::new(),
            muted: false,
            locked: false,
        }
    }

    /// Get the total duration of this track.
    pub fn duration(&self) -> RationalTime {
        self.items.iter().fold(RationalTime::ZERO, |acc, item| {
            acc + item.duration()
        })
    }

    /// Add a clip to the end of the track.
    pub fn append_clip(&mut self, clip: Clip) {
        self.items.push(TrackItem::Clip(clip));
    }

    /// Add a gap to the end of the track.
    pub fn append_gap(&mut self, duration: RationalTime) {
        self.items.push(TrackItem::Gap { duration });
    }
}
