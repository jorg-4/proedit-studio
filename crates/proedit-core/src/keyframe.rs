//! Keyframe animation system with Bézier interpolation.
//!
//! Supports easing via cubic Bézier curves with Newton-Raphson evaluation
//! for converting parameter t along the curve to the correct time mapping.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::time::RationalTime;

// ── Easing curves ───────────────────────────────────────────────

/// Cubic Bézier control points for easing (x1, y1, x2, y2).
/// The curve goes from (0,0) to (1,1).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct CubicBezier {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl CubicBezier {
    pub const fn new(x1: f64, y1: f64, x2: f64, y2: f64) -> Self {
        Self { x1, y1, x2, y2 }
    }

    /// Evaluate the X coordinate of the Bézier curve at parameter t.
    fn sample_x(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        3.0 * mt2 * t * self.x1 + 3.0 * mt * t2 * self.x2 + t3
    }

    /// Evaluate the Y coordinate of the Bézier curve at parameter t.
    fn sample_y(&self, t: f64) -> f64 {
        let t2 = t * t;
        let t3 = t2 * t;
        let mt = 1.0 - t;
        let mt2 = mt * mt;
        3.0 * mt2 * t * self.y1 + 3.0 * mt * t2 * self.y2 + t3
    }

    /// Derivative of X with respect to t.
    fn sample_dx(&self, t: f64) -> f64 {
        let mt = 1.0 - t;
        3.0 * mt * mt * self.x1 + 6.0 * mt * t * (self.x2 - self.x1) + 3.0 * t * t * (1.0 - self.x2)
    }

    /// Solve for the parameter t given an x value using Newton-Raphson.
    /// Returns the y value at that x.
    pub fn evaluate(&self, x: f64) -> f64 {
        if x <= 0.0 {
            return 0.0;
        }
        if x >= 1.0 {
            return 1.0;
        }

        // Newton-Raphson: find t such that sample_x(t) = x
        let mut t = x; // initial guess

        for _ in 0..8 {
            let x_est = self.sample_x(t) - x;
            let dx = self.sample_dx(t);
            if dx.abs() < 1e-12 {
                break;
            }
            t -= x_est / dx;
            t = t.clamp(0.0, 1.0);
            if x_est.abs() < 1e-10 {
                break;
            }
        }

        self.sample_y(t)
    }

    // Common easing presets
    pub const LINEAR: Self = Self::new(0.0, 0.0, 1.0, 1.0);
    pub const EASE: Self = Self::new(0.25, 0.1, 0.25, 1.0);
    pub const EASE_IN: Self = Self::new(0.42, 0.0, 1.0, 1.0);
    pub const EASE_OUT: Self = Self::new(0.0, 0.0, 0.58, 1.0);
    pub const EASE_IN_OUT: Self = Self::new(0.42, 0.0, 0.58, 1.0);
}

/// How to interpolate between keyframes.
#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub enum EasingCurve {
    /// No interpolation — hold the value until the next keyframe.
    Hold,
    /// Linear interpolation.
    #[default]
    Linear,
    /// Cubic Bézier easing.
    Bezier(CubicBezier),
}

// ── Keyframe ────────────────────────────────────────────────────

/// A single keyframe at a point in time.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Keyframe {
    /// Time of this keyframe (relative to clip or effect start).
    pub time: RationalTime,
    /// Value at this keyframe.
    pub value: f64,
    /// Easing curve to use when interpolating TO the next keyframe.
    pub easing: EasingCurve,
}

impl Keyframe {
    /// Create a new keyframe.
    pub fn new(time: RationalTime, value: f64) -> Self {
        Self {
            time,
            value,
            easing: EasingCurve::Linear,
        }
    }

    /// Create a keyframe with a specific easing curve.
    pub fn with_easing(time: RationalTime, value: f64, easing: EasingCurve) -> Self {
        Self {
            time,
            value,
            easing,
        }
    }
}

// ── Keyframe track ──────────────────────────────────────────────

/// A track of keyframes for a single animated parameter.
///
/// Keyframes are kept sorted by time. Interpolation between keyframes
/// uses the easing curve of the earlier keyframe.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyframeTrack {
    /// Human-readable parameter name.
    pub name: String,
    /// Sorted list of keyframes.
    keyframes: Vec<Keyframe>,
}

