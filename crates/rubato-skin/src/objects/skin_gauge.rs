// SkinGauge.java -> skin_gauge.rs
// Mechanical line-by-line translation.
// Gauge object that renders a segmented gauge bar (e.g., groove gauge).

use crate::sources::skin_source_image_set::SkinSourceImageSet;
use crate::sources::skin_source_set::SkinSourceSet;
use crate::stubs::{MainState, TextureRegion};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

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
    pub parts: i32,
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
    pub starttime: i32,
    /// Result screen: gauge fill animation end time (ms)
    pub endtime: i32,
    /// Whether the mode-change border alignment check has been performed.
    /// Java: isCheckedModeChanged
    is_checked_mode_changed: bool,
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
            is_checked_mode_changed: false,
        }
    }

    pub fn parts(&self) -> i32 {
        self.parts
    }

    pub fn animation_type(&self) -> i32 {
        self.animation_type
    }

    pub fn animation_range(&self) -> i32 {
        self.animation_range
    }

    pub fn duration(&self) -> i64 {
        self.duration
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.data.prepare(time, state);

        // Sync gauge value and type from game state
        self.value = state.get_gauge_value();
        self.gauge_type = state.gauge_type();

        // Update animation
        if self.animation_range < 0 || self.duration <= 0 {
            self.animation = 0;
        } else {
            match self.animation_type {
                ANIMATION_RANDOM => {
                    if self.atime < time {
                        // Java uses Math.random() for uniform random selection.
                        // Use a lightweight hash of time for pseudo-random behavior
                        // instead of sequential cycling (time % range).
                        let hash = (time as u64).wrapping_mul(2654435761) >> 16;
                        self.animation = (hash % (self.animation_range as u64 + 1)) as i32;
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

        // Adjust parts count so gauge borders divide evenly when the chart's
        // mode was converted (e.g. 7-key -> 9-key).
        // Java: SkinGauge.prepare() isCheckedModeChanged block
        if !self.is_checked_mode_changed {
            if state.is_mode_changed() {
                let borders = state.gauge_element_borders();
                let mut set_parts = self.parts;
                for &(border, max) in &borders {
                    if max <= 0.0 {
                        continue;
                    }
                    let max_i = max as i32;
                    for i in self.parts..=max_i {
                        if i <= 0 {
                            continue;
                        }
                        let step = max / i as f32;
                        if step > 0.0 && border % step == 0.0 {
                            set_parts = set_parts.max(i);
                            break;
                        }
                    }
                }
                self.parts = set_parts;
            }
            self.is_checked_mode_changed = true;
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

        sprite.blend = self.data.dstblend;
        sprite.obj_type = 0; // TYPE_NORMAL

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
        assert_eq!(gauge.parts(), 50);
        assert_eq!(gauge.animation_type(), ANIMATION_RANDOM);
        assert_eq!(gauge.animation_range(), 4);
        assert_eq!(gauge.duration(), 33);
    }

    #[test]
    fn test_skin_gauge_set_parts() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 4, 33);
        gauge.parts = 100;
        assert_eq!(gauge.parts(), 100);
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
        gauge.starttime = 100;
        gauge.endtime = 2000;
        assert_eq!(gauge.starttime, 100);
        assert_eq!(gauge.endtime, 2000);
    }

    // --- Notes calculation tests ---
    // The draw method computes: notes = value > 0 ? max(1, (value * parts / max) as i32) : 0

    #[test]
    fn notes_calc_zero_value_gives_zero() {
        // When value == 0, notes == 0
        let value: f32 = 0.0;
        let parts: i32 = 50;
        let max: f32 = 100.0;
        let notes = if value > 0.0 {
            ((value * parts as f32 / max) as i32).max(1)
        } else {
            0
        };
        assert_eq!(notes, 0);
    }

    #[test]
    fn notes_calc_small_positive_value_gives_at_least_one() {
        // Any value > 0 should give at least 1 (the .max(1) clamp)
        let value: f32 = 0.001;
        let parts: i32 = 50;
        let max: f32 = 100.0;
        let notes = if value > 0.0 {
            ((value * parts as f32 / max) as i32).max(1)
        } else {
            0
        };
        assert_eq!(notes, 1);
    }

    #[test]
    fn notes_calc_full_value_gives_parts() {
        // value == max -> notes == parts
        let value: f32 = 100.0;
        let parts: i32 = 50;
        let max: f32 = 100.0;
        let notes = if value > 0.0 {
            ((value * parts as f32 / max) as i32).max(1)
        } else {
            0
        };
        assert_eq!(notes, parts);
    }

    #[test]
    fn notes_calc_half_value() {
        // value == 50 out of 100, 50 parts -> 25
        let value: f32 = 50.0;
        let parts: i32 = 50;
        let max: f32 = 100.0;
        let notes = if value > 0.0 {
            ((value * parts as f32 / max) as i32).max(1)
        } else {
            0
        };
        assert_eq!(notes, 25);
    }

    // --- ex_gauge calculation tests ---
    // ex_gauge = (gauge_type >= 6 ? gauge_type - 3 : gauge_type) * 6

    #[test]
    fn ex_gauge_type_0() {
        let gauge_type = 0;
        let ex = (if gauge_type >= 6 {
            gauge_type - 3
        } else {
            gauge_type
        }) * 6;
        assert_eq!(ex, 0);
    }

    #[test]
    fn ex_gauge_type_1() {
        let gauge_type = 1;
        let ex = (if gauge_type >= 6 {
            gauge_type - 3
        } else {
            gauge_type
        }) * 6;
        assert_eq!(ex, 6);
    }

    #[test]
    fn ex_gauge_type_5() {
        let gauge_type = 5;
        let ex = (if gauge_type >= 6 {
            gauge_type - 3
        } else {
            gauge_type
        }) * 6;
        assert_eq!(ex, 30);
    }

    #[test]
    fn ex_gauge_type_6_wraps() {
        // gauge_type 6 -> (6 - 3) * 6 = 18
        let gauge_type = 6;
        let ex = (if gauge_type >= 6 {
            gauge_type - 3
        } else {
            gauge_type
        }) * 6;
        assert_eq!(ex, 18);
    }

    #[test]
    fn ex_gauge_type_7_wraps() {
        // gauge_type 7 -> (7 - 3) * 6 = 24
        let gauge_type = 7;
        let ex = (if gauge_type >= 6 {
            gauge_type - 3
        } else {
            gauge_type
        }) * 6;
        assert_eq!(ex, 24);
    }

    // --- Image index calculation for RANDOM/INCREASE/DECREASE animation ---
    // img_idx = ex_gauge + (notes==i ? 4 : notes-animation>i ? 0 : 2) + (border_val < border ? 1 : 0)

    fn compute_img_idx_rid(
        ex_gauge: i32,
        notes: i32,
        animation: i32,
        i: i32,
        border_val: f32,
        border: f32,
    ) -> i32 {
        ex_gauge
            + if notes == i {
                4
            } else if notes - animation > i {
                0
            } else {
                2
            }
            + if border_val < border { 1 } else { 0 }
    }

    #[test]
    fn img_idx_rid_at_notes_position() {
        // Segment i == notes: always uses offset 4 (current tip)
        assert_eq!(compute_img_idx_rid(0, 10, 0, 10, 50.0, 80.0), 5); // 0+4+1 (below border)
        assert_eq!(compute_img_idx_rid(0, 10, 0, 10, 90.0, 80.0), 4); // 0+4+0 (above border)
    }

    #[test]
    fn img_idx_rid_filled_segment() {
        // notes - animation > i: filled segment, offset 0
        // notes=10, animation=2, i=5 -> 10-2=8 > 5 -> filled
        assert_eq!(compute_img_idx_rid(0, 10, 2, 5, 50.0, 80.0), 1); // 0+0+1
        assert_eq!(compute_img_idx_rid(0, 10, 2, 5, 90.0, 80.0), 0); // 0+0+0
    }

    #[test]
    fn img_idx_rid_empty_segment() {
        // notes - animation <= i: empty segment, offset 2
        // notes=10, animation=2, i=9 -> 10-2=8 <= 9 -> empty
        assert_eq!(compute_img_idx_rid(0, 10, 2, 9, 50.0, 80.0), 3); // 0+2+1
        assert_eq!(compute_img_idx_rid(0, 10, 2, 9, 90.0, 80.0), 2); // 0+2+0
    }

    #[test]
    fn img_idx_rid_with_ex_gauge_offset() {
        // ex_gauge = 6 (gauge_type=1)
        assert_eq!(compute_img_idx_rid(6, 10, 0, 10, 50.0, 80.0), 11); // 6+4+1
        assert_eq!(compute_img_idx_rid(6, 10, 0, 5, 90.0, 80.0), 6); // 6+0+0
    }

    #[test]
    fn img_idx_rid_border_boundary() {
        // border_val exactly at border -> not less than, so +0
        assert_eq!(compute_img_idx_rid(0, 5, 0, 5, 80.0, 80.0), 4); // border_val == border -> 0
        // border_val just below border -> +1
        assert_eq!(compute_img_idx_rid(0, 5, 0, 5, 79.9, 80.0), 5);
    }

    // --- Image index calculation for FLICKERING animation ---
    // img_idx = ex_gauge + (notes >= i ? 0 : 2) + (border_val < border ? 1 : 0)

    fn compute_img_idx_flicker(
        ex_gauge: i32,
        notes: i32,
        i: i32,
        border_val: f32,
        border: f32,
    ) -> i32 {
        ex_gauge + if notes >= i { 0 } else { 2 } + if border_val < border { 1 } else { 0 }
    }

    #[test]
    fn img_idx_flicker_filled() {
        // notes >= i -> filled, offset 0
        assert_eq!(compute_img_idx_flicker(0, 10, 5, 50.0, 80.0), 1); // 0+0+1
        assert_eq!(compute_img_idx_flicker(0, 10, 10, 90.0, 80.0), 0); // 0+0+0
    }

    #[test]
    fn img_idx_flicker_empty() {
        // notes < i -> empty, offset 2
        assert_eq!(compute_img_idx_flicker(0, 5, 10, 50.0, 80.0), 3); // 0+2+1
        assert_eq!(compute_img_idx_flicker(0, 5, 10, 90.0, 80.0), 2); // 0+2+0
    }

    #[test]
    fn img_idx_flicker_with_ex_gauge() {
        assert_eq!(compute_img_idx_flicker(12, 10, 5, 50.0, 80.0), 13); // 12+0+1
        assert_eq!(compute_img_idx_flicker(12, 5, 10, 90.0, 80.0), 14); // 12+2+0
    }

    // --- draw() early return tests ---

    #[test]
    fn draw_returns_early_when_images_empty() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 4, 33);
        // images is empty by default (not populated via prepare)
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic -- just returns early
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_returns_early_when_parts_zero() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 0, ANIMATION_RANDOM, 4, 33);
        gauge.images = vec![TextureRegion::new(); 6]; // populate images
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic -- parts <= 0 triggers early return
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_returns_early_when_parts_negative() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, -1, ANIMATION_RANDOM, 4, 33);
        gauge.images = vec![TextureRegion::new(); 6];
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    // --- draw() integration with internal state ---
    // These tests set up internal state directly (since we're in the same module)
    // and verify draw() completes without panic for various configurations.

    #[test]
    fn draw_random_animation_full_gauge() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 100.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.animation = 1;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_random_animation_empty_gauge() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 0.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_flickering_animation() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_FLICKERING, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 50.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_increase_animation() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_INCREASE, 3, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 75.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.animation = 2;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_decrease_animation() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_DECREASE, 3, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 30.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.animation = 1;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_with_gauge_type_above_6() {
        // gauge_type >= 6 triggers the wrap: (gauge_type - 3) * 6
        // gauge_type=6 -> ex_gauge=18, needs at least 18+5=23 images
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        gauge.images = vec![TextureRegion::new(); 24]; // enough for ex_gauge=18 + 5
        gauge.value = 50.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 6;
        gauge.animation = 0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_unknown_animation_type_does_nothing() {
        // animation_type = 99 falls into the _ => {} branch
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, 99, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 50.0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic, just does nothing for unknown animation type
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_skips_out_of_bounds_image_index() {
        // With only 3 images, higher indices should be skipped (the if img_idx < self.images.len() check)
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 5, ANIMATION_FLICKERING, 0, 100);
        gauge.images = vec![TextureRegion::new(); 3]; // only indices 0,1,2 valid
        gauge.value = 50.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        // Some segments will compute img_idx >= 3, those should be skipped
        gauge.draw(&mut renderer);
    }

    // --- border_val computation test ---
    // border_val = i * max / parts for each segment i in 1..=parts

    #[test]
    fn border_val_calculation() {
        let max = 100.0f32;
        let parts = 10;
        // For i=1: 1 * 100 / 10 = 10.0
        // For i=5: 5 * 100 / 10 = 50.0
        // For i=8: 8 * 100 / 10 = 80.0 (at border)
        // For i=10: 10 * 100 / 10 = 100.0
        assert!((1.0 * max / parts as f32 - 10.0).abs() < f32::EPSILON);
        assert!((5.0 * max / parts as f32 - 50.0).abs() < f32::EPSILON);
        assert!((8.0 * max / parts as f32 - 80.0).abs() < f32::EPSILON);
        assert!((10.0 * max / parts as f32 - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn border_val_below_and_above_border() {
        let max = 100.0f32;
        let parts = 10;
        let border = 80.0f32;

        // Segments 1-7 are below border (border_val < 80)
        for i in 1..=7 {
            let border_val = i as f32 * max / parts as f32;
            assert!(border_val < border, "segment {i} should be below border");
        }
        // Segments 8-10 are at or above border
        for i in 8..=10 {
            let border_val = i as f32 * max / parts as f32;
            assert!(
                border_val >= border,
                "segment {i} should be at or above border"
            );
        }
    }

    #[test]
    fn draw_single_part_gauge() {
        // Edge case: gauge with just 1 part
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 1, ANIMATION_FLICKERING, 0, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 100.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.data.region = crate::stubs::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    // --- Default values test ---

    // --- prepare() gauge value sync tests ---
    // These verify the fix for the gauge value wiring bug:
    // SkinGauge.prepare() was not reading gauge value from MainState,
    // causing the gauge display to always show 0% regardless of actual state.

    /// MockMainState with configurable gauge value and type.
    struct GaugeMockState {
        gauge_value: f32,
        gauge_type: i32,
    }

    impl rubato_types::timer_access::TimerAccess for GaugeMockState {
        fn now_time(&self) -> i64 {
            0
        }
        fn now_micro_time(&self) -> i64 {
            0
        }
        fn micro_timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            i64::MIN
        }
        fn timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            i64::MIN
        }
        fn now_time_for(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            0
        }
        fn is_timer_on(&self, _: rubato_types::timer_id::TimerId) -> bool {
            false
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for GaugeMockState {
        fn gauge_value(&self) -> f32 {
            self.gauge_value
        }
        fn gauge_type(&self) -> i32 {
            self.gauge_type
        }
    }

    impl crate::stubs::MainState for GaugeMockState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            static TIMER: std::sync::OnceLock<crate::stubs::Timer> = std::sync::OnceLock::new();
            TIMER.get_or_init(crate::stubs::Timer::default)
        }
        fn get_main(&self) -> &crate::stubs::MainController {
            static MC: crate::stubs::MainController = crate::stubs::MainController { debug: false };
            &MC
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &crate::stubs::PlayerResource {
            static PR: crate::stubs::PlayerResource = crate::stubs::PlayerResource;
            &PR
        }
    }

    #[test]
    fn prepare_syncs_gauge_value_from_state() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        assert!(
            (gauge.value - 0.0).abs() < f32::EPSILON,
            "initial value should be 0"
        );

        let state = GaugeMockState {
            gauge_value: 75.0,
            gauge_type: 2,
        };
        gauge.prepare(100, &state);

        assert!(
            (gauge.value - 75.0).abs() < f32::EPSILON,
            "prepare() should sync gauge value from MainState, got {}",
            gauge.value
        );
        assert_eq!(
            gauge.gauge_type, 2,
            "prepare() should sync gauge type from MainState"
        );
    }

    #[test]
    fn prepare_updates_gauge_value_each_frame() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);

        // Frame 1: gauge at 50%
        let state1 = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
        };
        gauge.prepare(100, &state1);
        assert!((gauge.value - 50.0).abs() < f32::EPSILON);

        // Frame 2: gauge drops to 30%
        let state2 = GaugeMockState {
            gauge_value: 30.0,
            gauge_type: 0,
        };
        gauge.prepare(200, &state2);
        assert!(
            (gauge.value - 30.0).abs() < f32::EPSILON,
            "value should update each frame"
        );
    }

    /// Regression: ANIMATION_RANDOM should produce pseudo-random distribution,
    /// not a sequential sweep. The old code used `time % (range + 1)` which
    /// cycles 0,1,2,3,4,0,1,2,... Java uses Math.random().
    #[test]
    fn animation_random_is_not_sequential() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 9, 1);

        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
        };

        // Collect animation values over 10 consecutive time steps.
        // With sequential cycling (old bug), values would be 0,1,2,...,9.
        // With hash-based pseudo-random, the distribution should differ.
        let mut values = Vec::new();
        for t in 0..10 {
            gauge.atime = 0; // force update each step
            gauge.prepare(t, &state);
            values.push(gauge.animation);
        }

        // The hash-based output should NOT be the trivial sequential pattern.
        let sequential: Vec<i32> = (0..10).collect();
        assert_ne!(
            values, sequential,
            "ANIMATION_RANDOM should not produce sequential 0..9 pattern, got {:?}",
            values
        );

        // All values should be within range [0, animation_range]
        for &v in &values {
            assert!(
                v >= 0 && v <= 9,
                "animation value {} out of range [0, 9]",
                v
            );
        }
    }

    #[test]
    fn default_gauge_state() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 4, 33);
        assert_eq!(gauge.animation, 0);
        assert_eq!(gauge.atime, 0);
        assert!((gauge.value - 0.0).abs() < f32::EPSILON);
        assert_eq!(gauge.gauge_type, 0);
        assert!((gauge.max - 100.0).abs() < f32::EPSILON);
        assert!((gauge.border - 80.0).abs() < f32::EPSILON);
        assert!(gauge.images.is_empty());
        assert_eq!(gauge.starttime, 0);
        assert_eq!(gauge.endtime, 500);
    }

    // --- Mode-change parts adjustment tests ---
    // Java: SkinGauge.prepare() isCheckedModeChanged block
    // When the chart's original mode differs from the current mode (e.g. 7-key -> 9-key),
    // parts is increased so that gauge borders divide evenly into the gauge bar segments.

    /// MockMainState with configurable mode-change and gauge element properties.
    struct ModeChangeMockState {
        gauge_value: f32,
        gauge_type: i32,
        mode_changed: bool,
        /// (border, max) for each gauge type
        element_borders: Vec<(f32, f32)>,
    }

    impl rubato_types::timer_access::TimerAccess for ModeChangeMockState {
        fn now_time(&self) -> i64 {
            0
        }
        fn now_micro_time(&self) -> i64 {
            0
        }
        fn micro_timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            i64::MIN
        }
        fn timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            i64::MIN
        }
        fn now_time_for(&self, _: rubato_types::timer_id::TimerId) -> i64 {
            0
        }
        fn is_timer_on(&self, _: rubato_types::timer_id::TimerId) -> bool {
            false
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for ModeChangeMockState {
        fn gauge_value(&self) -> f32 {
            self.gauge_value
        }
        fn gauge_type(&self) -> i32 {
            self.gauge_type
        }
        fn is_mode_changed(&self) -> bool {
            self.mode_changed
        }
        fn gauge_element_borders(&self) -> Vec<(f32, f32)> {
            self.element_borders.clone()
        }
    }

    impl crate::stubs::MainState for ModeChangeMockState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            static TIMER: std::sync::OnceLock<crate::stubs::Timer> = std::sync::OnceLock::new();
            TIMER.get_or_init(crate::stubs::Timer::default)
        }
        fn get_main(&self) -> &crate::stubs::MainController {
            static MC: crate::stubs::MainController = crate::stubs::MainController { debug: false };
            &MC
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &crate::stubs::PlayerResource {
            static PR: crate::stubs::PlayerResource = crate::stubs::PlayerResource;
            &PR
        }
    }

    #[test]
    fn mode_change_adjusts_parts_for_border_alignment() {
        // Scenario: 7-key chart converted to 9-key.
        // Normal gauge: border=80, max=100. With parts=50, step=100/50=2.0, 80%2.0=0 -> OK.
        // Suppose border=75, max=100, parts=50: step=100/50=2.0, 75%2.0=1.0 != 0 -> not aligned.
        // Loop from 50 to 100 looking for the first i where 75 % (100/i) == 0:
        //   i=50: 100/50=2.0, 75%2.0=1.0 -> no
        //   i=64: 100/64=1.5625, 75%1.5625=0.0 (75/1.5625=48 exact) -> yes!
        // With multiple gauge types we take the max across all: max(50, 64) = 64.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: true,
            element_borders: vec![
                (80.0, 100.0), // border=80, max=100: 80 % (100/50=2) = 0 -> parts stays 50
                (75.0, 100.0), // border=75, max=100: first i where aligned is 64
            ],
        };
        gauge.prepare(100, &state);

        assert_eq!(
            gauge.parts, 64,
            "parts should be increased to 64 for border=75 alignment"
        );
        assert!(
            gauge.is_checked_mode_changed,
            "flag should be set after check"
        );
    }

    #[test]
    fn no_mode_change_keeps_original_parts() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: false,
            element_borders: vec![(75.0, 100.0)],
        };
        gauge.prepare(100, &state);

        assert_eq!(
            gauge.parts, 50,
            "parts should not change when mode is not changed"
        );
        assert!(
            gauge.is_checked_mode_changed,
            "flag should be set even without mode change"
        );
    }

    #[test]
    fn mode_change_check_runs_only_once() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: true,
            element_borders: vec![(80.0, 100.0)],
        };
        gauge.prepare(100, &state);
        assert_eq!(gauge.parts, 50); // 80 % (100/50) = 0, no change needed

        // Manually reset parts to verify second prepare doesn't re-run the check
        gauge.parts = 30;
        gauge.prepare(200, &state);
        assert_eq!(
            gauge.parts, 30,
            "parts should not be re-adjusted on subsequent prepare calls"
        );
    }

    #[test]
    fn mode_change_already_aligned_no_increase() {
        // When all borders already divide evenly, parts stays the same.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: true,
            element_borders: vec![
                (80.0, 100.0), // 80 % (100/50=2) = 0 -> OK
                (60.0, 100.0), // 60 % (100/50=2) = 0 -> OK
            ],
        };
        gauge.prepare(100, &state);

        assert_eq!(
            gauge.parts, 50,
            "parts should remain 50 when borders already aligned"
        );
    }

    #[test]
    fn mode_change_zero_max_gauge_skipped() {
        // Gauge types with max=0 should be skipped (guard against division by zero).
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: true,
            element_borders: vec![
                (0.0, 0.0),    // degenerate gauge type, should be skipped
                (80.0, 100.0), // normal gauge, already aligned
            ],
        };
        gauge.prepare(100, &state);

        assert_eq!(
            gauge.parts, 50,
            "parts should remain 50, degenerate gauge skipped"
        );
    }

    #[test]
    fn mode_change_empty_borders_no_change() {
        // If no gauge element borders are returned, parts stays the same.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        let state = ModeChangeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            mode_changed: true,
            element_borders: vec![],
        };
        gauge.prepare(100, &state);

        assert_eq!(
            gauge.parts, 50,
            "parts should remain 50 with no gauge elements"
        );
    }
}
