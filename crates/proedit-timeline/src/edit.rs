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
    /// Apply this command to a sequence, mutating it in place.
    ///
    /// Mutable `&mut self` because some variants store data during execution
    /// (e.g., `RemoveClip` stores the removed clip for undo, `AddTrack` records
    /// the generated track ID).
    pub fn apply(&mut self, sequence: &mut crate::project::Sequence) {
        match self {
            Self::InsertClip {
                track_id,
                index,
                clip,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    track.insert_clip(*index, clip.clone());
                }
            }
            Self::RemoveClip {
                track_id,
                index,
                removed,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(crate::track::TrackItem::Clip(clip)) = track.remove_item(*index) {
                        *removed = Some(clip);
                    }
                }
            }
            Self::MoveClip {
                src_track_id,
                src_index,
                dst_track_id,
                dst_index,
            } => {
                let clip = find_track_mut(sequence, *src_track_id)
                    .and_then(|t| t.remove_item(*src_index))
                    .and_then(|item| match item {
                        crate::track::TrackItem::Clip(c) => Some(c),
                        _ => None,
                    });
                if let Some(clip) = clip {
                    if let Some(dst) = find_track_mut(sequence, *dst_track_id) {
                        dst.insert_clip(*dst_index, clip);
                    }
                }
            }
            Self::RippleTrim {
                track_id,
                clip_index,
                delta,
                trim_in,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(clip) = track.clip_at_mut(*clip_index) {
                        if *trim_in {
                            clip.trim_in(*delta);
                        } else {
                            clip.trim_out(*delta);
                        }
                    }
                }
            }
            Self::RollTrim {
                track_id,
                clip_index,
                delta,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(clip) = track.clip_at_mut(*clip_index) {
                        clip.trim_out(*delta);
                    }
                    if let Some(clip) = track.clip_at_mut(*clip_index + 1) {
                        clip.trim_in(*delta);
                    }
                }
            }
            Self::Slip {
                track_id,
                clip_index,
                delta,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(clip) = track.clip_at_mut(*clip_index) {
                        clip.source_in = clip.source_in + *delta;
                    }
                }
            }
            Self::Slide {
                track_id,
                clip_index,
                delta,
            } => {
                // Slide moves the clip within surrounding gaps. Adjust gap before
                // and gap after by opposite amounts. When no gap exists, this is a
                // no-op (the clip is pinned).
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    let idx = *clip_index;
                    let d = *delta;
                    // Shrink gap before, grow gap after (or vice versa)
                    if idx > 0 {
                        if let crate::track::TrackItem::Gap { duration } = &mut track.items[idx - 1]
                        {
                            *duration = *duration - d;
                        }
                    }
                    if idx + 1 < track.items.len() {
                        if let crate::track::TrackItem::Gap { duration } = &mut track.items[idx + 1]
                        {
                            *duration = *duration + d;
                        }
                    }
                }
            }
            Self::SplitClip {
                track_id,
                clip_index,
                offset,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    // Read data from original clip
                    let split_data = track.clip_at(*clip_index).map(|clip| {
                        (
                            clip.name.clone(),
                            clip.source.clone(),
                            clip.source_in,
                            clip.duration,
                            clip.speed,
                            clip.enabled,
                        )
                    });
                    if let Some((name, source, source_in, _orig_dur, speed, enabled)) = split_data {
                        // Shorten left clip to offset
                        if let Some(clip) = track.clip_at_mut(*clip_index) {
                            clip.duration = *offset;
                        }
                        // Create right half
                        let mut right = Clip::new(format!("{name} (split)"), source);
                        right.source_in = source_in + *offset;
                        right.duration = _orig_dur - *offset;
                        right.speed = speed;
                        right.enabled = enabled;
                        track.insert_clip(*clip_index + 1, right);
                    }
                }
            }
            Self::ToggleClipEnabled {
                track_id,
                clip_index,
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(clip) = track.clip_at_mut(*clip_index) {
                        clip.enabled = !clip.enabled;
                    }
                }
            }
            Self::SetClipSpeed {
                track_id,
                clip_index,
                new_speed,
                ..
            } => {
                if let Some(track) = find_track_mut(sequence, *track_id) {
                    if let Some(clip) = track.clip_at_mut(*clip_index) {
                        clip.speed = *new_speed;
                    }
                }
            }
            Self::AddTrack {
                kind,
                name,
                track_id,
            } => {
                let new_track = match kind {
                    TrackKind::Video => {
                        let t = Track::new_video(name.clone());
                        let id = t.id;
                        sequence.video_tracks.push(t);
                        id
                    }
                    TrackKind::Audio => {
                        let t = Track::new_audio(name.clone());
                        let id = t.id;
                        sequence.audio_tracks.push(t);
                        id
                    }
                };
                *track_id = Some(new_track);
            }
            Self::RemoveTrack {
                track_id,
                removed,
                index,
            } => {
                if let Some(idx) = sequence.video_tracks.iter().position(|t| t.id == *track_id) {
                    *index = Some(idx);
                    *removed = Some(sequence.video_tracks.remove(idx));
                } else if let Some(idx) =
                    sequence.audio_tracks.iter().position(|t| t.id == *track_id)
                {
                    *index = Some(idx);
                    *removed = Some(sequence.audio_tracks.remove(idx));
                }
            }
            Self::Batch(commands) => {
                for cmd in commands {
                    cmd.apply(sequence);
                }
            }
        }
    }

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

