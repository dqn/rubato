use crate::core::float_formatter::FloatFormatter;
use crate::property::float_property::{FloatProperty, FloatPropertyEnum};
use crate::property::float_property_factory;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{MainState, SkinOffset, TextureRegion};
use crate::sources::skin_source_image_set::SkinSourceImageSet;
use crate::sources::skin_source_set::SkinSourceSet;
use crate::types::skin_object::{SkinObjectData, SkinObjectRenderer};

/// Display configuration for float number rendering.
pub struct FloatDisplayConfig {
    pub iketa: i32,
    pub fketa: i32,
    pub is_sign_visible: bool,
    pub align: i32,
    pub zeropadding: i32,
    pub space: i32,
    pub gain: f32,
}

/// Float number skin object
///
/// Translated from SkinFloat.java
pub struct SkinFloat {
    pub data: SkinObjectData,
    ff: FloatFormatter,
    image: Option<Box<dyn SkinSourceSet>>,
    mimage: Option<Box<dyn SkinSourceSet>>,
    ref_prop: Option<FloatPropertyEnum>,
    pub iketa: i32,
    pub fketa: i32,
    pub is_sign_visible: bool,
    pub gain: f32,
    keta: i32,
    pub zeropadding: i32,
    space: i32,
    align: i32,
    value: f32,
    shiftbase: i32,
    offsets: Option<Vec<SkinOffset>>,
    length: f32,
    current_images: Vec<Option<TextureRegion>>,
    image_set: Option<Vec<TextureRegion>>,
    shift: f32,
    // Design note: region and draw live on self.data (SkinObjectData).
    // SkinFloat previously had redundant fields here; removed to avoid
    // divergence from data.region / data.draw computed by prepare().
}

impl SkinFloat {
    fn new_base(display: FloatDisplayConfig) -> Self {
        let ff = FloatFormatter::new(
            display.iketa,
            display.fketa,
            display.is_sign_visible,
            display.zeropadding,
        );
        let keta = ff.keta_length();
        let actual_iketa = ff.iketa();
        let actual_fketa = ff.fketa();
        let current_images = vec![None; keta as usize];
        Self {
            data: SkinObjectData::new(),
            ff,
            image: None,
            mimage: None,
            ref_prop: None,
            iketa: actual_iketa,
            fketa: actual_fketa,
            is_sign_visible: display.is_sign_visible,
            gain: display.gain,
            keta,
            zeropadding: display.zeropadding,
            space: display.space,
            align: display.align,
            value: f32::MIN,
            shiftbase: 0,
            offsets: None,
            length: 0.0,
            current_images,
            image_set: None,
            shift: 0.0,
        }
    }

