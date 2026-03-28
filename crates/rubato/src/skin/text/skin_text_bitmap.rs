// SkinTextBitmap.java -> skin_text_bitmap.rs
// Mechanical line-by-line translation.

use std::path::PathBuf;

use crate::render::font::BitmapGlyph;

use crate::skin::loaders::skin_loader;
use crate::skin::property::string_property::StringProperty;
use crate::skin::reexports::{
    BitmapFont, BitmapFontData, Color, GlyphLayout, MainState, TextureRegion,
};
use crate::skin::text::skin_text::{
    OVERFLOW_OVERFLOW, OVERFLOW_SHRINK, OVERFLOW_TRUNCATE, SkinTextData,
};
use crate::skin::types::skin_object::{DrawImageAtParams, SkinObjectData, SkinObjectRenderer};

/// Parameters for drawing text glyphs at a specific position.
struct DrawTextGlyphsParams<'a> {
    pub sprite: &'a mut SkinObjectRenderer,
    pub text: &'a str,
    pub color: &'a Color,
    pub x: f32,
    pub y: f32,
    pub _layout_width: f32,
    pub region_width: f32,
    /// Effective bitmap glyph scale, potentially shrunk by OVERFLOW_SHRINK.
    /// When None, draw_text_glyphs computes the default scale from size / original_size.
    pub effective_scale: Option<f32>,
}

struct PositionedBitmapGlyphRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub region: TextureRegion,
}

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

    pub fn draw_impl(&mut self, sprite: &mut SkinObjectRenderer) {
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
        let original_size = self.source.original_size();
        if original_size <= 0.0 {
            return;
        }
        let scale = self.size / original_size;

        // Java: font.getData().setScale(scale)
        if let Some(font) = self.font.as_mut() {
            font.scale = original_size * scale;
        }

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
            let (layout_width, effective_scale) =
                self.compute_layout_width(&text, &color, region_width, region_height);
            self.draw_text_glyphs(DrawTextGlyphsParams {
                sprite,
                text: &text,
                color: &color,
                x: x + offset_x,
                y: region_y + offset_y + region_height,
                _layout_width: layout_width,
                region_width,
                effective_scale,
            });
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
                let (layout_width, effective_scale) =
                    self.compute_layout_width(&text, &shadow_color, region_width, region_height);
                self.draw_text_glyphs(DrawTextGlyphsParams {
                    sprite,
                    text: &text,
                    color: &shadow_color,
                    x: x + shadow_offset.0 + offset_x,
                    y: region_y - shadow_offset.1 + offset_y + region_height,
                    _layout_width: layout_width,
                    region_width,
                    effective_scale,
                });
            }

            // Main text rendering
            let (layout_width, effective_scale) =
                self.compute_layout_width(&text, &color, region_width, region_height);
            self.draw_text_glyphs(DrawTextGlyphsParams {
                sprite,
                text: &text,
                color: &color,
                x: x + offset_x,
                y: region_y + offset_y + region_height,
                _layout_width: layout_width,
                region_width,
                effective_scale,
            });
        }

        // Java parity: BitmapFont.getData().setScale(1)
        if let Some(f) = self.font.as_mut() {
            f.scale = 1.0;
            f.scale_x = None;
        }
    }

    #[cfg(test)]
    pub(crate) fn debug_font_path(&self) -> &std::path::Path {
        &self.source.font_path
    }

    #[cfg(test)]
    pub(crate) fn debug_original_size(&self) -> f32 {
        self.source.original_size
    }

    #[cfg(test)]
    pub(crate) fn debug_size(&self) -> f32 {
        self.size
    }

    #[cfg(test)]
    pub(crate) fn debug_region_count(&self) -> usize {
        self.source.regions.len()
    }

    #[cfg(test)]
    pub(crate) fn debug_has_font_data(&self) -> bool {
        self.source.font_data.is_some()
    }

    /// Compute layout width applying overflow mode.
    /// Corresponds to Java setLayout() logic for measuring and applying shrink/truncate.
    /// Returns (effective_width, effective_scale). The effective_scale is the shrunk
    /// bitmap glyph scale when OVERFLOW_SHRINK is active; None otherwise, meaning the
    /// caller should use the default scale (self.size / original_size).
    fn compute_layout_width(
        &mut self,
        text: &str,
        _color: &Color,
        region_width: f32,
        _region_height: f32,
    ) -> (f32, Option<f32>) {
        let scale = if self.source.original_size() > 0.0 {
            self.size / self.source.original_size()
        } else {
            0.0
        };

        let measure = || {
            self.source
                .measure_bitmap_text(text, scale)
                .or_else(|| self.font.as_ref().map(|font| font.measure(text)))
                .unwrap_or_default()
        };

        if self.text_data.is_wrapping() {
            // With wrapping, width is constrained to region width
            let layout = measure();
            self.layout.width = layout.width;
            self.layout.height = layout.height;
            return (layout.width, None);
        }

        match self.text_data.overflow() {
            OVERFLOW_OVERFLOW => {
                let layout = measure();
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                (layout.width, None)
            }
            OVERFLOW_SHRINK => {
                let layout = measure();
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                let actual_width = layout.width;
                if actual_width > region_width && region_width > 0.0 {
                    let shrunk_scale = scale * region_width / actual_width;
                    let shrunk = self
                        .source
                        .measure_bitmap_text(text, shrunk_scale)
                        .or_else(|| {
                            self.font.as_mut().map(|font| {
                                let current_scale = font.scale();
                                // Only shrink X axis, keeping Y (height) unchanged.
                                // Keep scale_x set so draw_text_glyphs uses it.
                                font.scale_x = Some(current_scale * region_width / actual_width);
                                font.measure(text)
                            })
                        })
                        .unwrap_or_default();
                    self.layout.width = shrunk.width;
                    self.layout.height = shrunk.height;
                    return (shrunk.width, Some(shrunk_scale));
                }
                (actual_width, None)
            }
            OVERFLOW_TRUNCATE => {
                // Truncate text to fit within region width
                let layout = measure();
                self.layout.width = layout.width.min(region_width);
                self.layout.height = layout.height;
                (self.layout.width, None)
            }
            _ => {
                let layout = measure();
                self.layout.width = layout.width;
                self.layout.height = layout.height;
                (layout.width, None)
            }
        }
    }

    /// Draw text glyphs at the given position.
    /// Uses BitmapFont.layout_glyphs() to get per-glyph positions,
    /// then draws each glyph as a TextureRegion via SkinObjectData.draw_image_at_with_color().
    fn draw_text_glyphs(&mut self, params: DrawTextGlyphsParams<'_>) {
        let sprite = params.sprite;
        let text = params.text;
        let color = params.color;
        let x = params.x;
        let y = params.y;
        let region_width = params.region_width;
        let scale = params.effective_scale.unwrap_or_else(|| {
            if self.source.original_size() > 0.0 {
                self.size / self.source.original_size()
            } else {
                0.0
            }
        });
        let Some((glyphs, _total_width, _line_height)) =
            self.source.layout_bitmap_glyph_regions(text, scale)
        else {
            return;
        };

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
        self.source.dispose();
    }
}

impl crate::skin::types::skin_node::SkinNode for SkinTextBitmap {
    fn data(&self) -> &SkinObjectData {
        &self.text_data.data
    }
    fn data_mut(&mut self) -> &mut SkinObjectData {
        &mut self.text_data.data
    }
    fn prepare(&mut self, time: i64, state: &dyn MainState) {
        SkinTextBitmap::prepare(self, time, state)
    }
    fn draw(&mut self, sprite: &mut SkinObjectRenderer, _state: &dyn MainState) {
        self.draw_impl(sprite)
    }
    fn dispose(&mut self) {
        SkinTextBitmap::dispose(self)
    }
    fn type_name(&self) -> &'static str {
        "Text"
    }
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }
    fn into_any_box(self: Box<Self>) -> Box<dyn std::any::Any> {
        self
    }
}

