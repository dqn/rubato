// Batched 2D quad renderer.
// Drop-in replacement for the SpriteBatch stub in render_reexports.rs.

use std::collections::HashMap;
use std::sync::Arc;

use crate::blend::BlendMode;
use crate::color::{Color, Matrix4};
use crate::gpu_texture_manager::{GpuTextureManager, PendingTexture};
use crate::render_pipeline::SpriteRenderPipeline;
use crate::shader::ShaderProgram;
use crate::texture::{Texture, TextureRegion};

/// A captured draw quad for GPU-free verification in E2E tests.
/// Records the position, size, color, texture, and blend mode of each
/// quad submitted to the SpriteBatch.
#[derive(Debug, Clone)]
pub struct CapturedDrawQuad {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub color: [f32; 4],
    pub texture_key: Option<String>,
    pub blend_mode: BlendMode,
}

/// Transform parameters for rotated sprite drawing.
pub struct SpriteTransform {
    pub x: f32,
    pub y: f32,
    pub center_x: f32,
    pub center_y: f32,
    pub width: f32,
    pub height: f32,
    pub scale_x: f32,
    pub scale_y: f32,
    pub angle: f32,
}

/// UV coordinate rectangle.
pub(crate) struct UVRect {
    pub u1: f32,
    pub v1: f32,
    pub u2: f32,
    pub v2: f32,
}

/// GPU resources needed for flushing sprite batches.
pub struct GpuRenderContext<'a> {
    pub device: &'a wgpu::Device,
    pub queue: &'a wgpu::Queue,
    pub pipeline: &'a SpriteRenderPipeline,
    pub uniform_bind_group: &'a wgpu::BindGroup,
    pub texture_manager: &'a GpuTextureManager,
}

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

/// A completed vertex segment ready for GPU submission.
/// Created when the active vertex buffer reaches MAX_VERTICES capacity.
#[derive(Debug)]
struct FlushedSegment {
    vertices: Vec<SpriteVertex>,
    draw_batches: Vec<DrawBatch>,
    pending_textures: HashMap<Arc<str>, PendingTexture>,
}

/// Batched 2D sprite renderer.
/// Corresponds to com.badlogic.gdx.graphics.g2d.SpriteBatch.
///
/// Collects sprite draw calls into a vertex buffer. Actual GPU submission
/// happens when `flush()` is called or when the batch reaches capacity.
/// When the active vertex buffer reaches MAX_VERTICES, it is moved into
/// `flushed_segments` and a fresh buffer is started, matching Java LibGDX's
/// auto-flush behavior (`if (idx >= vertices.length) flush()`).
#[derive(Debug, Default)]
pub struct SpriteBatch {
    vertices: Vec<SpriteVertex>,
    draw_batches: Vec<DrawBatch>,
    /// Textures encountered during draw calls, waiting for GPU upload
    pending_textures: HashMap<Arc<str>, PendingTexture>,
    /// Completed vertex segments waiting for GPU submission.
    /// Created by auto-flush when the active buffer reaches MAX_VERTICES.
    flushed_segments: Vec<FlushedSegment>,
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
    /// Optional capture buffer for GPU-free draw verification.
    /// When `Some`, every draw call records a `CapturedDrawQuad`.
    /// Zero overhead when `None` (just an Option check).
    capture_buffer: Option<Vec<CapturedDrawQuad>>,
}

#[allow(unused_variables)]
impl SpriteBatch {
    pub fn new() -> Self {
        Self {
            vertices: Vec::with_capacity(MAX_VERTICES),
            draw_batches: Vec::new(),
            pending_textures: HashMap::new(),
            flushed_segments: Vec::new(),
            current_color: [1.0, 1.0, 1.0, 1.0],
            blend_src: 0x0302, // GL_SRC_ALPHA
            blend_dst: 0x0303, // GL_ONE_MINUS_SRC_ALPHA
            projection: Matrix4::default().values,
            drawing: false,
            shader_type: 0,
            blend_mode: BlendMode::Normal,
            gpu_vertex_buffer: None,
            gpu_vertex_buffer_capacity: 0,
            capture_buffer: None,
        }
    }

