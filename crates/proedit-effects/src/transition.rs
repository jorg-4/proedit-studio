//! Transition system for video editing.

use proedit_core::{EasingCurve, FrameRate, RationalTime};
use serde::{Deserialize, Serialize};

/// Trait for video transitions between two clips.
pub trait Transition: Send + Sync {
    /// Get the transition name.
    fn name(&self) -> &str;

    /// Render the transition between frame A and frame B.
    /// Progress goes from 0.0 (pure A) to 1.0 (pure B).
    /// Input frames are RGBA u8.
    fn render(&self, a: &[u8], b: &[u8], w: u32, h: u32, progress: f32) -> Vec<u8>;
}

/// Parameters for a transition instance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransitionParams {
    pub duration: RationalTime,
    pub easing: EasingCurve,
}

impl Default for TransitionParams {
    fn default() -> Self {
        Self {
            duration: RationalTime::from_frames(24, FrameRate::FPS_24),
            easing: EasingCurve::Linear,
        }
    }
}

/// Registry of available transitions.
pub struct TransitionRegistry {
    transitions: Vec<Box<dyn Transition>>,
}

impl TransitionRegistry {
    /// Create a new registry with all built-in transitions.
    pub fn new() -> Self {
        let mut reg = Self {
            transitions: Vec::new(),
        };
        // Register built-ins
        reg.register(Box::new(super::transitions::CrossDissolve));
        reg.register(Box::new(super::transitions::DipToBlack));
        reg.register(Box::new(super::transitions::DipToWhite));
        reg.register(Box::new(super::transitions::Wipe::default()));
        reg.register(Box::new(super::transitions::Push::default()));
        reg.register(Box::new(super::transitions::Iris::default()));
        reg
    }

    /// Register a custom transition.
    pub fn register(&mut self, t: Box<dyn Transition>) {
        self.transitions.push(t);
    }

    /// Find a transition by name.
    pub fn find(&self, name: &str) -> Option<&dyn Transition> {
        self.transitions
            .iter()
            .find(|t| t.name() == name)
            .map(|t| t.as_ref())
    }

    /// Get all registered transition names.
    pub fn names(&self) -> Vec<&str> {
        self.transitions.iter().map(|t| t.name()).collect()
    }
}

impl Default for TransitionRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_has_builtins() {
        let reg = TransitionRegistry::new();
        let names = reg.names();
        assert!(names.contains(&"Cross Dissolve"));
        assert!(names.contains(&"Dip to Black"));
        assert!(names.contains(&"Dip to White"));
        assert!(names.contains(&"Wipe"));
        assert!(names.contains(&"Push"));
        assert!(names.contains(&"Iris"));
    }

    #[test]
    fn test_find_transition() {
        let reg = TransitionRegistry::new();
        assert!(reg.find("Cross Dissolve").is_some());
        assert!(reg.find("Nonexistent").is_none());
    }
}