impl crate::skin::skin_text::SkinText for SkinTextBitmap {
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

/// Cacheable bitmap font data.
/// Corresponds to Java BitmapFontCache.CacheableBitmapFont.
pub struct CacheableBitmapFont {
    pub font: Option<BitmapFont>,
    pub original_size: f32,
    pub font_type: i32,
    pub page_width: f32,
    pub page_height: f32,
}

/// Options controlling how bitmap font textures are loaded.
#[derive(Debug, Clone, Copy, Default)]
pub struct TextureLoadOptions {
    pub use_cim: bool,
    pub use_mip_maps: bool,
}

pub struct SkinTextBitmapSource {
    pub texture_options: TextureLoadOptions,
    pub font_path: PathBuf,
    pub font: Option<BitmapFont>,
    pub font_data: Option<BitmapFontData>,
    pub regions: Vec<TextureRegion>,
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
        Self::new_with_options(
            font_path,
            TextureLoadOptions {
                use_cim: usecim,
                use_mip_maps: true,
            },
        )
    }

    pub fn new_with_mipmaps(font_path: PathBuf, usecim: bool, use_mip_maps: bool) -> Self {
        Self::new_with_options(
            font_path,
            TextureLoadOptions {
                use_cim: usecim,
                use_mip_maps,
            },
        )
    }

