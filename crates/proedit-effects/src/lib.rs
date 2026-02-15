//! ProEdit Effects - GPU effects library and CPU processing pipelines
//!
//! Provides video effects, transitions, chroma keying, masking,
//! optical flow, motion blur, and frame interpolation.

pub mod chroma_key;
pub mod frame_interp;
pub mod mask;
pub mod motion_blur;
pub mod optical_flow;
pub mod transition;
pub mod transitions;

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
            effects: vec![
                Box::new(ChromaKeyEffect::new()),
                Box::new(GaussianBlurEffect::new()),
                Box::new(FilmGrainEffect::new()),
                Box::new(VignetteEffect::new()),
            ],
        }
    }

    /// Get all registered effects.
    pub fn effects(&self) -> &[Box<dyn VideoEffect>] {
        &self.effects
    }

    /// Find an effect by name.
    pub fn find(&self, name: &str) -> Option<&dyn VideoEffect> {
        self.effects
            .iter()
            .find(|e| e.name() == name)
            .map(|e| e.as_ref())
    }
}

impl Default for EffectsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// Built-in effect adapters (stub GPU render; real shaders added later)
// ---------------------------------------------------------------------------

/// Chroma key (green/blue screen) effect adapter.
///
/// Wraps the CPU-based `ChromaKeyProcessor` as a `VideoEffect`.
/// The `render` method is currently a stub — the GPU shader pipeline
/// will be connected in a future pass.
pub struct ChromaKeyEffect {
    params: Vec<ParamDescriptor>,
}

