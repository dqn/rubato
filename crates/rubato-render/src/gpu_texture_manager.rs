// GPU texture upload and bind group cache.
// Manages lazy upload of CPU-side RGBA data to wgpu textures.

use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::render_pipeline::SpriteRenderPipeline;

/// GPU resources needed for texture upload operations.
pub struct TextureUploadContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub texture_layout: &'a wgpu::BindGroupLayout,
    pub sampler_nearest: &'a wgpu::Sampler,
    pub sampler_linear: &'a wgpu::Sampler,
}

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
///
/// Tracks which textures are referenced each frame via `ensure_uploaded()`.
/// Call `evict_unused()` after rendering to free GPU textures that were
/// not referenced in the current frame (e.g., stale BGA video frames).
pub struct GpuTextureManager {
    entries: HashMap<Arc<str>, GpuTextureEntry>,
    /// Bind group for path-less textures (1x1 white fallback)
    fallback_bind_group_nearest: wgpu::BindGroup,
    fallback_bind_group_linear: wgpu::BindGroup,
    /// Counter for generating unique keys for path-less textures
    anon_counter: u64,
    /// Keys passed to `ensure_uploaded()` in the current frame.
    used_this_frame: HashSet<Arc<str>>,
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
            format: wgpu::TextureFormat::Rgba8Unorm,
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
            used_this_frame: HashSet::new(),
        }
    }

    /// Upload a texture to the GPU if not already cached.
    /// Also marks the key as used for the current frame (see `evict_unused()`).
    pub fn ensure_uploaded(
        &mut self,
        key: &Arc<str>,
        width: u32,
        height: u32,
        rgba_data: &[u8],
        ctx: &TextureUploadContext<'_>,
    ) {
        self.used_this_frame.insert(Arc::clone(key));
        // Pixmap-backed textures (keyed by stable monotonic ID) may have their
        // underlying data mutated between frames. Always re-upload them.
        let is_pixmap = key.starts_with("__pixmap_");
        if self.entries.contains_key(key) && !is_pixmap {
            return;
        }

        if width == 0 || height == 0 {
            return;
        }

        let expected_size = (width as usize) * (height as usize) * 4;
        if rgba_data.len() < expected_size {
            log::warn!(
                "Texture '{}' has insufficient data: {} bytes, expected {}",
                key,
                rgba_data.len(),
                expected_size
            );
            return;
        }

        let max_dim = ctx.device.limits().max_texture_dimension_2d;
        let (upload_width, upload_height, upload_data);
        if width > max_dim || height > max_dim {
            let scale = max_dim as f32 / width.max(height) as f32;
            let new_w = (width as f32 * scale).round() as u32;
            let new_h = (height as f32 * scale).round() as u32;
            let new_w = new_w.max(1).min(max_dim);
            let new_h = new_h.max(1).min(max_dim);
            log::info!(
                "Texture '{}' dimensions {}x{} exceed GPU limit {}; downscaling to {}x{}",
                key,
                width,
                height,
                max_dim,
                new_w,
                new_h,
            );
            upload_data = bilinear_resize(rgba_data, width, height, new_w, new_h);
            upload_width = new_w;
            upload_height = new_h;
        } else {
            upload_data = Vec::new(); // unused; we'll use rgba_data directly
            upload_width = width;
            upload_height = height;
        }

        let actual_data: &[u8] = if !upload_data.is_empty() {
            &upload_data
        } else {
            rgba_data
        };

        let size = wgpu::Extent3d {
            width: upload_width,
            height: upload_height,
            depth_or_array_layers: 1,
        };
        let wgpu_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("skin texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        ctx.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &wgpu_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            actual_data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * upload_width),
                rows_per_image: Some(upload_height),
            },
            size,
        );
        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let bind_group_nearest = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin texture bind group (nearest)"),
            layout: ctx.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(ctx.sampler_nearest),
                },
            ],
        });

        let bind_group_linear = ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("skin texture bind group (linear)"),
            layout: ctx.texture_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(ctx.sampler_linear),
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
    pub fn bind_group(&self, key: Option<&Arc<str>>, shader_type: i32) -> &wgpu::BindGroup {
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

    /// Remove a single texture entry by key, freeing its GPU resources.
    pub fn remove(&mut self, key: &Arc<str>) {
        self.entries.remove(key);
    }

    /// Evict all cached textures that were not passed to `ensure_uploaded()`
    /// since the last call to `evict_unused()`. Call once per frame after
    /// rendering to free stale GPU textures (e.g., old BGA video frames).
    pub fn evict_unused(&mut self) {
        let used = &self.used_this_frame;
        self.entries.retain(|k, _| used.contains(k));
        self.used_this_frame.clear();
    }

    /// Return the number of cached texture entries (for diagnostics).
    pub fn entry_count(&self) -> usize {
        self.entries.len()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};
    use std::sync::Arc;

    /// Test the eviction logic in isolation (mirrors `evict_unused` behavior).
    /// This avoids needing a real wgpu device for the test.
    fn evict_unused(entries: &mut HashMap<Arc<str>, ()>, used_this_frame: &mut HashSet<Arc<str>>) {
        let used = &*used_this_frame;
        entries.retain(|k, _| used.contains(k));
        used_this_frame.clear();
    }

    #[test]
    fn evict_unused_clears_all_entries_when_no_textures_referenced() {
        let mut entries = HashMap::new();
        entries.insert(Arc::<str>::from("tex_a"), ());
        entries.insert(Arc::<str>::from("tex_b"), ());
        entries.insert(Arc::<str>::from("tex_c"), ());
        let mut used = HashSet::new();

        // Blank frame: no textures referenced. All entries should be evicted.
        evict_unused(&mut entries, &mut used);
        assert!(
            entries.is_empty(),
            "All cached textures should be evicted when none were referenced"
        );
    }

    #[test]
    fn evict_unused_retains_only_referenced_textures() {
        let mut entries = HashMap::new();
        entries.insert(Arc::<str>::from("tex_a"), ());
        entries.insert(Arc::<str>::from("tex_b"), ());
        entries.insert(Arc::<str>::from("tex_c"), ());
        let mut used = HashSet::new();
        used.insert(Arc::<str>::from("tex_b"));

        evict_unused(&mut entries, &mut used);
        assert_eq!(entries.len(), 1);
        assert!(entries.contains_key(&Arc::<str>::from("tex_b") as &str));
    }
}

