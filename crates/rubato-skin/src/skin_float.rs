use crate::float_formatter::FloatFormatter;
use crate::property::float_property::FloatProperty;
use crate::property::float_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::skin_object::{SkinObjectData, SkinObjectRenderer};
use crate::skin_source_image_set::SkinSourceImageSet;
use crate::skin_source_set::SkinSourceSet;
use crate::stubs::{MainState, Rectangle, SkinOffset, TextureRegion};

/// Float number skin object
///
/// Translated from SkinFloat.java
pub struct SkinFloat {
    pub data: SkinObjectData,
    ff: FloatFormatter,
    image: Option<Box<dyn SkinSourceSet>>,
    mimage: Option<Box<dyn SkinSourceSet>>,
    ref_prop: Option<Box<dyn FloatProperty>>,
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
    fn new_base(
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        gain: f32,
    ) -> Self {
        let ff = FloatFormatter::new(iketa, fketa, is_sign_visible, zeropadding);
        let keta = ff.get_keta_length();
        let actual_iketa = ff.get_iketa();
        let actual_fketa = ff.get_fketa();
        let current_images = vec![None; keta as usize];
        Self {
            data: SkinObjectData::new(),
            ff,
            image: None,
            mimage: None,
            ref_prop: None,
            iketa: actual_iketa,
            fketa: actual_fketa,
            is_sign_visible,
            gain,
            keta,
            zeropadding,
            space,
            align,
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
        _timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_base(
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
        // Note: SkinSourceImageSet needs timer cloning which isn't trivial with Box<dyn TimerProperty>
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
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_base(
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
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
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        gain: f32,
    ) -> Self {
        Self::new_with_int_timer_int_id_mimage(
            image,
            None,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            id,
            gain,
        )
    }

    // Constructor with TimerProperty and int id
    pub fn new_with_timer_prop_int_id(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        gain: f32,
    ) -> Self {
        Self::new_with_timer_prop_int_id_mimage(
            image,
            None,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            id,
            gain,
        )
    }

    // Constructor with int timer and FloatProperty
    pub fn new_with_int_timer_float_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: i32,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn FloatProperty>,
        gain: f32,
    ) -> Self {
        Self::new_with_int_timer_float_prop_mimage(
            image,
            None,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            ref_prop,
            gain,
        )
    }

    // Constructor with TimerProperty and FloatProperty
    pub fn new_with_timer_prop_float_prop(
        image: Vec<Vec<Option<TextureRegion>>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn FloatProperty>,
        gain: f32,
    ) -> Self {
        Self::new_with_timer_prop_float_prop_mimage(
            image,
            None,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            ref_prop,
            gain,
        )
    }

    // Constructor with mimage, int timer, int id
    pub fn new_with_int_timer_int_id_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: i32,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_with_images_int_timer(
            image,
            mimage,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
        s.ref_prop = float_property_factory::get_float_property_by_id(id);
        s
    }

    // Constructor with mimage, TimerProperty, int id
    pub fn new_with_timer_prop_int_id_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        id: i32,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_with_images_timer_prop(
            image,
            mimage,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
        s.ref_prop = float_property_factory::get_float_property_by_id(id);
        s
    }

    // Constructor with mimage, int timer, FloatProperty
    pub fn new_with_int_timer_float_prop_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: i32,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn FloatProperty>,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_with_images_int_timer(
            image,
            mimage,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
        s.ref_prop = Some(ref_prop);
        s
    }

    // Constructor with mimage, TimerProperty, FloatProperty
    pub fn new_with_timer_prop_float_prop_mimage(
        image: Vec<Vec<Option<TextureRegion>>>,
        mimage: Option<Vec<Vec<Option<TextureRegion>>>>,
        timer: Option<Box<dyn TimerProperty>>,
        cycle: i32,
        iketa: i32,
        fketa: i32,
        is_sign_visible: bool,
        align: i32,
        zeropadding: i32,
        space: i32,
        ref_prop: Box<dyn FloatProperty>,
        gain: f32,
    ) -> Self {
        let mut s = Self::new_with_images_timer_prop(
            image,
            mimage,
            timer,
            cycle,
            iketa,
            fketa,
            is_sign_visible,
            align,
            zeropadding,
            space,
            gain,
        );
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

        let image = images.as_ref().unwrap().get_images(_time, _state);
        if image.is_none() {
            self.length = 0.0;
            self.draw = false;
            return;
        }
        let image = image.unwrap();

        self.value = v;
        self.image_set = Some(image.clone());
        self.shiftbase = 0;
        let digits = self.ff.calculate_and_get_digits(v.abs() as f64);
        for nowketa in 1..digits.len() {
            self.current_images[nowketa - 1] = if digits[nowketa] != -1 {
                image.get(digits[nowketa] as usize).cloned()
            } else {
                None
            };
            if digits[nowketa] == -1 {
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
        for j in 0..self.current_images.len() {
            if let Some(ref img) = self.current_images[j] {
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

    pub fn get_length(&self) -> f32 {
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
