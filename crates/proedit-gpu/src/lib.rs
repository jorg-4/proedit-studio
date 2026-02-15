//! ProEdit GPU - wgpu-based rendering pipeline
//!
//! Uses Metal backend on macOS for optimal M1 performance.

pub mod blend;
pub mod context;
pub mod pipeline;
pub mod render_graph;
pub mod texture;
pub mod texture_pool;

pub use blend::BlendMode;
pub use context::GpuContext;
pub use pipeline::BlitPipeline;
pub use render_graph::{FrameCache, NodeId, NodeOp, RenderGraph, RenderNode};
pub use texture::GpuTexture;
pub use texture_pool::TexturePool;
