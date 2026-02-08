//! GPU texture management.

use proedit_core::{FrameBuffer, PixelFormat, ProEditError, Result};

/// A GPU texture that can hold video frame data.
pub struct GpuTexture {
    pub texture: wgpu::Texture,
    pub view: wgpu::TextureView,
    pub width: u32,
    pub height: u32,
    pub format: wgpu::TextureFormat,
}

impl GpuTexture {
    /// Create a new GPU texture with the given dimensions.
    pub fn new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
        usage: wgpu::TextureUsages,
        label: Option<&str>,
    ) -> Self {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label,
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        Self {
            texture,
            view,
            width,
            height,
            format,
        }
    }

    /// Create a texture suitable for video frame upload.
    pub fn for_video_frame(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::new(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            Some("Video Frame Texture"),
        )
    }

    /// Create a render target texture.
    pub fn render_target(device: &wgpu::Device, width: u32, height: u32) -> Self {
        Self::new(
            device,
            width,
            height,
            wgpu::TextureFormat::Rgba8UnormSrgb,
            wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::COPY_SRC,
            Some("Render Target"),
        )
    }

    /// Upload a FrameBuffer to this texture.
    pub fn upload_frame(&self, queue: &wgpu::Queue, frame: &FrameBuffer) -> Result<()> {
        if frame.format != PixelFormat::Rgba8 {
            return Err(ProEditError::Gpu(
                "Only RGBA8 format supported for upload".to_string(),
            ));
        }

        if frame.width != self.width || frame.height != self.height {
            return Err(ProEditError::Gpu(format!(
                "Frame size {}x{} doesn't match texture size {}x{}",
                frame.width, frame.height, self.width, self.height
            )));
        }

        let plane = frame.primary_plane();

        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &plane.data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(plane.stride as u32),
                rows_per_image: Some(self.height),
            },
            wgpu::Extent3d {
                width: self.width,
                height: self.height,
                depth_or_array_layers: 1,
            },
        );

        Ok(())
    }

    /// Memory usage estimate in bytes.
    pub fn memory_size(&self) -> usize {
        let bytes_per_pixel = match self.format {
            wgpu::TextureFormat::Rgba8UnormSrgb | wgpu::TextureFormat::Rgba8Unorm => 4,
            wgpu::TextureFormat::Rgba16Float => 8,
            wgpu::TextureFormat::Rgba32Float => 16,
            _ => 4,
        };
        (self.width * self.height) as usize * bytes_per_pixel
    }
}
