// SkinNumber.java -> skin_number.rs
// Mechanical line-by-line translation.

use crate::property::integer_property::IntegerProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_source_image_set::SkinSourceImageSet;
use crate::skin_source_set::SkinSourceSet;
use crate::stubs::{MainState, SkinOffset, TextureRegion};

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
    pub fn new_with_int_timer(
        image: Vec<Vec<TextureRegion>>,
        mimage: Option<Vec<Vec<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        keta: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        align: i32,
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
            ref_prop: integer_property_factory::get_integer_property_by_id(id),
            keta,
            current_images: vec![None; keta as usize],
            zeropadding,
            space,
            align,
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
        keta: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn IntegerProperty>,
        align: i32,
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
            keta,
            current_images: vec![None; keta as usize],
            zeropadding,
            space,
            align,
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
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        keta: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        align: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                image,
                Some(timer),
                cycle,
            )),
            mimage: mimage.map(|m| -> Box<dyn SkinSourceSet> {
                Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                    m, 0, cycle,
                ))
            }),
            ref_prop: integer_property_factory::get_integer_property_by_id(id),
            keta,
            current_images: vec![None; keta as usize],
            zeropadding,
            space,
            align,
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
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        keta: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn IntegerProperty>,
        align: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: Box::new(SkinSourceImageSet::new_with_timer_from_vecs(
                image,
                Some(timer),
                cycle,
            )),
            mimage: mimage.map(|m| -> Box<dyn SkinSourceSet> {
                Box::new(SkinSourceImageSet::new_with_int_timer_from_vecs(
                    m, 0, cycle,
                ))
            }),
            ref_prop: Some(ref_prop),
            keta,
            current_images: vec![None; keta as usize],
            zeropadding,
            space,
            align,
            value: i32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            image_set: None,
            shift: 0.0,
        }
    }

    pub fn get_keta(&self) -> i32 {
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
        let image = images.unwrap();

        if self.value != value || self.image_set.as_ref() != Some(&image) {
            self.value = value;
            self.image_set = Some(image.clone());
            self.shiftbase = 0;
            let mut abs_value = value.unsigned_abs() as i32;
            for j in (0..self.current_images.len()).rev() {
                if self.mimage.is_some() && self.zeropadding > 0 {
                    if j == 0 {
                        self.current_images[j] = Some(image[11].clone());
                    } else if abs_value > 0 || j == self.current_images.len() - 1 {
                        self.current_images[j] = Some(image[(abs_value % 10) as usize].clone());
                    } else {
                        self.current_images[j] =
                            Some(image[if self.zeropadding == 2 { 10 } else { 0 }].clone());
                    }
                } else if abs_value > 0 || j == self.current_images.len() - 1 {
                    self.current_images[j] = Some(image[(abs_value % 10) as usize].clone());
                } else {
                    self.current_images[j] = if self.zeropadding == 2 {
                        Some(image[10].clone())
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
        for j in 0..self.current_images.len() {
            if let Some(ref img) = self.current_images[j].clone() {
                let region = self.data.region.clone();
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

    pub fn get_length(&self) -> f32 {
        self.length
    }

    pub fn dispose(&mut self) {
        self.image.dispose();
        if let Some(ref mut m) = self.mimage {
            m.dispose();
        }
        self.data.set_disposed();
    }
}
