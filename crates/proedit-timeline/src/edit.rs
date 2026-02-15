//! Edit operations with undo/redo support.
//!
//! Uses the Command pattern: every mutation is an `EditCommand` that knows
//! how to apply itself and produce its inverse for undo.

use proedit_core::RationalTime;
use uuid::Uuid;

use crate::clip::Clip;
use crate::track::{Track, TrackKind};

// ── Trim types ──────────────────────────────────────────────────

/// Professional NLE trim modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
    /// Trim in/out point — changes clip duration, shifts subsequent items.
    Ripple,
    /// Move the cut point between two adjacent clips — total duration unchanged.
    Roll,
    /// Shift source in/out within the clip — position and duration unchanged.
    Slip,
    /// Move clip within its gap — adjacent gaps absorb the change.
    Slide,
}

// ── Edit commands ───────────────────────────────────────────────

/// A reversible edit operation on the timeline.
#[derive(Debug, Clone)]
pub enum EditCommand {
    /// Insert a clip at position `index` on a track.
    InsertClip {
        track_id: Uuid,
        index: usize,
        clip: Clip,
    },
    /// Remove the clip at position `index` on a track.
    RemoveClip {
        track_id: Uuid,
        index: usize,
        /// Stored for undo — populated when the command is executed.
        removed: Option<Clip>,
    },
    /// Move a clip from one position to another (possibly across tracks).
    MoveClip {
        src_track_id: Uuid,
        src_index: usize,
        dst_track_id: Uuid,
        dst_index: usize,
    },
    /// Ripple trim: adjust in or out point, shifting subsequent items.
    RippleTrim {
        track_id: Uuid,
        clip_index: usize,
        /// Positive = lengthen clip (trim earlier in), negative = shorten.
        delta: RationalTime,
        /// True = trim in-point, false = trim out-point.
        trim_in: bool,
    },
    /// Roll trim: move the cut between clip_index and clip_index+1.
    RollTrim {
        track_id: Uuid,
        clip_index: usize,
        delta: RationalTime,
    },
    /// Slip: shift the source window without changing timeline position.
    Slip {
        track_id: Uuid,
        clip_index: usize,
        delta: RationalTime,
    },
    /// Slide: move clip within surrounding gaps.
    Slide {
        track_id: Uuid,
        clip_index: usize,
        delta: RationalTime,
    },
    /// Split a clip at a time offset (relative to clip start on timeline).
    SplitClip {
        track_id: Uuid,
        clip_index: usize,
        offset: RationalTime,
    },
    /// Toggle clip enabled state.
    ToggleClipEnabled { track_id: Uuid, clip_index: usize },
    /// Set clip speed.
    SetClipSpeed {
        track_id: Uuid,
        clip_index: usize,
        old_speed: f64,
        new_speed: f64,
    },
    /// Add a track to the sequence.
    AddTrack {
        kind: TrackKind,
        name: String,
        /// Populated after execution.
        track_id: Option<Uuid>,
    },
    /// Remove a track by ID.
    RemoveTrack {
        track_id: Uuid,
        /// Stored for undo.
        removed: Option<Track>,
        /// Original index in the track list.
        index: Option<usize>,
    },
    /// A batch of commands applied atomically.
    Batch(Vec<EditCommand>),
}

impl EditCommand {
    /// Produce the inverse command (for undo).
    pub fn inverse(&self) -> Self {
        match self {
            Self::InsertClip {
                track_id,
                index,
                clip,
            } => Self::RemoveClip {
                track_id: *track_id,
                index: *index,
                removed: Some(clip.clone()),
            },
            Self::RemoveClip {
                track_id,
                index,
                removed,
            } => Self::InsertClip {
                track_id: *track_id,
                index: *index,
                clip: removed.clone().expect("removed clip must be populated"),
            },
            Self::MoveClip {
                src_track_id,
                src_index,
                dst_track_id,
                dst_index,
            } => Self::MoveClip {
                src_track_id: *dst_track_id,
                src_index: *dst_index,
                dst_track_id: *src_track_id,
                dst_index: *src_index,
            },
            Self::RippleTrim {
                track_id,
                clip_index,
                delta,
                trim_in,
            } => Self::RippleTrim {
                track_id: *track_id,
                clip_index: *clip_index,
                delta: -*delta,
                trim_in: *trim_in,
            },
            Self::RollTrim {
                track_id,
                clip_index,
                delta,
            } => Self::RollTrim {
                track_id: *track_id,
                clip_index: *clip_index,
                delta: -*delta,
            },
            Self::Slip {
                track_id,
                clip_index,
                delta,
            } => Self::Slip {
                track_id: *track_id,
                clip_index: *clip_index,
                delta: -*delta,
            },
            Self::Slide {
                track_id,
                clip_index,
                delta,
            } => Self::Slide {
                track_id: *track_id,
                clip_index: *clip_index,
                delta: -*delta,
            },
            Self::SplitClip {
                track_id,
                clip_index,
                ..
            } => {
                // Undo split = merge clip_index and clip_index+1
                // Represented as removing the second clip and extending the first
                Self::RemoveClip {
                    track_id: *track_id,
                    index: clip_index + 1,
                    removed: None,
                }
            }
            Self::ToggleClipEnabled {
                track_id,
                clip_index,
            } => Self::ToggleClipEnabled {
                track_id: *track_id,
                clip_index: *clip_index,
            },
            Self::SetClipSpeed {
                track_id,
                clip_index,
                old_speed,
                new_speed,
            } => Self::SetClipSpeed {
                track_id: *track_id,
                clip_index: *clip_index,
                old_speed: *new_speed,
                new_speed: *old_speed,
            },
            Self::AddTrack {
                track_id, kind: _, ..
            } => Self::RemoveTrack {
                track_id: track_id.expect("track_id must be populated"),
                removed: None,
                index: None,
            },
            Self::RemoveTrack {
                track_id,
                removed,
                index: _,
            } => Self::AddTrack {
                kind: removed.as_ref().map(|t| t.kind).unwrap_or(TrackKind::Video),
                name: removed.as_ref().map(|t| t.name.clone()).unwrap_or_default(),
                track_id: Some(*track_id),
            },
            Self::Batch(commands) => {
                Self::Batch(commands.iter().rev().map(|c| c.inverse()).collect())
            }
        }
    }
}

