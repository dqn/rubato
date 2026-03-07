// SkinText.java -> skin_text.rs
// Mechanical line-by-line translation.

use crate::property::string_property::StringProperty;
use crate::property::string_property_factory;
use crate::stubs::{Color, MainState};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

pub const ALIGN_LEFT: i32 = 0;
pub const ALIGN_CENTER: i32 = 1;
pub const ALIGN_RIGHT: i32 = 2;

pub const OVERFLOW_OVERFLOW: i32 = 0;
pub const OVERFLOW_SHRINK: i32 = 1;
pub const OVERFLOW_TRUNCATE: i32 = 2;

// LibGDX Align constants (left=1, center=8, right=16 in GDX)
pub static ALIGN: [i32; 3] = [1, 8, 16];

pub struct SkinTextData {
    pub data: SkinObjectData,
    pub align: i32,
    pub ref_prop: Option<Box<dyn StringProperty>>,
    pub text: String,
    pub constant_text: Option<String>,
    pub editable: bool,
    pub wrapping: bool,
    pub overflow: i32,
    pub outline_color: Option<Color>,
    pub outline_width: f32,
    pub shadow_color: Option<Color>,
    pub shadow_offset: (f32, f32),
    pub shadow_smoothness: f32,
    pub current_text: Option<String>,
}

impl SkinTextData {
    pub fn new_with_id(id: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            align: ALIGN_LEFT,
            ref_prop: string_property_factory::string_property_by_id(id),
            text: String::new(),
            constant_text: None,
            editable: false,
            wrapping: false,
            overflow: 0,
            outline_color: None,
            outline_width: 0.0,
            shadow_color: None,
            shadow_offset: (0.0, 0.0),
            shadow_smoothness: 0.0,
            current_text: None,
        }
    }

    pub fn new_with_property(property: Box<dyn StringProperty>) -> Self {
        Self {
            data: SkinObjectData::new(),
            align: ALIGN_LEFT,
            ref_prop: Some(property),
            text: String::new(),
            constant_text: None,
            editable: false,
            wrapping: false,
            overflow: 0,
            outline_color: None,
            outline_width: 0.0,
            shadow_color: None,
            shadow_offset: (0.0, 0.0),
            shadow_smoothness: 0.0,
            current_text: None,
        }
    }

    pub fn align(&self) -> i32 {
        self.align
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn overflow(&self) -> i32 {
        self.overflow
    }

    pub fn is_editable(&self) -> bool {
        self.editable
    }

    pub fn is_wrapping(&self) -> bool {
        self.wrapping
    }

    pub fn outline_color(&self) -> Option<&Color> {
        self.outline_color.as_ref()
    }

    pub fn set_outline_color(&mut self, color: Color) {
        self.outline_color = Some(color);
    }

    pub fn outline_width(&self) -> f32 {
        self.outline_width
    }

    pub fn shadow_color(&self) -> Option<&Color> {
        self.shadow_color.as_ref()
    }

    pub fn set_shadow_color(&mut self, color: Color) {
        self.shadow_color = Some(color);
    }

    pub fn shadow_offset(&self) -> (f32, f32) {
        self.shadow_offset
    }

    pub fn set_shadow_offset(&mut self, x: f32, y: f32) {
        self.shadow_offset = (x, y);
    }

    pub fn shadow_smoothness(&self) -> f32 {
        self.shadow_smoothness
    }

    pub fn set_constant_text(&mut self, constant_text: String) {
        self.constant_text = Some(constant_text);
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
        self.current_text = if let Some(ref r) = self.ref_prop {
            Some(r.get(state))
        } else {
            self.constant_text.clone()
        };
        if self.current_text.is_none() || self.current_text.as_ref().is_none_or(|t| t.is_empty()) {
            self.data.draw = false;
        }
    }

    pub fn should_update_text(&self) -> bool {
        if let Some(ref current) = self.current_text {
            *current != self.text
        } else {
            false
        }
    }

    pub fn current_text(&self) -> Option<&str> {
        self.current_text.as_deref()
    }

    pub fn set_text(&mut self, text: String) {
        let text = if text.is_empty() {
            " ".to_string()
        } else {
            text
        };
        self.text = text;
    }
}

pub trait SkinText: Send {
    fn get_text_data(&self) -> &SkinTextData;
    fn get_text_data_mut(&mut self) -> &mut SkinTextData;
    fn prepare_font(&mut self, text: &str);
    fn prepare_text(&mut self, text: &str);
    fn draw_with_offset(&mut self, sprite: &mut SkinObjectRenderer, offset_x: f32, offset_y: f32);
    fn dispose(&mut self);
}
