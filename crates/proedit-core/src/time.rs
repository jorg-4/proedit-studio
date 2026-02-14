//! Time representation for frame-accurate editing
//!
//! Uses rational numbers to avoid floating-point accumulation errors.
//! All time values are represented as numerator/denominator pairs.

use num_rational::Rational64;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Sub};

/// A rational time value representing a point in time.
/// Uses rational arithmetic to maintain frame-accuracy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RationalTime {
    /// Time value as a rational number (seconds)
    value: Rational64,
}

impl RationalTime {
    /// Create a new RationalTime from numerator and denominator.
    /// The time is `numerator / denominator` seconds.
    #[inline]
    pub fn new(numerator: i64, denominator: i64) -> Self {
        Self {
            value: Rational64::new(numerator, denominator),
        }
    }

    /// Create a RationalTime from a frame number and frame rate.
    #[inline]
    pub fn from_frames(frames: i64, rate: FrameRate) -> Self {
        Self {
            value: Rational64::new(frames * rate.denominator as i64, rate.numerator as i64),
        }
    }

    /// Create a RationalTime from seconds as a float.
    /// Note: May introduce small precision errors.
    pub fn from_seconds_f64(seconds: f64) -> Self {
        // Use a high denominator for reasonable precision
        const PRECISION: i64 = 1_000_000;
        Self {
            value: Rational64::new((seconds * PRECISION as f64).round() as i64, PRECISION),
        }
    }

    /// Convert to seconds as f64.
    #[inline]
    pub fn to_seconds_f64(self) -> f64 {
        *self.value.numer() as f64 / *self.value.denom() as f64
    }

    /// Convert to frame number at the given frame rate.
    #[inline]
    pub fn to_frames(self, rate: FrameRate) -> i64 {
        let frames_rational =
            self.value * Rational64::new(rate.numerator as i64, rate.denominator as i64);
        // Floor to get the frame number
        *frames_rational.numer() / *frames_rational.denom()
    }

    /// Zero time constant.
    pub const ZERO: Self = Self {
        value: Rational64::new_raw(0, 1),
    };

    /// Check if this time is zero.
    #[inline]
    pub fn is_zero(self) -> bool {
        *self.value.numer() == 0
    }

    /// Get the absolute value of this time.
    #[inline]
    pub fn abs(self) -> Self {
        if *self.value.numer() < 0 {
            Self { value: -self.value }
        } else {
            self
        }
    }
}

impl Default for RationalTime {
    fn default() -> Self {
        Self::ZERO
    }
}

impl Add for RationalTime {
    type Output = Self;
    fn add(self, rhs: Self) -> Self {
        Self {
            value: self.value + rhs.value,
        }
    }
}

impl Sub for RationalTime {
    type Output = Self;
    fn sub(self, rhs: Self) -> Self {
        Self {
            value: self.value - rhs.value,
        }
    }
}

impl Mul<i64> for RationalTime {
    type Output = Self;
    fn mul(self, rhs: i64) -> Self {
        Self {
            value: self.value * rhs,
        }
    }
}

impl Div<i64> for RationalTime {
    type Output = Self;
    fn div(self, rhs: i64) -> Self {
        Self {
            value: self.value / rhs,
        }
    }
}

impl fmt::Display for RationalTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:.3}s", self.to_seconds_f64())
    }
}

/// Frame rate as a rational number (e.g., 24000/1001 for 23.976 fps).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct FrameRate {
    /// Numerator (e.g., 24000)
    pub numerator: u32,
    /// Denominator (e.g., 1001)
    pub denominator: u32,
}

impl FrameRate {
    /// Create a new frame rate.
    #[inline]
    pub const fn new(numerator: u32, denominator: u32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Convert to frames per second as f64.
    #[inline]
    pub fn to_fps_f64(self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Duration of a single frame.
    #[inline]
    pub fn frame_duration(self) -> RationalTime {
        RationalTime::new(self.denominator as i64, self.numerator as i64)
    }

    /// Common frame rates
    pub const FPS_23_976: Self = Self::new(24000, 1001);
    pub const FPS_24: Self = Self::new(24, 1);
    pub const FPS_25: Self = Self::new(25, 1);
    pub const FPS_29_97: Self = Self::new(30000, 1001);
    pub const FPS_30: Self = Self::new(30, 1);
    pub const FPS_50: Self = Self::new(50, 1);
    pub const FPS_59_94: Self = Self::new(60000, 1001);
    pub const FPS_60: Self = Self::new(60, 1);
}

impl Default for FrameRate {
    fn default() -> Self {
        Self::FPS_24
    }
}

impl fmt::Display for FrameRate {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let fps = self.to_fps_f64();
        if (fps - fps.round()).abs() < 0.001 {
            write!(f, "{} fps", fps.round() as u32)
        } else {
            write!(f, "{:.3} fps", fps)
        }
    }
}

/// A time range with inclusive start and exclusive end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeRange {
    /// Start time (inclusive)
    pub start: RationalTime,
    /// Duration of the range
    pub duration: RationalTime,
}

impl TimeRange {
    /// Create a new time range from start and duration.
    #[inline]
    pub fn new(start: RationalTime, duration: RationalTime) -> Self {
        Self { start, duration }
    }

    /// Create a time range from start and end times.
    #[inline]
    pub fn from_start_end(start: RationalTime, end: RationalTime) -> Self {
        Self {
            start,
            duration: end - start,
        }
    }

    /// End time (exclusive).
    #[inline]
    pub fn end(self) -> RationalTime {
        self.start + self.duration
    }

    /// Check if a time is within this range.
    #[inline]
    pub fn contains(self, time: RationalTime) -> bool {
        time >= self.start && time < self.end()
    }

    /// Check if two ranges overlap.
    pub fn overlaps(self, other: Self) -> bool {
        self.start < other.end() && other.start < self.end()
    }

    /// Compute the intersection of two ranges, if any.
    pub fn intersection(self, other: Self) -> Option<Self> {
        if !self.overlaps(other) {
            return None;
        }
        let start = if self.start > other.start {
            self.start
        } else {
            other.start
        };
        let end = if self.end() < other.end() {
            self.end()
        } else {
            other.end()
        };
        Some(Self::from_start_end(start, end))
    }

    /// Empty range starting at zero.
    pub const EMPTY: Self = Self {
        start: RationalTime::ZERO,
        duration: RationalTime::ZERO,
    };
}

impl Default for TimeRange {
    fn default() -> Self {
        Self::EMPTY
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rational_time_frames() {
        let rate = FrameRate::FPS_24;
        let time = RationalTime::from_frames(48, rate);
        assert_eq!(time.to_seconds_f64(), 2.0);
        assert_eq!(time.to_frames(rate), 48);
    }

    #[test]
    fn test_frame_rate_23_976() {
        let rate = FrameRate::FPS_23_976;
        let fps = rate.to_fps_f64();
        assert!((fps - 23.976).abs() < 0.001);
    }

    #[test]
    fn test_time_range_overlap() {
        let a = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(10, 1));
        let b = TimeRange::new(RationalTime::new(5, 1), RationalTime::new(10, 1));
        assert!(a.overlaps(b));

        let intersection = a.intersection(b).unwrap();
        assert_eq!(intersection.start, RationalTime::new(5, 1));
        assert_eq!(intersection.duration, RationalTime::new(5, 1));
    }

    #[test]
    fn test_time_arithmetic() {
        let a = RationalTime::new(1, 2); // 0.5 seconds
        let b = RationalTime::new(1, 4); // 0.25 seconds
        let sum = a + b;
        assert_eq!(sum.to_seconds_f64(), 0.75);
    }
}
