// Font rasterization using ab_glyph.
// Drop-in replacements for BitmapFont, BitmapFontData, GlyphLayout,
// FreeTypeFontGenerator, and FreeTypeFontParameter from rendering_stubs.rs.

use crate::color::Color;
use crate::glyph_atlas::GlyphAtlas;
use crate::sprite_batch::SpriteBatch;
use crate::texture::TextureRegion;

/// Font data (glyph metrics, kerning, etc.) parsed from AngelCode BMFont .fnt files.
/// Corresponds to com.badlogic.gdx.graphics.g2d.BitmapFont.BitmapFontData.
#[derive(Clone, Debug, Default)]
pub struct BitmapFontData {
    /// Paths to font texture page images (from "page" entries).
    pub image_paths: Vec<String>,
    /// Line height in pixels.
    pub line_height: f32,
    /// Baseline offset from top of line.
    pub base: f32,
    /// Font size as declared in .fnt header.
    pub font_size: f32,
    /// Texture page width.
    pub scale_w: f32,
    /// Texture page height.
    pub scale_h: f32,
    /// Glyph data keyed by character code.
    pub glyphs: std::collections::HashMap<u32, BitmapGlyph>,
}

/// Single glyph metrics from a .fnt file "char" entry.
#[derive(Clone, Copy, Debug, Default)]
pub struct BitmapGlyph {
    pub id: u32,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub xoffset: i32,
    pub yoffset: i32,
    pub xadvance: i32,
    pub page: i32,
}

impl BitmapFontData {
    /// Parse a .fnt file (AngelCode BMFont text format).
    pub fn from_fnt(path: &std::path::Path) -> Option<Self> {
        let content = std::fs::read_to_string(path).ok()?;
        Self::parse_fnt(&content, path.parent())
    }

    /// Parse .fnt content string, resolving image paths relative to `base_dir`.
    pub fn parse_fnt(content: &str, base_dir: Option<&std::path::Path>) -> Option<Self> {
        let mut data = Self::default();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("info ") {
                data.font_size = parse_fnt_field(line, "size=").unwrap_or(0) as f32;
            } else if line.starts_with("common ") {
                data.line_height = parse_fnt_field(line, "lineHeight=").unwrap_or(0) as f32;
                data.base = parse_fnt_field(line, "base=").unwrap_or(0) as f32;
                data.scale_w = parse_fnt_field(line, "scaleW=").unwrap_or(256) as f32;
                data.scale_h = parse_fnt_field(line, "scaleH=").unwrap_or(256) as f32;
            } else if line.starts_with("page ") {
                if let Some(file) = parse_fnt_string(line, "file=") {
                    let image_path = if let Some(dir) = base_dir {
                        dir.join(&file).to_string_lossy().to_string()
                    } else {
                        file
                    };
                    data.image_paths.push(image_path);
                }
            } else if line.starts_with("char ") {
                let id = parse_fnt_field(line, "id=").unwrap_or(0) as u32;
                let glyph = BitmapGlyph {
                    id,
                    x: parse_fnt_field(line, "x=").unwrap_or(0),
                    y: parse_fnt_field(line, "y=").unwrap_or(0),
                    width: parse_fnt_field(line, "width=").unwrap_or(0),
                    height: parse_fnt_field(line, "height=").unwrap_or(0),
                    xoffset: parse_fnt_field(line, "xoffset=").unwrap_or(0),
                    yoffset: parse_fnt_field(line, "yoffset=").unwrap_or(0),
                    xadvance: parse_fnt_field(line, "xadvance=").unwrap_or(0),
                    page: parse_fnt_field(line, "page=").unwrap_or(0),
                };
                data.glyphs.insert(id, glyph);
            }
        }

        Some(data)
    }
}

/// Parse an integer field like "key=123" from a .fnt line.
fn parse_fnt_field(line: &str, key: &str) -> Option<i32> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Parse a quoted string field like `file="name.png"` from a .fnt line.
fn parse_fnt_string(line: &str, key: &str) -> Option<String> {
    let start = line.find(key)? + key.len();
    let rest = &line[start..];
    if let Some(stripped) = rest.strip_prefix('"') {
        let end = stripped.find('"')?;
        Some(stripped[..end].to_string())
    } else {
        let end = rest.find(|c: char| c.is_whitespace()).unwrap_or(rest.len());
        Some(rest[..end].to_string())
    }
}

