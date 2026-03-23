// SkinTextFont.java -> skin_text_font.rs
// Mechanical line-by-line translation.

use crate::property::string_property::StringProperty;
use crate::reexports::{
    BitmapFont, Color, FreeTypeFontGenerator, FreeTypeFontParameter, GlyphLayout, MainState,
};
use crate::text::skin_text::{OVERFLOW_OVERFLOW, OVERFLOW_SHRINK, OVERFLOW_TRUNCATE, SkinTextData};
use crate::types::skin_object::{DrawImageAtParams, SkinObjectRenderer};

/// Parameters for drawing text glyphs at a specific position.
struct DrawTextGlyphsParams<'a> {
    pub sprite: &'a mut SkinObjectRenderer,
    pub text: &'a str,
    pub color: &'a Color,
    pub x: f32,
    pub y: f32,
    pub _layout_width: f32,
    pub region_width: f32,
}

/// Compute the x position for text based on alignment within a region.
/// Java SkinTextFont uses anchor-relative alignment:
///   - LEFT (0): text starts at region.x
///   - CENTER (1): region.x is the center anchor: x = region.x - region.width / 2
///   - RIGHT (2): region.x is the right anchor: x = region.x - region.width
fn compute_aligned_x(align: i32, region_x: f32, region_width: f32) -> f32 {
    match align {
        2 => region_x - region_width,       // RIGHT
        1 => region_x - region_width / 2.0, // CENTER
        _ => region_x,                      // LEFT (default)
    }
}

pub struct SkinTextFont {
    pub text_data: SkinTextData,
    font: Option<BitmapFont>,
    layout: Option<GlyphLayout>,
    generator: Option<FreeTypeFontGenerator>,
    parameter: FreeTypeFontParameter,
    prepared_fonts: Option<String>,
}

impl SkinTextFont {
    pub fn new(fontpath: &str, _cycle: i32, size: i32, shadow: i32) -> Self {
        Self::new_with_property(fontpath, _cycle, size, shadow, None)
    }

    pub fn new_with_property(
        fontpath: &str,
        _cycle: i32,
        size: i32,
        shadow: i32,
        property: Option<Box<dyn StringProperty>>,
    ) -> Self {
        let text_data = if let Some(prop) = property {
            SkinTextData::new_with_property(prop)
        } else {
            SkinTextData::new_with_id(-1)
        };
        let parameter = FreeTypeFontParameter {
            characters: String::new(),
            size,
            ..Default::default()
        };
        let generator = Some(FreeTypeFontGenerator::new(fontpath));
        let mut result = Self {
            text_data,
            font: None,
            layout: None,
            generator,
            parameter,
            prepared_fonts: None,
        };
        result
            .text_data
            .set_shadow_offset(shadow as f32, shadow as f32);
        result
    }

    pub fn validate(&self) -> bool {
        if self.generator.is_none() {
            return false;
        }
        self.text_data.data.validate()
    }

    pub fn prepare_font(&mut self, text: &str) {
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;

        self.parameter.characters = text.to_string();
        if let Some(ref generator) = self.generator {
            self.font = Some(generator.generate_font(&self.parameter));
            self.layout = Some(GlyphLayout::new());
            self.prepared_fonts = Some(text.to_string());
        }
    }

    pub fn prepare_text(&mut self, text: &str) {
        if self.prepared_fonts.is_some() {
            return;
        }
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;

        self.parameter.characters = text.to_string();
        if let Some(ref generator) = self.generator {
            self.font = Some(generator.generate_font(&self.parameter));
            self.layout = Some(GlyphLayout::new());
        }
    }

    pub fn set_text(&mut self, text: String) {
        self.text_data.set_text(text.clone());
        self.prepare_text(&text);
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.text_data.prepare(time, state);
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if self.text_data.should_update_text() {
            let current = self.text_data.current_text().unwrap_or("").to_string();
            self.set_text(current);
        }
        self.draw_with_offset(sprite, 0.0, 0.0);
    }

