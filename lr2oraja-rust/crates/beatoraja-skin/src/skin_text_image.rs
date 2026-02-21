// SkinTextImage.java -> skin_text_image.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;

use crate::skin_object::SkinObjectRenderer;
use crate::skin_text::SkinTextData;
use crate::stubs::{Color, MainState, Texture, TextureRegion};

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
                self.textwidth += ch.get_region_width() as f32;
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
            let current = self.text_data.get_current_text().unwrap_or("").to_string();
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
        let color = self.text_data.data.color.clone();
        let source_size = self.source.get_size() as f32;
        if source_size == 0.0 {
            return;
        }
        let width = self.textwidth * region.height / source_size
            + self.source.get_margin() as f32 * self.texts.len() as f32;

        let scale = if region.width < width {
            region.width / width
        } else {
            1.0
        };
        let align = self.text_data.get_align();
        let x = if align == 2 {
            region.x - width * scale
        } else if align == 1 {
            region.x - width * scale / 2.0
        } else {
            region.x
        };
        let mut dx: f32 = 0.0;
        for ch in &self.texts.clone() {
            let tw = ch.get_region_width() as f32 * scale * region.height / source_size;
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
            dx += tw + self.source.get_margin() as f32 * scale;
        }
    }

    pub fn dispose(&mut self) {
        self.source.dispose();
    }
}

pub struct SkinTextImageSource {
    size: i32,
    margin: i32,
    elements: HashMap<i32, SkinTextImageSourceElement>,
    usecim: bool,
    regions: HashMap<i32, SkinTextImageSourceRegion>,
}

impl SkinTextImageSource {
    pub fn new(usecim: bool) -> Self {
        Self {
            size: 0,
            margin: 0,
            elements: HashMap::with_capacity(400),
            usecim,
            regions: HashMap::with_capacity(10000),
        }
    }

    pub fn get_margin(&self) -> i32 {
        self.margin
    }

    pub fn set_margin(&mut self, margin: i32) {
        self.margin = margin;
    }

    pub fn get_size(&self) -> i32 {
        self.size
    }

    pub fn set_size(&mut self, size: i32) {
        self.size = size;
    }

    pub fn get_image(&mut self, index: i32) -> Option<TextureRegion> {
        let region = self.regions.get_mut(&index)?;
        if region.image.is_some() {
            return region.image.clone();
        }
        let element = self.elements.get_mut(&region.id)?;
        element.texture.as_ref()?;
        let tex = element.texture.as_ref().unwrap();
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

    pub fn get_path(&self, index: i32) -> Option<&str> {
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
