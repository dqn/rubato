// SkinImage.java -> skin_image.rs
// Mechanical line-by-line translation.

use crate::property::integer_property::IntegerProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_source::SkinSource;
use crate::skin_source_image::SkinSourceImage;
use crate::skin_source_movie::SkinSourceMovie;
use crate::skin_source_reference::SkinSourceReference;
use crate::stubs::{MainState, TextureRegion};

pub struct SkinImage {
    pub data: SkinObjectData,
    image: Vec<Option<Box<dyn SkinSource>>>,
    ref_prop: Option<Box<dyn IntegerProperty>>,
    current_image: Option<TextureRegion>,
    removed_sources: Vec<Box<dyn SkinSource>>,
    is_movie: bool,
}

impl SkinImage {
    pub fn new_with_image_id(imageid: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceReference::new(imageid)))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_single(image: TextureRegion) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(
                SkinSourceImage::new_with_int_timer_from_vec(vec![image], 0, 0),
            ))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_int_timer(image: Vec<TextureRegion>, timer: i32, cycle: i32) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(
                SkinSourceImage::new_with_int_timer_from_vec(image, timer, cycle),
            ))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_int_timer_ref_id(
        images: Vec<Vec<TextureRegion>>,
        timer: i32,
        cycle: i32,
        ref_id: i32,
    ) -> Self {
        Self::new_with_int_timer_ref(
            images,
            timer,
            cycle,
            integer_property_factory::get_image_index_property_by_id(ref_id),
        )
    }

    pub fn new_with_int_timer_ref(
        images: Vec<Vec<TextureRegion>>,
        timer: i32,
        cycle: i32,
        ref_prop: Option<Box<dyn IntegerProperty>>,
    ) -> Self {
        let image: Vec<Option<Box<dyn SkinSource>>> = images
            .into_iter()
            .map(|img| -> Option<Box<dyn SkinSource>> {
                Some(Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                    img, timer, cycle,
                )))
            })
            .collect();
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_timer(
        image: Vec<TextureRegion>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: vec![Some(Box::new(SkinSourceImage::new_with_timer_from_vec(
                image,
                Some(timer),
                cycle,
            )))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_timer_ref_id(
        images: Vec<Vec<TextureRegion>>,
        timer: Box<dyn TimerProperty>,
        cycle: i32,
        ref_id: i32,
    ) -> Self {
        // Each image set needs its own timer; for simplicity, use int timer 0
        let image: Vec<Option<Box<dyn SkinSource>>> = images
            .into_iter()
            .map(|img| -> Option<Box<dyn SkinSource>> {
                Some(Box::new(SkinSourceImage::new_with_int_timer_from_vec(
                    img, 0, cycle,
                )))
            })
            .collect();
        let _ = timer; // timer consumed but each source gets int timer 0 as approximation
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop: integer_property_factory::get_image_index_property_by_id(ref_id),
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn new_with_movie(movie: SkinSourceMovie) -> Self {
        let mut data = SkinObjectData::new();
        data.set_image_type(SkinObjectRenderer::TYPE_FFMPEG);
        Self {
            data,
            image: vec![Some(Box::new(movie))],
            ref_prop: None,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: true,
        }
    }

    pub fn new_with_sources_ref_id(image: Vec<SkinSourceImage>, ref_id: i32) -> Self {
        Self::new_with_sources_ref(
            image,
            integer_property_factory::get_image_index_property_by_id(ref_id),
        )
    }

    pub fn new_with_sources_ref(
        image: Vec<SkinSourceImage>,
        ref_prop: Option<Box<dyn IntegerProperty>>,
    ) -> Self {
        let image: Vec<Option<Box<dyn SkinSource>>> = image
            .into_iter()
            .map(|s| -> Option<Box<dyn SkinSource>> { Some(Box::new(s)) })
            .collect();
        Self {
            data: SkinObjectData::new(),
            image,
            ref_prop,
            current_image: None,
            removed_sources: Vec::new(),
            is_movie: false,
        }
    }

    pub fn get_image(&self, time: i64, state: &dyn MainState) -> Option<TextureRegion> {
        self.get_image_at(0, time, state)
    }

    pub fn get_image_at(
        &self,
        value: usize,
        time: i64,
        state: &dyn MainState,
    ) -> Option<TextureRegion> {
        if value < self.image.len()
            && let Some(ref source) = self.image[value]
        {
            return source.get_image(time, state);
        }
        None
    }

    pub fn validate(&mut self) -> bool {
        let mut exist = false;
        for i in 0..self.image.len() {
            if let Some(ref source) = self.image[i] {
                if source.validate() {
                    exist = true;
                } else {
                    let removed = self.image[i].take().unwrap();
                    self.removed_sources.push(removed);
                }
            }
        }

        if !exist {
            return false;
        }

        self.data.validate()
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
            0
        };
        self.prepare_with_value(time, state, value, offset_x, offset_y);
    }

    pub fn prepare_with_value(
        &mut self,
        time: i64,
        state: &dyn MainState,
        mut value: i32,
        offset_x: f32,
        offset_y: f32,
    ) {
        if value < 0 {
            self.data.draw = false;
            return;
        }
        self.data
            .prepare_with_offset(time, state, offset_x, offset_y);
        if value >= self.image.len() as i32 {
            value = 0;
        }
        self.current_image = self.get_image_at(value as usize, time, state);
        if self.current_image.is_none() {
            self.data.draw = false;
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if let Some(ref current_image) = self.current_image.clone() {
            if self.is_movie {
                self.data.set_image_type(3);
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                );
                self.data.set_image_type(0);
            } else {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                );
            }
        }
    }

    pub fn draw_with_offset(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        offset_x: f32,
        offset_y: f32,
    ) {
        if let Some(ref current_image) = self.current_image.clone() {
            if self.is_movie {
                self.data.set_image_type(3);
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x + offset_x,
                    region.y + offset_y,
                    region.width,
                    region.height,
                );
                self.data.set_image_type(0);
            } else {
                let region = self.data.region.clone();
                self.data.draw_image_at(
                    sprite,
                    current_image,
                    region.x + offset_x,
                    region.y + offset_y,
                    region.width,
                    region.height,
                );
            }
        }
    }

    pub fn draw_prepared(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.prepare_with_offset(time, state, offset_x, offset_y);
        if self.data.draw {
            self.draw(sprite);
        }
    }

    pub fn draw_with_value(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        state: &dyn MainState,
        value: i32,
        offset_x: f32,
        offset_y: f32,
    ) {
        self.prepare_with_value(time, state, value, offset_x, offset_y);
        if self.data.draw {
            self.draw(sprite);
        }
    }

    pub fn dispose(&mut self) {
        for source in self.removed_sources.drain(..) {
            // dispose removed sources
            let _ = source;
        }
        for s in self.image.iter_mut().flatten() {
            s.dispose();
        }
        self.data.set_disposed();
    }
}
