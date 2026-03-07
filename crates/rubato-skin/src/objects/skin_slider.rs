// SkinSlider.java -> skin_slider.rs
// Mechanical line-by-line translation.

use crate::property::float_property::FloatProperty;
use crate::property::float_property_factory;
use crate::property::float_writer::FloatWriter;
use crate::property::timer_property::TimerProperty;
use crate::sources::skin_source::SkinSource;
use crate::sources::skin_source_image::SkinSourceImage;
use crate::stubs::{MainState, TextureRegion};
use crate::types::skin_object::{RateProperty, SkinObjectData, SkinObjectRenderer};

pub struct SkinSlider {
    pub data: SkinObjectData,
    source: Box<dyn SkinSource>,
    direction: i32,
    range: i32,
    ref_prop: Option<Box<dyn FloatProperty>>,
    writer: Option<Box<dyn FloatWriter>>,
    current_image: Option<TextureRegion>,
    current_value: f32,
}

impl SkinSlider {
    pub fn new_with_int_timer(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        angle: i32,
        range: i32,
        type_id: i32,
        changeable: bool,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                image, timer, cycle,
            )),
            direction: angle,
            range,
            ref_prop: float_property_factory::rate_property_by_id(type_id),
            writer: if changeable {
                float_property_factory::rate_writer_by_id(type_id)
            } else {
                None
            },
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_int_timer_ref(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        angle: i32,
        range: i32,
        ref_prop: Box<dyn FloatProperty>,
    ) -> Self {
        Self::new_with_int_timer_ref_writer(image, timer, cycle, angle, range, ref_prop, None)
    }

    pub fn new_with_int_timer_ref_writer(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        angle: i32,
        range: i32,
        ref_prop: Box<dyn FloatProperty>,
        writer: Option<Box<dyn FloatWriter>>,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                image, timer, cycle,
            )),
            direction: angle,
            range,
            ref_prop: Some(ref_prop),
            writer,
            current_image: None,
            current_value: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_int_timer_minmax(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        angle: i32,
        range: i32,
        type_id: i32,
        min: i32,
        max: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                image, timer, cycle,
            )),
            direction: angle,
            range,
            ref_prop: Some(Box::new(RateProperty::new(type_id, min, max))),
            writer: None,
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        angle: i32,
        range: i32,
        type_id: i32,
        changeable: bool,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            direction: angle,
            range,
            ref_prop: float_property_factory::rate_property_by_id(type_id),
            writer: if changeable {
                float_property_factory::rate_writer_by_id(type_id)
            } else {
                None
            },
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer_ref_writer(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        angle: i32,
        range: i32,
        ref_prop: Box<dyn FloatProperty>,
        writer: Option<Box<dyn FloatWriter>>,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            direction: angle,
            range,
            ref_prop: Some(ref_prop),
            writer,
            current_image: None,
            current_value: 0.0,
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub fn new_with_timer_minmax(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        angle: i32,
        range: i32,
        type_id: i32,
        min: i32,
        max: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )),
            direction: angle,
            range,
            ref_prop: Some(Box::new(RateProperty::new(type_id, min, max))),
            writer: None,
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
            let region = self.data.draw_state.region.clone();
            let range = self.range as f32;
            let cv = self.current_value;
            let x = region.x
                + (if self.direction == 1 {
                    cv * range
                } else if self.direction == 3 {
                    -cv * range
                } else {
                    0.0
                });
            let y = region.y
                + (if self.direction == 0 {
                    cv * range
                } else if self.direction == 2 {
                    -cv * range
                } else {
                    0.0
                });
            self.data
                .draw_image_at(sprite, current_image, x, y, region.width, region.height);
        }
    }

    pub fn mouse_pressed(
        &mut self,
        state: &mut dyn MainState,
        _button: i32,
        x: i32,
        y: i32,
    ) -> bool {
        if let Some(ref writer) = self.writer {
            let region = &self.data.draw_state.region;
            let range = self.range as f32;
            match self.direction {
                0 => {
                    if region.x <= x as f32
                        && region.x + region.width >= x as f32
                        && region.y <= y as f32
                        && region.y + range >= y as f32
                    {
                        let value = if (y as f32 - region.y).abs() < 1.0 {
                            0.0
                        } else if (y as f32 - (region.y + range)).abs() < 1.0 {
                            1.0
                        } else {
                            (y as f32 - region.y) / range
                        };
                        writer.set(state, value);
                        return true;
                    }
                }
                1 => {
                    if region.x <= x as f32
                        && region.x + range >= x as f32
                        && region.y <= y as f32
                        && region.y + region.height >= y as f32
                    {
                        let value = if (x as f32 - region.x).abs() < 1.0 {
                            0.0
                        } else if (x as f32 - (region.x + range)).abs() < 1.0 {
                            1.0
                        } else {
                            (x as f32 - region.x) / range
                        };
                        writer.set(state, value);
                        return true;
                    }
                }
                2 => {
                    if region.x <= x as f32
                        && region.x + region.width >= x as f32
                        && region.y - range <= y as f32
                        && region.y >= y as f32
                    {
                        let value = if (y as f32 - region.y).abs() < 1.0 {
                            0.0
                        } else if (y as f32 - (region.y - range)).abs() < 1.0 {
                            1.0
                        } else {
                            (region.y - y as f32) / range
                        };
                        writer.set(state, value);
                        return true;
                    }
                }
                3 => {
                    if region.x >= x as f32
                        && region.x - range <= x as f32
                        && region.y <= y as f32
                        && region.y + region.height >= y as f32
                    {
                        let value = if (x as f32 - region.x).abs() < 1.0 {
                            0.0
                        } else if (x as f32 - (region.x - range)).abs() < 1.0 {
                            1.0
                        } else {
                            (region.x - x as f32) / range
                        };
                        writer.set(state, value);
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn dispose(&mut self) {
        self.source.dispose();
        self.data.set_disposed();
    }

    pub fn range(&self) -> i32 {
        self.range
    }

    pub fn slider_angle(&self) -> i32 {
        self.direction
    }

    pub fn ref_prop(&self) -> Option<&dyn FloatProperty> {
        self.ref_prop.as_deref()
    }

    pub fn direction(&self) -> i32 {
        self.direction
    }
}
