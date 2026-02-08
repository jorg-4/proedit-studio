//! GPU context management.

use proedit_core::{ProEditError, Result};
use std::sync::Arc;
use tracing::info;

/// GPU context holding device and queue.
pub struct GpuContext {
    pub instance: wgpu::Instance,
    pub adapter: wgpu::Adapter,
    pub device: Arc<wgpu::Device>,
    pub queue: Arc<wgpu::Queue>,
}

impl GpuContext {
    /// Create a new GPU context.
    ///
    /// On macOS, this uses the Metal backend for optimal M1 performance.
    pub async fn new() -> Result<Self> {
        // Prefer Metal on macOS, Vulkan on others
        #[cfg(target_os = "macos")]
        let backends = wgpu::Backends::METAL;
        #[cfg(not(target_os = "macos"))]
        let backends = wgpu::Backends::VULKAN | wgpu::Backends::DX12;

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends,
            ..Default::default()
        });

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: None,
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| ProEditError::Gpu("No suitable GPU adapter found".to_string()))?;

        info!("Using GPU adapter: {:?}", adapter.get_info());

        // Request device with limits suitable for video editing
        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("ProEdit Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits {
                        // Reasonable limits for 8GB M1
                        max_texture_dimension_2d: 8192,
                        max_buffer_size: 512 * 1024 * 1024, // 512MB max buffer
                        max_storage_buffer_binding_size: 256 * 1024 * 1024,
                        ..wgpu::Limits::default()
                    },
                    memory_hints: wgpu::MemoryHints::Performance,
                },
                None,
            )
            .await
            .map_err(|e| ProEditError::Gpu(format!("Failed to create device: {}", e)))?;

        Ok(Self {
            instance,
            adapter,
            device: Arc::new(device),
            queue: Arc::new(queue),
        })
    }

    /// Create a new GPU context (blocking version).
    pub fn new_blocking() -> Result<Self> {
        pollster::block_on(Self::new())
    }

    /// Get adapter info.
    pub fn adapter_info(&self) -> wgpu::AdapterInfo {
        self.adapter.get_info()
    }
}
