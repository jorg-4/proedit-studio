//! OFX (Open Effects) plugin host types.
//!
//! Provides data structures matching the OFX standard for describing
//! image effect plugins. No C FFI linking — types only.

use std::collections::HashMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::PluginError;

/// OFX status code.
pub type OfxStatus = i32;

/// Success status.
pub const OFX_OK: OfxStatus = 0;

/// Failure status.
pub const OFX_FAILED: OfxStatus = 1;

/// OFX property value.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OfxProperty {
    Int(Vec<i32>),
    Double(Vec<f64>),
    String(Vec<String>),
    Pointer(usize),
}

/// A set of named OFX properties.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OfxPropertySet {
    props: HashMap<String, OfxProperty>,
}

impl OfxPropertySet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, key: &str, value: OfxProperty) {
        self.props.insert(key.to_string(), value);
    }

    pub fn get(&self, key: &str) -> Option<&OfxProperty> {
        self.props.get(key)
    }

    pub fn get_int(&self, key: &str) -> Option<i32> {
        match self.props.get(key) {
            Some(OfxProperty::Int(v)) => v.first().copied(),
            _ => None,
        }
    }

    pub fn get_double(&self, key: &str) -> Option<f64> {
        match self.props.get(key) {
            Some(OfxProperty::Double(v)) => v.first().copied(),
            _ => None,
        }
    }

    pub fn get_string(&self, key: &str) -> Option<&str> {
        match self.props.get(key) {
            Some(OfxProperty::String(v)) => v.first().map(|s| s.as_str()),
            _ => None,
        }
    }

    pub fn keys(&self) -> impl Iterator<Item = &str> {
        self.props.keys().map(|s| s.as_str())
    }
}

/// OFX parameter type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum OfxParamType {
    Double,
    Int,
    Bool,
    Choice,
    Color,
    String2D,
    String3D,
}

/// Descriptor for a single OFX parameter.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfxParamDescriptor {
    pub name: String,
    pub param_type: OfxParamType,
    pub properties: OfxPropertySet,
}

/// Descriptor for an OFX image effect.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfxImageEffectDescriptor {
    pub name: String,
    pub label: String,
    pub group: String,
    pub params: Vec<OfxParamDescriptor>,
    pub supported_contexts: Vec<String>,
}

/// Information about a discovered OFX plugin.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OfxPluginInfo {
    pub identifier: String,
    pub version_major: u32,
    pub version_minor: u32,
    pub descriptor: OfxImageEffectDescriptor,
}

/// Scan a directory for OFX plugin bundles.
///
/// Currently returns an empty list since actual OFX C FFI loading
/// is not yet implemented.
pub fn scan_ofx_directory(path: &Path) -> Result<Vec<OfxPluginInfo>, PluginError> {
    if !path.exists() {
        return Err(PluginError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("OFX directory not found: {}", path.display()),
        )));
    }
    // No actual OFX loading yet — return empty
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_property_set() {
        let mut props = OfxPropertySet::new();
        props.set("count", OfxProperty::Int(vec![42]));
        props.set("name", OfxProperty::String(vec!["test".into()]));
        props.set("value", OfxProperty::Double(vec![1.234]));

        assert_eq!(props.get_int("count"), Some(42));
        assert_eq!(props.get_string("name"), Some("test"));
        assert!((props.get_double("value").unwrap() - 1.234).abs() < 0.001);
        assert!(props.get_int("missing").is_none());
    }

    #[test]
    fn test_param_descriptor() {
        let param = OfxParamDescriptor {
            name: "opacity".into(),
            param_type: OfxParamType::Double,
            properties: OfxPropertySet::new(),
        };
        assert_eq!(param.param_type, OfxParamType::Double);
    }

    #[test]
    fn test_effect_descriptor() {
        let desc = OfxImageEffectDescriptor {
            name: "blur".into(),
            label: "Gaussian Blur".into(),
            group: "Filter".into(),
            params: vec![],
            supported_contexts: vec!["Filter".into()],
        };
        assert_eq!(desc.supported_contexts.len(), 1);
    }

    #[test]
    fn test_scan_nonexistent_dir() {
        let result = scan_ofx_directory(Path::new("/nonexistent/ofx/plugins"));
        assert!(result.is_err());
    }
}
