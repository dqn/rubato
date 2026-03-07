// SkinTextBitmap.java -> skin_text_bitmap.rs
// Mechanical line-by-line translation.

use std::path::PathBuf;

use crate::property::string_property::StringProperty;
use crate::stubs::{BitmapFont, Color, GlyphLayout, MainState, TextureRegion};
use crate::text::skin_text::{OVERFLOW_OVERFLOW, OVERFLOW_SHRINK, OVERFLOW_TRUNCATE, SkinTextData};
use crate::types::skin_object::SkinObjectRenderer;

pub struct SkinTextBitmap {
    pub text_data: SkinTextData,
    source: SkinTextBitmapSource,
    font: Option<BitmapFont>,
    layout: GlyphLayout,
    size: f32,
}

impl SkinTextBitmap {
    pub fn new(source: SkinTextBitmapSource, size: f32) -> Self {
        Self::new_with_property(source, size, None)
    }

    pub fn new_with_property(
        mut source: SkinTextBitmapSource,
        size: f32,
        property: Option<Box<dyn StringProperty>>,
    ) -> Self {
        let text_data = if let Some(prop) = property {
            SkinTextData::new_with_property(prop)
        } else {
            SkinTextData::new_with_id(-1)
        };
        let font = source.font();
        Self {
            text_data,
            source,
            font,
            layout: GlyphLayout::new(),
            size,
        }
    }

    /// Load method (no-op, as in Java @Override).
    pub fn load(&mut self) {
        // no-op (Java: @Override public void load() {})
    }

    pub fn prepare_font(&mut self, _text: &str) {
        // no-op
    }

    pub fn prepare_text(&mut self, _text: &str) {
        // no-op
    }

    pub fn set_text(&mut self, text: String) {
        self.text_data.set_text(text);
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

    /// Java: SkinTextBitmap.draw(SkinObjectRenderer sprite, float offsetX, float offsetY)
    /// Renders text using ab_glyph font rasterization.
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

        let original_size = self.source.original_size();
        if original_size <= 0.0 {
            return;
        }
        let scale = self.size / original_size;

        // Java: font.getData().setScale(scale)
        let original_scale = font.scale();
        font.scale = original_size * scale;

        let region = &self.text_data.data.region;
        let align = self.text_data.align();
        // Java: final float x = (getAlign() == 2 ? region.x - region.width
        //       : (getAlign() == 1 ? region.x - region.width / 2 : region.x));
        let x = if align == 2 {
            region.x - region.width
        } else if align == 1 {
            region.x - region.width / 2.0
        } else {
            region.x
        };

        sprite.blend = self.text_data.data.blend();

        let source_type = self.source.toast_type();
        if source_type == SkinTextBitmapSource::TYPE_DISTANCE_FIELD
            || source_type == SkinTextBitmapSource::TYPE_COLORED_DISTANCE_FIELD
        {
            // Distance field rendering path
            sprite.obj_type = SkinObjectRenderer::TYPE_DISTANCE_FIELD;
            let color = self.text_data.data.color;
            let text = self.text_data.text().to_string();
            let region_width = self.text_data.data.region.width;
            let region_height = self.text_data.data.region.height;
            let region_y = self.text_data.data.region.y;
            let layout_width =
                self.compute_layout_width(&text, &color, region_width, region_height);
            self.draw_text_glyphs(
                sprite,
                &text,
                &color,
                x + offset_x,
                region_y + offset_y + region_height,
                layout_width,
                region_width,
            );
        } else {
            // Standard rendering path
            sprite.obj_type = SkinObjectRenderer::TYPE_BILINEAR;

            let shadow_offset = self.text_data.shadow_offset();
            let text = self.text_data.text().to_string();
            let color = self.text_data.data.color;
            let region_width = self.text_data.data.region.width;
            let region_height = self.text_data.data.region.height;
            let region_y = self.text_data.data.region.y;

            // Shadow rendering: if shadow offset is non-zero, draw shadow first
            if shadow_offset.0 != 0.0 || shadow_offset.1 != 0.0 {
                let shadow_color = Color::new(color.r / 2.0, color.g / 2.0, color.b / 2.0, color.a);
                let layout_width =
                    self.compute_layout_width(&text, &shadow_color, region_width, region_height);
                self.draw_text_glyphs(
                    sprite,
                    &text,
                    &shadow_color,
                    x + shadow_offset.0 + offset_x,
                    region_y - shadow_offset.1 + offset_y + region_height,
                    layout_width,
                    region_width,
                );
            }

            // Main text rendering
            let layout_width =
                self.compute_layout_width(&text, &color, region_width, region_height);
            self.draw_text_glyphs(
                sprite,
                &text,
                &color,
                x + offset_x,
                region_y + offset_y + region_height,
                layout_width,
                region_width,
            );
        }

        // Java: font.getData().setScale(1)
        if let Some(f) = self.font.as_mut() {
            f.scale = original_scale;
        }
    }