/// Bilinear interpolation resize for RGBA (4 bytes/pixel) image data.
fn bilinear_resize(src: &[u8], src_w: u32, src_h: u32, dst_w: u32, dst_h: u32) -> Vec<u8> {
    let mut dst = vec![0u8; (dst_w as usize) * (dst_h as usize) * 4];
    let src_w_f = src_w as f32;
    let src_h_f = src_h as f32;
    let dst_w_f = dst_w as f32;
    let dst_h_f = dst_h as f32;

    for dy in 0..dst_h {
        for dx in 0..dst_w {
            // Map destination pixel center to source coordinates
            let sx = (dx as f32 + 0.5) * src_w_f / dst_w_f - 0.5;
            let sy = (dy as f32 + 0.5) * src_h_f / dst_h_f - 0.5;

            let x0 = sx.floor() as i32;
            let y0 = sy.floor() as i32;
            let x1 = x0 + 1;
            let y1 = y0 + 1;

            let fx = sx - x0 as f32;
            let fy = sy - y0 as f32;

            // Clamp to source bounds
            let x0 = x0.clamp(0, src_w as i32 - 1) as u32;
            let y0 = y0.clamp(0, src_h as i32 - 1) as u32;
            let x1 = x1.clamp(0, src_w as i32 - 1) as u32;
            let y1 = y1.clamp(0, src_h as i32 - 1) as u32;

            let idx00 = ((y0 as usize) * (src_w as usize) + (x0 as usize)) * 4;
            let idx10 = ((y0 as usize) * (src_w as usize) + (x1 as usize)) * 4;
            let idx01 = ((y1 as usize) * (src_w as usize) + (x0 as usize)) * 4;
            let idx11 = ((y1 as usize) * (src_w as usize) + (x1 as usize)) * 4;

            let dst_idx = ((dy as usize) * (dst_w as usize) + (dx as usize)) * 4;
            for c in 0..4 {
                let v00 = src[idx00 + c] as f32;
                let v10 = src[idx10 + c] as f32;
                let v01 = src[idx01 + c] as f32;
                let v11 = src[idx11 + c] as f32;

                let v = v00 * (1.0 - fx) * (1.0 - fy)
                    + v10 * fx * (1.0 - fy)
                    + v01 * (1.0 - fx) * fy
                    + v11 * fx * fy;

                dst[dst_idx + c] = v.round() as u8;
            }
        }
    }
    dst
}