    /// Returns the total number of vertices pending GPU submission,
    /// including both auto-flushed segments and the active buffer.
    pub fn vertex_count(&self) -> usize {
        let flushed: usize = self.flushed_segments.iter().map(|s| s.vertices.len()).sum();
        flushed + self.vertices.len()
    }

    /// Returns true if there are vertices pending GPU submission,
    /// either in auto-flushed segments or the active buffer.
    pub fn has_pending_draw_data(&self) -> bool {
        !self.vertices.is_empty() || !self.flushed_segments.is_empty()
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

    // ============================================================
    // Draw capture (GPU-free observability for E2E tests)
    // ============================================================

    /// Enable draw capture. All subsequent draw calls will record a
    /// `CapturedDrawQuad` into an internal buffer.
    pub fn enable_capture(&mut self) {
        self.capture_buffer = Some(Vec::new());
    }

    /// Disable draw capture and drop the capture buffer.
    pub fn disable_capture(&mut self) {
        self.capture_buffer = None;
    }

    /// Return the captured quads, or an empty slice if capture is disabled.
    pub fn captured_quads(&self) -> &[CapturedDrawQuad] {
        self.capture_buffer.as_deref().unwrap_or(&[])
    }

    /// Clear the capture buffer without disabling capture.
    pub fn clear_captured(&mut self) {
        if let Some(ref mut buf) = self.capture_buffer {
            buf.clear();
        }
    }

    /// Directly set the blend mode, bypassing GL factor mapping.
    /// Use when the desired blend mode cannot be distinguished by GL factors alone
    /// (e.g. Subtractive shares factors with Additive but uses a different blend equation).
    pub fn set_blend_mode(&mut self, mode: BlendMode) {
        self.blend_mode = mode;
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
        self.pending_textures.clear();
        self.flushed_segments.clear();
    }

    /// Auto-flush: move the current active buffer into a completed segment
    /// and start fresh. Called when the vertex buffer reaches MAX_VERTICES.
    /// Matches Java LibGDX's `if (idx >= vertices.length) flush()`.
    fn auto_flush(&mut self) {
        if self.vertices.is_empty() {
            return;
        }
        let vertices = std::mem::replace(&mut self.vertices, Vec::with_capacity(MAX_VERTICES));
        let draw_batches = std::mem::take(&mut self.draw_batches);
        let pending_textures = std::mem::take(&mut self.pending_textures);
        self.flushed_segments.push(FlushedSegment {
            vertices,
            draw_batches,
            pending_textures,
        });
    }

    /// Flush batched vertices to GPU via a render pass.
    ///
    /// This is the actual GPU submission path. Processes any auto-flushed
    /// segments first, then the active buffer. Each segment is uploaded and
    /// drawn separately, keeping individual GPU uploads bounded to
    /// MAX_VERTICES. Reuses a persistent vertex buffer (growing
    /// geometrically when needed).
    ///
    /// # Required call sequence
    ///
    /// Before calling this method, callers **must**:
    /// 1. Call [`drain_pending_textures()`](Self::drain_pending_textures) to
    ///    collect textures that need GPU upload.
    /// 2. Upload them via `GpuTextureManager::ensure_uploaded()`.
    ///
    /// If pending textures have not been drained and uploaded, segments will
    /// render with missing bind groups, falling back to the white texture.
    pub fn flush_to_gpu<'a>(
        &mut self,
        render_pass: &mut wgpu::RenderPass<'a>,
        ctx: &'a GpuRenderContext<'a>,
    ) {
        debug_assert!(
            self.pending_textures.is_empty(),
            "flush_to_gpu called with undrained pending_textures in active buffer; \
             call drain_pending_textures() + ensure_uploaded() first"
        );
        debug_assert!(
            self.flushed_segments
                .iter()
                .all(|s| s.pending_textures.is_empty()),
            "flush_to_gpu called with undrained pending_textures in flushed segments; \
             call drain_pending_textures() + ensure_uploaded() first"
        );
        // Move the active buffer into a final segment so we can process
        // everything uniformly. This avoids borrow conflicts between
        // the vertex data and the GPU buffer fields on `self`.
        if !self.vertices.is_empty() {
            let vertices = std::mem::replace(&mut self.vertices, Vec::with_capacity(MAX_VERTICES));
            let draw_batches = std::mem::take(&mut self.draw_batches);
            let pending_textures = std::mem::take(&mut self.pending_textures);
            self.flushed_segments.push(FlushedSegment {
                vertices,
                draw_batches,
                pending_textures,
            });
        }

        // Process all segments in submission order
        let segments = std::mem::take(&mut self.flushed_segments);
        for segment in &segments {
            self.flush_segment_to_gpu(&segment.vertices, &segment.draw_batches, render_pass, ctx);
        }

        self.vertices.clear();
        self.draw_batches.clear();
        self.pending_textures.clear();
    }