impl KeyframeTrack {
    /// Create a new empty keyframe track.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            keyframes: Vec::new(),
        }
    }

    /// Create a track with a constant value (single keyframe at t=0).
    pub fn constant(name: impl Into<String>, value: f64) -> Self {
        let mut track = Self::new(name);
        track.set(RationalTime::ZERO, value, EasingCurve::Hold);
        track
    }

    /// Insert or update a keyframe. Maintains sorted order.
    pub fn set(&mut self, time: RationalTime, value: f64, easing: EasingCurve) {
        // Check if keyframe at this exact time exists
        if let Some(kf) = self.keyframes.iter_mut().find(|kf| kf.time == time) {
            kf.value = value;
            kf.easing = easing;
            return;
        }
        // Insert in sorted position
        let pos = self
            .keyframes
            .binary_search_by(|kf| kf.time.cmp(&time))
            .unwrap_or_else(|e| e);
        self.keyframes
            .insert(pos, Keyframe::with_easing(time, value, easing));
    }

    /// Remove the keyframe at the given time.
    pub fn remove(&mut self, time: RationalTime) -> bool {
        if let Some(pos) = self.keyframes.iter().position(|kf| kf.time == time) {
            self.keyframes.remove(pos);
            true
        } else {
            false
        }
    }

    /// Evaluate the track at a given time.
    pub fn evaluate(&self, time: RationalTime) -> f64 {
        match self.keyframes.len() {
            0 => 0.0,
            1 => self.keyframes[0].value,
            _ => {
                // Before first keyframe
                if time <= self.keyframes[0].time {
                    return self.keyframes[0].value;
                }
                // After last keyframe
                let last = self.keyframes.last().unwrap();
                if time >= last.time {
                    return last.value;
                }
                // Find the bracketing keyframes
                let idx = self
                    .keyframes
                    .partition_point(|kf| kf.time <= time)
                    .saturating_sub(1);
                let a = &self.keyframes[idx];
                let b = &self.keyframes[idx + 1];
                Self::interpolate(a, b, time)
            }
        }
    }

    /// Interpolate between two keyframes.
    fn interpolate(a: &Keyframe, b: &Keyframe, time: RationalTime) -> f64 {
        let t_start = a.time.to_seconds_f64();
        let t_end = b.time.to_seconds_f64();
        let span = t_end - t_start;
        if span <= 0.0 {
            return a.value;
        }

        let t = ((time.to_seconds_f64() - t_start) / span).clamp(0.0, 1.0);

        match a.easing {
            EasingCurve::Hold => a.value,
            EasingCurve::Linear => a.value + (b.value - a.value) * t,
            EasingCurve::Bezier(bezier) => {
                let eased_t = bezier.evaluate(t);
                a.value + (b.value - a.value) * eased_t
            }
        }
    }

    /// Get all keyframes (read-only).
    pub fn keyframes(&self) -> &[Keyframe] {
        &self.keyframes
    }

    /// Number of keyframes.
    pub fn len(&self) -> usize {
        self.keyframes.len()
    }

    /// Whether the track has no keyframes.
    pub fn is_empty(&self) -> bool {
        self.keyframes.is_empty()
    }

    /// Whether this track is animated (has more than one keyframe).
    pub fn is_animated(&self) -> bool {
        self.keyframes.len() > 1
    }

    /// Get the time range spanned by keyframes.
    pub fn time_range(&self) -> Option<crate::time::TimeRange> {
        if self.keyframes.is_empty() {
            return None;
        }
        let start = self.keyframes.first().unwrap().time;
        let end = self.keyframes.last().unwrap().time;
        Some(crate::time::TimeRange::from_start_end(start, end))
    }
}

impl fmt::Display for KeyframeTrack {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "KeyframeTrack({}, {} keyframes)",
            self.name,
            self.keyframes.len()
        )
    }
}

