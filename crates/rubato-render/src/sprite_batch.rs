// Batched 2D quad renderer.
// Drop-in replacement for the SpriteBatch stub in rendering_stubs.rs.

use std::collections::HashMap;
use std::sync::Arc;

use crate::blend::BlendMode;
use crate::color::{Color, Matrix4};
use crate::gpu_texture_manager::{GpuTextureManager, PendingTexture};
use crate::render_pipeline::SpriteRenderPipeline;
use crate::shader::ShaderProgram;
use crate::texture::{Texture, TextureRegion};

/// Vertex for a 2D sprite quad.
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct SpriteVertex {
    pub position: [f32; 2],
    pub tex_coord: [f32; 2],
    pub color: [f32; 4],
}

impl SpriteVertex {
    /// Returns the wgpu vertex buffer layout for SpriteVertex.
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<SpriteVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 8,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float32x2,
                },
                wgpu::VertexAttribute {
                    offset: 16,
                    shader_location: 2,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

/// Maximum number of sprites per batch before auto-flush.
/// Java LibGDX default: 1000 sprites = 6000 vertices.
const MAX_SPRITES: usize = 1000;
const MAX_VERTICES: usize = MAX_SPRITES * 6;

/// A contiguous range of vertices that share the same texture/shader/blend state.
#[derive(Debug)]
struct DrawBatch {
    texture_key: Option<Arc<str>>,
    pub shader_type: i32,
    blend_mode: BlendMode,
    vertex_start: u32,
    vertex_count: u32,
}

/// Batched 2D sprite renderer.
/// Corresponds to com.badlogic.gdx.graphics.g2d.SpriteBatch.
///
/// Collects sprite draw calls into a vertex buffer. Actual GPU submission
/// happens when `flush()` is called or when the batch reaches capacity.
#[derive(Debug, Default)]
pub struct SpriteBatch {
    vertices: Vec<SpriteVertex>,
    draw_batches: Vec<DrawBatch>,
    /// Textures encountered during draw calls, waiting for GPU upload
    pending_textures: HashMap<Arc<str>, PendingTexture>,
    current_color: [f32; 4],
    blend_src: i32,
    blend_dst: i32,
    projection: [f32; 16],
    pub drawing: bool,
    /// Current shader type (matches SkinObjectRenderer TYPE_* constants)
    pub shader_type: i32,
    /// Current blend mode derived from blend_src/blend_dst
    blend_mode: BlendMode,
    /// Persistent GPU vertex buffer, reused across frames.
    /// Grows geometrically (doubles) when capacity is insufficient.
    gpu_vertex_buffer: Option<wgpu::Buffer>,
    /// Current capacity of `gpu_vertex_buffer` in bytes.
    gpu_vertex_buffer_capacity: u64,
}

#[allow(unused_variables)]
impl SpriteBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(MAX_VERTICES),
            draw_batches: Vec::new(),
            pending_textures: HashMap::new(),
            current_color: [1.0, 1.0, 1.0, 1.0],
            blend_src: 0x0302, // GL_SRC_ALPHA
            blend_dst: 0x0303, // GL_ONE_MINUS_SRC_ALPHA
            projection: Matrix4::default().values,
            drawing: false,
            shader_type: 0,
            blend_mode: BlendMode::Normal,
            gpu_vertex_buffer: None,
            gpu_vertex_buffer_capacity: 0,
        }
    }

    /// Set the projection/transform matrix for the batch.
    /// Java: SpriteBatch.setTransformMatrix(matrix)
    pub fn set_transform_matrix(&mut self, matrix: &Matrix4) {
        self.projection = matrix.values;
    }

    /// Set the projection matrix directly.
    /// Java: SpriteBatch.setProjectionMatrix(matrix)
    pub fn set_projection_matrix(&mut self, matrix: &Matrix4) {
        self.projection = matrix.values;
    }

    pub fn set_shader(&mut self, shader: Option<&ShaderProgram>) {
        // Shader switching is handled by shader_type in the render pipeline.
    }

    /// Get the shader type for subsequent draw calls.
    /// Matches Java SkinObjectRenderer shader switching.
    pub fn shader_type(&self) -> i32 {
        self.shader_type
    }

    pub fn set_color(&mut self, color: &Color) {
        self.current_color = color.to_array();
    }

    pub fn color(&self) -> Color {
        Color::new(
            self.current_color[0],
            self.current_color[1],
            self.current_color[2],
            self.current_color[3],
        )
    }

    /// Java: SpriteBatch.setBlendFunction(src, dst)
    pub fn set_blend_function(&mut self, src: i32, dst: i32) {
        self.blend_src = src;
        self.blend_dst = dst;
        self.blend_mode = BlendMode::from_gl_factors(src, dst);
    }

    pub fn blend_mode(&self) -> BlendMode {
        self.blend_mode
    }

    /// Begin batching. Must be called before any draw calls.
    /// Java: SpriteBatch.begin()
    pub fn begin(&mut self) {
        self.drawing = true;
    }

    /// End batching. Flushes any remaining vertices.
    /// Java: SpriteBatch.end()
    pub fn end(&mut self) {
        self.drawing = false;
    }

    /// Flush the current batch to GPU.
    /// Java: SpriteBatch.flush()
    ///
    /// CPU-side: clears the vertex buffer. Actual GPU submission is done by
    /// `flush_to_gpu()` which requires a render pass.
    pub fn flush(&mut self) {
        self.vertices.clear();
        self.draw_batches.clear();
    }

    /// Flush batched vertices to GPU via a render pass.
    ///
    /// This is the actual GPU submission path. Reuses a persistent vertex
    /// buffer (growing geometrically when needed), binds the appropriate
    /// pipeline, and issues per-batch draw calls with the correct texture
    /// bind group for each batch.
    #[allow(clippy::too_many_arguments)]
    pub fn flush_to_gpu<'a>(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        pipeline: &'a SpriteRenderPipeline,
        uniform_bind_group: &'a wgpu::BindGroup,
        texture_manager: &'a GpuTextureManager,
    ) {
        if self.vertices.is_empty() {
            return;
        }

        // Reuse persistent vertex buffer; grow geometrically when needed
        let vertex_data: &[u8] = bytemuck::cast_slice(&self.vertices);
        let required_size = vertex_data.len() as u64;

        if self.gpu_vertex_buffer_capacity < required_size {
            // Grow to at least double the current capacity, or the required size
            let new_capacity = required_size.max(self.gpu_vertex_buffer_capacity * 2);
            self.gpu_vertex_buffer = Some(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("sprite vertex buffer"),
                size: new_capacity,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            self.gpu_vertex_buffer_capacity = new_capacity;
        }

        let vertex_buffer = self
            .gpu_vertex_buffer
            .as_ref()
            .expect("gpu_vertex_buffer is Some");
        queue.write_buffer(vertex_buffer, 0, vertex_data);

        render_pass.set_bind_group(0, uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..required_size));

        // If no draw batches recorded, fall back to single-batch rendering
        if self.draw_batches.is_empty() {
            let bind_group = texture_manager.bind_group(None, self.shader_type);
            if let Some(render_pipeline) = pipeline.pipeline(self.shader_type, self.blend_mode) {
                render_pass.set_pipeline(render_pipeline);
                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.draw(0..self.vertices.len() as u32, 0..1);
            }
        } else {
            // Issue one draw call per batch with the correct texture/pipeline
            for batch in &self.draw_batches {
                if batch.vertex_count == 0 {
                    continue;
                }
                let bind_group =
                    texture_manager.bind_group(batch.texture_key.as_ref(), batch.shader_type);
                if let Some(render_pipeline) =
                    pipeline.pipeline(batch.shader_type, batch.blend_mode)
                {
                    render_pass.set_pipeline(render_pipeline);
                    render_pass.set_bind_group(1, bind_group, &[]);
                    let start = batch.vertex_start;
                    let end = start + batch.vertex_count;
                    render_pass.draw(start..end, 0..1);
                }
            }
        }

        self.vertices.clear();
        self.draw_batches.clear();
    }

    /// Get the projection matrix values.
    pub fn projection(&self) -> &[f32; 16] {
        &self.projection
    }

    /// Draw a full texture at (x, y) with size (w, h).
    pub fn draw_texture(&mut self, texture: &Texture, x: f32, y: f32, w: f32, h: f32) {
        self.record_texture(texture);
        self.push_quad(x, y, w, h, 0.0, 0.0, 1.0, 1.0);
    }

    /// Draw a texture region at (x, y) with size (w, h).
    pub fn draw_region(&mut self, region: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
        if let Some(tex) = &region.texture {
            self.record_texture(tex);
        } else {
            self.ensure_batch(None);
        }
        self.push_quad(x, y, w, h, region.u, region.v, region.u2, region.v2);
    }

    /// Draw a texture region with rotation and scale.
    #[allow(clippy::too_many_arguments)]
    pub fn draw_region_rotated(
        &mut self,
        region: &TextureRegion,
        x: f32,
        y: f32,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        sx: f32,
        sy: f32,
        angle: f32,
    ) {
        if let Some(tex) = &region.texture {
            self.record_texture(tex);
        } else {
            self.ensure_batch(None);
        }

        let cos = angle.to_radians().cos();
        let sin = angle.to_radians().sin();

        // Compute corner offsets from origin, apply scale
        let corners: [(f32, f32); 4] = [
            (-cx * sx, -cy * sy),
            ((w - cx) * sx, -cy * sy),
            ((w - cx) * sx, (h - cy) * sy),
            (-cx * sx, (h - cy) * sy),
        ];

        let color = self.current_color;
        let (u1, v1, u2, v2) = (region.u, region.v, region.u2, region.v2);
        let uvs = [(u1, v1), (u2, v1), (u2, v2), (u1, v2)];

        // Two triangles: 0-1-2, 0-2-3
        let vertex_count_before = self.vertices.len();
        for &idx in &[0, 1, 2, 0, 2, 3] {
            let (ox, oy) = corners[idx];
            let px = x + cx + ox * cos - oy * sin;
            let py = y + cy + ox * sin + oy * cos;
            self.vertices.push(SpriteVertex {
                position: [px, py],
                tex_coord: [uvs[idx].0, uvs[idx].1],
                color,
            });
        }
        // Update vertex count of the current draw batch
        let added = (self.vertices.len() - vertex_count_before) as u32;
        if let Some(batch) = self.draw_batches.last_mut() {
            batch.vertex_count += added;
        }
    }

    /// Get the raw vertex data for GPU upload.
    pub fn vertices(&self) -> &[SpriteVertex] {
        &self.vertices
    }

    /// Drain pending textures that need GPU upload.
    pub fn drain_pending_textures(&mut self) -> HashMap<Arc<str>, PendingTexture> {
        std::mem::take(&mut self.pending_textures)
    }

    /// Record a texture for the current draw call and manage batch boundaries.
    fn record_texture(&mut self, texture: &Texture) {
        let key = texture.path.clone();

        // Register pending texture for GPU upload if it has rgba data
        if let Some(ref path) = key
            && !self.pending_textures.contains_key(path)
            && let Some(ref rgba_data) = texture.rgba_data
        {
            self.pending_textures.insert(
                Arc::clone(path),
                PendingTexture {
                    width: texture.width as u32,
                    height: texture.height as u32,
                    rgba_data: Arc::clone(rgba_data),
                },
            );
        }

        self.ensure_batch(key);
    }

    /// Ensure a draw batch exists for the current texture/shader/blend state.
    /// If the current batch has different state, start a new batch.
    fn ensure_batch(&mut self, texture_key: Option<Arc<str>>) {
        let needs_new_batch = if let Some(last) = self.draw_batches.last() {
            last.texture_key != texture_key
                || last.shader_type != self.shader_type
                || last.blend_mode != self.blend_mode
        } else {
            true
        };

        if needs_new_batch {
            self.draw_batches.push(DrawBatch {
                texture_key,
                shader_type: self.shader_type,
                blend_mode: self.blend_mode,
                vertex_start: self.vertices.len() as u32,
                vertex_count: 0,
            });
        }
    }

    #[cfg(test)]
    pub fn draw_batch_count(&self) -> usize {
        self.draw_batches.len()
    }

    /// Push a simple axis-aligned quad.
    #[allow(clippy::too_many_arguments)]
    fn push_quad(&mut self, x: f32, y: f32, w: f32, h: f32, u1: f32, v1: f32, u2: f32, v2: f32) {
        let color = self.current_color;
        // Two triangles: top-left, top-right, bottom-right, top-left, bottom-right, bottom-left
        let verts = [
            SpriteVertex {
                position: [x, y],
                tex_coord: [u1, v1],
                color,
            },
            SpriteVertex {
                position: [x + w, y],
                tex_coord: [u2, v1],
                color,
            },
            SpriteVertex {
                position: [x + w, y + h],
                tex_coord: [u2, v2],
                color,
            },
            SpriteVertex {
                position: [x, y],
                tex_coord: [u1, v1],
                color,
            },
            SpriteVertex {
                position: [x + w, y + h],
                tex_coord: [u2, v2],
                color,
            },
            SpriteVertex {
                position: [x, y + h],
                tex_coord: [u1, v2],
                color,
            },
        ];
        self.vertices.extend_from_slice(&verts);
        // Update vertex count of the current draw batch
        if let Some(batch) = self.draw_batches.last_mut() {
            batch.vertex_count += verts.len() as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sprite_batch_new() {
        let batch = SpriteBatch::new();
        assert!(batch.vertices().is_empty());
        assert!(!batch.drawing);
        assert_eq!(batch.blend_mode(), BlendMode::Normal);
    }

    #[test]
    fn test_sprite_batch_begin_end() {
        let mut batch = SpriteBatch::new();
        batch.begin();
        assert!(batch.drawing);
        batch.end();
        assert!(!batch.drawing);
    }

    #[test]
    fn test_sprite_batch_draw_texture_generates_6_vertices() {
        let mut batch = SpriteBatch::new();
        let tex = Texture::default();
        batch.draw_texture(&tex, 10.0, 20.0, 100.0, 50.0);
        // 1 quad = 2 triangles = 6 vertices
        assert_eq!(batch.vertices().len(), 6);
    }

    #[test]
    fn test_sprite_batch_draw_region_generates_6_vertices() {
        let mut batch = SpriteBatch::new();
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..Default::default()
        };
        batch.draw_region(&region, 0.0, 0.0, 64.0, 64.0);
        assert_eq!(batch.vertices().len(), 6);
    }

    #[test]
    fn test_sprite_batch_draw_region_vertex_positions() {
        let mut batch = SpriteBatch::new();
        let region = TextureRegion {
            u: 0.25,
            v: 0.25,
            u2: 0.75,
            v2: 0.75,
            ..Default::default()
        };
        batch.draw_region(&region, 10.0, 20.0, 30.0, 40.0);
        let verts = batch.vertices();
        // Check triangle corners: (10,20), (40,20), (40,60), (10,20), (40,60), (10,60)
        assert_eq!(verts[0].position, [10.0, 20.0]);
        assert_eq!(verts[1].position, [40.0, 20.0]);
        assert_eq!(verts[2].position, [40.0, 60.0]);
        assert_eq!(verts[3].position, [10.0, 20.0]);
        assert_eq!(verts[4].position, [40.0, 60.0]);
        assert_eq!(verts[5].position, [10.0, 60.0]);
        // Check UV coords
        assert_eq!(verts[0].tex_coord, [0.25, 0.25]);
        assert_eq!(verts[2].tex_coord, [0.75, 0.75]);
    }

    #[test]
    fn test_sprite_batch_color() {
        let mut batch = SpriteBatch::new();
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        batch.set_color(&red);
        let got = batch.color();
        assert_eq!(got.r, 1.0);
        assert_eq!(got.g, 0.0);
        assert_eq!(got.b, 0.0);
        assert_eq!(got.a, 1.0);
    }

    #[test]
    fn test_sprite_batch_draw_uses_current_color() {
        let mut batch = SpriteBatch::new();
        let green = Color::new(0.0, 1.0, 0.0, 0.5);
        batch.set_color(&green);
        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        for v in batch.vertices() {
            assert_eq!(v.color, [0.0, 1.0, 0.0, 0.5]);
        }
    }

    #[test]
    fn test_sprite_batch_flush_clears_vertices() {
        let mut batch = SpriteBatch::new();
        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(batch.vertices().len(), 6);
        batch.flush();
        assert!(batch.vertices().is_empty());
    }

    #[test]
    fn test_sprite_batch_multiple_draws_accumulate() {
        let mut batch = SpriteBatch::new();
        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        batch.draw_texture(&tex, 20.0, 20.0, 10.0, 10.0);
        // 2 quads = 12 vertices
        assert_eq!(batch.vertices().len(), 12);
    }

    #[test]
    fn test_sprite_batch_blend_mode_from_gl_factors() {
        let mut batch = SpriteBatch::new();
        // Default: Normal
        assert_eq!(batch.blend_mode(), BlendMode::Normal);
        // Additive: SRC_ALPHA, ONE
        batch.set_blend_function(0x0302, 1);
        assert_eq!(batch.blend_mode(), BlendMode::Additive);
        // Multiply: ZERO, SRC_COLOR
        batch.set_blend_function(0, 0x0300);
        assert_eq!(batch.blend_mode(), BlendMode::Multiply);
        // Inversion: ONE_MINUS_DST_COLOR, ZERO
        batch.set_blend_function(0x0307, 0);
        assert_eq!(batch.blend_mode(), BlendMode::Inversion);
        // Reset to normal
        batch.set_blend_function(0x0302, 0x0303);
        assert_eq!(batch.blend_mode(), BlendMode::Normal);
    }

    #[test]
    fn test_sprite_batch_shader_type() {
        let mut batch = SpriteBatch::new();
        assert_eq!(batch.shader_type(), 0);
        batch.shader_type = 3;
        assert_eq!(batch.shader_type(), 3);
    }

    #[test]
    fn test_sprite_batch_projection_matrix() {
        let mut batch = SpriteBatch::new();
        let mut mat = Matrix4::new();
        mat.set_to_ortho(0.0, 1920.0, 0.0, 1080.0, -1.0, 1.0);
        batch.set_projection_matrix(&mat);
        assert_eq!(batch.projection()[0], 2.0 / 1920.0);
        assert_eq!(batch.projection()[5], 2.0 / 1080.0);
    }

    #[test]
    fn test_sprite_batch_rotated_generates_6_vertices() {
        let mut batch = SpriteBatch::new();
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..Default::default()
        };
        batch.draw_region_rotated(
            &region, 100.0, 100.0, 32.0, 32.0, 64.0, 64.0, 1.0, 1.0, 45.0,
        );
        assert_eq!(batch.vertices().len(), 6);
    }

    #[test]
    fn test_sprite_vertex_layout() {
        let layout = SpriteVertex::desc();
        // stride = 2*4 + 2*4 + 4*4 = 32 bytes
        assert_eq!(
            layout.array_stride,
            std::mem::size_of::<SpriteVertex>() as u64
        );
        assert_eq!(layout.attributes.len(), 3);
        assert_eq!(layout.attributes[0].offset, 0);
        assert_eq!(layout.attributes[1].offset, 8);
        assert_eq!(layout.attributes[2].offset, 16);
    }

    #[test]
    fn test_draw_batches_split_on_texture_change() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        // Draw with texture A
        let tex_a = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("tex_a")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        let region_a = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            texture: Some(tex_a),
        };
        batch.draw_region(&region_a, 0.0, 0.0, 10.0, 10.0);

        // Draw with texture B
        let tex_b = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("tex_b")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        let region_b = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            texture: Some(tex_b),
        };
        batch.draw_region(&region_b, 20.0, 0.0, 10.0, 10.0);

        batch.end();

        assert_eq!(batch.vertices().len(), 12, "two quads = 12 vertices");
        assert_eq!(
            batch.draw_batch_count(),
            2,
            "different textures = 2 batches"
        );
    }

    #[test]
    fn test_draw_batches_same_texture_single_batch() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("tex_same")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 10,
            region_height: 10,
            texture: Some(tex),
        };
        batch.draw_region(&region, 0.0, 0.0, 10.0, 10.0);
        batch.draw_region(&region, 20.0, 0.0, 10.0, 10.0);

        batch.end();

        assert_eq!(batch.vertices().len(), 12);
        assert_eq!(batch.draw_batch_count(), 1, "same texture = 1 batch");
    }

    #[test]
    fn test_pending_textures_registered_on_draw() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture {
            width: 8,
            height: 8,
            disposed: false,
            path: Some(Arc::from("test_pending")),
            rgba_data: Some(Arc::new(vec![0u8; 256])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 8,
            region_height: 8,
            texture: Some(tex.clone()),
        };
        batch.draw_region(&region, 0.0, 0.0, 8.0, 8.0);
        // Draw again with same texture - should NOT duplicate
        batch.draw_region(&region, 10.0, 0.0, 8.0, 8.0);

        let pending = batch.drain_pending_textures();
        assert_eq!(pending.len(), 1, "same texture registered only once");
        assert!(pending.contains_key(&Arc::from("test_pending") as &Arc<str>));

        batch.end();
    }

    #[test]
    fn test_drain_pending_textures_clears() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("drain_test")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            region_x: 0,
            region_y: 0,
            region_width: 4,
            region_height: 4,
            texture: Some(tex),
        };
        batch.draw_region(&region, 0.0, 0.0, 4.0, 4.0);

        let first = batch.drain_pending_textures();
        assert_eq!(first.len(), 1);

        let second = batch.drain_pending_textures();
        assert!(second.is_empty(), "drain should clear pending textures");

        batch.end();
    }
}
