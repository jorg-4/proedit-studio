//! Time representation for frame-accurate editing
//!
//! Uses rational numbers to avoid floating-point accumulation errors.
//! All time values are represented as numerator/denominator pairs.

use num_rational::Rational64;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::ops::{Add, Div, Mul, Neg, Sub};

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

    /// Get the numerator of the internal rational.
    #[inline]
    pub fn numer(self) -> i64 {
        *self.value.numer()
    }

    /// Get the denominator of the internal rational.
    #[inline]
    pub fn denom(self) -> i64 {
        *self.value.denom()
    }

    /// Minimum of two times.
    #[inline]
    pub fn min(self, other: Self) -> Self {
        if self <= other {
            self
        } else {
            other
        }
    }

    /// Maximum of two times.
    #[inline]
    pub fn max(self, other: Self) -> Self {
        if self >= other {
            self
        } else {
            other
        }
    }

    /// Format as timecode string HH:MM:SS:FF at the given frame rate.
    pub fn to_timecode(self, rate: FrameRate) -> String {
        let total_frames = self.to_frames(rate).unsigned_abs();
        let negative = *self.value.numer() < 0;

        let fps = rate.nominal_fps();
        let frames = total_frames % fps as u64;
        let total_secs = total_frames / fps as u64;
        let seconds = total_secs % 60;
        let total_mins = total_secs / 60;
        let minutes = total_mins % 60;
        let hours = total_mins / 60;

        if negative {
            format!("-{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
        } else {
            format!("{:02}:{:02}:{:02}:{:02}", hours, minutes, seconds, frames)
        }
    }

    /// Format as drop-frame timecode (for 29.97/59.94 fps).
    /// Uses SMPTE drop-frame algorithm: drop frames 0 and 1 at the start
    /// of each minute, except every 10th minute.
    pub fn to_timecode_drop_frame(self, rate: FrameRate) -> String {
        let fps = rate.nominal_fps() as u64;
        // Drop-frame only valid for 29.97 and 59.94
        let d: u64 = match fps {
            30 if rate.denominator == 1001 => 2,
            60 if rate.denominator == 1001 => 4,
            _ => return self.to_timecode(rate), // fallback to non-drop
        };

        let n = self.to_frames(rate).unsigned_abs();
        let negative = *self.value.numer() < 0;

        // SMPTE drop-frame: compute which minute we're in, then add back drops
        let frames_per_10min = fps * 60 * 10 - d * 9;

        let ten_min_blocks = n / frames_per_10min;
        let remainder = n % frames_per_10min;

        // First minute of each 10-min block has no drops (fps*60 frames)
        let additional_minutes = if remainder < fps * 60 {
            0
        } else {
            1 + (remainder - fps * 60) / (fps * 60 - d)
        };

        let total_minutes = ten_min_blocks * 10 + additional_minutes;
        let drops = d * (total_minutes - total_minutes / 10);
        let display = n + drops;

        let frames = display % fps;
        let seconds = (display / fps) % 60;
        let minutes = (display / (fps * 60)) % 60;
        let hours = display / (fps * 60 * 60);

        if negative {
            format!("-{:02}:{:02}:{:02};{:02}", hours, minutes, seconds, frames)
        } else {
            format!("{:02}:{:02}:{:02};{:02}", hours, minutes, seconds, frames)
        }
    }

    /// Parse a timecode string "HH:MM:SS:FF" or "HH:MM:SS;FF" (drop-frame).
    pub fn from_timecode(tc: &str, rate: FrameRate) -> Option<Self> {
        let negative = tc.starts_with('-');
        let tc = tc.trim_start_matches('-');

        let is_drop_frame = tc.contains(';');
        let parts: Vec<&str> = tc.split([':', ';']).collect();
        if parts.len() != 4 {
            return None;
        }

        let hours: u64 = parts[0].parse().ok()?;
        let minutes: u64 = parts[1].parse().ok()?;
        let seconds: u64 = parts[2].parse().ok()?;
        let frames: u64 = parts[3].parse().ok()?;

        let fps = rate.nominal_fps() as u64;

        let total_frames = if is_drop_frame {
            let drop_count = match fps {
                30 if rate.denominator == 1001 => 2u64,
                60 if rate.denominator == 1001 => 4u64,
                _ => 0,
            };
            let total_minutes = hours * 60 + minutes;
            let drops = drop_count * (total_minutes - total_minutes / 10);
            hours * 3600 * fps + minutes * 60 * fps + seconds * fps + frames - drops
        } else {
            hours * 3600 * fps + minutes * 60 * fps + seconds * fps + frames
        };

        let time = Self::from_frames(total_frames as i64, rate);
        if negative {
            Some(-time)
        } else {
            Some(time)
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

impl Neg for RationalTime {
    type Output = Self;
    fn neg(self) -> Self {
        Self { value: -self.value }
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

    /// Nominal (integer) fps — rounds up for drop-frame rates.
    /// 23.976 → 24, 29.97 → 30, 59.94 → 60, etc.
    #[inline]
    pub fn nominal_fps(self) -> u32 {
        self.numerator.div_ceil(self.denominator)
    }

    /// Whether this frame rate uses drop-frame timecode.
    #[inline]
    pub fn is_drop_frame(self) -> bool {
        self.denominator == 1001
            && (self.numerator == 30000 || self.numerator == 60000 || self.numerator == 24000)
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

    /// Union of two ranges (bounding range).
    pub fn union(self, other: Self) -> Self {
        let start = self.start.min(other.start);
        let end = self.end().max(other.end());
        Self::from_start_end(start, end)
    }

    /// Check if this range has zero duration.
    #[inline]
    pub fn is_empty(self) -> bool {
        self.duration.is_zero()
    }

    /// Offset this range by a time delta.
    #[inline]
    pub fn offset(self, delta: RationalTime) -> Self {
        Self {
            start: self.start + delta,
            duration: self.duration,
        }
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

    #[test]
    fn test_negation() {
        let t = RationalTime::new(5, 1);
        let neg = -t;
        assert_eq!(neg.to_seconds_f64(), -5.0);
        assert_eq!(neg.abs(), t);
    }

    #[test]
    fn test_timecode_24fps() {
        let rate = FrameRate::FPS_24;
        // 1 hour, 2 minutes, 3 seconds, 4 frames
        let frames = 3600 * 24 + 2 * 60 * 24 + 3 * 24 + 4;
        let time = RationalTime::from_frames(frames, rate);
        assert_eq!(time.to_timecode(rate), "01:02:03:04");
    }

    #[test]
    fn test_timecode_roundtrip_24fps() {
        let rate = FrameRate::FPS_24;
        let tc = "00:01:30:12";
        let time = RationalTime::from_timecode(tc, rate).unwrap();
        assert_eq!(time.to_timecode(rate), tc);
    }

    #[test]
    fn test_timecode_drop_frame_29_97() {
        let rate = FrameRate::FPS_29_97;
        assert!(rate.is_drop_frame());
        assert_eq!(rate.nominal_fps(), 30);

        // Frame 0 → 00:00:00;00
        let time = RationalTime::from_frames(0, rate);
        assert_eq!(time.to_timecode_drop_frame(rate), "00:00:00;00");
    }

    #[test]
    fn test_from_timecode_drop_frame() {
        let rate = FrameRate::FPS_29_97;
        let tc = "00:01:00;02";
        let time = RationalTime::from_timecode(tc, rate).unwrap();
        // At 29.97, after drop-frame adjustments, verify round-trip
        let back = time.to_timecode_drop_frame(rate);
        assert_eq!(back, tc);
    }

    #[test]
    fn test_nominal_fps() {
        assert_eq!(FrameRate::FPS_24.nominal_fps(), 24);
        assert_eq!(FrameRate::FPS_25.nominal_fps(), 25);
        assert_eq!(FrameRate::FPS_29_97.nominal_fps(), 30);
        assert_eq!(FrameRate::FPS_30.nominal_fps(), 30);
        assert_eq!(FrameRate::FPS_59_94.nominal_fps(), 60);
        assert_eq!(FrameRate::FPS_60.nominal_fps(), 60);
    }

    #[test]
    fn test_time_range_union() {
        let a = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(5, 1));
        let b = TimeRange::new(RationalTime::new(3, 1), RationalTime::new(7, 1));
        let u = a.union(b);
        assert_eq!(u.start, RationalTime::new(0, 1));
        assert_eq!(u.end(), RationalTime::new(10, 1));
    }

    #[test]
    fn test_time_range_offset() {
        let r = TimeRange::new(RationalTime::new(5, 1), RationalTime::new(10, 1));
        let shifted = r.offset(RationalTime::new(3, 1));
        assert_eq!(shifted.start, RationalTime::new(8, 1));
        assert_eq!(shifted.duration, RationalTime::new(10, 1));
    }

    #[test]
    fn test_time_range_empty() {
        assert!(TimeRange::EMPTY.is_empty());
        let r = TimeRange::new(RationalTime::new(0, 1), RationalTime::new(1, 1));
        assert!(!r.is_empty());
    }
}

#[cfg(test)]
mod proptests {
    use super::*;
    use proptest::prelude::*;

    fn arb_frame_rate() -> impl Strategy<Value = FrameRate> {
        prop_oneof![
            Just(FrameRate::FPS_24),
            Just(FrameRate::FPS_25),
            Just(FrameRate::FPS_29_97),
            Just(FrameRate::FPS_30),
            Just(FrameRate::FPS_59_94),
            Just(FrameRate::FPS_60),
        ]
    }

    proptest! {
        /// frames → time → frames round-trips exactly.
        #[test]
        fn prop_frame_roundtrip(frames in 0i64..1_000_000, rate in arb_frame_rate()) {
            let time = RationalTime::from_frames(frames, rate);
            prop_assert_eq!(time.to_frames(rate), frames);
        }

        /// Addition is commutative.
        #[test]
        fn prop_add_commutative(
            a_num in -1_000_000i64..1_000_000,
            b_num in -1_000_000i64..1_000_000,
        ) {
            let a = RationalTime::new(a_num, 1000);
            let b = RationalTime::new(b_num, 1000);
            prop_assert_eq!(a + b, b + a);
        }

        /// (a + b) - b == a
        #[test]
        fn prop_add_sub_inverse(
            a_num in -1_000_000i64..1_000_000,
            b_num in -1_000_000i64..1_000_000,
        ) {
            let a = RationalTime::new(a_num, 1000);
            let b = RationalTime::new(b_num, 1000);
            prop_assert_eq!((a + b) - b, a);
        }

        /// abs(t) >= 0
        #[test]
        fn prop_abs_non_negative(num in -1_000_000i64..1_000_000) {
            let t = RationalTime::new(num, 1000);
            prop_assert!(t.abs() >= RationalTime::ZERO);
        }

        /// Timecode round-trip for non-drop rates.
        #[test]
        fn prop_timecode_roundtrip(
            frames in 0i64..100_000,
            rate in prop_oneof![
                Just(FrameRate::FPS_24),
                Just(FrameRate::FPS_25),
                Just(FrameRate::FPS_30),
                Just(FrameRate::FPS_60),
            ],
        ) {
            let time = RationalTime::from_frames(frames, rate);
            let tc = time.to_timecode(rate);
            let parsed = RationalTime::from_timecode(&tc, rate).unwrap();
            prop_assert_eq!(parsed.to_frames(rate), frames);
        }

        /// TimeRange contains its start but not its end.
        #[test]
        fn prop_range_boundaries(
            start in 0i64..10_000,
            dur in 1i64..10_000,
        ) {
            let r = TimeRange::new(
                RationalTime::new(start, 100),
                RationalTime::new(dur, 100),
            );
            prop_assert!(r.contains(r.start));
            prop_assert!(!r.contains(r.end()));
        }

        /// A range always overlaps with itself (if non-empty).
        #[test]
        fn prop_self_overlap(
            start in 0i64..10_000,
            dur in 1i64..10_000,
        ) {
            let r = TimeRange::new(
                RationalTime::new(start, 100),
                RationalTime::new(dur, 100),
            );
            prop_assert!(r.overlaps(r));
        }

        /// intersection(a, a) == a (for non-empty ranges).
        #[test]
        fn prop_self_intersection(
            start in 0i64..10_000,
            dur in 1i64..10_000,
        ) {
            let r = TimeRange::new(
                RationalTime::new(start, 100),
                RationalTime::new(dur, 100),
            );
            let i = r.intersection(r).unwrap();
            prop_assert_eq!(i.start, r.start);
            prop_assert_eq!(i.duration, r.duration);
        }
    }
}