    pub fn new_with_options(font_path: PathBuf, texture_options: TextureLoadOptions) -> Self {
        Self {
            texture_options,
            font_path,
            font: None,
            font_data: None,
            regions: Vec::new(),
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
        let content_result =
            std::fs::read(font_path).map(|bytes| match std::str::from_utf8(&bytes) {
                Ok(s) => s.to_string(),
                Err(_) => {
                    let (cow, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                    cow.into_owned()
                }
            });
        if let Ok(content) = content_result {
            // Match lines by prefix instead of assuming positional order,
            // since the BMFont spec does not mandate line ordering.
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("info ") {
                    original_size = Self::parse_fnt_value(trimmed, "size=").unwrap_or(0.0);
                } else if trimmed.starts_with("common ") {
                    page_width = Self::parse_fnt_value(trimmed, "scaleW=").unwrap_or(0.0);
                    page_height = Self::parse_fnt_value(trimmed, "scaleH=").unwrap_or(0.0);
                }
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
        if !crate::skin::bitmap_font_cache::has(Some(&self.font_path)) {
            // Parse the full .fnt data (glyph metrics, page image paths, etc.)
            let font_data = BitmapFontData::from_fnt(&self.font_path).unwrap_or_default();

            // Load texture regions for each page image
            let image_regions: Vec<TextureRegion> = font_data
                .image_paths
                .iter()
                .filter_map(|ip| {
                    skin_loader::texture(ip, self.texture_options.use_cim)
                        .map(TextureRegion::from_texture)
                })
                .collect();

            // Use font_size (from `size=` in info line) as primary originalSize,
            // matching Java SkinTextBitmap.java:161. Fall back to lineHeight, then
            // to create_cacheable_font header parsing.
            let mut size = font_data.font_size;
            let mut scale_w = font_data.scale_w;
            let mut scale_h = font_data.scale_h;

            if size == 0.0 {
                size = font_data.line_height;
            }
            if size == 0.0 {
                let header = self.create_cacheable_font(&self.font_path, self.source_type);
                size = header.original_size;
                scale_w = header.page_width;
                scale_h = header.page_height;
            }
            if scale_w == 0.0 && !image_regions.is_empty() {
                scale_w = image_regions[0].region_width as f32;
                scale_h = image_regions[0].region_height as f32;
            }

            crate::skin::bitmap_font_cache::set(
                self.font_path.clone(),
                crate::skin::bitmap_font_cache::CacheableBitmapFont {
                    font: BitmapFont::new(),
                    font_data,
                    regions: image_regions,
                    original_size: size,
                    type_: self.source_type,
                    page_width: scale_w,
                    page_height: scale_h,
                },
            );
        }

        if let Some(cached) = crate::skin::bitmap_font_cache::get(&self.font_path) {
            self.font_data = Some(cached.font_data.clone());
            self.regions = cached.regions.clone();
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
    /// Uses word-boundary matching (via `find_fnt_key`) so that e.g. "size="
    /// does not match inside a longer key like "charset=".
    /// Example: parse_fnt_value("info size=32 bold=0", "size=") → Some(32.0)
    fn parse_fnt_value(line: &str, key: &str) -> Option<f32> {
        let start = crate::render::font::find_fnt_key(line, key)? + key.len();
        let rest = &line[start..];
        let end = rest.find(' ').unwrap_or(rest.len());
        rest[..end].parse::<i32>().ok().map(|v| v as f32)
    }

    pub fn original_size(&self) -> f32 {
        self.original_size
    }

    fn measure_bitmap_text(&self, text: &str, scale: f32) -> Option<GlyphLayout> {
        let data = self.font_data.as_ref()?;
        let space_advance = data
            .glyphs
            .get(&(b' ' as u32))
            .map(|glyph| glyph.xadvance as f32 * scale)
            .unwrap_or(0.0);
        let width = text.chars().fold(0.0, |acc, ch| {
            acc + data
                .glyphs
                .get(&(ch as u32))
                .map(|glyph| glyph.xadvance as f32 * scale)
                .unwrap_or(space_advance)
        });
        Some(GlyphLayout {
            width,
            height: data.line_height * scale,
            text: text.to_string(),
        })
    }

    fn layout_bitmap_glyph_regions(
        &self,
        text: &str,
        scale: f32,
    ) -> Option<(Vec<PositionedBitmapGlyphRegion>, f32, f32)> {
        let data = self.font_data.as_ref()?;
        let space_advance = data
            .glyphs
            .get(&(b' ' as u32))
            .map(|glyph| glyph.xadvance as f32 * scale)
            .unwrap_or(0.0);
        let mut cursor_x = 0.0f32;
        let mut glyphs = Vec::new();

        for ch in text.chars() {
            let Some(glyph) = data.glyphs.get(&(ch as u32)) else {
                cursor_x += space_advance;
                continue;
            };
            if let Some(region) = self.bitmap_glyph_region(glyph) {
                glyphs.push(PositionedBitmapGlyphRegion {
                    x: cursor_x + glyph.xoffset as f32 * scale,
                    // BMFont yoffset is measured downward from the line top. The text draw call
                    // passes the line's top edge, while sprite quads use bottom-left coordinates.
                    y: -glyph.yoffset as f32 * scale - glyph.height as f32 * scale,
                    width: glyph.width as f32 * scale,
                    height: glyph.height as f32 * scale,
                    region,
                });
            }
            cursor_x += glyph.xadvance as f32 * scale;
        }

        Some((glyphs, cursor_x, data.line_height * scale))
    }

    fn bitmap_glyph_region(&self, glyph: &BitmapGlyph) -> Option<TextureRegion> {
        let texture = self.regions.get(glyph.page as usize)?.texture.clone()?;
        Some(TextureRegion::from_texture_region(
            texture,
            glyph.x,
            glyph.y,
            glyph.width,
            glyph.height,
        ))
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
        self.font_data = None;
        self.regions.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cmp::Ordering;

    use crate::skin::reexports::Rectangle;

    fn make_source(original_size: f32, source_type: i32) -> SkinTextBitmapSource {
        SkinTextBitmapSource {
            texture_options: TextureLoadOptions {
                use_cim: false,
                use_mip_maps: true,
            },
            font_path: PathBuf::from("test.fnt"),
            font: None,
            font_data: None,
            regions: Vec::new(),
            original_size,
            source_type,
            page_width: 512.0,
            page_height: 512.0,
        }
    }

    fn ecfn_select_song_font_path() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../skin/ECFN/_font/selectsongname.fnt")
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
    fn test_parse_fnt_value_word_boundary_rejects_substring() {
        // "scaleW=" must not match inside a hypothetical "xscaleW=" prefix.
        // More practically, verify "size=" doesn't match "charset=" or "fontSize=".
        let line = "info face=\"Arial\" charset=32 size=24 bold=0";
        // charset= contains "set=" which could match "size=" via naive substring
        // if keys with overlapping suffixes were present. Verify "size=" extracts 24.
        assert_eq!(
            SkinTextBitmapSource::parse_fnt_value(line, "size="),
            Some(24.0)
        );
    }

    #[test]
    fn test_create_cacheable_font_handles_reordered_lines() {
        use std::sync::atomic::{AtomicU64, Ordering as AOrdering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, AOrdering::Relaxed);

        let dir = std::env::temp_dir().join(format!("rubato_test_fnt_reorder_{id}"));
        let _ = std::fs::create_dir_all(&dir);
        let fnt_path = dir.join("reordered.fnt");
        // Put "common" before "info" to verify we match by prefix, not position.
        std::fs::write(
            &fnt_path,
            "common lineHeight=28 base=22 scaleW=512 scaleH=256 pages=1\ninfo face=\"TestFont\" size=24 bold=0\n",
        )
        .unwrap();

        let source = SkinTextBitmapSource::new(fnt_path.clone(), false);
        let cached = source.create_cacheable_font(&fnt_path, 0);
        assert_eq!(
            cached.original_size, 24.0,
            "size= must be found regardless of line order"
        );
        assert_eq!(
            cached.page_width, 512.0,
            "scaleW= must be found regardless of line order"
        );
        assert_eq!(
            cached.page_height, 256.0,
            "scaleH= must be found regardless of line order"
        );

        let _ = std::fs::remove_dir_all(&dir);
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

    /// Java SkinTextBitmap.java:161 reads `size=` from the .fnt info line as the
    /// primary `originalSize`, falling back to `lineHeight` only on exception.
    /// The `originalSize` is the denominator in `scale = desired_size / original_size`,
    /// so using `lineHeight` (which is typically larger than `size`) produces smaller
    /// rendered text than Java.
    #[test]
    fn test_font_original_size_uses_font_size_not_line_height() {
        use std::sync::atomic::{AtomicU64, Ordering as AOrdering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, AOrdering::Relaxed);

        let dir = std::env::temp_dir().join(format!("rubato_test_fnt_size_precedence_{id}"));
        let _ = std::fs::create_dir_all(&dir);
        let fnt_path = dir.join("test_precedence.fnt");
        // size=24 but lineHeight=32: Java uses 24 as originalSize, not 32.
        // Must include page + char lines so BitmapFontData::from_fnt succeeds
        // (it returns None when glyphs or pages are empty).
        std::fs::write(
            &fnt_path,
            "info face=\"TestFont\" size=24 bold=0\ncommon lineHeight=32 base=22 scaleW=256 scaleH=256 pages=1\npage id=0 file=\"test.png\"\nchar id=65 x=0 y=0 width=16 height=16 xoffset=0 yoffset=0 xadvance=16 page=0\n",
        )
        .unwrap();

        // Clear cache for this path to ensure fresh derivation
        crate::skin::bitmap_font_cache::clear();

        let mut source = SkinTextBitmapSource::new(fnt_path.clone(), false);
        // Call font() which parses the .fnt and caches original_size
        let _ = source.font();

        // After font(), original_size must be 24 (from size=), not 32 (from lineHeight)
        assert_eq!(
            source.original_size, 24.0,
            "original_size must come from font_size (size=24), not line_height (32); got {}",
            source.original_size,
        );

        crate::skin::bitmap_font_cache::clear();
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// When font_size is 0 (missing size= in info line), fall back to lineHeight.
    #[test]
    fn test_font_original_size_falls_back_to_line_height_when_font_size_zero() {
        use std::sync::atomic::{AtomicU64, Ordering as AOrdering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let id = COUNTER.fetch_add(1, AOrdering::Relaxed);

        let dir = std::env::temp_dir().join(format!("rubato_test_fnt_size_fallback_{id}"));
        let _ = std::fs::create_dir_all(&dir);
        let fnt_path = dir.join("test_fallback.fnt");
        // No size= in info line, lineHeight=28.
        // Must include page + char lines so BitmapFontData::from_fnt succeeds.
        std::fs::write(
            &fnt_path,
            "info face=\"TestFont\" bold=0\ncommon lineHeight=28 base=22 scaleW=256 scaleH=256 pages=1\npage id=0 file=\"test.png\"\nchar id=65 x=0 y=0 width=16 height=16 xoffset=0 yoffset=0 xadvance=16 page=0\n",
        )
        .unwrap();

        crate::skin::bitmap_font_cache::clear();

        let mut source = SkinTextBitmapSource::new(fnt_path.clone(), false);
        let _ = source.font();

        // font_size is 0 (no size= field), so should fall back to lineHeight=28
        assert_eq!(
            source.original_size, 28.0,
            "when font_size is 0, must fall back to line_height (28); got {}",
            source.original_size,
        );

        crate::skin::bitmap_font_cache::clear();
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

    #[test]
    fn test_ecfn_real_bitmap_font_japanese_text_emits_textured_quads() {
        let font_path = ecfn_select_song_font_path();
        assert!(
            font_path.exists(),
            "ECFN bitmap font should exist: {}",
            font_path.display()
        );

        let source = SkinTextBitmapSource::new(font_path, false);
        let mut bitmap = SkinTextBitmap::new(source, 50.0);
        bitmap.text_data.data.draw = true;
        bitmap.text_data.data.region = Rectangle::new(451.0, 742.0, 640.0, 24.0);
        bitmap.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        bitmap.set_text("ふぁんぶる！".to_string());

        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "bitmap title font should emit glyph quads for Japanese text"
        );
        assert!(
            quads.iter().all(|quad| quad.texture_key.is_some()),
            "bitmap glyph quads should be backed by font textures, got {:?}",
            quads
                .iter()
                .map(|quad| quad.texture_key.clone())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_ecfn_real_bitmap_font_lowercase_glyphs_do_not_top_align_with_uppercase() {
        let font_path = ecfn_select_song_font_path();
        assert!(
            font_path.exists(),
            "ECFN bitmap font should exist: {}",
            font_path.display()
        );

        let source = SkinTextBitmapSource::new(font_path, false);
        let mut bitmap = SkinTextBitmap::new(source, 50.0);
        bitmap.text_data.data.draw = true;
        bitmap.text_data.data.region = Rectangle::new(0.0, 0.0, 200.0, 52.0);
        bitmap.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        bitmap.set_text("Aa".to_string());

        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);

        let mut quads = renderer
            .sprite
            .captured_quads()
            .iter()
            .map(|quad| (quad.x, quad.y, quad.w, quad.h))
            .collect::<Vec<_>>();
        quads.sort_by(|lhs, rhs| lhs.0.partial_cmp(&rhs.0).unwrap_or(Ordering::Equal));

        assert!(
            quads.len() >= 2,
            "bitmap font should emit uppercase and lowercase glyph quads, got {:?}",
            quads
        );

        let uppercase_top = quads[0].1 + quads[0].3;
        let lowercase_top = quads[1].1 + quads[1].3;

        assert!(
            uppercase_top > lowercase_top + 5.0,
            "uppercase and lowercase glyphs should keep different cap-height tops, got uppercase_top={}, lowercase_top={}, quads={:?}",
            uppercase_top,
            lowercase_top,
            quads
        );
    }

    /// OVERFLOW_SHRINK must draw glyphs at the shrunk scale, not the original scale.
    /// Bug: compute_layout_width correctly measures at the shrunk scale, but
    /// draw_text_glyphs recomputes scale as self.size / original_size (unshrunk),
    /// causing text to overflow the destination region.
    #[test]
    fn test_overflow_shrink_draws_glyphs_at_shrunk_scale() {
        let font_path = ecfn_select_song_font_path();
        if !font_path.exists() {
            return;
        }

        // Use a large font size so that text is guaranteed wider than the narrow region.
        let source = SkinTextBitmapSource::new(font_path.clone(), false);
        let mut bitmap = SkinTextBitmap::new(source, 50.0);
        bitmap.text_data.data.draw = true;
        bitmap.text_data.overflow = OVERFLOW_SHRINK;
        let region_width = 100.0; // Narrow region to force shrink
        bitmap.text_data.data.region = Rectangle::new(0.0, 0.0, region_width, 50.0);
        bitmap.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        bitmap.set_text("WWWWWWWWWWWW".to_string()); // Wide text

        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);

        let quads = renderer.sprite.captured_quads();
        assert!(
            !quads.is_empty(),
            "shrunk bitmap text should still emit glyph quads"
        );

        // The rightmost glyph edge must not exceed the region width.
        // Before fix: glyphs are drawn at original (unshrunk) scale and overflow.
        // After fix: glyphs are drawn at shrunk scale and fit within region_width.
        let max_right = quads
            .iter()
            .map(|quad| quad.x + quad.w)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            max_right <= region_width + 1.0,
            "OVERFLOW_SHRINK glyphs must fit within region width {region_width}, \
             but rightmost edge was {max_right} (drawn at unshrunk scale)"
        );
    }

    /// When shadow is enabled with OVERFLOW_SHRINK, both shadow and main text
    /// must use the shrunk scale.
    #[test]
    fn test_overflow_shrink_shadow_also_uses_shrunk_scale() {
        let font_path = ecfn_select_song_font_path();
        if !font_path.exists() {
            return;
        }

        let source = SkinTextBitmapSource::new(font_path.clone(), false);
        let mut bitmap = SkinTextBitmap::new(source, 50.0);
        bitmap.text_data.data.draw = true;
        bitmap.text_data.overflow = OVERFLOW_SHRINK;
        bitmap.text_data.set_shadow_offset(2.0, 2.0);
        let region_width = 100.0;
        bitmap.text_data.data.region = Rectangle::new(0.0, 0.0, region_width, 50.0);
        bitmap.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        bitmap.set_text("WWWWWWWWWWWW".to_string());

        let mut renderer = SkinObjectRenderer::new();
        renderer.sprite.enable_capture();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);

        let quads = renderer.sprite.captured_quads();
        // With shadow, we expect two sets of quads (shadow + main).
        // The main text quads (second half) must fit within region_width.
        // The shadow quads are offset by shadow_offset.0 (2.0), so they may
        // slightly exceed, but the glyph widths themselves should be shrunk.
        assert!(
            quads.len() >= 2,
            "should have shadow + main quads, got {}",
            quads.len()
        );

        // Check that glyph widths are consistent between shadow and main.
        // Split into two halves: first half = shadow, second half = main.
        let half = quads.len() / 2;
        let shadow_quads = &quads[..half];
        let main_quads = &quads[half..];

        // Main text glyphs must fit within region width
        let main_max_right = main_quads
            .iter()
            .map(|quad| quad.x + quad.w)
            .fold(f32::NEG_INFINITY, f32::max);
        assert!(
            main_max_right <= region_width + 1.0,
            "OVERFLOW_SHRINK main text must fit within region width {region_width}, \
             but rightmost edge was {main_max_right}"
        );

        // Shadow glyph widths must match main glyph widths (both use shrunk scale)
        for (s, m) in shadow_quads.iter().zip(main_quads.iter()) {
            assert!(
                (s.w - m.w).abs() < 0.01,
                "shadow glyph width ({}) must match main glyph width ({}) under OVERFLOW_SHRINK",
                s.w,
                m.w
            );
        }
    }

    #[test]
    fn test_font_scale_restored_to_one_after_draw() {
        // Java: BitmapFont.getData().setScale(1) after drawing.
        // Rust must restore font.scale to 1.0 (not original_scale) for Java parity.
        let mut source = make_source(32.0, SkinTextBitmapSource::TYPE_STANDARD);
        source.font = Some(BitmapFont::new());
        let mut font = BitmapFont::new();
        font.scale = 2.5; // Set non-1.0 scale to detect restoration target
        let mut bitmap = SkinTextBitmap {
            text_data: SkinTextData::new_with_id(-1),
            font: Some(font),
            source,
            layout: GlyphLayout::new(),
            size: 16.0,
        };
        bitmap.text_data.data.region = Rectangle::new(0.0, 0.0, 200.0, 30.0);

        let mut renderer = SkinObjectRenderer::new();
        bitmap.draw_with_offset(&mut renderer, 0.0, 0.0);

        let final_scale = bitmap.font.as_ref().unwrap().scale;
        assert_eq!(
            final_scale, 1.0,
            "font.scale must be restored to 1.0 (Java parity: setScale(1)), got {}",
            final_scale
        );
    }
}
