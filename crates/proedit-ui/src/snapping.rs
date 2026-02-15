//! Snapping engine for timeline interactions.

use crate::timeline::{TimelineClip, TimelineState};

/// A point on the timeline that can be snapped to.
#[derive(Debug, Clone, Copy)]
pub struct SnapPoint {
    pub frame: f32,
    pub kind: SnapKind,
}

/// Kind of snap point.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnapKind {
    ClipEdge,
    Playhead,
    Marker,
    GridLine,
}

/// Engine for computing snap targets.
pub struct SnappingEngine {
    pub enabled: bool,
    /// Snap distance in pixels (will be divided by zoom).
    pub snap_distance_px: f32,
    /// Grid interval in frames (0 = disabled).
    pub grid_frames: f32,
}

impl SnappingEngine {
    pub fn new() -> Self {
        Self {
            enabled: true,
            snap_distance_px: 8.0,
            grid_frames: 0.0,
        }
    }

    /// Collect all snap points from the current timeline state.
    pub fn collect_snap_points(state: &TimelineState) -> Vec<SnapPoint> {
        let mut points = Vec::new();

        // Playhead
        points.push(SnapPoint {
            frame: state.playhead,
            kind: SnapKind::Playhead,
        });

        // Clip edges
        for clip in &state.clips {
            points.push(SnapPoint {
                frame: clip.start,
                kind: SnapKind::ClipEdge,
            });
            points.push(SnapPoint {
                frame: clip.start + clip.dur,
                kind: SnapKind::ClipEdge,
            });
        }

        // Markers
        for marker in &state.markers {
            points.push(SnapPoint {
                frame: marker.frame,
                kind: SnapKind::Marker,
            });
        }

        points
    }

    /// Find the closest snap point within snap distance.
    /// Returns the snapped frame value, or None if no snap found.
    /// `exclude_clip` allows ignoring a specific clip's edges (e.g., the clip being dragged).
    pub fn find_snap(
        &self,
        frame: f32,
        points: &[SnapPoint],
        zoom: f32,
        exclude_clip: Option<usize>,
    ) -> Option<f32> {
        if !self.enabled || zoom <= 0.0 {
            return None;
        }

        let snap_threshold = self.snap_distance_px / zoom;
        let _ = exclude_clip; // Reserved for filtering

        let mut best: Option<(f32, f32)> = None; // (snap_frame, distance)

        for sp in points {
            let dist = (sp.frame - frame).abs();
            if dist <= snap_threshold && (best.is_none() || dist < best.unwrap().1) {
                best = Some((sp.frame, dist));
            }
        }

        // Grid snapping
        if self.grid_frames > 0.0 {
            let grid_snap = (frame / self.grid_frames).round() * self.grid_frames;
            let dist = (grid_snap - frame).abs();
            if dist <= snap_threshold && (best.is_none() || dist < best.unwrap().1) {
                best = Some((grid_snap, dist));
            }
        }

        best.map(|(f, _)| f)
    }

    /// Snap a clip position, excluding its own edges from the snap points.
    pub fn snap_clip(&self, clip: &TimelineClip, new_start: f32, state: &TimelineState) -> f32 {
        if !self.enabled {
            return new_start;
        }

        let points = Self::collect_snap_points(state);
        // Filter out points from the clip itself
        let filtered: Vec<_> = points
            .into_iter()
            .filter(|p| {
                (p.frame - clip.start).abs() > 0.01
                    && (p.frame - clip.start - clip.dur).abs() > 0.01
            })
            .collect();

        // Try snapping left edge
        if let Some(snapped) = self.find_snap(new_start, &filtered, state.zoom, Some(clip.id)) {
            return snapped;
        }
        // Try snapping right edge
        let new_end = new_start + clip.dur;
        if let Some(snapped) = self.find_snap(new_end, &filtered, state.zoom, Some(clip.id)) {
            return snapped - clip.dur;
        }

        new_start
    }
}

impl Default for SnappingEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_snap_points() -> Vec<SnapPoint> {
        vec![
            SnapPoint {
                frame: 0.0,
                kind: SnapKind::ClipEdge,
            },
            SnapPoint {
                frame: 100.0,
                kind: SnapKind::ClipEdge,
            },
            SnapPoint {
                frame: 200.0,
                kind: SnapKind::Playhead,
            },
            SnapPoint {
                frame: 150.0,
                kind: SnapKind::Marker,
            },
        ]
    }

    #[test]
    fn test_find_snap_near_point() {
        let engine = SnappingEngine::new();
        let points = make_snap_points();
        // Frame 98, zoom 1.0 → should snap to 100.0
        let result = engine.find_snap(98.0, &points, 1.0, None);
        assert_eq!(result, Some(100.0));
    }

    #[test]
    fn test_find_snap_too_far() {
        let engine = SnappingEngine::new();
        let points = make_snap_points();
        // Frame 50, zoom 1.0 → nothing within 8px
        let result = engine.find_snap(50.0, &points, 1.0, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_find_snap_disabled() {
        let mut engine = SnappingEngine::new();
        engine.enabled = false;
        let points = make_snap_points();
        let result = engine.find_snap(100.0, &points, 1.0, None);
        assert!(result.is_none());
    }

    #[test]
    fn test_grid_snapping() {
        let mut engine = SnappingEngine::new();
        engine.grid_frames = 10.0;
        let result = engine.find_snap(52.0, &[], 1.0, None);
        assert_eq!(result, Some(50.0));
    }

    #[test]
    fn test_collect_snap_points() {
        let mut state = TimelineState {
            playhead: 50.0,
            ..Default::default()
        };
        state.clips.push(TimelineClip {
            id: 0,
            name: "test".into(),
            color: egui::Color32::RED,
            start: 10.0,
            dur: 30.0,
            track: 0,
            clip_type: crate::timeline::ClipKind::Video,
        });
        let points = SnappingEngine::collect_snap_points(&state);
        // Playhead + 2 clip edges
        assert!(points.len() >= 3);
    }
}
