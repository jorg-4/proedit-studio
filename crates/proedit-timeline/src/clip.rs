//! Clip types for the timeline.

use proedit_core::{RationalTime, TimeRange};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Reference to a media source.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipRef {
    /// Path to the media file
    pub path: String,
    /// Source duration
    pub source_duration: RationalTime,
}

impl ClipRef {
    /// Create a new clip reference.
    pub fn new(path: impl Into<String>, duration: RationalTime) -> Self {
        Self {
            path: path.into(),
            source_duration: duration,
        }
    }
}

/// A clip on the timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Clip {
    /// Unique clip ID
    pub id: Uuid,
    /// Clip name (displayed in UI)
    pub name: String,
    /// Reference to source media
    pub source: ClipRef,
    /// Source in point
    pub source_in: RationalTime,
    /// Duration on timeline
    pub duration: RationalTime,
    /// Playback speed (1.0 = normal)
    pub speed: f64,
    /// Is clip enabled
    pub enabled: bool,
}

impl Clip {
    /// Create a new clip from a source.
    pub fn new(name: impl Into<String>, source: ClipRef) -> Self {
        let duration = source.source_duration;
        Self {
            id: Uuid::new_v4(),
            name: name.into(),
            source,
            source_in: RationalTime::ZERO,
            duration,
            speed: 1.0,
            enabled: true,
        }
    }

    /// Get the source time range.
    pub fn source_range(&self) -> TimeRange {
        TimeRange::new(self.source_in, self.duration)
    }

    /// Get the source out point.
    pub fn source_out(&self) -> RationalTime {
        self.source_in + self.duration
    }

    /// Trim the clip's in point.
    pub fn trim_in(&mut self, delta: RationalTime) {
        self.source_in = self.source_in + delta;
        self.duration = self.duration - delta;
    }

    /// Trim the clip's out point.
    pub fn trim_out(&mut self, delta: RationalTime) {
        self.duration = self.duration + delta;
    }
}
