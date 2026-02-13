//! Animation helpers for smooth UI transitions.

/// A float that smoothly interpolates toward a target value.
#[derive(Debug, Clone)]
pub struct AnimFloat {
    pub current: f32,
    pub target: f32,
    /// Speed factor (higher = faster convergence).
    pub speed: f32,
}

impl AnimFloat {
    /// Create a new animated float at `value` with a given `speed`.
    pub fn new(value: f32, speed: f32) -> Self {
        Self {
            current: value,
            target: value,
            speed,
        }
    }

    /// Advance the animation by `dt` seconds and return the current value.
    pub fn tick(&mut self, dt: f32) -> f32 {
        self.current += (self.target - self.current) * (1.0 - (-self.speed * dt).exp());
        self.current
    }

    /// Set a new target.
    pub fn set_target(&mut self, target: f32) {
        self.target = target;
    }

    /// Snap to target immediately.
    pub fn snap(&mut self) {
        self.current = self.target;
    }

    /// Check if the animation has essentially converged.
    pub fn done(&self) -> bool {
        (self.current - self.target).abs() < 0.001
    }
}

impl Default for AnimFloat {
    fn default() -> Self {
        Self::new(0.0, 10.0)
    }
}

/// Pulse animation (opacity 1 → 0.35 → 1, period in seconds).
pub fn pulse(time: f64, period: f64) -> f32 {
    let t = (time % period) / period;
    let v = (t * std::f64::consts::TAU).cos();
    // Map cos [-1, 1] → [0.35, 1.0]
    (0.675 + 0.325 * v) as f32
}
