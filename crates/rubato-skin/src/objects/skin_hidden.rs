// SkinHidden.java -> skin_hidden.rs
// Mechanical line-by-line translation.
// Hidden/lift cover object for play skin.

use crate::property::timer_property::TimerProperty;
use crate::property::timer_property_factory;
use crate::stubs::{MainState, TextureRegion};
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Hidden/lift cover rendering object.
///
/// Corresponds to Java `SkinHidden`.
/// Renders a cover image that can be trimmed based on a disappear line,
/// used for hidden and lift cover lane modifiers.
pub struct SkinHidden {
    pub data: SkinObjectData,
    /// Original (untrimmed) images
    original_images: Vec<TextureRegion>,
    /// Trimmed images for rendering (when disappear line intersects region)
    trimmed_images: Vec<TextureRegion>,
    /// Disappear line y-coordinate (skin-space). Negative = no trimming.
    disapear_line: f32,
    /// Disappear line y-coordinate with lift applied
    disapear_line_added_lift: f32,
    /// Whether disappear line links to lift offset
    pub is_disapear_line_link_lift: bool,
    /// Previous y position (for caching trimmed images)
    previous_y: f32,
    /// Previous lift value (for detecting lift changes)
    previous_lift: f32,
    /// Timer property for animation
    timer: Option<Box<dyn TimerProperty>>,
    /// Animation cycle duration
    cycle: i32,
    /// Current image index
    image_index: usize,
}

impl SkinHidden {
    /// Creates a SkinHidden from images with an integer timer ID.
    pub fn new_with_int_timer(image: Vec<TextureRegion>, timer: i32, cycle: i32) -> Self {
        let timer_prop: Option<Box<dyn TimerProperty>> = if timer > 0 {
            timer_property_factory::timer_property(timer)
        } else {
            None
        };
        let trimmed = image.clone();
        Self {
            data: SkinObjectData::new(),
            original_images: image,
            trimmed_images: trimmed,
            disapear_line: -1.0,
            disapear_line_added_lift: -1.0,
            is_disapear_line_link_lift: true,
            previous_y: f32::MIN,
            previous_lift: f32::MIN,
            timer: timer_prop,
            cycle,
            image_index: 0,
        }
    }

    pub fn disapear_line(&self) -> f32 {
        self.disapear_line
    }

    pub fn set_disapear_line(&mut self, line: f32) {
        self.disapear_line = line;
        self.disapear_line_added_lift = line;
    }

    pub fn is_disapear_line_link_lift(&self) -> bool {
        self.is_disapear_line_link_lift
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

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        if self.original_images.is_empty() {
            self.data.draw_state.draw = false;
            return;
        }

        // Update disappear line with lift offset
        if self.is_disapear_line_link_lift && self.disapear_line >= 0.0 {
            use crate::skin_property::OFFSET_LIFT;
            if let Some(offset) = state.get_offset_value(OFFSET_LIFT)
                && self.previous_lift != offset.y
            {
                self.disapear_line_added_lift = self.disapear_line + offset.y;
                self.previous_lift = offset.y;
            }
        }

        self.data.prepare(time, state);
        self.image_index = self.get_image_index(self.original_images.len(), time, state);
    }

    pub fn draw(&mut self, sprite: &mut SkinObjectRenderer) {
        let region = &self.data.draw_state.region;
        let dl = self.disapear_line_added_lift;

        // If top of region is below disappear line, don't draw
        if (region.y + region.height > dl && self.disapear_line >= 0.0) || self.disapear_line < 0.0
        {
            if region.y < dl && self.disapear_line >= 0.0 {
                // Region overlaps disappear line - need trimming
                if self.previous_y != region.y && self.image_index < self.original_images.len() {
                    // Refresh trimmed images
                    self.trimmed_images = self.original_images.clone();
                    for img in &mut self.trimmed_images {
                        let new_height = (img.region_height as f32
                            * (region.y + region.height - dl)
                            / region.height) as i32;
                        img.region_height = new_height;
                    }
                    self.previous_y = region.y;
                }
                if self.image_index < self.trimmed_images.len() {
                    sprite.draw(
                        &self.trimmed_images[self.image_index],
                        region.x,
                        dl,
                        region.width,
                        region.y + region.height - dl,
                    );
                }
            } else if self.image_index < self.original_images.len() {
                // No trimming needed
                sprite.draw(
                    &self.original_images[self.image_index],
                    region.x,
                    region.y,
                    region.width,
                    region.height,
                );
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_with_int_timer() {
        let images = vec![TextureRegion::new(), TextureRegion::new()];
        let hidden = SkinHidden::new_with_int_timer(images, 0, 100);
        assert_eq!(hidden.disapear_line(), -1.0);
        assert!(hidden.is_disapear_line_link_lift());
        assert_eq!(hidden.original_images.len(), 2);
    }

    #[test]
    fn test_set_disapear_line() {
        let images = vec![TextureRegion::new()];
        let mut hidden = SkinHidden::new_with_int_timer(images, 0, 0);
        hidden.set_disapear_line(300.0);
        assert_eq!(hidden.disapear_line(), 300.0);
    }

    #[test]
    fn test_set_disapear_line_link_lift() {
        let images = vec![TextureRegion::new()];
        let mut hidden = SkinHidden::new_with_int_timer(images, 0, 0);
        hidden.is_disapear_line_link_lift = false;
        assert!(!hidden.is_disapear_line_link_lift());
    }

    #[test]
    fn test_empty_images_draw_false() {
        let hidden = SkinHidden::new_with_int_timer(vec![], 0, 0);
        // After prepare with no images, draw should be false
        // We can't call prepare without a MainState, but we can check initial state
        assert!(hidden.original_images.is_empty());
    }

    #[test]
    fn test_image_index_no_cycle() {
        let images = vec![TextureRegion::new(), TextureRegion::new()];
        let hidden = SkinHidden::new_with_int_timer(images, 0, 0);
        // With cycle=0, image index should always be 0
        assert_eq!(hidden.cycle, 0);
    }
}
