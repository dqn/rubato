// GPU texture upload and bind group cache.
// Manages lazy upload of CPU-side RGBA data to wgpu textures.

use std::collections::HashMap;
use std::sync::Arc;

use crate::render_pipeline::SpriteRenderPipeline;

/// Cached GPU texture entry: wgpu texture + two bind groups (nearest/linear sampler).
struct GpuTextureEntry {
    _texture: wgpu::Texture,
    _view: wgpu::TextureView,
    bind_group_nearest: wgpu::BindGroup,
    bind_group_linear: wgpu::BindGroup,
}

/// Pending texture data waiting to be uploaded to the GPU.
#[derive(Debug)]
pub struct PendingTexture {
    pub width: u32,
    pub height: u32,
    pub rgba_data: Arc<Vec<u8>>,
}

/// Manages GPU texture uploads and bind group caching.
/// Textures are uploaded lazily on first use and cached by path key.
pub struct GpuTextureManager {
    entries: HashMap<Arc<str>, GpuTextureEntry>,
    /// Bind group for path-less textures (1x1 white fallback)
    fallback_bind_group_nearest: wgpu::BindGroup,
    fallback_bind_group_linear: wgpu::BindGroup,
    /// Counter for generating unique keys for path-less textures
    anon_counter: u64,
}

impl GpuTextureManager {
    /// Create a new GpuTextureManager with a 1x1 white fallback texture.
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        sampler_nearest: &wgpu::Sampler,
        sampler_linear: &wgpu::Sampler,
    ) -> Self {
        // Create 1x1 white fallback texture
        let fallback_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("fallback white texture"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &fallback_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &[255u8, 255, 255, 255],
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        let fallback_view = fallback_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let fallback_bind_group_nearest = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fallback texture bind group (nearest)"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler_nearest),
                },
            ],
        });

        let fallback_bind_group_linear = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fallback texture bind group (linear)"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&fallback_view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler_linear),
                },
            ],
        });

        Self {
            entries: HashMap::new(),
            fallback_bind_group_nearest,
            fallback_bind_group_linear,
            anon_counter: 0,
        }
    }

    /// Upload a texture to the GPU if not already cached.
    /// Returns the cache key.
    #[allow(clippy::too_many_arguments)]
    pub fn ensure_uploaded(
        &mut self,
        key: &Arc<str>,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        texture_layout: &wgpu::BindGroupLayout,
        sampler_nearest: &wgpu::Sampler,
        sampler_linear: &wgpu::Sampler,
    ) {
        if self.entries.contains_key(key) {
            return;
        }

        if width == 0 || height == 0 {
            return;
        }

        let expected_size = (width * height * 4) as usize;
        if rgba_data.len() < expected_size {
            log::warn!(
                "Texture '{}' has insufficient data: {} bytes, expected {}",
                key,
                rgba_data.len(),
                expected_size
            );
            return;
        }

        let size = wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        };
        let wgpu_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skin texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * width),
                rows_per_image: Some(height),
            },
            size,
        );
        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_nearest = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin texture bind group (nearest)"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler_nearest),
                },
            ],
        });

        let bind_group_linear = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin texture bind group (linear)"),
            layout: texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(sampler_linear),
                },
            ],
        });

        self.entries.insert(
            Arc::clone(key),
            GpuTextureEntry {
                _texture: wgpu_texture,
                _view: view,
                bind_group_nearest,
                bind_group_linear,
            },
        );
    }

    /// Generate a unique anonymous key for path-less textures (e.g., from Pixmap).
    pub fn generate_anon_key(&mut self) -> Arc<str> {
        self.anon_counter += 1;
        Arc::from(format!("__anon_{}", self.anon_counter))
    }

    /// Get the bind group for a texture by key and shader type.
    /// Returns the fallback bind group if the key is not found.
    pub fn get_bind_group(&self, key: Option<&Arc<str>>, shader_type: i32) -> &wgpu::BindGroup {
        let linear = SpriteRenderPipeline::is_linear_sampler(shader_type);
        if let Some(key) = key
            && let Some(entry) = self.entries.get(key)
        {
            return if linear {
                &entry.bind_group_linear
            } else {
                &entry.bind_group_nearest
            };
        }
        if linear {
            &self.fallback_bind_group_linear
        } else {
            &self.fallback_bind_group_nearest
        }
    }
}
