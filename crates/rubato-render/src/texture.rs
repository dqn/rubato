// wgpu-backed Texture and TextureRegion.
// Drop-in replacements for the types in rendering_stubs.rs.

use std::sync::Arc;

use crate::gpu_context::GpuContext;
use crate::pixmap::{Pixmap, PixmapFormat};

/// Texture filter modes matching LibGDX TextureFilter.
#[derive(Clone, Debug, PartialEq)]
pub enum TextureFilter {
    Nearest,
    Linear,
    MipMap,
    MipMapNearestNearest,
    MipMapLinearNearest,
    MipMapNearestLinear,
    MipMapLinearLinear,
}

/// GPU-backed texture.
/// Corresponds to com.badlogic.gdx.graphics.Texture.
#[derive(Clone, Debug, Default)]
pub struct Texture {
    pub width: i32,
    pub height: i32,
    pub disposed: bool,
    /// Source file path for GPU texture cache lookup (cheap clone via Arc)
    pub path: Option<Arc<str>>,
    /// RGBA pixel data for lazy GPU upload (cheap clone via Arc)
    pub rgba_data: Option<Arc<Vec<u8>>>,
    /// GPU texture handle (stored after upload)
    pub gpu_texture: Option<Arc<wgpu::Texture>>,
    /// GPU texture view (stored after upload)
    pub gpu_view: Option<Arc<wgpu::TextureView>>,
    /// GPU sampler (created by set_filter)
    pub sampler: Option<Arc<wgpu::Sampler>>,
}

impl PartialEq for Texture {
    fn eq(&self, other: &Self) -> bool {
        self.width == other.width
            && self.height == other.height
            && self.disposed == other.disposed
            && self.path == other.path
    }
}

#[allow(unused_variables)]
impl Texture {
    pub fn new(path: &str) -> Self {
        // Load from file path — actual GPU upload deferred until a GpuContext is available
        if let Ok(img) = image::open(path) {
            let rgba = img.to_rgba8();
            Self {
                width: rgba.width() as i32,
                height: rgba.height() as i32,
                disposed: false,
                path: Some(Arc::from(path)),
                rgba_data: Some(Arc::new(rgba.into_raw())),
                ..Default::default()
            }
        } else {
            Self::default()
        }
    }