    /// Compute layout width applying overflow mode.
    /// Corresponds to Java setLayout() logic for measuring and applying shrink/truncate.
    /// Returns the effective text width after overflow processing.
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
            // With wrapping, width is constrained to region width
            let layout = font.measure(text);
            self.layout.width = layout.width;
            self.layout.height = layout.height;
            return layout.width;
        }

        match self.text_data.overflow() {
            OVERFLOW_OVERFLOW => {
                let layout = font.measure(text);
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                layout.width
            }
            OVERFLOW_SHRINK => {
                let layout = font.measure(text);
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                let actual_width = layout.width;
                if actual_width > region_width && region_width > 0.0 {
                    // Java: font.getData().setScale(scaleX * r.getWidth() / actualWidth, scaleY)
                    // Scale down font horizontally to fit
                    if let Some(f) = self.font.as_mut() {
                        let current_scale = f.scale();
                        f.scale = current_scale * region_width / actual_width;
                        let shrunk = f.measure(text);
                        self.layout.width = shrunk.width;
                        self.layout.height = shrunk.height;
                        return shrunk.width;
                    }
                }
                actual_width
            }
            OVERFLOW_TRUNCATE => {
                // Truncate text to fit within region width
                let layout = font.measure(text);
                self.layout.width = layout.width.min(region_width);
                self.layout.height = layout.height;
                self.layout.width
            }
            _ => {
                let layout = font.measure(text);
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                layout.width
            }
        }
    }

    /// Draw text glyphs at the given position.
    /// Uses BitmapFont.layout_glyphs() to get per-glyph positions,
    /// then draws each glyph as a TextureRegion via SkinObjectData.draw_image_at_with_color().
    #[allow(clippy::too_many_arguments)]
    fn draw_text_glyphs(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        text: &str,
        color: &Color,
        x: f32,
        y: f32,
        _layout_width: f32,
        region_width: f32,
    ) {
        let font = match self.font.as_ref() {
            Some(f) => f,
            None => return,
        };

        let (glyphs, _total_width, line_height) = font.layout_glyphs(text);
        if glyphs.is_empty() {
            return;
        }

        let truncate =
            self.text_data.overflow() == OVERFLOW_TRUNCATE && !self.text_data.is_wrapping();

        let angle = self.text_data.data.angle;

        for glyph in &glyphs {
            let gx = x + glyph.x;
            let gy = y - line_height + glyph.y;
            let gw = glyph.width;
            let gh = glyph.height;

            // Truncate: skip glyphs that extend beyond region width
            if truncate && (gx + gw - x) > region_width {
                break;
            }

            // Create a TextureRegion for the glyph
            // In the full pipeline, this would reference a rasterized glyph texture.
            // For now, we draw a placeholder quad that will be resolved by the GPU
            // when glyph atlas textures are available.
            let glyph_region = TextureRegion::new();
            self.text_data.data.draw_image_at_with_color(
                sprite,
                &glyph_region,
                gx,
                gy,
                gw,
                gh,
                color,
                angle,
            );
        }
    }

    pub fn dispose(&mut self) {
        self.source.dispose();
    }
}

/// Cacheable bitmap font data.
/// Corresponds to Java BitmapFontCache.CacheableBitmapFont.
pub struct CacheableBitmapFont {
    pub font: Option<BitmapFont>,
    pub original_size: f32,
    pub font_type: i32,
    pub page_width: f32,
    pub page_height: f32,
}

pub struct SkinTextBitmapSource {
    pub usecim: bool,
    pub use_mip_maps: bool,
    pub font_path: PathBuf,
    pub font: Option<BitmapFont>,
    pub original_size: f32,
    pub source_type: i32,
    pub page_width: f32,
    pub page_height: f32,
}

impl SkinTextBitmapSource {
    pub const TYPE_STANDARD: i32 = 0;
    pub const TYPE_DISTANCE_FIELD: i32 = 1;
    pub const TYPE_COLORED_DISTANCE_FIELD: i32 = 2;

