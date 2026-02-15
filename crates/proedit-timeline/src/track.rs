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

/// An item in a track (clip, gap, or transition).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TrackItem {
    Clip(Clip),
    Gap {
        duration: RationalTime,
    },
    Transition {
        transition_name: String,
        duration: RationalTime,
    },
}

impl TrackItem {
    /// Get the duration of this item.
    pub fn duration(&self) -> RationalTime {
        match self {
            TrackItem::Clip(clip) => clip.duration,
            TrackItem::Gap { duration } => *duration,
            TrackItem::Transition { duration, .. } => *duration,
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
        self.items
            .iter()
            .fold(RationalTime::ZERO, |acc, item| acc + item.duration())
    }

    /// Add a clip to the end of the track.
    pub fn append_clip(&mut self, clip: Clip) {
        self.items.push(TrackItem::Clip(clip));
    }

    /// Add a gap to the end of the track.
    pub fn append_gap(&mut self, duration: RationalTime) {
        self.items.push(TrackItem::Gap { duration });
    }

    /// Insert a clip at the given index.
    pub fn insert_clip(&mut self, index: usize, clip: Clip) {
        let index = index.min(self.items.len());
        self.items.insert(index, TrackItem::Clip(clip));
    }

    /// Remove the item at the given index. Returns the removed item.
    pub fn remove_item(&mut self, index: usize) -> Option<TrackItem> {
        if index < self.items.len() {
            Some(self.items.remove(index))
        } else {
            None
        }
    }

    /// Find a clip by UUID. Returns (index, &Clip).
    pub fn find_clip(&self, id: uuid::Uuid) -> Option<(usize, &Clip)> {
        self.items.iter().enumerate().find_map(|(i, item)| {
            if let TrackItem::Clip(clip) = item {
                if clip.id == id {
                    return Some((i, clip));
                }
            }
            None
        })
    }

    /// Find a clip mutably by UUID. Returns (index, &mut Clip).
    pub fn find_clip_mut(&mut self, id: uuid::Uuid) -> Option<(usize, &mut Clip)> {
        self.items.iter_mut().enumerate().find_map(|(i, item)| {
            if let TrackItem::Clip(item) = item {
                if item.id == id {
                    return Some((i, item));
                }
            }
            None
        })
    }

    /// Get the clip at the given item index (if it's a clip).
    pub fn clip_at(&self, index: usize) -> Option<&Clip> {
        match self.items.get(index) {
            Some(TrackItem::Clip(clip)) => Some(clip),
            _ => None,
        }
    }

    /// Get the clip mutably at the given item index.
    pub fn clip_at_mut(&mut self, index: usize) -> Option<&mut Clip> {
        match self.items.get_mut(index) {
            Some(TrackItem::Clip(clip)) => Some(clip),
            _ => None,
        }
    }

    /// Get the timeline start time of item at the given index.
    pub fn item_start_time(&self, index: usize) -> RationalTime {
        self.items[..index]
            .iter()
            .fold(RationalTime::ZERO, |acc, item| acc + item.duration())
    }

    /// Find which item contains the given time. Returns (index, time_within_item).
    pub fn item_at_time(&self, time: RationalTime) -> Option<(usize, RationalTime)> {
        let mut pos = RationalTime::ZERO;
        for (i, item) in self.items.iter().enumerate() {
            let end = pos + item.duration();
            if time >= pos && time < end {
                return Some((i, time - pos));
            }
            pos = end;
        }
        None
    }

    /// Collapse adjacent gaps into single gaps.
    pub fn consolidate_gaps(&mut self) {
        let mut i = 0;
        while i + 1 < self.items.len() {
            if let (TrackItem::Gap { duration: d1 }, TrackItem::Gap { duration: d2 }) =
                (&self.items[i], &self.items[i + 1])
            {
                let merged = *d1 + *d2;
                self.items[i] = TrackItem::Gap { duration: merged };
                self.items.remove(i + 1);
            } else {
                i += 1;
            }
        }
        // Remove trailing zero-duration gaps
        while let Some(TrackItem::Gap { duration }) = self.items.last() {
            if duration.is_zero() {
                self.items.pop();
            } else {
                break;
            }
        }
    }

    /// Insert a transition between two items.
    pub fn insert_transition(&mut self, between_index: usize, name: &str, duration: RationalTime) {
        let index = (between_index + 1).min(self.items.len());
        self.items.insert(
            index,
            TrackItem::Transition {
                transition_name: name.to_string(),
                duration,
            },
        );
    }

    /// Number of clips (excluding gaps) in this track.
    pub fn clip_count(&self) -> usize {
        self.items
            .iter()
            .filter(|item| matches!(item, TrackItem::Clip(_)))
            .count()
    }
}