impl Default for ChromaKeyEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl ChromaKeyEffect {
    pub fn new() -> Self {
        Self {
            params: vec![
                ParamDescriptor {
                    name: "key_color".into(),
                    display_name: "Key Color".into(),
                    default: ParamValue::Color([0.0, 1.0, 0.0, 1.0]),
                    min: None,
                    max: None,
                },
                ParamDescriptor {
                    name: "tolerance".into(),
                    display_name: "Tolerance".into(),
                    default: ParamValue::Float(0.35),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
                ParamDescriptor {
                    name: "softness".into(),
                    display_name: "Softness".into(),
                    default: ParamValue::Float(0.1),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
                ParamDescriptor {
                    name: "spill_suppression".into(),
                    display_name: "Spill Suppression".into(),
                    default: ParamValue::Float(0.6),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
            ],
        }
    }
}

impl VideoEffect for ChromaKeyEffect {
    fn name(&self) -> &str {
        "Chroma Key"
    }

    fn params(&self) -> &[ParamDescriptor] {
        &self.params
    }

    fn render(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _input: &GpuTexture,
        _output: &GpuTexture,
        _params: &ParamValues,
    ) -> Result<()> {
        // Stub — GPU chroma key shader will be wired here.
        Ok(())
    }
}

/// Gaussian blur effect (stub).
pub struct GaussianBlurEffect {
    params: Vec<ParamDescriptor>,
}

impl Default for GaussianBlurEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl GaussianBlurEffect {
    pub fn new() -> Self {
        Self {
            params: vec![
                ParamDescriptor {
                    name: "radius".into(),
                    display_name: "Radius".into(),
                    default: ParamValue::Float(5.0),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(100.0)),
                },
                ParamDescriptor {
                    name: "sigma".into(),
                    display_name: "Sigma".into(),
                    default: ParamValue::Float(1.5),
                    min: Some(ParamValue::Float(0.1)),
                    max: Some(ParamValue::Float(50.0)),
                },
            ],
        }
    }
}

impl VideoEffect for GaussianBlurEffect {
    fn name(&self) -> &str {
        "Gaussian Blur"
    }

    fn params(&self) -> &[ParamDescriptor] {
        &self.params
    }

    fn render(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _input: &GpuTexture,
        _output: &GpuTexture,
        _params: &ParamValues,
    ) -> Result<()> {
        // Stub — GPU blur shader will be wired here.
        Ok(())
    }
}

/// Film grain effect (stub).
pub struct FilmGrainEffect {
    params: Vec<ParamDescriptor>,
}

impl Default for FilmGrainEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl FilmGrainEffect {
    pub fn new() -> Self {
        Self {
            params: vec![
                ParamDescriptor {
                    name: "intensity".into(),
                    display_name: "Intensity".into(),
                    default: ParamValue::Float(0.3),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
                ParamDescriptor {
                    name: "size".into(),
                    display_name: "Grain Size".into(),
                    default: ParamValue::Float(1.0),
                    min: Some(ParamValue::Float(0.5)),
                    max: Some(ParamValue::Float(5.0)),
                },
                ParamDescriptor {
                    name: "seed".into(),
                    display_name: "Random Seed".into(),
                    default: ParamValue::Int(0),
                    min: Some(ParamValue::Int(0)),
                    max: Some(ParamValue::Int(i32::MAX)),
                },
            ],
        }
    }
}

impl VideoEffect for FilmGrainEffect {
    fn name(&self) -> &str {
        "Film Grain"
    }

    fn params(&self) -> &[ParamDescriptor] {
        &self.params
    }

    fn render(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _input: &GpuTexture,
        _output: &GpuTexture,
        _params: &ParamValues,
    ) -> Result<()> {
        // Stub — GPU grain shader will be wired here.
        Ok(())
    }
}

/// Vignette effect (stub).
pub struct VignetteEffect {
    params: Vec<ParamDescriptor>,
}

impl Default for VignetteEffect {
    fn default() -> Self {
        Self::new()
    }
}

impl VignetteEffect {
    pub fn new() -> Self {
        Self {
            params: vec![
                ParamDescriptor {
                    name: "intensity".into(),
                    display_name: "Intensity".into(),
                    default: ParamValue::Float(0.5),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
                ParamDescriptor {
                    name: "radius".into(),
                    display_name: "Radius".into(),
                    default: ParamValue::Float(0.75),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(2.0)),
                },
                ParamDescriptor {
                    name: "softness".into(),
                    display_name: "Softness".into(),
                    default: ParamValue::Float(0.5),
                    min: Some(ParamValue::Float(0.0)),
                    max: Some(ParamValue::Float(1.0)),
                },
                ParamDescriptor {
                    name: "color".into(),
                    display_name: "Color".into(),
                    default: ParamValue::Color([0.0, 0.0, 0.0, 1.0]),
                    min: None,
                    max: None,
                },
            ],
        }
    }
}

impl VideoEffect for VignetteEffect {
    fn name(&self) -> &str {
        "Vignette"
    }

    fn params(&self) -> &[ParamDescriptor] {
        &self.params
    }

    fn render(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _input: &GpuTexture,
        _output: &GpuTexture,
        _params: &ParamValues,
    ) -> Result<()> {
        // Stub — GPU vignette shader will be wired here.
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_contains_builtin_effects() {
        let registry = EffectsRegistry::new();
        let names: Vec<&str> = registry.effects().iter().map(|e| e.name()).collect();
        assert!(
            names.contains(&"Chroma Key"),
            "Registry should contain Chroma Key, got: {names:?}"
        );
        assert!(
            names.contains(&"Gaussian Blur"),
            "Registry should contain Gaussian Blur, got: {names:?}"
        );
        assert!(
            names.contains(&"Film Grain"),
            "Registry should contain Film Grain, got: {names:?}"
        );
        assert!(
            names.contains(&"Vignette"),
            "Registry should contain Vignette, got: {names:?}"
        );
    }

    #[test]
    fn registry_find_by_name() {
        let registry = EffectsRegistry::new();
        let ck = registry.find("Chroma Key");
        assert!(ck.is_some(), "Should find Chroma Key by name");
        assert_eq!(ck.unwrap().name(), "Chroma Key");

        assert!(registry.find("Nonexistent Effect").is_none());
    }

    #[test]
    fn chroma_key_effect_has_expected_params() {
        let effect = ChromaKeyEffect::new();
        let param_names: Vec<&str> = effect.params().iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"key_color"));
        assert!(param_names.contains(&"tolerance"));
        assert!(param_names.contains(&"softness"));
        assert!(param_names.contains(&"spill_suppression"));
    }

    #[test]
    fn gaussian_blur_effect_has_expected_params() {
        let effect = GaussianBlurEffect::new();
        let param_names: Vec<&str> = effect.params().iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"radius"));
        assert!(param_names.contains(&"sigma"));
    }

    #[test]
    fn film_grain_effect_has_expected_params() {
        let effect = FilmGrainEffect::new();
        let param_names: Vec<&str> = effect.params().iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"intensity"));
        assert!(param_names.contains(&"size"));
        assert!(param_names.contains(&"seed"));
    }

    #[test]
    fn vignette_effect_has_expected_params() {
        let effect = VignetteEffect::new();
        let param_names: Vec<&str> = effect.params().iter().map(|p| p.name.as_str()).collect();
        assert!(param_names.contains(&"intensity"));
        assert!(param_names.contains(&"radius"));
        assert!(param_names.contains(&"softness"));
        assert!(param_names.contains(&"color"));
    }

    #[test]
    fn registry_has_correct_count() {
        let registry = EffectsRegistry::new();
        assert_eq!(
            registry.effects().len(),
            4,
            "Should have exactly 4 built-in effects"
        );
    }
}