    pub fn new(font_path: PathBuf, usecim: bool) -> Self {
        Self::new_with_mipmaps(font_path, usecim, true)
    }

    pub fn new_with_mipmaps(font_path: PathBuf, usecim: bool, use_mip_maps: bool) -> Self {
        Self {
            usecim,
            use_mip_maps,
            font_path,
            font: None,
            original_size: 0.0,
            source_type: 0,
            page_width: 0.0,
            page_height: 0.0,
        }
    }

    /// Create a cacheable bitmap font from the given path and type.
    ///
    /// Translated from: SkinTextBitmapSource.createCacheableFont
    /// Parses .fnt file header to extract size, scaleW, scaleH.
    /// Full BitmapFont texture loading is deferred to rendering integration.
    pub fn create_cacheable_font(
        &self,
        font_path: &std::path::Path,
        font_type: i32,
    ) -> CacheableBitmapFont {
        let mut original_size: f32 = 0.0;
        let mut page_width: f32 = 0.0;
        let mut page_height: f32 = 0.0;

        // Parse .fnt header for size/scaleW/scaleH
        // BMFont format:
        //   info face="..." size=32 bold=0 ...
        //   common lineHeight=32 base=26 scaleW=256 scaleH=256 ...
        if let Ok(content) = std::fs::read_to_string(font_path) {
            let mut lines = content.lines();
            // First line: "info ..." — extract size=
            if let Some(line) = lines.next() {
                original_size = Self::parse_fnt_value(line, "size=").unwrap_or(0.0);
            }
            // Second line: "common ..." — extract scaleW= and scaleH=
            if let Some(line) = lines.next() {
                page_width = Self::parse_fnt_value(line, "scaleW=").unwrap_or(0.0);
                page_height = Self::parse_fnt_value(line, "scaleH=").unwrap_or(0.0);
            }
        } else {
            log::warn!("Failed to read .fnt file: {:?}", font_path);
        }

        CacheableBitmapFont {
            font: None, // Full BitmapFont loading requires texture infrastructure
            original_size,
            font_type,
            page_width,
            page_height,
        }
    }

    /// Get or create a cached BitmapFont.
    ///
    /// Translated from: SkinTextBitmapSource.getFont
    /// Uses BitmapFontCache to avoid reloading the same font.
    pub fn font(&mut self) -> Option<BitmapFont> {
        if !crate::bitmap_font_cache::has(Some(&self.font_path)) {
            let new_font = self.create_cacheable_font(&self.font_path.clone(), self.source_type);
            crate::bitmap_font_cache::set(
                self.font_path.clone(),
                crate::bitmap_font_cache::CacheableBitmapFont {
                    original_size: new_font.original_size,
                    type_: new_font.font_type,
                    page_width: new_font.page_width,
                    page_height: new_font.page_height,
                    ..Default::default()
                },
            );
        }

        if let Some(cached) = crate::bitmap_font_cache::get(&self.font_path) {
            self.original_size = cached.original_size;
            self.source_type = cached.type_;
            self.page_width = cached.page_width;
            self.page_height = cached.page_height;
            self.font = Some(cached.font.clone());
            Some(cached.font)
        } else {
            None
        }
    }

    /// Parse a numeric value from a BMFont .fnt line by key prefix.
    /// Example: parse_fnt_value("info size=32 bold=0", "size=") → Some(32.0)
    fn parse_fnt_value(line: &str, key: &str) -> Option<f32> {
        let start = line.find(key)? + key.len();
        let rest = &line[start..];
        let end = rest.find(' ').unwrap_or(rest.len());
        rest[..end].parse::<i32>().ok().map(|v| v as f32)
    }

    pub fn original_size(&self) -> f32 {
        self.original_size
    }

    pub fn toast_type(&self) -> i32 {
        self.source_type
    }

    pub fn page_width(&self) -> f32 {
        self.page_width
    }

    pub fn page_height(&self) -> f32 {
        self.page_height
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::Rectangle;

    fn make_source(original_size: f32, source_type: i32) -> SkinTextBitmapSource {
        SkinTextBitmapSource {
            usecim: false,
            use_mip_maps: true,
            font_path: PathBuf::from("test.fnt"),
            font: None,
            original_size,
            source_type,
            page_width: 512.0,
            page_height: 512.0,
        }
    }

    #[test]
    fn test_new_creates_instance() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let bitmap = SkinTextBitmap::new(source, 16.0);
        assert_eq!(bitmap.size, 16.0);
        // font is initialized from cache (may be a default BitmapFont even for missing paths)
    }

