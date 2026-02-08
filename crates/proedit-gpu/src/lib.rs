//! ProEdit GPU - wgpu-based rendering pipeline
//!
//! Uses Metal backend on macOS for optimal M1 performance.

pub mod context;
pub mod pipeline;
pub mod texture;

pub use context::GpuContext;
pub use pipeline::BlitPipeline;
pub use texture::GpuTexture;
