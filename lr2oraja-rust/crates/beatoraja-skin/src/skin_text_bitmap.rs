// SkinTextBitmap.java -> skin_text_bitmap.rs
// Mechanical line-by-line translation.

use std::path::PathBuf;

use crate::property::string_property::StringProperty;
use crate::skin_object::SkinObjectRenderer;
use crate::skin_text::{
    ALIGN, OVERFLOW_OVERFLOW, OVERFLOW_SHRINK, OVERFLOW_TRUNCATE, SkinTextData,
};
use crate::stubs::{BitmapFont, Color, GlyphLayout, MainState, Rectangle, TextureRegion};

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
        source: SkinTextBitmapSource,
        size: f32,
        property: Option<Box<dyn StringProperty>>,
    ) -> Self {
        let text_data = if let Some(prop) = property {
            SkinTextData::new_with_property(prop)
        } else {
            SkinTextData::new_with_id(-1)
        };
        let font = source.get_font();
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
            let current = self.text_data.get_current_text().unwrap_or("").to_string();
            self.set_text(current);
        }
        self.draw_with_offset(sprite, 0.0, 0.0);
    }

    pub fn draw_with_offset(
        &mut self,
        _sprite: &mut SkinObjectRenderer,
        _offset_x: f32,
        _offset_y: f32,
    ) {
        if self.font.is_none() {
            return;
        }
        let _scale = self.size / self.source.get_original_size();
        let _align_val = self.text_data.get_align();
        // font.getData().setScale(scale)
        // ... complex rendering with distance field / shadow support
        log::warn!("not yet implemented: SkinTextBitmap.draw requires LibGDX font rendering");
    }

    fn _set_layout(&mut self, _c: &Color, _r: &Rectangle) {
        let _align_val = ALIGN[self.text_data.get_align() as usize];
        if self.text_data.is_wrapping() {
            // layout.setText(font, getText(), c, r.getWidth(), ALIGN[getAlign()], true)
        } else {
            match self.text_data.get_overflow() {
                OVERFLOW_OVERFLOW => {}
                OVERFLOW_SHRINK => {}
                OVERFLOW_TRUNCATE => {}
                _ => {}
            }
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
    /// Corresponds to Java SkinTextBitmapSource.createCacheableFont.
    /// In Java, this loads a .fnt file, parses image paths, creates textures,
    /// and extracts size/pageWidth/pageHeight from the font file header.
    /// Stubbed: requires LibGDX BitmapFont infrastructure.
    pub fn create_cacheable_font(
        &self,
        _font_path: &std::path::Path,
        _font_type: i32,
    ) -> CacheableBitmapFont {
        log::warn!(
            "not yet implemented: SkinTextBitmapSource.createCacheableFont requires BitmapFont loading"
        );
        CacheableBitmapFont {
            font: None,
            original_size: 0.0,
            font_type: _font_type,
            page_width: 0.0,
            page_height: 0.0,
        }
    }

    pub fn get_font(&self) -> Option<BitmapFont> {
        // In Java, this loads from BitmapFontCache or creates via createCacheableFont.
        // Stubbed for Phase 7+ rendering dependency.
        log::warn!(
            "not yet implemented: SkinTextBitmapSource.getFont requires LibGDX BitmapFont loading"
        );
        None
    }

    pub fn get_original_size(&self) -> f32 {
        self.original_size
    }

    pub fn get_type(&self) -> i32 {
        self.source_type
    }

    pub fn set_type(&mut self, source_type: i32) {
        self.source_type = source_type;
    }

    pub fn get_page_width(&self) -> f32 {
        self.page_width
    }

    pub fn get_page_height(&self) -> f32 {
        self.page_height
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;
    }
}
