// Glyph atlas for caching rasterized glyphs as TextureRegions.
// Used by BitmapFont::draw() to render text via SpriteBatch.

use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::{Font, GlyphId, PxScale};

use crate::texture::{Texture, TextureRegion};

/// Cached glyph information within the atlas.
#[derive(Clone, Debug)]
pub struct CachedGlyph {
    /// X position in the atlas texture (pixels).
    pub atlas_x: u32,
    /// Y position in the atlas texture (pixels).
    pub atlas_y: u32,
    /// Width of the rasterized glyph (pixels).
    pub width: u32,
    /// Height of the rasterized glyph (pixels).
    pub height: u32,
    /// Horizontal bearing (offset from cursor to left edge of glyph).
    pub bearing_x: f32,
    /// Vertical bearing (offset from baseline to top edge of glyph).
    pub bearing_y: f32,
}

/// Key for glyph cache: (glyph_id, scale as integer bits).
type GlyphKey = (GlyphId, u32);

/// Initial atlas dimensions.
const ATLAS_WIDTH: u32 = 512;
const ATLAS_HEIGHT: u32 = 512;

/// Glyph atlas that rasterizes and caches glyphs in an RGBA texture.
/// Uses a simple row-packing strategy for atlas layout.
pub struct GlyphAtlas {
    atlas_width: u32,
    atlas_height: u32,
    pixels: Vec<u8>,
    /// Current cursor position for packing.
    cursor_x: u32,
    cursor_y: u32,
    /// Height of the tallest glyph in the current row.
    row_height: u32,
    /// Cached glyph data.
    cache: HashMap<GlyphKey, CachedGlyph>,
    /// Texture backing the atlas. Updated when new glyphs are rasterized.
    atlas_texture: Texture,
    /// Version counter for texture path uniqueness.
    version: u64,
}

impl GlyphAtlas {
    pub fn new() -> Self {
        let w = ATLAS_WIDTH;
        let h = ATLAS_HEIGHT;
        let pixels = vec![0u8; (w * h * 4) as usize];
        Self {
            atlas_width: w,
            atlas_height: h,
            pixels,
            cursor_x: 0,
            cursor_y: 0,
            row_height: 0,
            cache: HashMap::new(),
            atlas_texture: Texture::default(),
            version: 0,
        }
    }

    /// Get a cached glyph, or rasterize and cache it.
    /// Returns None if the glyph has no outline (e.g. space character).
    pub fn get_or_rasterize(
        &mut self,
        font: &ab_glyph::FontVec,
        glyph_id: GlyphId,
        scale: f32,
    ) -> Option<CachedGlyph> {
        let key = (glyph_id, scale.to_bits());
        if let Some(cached) = self.cache.get(&key) {
            return Some(cached.clone());
        }

        // Rasterize the glyph
        let glyph =
            glyph_id.with_scale_and_position(PxScale::from(scale), ab_glyph::point(0.0, 0.0));
        let outlined = font.outline_glyph(glyph)?;

        let bounds = outlined.px_bounds();
        let glyph_w = (bounds.max.x - bounds.min.x).ceil() as u32;
        let glyph_h = (bounds.max.y - bounds.min.y).ceil() as u32;

        if glyph_w == 0 || glyph_h == 0 {
            return None;
        }

        // Check if glyph fits in current row
        if self.cursor_x + glyph_w > self.atlas_width {
            // Move to next row
            self.cursor_y += self.row_height + 1;
            self.cursor_x = 0;
            self.row_height = 0;
        }

        // Check if atlas has room vertically
        if self.cursor_y + glyph_h > self.atlas_height {
            // Atlas full — grow it
            self.grow_atlas();
        }

        let atlas_x = self.cursor_x;
        let atlas_y = self.cursor_y;

        // Rasterize glyph pixels into atlas
        outlined.draw(|x, y, coverage| {
            let px = atlas_x + x;
            let py = atlas_y + y;
            if px < self.atlas_width && py < self.atlas_height {
                let idx = ((py * self.atlas_width + px) * 4) as usize;
                if idx + 3 < self.pixels.len() {
                    let alpha = (coverage * 255.0) as u8;
                    // White glyph with coverage-based alpha
                    self.pixels[idx] = 255;
                    self.pixels[idx + 1] = 255;
                    self.pixels[idx + 2] = 255;
                    self.pixels[idx + 3] = alpha;
                }
            }
        });

        // Update cursor
        self.cursor_x += glyph_w + 1;
        self.row_height = self.row_height.max(glyph_h);

        let cached = CachedGlyph {
            atlas_x,
            atlas_y,
            width: glyph_w,
            height: glyph_h,
            bearing_x: bounds.min.x,
            bearing_y: bounds.min.y,
        };
        self.cache.insert(key, cached.clone());

        // Mark atlas texture as dirty
        self.version += 1;
        self.update_texture();

        Some(cached)
    }