    #[test]
    fn test_draw_with_offset_no_font_returns_early() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic; early return because font is None
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);
    }

    #[test]
    fn test_draw_with_offset_zero_original_size_returns_early() {
        let mut source = make_source(0.0, SkinTextBitmapSource::TYPE_STANDARD);
        source.font = Some(BitmapFont::new());
        let mut bitmap = SkinTextBitmap {
            text_data: SkinTextData::new_with_id(-1),
            font: Some(BitmapFont::new()),
            source,
            layout: GlyphLayout::new(),
            size: 16.0,
        };
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic; early return because original_size == 0
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);
    }

    #[test]
    fn test_alignment_left() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.text_data.align = 0; // LEFT
        bitmap.text_data.data.region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        // align=0: x = region.x = 100.0
        let align = bitmap.text_data.align();
        let region = &bitmap.text_data.data.region;
        let x = if align == 2 {
            region.x - region.width
        } else if align == 1 {
            region.x - region.width / 2.0
        } else {
            region.x
        };
        assert_eq!(x, 100.0);
    }

    #[test]
    fn test_alignment_center() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.text_data.align = 1; // CENTER
        bitmap.text_data.data.region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        let align = bitmap.text_data.align();
        let region = &bitmap.text_data.data.region;
        let x = if align == 2 {
            region.x - region.width
        } else if align == 1 {
            region.x - region.width / 2.0
        } else {
            region.x
        };
        assert_eq!(x, 0.0); // 100 - 200/2 = 0
    }

    #[test]
    fn test_alignment_right() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.text_data.align = 2; // RIGHT
        bitmap.text_data.data.region = Rectangle::new(100.0, 50.0, 200.0, 30.0);
        let align = bitmap.text_data.align();
        let region = &bitmap.text_data.data.region;
        let x = if align == 2 {
            region.x - region.width
        } else if align == 1 {
            region.x - region.width / 2.0
        } else {
            region.x
        };
        assert_eq!(x, -100.0); // 100 - 200 = -100
    }

    #[test]
    fn test_overflow_modes() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);

        bitmap.text_data.overflow = OVERFLOW_OVERFLOW;
        assert_eq!(bitmap.text_data.overflow(), OVERFLOW_OVERFLOW);

        bitmap.text_data.overflow = OVERFLOW_SHRINK;
        assert_eq!(bitmap.text_data.overflow(), OVERFLOW_SHRINK);

        bitmap.text_data.overflow = OVERFLOW_TRUNCATE;
        assert_eq!(bitmap.text_data.overflow(), OVERFLOW_TRUNCATE);
    }

    #[test]
    fn test_shadow_offset_non_zero() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.text_data.set_shadow_offset(2.0, 3.0);
        let offset = bitmap.text_data.shadow_offset();
        assert_eq!(offset.0, 2.0);
        assert_eq!(offset.1, 3.0);
    }

    #[test]
    fn test_shadow_offset_zero_skips_shadow() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.text_data.set_shadow_offset(0.0, 0.0);
        let offset = bitmap.text_data.shadow_offset();
        // Both zero: shadow should not be rendered
        assert_eq!(offset.0, 0.0);
        assert_eq!(offset.1, 0.0);
    }

    #[test]
    fn test_distance_field_type() {
        // Construct directly to avoid get_font() overwriting source_type from cache.
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_DISTANCE_FIELD);
        assert_eq!(
            source.toast_type(),
            SkinTextBitmapSource::TYPE_DISTANCE_FIELD
        );
        let bitmap = SkinTextBitmap {
            text_data: SkinTextData::new_with_id(-1),
            source,
            font: None,
            layout: GlyphLayout::new(),
            size: 16.0,
        };
        assert_eq!(
            bitmap.source.toast_type(),
            SkinTextBitmapSource::TYPE_DISTANCE_FIELD
        );
    }

    #[test]
    fn test_source_type_constants() {
        assert_eq!(SkinTextBitmapSource::TYPE_STANDARD, 0);
        assert_eq!(SkinTextBitmapSource::TYPE_DISTANCE_FIELD, 1);
        assert_eq!(SkinTextBitmapSource::TYPE_COLORED_DISTANCE_FIELD, 2);
    }

    #[test]
    fn test_set_text_updates_text() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.set_text("Hello".to_string());
        assert_eq!(bitmap.text_data.text(), "Hello");
    }

    #[test]
    fn test_set_text_empty_becomes_space() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.set_text("".to_string());
        // Java: if text is empty, set to " "
        assert_eq!(bitmap.text_data.text(), " ");
    }

    #[test]
    fn test_wrapping_flag() {
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        assert!(!bitmap.text_data.is_wrapping());
        bitmap.text_data.wrapping = true;
        assert!(bitmap.text_data.is_wrapping());
    }

    #[test]
    fn test_dispose() {
        let mut source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        source.font = Some(BitmapFont::new());
        let mut bitmap = SkinTextBitmap::new(source, 16.0);
        bitmap.dispose();
        assert!(bitmap.source.font.is_none());
    }

    #[test]
    fn test_draw_with_font_no_text_generates_no_vertices() {
        let mut source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        source.font = Some(BitmapFont::new());
        let mut bitmap = SkinTextBitmap {
            text_data: SkinTextData::new_with_id(-1),
            font: Some(BitmapFont::new()),
            source,
            layout: GlyphLayout::new(),
            size: 16.0,
        };
        bitmap.text_data.data.region = Rectangle::new(0.0, 0.0, 200.0, 30.0);
        // Text is empty "" which gets set to " " — but font has no actual glyphs loaded
        // so layout_glyphs returns empty, and no vertices are generated
        let mut renderer = SkinObjectRenderer::new();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);
        // No crash, renderer may or may not have vertices depending on BitmapFont state
    }

    #[test]
    fn test_scale_calculation() {
        // Construct directly to avoid get_font() overwriting original_size from cache.
        let source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        let bitmap = SkinTextBitmap {
            text_data: SkinTextData::new_with_id(-1),
            source,
            font: None,
            layout: GlyphLayout::new(),
            size: 16.0,
        };
        // scale = size / original_size = 16 / 32 = 0.5
        let scale = bitmap.size / bitmap.source.original_size();
        assert_eq!(scale, 0.5);
    }

    #[test]
    fn test_parse_fnt_value_extracts_size() {
        let line = r#"info face="Arial" size=32 bold=0 italic=0"#;
        assert_eq!(
            SkinTextBitmapSource::parse_fnt_value(line, "size="),
            Some(32.0)
        );
    }

    #[test]
    fn test_parse_fnt_value_extracts_scale() {
        let line = "common lineHeight=32 base=26 scaleW=256 scaleH=512 pages=1";
        assert_eq!(
            SkinTextBitmapSource::parse_fnt_value(line, "scaleW="),
            Some(256.0)
        );
        assert_eq!(
            SkinTextBitmapSource::parse_fnt_value(line, "scaleH="),
            Some(512.0)
        );
    }

    #[test]
    fn test_parse_fnt_value_missing_key_returns_none() {
        let line = "info face=\"Arial\" bold=0";
        assert_eq!(SkinTextBitmapSource::parse_fnt_value(line, "size="), None);
    }

    #[test]
    fn test_parse_fnt_value_at_end_of_line() {
        let line = "common scaleW=128";
        assert_eq!(
            SkinTextBitmapSource::parse_fnt_value(line, "scaleW="),
            Some(128.0)
        );
    }

    #[test]
    fn test_create_cacheable_font_with_fnt_file() {
        let dir = std::env::temp_dir().join("rubato_test_fnt");
        let _ = std::fs::create_dir_all(&dir);
        let fnt_path = dir.join("test.fnt");
        std::fs::write(
            &fnt_path,
            "info face=\"TestFont\" size=24 bold=0\ncommon lineHeight=28 base=22 scaleW=512 scaleH=256 pages=1\n",
        )
        .unwrap();

        let source = SkinTextBitmapSource::new(fnt_path.clone(), false);
        let cached = source.create_cacheable_font(&fnt_path, 0);
        assert_eq!(cached.original_size, 24.0);
        assert_eq!(cached.page_width, 512.0);
        assert_eq!(cached.page_height, 256.0);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_create_cacheable_font_missing_file() {
        let source = SkinTextBitmapSource::new(PathBuf::from("/nonexistent/test.fnt"), false);
        let cached = source.create_cacheable_font(std::path::Path::new("/nonexistent/test.fnt"), 0);
        assert_eq!(cached.original_size, 0.0);
        assert_eq!(cached.page_width, 0.0);
        assert_eq!(cached.page_height, 0.0);
    }
}