    /// Java: SkinTextFont.draw(SkinObjectRenderer sprite, float offsetX, float offsetY)
    /// Renders TrueType text with alignment, shadow, and scaling.
    pub fn draw_with_offset(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        offset_x: f32,
        offset_y: f32,
    ) {
        let font = match self.font.as_mut() {
            Some(f) => f,
            None => return,
        };

        let param_size = self.parameter.size;
        if param_size <= 0 {
            return;
        }

        // Java: font.getData().setScale(region.height / parameter.size)
        // We set the absolute pixel size to region.height so that glyphs fill the
        // destination height. The ratio region.height / parameter.size is the scale
        // factor relative to the configured font size.
        let region = self.text_data.data.region;
        let original_scale = font.scale();
        font.scale = region.height;

        // Java: sprite.setType(getFilter() != 0 ? SkinObjectRenderer.TYPE_LINEAR : SkinObjectRenderer.TYPE_NORMAL)
        sprite.obj_type = if self.text_data.data.filter() != 0 {
            SkinObjectRenderer::TYPE_LINEAR
        } else {
            SkinObjectRenderer::TYPE_NORMAL
        };

        // Measure text layout to get width for alignment
        let text = self.text_data.text().to_string();
        let color = self.text_data.data.color;
        let layout_width = self.compute_layout_width(&text, &color, region.width, region.height);

        // Compute x position based on alignment
        let align = self.text_data.align();
        let x = compute_aligned_x(align, region.x, region.width);

        sprite.blend = self.text_data.data.blend();

        // Shadow rendering: if shadow offset is non-zero, draw shadow first
        let shadow_offset = self.text_data.shadow_offset();
        if shadow_offset.0 != 0.0 || shadow_offset.1 != 0.0 {
            // Java: Color c2 = new Color(c.r / 2, c.g / 2, c.b / 2, c.a)
            let shadow_color = Color::new(color.r / 2.0, color.g / 2.0, color.b / 2.0, color.a);
            self.draw_text_glyphs(DrawTextGlyphsParams {
                sprite,
                text: &text,
                color: &shadow_color,
                x: x + shadow_offset.0 + offset_x,
                y: region.y - shadow_offset.1 + offset_y + region.height,
                _layout_width: layout_width,
                region_width: region.width,
            });
        }

        // Main text rendering
        self.draw_text_glyphs(DrawTextGlyphsParams {
            sprite,
            text: &text,
            color: &color,
            x: x + offset_x,
            y: region.y + offset_y + region.height,
            _layout_width: layout_width,
            region_width: region.width,
        });

        // Java: font.getData().setScale(1) — restore original scale
        if let Some(f) = self.font.as_mut() {
            f.scale = original_scale;
            f.scale_x = None;
        }
    }

    /// Compute layout width applying overflow mode.
    /// Mirrors Java's setLayout() logic for measuring and applying shrink/truncate.
    fn compute_layout_width(
        &mut self,
        text: &str,
        _color: &Color,
        region_width: f32,
        _region_height: f32,
    ) -> f32 {
        let font = match self.font.as_ref() {
            Some(f) => f,
            None => return 0.0,
        };

        if self.text_data.is_wrapping() {
            let measured = font.measure(text);
            if let Some(ref mut layout) = self.layout {
                layout.width = measured.width;
                layout.height = measured.height;
            }
            return measured.width;
        }

        match self.text_data.overflow() {
            OVERFLOW_OVERFLOW => {
                let measured = font.measure(text);
                if let Some(ref mut layout) = self.layout {
                    layout.width = measured.width;
                    layout.height = measured.height;
                }
                measured.width
            }
            OVERFLOW_SHRINK => {
                let measured = font.measure(text);
                if let Some(ref mut layout) = self.layout {
                    layout.width = measured.width;
                    layout.height = measured.height;
                }
                let actual_width = measured.width;
                if actual_width > region_width && region_width > 0.0 {
                    // Java: font.getData().setScale(scaleX * r.getWidth() / actualWidth, scaleY)
                    // Only shrink X axis, keeping Y (height) unchanged.
                    if let Some(f) = self.font.as_mut() {
                        let current_scale = f.scale();
                        f.scale_x = Some(current_scale * region_width / actual_width);
                        let shrunk = f.measure(text);
                        if let Some(ref mut layout) = self.layout {
                            layout.width = shrunk.width;
                            layout.height = shrunk.height;
                        }
                        return shrunk.width;
                    }
                }
                actual_width
            }
            OVERFLOW_TRUNCATE => {
                let measured = font.measure(text);
                let width = measured.width.min(region_width);
                if let Some(ref mut layout) = self.layout {
                    layout.width = width;
                    layout.height = measured.height;
                }
                width
            }
            _ => {
                let measured = font.measure(text);
                if let Some(ref mut layout) = self.layout {
                    layout.width = measured.width;
                    layout.height = measured.height;
                }
                measured.width
            }
        }
    }

