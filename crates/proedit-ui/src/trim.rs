//! Trim handle interaction for timeline clips.

use crate::timeline::TimelineClip;
use egui::{CursorIcon, Pos2, Rect};

/// Which edge of a clip is being trimmed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimEdge {
    Left,
    Right,
}

/// Trim mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrimMode {
    /// Ripple trim: moves the edit point and shifts subsequent clips.
    Ripple,
    /// Roll trim: moves the edit point between two adjacent clips.
    Roll,
}

/// Active trim state.
#[derive(Debug, Clone)]
pub struct TrimState {
    pub active: bool,
    pub clip_id: usize,
    pub edge: TrimEdge,
    pub mode: TrimMode,
    pub start_frame: f32,
    pub current_frame: f32,
    pub original_clip_start: f32,
    pub original_clip_dur: f32,
}

impl TrimState {
    /// Create a new trim state for the given clip and edge.
    pub fn new(clip: &TimelineClip, edge: TrimEdge, start_frame: f32) -> Self {
        Self {
            active: true,
            clip_id: clip.id,
            edge,
            mode: TrimMode::Ripple,
            start_frame,
            current_frame: start_frame,
            original_clip_start: clip.start,
            original_clip_dur: clip.dur,
        }
    }

    /// How far the trim has moved from its starting position.
    pub fn delta(&self) -> f32 {
        self.current_frame - self.start_frame
    }
}

/// Hit test a position against a clip's trim handles.
///
/// Returns `Some(TrimEdge)` if the position is over a trim handle, otherwise `None`.
pub fn hit_test_trim_handle(clip_rect: Rect, pos: Pos2, handle_width: f32) -> Option<TrimEdge> {
    if !clip_rect.contains(pos) {
        return None;
    }

    let left_handle = Rect::from_min_size(
        clip_rect.min,
        egui::Vec2::new(handle_width, clip_rect.height()),
    );
    if left_handle.contains(pos) {
        return Some(TrimEdge::Left);
    }

    let right_handle = Rect::from_min_size(
        Pos2::new(clip_rect.right() - handle_width, clip_rect.top()),
        egui::Vec2::new(handle_width, clip_rect.height()),
    );
    if right_handle.contains(pos) {
        return Some(TrimEdge::Right);
    }

    None
}

/// Apply a trim operation to a clip.
///
/// Modifies the clip's start and duration based on the trim state
/// and the snapped target frame.
pub fn apply_trim(clip: &mut TimelineClip, trim: &TrimState, snapped_frame: f32) {
    let delta = snapped_frame - trim.start_frame;

    match trim.edge {
        TrimEdge::Left => {
            let new_start = (trim.original_clip_start + delta).max(0.0);
            let max_start = trim.original_clip_start + trim.original_clip_dur - 1.0;
            let clamped_start = new_start.min(max_start);
            let start_delta = clamped_start - trim.original_clip_start;
            clip.start = clamped_start;
            clip.dur = (trim.original_clip_dur - start_delta).max(1.0);
        }
        TrimEdge::Right => {
            let new_dur = (trim.original_clip_dur + delta).max(1.0);
            clip.dur = new_dur;
        }
    }
}

/// Get the appropriate cursor icon for a trim edge.
pub fn trim_cursor(edge: TrimEdge) -> CursorIcon {
    match edge {
        TrimEdge::Left => CursorIcon::ResizeWest,
        TrimEdge::Right => CursorIcon::ResizeEast,
    }
}

/// State for clip dragging (moving clips).
#[derive(Debug, Clone)]
pub struct ClipDragState {
    pub clip_id: usize,
    pub offset_frame: f32,
    pub original_track: usize,
    pub snap_indicator: Option<f32>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timeline::ClipKind;
    use egui::Color32;

    fn make_test_clip() -> TimelineClip {
        TimelineClip {
            id: 0,
            name: "test".into(),
            color: Color32::BLUE,
            start: 100.0,
            dur: 50.0,
            track: 0,
            clip_type: ClipKind::Video,
        }
    }

    #[test]
    fn test_hit_test_left_handle() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::Vec2::new(200.0, 30.0));
        let result = hit_test_trim_handle(rect, Pos2::new(102.0, 65.0), 6.0);
        assert_eq!(result, Some(TrimEdge::Left));
    }

    #[test]
    fn test_hit_test_right_handle() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::Vec2::new(200.0, 30.0));
        let result = hit_test_trim_handle(rect, Pos2::new(298.0, 65.0), 6.0);
        assert_eq!(result, Some(TrimEdge::Right));
    }

    #[test]
    fn test_hit_test_body() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::Vec2::new(200.0, 30.0));
        let result = hit_test_trim_handle(rect, Pos2::new(200.0, 65.0), 6.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_hit_test_outside() {
        let rect = Rect::from_min_size(Pos2::new(100.0, 50.0), egui::Vec2::new(200.0, 30.0));
        let result = hit_test_trim_handle(rect, Pos2::new(50.0, 65.0), 6.0);
        assert!(result.is_none());
    }

    #[test]
    fn test_apply_trim_right() {
        let mut clip = make_test_clip();
        let trim = TrimState {
            active: true,
            clip_id: 0,
            edge: TrimEdge::Right,
            mode: TrimMode::Ripple,
            start_frame: 150.0,
            current_frame: 170.0,
            original_clip_start: 100.0,
            original_clip_dur: 50.0,
        };
        apply_trim(&mut clip, &trim, 170.0);
        assert!((clip.dur - 70.0).abs() < 0.01);
        assert!((clip.start - 100.0).abs() < 0.01); // Start unchanged
    }

    #[test]
    fn test_apply_trim_left() {
        let mut clip = make_test_clip();
        let trim = TrimState {
            active: true,
            clip_id: 0,
            edge: TrimEdge::Left,
            mode: TrimMode::Ripple,
            start_frame: 100.0,
            current_frame: 120.0,
            original_clip_start: 100.0,
            original_clip_dur: 50.0,
        };
        apply_trim(&mut clip, &trim, 120.0);
        assert!((clip.start - 120.0).abs() < 0.01);
        assert!((clip.dur - 30.0).abs() < 0.01);
    }

    #[test]
    fn test_apply_trim_left_clamp() {
        let mut clip = make_test_clip();
        let trim = TrimState {
            active: true,
            clip_id: 0,
            edge: TrimEdge::Left,
            mode: TrimMode::Ripple,
            start_frame: 100.0,
            current_frame: 200.0, // Beyond end of clip
            original_clip_start: 100.0,
            original_clip_dur: 50.0,
        };
        apply_trim(&mut clip, &trim, 200.0);
        // Should clamp: can't trim past the end
        assert!(clip.dur >= 1.0);
    }

    #[test]
    fn test_trim_cursor() {
        assert_eq!(trim_cursor(TrimEdge::Left), CursorIcon::ResizeWest);
        assert_eq!(trim_cursor(TrimEdge::Right), CursorIcon::ResizeEast);
    }

    #[test]
    fn test_trim_state_delta() {
        let clip = make_test_clip();
        let mut trim = TrimState::new(&clip, TrimEdge::Right, 150.0);
        trim.current_frame = 170.0;
        assert!((trim.delta() - 20.0).abs() < 0.01);
    }
}