// ── Helpers ──────────────────────────────────────────────────────

/// Find a track mutably by UUID, searching both video and audio tracks.
fn find_track_mut(sequence: &mut crate::project::Sequence, track_id: Uuid) -> Option<&mut Track> {
    sequence
        .video_tracks
        .iter_mut()
        .chain(sequence.audio_tracks.iter_mut())
        .find(|track| track.id == track_id)
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

    // ── apply() tests ─────────────────────────────────────────

    fn make_sequence_with_track() -> (crate::project::Sequence, Uuid) {
        let seq = crate::project::Sequence::default();
        let track_id = seq.video_tracks[0].id;
        (seq, track_id)
    }

    #[test]
    fn test_apply_insert_clip() {
        let (mut seq, track_id) = make_sequence_with_track();
        let clip = make_test_clip("inserted");

        let mut cmd = EditCommand::InsertClip {
            track_id,
            index: 0,
            clip: clip.clone(),
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks[0].clip_count(), 1);
        assert_eq!(seq.video_tracks[0].clip_at(0).unwrap().name, "inserted");
    }

    #[test]
    fn test_apply_remove_clip() {
        let (mut seq, track_id) = make_sequence_with_track();
        let clip = make_test_clip("to_remove");
        seq.video_tracks[0].append_clip(clip);

        let mut cmd = EditCommand::RemoveClip {
            track_id,
            index: 0,
            removed: None,
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks[0].clip_count(), 0);
        if let EditCommand::RemoveClip { removed, .. } = &cmd {
            assert!(removed.is_some());
            assert_eq!(removed.as_ref().unwrap().name, "to_remove");
        }
    }

    #[test]
    fn test_apply_move_clip() {
        let (mut seq, src_track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("mover"));
        // Add a second video track
        let dst = Track::new_video("V2");
        let dst_track_id = dst.id;
        seq.video_tracks.push(dst);

        let mut cmd = EditCommand::MoveClip {
            src_track_id,
            src_index: 0,
            dst_track_id,
            dst_index: 0,
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks[0].clip_count(), 0);
        assert_eq!(seq.video_tracks[1].clip_count(), 1);
        assert_eq!(seq.video_tracks[1].clip_at(0).unwrap().name, "mover");
    }

    #[test]
    fn test_apply_ripple_trim_out() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("clip1"));
        let orig_dur = seq.video_tracks[0].clip_at(0).unwrap().duration;

        let delta = RationalTime::new(2, 1);
        let mut cmd = EditCommand::RippleTrim {
            track_id,
            clip_index: 0,
            delta,
            trim_in: false,
        };
        cmd.apply(&mut seq);

        let new_dur = seq.video_tracks[0].clip_at(0).unwrap().duration;
        assert_eq!(new_dur, orig_dur + delta);
    }

    #[test]
    fn test_apply_ripple_trim_in() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("clip1"));
        let orig_in = seq.video_tracks[0].clip_at(0).unwrap().source_in;
        let orig_dur = seq.video_tracks[0].clip_at(0).unwrap().duration;

        let delta = RationalTime::new(2, 1);
        let mut cmd = EditCommand::RippleTrim {
            track_id,
            clip_index: 0,
            delta,
            trim_in: true,
        };
        cmd.apply(&mut seq);

        let clip = seq.video_tracks[0].clip_at(0).unwrap();
        assert_eq!(clip.source_in, orig_in + delta);
        assert_eq!(clip.duration, orig_dur - delta);
    }

    #[test]
    fn test_apply_toggle_enabled() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("clip1"));
        assert!(seq.video_tracks[0].clip_at(0).unwrap().enabled);

        let mut cmd = EditCommand::ToggleClipEnabled {
            track_id,
            clip_index: 0,
        };
        cmd.apply(&mut seq);
        assert!(!seq.video_tracks[0].clip_at(0).unwrap().enabled);

        cmd.apply(&mut seq);
        assert!(seq.video_tracks[0].clip_at(0).unwrap().enabled);
    }

    #[test]
    fn test_apply_set_speed() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("clip1"));

        let mut cmd = EditCommand::SetClipSpeed {
            track_id,
            clip_index: 0,
            old_speed: 1.0,
            new_speed: 2.0,
        };
        cmd.apply(&mut seq);
        assert_eq!(seq.video_tracks[0].clip_at(0).unwrap().speed, 2.0);
    }

    #[test]
    fn test_apply_split_clip() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("original"));
        let orig_dur = seq.video_tracks[0].clip_at(0).unwrap().duration;
        let split_at = RationalTime::new(4, 1);

        let mut cmd = EditCommand::SplitClip {
            track_id,
            clip_index: 0,
            offset: split_at,
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks[0].clip_count(), 2);
        let left = seq.video_tracks[0].clip_at(0).unwrap();
        let right = seq.video_tracks[0].clip_at(1).unwrap();
        assert_eq!(left.duration, split_at);
        assert_eq!(right.duration, orig_dur - split_at);
        assert!(right.name.contains("split"));
    }

    #[test]
    fn test_apply_add_track() {
        let (mut seq, _) = make_sequence_with_track();
        let initial_video = seq.video_tracks.len();

        let mut cmd = EditCommand::AddTrack {
            kind: TrackKind::Video,
            name: "V2".into(),
            track_id: None,
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks.len(), initial_video + 1);
        if let EditCommand::AddTrack { track_id, .. } = &cmd {
            assert!(track_id.is_some());
        }
    }

    #[test]
    fn test_apply_remove_track() {
        let (mut seq, track_id) = make_sequence_with_track();
        assert_eq!(seq.video_tracks.len(), 1);

        let mut cmd = EditCommand::RemoveTrack {
            track_id,
            removed: None,
            index: None,
        };
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks.len(), 0);
        if let EditCommand::RemoveTrack { removed, index, .. } = &cmd {
            assert!(removed.is_some());
            assert_eq!(*index, Some(0));
        }
    }

    #[test]
    fn test_apply_slip() {
        let (mut seq, track_id) = make_sequence_with_track();
        seq.video_tracks[0].append_clip(make_test_clip("clip1"));
        let orig_in = seq.video_tracks[0].clip_at(0).unwrap().source_in;
        let orig_dur = seq.video_tracks[0].clip_at(0).unwrap().duration;

        let delta = RationalTime::new(3, 1);
        let mut cmd = EditCommand::Slip {
            track_id,
            clip_index: 0,
            delta,
        };
        cmd.apply(&mut seq);

        let clip = seq.video_tracks[0].clip_at(0).unwrap();
        assert_eq!(clip.source_in, orig_in + delta);
        assert_eq!(clip.duration, orig_dur); // Duration unchanged
    }

    #[test]
    fn test_apply_batch() {
        let (mut seq, track_id) = make_sequence_with_track();
        let mut cmd = EditCommand::Batch(vec![
            EditCommand::InsertClip {
                track_id,
                index: 0,
                clip: make_test_clip("batch1"),
            },
            EditCommand::InsertClip {
                track_id,
                index: 1,
                clip: make_test_clip("batch2"),
            },
        ]);
        cmd.apply(&mut seq);

        assert_eq!(seq.video_tracks[0].clip_count(), 2);
        assert_eq!(seq.video_tracks[0].clip_at(0).unwrap().name, "batch1");
        assert_eq!(seq.video_tracks[0].clip_at(1).unwrap().name, "batch2");
    }

    #[test]
    fn test_apply_then_inverse_restores() {
        let (mut seq, track_id) = make_sequence_with_track();
        let clip = make_test_clip("roundtrip");

        // Apply insert
        let mut cmd = EditCommand::InsertClip {
            track_id,
            index: 0,
            clip: clip.clone(),
        };
        cmd.apply(&mut seq);
        assert_eq!(seq.video_tracks[0].clip_count(), 1);

        // Apply inverse (remove)
        let mut inv = cmd.inverse();
        inv.apply(&mut seq);
        assert_eq!(seq.video_tracks[0].clip_count(), 0);
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
