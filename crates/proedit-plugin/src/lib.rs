//! ProEdit Plugin — OFX hosting and WASM sandbox for third-party effects.

pub mod error;
pub mod ofx;
pub mod plugin_manager;
pub mod wasm_sandbox;

pub use error::PluginError;
pub use ofx::{
    OfxImageEffectDescriptor, OfxParamDescriptor, OfxParamType, OfxPluginInfo, OfxPropertySet,
};
pub use plugin_manager::PluginManager;
pub use wasm_sandbox::{WasmParamDescriptor, WasmPlugin, WasmRuntime};
