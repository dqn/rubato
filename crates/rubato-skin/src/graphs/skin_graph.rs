// SkinGraph.java -> skin_graph.rs
// Mechanical line-by-line translation.

use crate::property::float_property::FloatProperty;
use crate::property::float_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::sources::skin_source::SkinSource;
use crate::sources::skin_source_image::SkinSourceImage;
use crate::sources::skin_source_reference::SkinSourceReference;
use crate::stubs::{MainState, TextureRegion};
use crate::types::skin_object::{RateProperty, SkinObjectData, SkinObjectRenderer};

pub struct SkinGraph {
    pub data: SkinObjectData,
    source: Box<dyn SkinSource>,
    ref_prop: Option<Box<dyn FloatProperty>>,
    pub direction: i32,
    current: TextureRegion,
    current_image: Option<TextureRegion>,
    current_value: f32,
}

impl SkinGraph {
    pub fn new_with_image_id(imageid: i32, id: i32, direction: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceReference::new(imageid)),
            ref_prop: float_property_factory::rate_property_by_id(id),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_image_id_minmax(
        imageid: i32,
        id: i32,
        min: i32,
        max: i32,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceReference::new(imageid)),
            ref_prop: Some(Box::new(RateProperty::new(id, min, max))),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_int_timer(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        id: i32,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                image, timer, cycle,
            )),
            ref_prop: float_property_factory::rate_property_by_id(id),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_int_timer_minmax(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        id: i32,
        min: i32,
        max: i32,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                image, timer, cycle,
            )),
            ref_prop: Some(Box::new(RateProperty::new(id, min, max))),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        id: i32,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            ref_prop: float_property_factory::rate_property_by_id(id),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer_ref(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        ref_prop: Box<dyn FloatProperty>,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            ref_prop: Some(ref_prop),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer_minmax(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        id: i32,
        min: i32,
        max: i32,
        direction: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            ref_prop: Some(Box::new(RateProperty::new(id, min, max))),
            direction,
            current: TextureRegion::new(),
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn validate(&self) -> bool {
        if !self.source.validate() {
            return false;
        }
        self.data.validate()
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);
        if !self.data.draw_state.draw {
            return;
        }
        self.current_image = self.source.get_image(time, state);
        if self.current_image.is_none() {
            self.data.draw_state.draw = false;
            return;
        }
        self.current_value = if let Some(ref r) = self.ref_prop {
            r.get(state)
        } else {
            0.0
        };
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref current_image) = self.current_image.clone() {
            if self.direction == 1 {
                // Java: current.setRegion(currentImage, 0, h - h*value, w, h*value)
                self.current.set_region_from_parent(
                    current_image,
                    0,
                    current_image.region_height
                        - (current_image.region_height as f32 * self.current_value) as i32,
                    current_image.region_width,
                    (current_image.region_height as f32 * self.current_value) as i32,
                );
                let region = self.data.draw_state.region.clone();
                self.data.draw_image_at(
                    sprite,
                    &self.current,
                    region.x,
                    region.y,
                    region.width,
                    region.height * self.current_value,
                );
            } else {
                // Java: current.setRegion(currentImage, 0, 0, w*value, h)
                self.current.set_region_from_parent(
                    current_image,
                    0,
                    0,
                    (current_image.region_width as f32 * self.current_value) as i32,
                    current_image.region_height,
                );
                let region = self.data.draw_state.region.clone();
                self.data.draw_image_at(
                    sprite,
                    &self.current,
                    region.x,
                    region.y,
                    region.width * self.current_value,
                    region.height,
                );
            }
        }
    }

    pub fn ref_prop(&self) -> Option<&dyn FloatProperty> {
        self.ref_prop.as_deref()
    }

    pub fn direction(&self) -> i32 {
        self.direction
    }

    pub fn dispose(&mut self) {
        self.source.dispose();
        self.data.set_disposed();
    }
}