    /// Upload and draw a single vertex segment.
    fn flush_segment_to_gpu<'a>(
        &mut self,
        vertices: &[SpriteVertex],
        draw_batches: &[DrawBatch],
        render_pass: &mut wgpu::RenderPass<'a>,
        ctx: &'a GpuRenderContext<'a>,
    ) {
        if vertices.is_empty() {
            return;
        }

        // Reuse persistent vertex buffer; grow geometrically when needed
        let vertex_data: &[u8] = bytemuck::cast_slice(vertices);
        let required_size = vertex_data.len() as u64;

        if self.gpu_vertex_buffer_capacity < required_size {
            // Grow to at least double the current capacity, or the required size
            let new_capacity = required_size.max(self.gpu_vertex_buffer_capacity * 2);
            self.gpu_vertex_buffer = Some(ctx.device.create_buffer(&wgpu::BufferDescriptor {
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
        ctx.queue.write_buffer(vertex_buffer, 0, vertex_data);

        render_pass.set_bind_group(0, ctx.uniform_bind_group, &[]);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..required_size));

        // If no draw batches recorded, fall back to single-batch rendering
        if draw_batches.is_empty() {
            let bind_group = ctx.texture_manager.bind_group(None, self.shader_type);
            if let Some(render_pipeline) = ctx.pipeline.pipeline(self.shader_type, self.blend_mode)
            {
                render_pass.set_pipeline(render_pipeline);
                render_pass.set_bind_group(1, bind_group, &[]);
                render_pass.draw(0..vertices.len() as u32, 0..1);
            }
        } else {
            // Issue one draw call per batch with the correct texture/pipeline
            for batch in draw_batches {
                if batch.vertex_count == 0 {
                    continue;
                }
                let bind_group = ctx
                    .texture_manager
                    .bind_group(batch.texture_key.as_ref(), batch.shader_type);
                if let Some(render_pipeline) =
                    ctx.pipeline.pipeline(batch.shader_type, batch.blend_mode)
                {
                    render_pass.set_pipeline(render_pipeline);
                    render_pass.set_bind_group(1, bind_group, &[]);
                    let start = batch.vertex_start;
                    let end = start + batch.vertex_count;
                    render_pass.draw(start..end, 0..1);
                }
            }
        }
    }

    /// Get the projection matrix values.
    pub fn projection(&self) -> &[f32; 16] {
        &self.projection
    }

    /// Draw a full texture at (x, y) with size (w, h).
    pub fn draw_texture(&mut self, texture: &Texture, x: f32, y: f32, w: f32, h: f32) {
        // Auto-flush when approaching capacity, matching Java LibGDX's
        // `if (idx >= vertices.length) flush()` before adding new vertices.
        if self.vertices.len() + 6 > MAX_VERTICES {
            self.auto_flush();
        }
        self.record_texture(texture);
        self.push_quad(
            x,
            y,
            w,
            h,
            UVRect {
                u1: 0.0,
                v1: 0.0,
                u2: 1.0,
                v2: 1.0,
            },
        );
    }

    /// Draw a texture region at (x, y) with size (w, h).
    pub fn draw_region(&mut self, region: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
        // Auto-flush when approaching capacity, matching Java LibGDX's
        // `if (idx >= vertices.length) flush()` before adding new vertices.
        if self.vertices.len() + 6 > MAX_VERTICES {
            self.auto_flush();
        }
        if let Some(tex) = &region.texture {
            self.record_texture(tex);
        } else {
            self.ensure_batch(None);
        }
        self.push_quad(
            x,
            y,
            w,
            h,
            UVRect {
                u1: region.u,
                v1: region.v,
                u2: region.u2,
                v2: region.v2,
            },
        );
    }

    /// Draw a texture region with rotation and scale.
    pub fn draw_region_rotated(&mut self, region: &TextureRegion, transform: &SpriteTransform) {
        // Auto-flush when approaching capacity, matching Java LibGDX's
        // `if (idx >= vertices.length) flush()` before adding new vertices.
        if self.vertices.len() + 6 > MAX_VERTICES {
            self.auto_flush();
        }
        if let Some(tex) = &region.texture {
            self.record_texture(tex);
        } else {
            self.ensure_batch(None);
        }

        // Capture the quad if capture is enabled (use transform position/size)
        if let Some(ref mut buf) = self.capture_buffer {
            let texture_key = self
                .draw_batches
                .last()
                .and_then(|b| b.texture_key.as_ref().map(|k| k.to_string()));
            buf.push(CapturedDrawQuad {
                x: transform.x,
                y: transform.y,
                w: transform.width * transform.scale_x,
                h: transform.height * transform.scale_y,
                color: self.current_color,
                texture_key,
                blend_mode: self.blend_mode,
            });
        }

        let cos = transform.angle.to_radians().cos();
        let sin = transform.angle.to_radians().sin();

        // Compute corner offsets from origin, apply scale
        let corners: [(f32, f32); 4] = [
            (
                -transform.center_x * transform.scale_x,
                -transform.center_y * transform.scale_y,
            ),
            (
                (transform.width - transform.center_x) * transform.scale_x,
                -transform.center_y * transform.scale_y,
            ),
            (
                (transform.width - transform.center_x) * transform.scale_x,
                (transform.height - transform.center_y) * transform.scale_y,
            ),
            (
                -transform.center_x * transform.scale_x,
                (transform.height - transform.center_y) * transform.scale_y,
            ),
        ];

        let color = self.current_color;
        let (u1, v1, u2, v2) = (region.u, region.v, region.u2, region.v2);
        // Y-up projection: swap v1/v2 so that the texture top maps to the
        // visual top of the rotated quad, matching push_quad's V flip.
        let uvs = [(u1, v2), (u2, v2), (u2, v1), (u1, v1)];

        // Two triangles: 0-1-2, 0-2-3
        let vertex_count_before = self.vertices.len();
        for &idx in &[0, 1, 2, 0, 2, 3] {
            let (ox, oy) = corners[idx];
            let px = transform.x + transform.center_x + ox * cos - oy * sin;
            let py = transform.y + transform.center_y + ox * sin + oy * cos;
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
    /// Includes textures from both auto-flushed segments and the active buffer.
    pub fn drain_pending_textures(&mut self) -> HashMap<Arc<str>, PendingTexture> {
        let mut all_pending = HashMap::new();
        for segment in &mut self.flushed_segments {
            all_pending.extend(std::mem::take(&mut segment.pending_textures));
        }
        all_pending.extend(std::mem::take(&mut self.pending_textures));
        all_pending
    }

    /// Record a texture for the current draw call and manage batch boundaries.
    fn record_texture(&mut self, texture: &Texture) {
        // For pixmap-backed textures (no path), generate a synthetic key from the
        // Arc pointer address so they get registered for GPU upload.
        let key = texture.path.clone().or_else(|| {
            texture
                .rgba_data
                .as_ref()
                .map(|data| Arc::from(format!("__pixmap_{:x}", Arc::as_ptr(data) as usize)))
        });

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
    fn push_quad(&mut self, x: f32, y: f32, w: f32, h: f32, uv: UVRect) {
        // Capture the quad if capture is enabled (zero overhead when None)
        if let Some(ref mut buf) = self.capture_buffer {
            let texture_key = self
                .draw_batches
                .last()
                .and_then(|b| b.texture_key.as_ref().map(|k| k.to_string()));
            buf.push(CapturedDrawQuad {
                x,
                y,
                w,
                h,
                color: self.current_color,
                texture_key,
                blend_mode: self.blend_mode,
            });
        }

        let color = self.current_color;
        // Y-up projection: (x, y) is bottom-left, (x+w, y+h) is top-right.
        // wgpu textures have UV (0,0) at top-left, so swap v1/v2 so that
        // the texture top (v1) maps to the visual top of the quad (y+h).
        // This matches Java LibGDX SpriteBatch which also swaps V for Y-up.
        let verts = [
            SpriteVertex {
                position: [x, y],
                tex_coord: [uv.u1, uv.v2],
                color,
            },
            SpriteVertex {
                position: [x + w, y],
                tex_coord: [uv.u2, uv.v2],
                color,
            },
            SpriteVertex {
                position: [x + w, y + h],
                tex_coord: [uv.u2, uv.v1],
                color,
            },
            SpriteVertex {
                position: [x, y],
                tex_coord: [uv.u1, uv.v2],
                color,
            },
            SpriteVertex {
                position: [x + w, y + h],
                tex_coord: [uv.u2, uv.v1],
                color,
            },
            SpriteVertex {
                position: [x, y + h],
                tex_coord: [uv.u1, uv.v1],
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
        // Check UV coords (v1/v2 swapped for Y-up projection)
        assert_eq!(verts[0].tex_coord, [0.25, 0.75]);
        assert_eq!(verts[2].tex_coord, [0.75, 0.25]);
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
            &region,
            &SpriteTransform {
                x: 100.0,
                y: 100.0,
                center_x: 32.0,
                center_y: 32.0,
                width: 64.0,
                height: 64.0,
                scale_x: 1.0,
                scale_y: 1.0,
                angle: 45.0,
            },
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

    /// Verify that flush() clears pending_textures along with vertices and
    /// draw_batches. flush_to_gpu() must maintain the same invariant so that
    /// stale pending textures never survive a GPU submission cycle.
    #[test]
    fn test_flush_clears_pending_textures() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("flush_pending_test")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        batch.draw_texture(&tex, 0.0, 0.0, 4.0, 4.0);

        assert!(!batch.vertices().is_empty());
        // pending_textures is private but drain reveals its state
        // Flush should clear everything including pending textures
        batch.flush();
        assert!(batch.vertices().is_empty());
        let drained = batch.drain_pending_textures();
        assert!(
            drained.is_empty(),
            "flush() must clear pending_textures so drain returns empty"
        );

        batch.end();
    }

    #[test]
    fn test_capture_disabled_by_default() {
        let batch = SpriteBatch::new();
        assert!(batch.captured_quads().is_empty());
    }

    #[test]
    fn test_capture_records_quads() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let tex = Texture {
            width: 10,
            height: 10,
            disposed: false,
            path: Some(Arc::from("capture_tex")),
            rgba_data: Some(Arc::new(vec![255u8; 400])),
            ..Default::default()
        };
        batch.draw_texture(&tex, 5.0, 10.0, 20.0, 30.0);

        let quads = batch.captured_quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].x, 5.0);
        assert_eq!(quads[0].y, 10.0);
        assert_eq!(quads[0].w, 20.0);
        assert_eq!(quads[0].h, 30.0);
        assert_eq!(quads[0].texture_key.as_deref(), Some("capture_tex"));
        assert_eq!(quads[0].blend_mode, BlendMode::Normal);
        assert_eq!(quads[0].color, [1.0, 1.0, 1.0, 1.0]);
    }

    #[test]
    fn test_capture_records_color_and_blend() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();
        batch.set_color(&Color::new(1.0, 0.0, 0.5, 0.8));
        batch.set_blend_function(0x0302, 1); // Additive

        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);

        let quads = batch.captured_quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].color, [1.0, 0.0, 0.5, 0.8]);
        assert_eq!(quads[0].blend_mode, BlendMode::Additive);
    }

    #[test]
    fn test_capture_no_recording_when_disabled() {
        let mut batch = SpriteBatch::new();
        // capture not enabled
        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        assert!(batch.captured_quads().is_empty());
    }

    #[test]
    fn test_capture_clear() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(batch.captured_quads().len(), 1);

        batch.clear_captured();
        assert!(batch.captured_quads().is_empty());
    }

    #[test]
    fn test_capture_disable_drops_buffer() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let tex = Texture::default();
        batch.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(batch.captured_quads().len(), 1);

        batch.disable_capture();
        assert!(batch.captured_quads().is_empty());
    }

    #[test]
    fn test_capture_draw_region() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let tex = Texture {
            width: 16,
            height: 16,
            disposed: false,
            path: Some(Arc::from("region_tex")),
            rgba_data: Some(Arc::new(vec![0u8; 1024])),
            ..Default::default()
        };
        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 0.5,
            v2: 0.5,
            region_x: 0,
            region_y: 0,
            region_width: 8,
            region_height: 8,
            texture: Some(tex),
        };
        batch.draw_region(&region, 100.0, 200.0, 50.0, 60.0);

        let quads = batch.captured_quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].x, 100.0);
        assert_eq!(quads[0].y, 200.0);
        assert_eq!(quads[0].w, 50.0);
        assert_eq!(quads[0].h, 60.0);
        assert_eq!(quads[0].texture_key.as_deref(), Some("region_tex"));
    }

    #[test]
    fn test_capture_rotated_draw() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..Default::default()
        };
        batch.draw_region_rotated(
            &region,
            &SpriteTransform {
                x: 50.0,
                y: 60.0,
                center_x: 16.0,
                center_y: 16.0,
                width: 32.0,
                height: 32.0,
                scale_x: 2.0,
                scale_y: 1.5,
                angle: 45.0,
            },
        );

        let quads = batch.captured_quads();
        assert_eq!(quads.len(), 1);
        assert_eq!(quads[0].x, 50.0);
        assert_eq!(quads[0].y, 60.0);
        assert_eq!(quads[0].w, 64.0); // 32.0 * 2.0
        assert_eq!(quads[0].h, 48.0); // 32.0 * 1.5
    }

    #[test]
    fn test_capture_multiple_quads() {
        let mut batch = SpriteBatch::new();
        batch.enable_capture();

        let tex_a = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("tex_a")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        let tex_b = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("tex_b")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        batch.draw_texture(&tex_a, 0.0, 0.0, 10.0, 10.0);
        batch.draw_texture(&tex_b, 20.0, 20.0, 10.0, 10.0);

        let quads = batch.captured_quads();
        assert_eq!(quads.len(), 2);
        assert_eq!(quads[0].texture_key.as_deref(), Some("tex_a"));
        assert_eq!(quads[1].texture_key.as_deref(), Some("tex_b"));
        assert_eq!(quads[0].x, 0.0);
        assert_eq!(quads[1].x, 20.0);
    }

    /// Verify that the active vertex buffer never exceeds MAX_VERTICES.
    /// Before this fix, the Vec would grow unboundedly past MAX_VERTICES.
    #[test]
    fn test_auto_flush_caps_active_vertex_buffer() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture::default();
        // Draw more quads than MAX_SPRITES (1000) to trigger auto-flush
        let total_quads = MAX_SPRITES + 100;
        for i in 0..total_quads {
            batch.draw_texture(&tex, i as f32, 0.0, 10.0, 10.0);
            // The active buffer must never exceed MAX_VERTICES
            assert!(
                batch.vertices.len() <= MAX_VERTICES,
                "active vertex buffer grew to {} (MAX_VERTICES={}), quad #{}",
                batch.vertices.len(),
                MAX_VERTICES,
                i + 1,
            );
        }

        // Total vertex count across all segments should equal total_quads * 6
        assert_eq!(
            batch.vertex_count(),
            total_quads * 6,
            "total vertex count must include all auto-flushed segments"
        );

        // At least one auto-flush should have occurred
        assert!(
            !batch.flushed_segments.is_empty(),
            "auto-flush should have created at least one completed segment"
        );

        batch.end();
    }

    /// Verify that auto-flush preserves all draw data: the total vertex
    /// count must match what would have accumulated without the cap.
    #[test]
    fn test_auto_flush_preserves_all_vertices() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture::default();
        let total_quads = MAX_SPRITES * 3 + 50; // triggers multiple auto-flushes
        for i in 0..total_quads {
            batch.draw_texture(&tex, i as f32, 0.0, 1.0, 1.0);
        }

        assert_eq!(
            batch.vertex_count(),
            total_quads * 6,
            "no vertices should be lost during auto-flush"
        );

        batch.end();
    }

    /// Verify that has_pending_draw_data() returns true when data exists
    /// only in flushed segments (active buffer empty after exact-capacity flush).
    #[test]
    fn test_has_pending_draw_data_after_auto_flush() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture::default();
        // Fill exactly to capacity, then one more to trigger auto-flush
        for _ in 0..MAX_SPRITES {
            batch.draw_texture(&tex, 0.0, 0.0, 1.0, 1.0);
        }
        // At this point vertices.len() == MAX_VERTICES, next draw triggers auto-flush
        batch.draw_texture(&tex, 0.0, 0.0, 1.0, 1.0);

        // The active buffer has just the last quad (6 vertices)
        assert_eq!(batch.vertices.len(), 6);
        // But has_pending_draw_data should be true (data in segments + active)
        assert!(batch.has_pending_draw_data());

        batch.end();
    }

    /// Verify that flush() clears both flushed segments and active buffer.
    #[test]
    fn test_flush_clears_flushed_segments() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let tex = Texture::default();
        for _ in 0..(MAX_SPRITES + 10) {
            batch.draw_texture(&tex, 0.0, 0.0, 1.0, 1.0);
        }
        assert!(!batch.flushed_segments.is_empty());

        batch.flush();
        assert!(batch.vertices.is_empty());
        assert!(batch.flushed_segments.is_empty());
        assert_eq!(batch.vertex_count(), 0);
        assert!(!batch.has_pending_draw_data());

        batch.end();
    }

    /// Verify that drain_pending_textures collects textures from all segments.
    #[test]
    fn test_drain_pending_textures_includes_flushed_segments() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        // Create texture that will be in the first segment
        let tex_a = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("seg_tex_a")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        // Fill first segment to capacity with tex_a
        for _ in 0..MAX_SPRITES {
            batch.draw_texture(&tex_a, 0.0, 0.0, 1.0, 1.0);
        }

        // Trigger auto-flush and add a different texture in the new segment
        let tex_b = Texture {
            width: 4,
            height: 4,
            disposed: false,
            path: Some(Arc::from("seg_tex_b")),
            rgba_data: Some(Arc::new(vec![0u8; 64])),
            ..Default::default()
        };
        batch.draw_texture(&tex_b, 0.0, 0.0, 1.0, 1.0);

        let pending = batch.drain_pending_textures();
        assert!(
            pending.contains_key(&Arc::from("seg_tex_a") as &Arc<str>),
            "texture from flushed segment must be included"
        );
        assert!(
            pending.contains_key(&Arc::from("seg_tex_b") as &Arc<str>),
            "texture from active buffer must be included"
        );

        batch.end();
    }

    /// Verify that draw_region_rotated also triggers auto-flush.
    #[test]
    fn test_auto_flush_draw_region_rotated() {
        let mut batch = SpriteBatch::new();
        batch.begin();

        let region = TextureRegion {
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..Default::default()
        };
        let transform = SpriteTransform {
            x: 0.0,
            y: 0.0,
            center_x: 5.0,
            center_y: 5.0,
            width: 10.0,
            height: 10.0,
            scale_x: 1.0,
            scale_y: 1.0,
            angle: 45.0,
        };

        for _ in 0..(MAX_SPRITES + 10) {
            batch.draw_region_rotated(&region, &transform);
            assert!(
                batch.vertices.len() <= MAX_VERTICES,
                "active buffer exceeded MAX_VERTICES during draw_region_rotated"
            );
        }

        assert_eq!(batch.vertex_count(), (MAX_SPRITES + 10) * 6);

        batch.end();
    }
}
