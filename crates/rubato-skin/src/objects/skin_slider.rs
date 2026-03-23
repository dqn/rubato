// SkinSlider.java -> skin_slider.rs
// Mechanical line-by-line translation.

use crate::property::float_property::{FloatProperty, FloatPropertyEnum};
use crate::property::float_property_factory;
use crate::property::float_writer::FloatWriter;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{MainState, TextureRegion};
use crate::sources::skin_source::SkinSource;
use crate::sources::skin_source_image::SkinSourceImage;
use crate::types::skin_object::{RateProperty, SkinObjectData, SkinObjectRenderer};

/// Parameters for constructing a SkinSlider with integer timer and min/max rate.
pub struct SliderIntTimerMinmaxParams {
    pub image: Vec<TextureRegion>,
    pub timer: i32,
    pub cycle: i32,
    pub angle: i32,
    pub range: i32,
    pub type_id: i32,
    pub min: i32,
    pub max: i32,
}

/// Parameters for constructing a SkinSlider with timer property and min/max rate.
pub struct SliderTimerMinmaxParams {
    pub image: Vec<TextureRegion>,
    pub timer: TimerPropertyEnum,
    pub cycle: i32,
    pub angle: i32,
    pub range: i32,
    pub type_id: i32,
    pub min: i32,
    pub max: i32,
}

pub struct SkinSlider {
    pub data: SkinObjectData,
    source: Box<dyn SkinSource>,
    direction: i32,
    range: i32,
    ref_prop: Option<FloatPropertyEnum>,
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
        ref_prop: FloatPropertyEnum,
    ) -> Self {
        Self::new_with_int_timer_ref_writer(image, timer, cycle, angle, range, ref_prop, None)
    }

    pub fn new_with_int_timer_ref_writer(
        image: Vec<TextureRegion>,
        timer: i32,
        cycle: i32,
        angle: i32,
        range: i32,
        ref_prop: FloatPropertyEnum,
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

    pub fn new_with_int_timer_minmax(params: SliderIntTimerMinmaxParams) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                params.image,
                params.timer,
                params.cycle,
            )),
            direction: params.angle,
            range: params.range,
            ref_prop: Some(FloatPropertyEnum::Rate(RateProperty::new(
                params.type_id,
                params.min,
                params.max,
            ))),
            writer: None,
            current_image: None,
            current_value: 0.0,
        }
    }

    pub fn new_with_timer(
        image: Vec<TextureRegion>,
        timer: TimerPropertyEnum,
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
        timer: TimerPropertyEnum,
        cycle: i32,
        angle: i32,
        range: i32,
        ref_prop: FloatPropertyEnum,
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

    pub fn new_with_timer_minmax(params: SliderTimerMinmaxParams) -> Self {
        Self {
            data: SkinObjectData::new(),
            source: Box::new(SkinSourceImage::new_with_timer_from_vec(
                params.image,
                Some(params.timer),
                params.cycle,
            )),
            direction: params.angle,
            range: params.range,
            ref_prop: Some(FloatPropertyEnum::Rate(RateProperty::new(
                params.type_id,
                params.min,
                params.max,
            ))),
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
        if !self.data.draw {
            return;
        }
        if self.range <= 0 {
            self.data.draw = false;
            return;
        }
        self.current_image = self.source.get_image(time, state);
        if self.current_image.is_none() {
            self.data.draw = false;
            return;
        }
        self.current_value = if let Some(ref r) = self.ref_prop {
            r.get(state).clamp(0.0, 1.0)
        } else {
            0.0
        };
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref current_image) = self.current_image {
            let region = self.data.region;
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
        if self.range <= 0 {
            return false;
        }
        if let Some(ref writer) = self.writer {
            let region = &self.data.region;
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

    pub fn ref_prop(&self) -> Option<&FloatPropertyEnum> {
        self.ref_prop.as_ref()
    }

    pub fn direction(&self) -> i32 {
        self.direction
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::TextureRegion;
    use crate::test_helpers::MockMainState;
    use crate::types::skin_object::{DestinationParams, SkinObjectRenderer};

    fn make_region() -> TextureRegion {
        TextureRegion {
            region_width: 24,
            region_height: 24,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..TextureRegion::default()
        }
    }

    fn setup_slider_data(data: &mut crate::types::skin_object::SkinObjectData) {
        data.set_destination_with_int_timer_ops(
            &DestinationParams {
                time: 0,
                x: 0.0,
                y: 0.0,
                w: 24.0,
                h: 200.0,
                acc: 0,
                a: 255,
                r: 255,
                g: 255,
                b: 255,
                blend: 0,
                filter: 0,
                angle: 0,
                center: 0,
                loop_val: 0,
            },
            0,
            &[0],
        );
    }

    #[test]
    fn test_slider_zero_range_not_drawn() {
        // Regression: prepare() lacked the range <= 0 guard that mouse_pressed()
        // had, allowing zero-range sliders to render.
        let mut slider = SkinSlider::new_with_int_timer(
            vec![make_region()],
            0,
            0,
            0, // direction
            0, // range = 0
            0, // type_id
            false,
        );
        setup_slider_data(&mut slider.data);

        let state = MockMainState::default();
        slider.prepare(0, &state);
        assert!(
            !slider.data.draw,
            "slider with zero range must not be drawn"
        );
    }

    #[test]
    fn test_slider_negative_range_not_drawn() {
        // Regression: negative range rendered inverted but was non-interactive.
        let mut slider = SkinSlider::new_with_int_timer(
            vec![make_region()],
            0,
            0,
            0,    // direction
            -100, // negative range
            0,    // type_id
            false,
        );
        setup_slider_data(&mut slider.data);

        let state = MockMainState::default();
        slider.prepare(0, &state);
        assert!(
            !slider.data.draw,
            "slider with negative range must not be drawn"
        );
    }

    #[test]
    fn test_slider_positive_range_draws() {
        // Positive range should still be drawn normally.
        let mut slider = SkinSlider::new_with_int_timer(
            vec![make_region()],
            0,
            0,
            0,   // direction
            100, // positive range
            0,   // type_id
            false,
        );
        setup_slider_data(&mut slider.data);

        let state = MockMainState::default();
        slider.prepare(0, &state);
        assert!(
            slider.data.draw,
            "slider with positive range should be drawn"
        );

        let mut renderer = SkinObjectRenderer::new();
        slider.draw(&mut renderer);
        assert_eq!(renderer.sprite.vertices().len(), 6); // 1 quad
    }
}
