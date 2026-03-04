// wgpu render pipeline for sprite rendering.
// Corresponds to the combination of LibGDX SpriteBatch + ShaderProgram + blend state.

use crate::blend::BlendMode;
use crate::shader::{BILINEAR_SHADER_WGSL, SPRITE_SHADER_WGSL};
use crate::sprite_batch::SpriteVertex;

/// Shader type IDs matching Java SkinObjectRenderer constants.
/// TYPE_NORMAL = 0, TYPE_LINEAR = 1, TYPE_BILINEAR = 2,
/// TYPE_FFMPEG = 3, TYPE_LAYER = 4, TYPE_DISTANCE_FIELD = 5
pub const SHADER_TYPE_NORMAL: i32 = 0;
pub const SHADER_TYPE_LINEAR: i32 = 1;
pub const SHADER_TYPE_BILINEAR: i32 = 2;
pub const SHADER_TYPE_FFMPEG: i32 = 3;
pub const SHADER_TYPE_LAYER: i32 = 4;
pub const SHADER_TYPE_DISTANCE_FIELD: i32 = 5;

/// Manages wgpu render pipelines for sprite rendering.
/// Each combination of (shader_type, blend_mode) maps to a separate wgpu::RenderPipeline.
pub struct SpriteRenderPipeline {
    /// Bind group layout for uniform buffer (projection matrix)
    pub uniform_layout: wgpu::BindGroupLayout,
    /// Bind group layout for texture + sampler
    pub texture_layout: wgpu::BindGroupLayout,
    /// Pipelines indexed by (shader_type, blend_mode)
    pipelines: Vec<PipelineEntry>,
    /// Nearest-neighbor sampler (TYPE_NORMAL)
    pub sampler_nearest: wgpu::Sampler,
    /// Linear sampler (TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD)
    pub sampler_linear: wgpu::Sampler,
}

struct PipelineEntry {
    shader_type: i32,
    blend_mode: BlendMode,
    pipeline: wgpu::RenderPipeline,
}

impl SpriteRenderPipeline {
    /// Create render pipelines for all shader/blend combinations.
    pub fn new(device: &wgpu::Device, output_format: wgpu::TextureFormat) -> Self {
        // Bind group layout 0: uniform buffer (projection matrix)
        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite uniform layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        // Bind group layout 1: texture + sampler
        let texture_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("sprite texture layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        multisampled: false,
                        view_dimension: wgpu::TextureViewDimension::D2,
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("sprite pipeline layout"),
            bind_group_layouts: &[&uniform_layout, &texture_layout],
            push_constant_ranges: &[],
        });

