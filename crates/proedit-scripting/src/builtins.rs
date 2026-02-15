//! Built-in math functions for expressions (AE-compatible).

use std::collections::HashMap;

use crate::context::ExpressionContext;

/// Register built-in variable values from the context.
/// Used as a fallback when boa is not available.
pub fn register_builtins(ctx: &ExpressionContext) -> HashMap<String, f64> {
    let mut vars = HashMap::new();
    vars.insert("time".into(), ctx.time);
    vars.insert("frame".into(), ctx.frame as f64);
    vars.insert("fps".into(), ctx.fps);
    vars.insert("comp_duration".into(), ctx.comp_duration);
    vars.insert("value".into(), ctx.value);
    vars.insert("comp_width".into(), ctx.comp_width);
    vars.insert("comp_height".into(), ctx.comp_height);
    vars.insert("PI".into(), std::f64::consts::PI);
    vars.insert("E".into(), std::f64::consts::E);
    vars
}

/// linear(t, t_min, t_max, val_min, val_max) — linear interpolation.
pub fn linear(t: f64, t_min: f64, t_max: f64, val_min: f64, val_max: f64) -> f64 {
    if t_max <= t_min {
        return val_min;
    }
    let ratio = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    val_min + (val_max - val_min) * ratio
}

/// ease(t, t_min, t_max, val_min, val_max) — smooth ease in/out.
pub fn ease(t: f64, t_min: f64, t_max: f64, val_min: f64, val_max: f64) -> f64 {
    let ratio = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    // Smoothstep
    let s = ratio * ratio * (3.0 - 2.0 * ratio);
    val_min + (val_max - val_min) * s
}

/// easeIn(t, t_min, t_max, val_min, val_max) — accelerating ease.
pub fn ease_in(t: f64, t_min: f64, t_max: f64, val_min: f64, val_max: f64) -> f64 {
    let ratio = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    let s = ratio * ratio;
    val_min + (val_max - val_min) * s
}

/// easeOut(t, t_min, t_max, val_min, val_max) — decelerating ease.
pub fn ease_out(t: f64, t_min: f64, t_max: f64, val_min: f64, val_max: f64) -> f64 {
    let ratio = ((t - t_min) / (t_max - t_min)).clamp(0.0, 1.0);
    let s = ratio * (2.0 - ratio);
    val_min + (val_max - val_min) * s
}

/// wiggle(freq, amp) — seeded pseudo-random noise.
pub fn wiggle(time: f64, freq: f64, amp: f64) -> f64 {
    // Simple seeded noise using sine combination
    let t = time * freq;
    let tau = std::f64::consts::TAU;
    let noise = (t * tau).sin() * 0.5
        + (t * 2.3 * tau + 1.7).sin() * 0.25
        + (t * 4.7 * tau + 3.1).sin() * 0.125;
    noise * amp
}

/// Clamp a value to a range.
pub fn clamp(value: f64, min: f64, max: f64) -> f64 {
    value.clamp(min, max)
}

/// Linear interpolation between two values.
pub fn lerp(a: f64, b: f64, t: f64) -> f64 {
    a + (b - a) * t
}

/// Convert degrees to radians.
pub fn degrees_to_radians(deg: f64) -> f64 {
    deg * std::f64::consts::PI / 180.0
}

/// Convert radians to degrees.
pub fn radians_to_degrees(rad: f64) -> f64 {
    rad * 180.0 / std::f64::consts::PI
}

/// Seeded random number in [0, 1).
pub fn random_seeded(seed: u64) -> f64 {
    // Simple hash-based PRNG
    let x = seed
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    (x >> 33) as f64 / (1u64 << 31) as f64
}

/// Random number in [min, max).
pub fn random_range(seed: u64, min: f64, max: f64) -> f64 {
    min + random_seeded(seed) * (max - min)
}

/// Length of a 2D vector.
pub fn length(x: f64, y: f64) -> f64 {
    (x * x + y * y).sqrt()
}

/// Normalize a 2D vector. Returns (0, 0) if length is near zero.
pub fn normalize(x: f64, y: f64) -> (f64, f64) {
    let len = length(x, y);
    if len < 1e-10 {
        (0.0, 0.0)
    } else {
        (x / len, y / len)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_linear() {
        assert!((linear(0.5, 0.0, 1.0, 0.0, 100.0) - 50.0).abs() < 0.01);
        assert!((linear(0.0, 0.0, 1.0, 0.0, 100.0) - 0.0).abs() < 0.01);
        assert!((linear(1.0, 0.0, 1.0, 0.0, 100.0) - 100.0).abs() < 0.01);
        // Clamped outside range
        assert!((linear(-1.0, 0.0, 1.0, 0.0, 100.0) - 0.0).abs() < 0.01);
        assert!((linear(2.0, 0.0, 1.0, 0.0, 100.0) - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_ease() {
        // At endpoints
        assert!((ease(0.0, 0.0, 1.0, 0.0, 100.0) - 0.0).abs() < 0.01);
        assert!((ease(1.0, 0.0, 1.0, 0.0, 100.0) - 100.0).abs() < 0.01);
        // Midpoint of smoothstep is 50
        assert!((ease(0.5, 0.0, 1.0, 0.0, 100.0) - 50.0).abs() < 1.0);
    }

    #[test]
    fn test_ease_in_out() {
        // ease_in starts slow
        let ei = ease_in(0.25, 0.0, 1.0, 0.0, 100.0);
        let lin = linear(0.25, 0.0, 1.0, 0.0, 100.0);
        assert!(ei < lin, "ease_in should be below linear at t=0.25");

        // ease_out starts fast
        let eo = ease_out(0.25, 0.0, 1.0, 0.0, 100.0);
        assert!(eo > lin, "ease_out should be above linear at t=0.25");
    }

    #[test]
    fn test_wiggle_varies() {
        let v1 = wiggle(0.0, 2.0, 50.0);
        let v2 = wiggle(0.25, 2.0, 50.0);
        // Different times should produce different values
        assert!((v1 - v2).abs() > 0.01);
        // Amplitude should be bounded
        assert!(v1.abs() <= 50.0);
    }

    #[test]
    fn test_clamp_fn() {
        assert_eq!(clamp(5.0, 0.0, 10.0), 5.0);
        assert_eq!(clamp(-1.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp(15.0, 0.0, 10.0), 10.0);
    }

    #[test]
    fn test_lerp_fn() {
        assert!((lerp(0.0, 100.0, 0.5) - 50.0).abs() < 0.01);
        assert!((lerp(10.0, 20.0, 0.0) - 10.0).abs() < 0.01);
    }

    #[test]
    fn test_angle_conversion() {
        assert!((degrees_to_radians(180.0) - std::f64::consts::PI).abs() < 0.001);
        assert!((radians_to_degrees(std::f64::consts::PI) - 180.0).abs() < 0.001);
    }

    #[test]
    fn test_length_normalize() {
        assert!((length(3.0, 4.0) - 5.0).abs() < 0.001);
        let (nx, ny) = normalize(3.0, 4.0);
        assert!((length(nx, ny) - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_register_builtins() {
        let ctx = ExpressionContext::default();
        let vars = register_builtins(&ctx);
        assert_eq!(*vars.get("fps").unwrap(), 24.0);
        assert!(vars.contains_key("PI"));
    }
}
