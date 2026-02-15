//! WASM plugin sandbox types.
//!
//! Defines the trait and types for WASM-based plugins.
//! Actual wasmtime integration is deferred to a future `wasm` feature.

use std::collections::HashMap;

use crate::error::PluginError;

/// Descriptor for a WASM plugin parameter.
#[derive(Debug, Clone)]
pub struct WasmParamDescriptor {
    pub name: String,
    pub display_name: String,
    pub default: f64,
    pub min: f64,
    pub max: f64,
}

/// Trait for WASM-based effect plugins.
pub trait WasmPlugin: Send + Sync {
    fn name(&self) -> &str;
    fn params(&self) -> &[WasmParamDescriptor];
    fn process(
        &self,
        input: &[u8],
        output: &mut [u8],
        width: u32,
        height: u32,
        params: &HashMap<String, f64>,
    ) -> Result<(), PluginError>;
}

/// Runtime for loading and managing WASM plugins.
pub struct WasmRuntime {
    plugins: Vec<Box<dyn WasmPlugin>>,
}

impl WasmRuntime {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    /// Attempt to load a WASM plugin from raw bytes.
    ///
    /// Currently always returns an error since the wasmtime runtime
    /// is not yet linked.
    pub fn load_plugin(&mut self, _wasm_bytes: &[u8]) -> Result<(), PluginError> {
        Err(PluginError::WasmNotAvailable)
    }

    /// Get all loaded WASM plugins.
    pub fn plugins(&self) -> &[Box<dyn WasmPlugin>] {
        &self.plugins
    }

    /// Number of loaded plugins.
    pub fn plugin_count(&self) -> usize {
        self.plugins.len()
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wasm_runtime_new() {
        let rt = WasmRuntime::new();
        assert_eq!(rt.plugin_count(), 0);
        assert!(rt.plugins().is_empty());
    }

    #[test]
    fn test_load_plugin_not_available() {
        let mut rt = WasmRuntime::new();
        let result = rt.load_plugin(&[0, 1, 2, 3]);
        assert!(result.is_err());
    }

    #[test]
    fn test_param_descriptor() {
        let param = WasmParamDescriptor {
            name: "blur_radius".into(),
            display_name: "Blur Radius".into(),
            default: 5.0,
            min: 0.0,
            max: 100.0,
        };
        assert_eq!(param.name, "blur_radius");
        assert!((param.default - 5.0).abs() < 0.01);
    }
}
