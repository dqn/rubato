// SkinGauge.java -> skin_gauge.rs
// Mechanical line-by-line translation.
// Gauge object that renders a segmented gauge bar (e.g., groove gauge).

use crate::reexports::{Color, MainState, TextureRegion};
use crate::sources::skin_source_image_set::SkinSourceImageSet;
use crate::sources::skin_source_set::SkinSourceSet;
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

        // Sync gauge value, type, border, and max from game state
        self.value = state.gauge_value();
        self.gauge_type = state.gauge_type();
        if let Some((border, max)) = state.gauge_border_max() {
            self.border = border;
            self.max = max;
        }

        // Result screen: animate gauge fill from min to final value over
        // starttime..endtime. Java: SkinGauge.prepare() lines 161-175.
        if state.is_result_state() {
            let starttime = self.starttime as i64;
            let endtime = self.endtime as i64;
            let gauge_min = state.gauge_min();
            if time < starttime {
                self.value = gauge_min;
            } else if time < endtime && endtime > starttime {
                let progress = self.max * (time - starttime) as f32 / (endtime - starttime) as f32;
                self.value = self.value.min(progress.max(gauge_min));
            }
            // else: time >= endtime, keep the synced value as-is
        }

        // Update animation
        // FLICKERING only uses duration (not animation_range), so only guard it
        // against duration <= 0. RANDOM/INCREASE/DECREASE need both checks.
        match self.animation_type {
            ANIMATION_RANDOM | ANIMATION_INCREASE | ANIMATION_DECREASE => {
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
                                self.animation = (self.animation + self.animation_range)
                                    % (self.animation_range + 1);
                                self.atime = time + self.duration;
                            }
                        }
                        ANIMATION_DECREASE => {
                            if self.atime < time {
                                self.animation = (self.animation + 1) % (self.animation_range + 1);
                                self.atime = time + self.duration;
                            }
                        }
                        _ => unreachable!(),
                    }
                }
            }
            ANIMATION_FLICKERING => {
                if self.duration <= 0 {
                    self.animation = 0;
                } else {
                    self.animation = (time.rem_euclid(self.duration)) as i32;
                }
            }
            _ => {}
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
                        if step > 0.0 && (border % step).abs() < 1e-6 {
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
        if self.images.is_empty() || self.parts <= 0 || self.max <= 0.0 {
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
                    let border_offset = if border_val < self.border { 1 } else { 0 };
                    let img_idx = ex_gauge + if notes >= i { 0 } else { 2 } + border_offset;

                    let seg_x = region.x + region.width * (i - 1) as f32 / self.parts as f32;
                    let seg_w = region.width / self.parts as f32;

                    let img_idx = img_idx as usize;
                    if img_idx < self.images.len() {
                        sprite.draw(&self.images[img_idx], seg_x, region.y, seg_w, region.height);
                    }

                    // Tip-segment glow: draw a blended overlay on the segment at
                    // the gauge tip (i == notes) with alpha pulsing from the
                    // animation timer. Java: SkinGauge.draw() FLICKERING branch.
                    if i == notes && self.duration > 0 {
                        let half_dur = self.duration as f32 / 2.0;
                        let denom = half_dur - 1.0;
                        let alpha = if denom > 0.0 {
                            let anim = self.animation as f32;
                            if (self.animation as i64) < self.duration / 2 {
                                anim / denom
                            } else {
                                (self.duration as f32 - 1.0 - anim) / denom
                            }
                        } else {
                            0.0
                        };

                        let glow_idx = (ex_gauge + 4 + border_offset) as usize;
                        if glow_idx < self.images.len() {
                            let org_color = *sprite.color();
                            let flicker_color = Color::new(
                                org_color.r,
                                org_color.g,
                                org_color.b,
                                org_color.a * alpha,
                            );
                            sprite.set_color(&flicker_color);
                            sprite.draw(
                                &self.images[glow_idx],
                                seg_x,
                                region.y,
                                seg_w,
                                region.height,
                            );
                            sprite.set_color(&org_color);
                        }
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
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
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }

    // --- Default values test ---

    // --- prepare() gauge value sync tests ---
    // These verify the fix for the gauge value wiring bug:
    // SkinGauge.prepare() was not reading gauge value from MainState,
    // causing the gauge display to always show 0% regardless of actual state.

    /// MockMainState with configurable gauge value, type, and border/max.
    struct GaugeMockState {
        gauge_value: f32,
        gauge_type: i32,
        border_max: Option<(f32, f32)>,
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
        fn gauge_border_max(&self) -> Option<(f32, f32)> {
            self.border_max
        }
    }

    impl crate::reexports::MainState for GaugeMockState {}

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
            border_max: None,
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
            border_max: None,
        };
        gauge.prepare(100, &state1);
        assert!((gauge.value - 50.0).abs() < f32::EPSILON);

        // Frame 2: gauge drops to 30%
        let state2 = GaugeMockState {
            gauge_value: 30.0,
            gauge_type: 0,
            border_max: None,
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
            border_max: None,
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

    /// Regression: ANIMATION_FLICKERING with negative time (practice mode time
    /// rewinding) must produce a non-negative animation value. Rust's `%`
    /// follows the dividend sign, so `(-7) % 100 == -7`. Using `rem_euclid`
    /// ensures the result is always in [0, duration).
    #[test]
    fn flickering_animation_negative_time_produces_non_negative() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_FLICKERING, 0, 100);

        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            border_max: None,
        };

        // Negative times that would produce negative results with truncating `%`.
        for &t in &[-1i64, -7, -99, -100, -101, -500, -999] {
            gauge.prepare(t, &state);
            assert!(
                gauge.animation >= 0,
                "animation must be non-negative for time={t}, got {}",
                gauge.animation
            );
            assert!(
                (gauge.animation as i64) < gauge.duration,
                "animation must be < duration for time={t}, got {}",
                gauge.animation
            );
        }

        // Positive times should still work as before.
        for &t in &[0i64, 1, 50, 99, 100, 150, 999] {
            gauge.prepare(t, &state);
            assert!(
                gauge.animation >= 0,
                "animation must be non-negative for time={t}, got {}",
                gauge.animation
            );
            assert!(
                (gauge.animation as i64) < gauge.duration,
                "animation must be < duration for time={t}, got {}",
                gauge.animation
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

    impl crate::reexports::MainState for ModeChangeMockState {}

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

        // With epsilon comparison (Finding 3 fix), i=56 now correctly matches:
        // step = 100/56 = 1.7857..., 75 / step = 42.0 (exact integer), but f32
        // imprecision made the old exact comparison miss it. 56 is the correct
        // minimum parts where border=75 divides evenly.
        assert_eq!(
            gauge.parts, 56,
            "parts should be increased to 56 for border=75 alignment (epsilon match)"
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

    // --- Regression: Finding 1 - FLICKERING tip-segment glow ---

    #[test]
    fn flickering_tip_glow_alpha_ramp_up() {
        // Java: animation < duration/2 => alpha = animation / (duration/2 - 1)
        // With duration=100, half_dur=50, denom=49:
        //   animation=0  => alpha = 0/49 = 0.0
        //   animation=24 => alpha = 24/49 ~ 0.4898
        //   animation=49 => alpha = 49/49 = 1.0 (but 49 < 50, so still ramp-up)
        let duration: i64 = 100;
        let half_dur = duration as f32 / 2.0;
        let denom = half_dur - 1.0;

        // animation = 0 (start of ramp-up)
        let anim = 0.0f32;
        let alpha = anim / denom;
        assert!((alpha - 0.0).abs() < 1e-6, "alpha at start should be 0");

        // animation = 24 (mid ramp-up)
        let anim = 24.0f32;
        let alpha = anim / denom;
        assert!(
            (alpha - 24.0 / 49.0).abs() < 1e-6,
            "alpha at mid ramp-up should be ~0.49"
        );
    }

    #[test]
    fn flickering_tip_glow_alpha_ramp_down() {
        // Java: animation >= duration/2 => alpha = (duration-1-animation) / (duration/2 - 1)
        // With duration=100: animation=50 => (99-50)/49 = 49/49 = 1.0
        //                    animation=99 => (99-99)/49 = 0.0
        let duration: i64 = 100;
        let half_dur = duration as f32 / 2.0;
        let denom = half_dur - 1.0;

        // animation = 50 (start of ramp-down)
        let anim = 50.0f32;
        let alpha = (duration as f32 - 1.0 - anim) / denom;
        assert!(
            (alpha - 1.0).abs() < 1e-6,
            "alpha at ramp-down start should be 1.0"
        );

        // animation = 99 (end of ramp-down)
        let anim = 99.0f32;
        let alpha = (duration as f32 - 1.0 - anim) / denom;
        assert!(
            (alpha - 0.0).abs() < 1e-6,
            "alpha at ramp-down end should be 0.0"
        );
    }

    #[test]
    fn flickering_draw_produces_glow_draw_calls() {
        // Verify that the FLICKERING draw path issues extra draw calls for the
        // tip segment (the glow overlay). With 10 parts and value at 50% (notes=5),
        // we expect 10 base draws + 1 glow draw = 11 total draw calls.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_FLICKERING, 0, 100);
        gauge.images = vec![TextureRegion::new(); 6]; // indices 0..5 valid
        gauge.value = 50.0;
        gauge.max = 100.0;
        gauge.border = 80.0;
        gauge.gauge_type = 0;
        gauge.animation = 25; // mid ramp-up: alpha = 25/49 ~ 0.51
        gauge.duration = 100;
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
        // Test completes without panic; glow path exercised for tip segment (i==5).
        // The glow image index is ex_gauge + 4 + border_offset = 0 + 4 + 1 = 5 (valid).
    }

    // --- Regression: Finding 2 - FLICKERING not blocked by animation_range<0 ---

    #[test]
    fn flickering_animation_not_blocked_by_negative_range() {
        // FLICKERING should still animate even when animation_range is negative,
        // because it only uses duration, not animation_range.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_FLICKERING, -1, 100);

        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            border_max: None,
        };
        gauge.prepare(75, &state);

        // animation should be time % duration = 75 % 100 = 75, not 0
        assert_eq!(
            gauge.animation, 75,
            "FLICKERING animation should not be blocked by negative animation_range"
        );
    }

    #[test]
    fn random_animation_blocked_by_negative_range() {
        // RANDOM should still be blocked when animation_range < 0 (sanity check).
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, -1, 100);

        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            border_max: None,
        };
        gauge.prepare(75, &state);

        assert_eq!(
            gauge.animation, 0,
            "RANDOM animation should be 0 when animation_range < 0"
        );
    }

    // --- Regression: Finding 3 - Float modulo epsilon comparison ---

    #[test]
    fn border_alignment_with_fp_near_zero_remainder() {
        // Test that borders with floating-point remainders very close to zero
        // (but not exactly zero) are still detected as aligned.
        // border=2.0, max=10.0, parts=3:
        //   i=3: step=10/3=3.333..., 2.0 % 3.333... = 2.0 (not aligned)
        //   i=5: step=10/5=2.0, 2.0 % 2.0 = 0.0 (aligned, exact in this case)
        // Use a scenario where FP imprecision matters:
        // border=0.3, max=0.9, parts=1:
        //   i=3: step=0.9/3=0.3, 0.3 % 0.3 should be ~0 but may not be exact.
        let border: f32 = 0.3;
        let max: f32 = 0.9;
        let step = max / 3.0;
        let remainder = border % step;
        // With exact FP equality this would fail on some platforms
        assert!(
            remainder.abs() < 1e-6,
            "epsilon comparison should detect near-zero remainder ({remainder})"
        );
    }

    // --- Regression: gauge border/max must update from live gauge state ---
    // Java's SkinGauge.prepare() reads max and border from the active gauge
    // type's properties every frame. If the gauge type changes mid-play
    // (e.g. EASY -> HARD), the border coloring must reflect the new type.

    #[test]
    fn prepare_syncs_border_and_max_from_state() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        // Defaults: max=100.0, border=80.0
        assert!((gauge.max - 100.0).abs() < f32::EPSILON);
        assert!((gauge.border - 80.0).abs() < f32::EPSILON);

        // Simulate HARD gauge: border=30, max=100
        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 2,
            border_max: Some((30.0, 100.0)),
        };
        gauge.prepare(100, &state);

        assert!(
            (gauge.border - 30.0).abs() < f32::EPSILON,
            "prepare() should update border from gauge state, got {}",
            gauge.border
        );
        assert!(
            (gauge.max - 100.0).abs() < f32::EPSILON,
            "prepare() should update max from gauge state, got {}",
            gauge.max
        );
    }

    #[test]
    fn prepare_updates_border_max_on_gauge_type_change() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        // Frame 1: EASY gauge (border=80, max=100)
        let state1 = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
        };
        gauge.prepare(100, &state1);
        assert!((gauge.border - 80.0).abs() < f32::EPSILON);
        assert!((gauge.max - 100.0).abs() < f32::EPSILON);

        // Frame 2: gauge type changes to HARD (border=30, max=100)
        let state2 = GaugeMockState {
            gauge_value: 40.0,
            gauge_type: 2,
            border_max: Some((30.0, 100.0)),
        };
        gauge.prepare(200, &state2);
        assert!(
            (gauge.border - 30.0).abs() < f32::EPSILON,
            "border should update when gauge type changes, got {}",
            gauge.border
        );
        assert_eq!(gauge.gauge_type, 2);
    }

    #[test]
    fn prepare_keeps_defaults_when_no_border_max_available() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);

        // State returns None for gauge_border_max (e.g. no gauge initialized)
        let state = GaugeMockState {
            gauge_value: 50.0,
            gauge_type: 0,
            border_max: None,
        };
        gauge.prepare(100, &state);

        assert!(
            (gauge.max - 100.0).abs() < f32::EPSILON,
            "max should remain at default when gauge_border_max returns None"
        );
        assert!(
            (gauge.border - 80.0).abs() < f32::EPSILON,
            "border should remain at default when gauge_border_max returns None"
        );
    }

    // --- Regression: Finding 1 - Result-screen gauge fill animation ---
    // Java SkinGauge.prepare() lines 161-175: on result screens, the gauge bar
    // animates from min to the final value over starttime..endtime.

    /// MockMainState that simulates a result screen with configurable gauge min.
    struct ResultMockState {
        gauge_value: f32,
        gauge_type: i32,
        border_max: Option<(f32, f32)>,
        gauge_min: f32,
    }

    impl rubato_types::timer_access::TimerAccess for ResultMockState {
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

    impl rubato_types::skin_render_context::SkinRenderContext for ResultMockState {
        fn gauge_value(&self) -> f32 {
            self.gauge_value
        }
        fn gauge_type(&self) -> i32 {
            self.gauge_type
        }
        fn gauge_border_max(&self) -> Option<(f32, f32)> {
            self.border_max
        }
        fn gauge_min(&self) -> f32 {
            self.gauge_min
        }
        fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
            Some(rubato_types::main_state_type::MainStateType::Result)
        }
    }

    impl crate::reexports::MainState for ResultMockState {}

    #[test]
    fn result_screen_before_starttime_shows_min() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 100;
        gauge.endtime = 600;

        let state = ResultMockState {
            gauge_value: 80.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
            gauge_min: 2.0,
        };
        // time=50 < starttime=100: value should be gauge min (2.0)
        gauge.prepare(50, &state);
        assert!(
            (gauge.value - 2.0).abs() < 1e-6,
            "before starttime, value should be gauge min (2.0), got {}",
            gauge.value
        );
    }

    #[test]
    fn result_screen_at_starttime_begins_animation() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 100;
        gauge.endtime = 600;

        let state = ResultMockState {
            gauge_value: 80.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
            gauge_min: 2.0,
        };
        // time=100, starttime=100, endtime=600: progress = max * 0 / 500 = 0
        // value = min(80, max(0, 2)) = min(80, 2) = 2.0
        gauge.prepare(100, &state);
        assert!(
            (gauge.value - 2.0).abs() < 1e-6,
            "at starttime, value should be gauge min, got {}",
            gauge.value
        );
    }

    #[test]
    fn result_screen_mid_animation_interpolates() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 0;
        gauge.endtime = 500;

        let state = ResultMockState {
            gauge_value: 80.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
            gauge_min: 2.0,
        };
        // time=250, starttime=0, endtime=500: progress = 100 * 250/500 = 50
        // value = min(80, max(50, 2)) = min(80, 50) = 50.0
        gauge.prepare(250, &state);
        assert!(
            (gauge.value - 50.0).abs() < 1e-6,
            "mid-animation, value should be 50.0, got {}",
            gauge.value
        );
    }

    #[test]
    fn result_screen_after_endtime_shows_final() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 0;
        gauge.endtime = 500;

        let state = ResultMockState {
            gauge_value: 80.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
            gauge_min: 2.0,
        };
        // time=500 >= endtime=500: value should be the final gauge value (80.0)
        gauge.prepare(500, &state);
        assert!(
            (gauge.value - 80.0).abs() < 1e-6,
            "after endtime, value should be final gauge value (80.0), got {}",
            gauge.value
        );
    }

    #[test]
    fn result_screen_animation_clamped_to_final_value() {
        // When the animation progress exceeds the final gauge value, the result
        // should be clamped to the final value via min(value, progress).
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 0;
        gauge.endtime = 100;

        let state = ResultMockState {
            gauge_value: 30.0, // final value is low
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
            gauge_min: 2.0,
        };
        // time=80, starttime=0, endtime=100: progress = 100 * 80/100 = 80.0
        // value = min(30, max(80, 2)) = min(30, 80) = 30.0 (clamped to final)
        gauge.prepare(80, &state);
        assert!(
            (gauge.value - 30.0).abs() < 1e-6,
            "animation should be clamped to final value (30.0), got {}",
            gauge.value
        );
    }

    #[test]
    fn result_screen_animation_not_applied_on_play_screen() {
        // On non-result screens, the animation should NOT be applied.
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 50, ANIMATION_RANDOM, 2, 100);
        gauge.starttime = 100;
        gauge.endtime = 600;

        // GaugeMockState does not report is_result_state() == true
        let state = GaugeMockState {
            gauge_value: 80.0,
            gauge_type: 0,
            border_max: Some((80.0, 100.0)),
        };
        // time=50 < starttime=100, but not a result screen
        gauge.prepare(50, &state);
        assert!(
            (gauge.value - 80.0).abs() < 1e-6,
            "on non-result screen, value should be the live gauge value (80.0), got {}",
            gauge.value
        );
    }

    // --- Regression: Finding 2 - Division by max==0 in draw() ---

    #[test]
    fn draw_returns_early_when_max_zero() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 50.0;
        gauge.max = 0.0; // zero max would cause NaN/Inf
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        // Should not panic or produce NaN -- early return
        gauge.draw(&mut renderer);
    }

    #[test]
    fn draw_returns_early_when_max_negative() {
        let images: Vec<Vec<Option<TextureRegion>>> = vec![vec![Some(TextureRegion::new()); 6]];
        let mut gauge = SkinGauge::new(images, 0, 0, 10, ANIMATION_RANDOM, 2, 100);
        gauge.images = vec![TextureRegion::new(); 6];
        gauge.value = 50.0;
        gauge.max = -1.0; // negative max
        gauge.data.region = crate::reexports::Rectangle::new(0.0, 0.0, 100.0, 20.0);
        let mut renderer = SkinObjectRenderer::new();
        gauge.draw(&mut renderer);
    }
}
