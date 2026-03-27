// Re-export the real SpriteBatch from beatoraja-render.
// The stub unit struct is no longer needed since Phase 22a provided the GPU implementation.
pub use rubato_render::sprite_batch::SpriteBatch;

/// SpriteBatchHelper - creates SpriteBatch with macOS-compatible shader
///
/// Hack for macOS - see https://github.com/libgdx/libgdx/issues/6897
/// On macOS, LibGDX needs OpenGL 3.2 core profile shaders.
/// In Rust/wgpu, WGSL shaders are used instead so this is no longer needed,
/// but the factory method is preserved for API compatibility.
pub struct SpriteBatchHelper;

impl SpriteBatchHelper {
    /// Vertex shader source (GLSL 150 for macOS compatibility)
    /// Preserved for reference; wgpu uses WGSL shaders in render_pipeline.rs.
    pub const VERTEX_SHADER: &'static str = concat!(
        "#version 150\n",
        "in vec4 a_position;\n",
        "in vec4 a_color;\n",
        "in vec2 a_texCoord0;\n",
        "uniform mat4 u_projTrans;\n",
        "out vec4 v_color;\n",
        "out vec2 v_texCoords;\n",
        "\n",
        "void main()\n",
        "{\n",
        "   v_color = a_color;\n",
        "   v_color.a = v_color.a * (255.0/254.0);\n",
        "   v_texCoords = a_texCoord0;\n",
        "   gl_Position =  u_projTrans * a_position;\n",
        "}\n",
    );

    /// Fragment shader source (GLSL 150 for macOS compatibility)
    /// Preserved for reference; wgpu uses WGSL shaders in render_pipeline.rs.
    pub const FRAGMENT_SHADER: &'static str = concat!(
        "#version 150\n",
        "#ifdef GL_ES\n",
        "#define LOWP lowp\n",
        "precision mediump float;\n",
        "#else\n",
        "#define LOWP \n",
        "#endif\n",
        "in LOWP vec4 v_color;\n",
        "in vec2 v_texCoords;\n",
        "uniform sampler2D u_texture;\n",
        "out vec4 fragColor;\n",
        "void main()\n",
        "{\n",
        "  fragColor = v_color * texture(u_texture, v_texCoords);\n",
        "}",
    );

    pub fn create_sprite_batch_shader() {
        // ShaderProgramFactory.fromString(vertexShader, fragmentShader, true, true)
        // In wgpu, shaders are compiled as part of SpriteRenderPipeline::new()
        // wgpu uses WGSL shaders compiled in SpriteRenderPipeline::new(), not GLSL ShaderPrograms
    }

    pub fn create_sprite_batch() -> SpriteBatch {
        // new SpriteBatch(1000, ShaderCompatibilityHelper.mustUse32CShader() ? ... : null)
        SpriteBatch::new()
    }
}
