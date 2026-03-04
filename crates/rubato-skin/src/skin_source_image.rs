use crate::property::timer_property::TimerProperty;
use crate::skin_source::SkinSource;
use crate::stubs::{MainState, TextureRegion};

/// Skin source image (SkinSourceImage.java)
pub struct SkinSourceImage {
    image: Vec<Option<TextureRegion>>,
    timer: Option<Box<dyn TimerProperty>>,
    cycle: i32,
    disposed: bool,
}

impl SkinSourceImage {
    pub fn new_single(image: TextureRegion) -> Self {
        Self {
            image: vec![Some(image)],
            timer: None,
            cycle: 0,
            disposed: false,
        }
    }

    pub fn new_with_int_timer_from_vec(image: Vec<TextureRegion>, timer: i32, cycle: i32) -> Self {
        Self::new_with_int_timer(image.into_iter().map(Some).collect(), timer, cycle)
    }

    pub fn new_with_int_timer(image: Vec<Option<TextureRegion>>, timer: i32, cycle: i32) -> Self {
        let timer_prop: Option<Box<dyn TimerProperty>> = if timer > 0 {
            crate::property::timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        Self {
            image,
            timer: timer_prop,
            cycle,
            disposed: false,
        }
    }

    pub fn new_with_timer_from_vec(
        image: Vec<TextureRegion>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
    ) -> Self {
        Self::new_with_timer(image.into_iter().map(Some).collect(), timer, cycle)
    }

    pub fn new_with_timer(
        image: Vec<Option<TextureRegion>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
    ) -> Self {
        Self {
            image,
            timer,
            cycle,
            disposed: false,
        }
    }

    pub fn get_images(&self) -> &[Option<TextureRegion>] {
        &self.image
    }

    fn get_image_index(&self, length: usize, time: i64, state: &dyn MainState) -> usize {
        if self.cycle == 0 {
            return 0;
        }

        let mut time = time;
        if let Some(ref timer) = self.timer {
            if timer.is_off(state) {
                return 0;
            }
            time -= timer.get(state);
        }
        if time < 0 {
            return 0;
        }
        ((time * length as i64 / self.cycle as i64) % length as i64) as usize
    }
}

impl SkinSource for SkinSourceImage {
    fn get_image(&self, time: i64, state: &dyn MainState) -> Option<TextureRegion> {
        if !self.image.is_empty() {
            let idx = self.get_image_index(self.image.len(), time, state);
            self.image[idx].clone()
        } else {
            None
        }
    }

    fn validate(&self) -> bool {
        if self.image.is_empty() {
            return false;
        }
        let mut exist = false;
        for tr in &self.image {
            if tr.is_some() {
                exist = true;
            }
        }
        exist
    }

    fn dispose(&mut self) {
        if !self.disposed {
            // dispose textures
            self.disposed = true;
        }
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}
