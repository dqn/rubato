// Font rasterization using ab_glyph.
// Drop-in replacements for BitmapFont, BitmapFontData, GlyphLayout,
// FreeTypeFontGenerator, and FreeTypeFontParameter from render_reexports.rs.

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
    /// Tries UTF-8 first, then falls back to MS932 (Shift_JIS) for Japanese skin assets.
    pub fn from_fnt(path: &std::path::Path) -> Option<Self> {
        let bytes = std::fs::read(path).ok()?;
        let content = match std::str::from_utf8(&bytes) {
            Ok(s) => s.to_string(),
            Err(_) => {
                let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                cow.into_owned()
            }
        };
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
                // NOTE: page id= is ignored; pages are stored in declaration order.
                // BMFont spec requires sequential IDs starting at 0, so this works
                // for conforming fonts. Non-sequential page IDs would cause wrong
                // texture lookups in glyph.page indexing.
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

        if data.glyphs.is_empty() || data.image_paths.is_empty() {
            return None;
        }

        Some(data)
    }
}

/// Parse an integer field like "key=123" from a .fnt line.
/// Uses word-boundary matching: the key must be preceded by a space or be at the
/// start of the line so that e.g. "x=" does not match inside "xoffset=".
fn parse_fnt_field(line: &str, key: &str) -> Option<i32> {
    let start = find_fnt_key(line, key)? + key.len();
    let rest = &line[start..];
    let end = rest
        .find(|c: char| !c.is_ascii_digit() && c != '-')
        .unwrap_or(rest.len());
    rest[..end].parse().ok()
}

/// Find the byte offset of `key` in `line` using word-boundary matching.
/// The key must be at the start of the line or preceded by a space/tab,
/// so that e.g. "x=" does not match inside "xoffset=".
fn find_fnt_key(line: &str, key: &str) -> Option<usize> {
    let mut search_from = 0;
    while search_from < line.len() {
        if let Some(pos) = line[search_from..].find(key) {
            let abs_pos = search_from + pos;
            if abs_pos == 0 || matches!(line.as_bytes()[abs_pos - 1], b' ' | b'\t') {
                return Some(abs_pos);
            }
            // Not a word boundary; skip past this match and keep searching.
            search_from = abs_pos + 1;
        } else {
            return None;
        }
    }
    None
}

/// Parse a quoted string field like `file="name.png"` from a .fnt line.
fn parse_fnt_string(line: &str, key: &str) -> Option<String> {
    let start = find_fnt_key(line, key)? + key.len();
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

/// Positioned glyph backed by a rasterized atlas region.
#[derive(Clone, Debug)]
pub struct PositionedGlyphRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub region: TextureRegion,
}

/// Bitmap font for text rendering.
/// Corresponds to com.badlogic.gdx.graphics.g2d.BitmapFont.
use std::sync::Arc;

pub struct BitmapFont {
    font: Option<Arc<ab_glyph::FontVec>>,
    pub scale: f32,
    /// Optional X-axis scale override for non-uniform scaling (OVERFLOW_SHRINK).
    /// When Some, horizontal metrics use this value while vertical metrics use `scale`.
    pub scale_x: Option<f32>,
    color: [f32; 4],
    atlas: Option<GlyphAtlas>,
}

impl Clone for BitmapFont {
    fn clone(&self) -> Self {
        Self {
            font: self.font.clone(),
            scale: self.scale,
            scale_x: self.scale_x,
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
            scale_x: None,
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
            scale_x: None,
            color: [1.0, 1.0, 1.0, 1.0],
            atlas: None,
        }
    }

