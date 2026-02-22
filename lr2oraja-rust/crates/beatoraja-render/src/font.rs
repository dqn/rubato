// Font rasterization using ab_glyph.
// Drop-in replacements for BitmapFont, BitmapFontData, GlyphLayout,
// FreeTypeFontGenerator, and FreeTypeFontParameter from rendering_stubs.rs.

use crate::color::Color;
use crate::sprite_batch::SpriteBatch;
use crate::texture::TextureRegion;

/// Font data (glyph metrics, kerning, etc.).
/// Corresponds to com.badlogic.gdx.graphics.g2d.BitmapFont.BitmapFontData.
#[derive(Clone, Debug, Default)]
pub struct BitmapFontData;

/// Positioned glyph for rendering.
/// Contains the character, its pixel position, and size within the layout.
#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub ch: char,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Bitmap font for text rendering.
/// Corresponds to com.badlogic.gdx.graphics.g2d.BitmapFont.
use std::sync::Arc;

#[derive(Clone, Debug, Default)]
pub struct BitmapFont {
    font: Option<Arc<ab_glyph::FontVec>>,
    scale: f32,
    color: [f32; 4],
}

#[allow(unused_variables)]
impl BitmapFont {
    pub fn new() -> Self {
        Self {
            font: None,
            scale: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    /// Create from a font file with a given pixel size.
    pub fn from_file(path: &str, size: f32) -> Self {
        let font = std::fs::read(path)
            .ok()
            .and_then(|data| ab_glyph::FontVec::try_from_vec(data).ok())
            .map(Arc::new);
        Self {
            font,
            scale: size,
            color: [1.0, 1.0, 1.0, 1.0],
        }
    }

    pub fn get_font(&self) -> Option<&Arc<ab_glyph::FontVec>> {
        self.font.as_ref()
    }

    pub fn get_regions(&self) -> Vec<TextureRegion> {
        vec![]
    }

    pub fn get_scale(&self) -> f32 {
        self.scale
    }

    pub fn set_scale(&mut self, scale: f32) {
        self.scale = scale;
    }

    pub fn set_color(&mut self, color: &Color) {
        self.color = color.to_array();
    }

    pub fn draw(&self, batch: &mut SpriteBatch, text: &str, x: f32, y: f32) {
        // TODO: rasterize glyphs into texture atlas and submit quads to batch
    }

    pub fn draw_layout(&self, batch: &mut SpriteBatch, layout: &GlyphLayout, x: f32, y: f32) {
        // TODO: render pre-measured layout
    }

    pub fn dispose(&mut self) {
        self.font = None;
    }

    /// Measure text and return a GlyphLayout with width/height.
    pub fn measure(&self, text: &str) -> GlyphLayout {
        use ab_glyph::{Font, ScaleFont};
        let Some(font) = self.font.as_ref() else {
            return GlyphLayout::default();
        };
        let scaled = font.as_scaled(ab_glyph::PxScale::from(self.scale));

        let mut width = 0.0f32;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                width += scaled.kern(prev, glyph_id);
            }
            width += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }

        let height = scaled.height();
        GlyphLayout { width, height }
    }

    /// Compute positioned glyphs for text at the current scale.
    /// Returns a list of glyphs with their pixel positions and dimensions,
    /// plus the total layout width and height.
    pub fn layout_glyphs(&self, text: &str) -> (Vec<PositionedGlyph>, f32, f32) {
        use ab_glyph::{Font, ScaleFont};
        let Some(font) = self.font.as_ref() else {
            return (vec![], 0.0, 0.0);
        };
        let scaled = font.as_scaled(ab_glyph::PxScale::from(self.scale));

        let mut glyphs = Vec::new();
        let mut cursor_x = 0.0f32;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
        let height = scaled.height();
        let ascent = scaled.ascent();

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }
            let h_advance = scaled.h_advance(glyph_id);
            // Use h_advance as glyph width for positioning; height is line height
            glyphs.push(PositionedGlyph {
                ch,
                x: cursor_x,
                y: 0.0,
                width: h_advance,
                height,
            });
            cursor_x += h_advance;
            prev_glyph = Some(glyph_id);
        }

        (glyphs, cursor_x, height)
    }
}

/// Pre-measured text layout.
/// Corresponds to com.badlogic.gdx.graphics.g2d.GlyphLayout.
#[derive(Clone, Debug, Default)]
pub struct GlyphLayout {
    pub width: f32,
    pub height: f32,
}

impl GlyphLayout {
    pub fn new() -> Self {
        Self::default()
    }
}

/// FreeType font generator.
/// Corresponds to com.badlogic.gdx.graphics.g2d.freetype.FreeTypeFontGenerator.
#[derive(Clone, Debug, Default)]
pub struct FreeTypeFontGenerator {
    path: String,
}

impl FreeTypeFontGenerator {
    pub fn new(font_file: &str) -> Self {
        Self {
            path: font_file.to_string(),
        }
    }

    pub fn generate_font(&self, param: &FreeTypeFontParameter) -> BitmapFont {
        BitmapFont::from_file(&self.path, param.size as f32)
    }

    pub fn dispose(&mut self) {}
}

/// Parameters for FreeType font generation.
#[derive(Clone, Debug, Default)]
pub struct FreeTypeFontParameter {
    pub size: i32,
    pub border_width: f32,
    pub border_color: Color,
    pub color: Color,
    pub characters: String,
}
