use crate::property::timer_property::TimerProperty;
use crate::skin_source_set::SkinSourceSet;
use crate::stubs::{MainState, TextureRegion};

/// Skin source image set (SkinSourceImageSet.java)
pub struct SkinSourceImageSet {
    image: Vec<Vec<Option<TextureRegion>>>,
    timer: Option<Box<dyn TimerProperty>>,
    cycle: i32,
    disposed: bool,
}

impl SkinSourceImageSet {
    pub fn new_with_int_timer_from_vecs(
        image: Vec<Vec<TextureRegion>>,
        timer: i32,
        cycle: i32,
    ) -> Self {
        Self::new_with_int_timer(
            image
                .into_iter()
                .map(|v| v.into_iter().map(Some).collect())
                .collect(),
            timer,
            cycle,
        )
    }

    pub fn new_with_int_timer(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: i32,
        cycle: i32,
    ) -> Self {
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

    pub fn new_with_timer_from_vecs(
        image: Vec<Vec<TextureRegion>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
    ) -> Self {
        Self::new_with_timer(
            image
                .into_iter()
                .map(|v| v.into_iter().map(Some).collect())
                .collect(),
            timer,
            cycle,
        )
    }

    pub fn new_with_timer(
        image: Vec<Vec<Option<TextureRegion>>>,
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

    pub fn get_all_images(&self) -> &[Vec<Option<TextureRegion>>] {
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

impl SkinSourceSet for SkinSourceImageSet {
    fn get_images(&self, time: i64, state: &dyn MainState) -> Option<Vec<TextureRegion>> {
        if !self.image.is_empty() {
            let idx = self.get_image_index(self.image.len(), time, state);
            Some(
                self.image[idx]
                    .iter()
                    .map(|tr| tr.clone().unwrap_or_default())
                    .collect(),
            )
        } else {
            None
        }
    }

    fn validate(&self) -> bool {
        if self.image.is_empty() {
            return false;
        }
        let mut exist = false;
        for trs in &self.image {
            for tr in trs {
                if tr.is_some() {
                    exist = true;
                }
            }
        }
        exist
    }

    fn dispose(&mut self) {
        if !self.disposed {
            self.disposed = true;
        }
    }

    fn is_disposed(&self) -> bool {
        self.disposed
    }
}