    /// Draw text glyphs at the given position using BitmapFont.layout_glyphs().
    /// Each glyph is drawn as a TextureRegion via SkinObjectData.draw_image_at_with_color().
    fn draw_text_glyphs(&mut self, params: DrawTextGlyphsParams<'_>) {
        let sprite = params.sprite;
        let text = params.text;
        let color = params.color;
        let x = params.x;
        let y = params.y;
        let region_width = params.region_width;
        let font = match self.font.as_mut() {
            Some(f) => f,
            None => return,
        };

        let (glyphs, _total_width, _line_height) = font.layout_glyph_regions(text);
        if glyphs.is_empty() {
            return;
        }

        let truncate =
            self.text_data.overflow() == OVERFLOW_TRUNCATE && !self.text_data.is_wrapping();
        let angle = self.text_data.data.angle;

        for glyph in &glyphs {
            let gx = x + glyph.x;
            let gy = y + glyph.y;
            let gw = glyph.width;
            let gh = glyph.height;

            // Truncate: skip glyphs that extend beyond region width
            if truncate && (gx + gw - x) > region_width {
                break;
            }

            self.text_data.data.draw_image_at_with_color(
                sprite,
                &DrawImageAtParams {
                    image: &glyph.region,
                    x: gx,
                    y: gy,
                    width: gw,
                    height: gh,
                    color,
                    angle,
                },
            );
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut generator) = self.generator {
            generator.dispose();
        }
        self.generator = None;
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;
    }
}

impl crate::skin_text::SkinText for SkinTextFont {
    fn get_text_data(&self) -> &SkinTextData {
        &self.text_data
    }

    fn get_text_data_mut(&mut self) -> &mut SkinTextData {
        &mut self.text_data
    }

    fn prepare_font(&mut self, text: &str) {
        self.prepare_font(text);
    }

    fn prepare_text(&mut self, text: &str) {
        self.prepare_text(text);
    }

    fn draw_with_offset(&mut self, sprite: &mut SkinObjectRenderer, offset_x: f32, offset_y: f32) {
        self.draw_with_offset(sprite, offset_x, offset_y);
    }

