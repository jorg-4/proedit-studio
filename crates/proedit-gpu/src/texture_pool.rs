//! GPU texture pool for efficient texture reuse.
//!
//! Avoids allocating/deallocating GPU textures per frame by maintaining
//! a pool of textures keyed by (width, height, format).

use crate::texture::GpuTexture;
use std::collections::HashMap;

/// Key for pooled textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct TextureKey {
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

/// Pool of reusable GPU textures.
pub struct TexturePool {
    /// Available (free) textures, keyed by dimensions + format.
    free: HashMap<TextureKey, Vec<GpuTexture>>,
    /// Total memory used by all pooled textures.
    total_memory: usize,
    /// Maximum memory budget for the pool.
    max_memory: usize,
}

impl TexturePool {
    /// Create a new texture pool with the given memory budget.
    pub fn new(max_memory: usize) -> Self {
        Self {
            free: HashMap::new(),
            total_memory: 0,
            max_memory,
        }
    }

    /// Acquire a texture from the pool or create a new one.
    pub fn acquire(
        &mut self,
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
    ) -> GpuTexture {
        let key = TextureKey {
            width,
            height,
            format,
        };

        // Try to get one from the pool
        if let Some(textures) = self.free.get_mut(&key) {
            if let Some(tex) = textures.pop() {
                self.total_memory -= tex.memory_size();
                return tex;
            }
        }

        // Create a new texture
        GpuTexture::new(device, width, height, format, usage, Some("Pooled Texture"))
    }

    /// Return a texture to the pool for reuse.
    pub fn release(&mut self, texture: GpuTexture) {
        let mem = texture.memory_size();

        // If returning this texture would exceed budget, drop it instead
        if self.total_memory + mem > self.max_memory {
            return; // texture is dropped
        }

        let key = TextureKey {
            width: texture.width,
            height: texture.height,
            format: texture.format,
        };

        self.total_memory += mem;
        self.free.entry(key).or_default().push(texture);
    }

    /// Total memory used by pooled (free) textures.
    pub fn memory_usage(&self) -> usize {
        self.total_memory
    }

    /// Number of textures in the pool.
    pub fn texture_count(&self) -> usize {
        self.free.values().map(|v| v.len()).sum()
    }

    /// Clear all pooled textures.
    pub fn clear(&mut self) {
        self.free.clear();
        self.total_memory = 0;
    }

    /// Evict textures until memory is at or below the target.
    pub fn evict_to(&mut self, target_memory: usize) {
        while self.total_memory > target_memory {
            // Find the key with the most textures and remove one
            let key = self
                .free
                .iter()
                .filter(|(_, v)| !v.is_empty())
                .max_by_key(|(_, v)| v.len())
                .map(|(k, _)| *k);

            if let Some(key) = key {
                if let Some(textures) = self.free.get_mut(&key) {
                    if let Some(tex) = textures.pop() {
                        self.total_memory -= tex.memory_size();
                    }
                    if textures.is_empty() {
                        self.free.remove(&key);
                    }
                }
            } else {
                break;
            }
        }
    }
}
