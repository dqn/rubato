// SkinTextFont.java -> skin_text_font.rs
// Mechanical line-by-line translation.

use crate::property::string_property::StringProperty;
use crate::skin_object::SkinObjectRenderer;
use crate::skin_text::{
    ALIGN, OVERFLOW_OVERFLOW, OVERFLOW_SHRINK, OVERFLOW_TRUNCATE, SkinTextData,
};
use crate::stubs::{
    BitmapFont, Color, FreeTypeFontGenerator, FreeTypeFontParameter, GlyphLayout, MainState,
};

pub struct SkinTextFont {
    pub text_data: SkinTextData,
    font: Option<BitmapFont>,
    layout: Option<GlyphLayout>,
    generator: Option<FreeTypeFontGenerator>,
    parameter: FreeTypeFontParameter,
    prepared_fonts: Option<String>,
}

impl SkinTextFont {
    pub fn new(fontpath: &str, _cycle: i32, size: i32, shadow: i32) -> Self {
        Self::new_with_property(fontpath, _cycle, size, shadow, None)
    }

    pub fn new_with_property(
        fontpath: &str,
        _cycle: i32,
        size: i32,
        shadow: i32,
        property: Option<Box<dyn StringProperty>>,
    ) -> Self {
        let text_data = if let Some(prop) = property {
            SkinTextData::new_with_property(prop)
        } else {
            SkinTextData::new_with_id(-1)
        };
        let parameter = FreeTypeFontParameter {
            characters: String::new(),
            size,
            ..Default::default()
        };
        let generator = Some(FreeTypeFontGenerator::new(fontpath));
        let mut result = Self {
            text_data,
            font: None,
            layout: None,
            generator,
            parameter,
            prepared_fonts: None,
        };
        result
            .text_data
            .set_shadow_offset(shadow as f32, shadow as f32);
        result
    }

    pub fn validate(&self) -> bool {
        if self.generator.is_none() {
            return false;
        }
        self.text_data.data.validate()
    }

    pub fn prepare_font(&mut self, text: &str) {
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;

        self.parameter.characters = text.to_string();
        if let Some(ref generator) = self.generator {
            self.font = Some(generator.generate_font(&self.parameter));
            self.layout = Some(GlyphLayout::new());
            self.prepared_fonts = Some(text.to_string());
        }
    }

    pub fn prepare_text(&mut self, text: &str) {
        if self.prepared_fonts.is_some() {
            return;
        }
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;

        self.parameter.characters = text.to_string();
        if let Some(ref generator) = self.generator {
            self.font = Some(generator.generate_font(&self.parameter));
            self.layout = Some(GlyphLayout::new());
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
        _sprite: &mut SkinObjectRenderer,
        _offset_x: f32,
        _offset_y: f32,
    ) {
        if self.font.is_some() {
            // font.getData().setScale(region.height / parameter.size)
            // sprite.setType(...)
            // layout.setText(...)
            // sprite.draw(font, layout, x, y)
            log::warn!("not yet implemented: SkinTextFont.draw requires LibGDX font rendering");
        }
    }

    fn _set_layout(&mut self, _c: &Color, _region: &crate::stubs::Rectangle) {
        if self.font.is_none() || self.layout.is_none() {
            return;
        }
        let _align_val = ALIGN[self.text_data.get_align() as usize];
        if self.text_data.is_wrapping() {
            // layout.setText(font, getText(), c, r.getWidth(), ALIGN[getAlign()], true)
        } else {
            match self.text_data.get_overflow() {
                OVERFLOW_OVERFLOW => {
                    // layout.setText(font, getText(), c, r.getWidth(), ALIGN[getAlign()], false)
                }
                OVERFLOW_SHRINK => {
                    // layout.setText(...)
                    // if actualWidth > r.getWidth() => scale and re-layout
                }
                OVERFLOW_TRUNCATE => {
                    // layout.setText(font, getText(), 0, getText().length(), c, r.getWidth(), ...)
                }
                _ => {}
            }
        }
    }

    pub fn dispose(&mut self) {
        if let Some(ref mut generator) = self.generator {
            generator.dispose();
        }
        self.generator = None;
        if let Some(ref mut font) = self.font {
            font.dispose();
        }
        self.font = None;
    }
}