    fn new_with_images_timer_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
    ) -> Self {
        let mut s = Self::new_base(display);
        s.image = Some(Box::new(SkinSourceImageSet::new_with_timer(
            image,
            timer.clone(),
            cycle,
        )));
        if let Some(mimg) = mimage {
            s.mimage = Some(Box::new(SkinSourceImageSet::new_with_timer(
                mimg, timer, cycle,
            )));
        }
        s
    }

    fn new_with_images_int_timer(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: i32,
        cycle: i32,
        display: FloatDisplayConfig,
    ) -> Self {
        let mut s = Self::new_base(display);
        s.image = Some(Box::new(SkinSourceImageSet::new_with_int_timer(
            image, timer, cycle,
        )));
        if let Some(mimg) = mimage {
            s.mimage = Some(Box::new(SkinSourceImageSet::new_with_int_timer(
                mimg, timer, cycle,
            )));
        }
        s
    }

    // Constructor with int timer and int id
    pub fn new_with_int_timer_int_id(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        display: FloatDisplayConfig,
        id: i32,
    ) -> Self {
        Self::new_with_int_timer_int_id_mimage(image, None, timer, cycle, display, id)
    }

    // Constructor with TimerProperty and int id
    pub fn new_with_timer_prop_int_id(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
        id: i32,
    ) -> Self {
        Self::new_with_timer_prop_int_id_mimage(image, None, timer, cycle, display, id)
    }

    // Constructor with int timer and FloatProperty
    pub fn new_with_int_timer_float_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        display: FloatDisplayConfig,
        ref_prop: FloatPropertyEnum,
    ) -> Self {
        Self::new_with_int_timer_float_prop_mimage(image, None, timer, cycle, display, ref_prop)
    }

    // Constructor with TimerProperty and FloatProperty
    pub fn new_with_timer_prop_float_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
        ref_prop: FloatPropertyEnum,
    ) -> Self {
        Self::new_with_timer_prop_float_prop_mimage(image, None, timer, cycle, display, ref_prop)
    }

    // Constructor with mimage, int timer, int id
    pub fn new_with_int_timer_int_id_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: i32,
        cycle: i32,
        display: FloatDisplayConfig,
        id: i32,
    ) -> Self {
        let mut s = Self::new_with_images_int_timer(image, mimage, timer, cycle, display);
        s.ref_prop = float_property_factory::float_property_by_id(id);
        s
    }

    // Constructor with mimage, TimerProperty, int id
    pub fn new_with_timer_prop_int_id_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
        id: i32,
    ) -> Self {
        let mut s = Self::new_with_images_timer_prop(image, mimage, timer, cycle, display);
        s.ref_prop = float_property_factory::float_property_by_id(id);
        s
    }

    // Constructor with mimage, int timer, FloatProperty
    pub fn new_with_int_timer_float_prop_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: i32,
        cycle: i32,
        display: FloatDisplayConfig,
        ref_prop: FloatPropertyEnum,
    ) -> Self {
        let mut s = Self::new_with_images_int_timer(image, mimage, timer, cycle, display);
        s.ref_prop = Some(ref_prop);
        s
    }

    // Constructor with mimage, TimerProperty, FloatProperty
    pub fn new_with_timer_prop_float_prop_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
        ref_prop: FloatPropertyEnum,
    ) -> Self {
        let mut s = Self::new_with_images_timer_prop(image, mimage, timer, cycle, display);
        s.ref_prop = Some(ref_prop);
        s
    }

    pub fn set_offsets(&mut self, offsets: Vec<SkinOffset>) {
        self.offsets = Some(offsets);
    }

    /// Prepare for rendering (enum dispatch entry point).
    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_offset(time, state, 0.0, 0.0);
    }

    pub fn prepare_simple(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_offset(time, state, 0.0, 0.0);
    }

    pub fn prepare_with_offset(
        &mut self,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        let value = if let Some(ref ref_prop) = self.ref_prop {
            ref_prop.get(state)
        } else {
            f32::MIN
        };
        self.prepare_with_value(time, state, value, offset_x, offset_y);
    }

    pub fn prepare_with_value(
        &mut self,
        time: i64,
        state: &dyn MainState,
        value: f32,
        offset_x: f32,
        offset_y: f32,
    ) {
        let v = value * self.gain;
        if value == f32::MIN
            || value == f32::MAX
            || v.is_infinite()
            || v.is_nan()
            || v == f32::MIN
            || v == f32::MAX
            || self.keta == 0
        {
            self.length = 0.0;
            self.data.draw = false;
            return;
        }
        let images = if self.mimage.is_none() || v >= 0.0 {
            &self.image
        } else {
            &self.mimage
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

        let image = images
            .as_ref()
            .expect("images is Some")
            .get_images(time, state);
        if image.is_none() {
            self.length = 0.0;
            self.data.draw = false;
            return;
        }
        let image = image.expect("image");

        self.value = v;
        self.image_set = Some(image.clone());
        self.shiftbase = 0;
        let digits = self.ff.calculate_and_get_digits(v.abs() as f64);
        for (idx, &digit) in digits[1..].iter().enumerate() {
            self.current_images[idx] = if digit != -1 {
                image.get(digit as usize).cloned()
            } else {
                None
            };
            if digit == -1 {
                self.shiftbase += 1;
            }
        }

        self.length = (self.data.region.width + self.space as f32)
            * (self.current_images.len() as i32 - self.shiftbase) as f32;
        self.shift = if self.align == 0 {
            0.0
        } else if self.align == 1 {
            (self.data.region.width + self.space as f32) * self.shiftbase as f32
        } else {
            (self.data.region.width + self.space as f32) * 0.5 * self.shiftbase as f32
        };
    }

    /// Draw the float number.
    /// Corresponds to Java SkinFloat.draw(SkinObjectRenderer sprite)
    pub fn draw(&self, sprite: &mut SkinObjectRenderer) {
        for (j, current_img) in self.current_images.iter().enumerate() {
            if let Some(img) = current_img {
                if let Some(ref offsets) = self.offsets {
                    if j < offsets.len() {
                        sprite.draw(
                            img,
                            self.data.region.x
                                + (self.data.region.width + self.space as f32) * j as f32
                                + self.shift
                                + offsets[j].x,
                            self.data.region.y + offsets[j].y,
                            self.data.region.width + offsets[j].w,
                            self.data.region.height + offsets[j].h,
                        );
                    } else {
                        sprite.draw(
                            img,
                            self.data.region.x
                                + (self.data.region.width + self.space as f32) * j as f32
                                + self.shift,
                            self.data.region.y,
                            self.data.region.width,
                            self.data.region.height,
                        );
                    }
                } else {
                    sprite.draw(
                        img,
                        self.data.region.x
                            + (self.data.region.width + self.space as f32) * j as f32
                            + self.shift,
                        self.data.region.y,
                        self.data.region.width,
                        self.data.region.height,
                    );
                }
            }
        }
    }

    /// Draw with value and state.
    /// Corresponds to Java SkinFloat.draw(SkinObjectRenderer, long, float, MainState, float, float)
    pub fn draw_with_value(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        time: i64,
        value: f32,
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

    pub fn dispose(&mut self) {
        if let Some(ref mut image) = self.image {
            image.dispose();
        }
        self.image = None;
        if let Some(ref mut mimage) = self.mimage {
            mimage.dispose();
        }
        self.mimage = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::skin_object::DestinationParams;
    use crate::test_helpers::MockMainState;

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

    /// Helper: create digit images for SkinFloat (12 entries: 0-9, space, minus).
    fn make_float_images() -> Vec<Vec<Option<TextureRegion>>> {
        let digits: Vec<Option<TextureRegion>> =
            (0..12).map(|_| Some(make_region(24, 32))).collect();
        vec![digits]
    }

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    fn setup_data(data: &mut crate::skin_object::SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            &DestinationParams {
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
    fn test_skin_float_prepare_sets_data_draw_true() {
        // Bug 1: prepare_with_value must call self.data.prepare_with_offset()
        // so that self.data.draw becomes true and self.data.region is computed.
        let display = FloatDisplayConfig {
            iketa: 3,
            fketa: 2,
            is_sign_visible: false,
            align: 0,
            zeropadding: 0,
            space: 0,
            gain: 1.0,
        };
        let mut sf = SkinFloat::new_with_images_int_timer(make_float_images(), None, 0, 0, display);
        setup_data(&mut sf.data, 10.0, 20.0, 24.0, 32.0);

        let state = MockMainState::default();
        sf.prepare_with_value(0, &state, 3.14, 0.0, 0.0);

        // After prepare, data.draw must be true (SkinObjectData was prepared)
        assert!(
            sf.data.draw,
            "SkinObjectData.draw should be true after prepare_with_value"
        );
        // data.region should have been computed from the destination
        assert!(
            sf.data.region.width > 0.0,
            "SkinObjectData.region.width should be set"
        );
    }
}