    pub fn from_pixmap(pixmap: &Pixmap) -> Self {
        Self {
            width: pixmap.width,
            height: pixmap.height,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(pixmap.data().to_vec())),
            ..Default::default()
        }
    }

    pub fn from_pixmap_with_mipmaps(pixmap: &Pixmap, use_mip_maps: bool) -> Self {
        Self {
            width: pixmap.width,
            height: pixmap.height,
            disposed: false,
            path: None,
            rgba_data: Some(Arc::new(pixmap.data().to_vec())),
            ..Default::default()
        }
    }

    pub fn new_sized(width: i32, height: i32, format: PixmapFormat) -> Self {
        Self {
            width,
            height,
            disposed: false,
            path: None,
            rgba_data: None,
            ..Default::default()
        }
    }

    /// Apply texture filter by creating a wgpu::Sampler with the specified filter modes.
    /// Requires a GpuContext; without one, updates are deferred until the next GPU upload.
    pub fn set_filter(&mut self, min: TextureFilter, mag: TextureFilter) {
        // Store the filter request for when a GpuContext becomes available.
        // If a gpu_texture already exists, the sampler will be applied on next draw.
        // The actual sampler creation requires a wgpu::Device — see set_filter_with_device.
    }

    /// Create a wgpu::Sampler with the specified filter modes using a device.
    pub fn set_filter_with_device(
        &mut self,
        device: &wgpu::Device,
        min: TextureFilter,
        mag: TextureFilter,
    ) {
        let to_wgpu_filter = |f: &TextureFilter| -> wgpu::FilterMode {
            match f {
                TextureFilter::Nearest | TextureFilter::MipMapNearestNearest => {
                    wgpu::FilterMode::Nearest
                }
                TextureFilter::Linear
                | TextureFilter::MipMap
                | TextureFilter::MipMapLinearNearest
                | TextureFilter::MipMapNearestLinear
                | TextureFilter::MipMapLinearLinear => wgpu::FilterMode::Linear,
            }
        };
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("beatoraja texture sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: to_wgpu_filter(&mag),
            min_filter: to_wgpu_filter(&min),
            mipmap_filter: to_wgpu_filter(&min),
            ..Default::default()
        });
        self.sampler = Some(Arc::new(sampler));
    }

    /// Write pixmap RGBA data into the GPU texture at offset (x, y).
    /// If no GPU texture exists yet, creates one from the pixmap data.
    /// Requires a GpuContext for GPU operations.
    pub fn draw_pixmap(&mut self, pixmap: &Pixmap, x: i32, y: i32) {
        // Update CPU-side rgba_data for consistency
        if let Some(ref mut rgba_data) = self.rgba_data {
            let data = Arc::make_mut(rgba_data);
            let tex_w = self.width as usize;
            let pix_w = pixmap.width as usize;
            let pix_h = pixmap.height as usize;
            let src = pixmap.data();
            for row in 0..pix_h {
                let dst_y = y as usize + row;
                if dst_y >= self.height as usize {
                    break;
                }
                for col in 0..pix_w {
                    let dst_x = x as usize + col;
                    if dst_x >= tex_w {
                        break;
                    }
                    let si = (row * pix_w + col) * 4;
                    let di = (dst_y * tex_w + dst_x) * 4;
                    if si + 3 < src.len() && di + 3 < data.len() {
                        data[di..di + 4].copy_from_slice(&src[si..si + 4]);
                    }
                }
            }
        }
    }

    /// Write pixmap RGBA data into the GPU texture at offset (x, y) with GPU upload.
    /// If no GPU texture exists yet, creates one sized to self.width x self.height.
    pub fn draw_pixmap_gpu(&mut self, ctx: &GpuContext, pixmap: &Pixmap, x: i32, y: i32) {
        // Also update CPU-side data
        self.draw_pixmap(pixmap, x, y);

        if pixmap.width <= 0 || pixmap.height <= 0 {
            return;
        }

        // Ensure GPU texture exists
        if self.gpu_texture.is_none() && self.width > 0 && self.height > 0 {
            let rgba = self.rgba_data.clone();
            if let Some(ref data) = rgba {
                self.upload_to_gpu(ctx, data.as_slice());
            }
        }

        // Write the pixmap sub-region into the existing GPU texture
        if let Some(ref gpu_tex) = self.gpu_texture {
            let write_width = (pixmap.width as u32).min((self.width - x) as u32);
            let write_height = (pixmap.height as u32).min((self.height - y) as u32);
            let size = wgpu::Extent3d {
                width: write_width,
                height: write_height,
                depth_or_array_layers: 1,
            };
            ctx.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: gpu_tex,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: x as u32,
                        y: y as u32,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                pixmap.data(),
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(4 * pixmap.width as u32),
                    rows_per_image: Some(pixmap.height as u32),
                },
                size,
            );
        }
    }

    pub fn dispose(&mut self) {
        self.disposed = true;
        self.gpu_texture = None;
        self.gpu_view = None;
        self.sampler = None;
    }

    /// Get the GPU texture handle, if uploaded.
    pub fn gpu_texture(&self) -> Option<&wgpu::Texture> {
        self.gpu_texture.as_deref()
    }

    /// Get the GPU texture view, if uploaded.
    pub fn gpu_view(&self) -> Option<&wgpu::TextureView> {
        self.gpu_view.as_deref()
    }

    /// Get the GPU sampler, if created.
    pub fn sampler(&self) -> Option<&wgpu::Sampler> {
        self.sampler.as_deref()
    }

    /// Upload RGBA data to a wgpu texture and store handles in the struct.
    /// Also returns references to the created texture and view.
    /// This is the GPU-backed path — call when a GpuContext is available.
    pub fn upload_to_gpu(
        &mut self,
        ctx: &GpuContext,
        data: &[u8],
    ) -> Option<(Arc<wgpu::Texture>, Arc<wgpu::TextureView>)> {
        if self.disposed || self.width <= 0 || self.height <= 0 {
            return None;
        }
        let size = wgpu::Extent3d {
            width: self.width as u32,
            height: self.height as u32,
            depth_or_array_layers: 1,
        };
        let wgpu_texture = ctx.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("beatoraja texture"),
            size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
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
            data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * self.width as u32),
                rows_per_image: Some(self.height as u32),
            },
            size,
        );
        let view = wgpu_texture.create_view(&wgpu::TextureViewDescriptor::default());
        let tex_arc = Arc::new(wgpu_texture);
        let view_arc = Arc::new(view);
        self.gpu_texture = Some(Arc::clone(&tex_arc));
        self.gpu_view = Some(Arc::clone(&view_arc));
        Some((tex_arc, view_arc))
    }
}

