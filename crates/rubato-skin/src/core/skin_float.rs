use crate::core::float_formatter::FloatFormatter;
use crate::property::float_property::{FloatProperty, FloatPropertyEnum};
use crate::property::float_property_factory;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{MainState, Rectangle, SkinOffset, TextureRegion};
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
    pub region: Rectangle,
    pub draw: bool,
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
            region: Rectangle::default(),
            draw: false,
        }
    }

    fn new_with_images_timer_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        _timer: Option<TimerPropertyEnum>,
        cycle: i32,
        display: FloatDisplayConfig,
    ) -> Self {
        let mut s = Self::new_base(display);
        // Note: SkinSourceImageSet needs timer cloning which isn't trivial with TimerPropertyEnum
        // For now, create without timer
        s.image = Some(Box::new(SkinSourceImageSet::new_with_timer(
            image, None, cycle,
        )));
        if let Some(mimg) = mimage {
            s.mimage = Some(Box::new(SkinSourceImageSet::new_with_timer(
                mimg, None, cycle,
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
        _time: i64,
        _state: &dyn MainState,
        value: f32,
        _offset_x: f32,
        _offset_y: f32,
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
            self.draw = false;
            return;
        }
        let images = if self.mimage.is_none() || v >= 0.0 {
            &self.image
        } else {
            &self.mimage
        };
        if images.is_none() {
            self.length = 0.0;
            self.draw = false;
            return;
        }
        // super.prepare(time, state, offsetX, offsetY) would be called here
        self.draw = true;

        let image = images
            .as_ref()
            .expect("images is Some")
            .get_images(_time, _state);
        if image.is_none() {
            self.length = 0.0;
            self.draw = false;
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

        self.length = (self.region.width + self.space as f32)
            * (self.current_images.len() as i32 - self.shiftbase) as f32;
        self.shift = if self.align == 0 {
            0.0
        } else if self.align == 1 {
            (self.region.width + self.space as f32) * self.shiftbase as f32
        } else {
            (self.region.width + self.space as f32) * 0.5 * self.shiftbase as f32
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
                            self.region.x
                                + (self.region.width + self.space as f32) * j as f32
                                + self.shift
                                + offsets[j].x,
                            self.region.y + offsets[j].y,
                            self.region.width + offsets[j].w,
                            self.region.height + offsets[j].h,
                        );
                    } else {
                        sprite.draw(
                            img,
                            self.region.x
                                + (self.region.width + self.space as f32) * j as f32
                                + self.shift,
                            self.region.y,
                            self.region.width,
                            self.region.height,
                        );
                    }
                } else {
                    sprite.draw(
                        img,
                        self.region.x
                            + (self.region.width + self.space as f32) * j as f32
                            + self.shift,
                        self.region.y,
                        self.region.width,
                        self.region.height,
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
        if self.draw {
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
