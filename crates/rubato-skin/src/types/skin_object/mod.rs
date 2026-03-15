// SkinObject.java -> skin_object.rs
// Mechanical line-by-line translation.

use crate::core::stretch_type::StretchType;
use crate::property::boolean_property::BooleanProperty;
use crate::property::event::Event;
use crate::property::event_factory;
use crate::property::float_property::FloatProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{Color, MainState, Rectangle, SkinOffset, TextureRegion};

mod destination;
mod draw;
mod prepare;

/// SkinObjectRenderer (inner class of Skin, but used by all SkinObject draw calls)
mod renderer;
pub use renderer::*;

/// Parameters for drawing an image at a specific position with color and rotation.
pub struct DrawImageAtParams<'a> {
    pub image: &'a TextureRegion,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub color: &'a Color,
    pub angle: i32,
}

/// SkinObjectDestination (inner class of SkinObject)
#[derive(Clone, Debug)]
pub struct SkinObjectDestination {
    pub time: i64,
    pub region: Rectangle,
    pub acc: i32,
    pub color: Color,
    pub angle: i32,
}

impl SkinObjectDestination {
    pub fn new(time: i64, region: Rectangle, color: Color, angle: i32, acc: i32) -> Self {
        Self {
            time,
            region,
            acc,
            color,
            angle,
        }
    }
}

/// RateProperty: IntegerProperty -> min-max ratio as FloatProperty
pub struct RateProperty {
    ref_prop: Option<Box<dyn crate::property::integer_property::IntegerProperty>>,
    min: i32,
    max: i32,
}

impl RateProperty {
    pub fn new(type_id: i32, min: i32, max: i32) -> Self {
        Self {
            ref_prop: integer_property_factory::integer_property_by_id(type_id),
            min,
            max,
        }
    }
}

impl FloatProperty for RateProperty {
    fn get(&self, state: &dyn MainState) -> f32 {
        let value = if let Some(ref r) = self.ref_prop {
            r.get(state)
        } else {
            0
        };
        if self.min == self.max {
            0.0
        } else if self.min < self.max {
            if value > self.max {
                1.0
            } else if value < self.min {
                0.0
            } else {
                ((value as f32 - self.min as f32) / (self.max as f32 - self.min as f32)).abs()
            }
        } else if value < self.max {
            1.0
        } else if value > self.min {
            0.0
        } else {
            ((value as f32 - self.min as f32) / (self.max as f32 - self.min as f32)).abs()
        }
    }
}

/// Parameters for skin object destination (position, color, blend, etc.)
pub struct DestinationParams {
    pub time: i64,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub acc: i32,
    pub a: i32,
    pub r: i32,
    pub g: i32,
    pub b: i32,
    pub blend: i32,
    pub filter: i32,
    pub angle: i32,
    pub center: i32,
    pub loop_val: i32,
}

/// Shared data for all SkinObject types.
pub struct SkinObjectData {
    pub offset: Vec<i32>,
    pub relative: bool,
    pub dsttimer: Option<TimerPropertyEnum>,
    pub dstloop: i32,
    pub dstblend: i32,
    pub dstfilter: i32,
    pub image_type: i32,
    pub dstcenter: i32,
    pub acc: i32,
    pub clickevent: Option<Box<dyn Event>>,
    pub clickevent_type: i32,
    pub dstop: Vec<i32>,
    pub dstdraw: Vec<Box<dyn BooleanProperty>>,
    pub mouse_rect: Option<Rectangle>,
    pub stretch: StretchType,
    pub centerx: f32,
    pub centery: f32,
    pub dst: Vec<SkinObjectDestination>,
    pub name: Option<String>,

    // optimization fields
    pub starttime: i64,
    pub endtime: i64,

    pub draw: bool,
    pub visible: bool,
    pub region: Rectangle,
    pub color: Color,
    pub angle: i32,
    pub off: Vec<Option<SkinOffset>>,

    pub fixr: Option<Rectangle>,
    pub fixc: Option<Color>,
    pub fixa: i32,

    pub nowtime: i64,
    pub rate: f32,
    pub index: i32,

    pub tmp_rect: Rectangle,
    pub tmp_image: TextureRegion,

    pub disposed: bool,
}