/// Positioned glyph for rendering.
/// Contains the character, its pixel position, and size within the layout.
#[derive(Clone, Copy, Debug)]
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

pub struct BitmapFont {
    font: Option<Arc<ab_glyph::FontVec>>,
    pub scale: f32,
    color: [f32; 4],
    atlas: Option<GlyphAtlas>,
}

impl Clone for BitmapFont {
    fn clone(&self) -> Self {
        Self {
            font: self.font.clone(),
            scale: self.scale,
            color: self.color,
            atlas: None, // Atlas is lazily rebuilt on clone
        }
    }
}

impl std::fmt::Debug for BitmapFont {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BitmapFont")
            .field("font", &self.font.is_some())
            .field("scale", &self.scale)
            .field("color", &self.color)
            .field("has_atlas", &self.atlas.is_some())
            .finish()
    }
}

impl Default for BitmapFont {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(unused_variables)]
impl BitmapFont {
    pub fn new() -> Self {
        Self {
            font: None,
            scale: 16.0,
            color: [1.0, 1.0, 1.0, 1.0],
            atlas: None,
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
            atlas: None,
        }
    }

    pub fn font(&self) -> Option<&Arc<ab_glyph::FontVec>> {
        self.font.as_ref()
    }

    pub fn regions(&self) -> Vec<TextureRegion> {
        vec![]
    }

    pub fn scale(&self) -> f32 {
        self.scale
    }

    pub fn set_color(&mut self, color: &Color) {
        self.color = color.to_array();
    }

    /// Draw text at (x, y) using the glyph atlas.
    /// Uses a two-pass approach: first rasterizes all needed glyphs and collects
    /// positions, then flushes the atlas texture once, then draws all quads.
    /// This produces 1 texture clone per draw() call instead of N (one per new glyph).
    pub fn draw(&mut self, batch: &mut SpriteBatch, text: &str, x: f32, y: f32) {
        use crate::glyph_atlas::CachedGlyph;
        use ab_glyph::{Font, ScaleFont};

        let Some(font) = self.font.clone() else {
            return;
        };
        let atlas = self.atlas.get_or_insert_with(GlyphAtlas::new);
        let scaled = font.as_scaled(ab_glyph::PxScale::from(self.scale));

        // Save current batch color
        let saved_color = batch.color();
        batch.set_color(&Color::new(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        ));

        let ascent = scaled.ascent();

        // Pass 1: rasterize all needed glyphs and collect positions
        let mut glyphs_to_draw: Vec<(CachedGlyph, f32, f32)> = Vec::new();
        let mut cursor_x = x;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }

            if let Some(cached) = atlas.get_or_rasterize(&font, glyph_id, self.scale) {
                let gx = cursor_x + cached.bearing_x;
                let gy = y + ascent + cached.bearing_y;
                glyphs_to_draw.push((cached, gx, gy));
            }

            cursor_x += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }

        // Snapshot texture once (1 clone instead of N)
        atlas.flush_texture_if_dirty();

        // Pass 2: draw all quads (all reference same texture version → 1 DrawBatch)
        for (cached, gx, gy) in &glyphs_to_draw {
            let region = atlas.texture_region(cached);
            batch.draw_region(&region, *gx, *gy, cached.width as f32, cached.height as f32);
        }

        // Restore batch color
        batch.set_color(&saved_color);
    }

    pub fn draw_layout(&self, batch: &mut SpriteBatch, layout: &GlyphLayout, x: f32, y: f32) {
        // Layout-based drawing not yet implemented; text is drawn via draw()
    }

    pub fn dispose(&mut self) {
        self.font = None;
        self.atlas = None;
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

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }
            let h_advance = scaled.h_advance(glyph_id);
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
#[derive(Clone, Copy, Debug, Default)]
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
