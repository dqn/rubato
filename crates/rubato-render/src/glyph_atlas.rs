// Glyph atlas for caching rasterized glyphs as TextureRegions.
// Used by BitmapFont::draw() to render text via SpriteBatch.

use std::collections::HashMap;
use std::sync::Arc;

use ab_glyph::{Font, GlyphId, PxScale};

use crate::texture::{Texture, TextureRegion};

/// Cached glyph information within the atlas.
#[derive(Clone, Copy, Debug)]
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
/// Maximum atlas height. Capped at the minimum guaranteed GPU texture
/// dimension (8192) to prevent exceeding device limits.
const MAX_ATLAS_HEIGHT: u32 = 8192;

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
    /// Stable texture key with `__pixmap_` prefix. The `__pixmap_` prefix causes
    /// `GpuTextureManager::ensure_uploaded()` to always re-upload the texture data,
    /// while the stable key avoids allocating a new GPU texture each frame.
    stable_key: Arc<str>,
    /// Whether pixel data has been modified since last texture upload.
    dirty: bool,
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
            stable_key: Arc::from("__pixmap_glyph_atlas"),
            dirty: false,
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
            return Some(*cached);
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

        // Skip glyphs wider than the atlas (atlas only grows vertically).
        // Attempting to place such a glyph would waste rows without ever
        // fitting, and the draw callback would clip the excess anyway.
        if glyph_w > self.atlas_width {
            log::warn!(
                "Glyph {:?} at scale {} is {}px wide, exceeding atlas width {}; skipping",
                glyph_id,
                scale,
                glyph_w,
                self.atlas_width,
            );
            return None;
        }

        // Check if glyph fits in current row
        if self.cursor_x + glyph_w > self.atlas_width {
            // Move to next row
            self.cursor_y += self.row_height + 1;
            self.cursor_x = 0;
            self.row_height = 0;
        }

        // Check if atlas has room vertically; a very large glyph may need multiple doublings
        while self.cursor_y + glyph_h > self.atlas_height {
            if self.atlas_height >= MAX_ATLAS_HEIGHT {
                log::warn!(
                    "Glyph atlas reached maximum height {}; skipping glyph {:?} at scale {}",
                    MAX_ATLAS_HEIGHT,
                    glyph_id,
                    scale,
                );
                return None;
            }
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
        self.cache.insert(key, cached);

        // Mark atlas as needing texture upload (deferred to flush)
        self.dirty = true;

        Some(cached)
    }

    /// Flush the texture if any glyphs were rasterized since the last flush.
    /// This batches all pending glyph rasterizations into a single texture upload,
    /// avoiding per-glyph `pixels.clone()` overhead.
    pub fn flush_texture_if_dirty(&mut self) {
        if self.dirty {
            self.update_texture();
            self.dirty = false;
        }
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
    /// Uses a stable `__pixmap_`-prefixed key so `GpuTextureManager::ensure_uploaded()`
    /// always re-uploads the data without allocating a new GPU texture each frame.
    fn update_texture(&mut self) {
        self.atlas_texture = Texture {
            width: self.atlas_width as i32,
            height: self.atlas_height as i32,
            disposed: false,
            path: Some(Arc::clone(&self.stable_key)),
            rgba_data: Some(Arc::new(self.pixels.clone())),
            ..Default::default()
        };
    }

    /// Double the atlas height to accommodate more glyphs, capped at MAX_ATLAS_HEIGHT.
    fn grow_atlas(&mut self) {
        let new_height = (self.atlas_height * 2).min(MAX_ATLAS_HEIGHT);
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
    fn test_flush_texture_if_dirty_only_updates_when_dirty() {
        let mut atlas = GlyphAtlas::new();
        assert!(!atlas.dirty);

        // Flush when not dirty: texture path stays None (default)
        atlas.flush_texture_if_dirty();
        assert!(atlas.atlas_texture.path.is_none());

        // Simulate dirty state
        atlas.dirty = true;
        atlas.flush_texture_if_dirty();
        assert!(!atlas.dirty);
        // After flush, texture uses the stable __pixmap_ key
        assert_eq!(
            atlas.atlas_texture.path.as_deref(),
            Some("__pixmap_glyph_atlas")
        );

        // Flush again when not dirty: texture unchanged
        let prev_path = atlas.atlas_texture.path.clone();
        atlas.flush_texture_if_dirty();
        assert_eq!(atlas.atlas_texture.path, prev_path);
    }

    /// Load the test font (NotoSansJP from assets/).
    fn test_font() -> ab_glyph::FontVec {
        let font_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/fonts/NotoSansJP-Regular.ttf");
        let font_data = std::fs::read(&font_path).unwrap_or_else(|e| {
            panic!("Failed to read test font at {}: {}", font_path.display(), e)
        });
        ab_glyph::FontVec::try_from_vec(font_data).expect("Failed to parse test font")
    }

    #[test]
    fn test_multiple_rasterizations_single_flush() {
        let font = test_font();
        let mut atlas = GlyphAtlas::new();

        // Rasterize multiple glyphs -- texture should NOT be updated yet
        for ch in ['A', 'B', 'C', 'D', 'E'] {
            use ab_glyph::Font;
            let glyph_id = font.glyph_id(ch);
            atlas.get_or_rasterize(&font, glyph_id, 24.0);
        }
        assert!(atlas.dirty);
        assert!(atlas.atlas_texture.path.is_none()); // Not flushed yet

        // Single flush updates texture with stable key
        atlas.flush_texture_if_dirty();
        assert!(!atlas.dirty);
        assert_eq!(
            atlas.atlas_texture.path.as_deref(),
            Some("__pixmap_glyph_atlas")
        );

        // Rasterize more glyphs
        for ch in ['F', 'G'] {
            use ab_glyph::Font;
            let glyph_id = font.glyph_id(ch);
            atlas.get_or_rasterize(&font, glyph_id, 24.0);
        }
        atlas.flush_texture_if_dirty();
        // Key remains stable (same key, new data via __pixmap_ re-upload contract)
        assert_eq!(
            atlas.atlas_texture.path.as_deref(),
            Some("__pixmap_glyph_atlas")
        );

        // Cached glyphs don't set dirty
        for ch in ['A', 'B', 'C'] {
            use ab_glyph::Font;
            let glyph_id = font.glyph_id(ch);
            atlas.get_or_rasterize(&font, glyph_id, 24.0);
        }
        assert!(!atlas.dirty);
    }

    #[test]
    fn test_oversized_glyph_returns_none() {
        use ab_glyph::Font;
        let font = test_font();
        let mut atlas = GlyphAtlas::new();

        // Use a scale large enough that the glyph exceeds ATLAS_WIDTH (512px).
        // A typical Latin glyph at scale 1000+ easily exceeds 512px width.
        let glyph_id = font.glyph_id('W');
        let result = atlas.get_or_rasterize(&font, glyph_id, 2000.0);
        assert!(
            result.is_none(),
            "Glyph wider than atlas width should return None"
        );
        // Atlas state should not be corrupted
        assert!(!atlas.dirty, "Oversized glyph should not mark atlas dirty");
        assert!(
            atlas.cache.is_empty(),
            "Oversized glyph should not be cached"
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

    /// Regression: grow_atlas must cap at MAX_ATLAS_HEIGHT to avoid exceeding GPU limits.
    #[test]
    fn test_grow_atlas_caps_at_max_height() {
        let mut atlas = GlyphAtlas::new();
        // Grow until capped
        while atlas.atlas_height < MAX_ATLAS_HEIGHT {
            atlas.grow_atlas();
        }
        assert_eq!(atlas.atlas_height, MAX_ATLAS_HEIGHT);

        // Further growth should not exceed the cap
        atlas.grow_atlas();
        assert_eq!(
            atlas.atlas_height, MAX_ATLAS_HEIGHT,
            "atlas height must not exceed MAX_ATLAS_HEIGHT"
        );
    }
}