pub(crate) static CENTERX: [f32; 10] = [0.5, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
pub(crate) static CENTERY: [f32; 10] = [0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0];

impl Default for SkinObjectData {
    fn default() -> Self {
        Self {
            offset: Vec::new(),
            relative: false,
            dsttimer: None,
            dstloop: 0,
            dstblend: 0,
            dstfilter: 0,
            image_type: 0,
            dstcenter: 0,
            acc: 0,
            clickevent: None,
            clickevent_type: 0,
            dstop: Vec::new(),
            dstdraw: Vec::new(),
            mouse_rect: None,
            stretch: StretchType::Stretch,
            centerx: 0.0,
            centery: 0.0,
            dst: Vec::new(),
            name: None,
            starttime: 0,
            endtime: 0,
            draw: false,
            visible: true,
            region: Rectangle::default(),
            color: Color::default(),
            angle: 0,
            off: Vec::new(),
            fixr: None,
            fixc: None,
            fixa: i32::MIN,
            nowtime: 0,
            rate: 0.0,
            index: 0,
            tmp_rect: Rectangle::default(),
            tmp_image: TextureRegion::new(),
            disposed: false,
        }
    }
}

/// Accessors, interaction, and simple utility methods.
impl SkinObjectData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all_destination(&self) -> &[SkinObjectDestination] {
        &self.dst
    }

    pub fn draw_condition(&self) -> &[Box<dyn BooleanProperty>] {
        &self.dstdraw
    }

    pub fn option(&self) -> &[i32] {
        &self.dstop
    }

    pub fn set_stretch_by_id(&mut self, stretch: i32) {
        if stretch < 0 {
            return;
        }
        for st in StretchType::values() {
            if st.id() == stretch {
                self.stretch = *st;
                return;
            }
        }
    }

    pub fn stretch(&self) -> StretchType {
        self.stretch
    }

    pub fn blend(&self) -> i32 {
        self.dstblend
    }

    pub fn destination(&self, _time: i64, _state: &dyn MainState) -> Option<&Rectangle> {
        if self.draw { Some(&self.region) } else { None }
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn validate(&self) -> bool {
        !self.dst.is_empty()
    }

    pub fn load(&mut self) {
        // no-op by default
    }

    pub fn mouse_pressed(&self, state: &mut dyn MainState, button: i32, x: i32, y: i32) -> bool {
        if let Some(ref clickevent) = self.clickevent {
            let r = &self.region;
            let button_events: [i32; 5] = [1, -1, 1, 1, -1];
            let inc = if button >= 0 && (button as usize) < button_events.len() {
                button_events[button as usize]
            } else {
                0
            };
            match self.clickevent_type {
                0 => {
                    if r.x <= x as f32
                        && r.x + r.width >= x as f32
                        && r.y <= y as f32
                        && r.y + r.height >= y as f32
                    {
                        clickevent.exec(state, inc, 0);
                        return true;
                    }
                }
                1 => {
                    if r.x <= x as f32
                        && r.x + r.width >= x as f32
                        && r.y <= y as f32
                        && r.y + r.height >= y as f32
                    {
                        clickevent.exec(state, -inc, 0);
                        return true;
                    }
                }
                2 => {
                    if r.x <= x as f32
                        && r.x + r.width >= x as f32
                        && r.y <= y as f32
                        && r.y + r.height >= y as f32
                    {
                        clickevent.exec(
                            state,
                            if x as f32 >= r.x + r.width / 2.0 {
                                1
                            } else {
                                -1
                            },
                            0,
                        );
                        return true;
                    }
                }
                3 => {
                    if r.x <= x as f32
                        && r.x + r.width >= x as f32
                        && r.y <= y as f32
                        && r.y + r.height >= y as f32
                    {
                        clickevent.exec(
                            state,
                            if y as f32 >= r.y + r.height / 2.0 {
                                1
                            } else {
                                -1
                            },
                            0,
                        );
                        return true;
                    }
                }
                _ => {}
            }
        }
        false
    }

    pub fn clickevent_id(&self) -> i32 {
        self.clickevent
            .as_ref()
            .map(|e| e.get_event_id().as_i32())
            .unwrap_or(0)
    }

    pub fn clickevent(&self) -> Option<&dyn Event> {
        self.clickevent.as_deref()
    }

    pub fn set_clickevent_by_id(&mut self, clickevent: i32) {
        self.clickevent = event_factory::event_by_id(clickevent);
    }

    pub fn set_clickevent(&mut self, clickevent: Box<dyn Event>) {
        self.clickevent = Some(clickevent);
    }

    pub fn clickevent_type(&self) -> i32 {
        self.clickevent_type
    }

    pub fn is_relative(&self) -> bool {
        self.relative
    }

    pub fn offset_id(&self) -> &[i32] {
        &self.offset
    }

    pub fn offsets(&self) -> &[Option<SkinOffset>] {
        &self.off
    }

    pub fn destination_timer(&self) -> Option<&TimerPropertyEnum> {
        self.dsttimer.as_ref()
    }

    pub fn image_type(&self) -> i32 {
        self.image_type
    }

    pub fn filter(&self) -> i32 {
        self.dstfilter
    }

    pub fn set_mouse_rect(&mut self, x2: f32, y2: f32, w2: f32, h2: f32) {
        self.mouse_rect = Some(Rectangle::new(x2, y2, w2, h2));
    }

    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn is_disposed(&self) -> bool {
        self.disposed
    }

    pub fn set_disposed(&mut self) {
        self.disposed = true;
    }
}

#[cfg(test)]
mod tests;
