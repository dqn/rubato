// SkinTextImage.java -> skin_text_image.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use crate::stubs::{MainState, Texture, TextureRegion};
use crate::text::skin_text::SkinTextData;
use crate::types::skin_object::SkinObjectRenderer;

pub struct SkinTextImage {
    pub text_data: SkinTextData,
    source: SkinTextImageSource,
    texts: Vec<TextureRegion>,
    textwidth: f32,
}

impl SkinTextImage {
    pub fn new(source: SkinTextImageSource) -> Self {
        Self::new_with_id(source, -1)
    }

    pub fn new_with_id(source: SkinTextImageSource, id: i32) -> Self {
        Self {
            text_data: SkinTextData::new_with_id(id),
            source,
            texts: Vec::with_capacity(256),
            textwidth: 0.0,
        }
    }

    pub fn prepare_font(&mut self, text: &str) {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        let mut i = 0;
        while i < bytes.len() {
            let mut code: i32 = 0;
            code |= (bytes[i] as i32) & 0xff;
            i += 1;
            if i < bytes.len() {
                code |= ((bytes[i] as i32) & 0xff) << 8;
                i += 1;
            }
            if (0xdc00..0xff00).contains(&code) && i < bytes.len() {
                code |= ((bytes[i] as i32) & 0xff) << 16;
                i += 1;
                if i < bytes.len() {
                    code |= ((bytes[i] as i32) & 0xff) << 24;
                    i += 1;
                }
            }
            self.source.get_image(code);
        }
    }

