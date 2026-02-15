//! Expression evaluation context.

use serde::{Deserialize, Serialize};

/// Context variables available to expressions.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpressionContext {
    /// Current time in seconds.
    pub time: f64,
    /// Current frame number.
    pub frame: i64,
    /// Composition frames per second.
    pub fps: f64,
    /// Total composition duration in seconds.
    pub comp_duration: f64,
    /// Current property value before expression.
    pub value: f64,
    /// Composition width in pixels.
    pub comp_width: f64,
    /// Composition height in pixels.
    pub comp_height: f64,
}

impl Default for ExpressionContext {
    fn default() -> Self {
        Self {
            time: 0.0,
            frame: 0,
            fps: 24.0,
            comp_duration: 10.0,
            value: 0.0,
            comp_width: 1920.0,
            comp_height: 1080.0,
        }
    }
}

impl ExpressionContext {
    /// Create a context for a specific frame.
    pub fn at_frame(frame: i64, fps: f64) -> Self {
        Self {
            time: frame as f64 / fps,
            frame,
            fps,
            ..Default::default()
        }
    }

    /// Set the current property value.
    pub fn with_value(mut self, value: f64) -> Self {
        self.value = value;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_context() {
        let ctx = ExpressionContext::default();
        assert_eq!(ctx.time, 0.0);
        assert_eq!(ctx.fps, 24.0);
    }

    #[test]
    fn test_at_frame() {
        let ctx = ExpressionContext::at_frame(48, 24.0);
        assert!((ctx.time - 2.0).abs() < 0.001);
        assert_eq!(ctx.frame, 48);
    }

    #[test]
    fn test_with_value() {
        let ctx = ExpressionContext::default().with_value(42.0);
        assert_eq!(ctx.value, 42.0);
    }
}