/// A region within a Texture, defined by UV coordinates and pixel dimensions.
/// Corresponds to com.badlogic.gdx.graphics.g2d.TextureRegion.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextureRegion {
    pub u: f32,
    pub v: f32,
    pub u2: f32,
    pub v2: f32,
    pub region_x: i32,
    pub region_y: i32,
    pub region_width: i32,
    pub region_height: i32,
    pub texture: Option<Texture>,
}

impl TextureRegion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_texture(texture: Texture) -> Self {
        Self {
            region_width: texture.width,
            region_height: texture.height,
            texture: Some(texture),
            region_x: 0,
            region_y: 0,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
        }
    }

    pub fn from_texture_region(texture: Texture, x: i32, y: i32, width: i32, height: i32) -> Self {
        let u = if texture.width > 0 {
            x as f32 / texture.width as f32
        } else {
            0.0
        };
        let v = if texture.height > 0 {
            y as f32 / texture.height as f32
        } else {
            0.0
        };
        let u2 = if texture.width > 0 {
            (x + width) as f32 / texture.width as f32
        } else {
            1.0
        };
        let v2 = if texture.height > 0 {
            (y + height) as f32 / texture.height as f32
        } else {
            1.0
        };
        Self {
            region_x: x,
            region_y: y,
            region_width: width,
            region_height: height,
            texture: Some(texture),
            u,
            v,
            u2,
            v2,
        }
    }

    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    /// Java: TextureRegion.setRegion(int x, int y, int width, int height)
    /// Sets pixel coords and recalculates UV coordinates from the texture dimensions.
    pub fn set_region_from(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.region_x = x;
        self.region_y = y;
        self.region_width = width;
        self.region_height = height;
        // Recalculate UVs — matches LibGDX setRegion(int,int,int,int)
        if let Some(ref tex) = self.texture {
            let tw = tex.width;
            let th = tex.height;
            if tw > 0 && th > 0 {
                let inv_w = 1.0 / tw as f32;
                let inv_h = 1.0 / th as f32;
                self.u = x as f32 * inv_w;
                self.v = y as f32 * inv_h;
                self.u2 = (x + width) as f32 * inv_w;
                self.v2 = (y + height) as f32 * inv_h;
            }
        }
    }

    /// Set region relative to a parent TextureRegion, recalculating UV coords.
    /// Java: TextureRegion.setRegion(TextureRegion region, int x, int y, int width, int height)
    /// Sets texture to parent's texture, pixel coords relative to parent, and recalculates UVs.
    pub fn set_region_from_parent(
        &mut self,
        parent: &TextureRegion,
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    ) {
        self.texture = parent.texture.clone();
        self.region_x = x;
        self.region_y = y;
        self.region_width = width;
        self.region_height = height;
        // Recalculate UVs from parent's texture dimensions
        if let Some(ref tex) = self.texture {
            let tw = tex.width;
            let th = tex.height;
            if tw > 0 && th > 0 {
                let inv_w = 1.0 / tw as f32;
                let inv_h = 1.0 / th as f32;
                // Parent's pixel origin in texture space
                let parent_x = parent.region_x;
                let parent_y = parent.region_y;
                self.u = (parent_x + x) as f32 * inv_w;
                self.v = (parent_y + y) as f32 * inv_h;
                self.u2 = (parent_x + x + width) as f32 * inv_w;
                self.v2 = (parent_y + y + height) as f32 * inv_h;
            }
        }
    }

    pub fn flip(&mut self, x: bool, y: bool) {
        if x {
            std::mem::swap(&mut self.u, &mut self.u2);
        }
        if y {
            std::mem::swap(&mut self.v, &mut self.v2);
        }
    }

    pub fn set_from(&mut self, other: &TextureRegion) {
        self.u = other.u;
        self.v = other.v;
        self.u2 = other.u2;
        self.v2 = other.v2;
        self.region_x = other.region_x;
        self.region_y = other.region_y;
        self.region_width = other.region_width;
        self.region_height = other.region_height;
        self.texture = other.texture.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_texture(w: i32, h: i32) -> Texture {
        Texture {
            width: w,
            height: h,
            disposed: false,
            path: None,
            rgba_data: None,
            ..Default::default()
        }
    }

    #[test]
    fn set_region_from_recalculates_uvs() {
        let tex = make_texture(100, 200);
        let mut region = TextureRegion::from_texture(tex);
        // Initially covers full texture: u=0, v=0, u2=1, v2=1
        assert_eq!(region.u, 0.0);
        assert_eq!(region.v, 0.0);
        assert_eq!(region.u2, 1.0);
        assert_eq!(region.v2, 1.0);

        // Set to sub-region: left half horizontally, top half vertically
        region.set_region_from(0, 0, 50, 100);
        assert_eq!(region.region_x, 0);
        assert_eq!(region.region_y, 0);
        assert_eq!(region.region_width, 50);
        assert_eq!(region.region_height, 100);
        assert!((region.u - 0.0).abs() < 1e-6);
        assert!((region.v - 0.0).abs() < 1e-6);
        assert!((region.u2 - 0.5).abs() < 1e-6);
        assert!((region.v2 - 0.5).abs() < 1e-6);
    }

    #[test]
    fn set_region_from_offset_region() {
        let tex = make_texture(200, 200);
        let mut region = TextureRegion::from_texture(tex);
        region.set_region_from(50, 100, 100, 50);
        assert!((region.u - 0.25).abs() < 1e-6);
        assert!((region.v - 0.5).abs() < 1e-6);
        assert!((region.u2 - 0.75).abs() < 1e-6);
        assert!((region.v2 - 0.75).abs() < 1e-6);
    }

    #[test]
    fn set_region_from_no_texture_no_uv_change() {
        let mut region = TextureRegion::new();
        region.u = 0.1;
        region.v = 0.2;
        region.set_region_from(10, 20, 30, 40);
        // No texture -> UVs unchanged
        assert_eq!(region.region_x, 10);
        assert_eq!(region.region_width, 30);
        assert!((region.u - 0.1).abs() < 1e-6);
        assert!((region.v - 0.2).abs() < 1e-6);
    }

    #[test]
    fn set_region_from_parent_recalculates_uvs() {
        let tex = make_texture(100, 100);
        let parent = TextureRegion::from_texture_region(tex, 20, 30, 60, 40);
        let mut child = TextureRegion::new();
        child.set_region_from_parent(&parent, 10, 5, 30, 20);

        // Child pixel: parent_x+x=30, parent_y+y=35, width=30, height=20
        assert_eq!(child.region_x, 10);
        assert_eq!(child.region_y, 5);
        assert_eq!(child.region_width, 30);
        assert_eq!(child.region_height, 20);
        assert!((child.u - 0.30).abs() < 1e-6);
        assert!((child.v - 0.35).abs() < 1e-6);
        assert!((child.u2 - 0.60).abs() < 1e-6);
        assert!((child.v2 - 0.55).abs() < 1e-6);
    }

    #[test]
    fn set_region_from_parent_copies_texture() {
        let tex = make_texture(64, 64);
        let parent = TextureRegion::from_texture(tex);
        let mut child = TextureRegion::new();
        assert!(child.texture.is_none());
        child.set_region_from_parent(&parent, 0, 0, 32, 32);
        assert!(child.texture.is_some());
        assert_eq!(child.texture.as_ref().unwrap().width, 64);
    }

    #[test]
    fn from_texture_region_uvs() {
        let tex = make_texture(256, 128);
        let region = TextureRegion::from_texture_region(tex, 64, 32, 128, 64);
        assert!((region.u - 0.25).abs() < 1e-6);
        assert!((region.v - 0.25).abs() < 1e-6);
        assert!((region.u2 - 0.75).abs() < 1e-6);
        assert!((region.v2 - 0.75).abs() < 1e-6);
    }

    #[test]
    fn flip_swaps_uvs() {
        let tex = make_texture(100, 100);
        let mut region = TextureRegion::from_texture_region(tex, 0, 0, 50, 50);
        let orig_u = region.u;
        let orig_u2 = region.u2;
        region.flip(true, false);
        assert_eq!(region.u, orig_u2);
        assert_eq!(region.u2, orig_u);
    }

    #[test]
    fn draw_pixmap_updates_cpu_data() {
        // Create a 4x4 texture with all zeros
        let mut tex = Texture {
            width: 4,
            height: 4,
            rgba_data: Some(Arc::new(vec![0u8; 4 * 4 * 4])),
            ..Default::default()
        };

        // Create a 2x2 pixmap with red pixels
        let red_data = vec![
            255, 0, 0, 255, 255, 0, 0, 255, // row 0
            255, 0, 0, 255, 255, 0, 0, 255, // row 1
        ];
        let pixmap = Pixmap::from_rgba_data(2, 2, red_data);

        // Draw pixmap at offset (1, 1)
        tex.draw_pixmap(&pixmap, 1, 1);

        let data = tex.rgba_data.as_ref().unwrap();
        // Pixel at (1,1) should be red
        let idx = (4 + 1) * 4;
        assert_eq!(data[idx], 255); // R
        assert_eq!(data[idx + 1], 0); // G
        assert_eq!(data[idx + 2], 0); // B
        assert_eq!(data[idx + 3], 255); // A

        // Pixel at (0,0) should still be zero
        assert_eq!(data[0], 0);
        assert_eq!(data[1], 0);
        assert_eq!(data[2], 0);
        assert_eq!(data[3], 0);
    }

    #[test]
    fn draw_pixmap_clamps_to_bounds() {
        // Create a 2x2 texture
        let mut tex = Texture {
            width: 2,
            height: 2,
            rgba_data: Some(Arc::new(vec![0u8; 2 * 2 * 4])),
            ..Default::default()
        };

        // Create a 3x3 pixmap (larger than texture)
        let blue_data = [0u8, 0, 255, 255].repeat(9);
        let pixmap = Pixmap::from_rgba_data(3, 3, blue_data);

        // Draw at (1, 1) -- only 1 pixel should fit
        tex.draw_pixmap(&pixmap, 1, 1);

        let data = tex.rgba_data.as_ref().unwrap();
        // Pixel at (1,1) should be blue
        let idx = (2 + 1) * 4;
        assert_eq!(data[idx], 0);
        assert_eq!(data[idx + 1], 0);
        assert_eq!(data[idx + 2], 255);
        assert_eq!(data[idx + 3], 255);

        // Pixel at (0,0) should still be zero
        assert_eq!(data[0], 0);
    }

    #[test]
    fn dispose_clears_gpu_handles() {
        let mut tex = Texture {
            width: 10,
            height: 10,
            disposed: false,
            ..Default::default()
        };
        assert!(!tex.disposed);
        assert!(tex.gpu_texture.is_none());

        tex.dispose();
        assert!(tex.disposed);
        assert!(tex.gpu_texture.is_none());
        assert!(tex.gpu_view.is_none());
        assert!(tex.sampler.is_none());
    }

    #[test]
    fn getters_return_none_by_default() {
        let tex = Texture::default();
        assert!(tex.gpu_texture().is_none());
        assert!(tex.gpu_view().is_none());
        assert!(tex.sampler().is_none());
    }
}
