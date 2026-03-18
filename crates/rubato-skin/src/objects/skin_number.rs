// SkinNumber.java -> skin_number.rs
// Mechanical line-by-line translation.

use crate::property::integer_property::IntegerProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{MainState, SkinOffset, TextureRegion};
use crate::sources::skin_source_image_set::SkinSourceImageSet;
use crate::sources::skin_source_set::SkinSourceSet;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Display configuration for integer number rendering.
pub struct NumberDisplayConfig {
    pub keta: i32,
    pub zeropadding: i32,
    pub space: i32,
    pub align: i32,
}

pub struct SkinNumber {
    pub data: SkinObjectData,
    image: Box<dyn SkinSourceSet>,
    mimage: Option<Box<dyn SkinSourceSet>>,
    ref_prop: Option<Box<dyn IntegerProperty>>,
    pub keta: i32,
    pub zeropadding: i32,
    pub space: i32,
    pub align: i32,
    value: i32,
    shiftbase: i32,
    offsets: Option<Vec<SkinOffset>>,
    length: f32,
    current_images: Vec<Option<TextureRegion>>,
    image_set: Option<Vec<TextureRegion>>,
    shift: f32,
}

impl SkinNumber {
    /// Create an empty SkinNumber with no sources (used in tests).
    pub fn new_empty() -> Self {
        use crate::skin_source_image_set::SkinSourceImageSet;
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                vec![],
                0,
                0,
            )),
            mimage: None,
            ref_prop: None,
            keta: 0,
            current_images: Vec::new(),
            zeropadding: 0,
            space: 0,
            align: 0,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn new_with_int_timer(
        image: Vec<Vec<TextureRegion>>,
        mimage: Option<Vec<Vec<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        config: NumberDisplayConfig,
        id: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                image, timer, cycle,
            )),
            mimage: mimage.map(|m| -> Box<dyn SkinSourceSet> {
                Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                    m, timer, cycle,
                ))
            }),
            ref_prop: integer_property_factory::integer_property_by_id(id),
            current_images: vec![None; config.keta.max(0) as usize],
            keta: config.keta,
            zeropadding: config.zeropadding,
            space: config.space,
            align: config.align,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn new_with_int_timer_ref(
        image: Vec<Vec<TextureRegion>>,
        mimage: Option<Vec<Vec<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        config: NumberDisplayConfig,
        ref_prop: Box<dyn IntegerProperty>,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                image, timer, cycle,
            )),
            mimage: mimage.map(|m| -> Box<dyn SkinSourceSet> {
                Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                    m, timer, cycle,
                ))
            }),
            ref_prop: Some(ref_prop),
            current_images: vec![None; config.keta.max(0) as usize],
            keta: config.keta,
            zeropadding: config.zeropadding,
            space: config.space,
            align: config.align,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn new_with_timer(
        image: Vec<Vec<TextureRegion>>,
        mimage: Option<Vec<Vec<TextureRegion>>>,
        timer: TimerPropertyEnum,
        cycle: i32,
        config: NumberDisplayConfig,
        id: i32,
    ) -> Self {
        let mimage_source = mimage.map(|m| -> Box<dyn SkinSourceSet> {
            Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                m,
                Some(timer.clone()),
                cycle,
            ))
        });
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                image,
                Some(timer),
                cycle,
            )),
            mimage: mimage_source,
            ref_prop: integer_property_factory::integer_property_by_id(id),
            current_images: vec![None; config.keta.max(0) as usize],
            keta: config.keta,
            zeropadding: config.zeropadding,
            space: config.space,
            align: config.align,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn new_with_timer_ref(
        image: Vec<Vec<TextureRegion>>,
        mimage: Option<Vec<Vec<TextureRegion>>>,
        timer: TimerPropertyEnum,
        cycle: i32,
        config: NumberDisplayConfig,
        ref_prop: Box<dyn IntegerProperty>,
    ) -> Self {
        let mimage_source = mimage.map(|m| -> Box<dyn SkinSourceSet> {
            Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                m,
                Some(timer.clone()),
                cycle,
            ))
        });
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                image,
                Some(timer),
                cycle,
            )),
            mimage: mimage_source,
            ref_prop: Some(ref_prop),
            current_images: vec![None; config.keta.max(0) as usize],
            keta: config.keta,
            zeropadding: config.zeropadding,
            space: config.space,
            align: config.align,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn keta(&self) -> i32 {
        self.keta
    }

    pub fn set_offsets(&mut self, offsets: Vec<SkinOffset>) {
        self.offsets = Some(offsets);
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_offset(time, state, 0.0, 0.0);
    }

    pub fn prepare_with_offset(
        &mut self,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        let value = if let Some(ref r) = self.ref_prop {
            r.get(state)
        } else {
            i32::MIN
        };
        self.prepare_with_value(time, state, value, offset_x, offset_y);
    }

    pub fn prepare_with_value(
        &mut self,
        time: i64,
        state: &dyn MainState,
        value: i32,
        offset_x: f32,
        offset_y: f32,
    ) {
        if value == i32::MIN || value == i32::MAX {
            self.length = 0.0;
            self.data.draw = false;
            return;
        }
        let uses_signed_sheet = value < 0 && self.mimage.is_some();
        let images: Option<Vec<TextureRegion>> = if value >= 0 || self.mimage.is_none() {
            self.image.get_images(time, state)
        } else if let Some(ref mimage) = self.mimage {
            mimage.get_images(time, state)
        } else {
            None
        };
        if images.is_none() {
            self.length = 0.0;
            self.data.draw = false;
            return;
        }
        self.data
            .prepare_with_offset(time, state, offset_x, offset_y);
        if !self.data.draw {
            self.length = 0.0;
            return;
        }
        let image = images.expect("images");
        let minimum_image_count = if uses_signed_sheet { 12 } else { 10 };
        if image.len() < minimum_image_count {
            self.length = 0.0;
            self.data.draw = false;
            return;
        }
        let blank_index = if image.len() > 10 { 10 } else { 0 };

        if self.value != value || self.image_set.as_ref() != Some(&image) {
            self.value = value;
            self.image_set = Some(image.clone());
            self.shiftbase = 0;
            debug_assert!(
                value != i32::MIN,
                "i32::MIN sentinel should be caught earlier"
            );
            let mut abs_value = value.unsigned_abs() as i32;
            for j in (0..self.current_images.len()).rev() {
                if self.mimage.is_some() && self.zeropadding > 0 {
                    if j == 0 {
                        self.current_images[j] = Some(image[11].clone());
                    } else if abs_value > 0 || j == self.current_images.len() - 1 {
                        self.current_images[j] = Some(image[(abs_value % 10) as usize].clone());
                    } else {
                        self.current_images[j] = Some(
                            image[if self.zeropadding == 2 {
                                blank_index
                            } else {
                                0
                            }]
                            .clone(),
                        );
                    }
                } else if abs_value > 0 || j == self.current_images.len() - 1 {
                    self.current_images[j] = Some(image[(abs_value % 10) as usize].clone());
                } else {
                    self.current_images[j] = if self.zeropadding == 2 {
                        Some(image[blank_index].clone())
                    } else if self.zeropadding == 1 {
                        Some(image[0].clone())
                    } else if self.mimage.is_some() {
                        let next = &self.current_images[j + 1];
                        if next.is_some() && *next != Some(image[11].clone()) {
                            Some(image[11].clone())
                        } else {
                            None
                        }
                    } else {
                        None
                    };
                }
                if self.current_images[j].is_none() {
                    self.shiftbase += 1;
                }
                abs_value /= 10;
            }
        }
        let region_width = self.data.region.width;
        self.length = (region_width + self.space as f32)
            * (self.current_images.len() as f32 - self.shiftbase as f32);
        self.shift = if self.align == 0 {
            0.0
        } else if self.align == 1 {
            (region_width + self.space as f32) * self.shiftbase as f32
        } else {
            (region_width + self.space as f32) * 0.5 * self.shiftbase as f32
        };
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        for (j, current_img) in self.current_images.iter().enumerate() {
            if let Some(img) = current_img {
                let region = self.data.region;
                if let Some(ref offsets) = self.offsets {
                    if j < offsets.len() {
                        self.data.draw_image_at(
                            sprite,
                            img,
                            region.x + (region.width + self.space as f32) * j as f32 - self.shift
                                + offsets[j].x,
                            region.y + offsets[j].y,
                            region.width + offsets[j].w,
                            region.height + offsets[j].h,
                        );
                    } else {
                        self.data.draw_image_at(
                            sprite,
                            img,
                            region.x + (region.width + self.space as f32) * j as f32 - self.shift,
                            region.y,
                            region.width,
                            region.height,
                        );
                    }
                } else {
                    self.data.draw_image_at(
                        sprite,
                        img,
                        region.x + (region.width + self.space as f32) * j as f32 - self.shift,
                        region.y,
                        region.width,
                        region.height,
                    );
                }
            }
        }
    }

    pub fn draw_with_value(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        value: i32,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.prepare_with_value(time, state, value, offset_x, offset_y);
        if self.data.draw {
            self.draw(sprite);
        }
    }

    pub fn length(&self) -> f32 {
        self.length
    }

    pub fn ref_prop(&self) -> Option<&dyn IntegerProperty> {
        self.ref_prop.as_deref()
    }

    pub fn dispose(&mut self) {
        self.image.dispose();
        if let Some(ref mut m) = self.mimage {
            m.dispose();
        }
        self.data.set_disposed();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::{Color, Rectangle, TextureRegion};
    use crate::skin_object::SkinObjectRenderer;
    use crate::test_helpers::MockMainState;

    /// Helper: make a TextureRegion with known dimensions.
    fn make_region(w: i32, h: i32) -> TextureRegion {
        TextureRegion {
            region_width: w,
            region_height: h,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
            ..TextureRegion::default()
        }
    }

    /// Helper: create digit images for SkinNumber (12 entries: digits 0-9, space=10, minus=11).
    fn make_digit_images() -> Vec<Vec<TextureRegion>> {
        let digits: Vec<TextureRegion> = (0..12).map(|_| make_region(24, 32)).collect();
        vec![digits]
    }

    fn make_plain_digit_images() -> Vec<Vec<TextureRegion>> {
        let digits: Vec<TextureRegion> = (0..10).map(|_| make_region(24, 32)).collect();
        vec![digits]
    }

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    fn setup_data(data: &mut crate::skin_object::SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            &crate::skin_object::DestinationParams {
                time: 0,
                x,
                y,
                w,
                h,
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
    fn test_skin_number_draw_basic_single_digit() {
        // Value=5, keta=1, left-aligned
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 1,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 10.0, 20.0, 24.0, 32.0);

        // Directly prepare with value
        num.data.draw = true;
        num.data.region = Rectangle::new(10.0, 20.0, 24.0, 32.0);
        num.data.color = Color::new(1.0, 1.0, 1.0, 1.0);
        num.value = i32::MIN; // Force recalculation
        num.prepare_with_value(0, &MockMainState::default(), 5, 0.0, 0.0);
        assert!(num.data.draw);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // 1 digit = 1 quad = 6 vertices
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_number_draw_multi_digit_spacing() {
        // Value=123, keta=3, space=2, left-aligned (align=0)
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 1,
                space: 2,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 100.0, 50.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 123, 0.0, 0.0);
        assert!(num.data.draw);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // 3 digits = 3 quads = 18 vertices
        assert_eq!(renderer.sprite.vertices().len(), 18);

        // Verify digit positions: each digit at x + (width + space) * j
        // digit 0 (hundreds): x=100 + (24+2)*0 = 100
        // digit 1 (tens):     x=100 + (24+2)*1 = 126
        // digit 2 (ones):     x=100 + (24+2)*2 = 152
        let verts = renderer.sprite.vertices();
        assert!((verts[0].position[0] - 100.01).abs() < 0.02);
        assert!((verts[6].position[0] - 126.01).abs() < 0.02);
        assert!((verts[12].position[0] - 152.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_number_draw_alignment_right() {
        // Value=5, keta=3, zeropadding=0, align=1 (right)
        // shift = (region.width + space) * shiftbase
        // For value=5, keta=3: digits=[None, None, 5] => shiftbase=2
        // shift = (24 + 0) * 2 = 48
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 0,
                space: 0,
                align: 1,
            },
            0,
        );
        setup_data(&mut num.data, 100.0, 50.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 5, 0.0, 0.0);
        assert!(num.data.draw);

        // shiftbase=2, align=1 (right) => shift = 24 * 2 = 48
        assert_eq!(num.shift, 48.0);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // Only 1 digit drawn (index 2, ones place)
        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Position: x=100 + (24+0)*2 - 48 = 100 + 48 - 48 = 100
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 100.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_number_draw_alignment_center() {
        // Value=5, keta=3, zeropadding=0, align=2 (center)
        // shift = (region.width + space) * 0.5 * shiftbase
        // shiftbase=2 => shift = 24 * 0.5 * 2 = 24
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 0,
                space: 0,
                align: 2,
            },
            0,
        );
        setup_data(&mut num.data, 100.0, 50.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 5, 0.0, 0.0);

        // shiftbase=2, align=2 (center) => shift = 24 * 0.5 * 2 = 24
        assert_eq!(num.shift, 24.0);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // Only 1 digit drawn
        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Position: x=100 + (24+0)*2 - 24 = 100 + 48 - 24 = 124
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 124.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_number_draw_alignment_left() {
        // Value=5, keta=3, zeropadding=0, align=0 (left)
        // shift = 0
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 100.0, 50.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 5, 0.0, 0.0);

        assert_eq!(num.shift, 0.0);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // Only 1 digit drawn (at position j=2)
        assert_eq!(renderer.sprite.vertices().len(), 6);
        // Position: x=100 + (24+0)*2 - 0 = 148
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 148.01).abs() < 0.02);
    }

    #[test]
    fn test_skin_number_draw_zero_padded() {
        // Value=5, keta=3, zeropadding=1, all digits should show (zero-filled)
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 1,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 0.0, 0.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 5, 0.0, 0.0);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        // All 3 digits drawn: 0, 0, 5
        assert_eq!(renderer.sprite.vertices().len(), 18);
    }

    #[test]
    fn test_skin_number_draw_with_offsets() {
        // Value=42, keta=2, with per-digit offsets
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 2,
                zeropadding: 1,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 10.0, 20.0, 24.0, 32.0);

        let offsets = vec![
            SkinOffset {
                x: 2.0,
                y: 3.0,
                w: 4.0,
                h: 5.0,
                r: 0.0,
                a: 0.0,
            },
            SkinOffset {
                x: -1.0,
                y: -2.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        ];
        num.set_offsets(offsets);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 42, 0.0, 0.0);

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);

        assert_eq!(renderer.sprite.vertices().len(), 12); // 2 digits

        // Digit 0 (tens=4): x=10 + (24+0)*0 + offset[0].x(2) = 12 + 0.01
        let v0 = &renderer.sprite.vertices()[0];
        assert!((v0.position[0] - 12.01).abs() < 0.02);
        assert!((v0.position[1] - 23.01).abs() < 0.02); // 20 + offset.y(3)

        // Digit 0 width should be 24 + offset.w(4) = 28
        let v1 = &renderer.sprite.vertices()[1];
        assert!((v1.position[0] - 12.01 - 28.0).abs() < 0.02);

        // Digit 1 (ones=2): x=10 + (24+0)*1 + offset[1].x(-1) = 33 + 0.01
        let v6 = &renderer.sprite.vertices()[6];
        assert!((v6.position[0] - 33.01).abs() < 0.02);
        assert!((v6.position[1] - 18.01).abs() < 0.02); // 20 + offset.y(-2)
    }

    #[test]
    fn test_skin_number_length_calculation() {
        // Value=123, keta=5, space=4, zeropadding=0
        // shiftbase=2 (two leading nulls), length = (width+space) * (keta-shiftbase)
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 5,
                zeropadding: 0,
                space: 4,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 0.0, 0.0, 20.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 123, 0.0, 0.0);

        // length = (20 + 4) * (5 - 2) = 24 * 3 = 72
        assert_eq!(num.length(), 72.0);
    }

    #[test]
    fn test_skin_number_mimage_uses_timer_property() {
        // Regression: new_with_timer must pass the TimerPropertyEnum to mimage,
        // not fall back to int timer 0 (which becomes None and ignores timer state).
        //
        // Setup: 2 animation frames for mimage, cycle=1000ms, timer 10 activated at t=0.
        // At time=500ms the source should select frame index 1 (halfway through cycle).
        // With the old bug (timer=None), frame index would always be 0.

        use crate::property::timer_property_factory;

        // Two sets of 12-digit images each (two animation frames).
        let frame0: Vec<TextureRegion> = (0..12).map(|_| make_region(24, 32)).collect();
        let mut frame1_digit: Vec<TextureRegion> = (0..12).map(|_| make_region(48, 32)).collect();
        // Make frame1 distinguishable: use different region_width for digit 5.
        frame1_digit[5] = make_region(99, 32);

        let image_frames = vec![frame0.clone()];
        let mimage_frames = vec![frame0, frame1_digit];

        let timer = timer_property_factory::timer_property(10).unwrap();

        let mut num = SkinNumber::new_with_timer(
            image_frames,
            Some(mimage_frames),
            timer,
            1000, // cycle = 1000ms
            NumberDisplayConfig {
                keta: 1,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 0.0, 0.0, 24.0, 32.0);

        // Activate timer 10 at micro-time 0.
        let mut state = MockMainState::default();
        state.timer.now_time = 500;
        state.timer.now_micro_time = 500_000;
        state.timer.set_timer_value(10, 0);

        // Negative value triggers mimage path.
        num.prepare_with_value(500, &state, -5, 0.0, 0.0);
        assert!(
            num.data.draw,
            "mimage should draw when timer is ON and value is negative"
        );
    }

    #[test]
    fn test_skin_number_mimage_timer_ref_uses_timer_property() {
        // Same regression test but for new_with_timer_ref.
        use crate::property::timer_property_factory;

        let frame0: Vec<TextureRegion> = (0..12).map(|_| make_region(24, 32)).collect();
        let frame1: Vec<TextureRegion> = (0..12).map(|_| make_region(48, 32)).collect();

        let image_frames = vec![frame0.clone()];
        let mimage_frames = vec![frame0, frame1];

        let timer = timer_property_factory::timer_property(10).unwrap();
        let ref_prop: Box<dyn IntegerProperty> = Box::new(ConstIntProp(0));

        let mut num = SkinNumber::new_with_timer_ref(
            image_frames,
            Some(mimage_frames),
            timer,
            1000,
            NumberDisplayConfig {
                keta: 1,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            ref_prop,
        );
        setup_data(&mut num.data, 0.0, 0.0, 24.0, 32.0);

        let mut state = MockMainState::default();
        state.timer.now_time = 500;
        state.timer.now_micro_time = 500_000;
        state.timer.set_timer_value(10, 0);

        num.prepare_with_value(500, &state, -5, 0.0, 0.0);
        assert!(
            num.data.draw,
            "mimage should draw when timer is ON and value is negative"
        );
    }

    /// Constant integer property for testing.
    struct ConstIntProp(i32);
    impl IntegerProperty for ConstIntProp {
        fn get(&self, _state: &dyn crate::reexports::MainState) -> i32 {
            self.0
        }
    }

    #[test]
    fn test_skin_number_draw_invalid_value() {
        let mut num = SkinNumber::new_with_int_timer(
            make_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 0,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 0.0, 0.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, i32::MIN, 0.0, 0.0);

        assert!(!num.data.draw);
        assert_eq!(num.length(), 0.0);
    }

    #[test]
    fn test_skin_number_draws_with_plain_10_digit_sheet() {
        let mut num = SkinNumber::new_with_int_timer(
            make_plain_digit_images(),
            None,
            0,
            0,
            NumberDisplayConfig {
                keta: 3,
                zeropadding: 1,
                space: 0,
                align: 0,
            },
            0,
        );
        setup_data(&mut num.data, 10.0, 20.0, 24.0, 32.0);

        let state = MockMainState::default();
        num.prepare_with_value(0, &state, 42, 0.0, 0.0);

        assert!(
            num.data.draw,
            "10-digit number sheets must remain drawable for JSON/LR2 value objects"
        );

        let mut renderer = SkinObjectRenderer::new();
        num.draw(&mut renderer);
        assert_eq!(renderer.sprite.vertices().len(), 18);
    }
}