// ── Undo stack ──────────────────────────────────────────────────

/// Undo/redo history stack.
#[derive(Debug)]
pub struct UndoStack {
    /// Commands that have been executed (most recent last).
    undo: Vec<EditCommand>,
    /// Commands that have been undone (most recent last).
    redo: Vec<EditCommand>,
    /// Maximum history depth.
    max_depth: usize,
}

impl UndoStack {
    /// Create a new undo stack with the given maximum depth.
    pub fn new(max_depth: usize) -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            max_depth,
        }
    }

    /// Push a command onto the undo stack after it has been executed.
    /// Clears the redo stack (new action invalidates redo history).
    pub fn push(&mut self, command: EditCommand) {
        self.redo.clear();
        self.undo.push(command);
        if self.undo.len() > self.max_depth {
            self.undo.remove(0);
        }
    }

    /// Pop the most recent command for undo. Returns the inverse command.
    pub fn undo(&mut self) -> Option<EditCommand> {
        let cmd = self.undo.pop()?;
        let inverse = cmd.inverse();
        self.redo.push(cmd);
        Some(inverse)
    }

    /// Pop the most recent undone command for redo. Returns the original command.
    pub fn redo(&mut self) -> Option<EditCommand> {
        let cmd = self.redo.pop()?;
        self.undo.push(cmd.clone());
        Some(cmd)
    }

    /// Check if undo is available.
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    /// Check if redo is available.
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    /// Clear all history.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
    }

    /// Number of undo steps available.
    pub fn undo_count(&self) -> usize {
        self.undo.len()
    }

    /// Number of redo steps available.
    pub fn redo_count(&self) -> usize {
        self.redo.len()
    }
}

impl Default for UndoStack {
    fn default() -> Self {
        Self::new(200)
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clip::{Clip, ClipRef};
    use proedit_core::RationalTime;

    fn make_test_clip(name: &str) -> Clip {
        Clip::new(name, ClipRef::new("test.mp4", RationalTime::new(10, 1)))
    }

    #[test]
    fn test_undo_redo_insert_remove() {
        let mut stack = UndoStack::new(100);
        let clip = make_test_clip("clip1");

        let cmd = EditCommand::InsertClip {
            track_id: Uuid::nil(),
            index: 0,
            clip: clip.clone(),
        };

        stack.push(cmd);
        assert!(stack.can_undo());
        assert!(!stack.can_redo());

        // Undo should give us a RemoveClip
        let undo_cmd = stack.undo().unwrap();
        assert!(matches!(undo_cmd, EditCommand::RemoveClip { .. }));
        assert!(!stack.can_undo());
        assert!(stack.can_redo());

        // Redo should give us the original InsertClip back
        let redo_cmd = stack.redo().unwrap();
        assert!(matches!(redo_cmd, EditCommand::InsertClip { .. }));
        assert!(stack.can_undo());
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_new_action_clears_redo() {
        let mut stack = UndoStack::new(100);
        let clip1 = make_test_clip("clip1");
        let clip2 = make_test_clip("clip2");

        stack.push(EditCommand::InsertClip {
            track_id: Uuid::nil(),
            index: 0,
            clip: clip1,
        });
        stack.undo();
        assert!(stack.can_redo());

        // New action clears redo
        stack.push(EditCommand::InsertClip {
            track_id: Uuid::nil(),
            index: 0,
            clip: clip2,
        });
        assert!(!stack.can_redo());
    }

    #[test]
    fn test_max_depth() {
        let mut stack = UndoStack::new(3);
        for i in 0..5 {
            stack.push(EditCommand::ToggleClipEnabled {
                track_id: Uuid::nil(),
                clip_index: i,
            });
        }
        assert_eq!(stack.undo_count(), 3);
    }

    #[test]
    fn test_ripple_trim_inverse() {
        let cmd = EditCommand::RippleTrim {
            track_id: Uuid::nil(),
            clip_index: 0,
            delta: RationalTime::new(5, 1),
            trim_in: true,
        };
        let inv = cmd.inverse();
        if let EditCommand::RippleTrim { delta, .. } = inv {
            assert_eq!(delta, RationalTime::new(-5, 1));
        } else {
            panic!("expected RippleTrim inverse");
        }
    }

    #[test]
    fn test_batch_inverse_reverses_order() {
        let cmd = EditCommand::Batch(vec![
            EditCommand::ToggleClipEnabled {
                track_id: Uuid::nil(),
                clip_index: 0,
            },
            EditCommand::SetClipSpeed {
                track_id: Uuid::nil(),
                clip_index: 0,
                old_speed: 1.0,
                new_speed: 2.0,
            },
        ]);
        let inv = cmd.inverse();
        if let EditCommand::Batch(cmds) = inv {
            assert_eq!(cmds.len(), 2);
            // First in inverse = last in original (reversed)
            assert!(
                matches!(cmds[0], EditCommand::SetClipSpeed { new_speed, .. } if new_speed == 1.0)
            );
            assert!(matches!(cmds[1], EditCommand::ToggleClipEnabled { .. }));
        } else {
            panic!("expected Batch inverse");
        }
    }
}