        // Create shader modules
        let main_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("sprite shader"),
            source: wgpu::ShaderSource::Wgsl(SPRITE_SHADER_WGSL.into()),
        });
        let bilinear_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("bilinear shader"),
            source: wgpu::ShaderSource::Wgsl(BILINEAR_SHADER_WGSL.into()),
        });

        let blend_modes = [
            BlendMode::Normal,
            BlendMode::Additive,
            BlendMode::Subtractive,
            BlendMode::Multiply,
            BlendMode::Inversion,
        ];

        // (shader_module, fragment_entry_point, shader_type)
        let shader_configs: Vec<(&wgpu::ShaderModule, &str, i32)> = vec![
            (&main_shader, "fs_main", SHADER_TYPE_NORMAL),
            (&main_shader, "fs_main", SHADER_TYPE_LINEAR),
            (&bilinear_shader, "fs_bilinear", SHADER_TYPE_BILINEAR),
            (&main_shader, "fs_ffmpeg", SHADER_TYPE_FFMPEG),
            (&main_shader, "fs_layer", SHADER_TYPE_LAYER),
            // TYPE_DISTANCE_FIELD uses fs_main for now (distance field shader is complex,
            // and the Java version requires uniforms not yet wired)
            (&main_shader, "fs_main", SHADER_TYPE_DISTANCE_FIELD),
        ];

        let vertex_buffers = [SpriteVertex::desc()];

        let mut pipelines = Vec::new();
        for &(shader_module, frag_entry, shader_type) in &shader_configs {
            for &blend_mode in &blend_modes {
                let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                    label: Some(&format!(
                        "sprite pipeline (shader={}, blend={:?})",
                        shader_type, blend_mode
                    )),
                    layout: Some(&pipeline_layout),
                    vertex: wgpu::VertexState {
                        module: shader_module,
                        entry_point: Some("vs_main"),
                        buffers: &vertex_buffers,
                        compilation_options: Default::default(),
                    },
                    fragment: Some(wgpu::FragmentState {
                        module: shader_module,
                        entry_point: Some(frag_entry),
                        targets: &[Some(wgpu::ColorTargetState {
                            format: output_format,
                            blend: Some(blend_mode.to_wgpu_blend_state()),
                            write_mask: wgpu::ColorWrites::ALL,
                        })],
                        compilation_options: Default::default(),
                    }),
                    primitive: wgpu::PrimitiveState {
                        topology: wgpu::PrimitiveTopology::TriangleList,
                        strip_index_format: None,
                        front_face: wgpu::FrontFace::Ccw,
                        cull_mode: None,
                        polygon_mode: wgpu::PolygonMode::Fill,
                        unclipped_depth: false,
                        conservative: false,
                    },
                    depth_stencil: None,
                    multisample: wgpu::MultisampleState::default(),
                    multiview: None,
                    cache: None,
                });
                pipelines.push(PipelineEntry {
                    shader_type,
                    blend_mode,
                    pipeline,
                });
            }
        }

        // Samplers matching Java: Nearest for TYPE_NORMAL, Linear for others
        let sampler_nearest = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite sampler nearest"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Nearest,
            mipmap_filter: wgpu::FilterMode::Nearest,
            ..Default::default()
        });

        let sampler_linear = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("sprite sampler linear"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        Self {
            uniform_layout,
            texture_layout,
            pipelines,
            sampler_nearest,
            sampler_linear,
        }
    }

    /// Get the render pipeline for the given shader type and blend mode.
    pub fn get_pipeline(
        &self,
        shader_type: i32,
        blend_mode: BlendMode,
    ) -> Option<&wgpu::RenderPipeline> {
        self.pipelines
            .iter()
            .find(|e| e.shader_type == shader_type && e.blend_mode == blend_mode)
            .map(|e| &e.pipeline)
    }

    /// Get the appropriate sampler for the given shader type.
    /// Java: TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD use Linear filter.
    pub fn get_sampler(&self, shader_type: i32) -> &wgpu::Sampler {
        match shader_type {
            SHADER_TYPE_LINEAR | SHADER_TYPE_FFMPEG | SHADER_TYPE_DISTANCE_FIELD => {
                &self.sampler_linear
            }
            _ => &self.sampler_nearest,
        }
    }

    /// Returns true if the given shader type uses a linear sampler.
    /// Java: TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD use Linear filter.
    pub fn is_linear_sampler(shader_type: i32) -> bool {
        matches!(
            shader_type,
            SHADER_TYPE_LINEAR | SHADER_TYPE_FFMPEG | SHADER_TYPE_DISTANCE_FIELD
        )
    }

    /// Get the total number of pipelines created.
    pub fn pipeline_count(&self) -> usize {
        self.pipelines.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_type_constants_match_java() {
        // Java: SkinObjectRenderer constants
        assert_eq!(SHADER_TYPE_NORMAL, 0);
        assert_eq!(SHADER_TYPE_LINEAR, 1);
        assert_eq!(SHADER_TYPE_BILINEAR, 2);
        assert_eq!(SHADER_TYPE_FFMPEG, 3);
        assert_eq!(SHADER_TYPE_LAYER, 4);
        assert_eq!(SHADER_TYPE_DISTANCE_FIELD, 5);
    }

    #[tokio::test]
    async fn test_render_pipeline_creation() {
        let ctx = match crate::gpu_context::GpuContext::new_headless().await {
            Ok(ctx) => ctx,
            Err(_) => {
                // Skip test if no GPU adapter available (CI environment)
                return;
            }
        };
        let format = ctx.surface_format();
        let pipeline = SpriteRenderPipeline::new(&ctx.device, format);

        // 6 shader types * 5 blend modes = 30 pipelines
        assert_eq!(pipeline.pipeline_count(), 30);

        // Verify we can look up pipelines for all combinations
        let blend_modes = [
            BlendMode::Normal,
            BlendMode::Additive,
            BlendMode::Subtractive,
            BlendMode::Multiply,
            BlendMode::Inversion,
        ];
        for shader_type in 0..=5 {
            for &blend_mode in &blend_modes {
                assert!(
                    pipeline.get_pipeline(shader_type, blend_mode).is_some(),
                    "Missing pipeline for shader_type={}, blend_mode={:?}",
                    shader_type,
                    blend_mode
                );
            }
        }
    }

    #[tokio::test]
    async fn test_sampler_selection() {
        let ctx = match crate::gpu_context::GpuContext::new_headless().await {
            Ok(ctx) => ctx,
            Err(_) => return,
        };
        let format = ctx.surface_format();
        let pipeline = SpriteRenderPipeline::new(&ctx.device, format);

        // TYPE_NORMAL -> nearest
        let nearest = pipeline.get_sampler(SHADER_TYPE_NORMAL) as *const _;
        let linear = pipeline.get_sampler(SHADER_TYPE_LINEAR) as *const _;

        // TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD -> linear
        assert_eq!(pipeline.get_sampler(SHADER_TYPE_LINEAR) as *const _, linear);
        assert_eq!(pipeline.get_sampler(SHADER_TYPE_FFMPEG) as *const _, linear);
        assert_eq!(
            pipeline.get_sampler(SHADER_TYPE_DISTANCE_FIELD) as *const _,
            linear
        );

        // TYPE_NORMAL, TYPE_BILINEAR, TYPE_LAYER -> nearest
        assert_eq!(
            pipeline.get_sampler(SHADER_TYPE_NORMAL) as *const _,
            nearest
        );
        assert_eq!(
            pipeline.get_sampler(SHADER_TYPE_BILINEAR) as *const _,
            nearest
        );
        assert_eq!(pipeline.get_sampler(SHADER_TYPE_LAYER) as *const _, nearest);
    }
}
