//! Unified plugin manager for OFX and WASM plugins.

use std::path::PathBuf;

use crate::ofx::{self, OfxPluginInfo};
use crate::wasm_sandbox::WasmRuntime;

/// Unified plugin manager.
pub struct PluginManager {
    ofx_plugins: Vec<OfxPluginInfo>,
    wasm_runtime: WasmRuntime,
}

impl PluginManager {
    pub fn new() -> Self {
        Self {
            ofx_plugins: Vec::new(),
            wasm_runtime: WasmRuntime::new(),
        }
    }

    /// Scan given directories for plugins (OFX bundles, WASM files).
    /// Returns names of discovered plugins.
    pub fn scan_plugins(&mut self, search_paths: &[PathBuf]) -> Vec<String> {
        let mut found = Vec::new();
        for path in search_paths {
            if let Ok(plugins) = ofx::scan_ofx_directory(path) {
                for p in plugins {
                    found.push(p.identifier.clone());
                    self.ofx_plugins.push(p);
                }
            }
        }
        found
    }

    /// Get all discovered OFX plugins.
    pub fn ofx_plugins(&self) -> &[OfxPluginInfo] {
        &self.ofx_plugins
    }

    /// Get the WASM runtime.
    pub fn wasm_runtime(&self) -> &WasmRuntime {
        &self.wasm_runtime
    }

    /// Get all plugin names (OFX + WASM).
    pub fn all_plugin_names(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .ofx_plugins
            .iter()
            .map(|p| p.identifier.clone())
            .collect();
        for p in self.wasm_runtime.plugins() {
            names.push(p.name().to_string());
        }
        names
    }

    /// Total number of plugins.
    pub fn plugin_count(&self) -> usize {
        self.ofx_plugins.len() + self.wasm_runtime.plugin_count()
    }
}

impl Default for PluginManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plugin_manager_new() {
        let pm = PluginManager::new();
        assert_eq!(pm.plugin_count(), 0);
        assert!(pm.all_plugin_names().is_empty());
    }

    #[test]
    fn test_scan_empty_paths() {
        let mut pm = PluginManager::new();
        let found = pm.scan_plugins(&[]);
        assert!(found.is_empty());
    }

    #[test]
    fn test_ofx_plugins_empty() {
        let pm = PluginManager::new();
        assert!(pm.ofx_plugins().is_empty());
    }
}
