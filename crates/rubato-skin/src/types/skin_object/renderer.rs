use crate::stubs::{BitmapFont, Color, GlyphLayout, SpriteBatch, Texture, TextureRegion};

/// Corresponds to Skin.SkinObjectRenderer in Java.
///
/// Manages shader switching, blend state, and color for sprite draw calls.
/// Java: holds SpriteBatch + ShaderProgram[] + blend/type/color state.
pub struct SkinObjectRenderer {
    pub color: Color,
    pub blend: i32,
    pub obj_type: i32,
    /// Current active shader type (tracks which shader is set on the sprite batch)
    pub(super) current_shader: i32,
    /// Saved color before pre_draw, restored in post_draw
    orgcolor: Option<Color>,
    pub sprite: SpriteBatch,
}

impl Default for SkinObjectRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinObjectRenderer {
    pub const TYPE_NORMAL: i32 = 0;
    pub const TYPE_LINEAR: i32 = 1;
    pub const TYPE_BILINEAR: i32 = 2;
    pub const TYPE_FFMPEG: i32 = 3;
    pub const TYPE_LAYER: i32 = 4;
    pub const TYPE_DISTANCE_FIELD: i32 = 5;

    // GL blend constants (matching Java)
    const GL_SRC_ALPHA: i32 = 0x0302;
    const GL_ONE: i32 = 1;
    const GL_ONE_MINUS_SRC_ALPHA: i32 = 0x0303;
    const GL_ZERO: i32 = 0;
    const GL_SRC_COLOR: i32 = 0x0300;
    const GL_ONE_MINUS_DST_COLOR: i32 = 0x0307;

    pub fn new() -> Self {
        let mut sprite = SpriteBatch::new();
        // Java: sprite.setShader(shaders[current]); sprite.setColor(Color.WHITE);
        sprite.shader_type = Self::TYPE_NORMAL;
        sprite.set_color(&Color::new(1.0, 1.0, 1.0, 1.0));
        Self {
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            blend: 0,
            obj_type: 0,
            current_shader: Self::TYPE_NORMAL,
            orgcolor: None,
            sprite,
        }
    }

    pub fn set_color(&mut self, color: &Color) {
        self.color.set(color);
    }

    pub fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.color.set_rgba(r, g, b, a);
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn blend(&self) -> i32 {
        self.blend
    }

    pub fn toast_type(&self) -> i32 {
        self.obj_type
    }

    /// Set texture filter based on current type.
    /// Java: sets Linear filter for TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD.
    /// In wgpu, filtering is handled by sampler selection in the render pipeline.
    fn set_filter(&self, _image: &TextureRegion) {
        // In wgpu, filtering is configured on samplers via SpriteRenderPipeline::get_sampler().
        // The sampler is selected based on shader_type when creating texture bind groups.
    }

    /// Pre-draw setup: shader switching, blend mode, color.
    /// Java: Skin.java lines 496-537
    fn pre_draw(&mut self) {
        // Java: if(shaders[current] != shaders[type]) { sprite.setShader(shaders[type]); current = type; }
        if self.current_shader != self.obj_type {
            self.sprite.shader_type = self.obj_type;
            self.current_shader = self.obj_type;
        }

        // Java: switch(blend) — set blend function
        match self.blend {
            2 => {
                // Additive: SRC_ALPHA, ONE
                self.sprite
                    .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE);
            }
            3 => {
                // Subtractive: SRC_ALPHA, ONE (with GL_FUNC_SUBTRACT equation)
                // In wgpu, this is handled by the BlendMode::Subtractive pipeline
                self.sprite
                    .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE);
            }
            4 => {
                // Multiply: ZERO, SRC_COLOR
                self.sprite
                    .set_blend_function(Self::GL_ZERO, Self::GL_SRC_COLOR);
            }
            9 => {
                // Inversion: ONE_MINUS_DST_COLOR, ZERO
                self.sprite
                    .set_blend_function(Self::GL_ONE_MINUS_DST_COLOR, Self::GL_ZERO);
            }
            _ => {}
        }

        // Java: orgcolor = sprite.getColor(); sprite.setColor(color);
        self.orgcolor = Some(self.sprite.color());
        self.sprite.set_color(&self.color);
    }

    /// Post-draw cleanup: restore color and blend mode.
    /// Java: Skin.java lines 539-547
    fn post_draw(&mut self) {
        // Java: if(orgcolor != null) { sprite.setColor(orgcolor); }
        if let Some(ref orgcolor) = self.orgcolor.take() {
            self.sprite.set_color(orgcolor);
        }

        // Java: if (blend >= 2) { sprite.setBlendFunction(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA); }
        if self.blend >= 2 {
            self.sprite
                .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE_MINUS_SRC_ALPHA);
        }
    }

    /// Java: sprite.draw(image, x + 0.01f, y + 0.01f, w, h)
    /// The 0.01 offset is a workaround for a Windows TextureRegion rendering issue.
    pub fn draw(&mut self, image: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
        self.set_filter(image);
        self.pre_draw();
        self.sprite.draw_region(image, x + 0.01, y + 0.01, w, h);
        self.post_draw();
    }

    /// Java: sprite.draw(image, x + 0.01f, y + 0.01f, cx * w, cy * h, w, h, 1, 1, angle)
    #[allow(clippy::too_many_arguments)]
    pub fn draw_rotated(
        &mut self,
        image: &TextureRegion,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        cx: f32,
        cy: f32,
        angle: i32,
    ) {
        self.set_filter(image);
        self.pre_draw();
        self.sprite.draw_region_rotated(
            image,
            &rubato_render::sprite_batch::SpriteTransform {
                x: x + 0.01,
                y: y + 0.01,
                center_x: cx * w,
                center_y: cy * h,
                width: w,
                height: h,
                scale_x: 1.0,
                scale_y: 1.0,
                angle: angle as f32,
            },
        );
        self.post_draw();
    }

    /// Draw a full Texture at (x, y) with size (w, h).
    /// Java: SkinObjectRenderer.draw(Texture image, float x, float y, float w, float h)
    pub fn draw_texture(&mut self, image: &Texture, x: f32, y: f32, w: f32, h: f32) {
        // Java: setFilter(image)
        // In wgpu, filtering is configured on samplers via SpriteRenderPipeline::get_sampler().
        self.pre_draw();
        self.sprite.draw_texture(image, x, y, w, h);
        self.post_draw();
    }

    /// Draw text using a BitmapFont with color.
    /// Java: SkinObjectRenderer.draw(BitmapFont font, String s, float x, float y, Color c)
    ///
    /// Sets the font color, then delegates to font.draw(sprite, text, x, y) which
    /// rasterizes glyphs and submits quads to the SpriteBatch.
    pub fn draw_font(&mut self, font: &mut BitmapFont, text: &str, x: f32, y: f32, color: &Color) {
        // Java: for (TextureRegion region : font.getRegions()) { setFilter(region); }
        // In wgpu, filtering is handled by sampler selection based on shader_type.
        self.pre_draw();
        font.set_color(color);
        font.draw(&mut self.sprite, text, x, y);
        self.post_draw();
    }

    /// Draw pre-laid-out text using a BitmapFont and GlyphLayout.
    /// Java: SkinObjectRenderer.draw(BitmapFont font, GlyphLayout layout, float x, float y)
    pub fn draw_font_layout(&mut self, font: &BitmapFont, layout: &GlyphLayout, x: f32, y: f32) {
        self.pre_draw();
        font.draw_layout(&mut self.sprite, layout, x, y);
        self.post_draw();
    }
}