    /// Create a TextureRegion for a cached glyph.
    pub fn texture_region(&self, glyph: &CachedGlyph) -> TextureRegion {
        let u = glyph.atlas_x as f32 / self.atlas_width as f32;
        let v = glyph.atlas_y as f32 / self.atlas_height as f32;
        let u2 = (glyph.atlas_x + glyph.width) as f32 / self.atlas_width as f32;
        let v2 = (glyph.atlas_y + glyph.height) as f32 / self.atlas_height as f32;

        TextureRegion {
            u,
            v,
            u2,
            v2,
            region_x: glyph.atlas_x as i32,
            region_y: glyph.atlas_y as i32,
            region_width: glyph.width as i32,
            region_height: glyph.height as i32,
            texture: Some(self.atlas_texture.clone()),
        }
    }

    /// Update the atlas texture with current pixel data.
    fn update_texture(&mut self) {
        let path_str = format!("__glyph_atlas_v{}", self.version);
        self.atlas_texture = Texture {
            width: self.atlas_width as i32,
            height: self.atlas_height as i32,
            disposed: false,
            path: Some(Arc::from(path_str.as_str())),
            rgba_data: Some(Arc::new(self.pixels.clone())),
            ..Default::default()
        };
    }

    /// Double the atlas height to accommodate more glyphs.
    fn grow_atlas(&mut self) {
        let new_height = self.atlas_height * 2;
        let new_size = (self.atlas_width * new_height * 4) as usize;
        self.pixels.resize(new_size, 0);
        self.atlas_height = new_height;
    }
}

impl Default for GlyphAtlas {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_glyph_atlas_new() {
        let atlas = GlyphAtlas::new();
        assert_eq!(atlas.atlas_width, ATLAS_WIDTH);
        assert_eq!(atlas.atlas_height, ATLAS_HEIGHT);
        assert!(atlas.cache.is_empty());
    }

    #[test]
    fn test_glyph_atlas_grow() {
        let mut atlas = GlyphAtlas::new();
        let original_height = atlas.atlas_height;
        atlas.grow_atlas();
        assert_eq!(atlas.atlas_height, original_height * 2);
        assert_eq!(
            atlas.pixels.len(),
            (atlas.atlas_width * atlas.atlas_height * 4) as usize
        );
    }

    #[test]
    fn test_texture_region_uvs() {
        let atlas = GlyphAtlas::new();
        let glyph = CachedGlyph {
            atlas_x: 10,
            atlas_y: 20,
            width: 8,
            height: 12,
            bearing_x: 1.0,
            bearing_y: -10.0,
        };
        let region = atlas.texture_region(&glyph);
        let expected_u = 10.0 / ATLAS_WIDTH as f32;
        let expected_v = 20.0 / ATLAS_HEIGHT as f32;
        let expected_u2 = 18.0 / ATLAS_WIDTH as f32;
        let expected_v2 = 32.0 / ATLAS_HEIGHT as f32;
        assert!((region.u - expected_u).abs() < 1e-6);
        assert!((region.v - expected_v).abs() < 1e-6);
        assert!((region.u2 - expected_u2).abs() < 1e-6);
        assert!((region.v2 - expected_v2).abs() < 1e-6);
    }
}