// ── Tests ───────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::time::RationalTime;

    #[test]
    fn test_linear_interpolation() {
        let mut track = KeyframeTrack::new("opacity");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Linear);
        track.set(RationalTime::new(1, 1), 1.0, EasingCurve::Linear);

        assert!((track.evaluate(RationalTime::new(0, 1)) - 0.0).abs() < 0.001);
        assert!((track.evaluate(RationalTime::new(1, 2)) - 0.5).abs() < 0.001);
        assert!((track.evaluate(RationalTime::new(1, 1)) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_hold_interpolation() {
        let mut track = KeyframeTrack::new("visible");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Hold);
        track.set(RationalTime::new(1, 1), 1.0, EasingCurve::Hold);

        // Hold should keep the first value until the next keyframe
        assert!((track.evaluate(RationalTime::new(1, 2)) - 0.0).abs() < 0.001);
        assert!((track.evaluate(RationalTime::new(1, 1)) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_bezier_ease_in_out() {
        let mut track = KeyframeTrack::new("position");
        track.set(
            RationalTime::new(0, 1),
            0.0,
            EasingCurve::Bezier(CubicBezier::EASE_IN_OUT),
        );
        track.set(RationalTime::new(1, 1), 100.0, EasingCurve::Linear);

        let mid = track.evaluate(RationalTime::new(1, 2));
        // Ease-in-out should be near 50 at the midpoint (symmetric curve)
        assert!((mid - 50.0).abs() < 5.0);

        // Should start slow (ease-in)
        let early = track.evaluate(RationalTime::new(1, 10));
        assert!(early < 10.0); // slower than linear at t=0.1
    }

    #[test]
    fn test_cubic_bezier_endpoints() {
        let bezier = CubicBezier::EASE;
        assert!((bezier.evaluate(0.0) - 0.0).abs() < 0.001);
        assert!((bezier.evaluate(1.0) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_cubic_bezier_linear() {
        let bezier = CubicBezier::LINEAR;
        for i in 0..=10 {
            let x = i as f64 / 10.0;
            let y = bezier.evaluate(x);
            assert!(
                (y - x).abs() < 0.001,
                "linear bezier at x={}: got y={}",
                x,
                y
            );
        }
    }

    #[test]
    fn test_keyframe_track_clamp_edges() {
        let mut track = KeyframeTrack::new("test");
        track.set(RationalTime::new(1, 1), 10.0, EasingCurve::Linear);
        track.set(RationalTime::new(3, 1), 30.0, EasingCurve::Linear);

        // Before first keyframe → first value
        assert!((track.evaluate(RationalTime::new(0, 1)) - 10.0).abs() < 0.001);
        // After last keyframe → last value
        assert!((track.evaluate(RationalTime::new(5, 1)) - 30.0).abs() < 0.001);
    }

    #[test]
    fn test_keyframe_remove() {
        let mut track = KeyframeTrack::new("test");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Linear);
        track.set(RationalTime::new(1, 1), 1.0, EasingCurve::Linear);
        assert_eq!(track.len(), 2);

        assert!(track.remove(RationalTime::new(1, 1)));
        assert_eq!(track.len(), 1);

        assert!(!track.remove(RationalTime::new(5, 1)));
    }

    #[test]
    fn test_keyframe_overwrite() {
        let mut track = KeyframeTrack::new("test");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Linear);
        track.set(RationalTime::new(0, 1), 5.0, EasingCurve::Hold);
        assert_eq!(track.len(), 1);
        assert!((track.evaluate(RationalTime::ZERO) - 5.0).abs() < 0.001);
    }

    #[test]
    fn test_constant_track() {
        let track = KeyframeTrack::constant("scale", 1.5);
        assert!(!track.is_animated());
        assert!((track.evaluate(RationalTime::new(100, 1)) - 1.5).abs() < 0.001);
    }

    #[test]
    fn test_multiple_keyframes() {
        let mut track = KeyframeTrack::new("position");
        track.set(RationalTime::new(0, 1), 0.0, EasingCurve::Linear);
        track.set(RationalTime::new(1, 1), 100.0, EasingCurve::Linear);
        track.set(RationalTime::new(2, 1), 50.0, EasingCurve::Linear);

        assert!((track.evaluate(RationalTime::new(1, 2)) - 50.0).abs() < 0.001);
        assert!((track.evaluate(RationalTime::new(1, 1)) - 100.0).abs() < 0.001);
        assert!((track.evaluate(RationalTime::new(3, 2)) - 75.0).abs() < 0.001);
    }
}
