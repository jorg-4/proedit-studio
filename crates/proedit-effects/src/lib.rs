//! ProEdit Effects - GPU effects library
//!
//! Provides video effects implemented as GPU compute shaders.

use proedit_core::Result;
use proedit_gpu::GpuTexture;
use serde::{Deserialize, Serialize};

/// Effect parameter types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParamValue {
    Float(f32),
    Int(i32),
    Bool(bool),
    Color([f32; 4]),
    Vec2([f32; 2]),
}

/// Effect parameter descriptor.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParamDescriptor {
    pub name: String,
    pub display_name: String,
    pub default: ParamValue,
    pub min: Option<ParamValue>,
    pub max: Option<ParamValue>,
}

/// Collection of parameter values.
pub type ParamValues = std::collections::HashMap<String, ParamValue>;

/// Trait for video effects.
pub trait VideoEffect: Send + Sync {
    /// Get the effect name.
    fn name(&self) -> &str;

    /// Get parameter descriptors.
    fn params(&self) -> &[ParamDescriptor];

    /// Render the effect.
    fn render(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        input: &GpuTexture,
        output: &GpuTexture,
        params: &ParamValues,
    ) -> Result<()>;
}

/// Built-in effects registry.
pub struct EffectsRegistry {
    effects: Vec<Box<dyn VideoEffect>>,
}

impl EffectsRegistry {
    /// Create a new registry with built-in effects.
    pub fn new() -> Self {
        Self {
            effects: Vec::new(),
        }
    }

    /// Get all registered effects.
    pub fn effects(&self) -> &[Box<dyn VideoEffect>] {
        &self.effects
    }

    /// Find an effect by name.
    pub fn find(&self, name: &str) -> Option<&dyn VideoEffect> {
        self.effects.iter().find(|e| e.name() == name).map(|e| e.as_ref())
    }
}

impl Default for EffectsRegistry {
    fn default() -> Self {
        Self::new()
    }
}