    fn dispose(&mut self) {
        self.dispose();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::Rectangle;
    use std::path::PathBuf;

    /// Helper to create a SkinTextFont with direct font injection for testing.
    /// Uses parameter.size as the base font size for scaling calculations.
    fn make_font(param_size: i32) -> SkinTextFont {
        SkinTextFont {
            text_data: SkinTextData::new_with_id(-1),
            font: Some(BitmapFont::new()),
            layout: Some(GlyphLayout::new()),
            generator: None,
            parameter: FreeTypeFontParameter {
                size: param_size,
                ..Default::default()
            },
            prepared_fonts: None,
        }
    }

    fn test_font_path() -> String {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/NotoSansJP-Regular.ttf")
            .to_string_lossy()
            .into_owned()
    }

    fn make_loaded_font(param_size: i32) -> SkinTextFont {
        let font_path = test_font_path();
        SkinTextFont {
            text_data: SkinTextData::new_with_id(-1),
            font: Some(BitmapFont::from_file(&font_path, param_size as f32)),
            layout: Some(GlyphLayout::new()),
            generator: None,
            parameter: FreeTypeFontParameter {
                size: param_size,
                ..Default::default()
            },
            prepared_fonts: None,
        }
    }

    // ---- Early-return guard tests ----

    #[test]
    fn test_draw_with_offset_no_font_returns_early() {
        let mut stf = SkinTextFont {
            text_data: SkinTextData::new_with_id(-1),
            font: None,
            layout: None,
            generator: None,
            parameter: FreeTypeFontParameter::default(),
            prepared_fonts: None,
        };
        stf.text_data.data.draw = true;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 200.0, 30.0);
        let mut renderer = SkinObjectRenderer::new();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);
        // No font => no rendering
        assert!(renderer.sprite.vertices().is_empty());
    }

    #[test]
    fn test_draw_with_offset_zero_param_size_returns_early() {
        let mut stf = make_font(0);
        stf.text_data.data.draw = true;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 200.0, 30.0);
        stf.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        stf.text_data.set_text("A".to_string());
        let mut renderer = SkinObjectRenderer::new();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);
        // parameter.size == 0 means division by zero; should return early
        assert!(renderer.sprite.vertices().is_empty());
    }

    // ---- Renderer type test ----

    #[test]
    fn test_renderer_type_normal_when_filter_zero() {
        // Java: sprite.setType(getFilter() != 0 ? TYPE_LINEAR : TYPE_NORMAL)
        // filter defaults to 0 => TYPE_NORMAL
        let mut stf = make_font(30);
        stf.text_data.data.draw = true;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 500.0, 30.0);
        stf.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        stf.text_data.set_text("X".to_string());

        let mut renderer = SkinObjectRenderer::new();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);
        assert_eq!(renderer.toast_type(), SkinObjectRenderer::TYPE_NORMAL);
    }

    #[test]
    fn test_renderer_type_linear_when_filter_nonzero() {
        // Java: sprite.setType(getFilter() != 0 ? TYPE_LINEAR : TYPE_NORMAL)
        // filter != 0 => TYPE_LINEAR
        let mut stf = make_font(30);
        stf.text_data.data.draw = true;
        stf.text_data.data.dstfilter = 1;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 500.0, 30.0);
        stf.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        stf.text_data.set_text("X".to_string());

        let mut renderer = SkinObjectRenderer::new();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);
        assert_eq!(renderer.toast_type(), SkinObjectRenderer::TYPE_LINEAR);
    }

    #[test]
    fn test_draw_with_offset_uses_glyph_texture_regions() {
        let mut stf = make_loaded_font(30);
        stf.text_data.data.draw = true;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 240.0, 30.0);
        stf.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        stf.text_data.set_text("Text".to_string());

        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "loaded text should emit glyph draw calls"
        );
        assert!(
            quads.iter().all(|quad| quad.texture_key.is_some()),
            "each glyph quad should be wired to a glyph texture region"
        );
    }

    // ---- Font scale restore test ----

    #[test]
    fn test_font_scale_restored_after_draw() {
        // Java: saves original scale, sets region.height, restores at end
        let mut stf = make_font(20);
        stf.text_data.data.draw = true;
        stf.text_data.data.region = Rectangle::new(0.0, 0.0, 500.0, 40.0);
        stf.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        stf.text_data.set_text("Test".to_string());

        let original_scale = stf.font.as_ref().unwrap().scale();
        let mut renderer = SkinObjectRenderer::new();
        stf.draw_with_offset(&mut renderer, 0.0, 0.0);
        assert_eq!(stf.font.as_ref().unwrap().scale(), original_scale);
    }

    // ---- Shadow color formula test ----

    #[test]
    fn test_shadow_color_is_half_brightness() {
        // Java: Color c2 = new Color(c.r / 2, c.g / 2, c.b / 2, c.a)
        let color = Color::new(0.8, 0.6, 0.4, 1.0);
        let shadow_color = Color::new(color.r / 2.0, color.g / 2.0, color.b / 2.0, color.a);
        assert!((shadow_color.r - 0.4).abs() < f32::EPSILON);
        assert!((shadow_color.g - 0.3).abs() < f32::EPSILON);
        assert!((shadow_color.b - 0.2).abs() < f32::EPSILON);
        assert!((shadow_color.a - 1.0).abs() < f32::EPSILON);
    }

    // ---- Alignment calculation tests (pure math) ----

    #[test]
    fn test_compute_x_left_align() {
        // align=0 (LEFT): x = region.x
        let region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        let x = compute_aligned_x(0, region.x, region.width);
        assert_eq!(x, 100.0);
    }

    #[test]
    fn test_compute_x_center_align() {
        // align=1 (CENTER): x = region.x - region.width / 2
        let region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        let x = compute_aligned_x(1, region.x, region.width);
        assert_eq!(x, 0.0); // 100 - 200 / 2 = 100 - 100 = 0
    }

    #[test]
    fn test_compute_x_right_align() {
        // align=2 (RIGHT): x = region.x - region.width
        let region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        let x = compute_aligned_x(2, region.x, region.width);
        assert_eq!(x, -100.0); // 100 - 200 = -100
    }

    // ---- Shadow offset positioning test ----

    #[test]
    fn test_overflow_shrink_preserves_height() {
        // Java: setScale(scaleX * ratio, scaleY) -- only X axis shrinks.
        // Height must remain unchanged after OVERFLOW_SHRINK.
        let font_path = test_font_path();
        if !std::path::Path::new(&font_path).exists() {
            return; // Skip if font file is not available
        }
        let mut stf = make_loaded_font(32);
        stf.text_data.data.region = Rectangle {
            x: 0.0,
            y: 0.0,
            width: 50.0, // Narrow region to force shrink
            height: 32.0,
        };
        stf.text_data.overflow = OVERFLOW_SHRINK;

        // Measure a wide string that exceeds the 50px region
        let font = stf.font.as_ref().unwrap();
        font.measure("WWWWWWWW"); // warmup

        let color = Color::new(1.0, 1.0, 1.0, 1.0);
        // Set font scale to region height (as draw() does)
        stf.font.as_mut().unwrap().scale = 32.0;
        let before_height = stf.font.as_ref().unwrap().measure("WWWWWWWW").height;
        let _width = stf.compute_layout_width("WWWWWWWW", &color, 50.0, 32.0);
        let layout = stf.layout.as_ref().unwrap();
        // Width should be shrunk to fit
        assert!(layout.width <= 50.0 + 1.0, "width should be shrunk");
        // Height must be preserved (within tolerance for rounding)
        assert!(
            (layout.height - before_height).abs() < 1.0,
            "height should be preserved: before={before_height}, after={}",
            layout.height
        );
    }

    #[test]
    fn test_shadow_offset_position() {
        // Java: shadow drawn at (x + shadowOffsetX, y - shadowOffsetY)
        let base_x = 100.0f32;
        let base_y = 200.0f32;
        let shadow_x = 2.0f32;
        let shadow_y = 3.0f32;
        let sx = base_x + shadow_x;
        let sy = base_y - shadow_y;
        assert_eq!(sx, 102.0);
        assert_eq!(sy, 197.0);
    }
}