    pub fn prepare_text(&mut self, text: &str) {
        let bytes: Vec<u8> = text.encode_utf16().flat_map(|c| c.to_le_bytes()).collect();
        self.textwidth = 0.0;
        self.texts.clear();
        let mut i = 0;
        while i < bytes.len() {
            let mut code: i32 = 0;
            code |= (bytes[i] as i32) & 0xff;
            i += 1;
            if i < bytes.len() {
                code |= ((bytes[i] as i32) & 0xff) << 8;
                i += 1;
            }
            if (0xdc00..0xff00).contains(&code) && i < bytes.len() {
                code |= ((bytes[i] as i32) & 0xff) << 16;
                i += 1;
                if i < bytes.len() {
                    code |= ((bytes[i] as i32) & 0xff) << 24;
                    i += 1;
                }
            }
            if let Some(ch) = self.source.get_image(code) {
                self.textwidth += ch.region_width as f32;
                self.texts.push(ch);
            }
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

    pub fn draw_with_offset(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        offset_x: f32,
        offset_y: f32,
    ) {
        let region = self.text_data.data.region.clone();
        let color = self.text_data.data.color;
        let source_size = self.source.size() as f32;
        if source_size == 0.0 {
            return;
        }
        let width = self.textwidth * region.height / source_size
            + self.source.margin() as f32 * self.texts.len() as f32;

        let scale = if region.width < width {
            region.width / width
        } else {
            1.0
        };
        let align = self.text_data.align();
        let x = if align == 2 {
            region.x - width * scale
        } else if align == 1 {
            region.x - width * scale / 2.0
        } else {
            region.x
        };
        let mut dx: f32 = 0.0;
        for ch in &self.texts {
            let tw = ch.region_width as f32 * scale * region.height / source_size;
            self.text_data.data.draw_image_at_with_color(
                sprite,
                ch,
                x + dx + offset_x,
                region.y + offset_y,
                tw,
                region.height,
                &color,
                0,
            );
            dx += tw + self.source.margin() as f32 * scale;
        }
    }

    pub fn dispose(&mut self) {
        self.source.dispose();
    }
}

impl crate::skin_text::SkinText for SkinTextImage {
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

pub struct SkinTextImageSource {
    pub size: i32,
    pub margin: i32,
    elements: HashMap<i32, SkinTextImageSourceElement>,
    _usecim: bool,
    regions: HashMap<i32, SkinTextImageSourceRegion>,
}

impl SkinTextImageSource {
    pub fn new(usecim: bool) -> Self {
        Self {
            size: 0,
            margin: 0,
            elements: HashMap::with_capacity(400),
            _usecim: usecim,
            regions: HashMap::with_capacity(10000),
        }
    }

    pub fn margin(&self) -> i32 {
        self.margin
    }

    pub fn size(&self) -> i32 {
        self.size
    }

    pub fn get_image(&mut self, index: i32) -> Option<TextureRegion> {
        let region = self.regions.get_mut(&index)?;
        if region.image.is_some() {
            return region.image.clone();
        }
        let element = self.elements.get_mut(&region.id)?;
        element.texture.as_ref()?;
        let tex = element.texture.as_ref().expect("texture is Some");
        region.image = Some(TextureRegion::from_texture_region(
            tex.clone(),
            region.x,
            region.y,
            region.w,
            region.h,
        ));
        region.image.clone()
    }

    pub fn set_image(&mut self, index: i32, id: i32, x: i32, y: i32, w: i32, h: i32) {
        self.regions
            .insert(index, SkinTextImageSourceRegion::new(id, x, y, w, h));
    }

    pub fn path(&self, index: i32) -> Option<&str> {
        self.elements.get(&index).map(|e| e.path.as_str())
    }

    pub fn set_path(&mut self, index: i32, p: String) {
        let element = SkinTextImageSourceElement {
            path: p,
            texture: None,
        };
        self.elements.insert(index, element);
    }

    pub fn dispose(&mut self) {
        for tr in self.elements.values_mut() {
            if let Some(ref mut texture) = tr.texture {
                texture.dispose();
            }
        }
    }
}

struct SkinTextImageSourceElement {
    path: String,
    texture: Option<Texture>,
}

struct SkinTextImageSourceRegion {
    id: i32,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    image: Option<TextureRegion>,
}

impl SkinTextImageSourceRegion {
    fn new(id: i32, x: i32, y: i32, w: i32, h: i32) -> Self {
        Self {
            id,
            x,
            y,
            w,
            h,
            image: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_object::SkinObjectRenderer;
    use crate::stubs::{Color, Rectangle, Texture, TextureRegion};

    /// Helper: create a SkinTextImageSource with pre-populated glyph images.
    /// Inserts ASCII glyphs (char codes) mapping to TextureRegions with known widths.
    fn make_source_with_glyphs(
        glyph_widths: &[(char, i32)],
        size: i32,
        margin: i32,
    ) -> SkinTextImageSource {
        let mut source = SkinTextImageSource::new(false);
        source.size = size;
        source.margin = margin;

        let tex = Texture {
            width: 512,
            height: 512,
            disposed: false,
            ..Default::default()
        };

        // For each glyph, pre-insert it into regions with an image already set
        for &(ch, w) in glyph_widths {
            let code = ch as i32;
            let region = TextureRegion {
                region_width: w,
                region_height: size,
                u: 0.0,
                v: 0.0,
                u2: w as f32 / 512.0,
                v2: size as f32 / 512.0,
                texture: Some(tex.clone()),
                ..TextureRegion::default()
            };
            // Pre-insert with image already populated
            source.regions.insert(
                code,
                SkinTextImageSourceRegion {
                    id: 0,
                    x: 0,
                    y: 0,
                    w,
                    h: size,
                    image: Some(region),
                },
            );
        }

        source
    }

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    fn setup_data(data: &mut crate::skin_object::SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            0,
            x,
            y,
            w,
            h,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
    }

    #[test]
    fn test_skin_text_image_draw_basic_glyph_layout() {
        // "AB" with glyph widths A=20, B=30, size=40, margin=0
        let source = make_source_with_glyphs(&[('A', 20), ('B', 30)], 40, 0);
        let mut sti = SkinTextImage::new(source);
        setup_data(&mut sti.text_data.data, 100.0, 200.0, 500.0, 40.0);

        sti.set_text("AB".to_string());
        assert_eq!(sti.textwidth, 50.0); // 20 + 30

        // Manually set draw state
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(100.0, 200.0, 500.0, 40.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        // 2 glyphs = 2 quads = 12 vertices
        assert_eq!(renderer.sprite.vertices().len(), 12);

        let verts = renderer.sprite.vertices();
        // width = textwidth * region.height / source_size + margin * texts.size
        // width = 50 * 40 / 40 + 0 * 2 = 50
        // scale = min(1.0, 500/50) = 1.0
        // x = region.x = 100 (align=0, left)
        // Glyph A: tw = 20 * 1.0 * 40 / 40 = 20, at x=100
        assert!((verts[0].position[0] - 100.01).abs() < 0.02);
        // Glyph B: tw = 30 * 1.0 * 40 / 40 = 30, at x=100+20+0=120
        assert!((verts[6].position[0] - 120.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_alignment_right() {
        let source = make_source_with_glyphs(&[('X', 10)], 20, 0);
        let mut sti = SkinTextImage::new(source);
        sti.text_data.align = 2; // right
        setup_data(&mut sti.text_data.data, 100.0, 0.0, 200.0, 20.0);

        sti.set_text("X".to_string());
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(100.0, 0.0, 200.0, 20.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        assert_eq!(renderer.sprite.vertices().len(), 6);

        // width = 10 * 20 / 20 + 0 * 1 = 10
        // scale = 1.0 (200 > 10)
        // align=2: x = region.x - width*scale = 100 - 10 = 90
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 90.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_alignment_center() {
        let source = make_source_with_glyphs(&[('Y', 10)], 20, 0);
        let mut sti = SkinTextImage::new(source);
        sti.text_data.align = 1; // center
        setup_data(&mut sti.text_data.data, 100.0, 0.0, 200.0, 20.0);

        sti.set_text("Y".to_string());
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(100.0, 0.0, 200.0, 20.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        assert_eq!(renderer.sprite.vertices().len(), 6);

        // width = 10 * 20 / 20 + 0 * 1 = 10
        // scale = 1.0
        // align=1: x = region.x - width*scale/2 = 100 - 5 = 95
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 95.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_scaling_when_exceeds_width() {
        // Create text wider than region to trigger scaling
        // "ABCDE" with each glyph w=40, size=20, margin=0
        let source = make_source_with_glyphs(
            &[('A', 40), ('B', 40), ('C', 40), ('D', 40), ('E', 40)],
            20,
            0,
        );
        let mut sti = SkinTextImage::new(source);
        setup_data(&mut sti.text_data.data, 0.0, 0.0, 100.0, 20.0);

        sti.set_text("ABCDE".to_string());
        // textwidth = 200 (5 glyphs * 40)
        assert_eq!(sti.textwidth, 200.0);

        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(0.0, 0.0, 100.0, 20.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        // 5 glyphs = 30 vertices
        assert_eq!(renderer.sprite.vertices().len(), 30);

        // width = 200 * 20 / 20 + 0 * 5 = 200
        // scale = 100 / 200 = 0.5
        // Each glyph tw = 40 * 0.5 * 20 / 20 = 20
        // Total rendered width = 5 * 20 = 100 (fits in region)
        let verts = renderer.sprite.vertices();
        // Glyph A at x=0
        assert!((verts[0].position[0] - 0.01).abs() < 0.02);
        // Glyph B at x = 0 + 20 = 20
        assert!((verts[6].position[0] - 20.01).abs() < 0.02);
        // Glyph C at x = 0 + 40 = 40
        assert!((verts[12].position[0] - 40.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_with_margin() {
        // "AB" with glyph widths A=20, B=30, size=40, margin=5
        let source = make_source_with_glyphs(&[('A', 20), ('B', 30)], 40, 5);
        let mut sti = SkinTextImage::new(source);
        setup_data(&mut sti.text_data.data, 10.0, 20.0, 500.0, 40.0);

        sti.set_text("AB".to_string());
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(10.0, 20.0, 500.0, 40.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        assert_eq!(renderer.sprite.vertices().len(), 12);

        // width = 50 * 40/40 + 5 * 2 = 50 + 10 = 60
        // scale = 1.0 (500 > 60)
        // Glyph A: tw = 20 * 1.0 * 40/40 = 20, at x=10
        let verts = renderer.sprite.vertices();
        assert!((verts[0].position[0] - 10.01).abs() < 0.02);
        // Glyph B: at x = 10 + 20 + 5*1.0 = 35
        assert!((verts[6].position[0] - 35.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_with_offset() {
        let source = make_source_with_glyphs(&[('Z', 16)], 32, 0);
        let mut sti = SkinTextImage::new(source);
        setup_data(&mut sti.text_data.data, 50.0, 60.0, 200.0, 32.0);

        sti.set_text("Z".to_string());
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(50.0, 60.0, 200.0, 32.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 7.0, 11.0);

        assert_eq!(renderer.sprite.vertices().len(), 6);
        // x = region.x + dx + offset_x = 50 + 0 + 7 = 57
        // y = region.y + offset_y = 60 + 11 = 71
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 57.01).abs() < 0.02);
        assert!((v0.position[1] - 71.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_text_image_draw_zero_source_size_returns_early() {
        // source size=0 should return early without drawing
        let source = make_source_with_glyphs(&[('A', 20)], 0, 0);
        let mut sti = SkinTextImage::new(source);
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(0.0, 0.0, 100.0, 32.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        // No vertices should be generated
        assert!(renderer.sprite.vertices().is_empty());
    }

    #[test]
    fn test_skin_text_image_draw_height_scaling() {
        // region.height != source.size => glyphs scale proportionally
        // glyph A w=20, size=40, region.height=80 => height doubles
        let source = make_source_with_glyphs(&[('A', 20)], 40, 0);
        let mut sti = SkinTextImage::new(source);
        setup_data(&mut sti.text_data.data, 0.0, 0.0, 500.0, 80.0);

        sti.set_text("A".to_string());
        sti.text_data.data.draw = true;
        sti.text_data.data.region = Rectangle::new(0.0, 0.0, 500.0, 80.0);
        sti.text_data.data.color = Color::new(1.0, 1.0, 1.0, 1.0);

        let mut renderer = SkinObjectRenderer::new();
        sti.draw_with_offset(&mut renderer, 0.0, 0.0);

        assert_eq!(renderer.sprite.vertices().len(), 6);
        let verts = renderer.sprite.vertices();
        // tw = glyph_width * scale * region.height / source_size
        // tw = 20 * 1.0 * 80 / 40 = 40
        // Quad width: v1.x - v0.x = 40
        let glyph_width = verts[1].position[0] - verts[0].position[0];
        assert!((glyph_width - 40.0).abs() < 0.02);
        // Quad height: v2.y - v0.y = 80 (region.height)
        let glyph_height = verts[2].position[1] - verts[0].position[1];
        assert!((glyph_height - 80.0).abs() < 0.02);
    }
}
