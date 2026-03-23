// Shader pipeline wrapper.
// Drop-in replacement for the ShaderProgram stub in render_reexports.rs.

/// Wrapper around a wgpu render pipeline.
/// Corresponds to com.badlogic.gdx.graphics.glutils.ShaderProgram.
#[derive(Clone, Debug, Default)]
pub struct ShaderProgram;

impl ShaderProgram {
    pub fn new() -> Self {
        Self
    }
}

/// WGSL sprite shader source.
/// Translates the Java/LibGDX SpriteBatch default shader + SpriteBatchHelper.
///
/// Java vertex shader:
///   v_color = a_color; v_color.a *= 255.0/254.0;
///   v_texCoords = a_texCoord0;
///   gl_Position = u_projTrans * a_position;
///
/// Java fragment shader:
///   fragColor = v_color * texture(u_texture, v_texCoords);
pub const SPRITE_SHADER_WGSL: &str = r#"
// Uniform: orthographic projection matrix
struct Uniforms {
    proj_trans: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// Texture + sampler
@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.proj_trans * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    // Java: v_color.a = v_color.a * (255.0/254.0)
    out.color = vec4<f32>(in.color.rgb, in.color.a * (255.0 / 254.0));
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // Java: fragColor = v_color * texture(u_texture, v_texCoords)
    let tex_color = textureSample(t_diffuse, s_diffuse, in.tex_coord);
    return in.color * tex_color;
}

// FFmpeg shader: pass through RGBA unchanged.
// FFmpegProcessor already converts frames to RGBA, so no swizzle is needed.
@fragment
fn fs_ffmpeg(in: VertexOutput) -> @location(0) vec4<f32> {
    let c4 = textureSample(t_diffuse, s_diffuse, in.tex_coord);
    return in.color * c4;
}

// Layer shader: near-black pixels become transparent
// Java: if(r==0 && g==0 && b==0) { alpha=0 } else { normal }
// Threshold 0.004 (~1/255) tolerates linear-filtering interpolation artifacts
@fragment
fn fs_layer(in: VertexOutput) -> @location(0) vec4<f32> {
    let c4 = textureSample(t_diffuse, s_diffuse, in.tex_coord);
    if (c4.r < 0.004 && c4.g < 0.004 && c4.b < 0.004) {
        return in.color * vec4<f32>(c4.r, c4.g, c4.b, 0.0);
    } else {
        return in.color * c4;
    }
}

// Distance field shader: SDF text rendering
// Java: SkinObjectRenderer TYPE_DISTANCE_FIELD
@fragment
fn fs_distance_field(in: VertexOutput) -> @location(0) vec4<f32> {
    let distance = textureSample(t_diffuse, s_diffuse, in.tex_coord).a;
    let smoothing = fwidth(distance) * 0.5;
    let alpha = smoothstep(0.5 - smoothing, 0.5 + smoothing, distance);
    return vec4<f32>(in.color.rgb, in.color.a * alpha);
}
"#;

/// Bilinear filter WGSL shader.
/// Java bilinear.frag: custom bilinear interpolation with alpha-premultiplied blending.
pub const BILINEAR_SHADER_WGSL: &str = r#"
struct Uniforms {
    proj_trans: mat4x4<f32>,
};
@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(1) @binding(0)
var t_diffuse: texture_2d<f32>;
@group(1) @binding(1)
var s_diffuse: sampler;

struct VertexInput {
    @location(0) position: vec2<f32>,
    @location(1) tex_coord: vec2<f32>,
    @location(2) color: vec4<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) tex_coord: vec2<f32>,
    @location(1) color: vec4<f32>,
};

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.proj_trans * vec4<f32>(in.position, 0.0, 1.0);
    out.tex_coord = in.tex_coord;
    out.color = vec4<f32>(in.color.rgb, in.color.a * (255.0 / 254.0));
    return out;
}

@fragment
fn fs_bilinear(in: VertexOutput) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(t_diffuse));
    let center_a = textureSample(t_diffuse, s_diffuse, in.tex_coord).a;
    if (center_a > 0.0) {
        let texel_size_x = 0.5 / tex_size.x;
        let texel_size_y = 0.5 / tex_size.y;
        let p0q0 = textureSample(t_diffuse, s_diffuse, in.tex_coord + vec2<f32>(-texel_size_x, -texel_size_y));
        let p1q0 = textureSample(t_diffuse, s_diffuse, in.tex_coord + vec2<f32>(texel_size_x, -texel_size_y));
        let p0q1 = textureSample(t_diffuse, s_diffuse, in.tex_coord + vec2<f32>(-texel_size_x, texel_size_y));
        let p1q1 = textureSample(t_diffuse, s_diffuse, in.tex_coord + vec2<f32>(texel_size_x, texel_size_y));
        let a = fract(in.tex_coord.x * tex_size.x + 0.5);
        let p_interp_q0 = mix(p0q0 * p0q0.a, p1q0 * p1q0.a, a);
        let p_interp_q1 = mix(p0q1 * p0q1.a, p1q1 * p1q1.a, a);
        let b = fract(in.tex_coord.y * tex_size.y + 0.5);
        var result = mix(p_interp_q0, p_interp_q1, b) / center_a;
        result.a = center_a;
        return in.color * result;
    } else {
        return vec4<f32>(0.0, 0.0, 0.0, 0.0);
    }
}
"#;