    /// Return the PxScale used for glyph metrics. When `scale_x` is set,
    /// horizontal and vertical scales differ (Java setScale(scaleX, scaleY) parity).
    pub fn px_scale(&self) -> ab_glyph::PxScale {
        ab_glyph::PxScale {
            x: self.scale_x.unwrap_or(self.scale),
            y: self.scale,
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
        // Save current batch color
        let saved_color = batch.color();
        batch.set_color(&Color::new(
            self.color[0],
            self.color[1],
            self.color[2],
            self.color[3],
        ));

        let (glyphs, _width, _height) = self.layout_glyph_regions(text);
        for glyph in &glyphs {
            batch.draw_region(
                &glyph.region,
                x + glyph.x,
                y + glyph.y,
                glyph.width,
                glyph.height,
            );
        }

        // Restore batch color
        batch.set_color(&saved_color);
    }

    pub fn draw_layout(&mut self, batch: &mut SpriteBatch, layout: &GlyphLayout, x: f32, y: f32) {
        self.draw(batch, &layout.text, x, y);
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
        let scaled = font.as_scaled(self.px_scale());

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
        GlyphLayout {
            width,
            height,
            text: text.to_string(),
        }
    }

    /// Compute positioned glyphs for text at the current scale.
    /// Returns a list of glyphs with their pixel positions and dimensions,
    /// plus the total layout width and height.
    pub fn layout_glyphs(&self, text: &str) -> (Vec<PositionedGlyph>, f32, f32) {
        use ab_glyph::{Font, ScaleFont};
        let Some(font) = self.font.as_ref() else {
            return (vec![], 0.0, 0.0);
        };
        let scaled = font.as_scaled(self.px_scale());

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

    /// Compute positioned glyphs backed by atlas texture regions.
    pub fn layout_glyph_regions(&mut self, text: &str) -> (Vec<PositionedGlyphRegion>, f32, f32) {
        use crate::glyph_atlas::CachedGlyph;
        use ab_glyph::{Font, ScaleFont};

        let Some(font) = self.font.clone() else {
            return (vec![], 0.0, 0.0);
        };

        let px = self.px_scale();
        let atlas = self.atlas.get_or_insert_with(GlyphAtlas::new);
        let scaled = font.as_scaled(px);
        let ascent = scaled.ascent();
        let line_height = scaled.height();
        // Ratio for scaling rasterized glyph bitmaps (rendered at Y scale) to X scale.
        let x_ratio = if self.scale > 0.0 {
            px.x / self.scale
        } else {
            1.0
        };

        let mut cursor_x = 0.0f32;
        let mut prev_glyph: Option<ab_glyph::GlyphId> = None;
        let mut cached_glyphs: Vec<(CachedGlyph, f32, f32)> = Vec::new();

        for ch in text.chars() {
            let glyph_id = scaled.glyph_id(ch);
            if let Some(prev) = prev_glyph {
                cursor_x += scaled.kern(prev, glyph_id);
            }

            // Rasterize at Y scale for correct height; X scaling applied to positioning/width.
            if let Some(cached) = atlas.get_or_rasterize(&font, glyph_id, self.scale) {
                cached_glyphs.push((
                    cached,
                    cursor_x + cached.bearing_x * x_ratio,
                    ascent + cached.bearing_y,
                ));
            }

            cursor_x += scaled.h_advance(glyph_id);
            prev_glyph = Some(glyph_id);
        }

        atlas.flush_texture_if_dirty();

        let glyphs = cached_glyphs
            .into_iter()
            .map(|(cached, x, y)| PositionedGlyphRegion {
                x,
                y,
                width: cached.width as f32 * x_ratio,
                height: cached.height as f32,
                region: atlas.texture_region(&cached),
            })
            .collect();

        (glyphs, cursor_x, line_height)
    }
}

/// Pre-measured text layout.
/// Corresponds to com.badlogic.gdx.graphics.g2d.GlyphLayout.
#[derive(Clone, Debug, Default)]
pub struct GlyphLayout {
    pub width: f32,
    pub height: f32,
    /// The text that was measured. Stored so that `draw_layout()` can render
    /// without the caller re-supplying the string.
    pub text: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    // --- BitmapFontData::parse_fnt tests ---

    #[test]
    fn parse_fnt_empty_string() {
        let data = BitmapFontData::parse_fnt("", None);
        assert!(
            data.is_none(),
            "empty .fnt content must return None (no glyphs, no pages)"
        );
    }

    #[test]
    fn parse_fnt_info_line() {
        let content = "info face=\"TestFont\" size=24 bold=0\n\
page id=0 file=\"t.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.font_size, 24.0);
    }

    #[test]
    fn parse_fnt_common_line() {
        let content = "common lineHeight=32 base=25 scaleW=512 scaleH=256\n\
page id=0 file=\"t.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.line_height, 32.0);
        assert_eq!(data.base, 25.0);
        assert_eq!(data.scale_w, 512.0);
        assert_eq!(data.scale_h, 256.0);
    }

    #[test]
    fn parse_fnt_page_line_with_quotes() {
        let content = "page id=0 file=\"font_page0.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.image_paths.len(), 1);
        assert_eq!(data.image_paths[0], "font_page0.png");
    }

    #[test]
    fn parse_fnt_page_line_without_quotes() {
        let content = "page id=0 file=font_page0.png\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.image_paths.len(), 1);
        assert_eq!(data.image_paths[0], "font_page0.png");
    }

    #[test]
    fn parse_fnt_char_line() {
        let content = "page id=0 file=\"t.png\"\n\
char id=65 x=10 y=20 width=30 height=40 xoffset=2 yoffset=3 xadvance=35 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.glyphs.len(), 1);
        let glyph = data.glyphs.get(&65).unwrap();
        assert_eq!(glyph.id, 65);
        assert_eq!(glyph.x, 10);
        assert_eq!(glyph.y, 20);
        assert_eq!(glyph.width, 30);
        assert_eq!(glyph.height, 40);
        assert_eq!(glyph.xoffset, 2);
        assert_eq!(glyph.yoffset, 3);
        assert_eq!(glyph.xadvance, 35);
        assert_eq!(glyph.page, 0);
    }

    #[test]
    fn parse_fnt_multiple_chars() {
        let content = "\
page id=0 file=\"t.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n\
char id=66 x=10 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n\
char id=67 x=20 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.glyphs.len(), 3);
        assert!(data.glyphs.contains_key(&65));
        assert!(data.glyphs.contains_key(&66));
        assert!(data.glyphs.contains_key(&67));
    }

    #[test]
    fn parse_fnt_ignores_unknown_lines() {
        let content = "\
page id=0 file=\"t.png\"\n\
kerning first=65 second=66 amount=-1\n\
chars count=1\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        assert_eq!(data.glyphs.len(), 1);
    }

    #[test]
    fn parse_fnt_negative_offsets() {
        let content = "page id=0 file=\"t.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=-2 yoffset=-3 xadvance=8 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None).unwrap();
        let glyph = data.glyphs.get(&65).unwrap();
        assert_eq!(glyph.xoffset, -2);
        assert_eq!(glyph.yoffset, -3);
    }

    #[test]
    fn parse_fnt_page_with_base_dir() {
        let base_dir = std::path::Path::new("/fonts");
        let content = "page id=0 file=\"atlas.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, Some(base_dir)).unwrap();
        assert_eq!(data.image_paths.len(), 1);
        assert!(data.image_paths[0].contains("atlas.png"));
        assert!(data.image_paths[0].starts_with("/fonts"));
    }

    // --- parse_fnt_field / find_fnt_key word-boundary matching regression tests ---

    #[test]
    fn parse_fnt_field_x_not_confused_with_xoffset() {
        // When xoffset= appears before x= in the line, substring matching would
        // incorrectly match "x=" inside "xoffset=" and return the xoffset value.
        let line = "page id=0 file=\"t.png\"\n\
char id=65 xoffset=99 x=10 y=20 width=30 height=40 yoffset=3 xadvance=35 page=0";
        let data = BitmapFontData::parse_fnt(line, None).unwrap();
        let glyph = data.glyphs.get(&65).unwrap();
        assert_eq!(glyph.x, 10, "x= must not match inside xoffset=");
        assert_eq!(glyph.xoffset, 99);
    }

    #[test]
    fn parse_fnt_field_y_not_confused_with_yoffset() {
        // Same issue: "y=" is a substring of "yoffset=".
        let line = "page id=0 file=\"t.png\"\n\
char id=65 x=10 yoffset=88 y=20 width=30 height=40 xoffset=3 xadvance=35 page=0";
        let data = BitmapFontData::parse_fnt(line, None).unwrap();
        let glyph = data.glyphs.get(&65).unwrap();
        assert_eq!(glyph.y, 20, "y= must not match inside yoffset=");
        assert_eq!(glyph.yoffset, 88);
    }

    #[test]
    fn find_fnt_key_at_line_start() {
        assert_eq!(find_fnt_key("x=10 y=20", "x="), Some(0));
    }

    #[test]
    fn find_fnt_key_after_space() {
        assert_eq!(find_fnt_key("char x=10", "x="), Some(5));
    }

    #[test]
    fn find_fnt_key_rejects_substring_match() {
        // "x=" must not match inside "xoffset="
        assert_eq!(find_fnt_key("xoffset=3", "x="), None);
    }

    #[test]
    fn find_fnt_key_skips_substring_finds_real_match() {
        // First occurrence of "x=" is inside "xoffset=", second is the real "x=".
        assert_eq!(find_fnt_key("xoffset=3 x=10", "x="), Some(10));
    }

    // --- GlyphLayout tests ---

    #[test]
    fn glyph_layout_default() {
        let layout = GlyphLayout::default();
        assert_eq!(layout.width, 0.0);
        assert_eq!(layout.height, 0.0);
        assert!(layout.text.is_empty());
    }

    #[test]
    fn glyph_layout_new_equals_default() {
        let a = GlyphLayout::new();
        let b = GlyphLayout::default();
        assert_eq!(a.width, b.width);
        assert_eq!(a.height, b.height);
    }

    // --- BitmapFont tests ---

    #[test]
    fn bitmap_font_default_scale() {
        let font = BitmapFont::new();
        assert_eq!(font.scale, 16.0);
    }

    #[test]
    fn bitmap_font_no_font_loaded() {
        let font = BitmapFont::new();
        assert!(font.font().is_none());
    }

    #[test]
    fn bitmap_font_from_nonexistent_file() {
        let font = BitmapFont::from_file("/nonexistent/font.ttf", 24.0);
        assert!(font.font().is_none());
        assert_eq!(font.scale, 24.0);
    }

    #[test]
    fn bitmap_font_measure_empty_text() {
        let font = BitmapFont::new();
        let layout = font.measure("");
        assert_eq!(layout.width, 0.0);
    }

    #[test]
    fn bitmap_font_regions_empty() {
        let font = BitmapFont::new();
        assert!(font.regions().is_empty());
    }

    #[test]
    fn bitmap_font_clone() {
        let font = BitmapFont::new();
        let cloned = font.clone();
        assert_eq!(font.scale, cloned.scale);
    }

    #[test]
    fn bitmap_font_dispose() {
        let mut font = BitmapFont::new();
        font.dispose();
        assert!(font.font().is_none());
    }

    #[test]
    fn bitmap_font_set_color() {
        let mut font = BitmapFont::new();
        font.set_color(&Color::new(1.0, 0.0, 0.0, 0.5));
        // Color is stored internally; verify it doesn't panic
    }

    // --- FreeTypeFontGenerator tests ---

    #[test]
    fn freetype_generator_new() {
        let generator = FreeTypeFontGenerator::new("test.ttf");
        let debug = format!("{:?}", generator);
        assert!(debug.contains("test.ttf"));
    }

    #[test]
    fn freetype_generator_dispose_no_panic() {
        let mut generator = FreeTypeFontGenerator::new("test.ttf");
        generator.dispose(); // Should not panic
    }

    #[test]
    fn parse_fnt_no_glyphs_returns_none() {
        // Has a page but no char entries -- should be rejected.
        let content = "\
info face=\"Test\" size=16 bold=0\n\
common lineHeight=20 base=16 scaleW=256 scaleH=256\n\
page id=0 file=\"test.png\"\n";
        let data = BitmapFontData::parse_fnt(content, None);
        assert!(
            data.is_none(),
            "parse_fnt must return None when no glyphs are parsed"
        );
    }

    #[test]
    fn parse_fnt_no_pages_returns_none() {
        // Has glyphs but no page entries -- should be rejected.
        let content = "\
info face=\"Test\" size=16 bold=0\n\
common lineHeight=20 base=16 scaleW=256 scaleH=256\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None);
        assert!(
            data.is_none(),
            "parse_fnt must return None when no pages are parsed"
        );
    }

    #[test]
    fn parse_fnt_valid_returns_some() {
        // Both pages and glyphs present -- should succeed.
        let content = "\
info face=\"Test\" size=16 bold=0\n\
common lineHeight=20 base=16 scaleW=256 scaleH=256\n\
page id=0 file=\"test.png\"\n\
char id=65 x=0 y=0 width=10 height=10 xoffset=0 yoffset=0 xadvance=12 page=0\n";
        let data = BitmapFontData::parse_fnt(content, None);
        assert!(
            data.is_some(),
            "parse_fnt must return Some when both glyphs and pages are present"
        );
    }
}
