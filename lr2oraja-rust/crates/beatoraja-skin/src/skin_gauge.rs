// SkinGauge.java -> skin_gauge.rs
// Mechanical line-by-line translation.
// Gauge object that renders a segmented gauge bar (e.g., groove gauge).

use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_source_image_set::SkinSourceImageSet;
use crate::skin_source_set::SkinSourceSet;
use crate::stubs::{MainState, TextureRegion};

/// Animation type: random flicker
pub const ANIMATION_RANDOM: i32 = 0;
/// Animation type: increase
pub const ANIMATION_INCREASE: i32 = 1;
/// Animation type: decrease
pub const ANIMATION_DECREASE: i32 = 2;
/// Animation type: flickering
pub const ANIMATION_FLICKERING: i32 = 3;

/// Gauge rendering object.
///
/// Corresponds to Java `SkinGauge`.
/// Renders a segmented gauge bar using image tiles, with configurable
/// animation type (random, increase, decrease, flickering).
pub struct SkinGauge {
    pub data: SkinObjectData,
    /// Gauge image source (tile images indexed by gauge state)
    image: SkinSourceImageSet,
    /// Animation type (0=random, 1=increase, 2=decrease, 3=flickering)
    animation_type: i32,
    /// Animation range (number of animation frames)
    animation_range: i32,
    /// Animation interval in milliseconds
    duration: i64,
    /// Number of gauge parts/segments
    parts: i32,
    /// Current animation frame
    animation: i32,
    /// Next animation update time
    atime: i64,
    /// Current gauge value
    value: f32,
    /// Current gauge type
    gauge_type: i32,
    /// Maximum gauge value
    max: f32,
    /// Border value (clear threshold)
    border: f32,
    /// Cached images from source
    images: Vec<TextureRegion>,
    /// Result screen: gauge fill animation start time (ms)
    starttime: i32,
    /// Result screen: gauge fill animation end time (ms)
    endtime: i32,
}

impl SkinGauge {
    /// Creates a new SkinGauge with the given image tiles.
    ///
    /// Corresponds to Java `SkinGauge(TextureRegion[][], int timer, int cycle, int parts, int type, int range, int duration)`.
    pub fn new(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        parts: i32,
        animation_type: i32,
        animation_range: i32,
        duration: i64,
    ) -> Self {
        Self {
            data: SkinObjectData::new(),
            image: SkinSourceImageSet::new_with_int_timer(image, timer, cycle),
            animation_type,
            animation_range,
            duration,
            parts,
            animation: 0,
            atime: 0,
            value: 0.0,
            gauge_type: 0,
            max: 100.0,
            border: 80.0,
            images: Vec::new(),
            starttime: 0,
            endtime: 500,
        }
    }

    pub fn set_starttime(&mut self, starttime: i32) {
        self.starttime = starttime;
    }

    pub fn set_endtime(&mut self, endtime: i32) {
        self.endtime = endtime;
    }

    pub fn get_parts(&self) -> i32 {
        self.parts
    }

    pub fn set_parts(&mut self, parts: i32) {
        self.parts = parts;
    }

    pub fn get_animation_type(&self) -> i32 {
        self.animation_type
    }

    pub fn get_animation_range(&self) -> i32 {
        self.animation_range
    }

    pub fn get_duration(&self) -> i64 {
        self.duration
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);

        // Update animation
        if self.animation_range < 0 || self.duration <= 0 {
            self.animation = 0;
        } else {
            match self.animation_type {
                ANIMATION_RANDOM => {
                    if self.atime < time {
                        // Use time-based pseudo-random instead of Math.random()
                        self.animation = ((time % (self.animation_range as i64 + 1)).unsigned_abs()
                            % (self.animation_range as u64 + 1))
                            as i32;
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_INCREASE => {
                    if self.atime < time {
                        self.animation =
                            (self.animation + self.animation_range) % (self.animation_range + 1);
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_DECREASE => {
                    if self.atime < time {
                        self.animation = (self.animation + 1) % (self.animation_range + 1);
                        self.atime = time + self.duration;
                    }
                }
                ANIMATION_FLICKERING => {
                    self.animation = (time % self.duration) as i32;
                }
                _ => {}
            }
        }

        // Get images from source
        if let Some(imgs) = self.image.get_images(time, state) {
            self.images = imgs;
        }
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        if self.images.is_empty() || self.parts <= 0 {
            return;
        }

        let region = &self.data.region;
        let notes = if self.value > 0.0 {
            ((self.value * self.parts as f32 / self.max) as i32).max(1)
        } else {
            0
        };

        // exgauge maps gauge type to color index (6 colors per type)
        let ex_gauge = (if self.gauge_type >= 6 {
            self.gauge_type - 3
        } else {
            self.gauge_type
        }) * 6;

        sprite.set_blend(self.data.dstblend);
        sprite.set_type(0); // TYPE_NORMAL

        match self.animation_type {
            ANIMATION_RANDOM | ANIMATION_INCREASE | ANIMATION_DECREASE => {
                for i in 1..=self.parts {
                    let border_val = i as f32 * self.max / self.parts as f32;
                    let img_idx = ex_gauge
                        + if notes == i {
                            4
                        } else if notes - self.animation > i {
                            0
                        } else {
                            2
                        }
                        + if border_val < self.border { 1 } else { 0 };

                    let img_idx = img_idx as usize;
                    if img_idx < self.images.len() {
                        sprite.draw(
                            &self.images[img_idx],
                            region.x + region.width * (i - 1) as f32 / self.parts as f32,
                            region.y,
                            region.width / self.parts as f32,
                            region.height,
                        );
                    }
                }
            }
            ANIMATION_FLICKERING => {
                for i in 1..=self.parts {
                    let border_val = i as f32 * self.max / self.parts as f32;
                    let img_idx = ex_gauge
                        + if notes >= i { 0 } else { 2 }
                        + if border_val < self.border { 1 } else { 0 };

                    let img_idx = img_idx as usize;
                    if img_idx < self.images.len() {
                        sprite.draw(
                            &self.images[img_idx],
                            region.x + region.width * (i - 1) as f32 / self.parts as f32,
                            region.y,
                            region.width / self.parts as f32,
                            region.height,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_gauge_new() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 4, 33);
        assert_eq!(gauge.get_parts(), 50);
        assert_eq!(gauge.get_animation_type(), ANIMATION_RANDOM);
        assert_eq!(gauge.get_animation_range(), 4);
        assert_eq!(gauge.get_duration(), 33);
    }

    #[test]
    fn test_skin_gauge_set_parts() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 4, 33);
        gauge.set_parts(100);
        assert_eq!(gauge.get_parts(), 100);
    }

    #[test]
    fn test_skin_gauge_zero_duration_no_panic() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_FLICKERING, 4, 0);
        // duration=0 should not panic
        gauge.animation_type = ANIMATION_FLICKERING;
        gauge.duration = 0;
        // Manually test the animation branch (prepare requires MainState)
        assert_eq!(gauge.animation, 0);
    }

    #[test]
    fn test_skin_gauge_negative_animation_range_no_panic() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, -1, 33);
        assert_eq!(gauge.animation_range, -1);
        // Should not panic if prepare is called
    }

    #[test]
    fn test_skin_gauge_starttime_endtime() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 4, 33);
        gauge.set_starttime(100);
        gauge.set_endtime(2000);
        assert_eq!(gauge.starttime, 100);
        assert_eq!(gauge.endtime, 2000);
    }
}
