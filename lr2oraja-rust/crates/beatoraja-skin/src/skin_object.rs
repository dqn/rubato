// SkinObject.java -> skin_object.rs
// Mechanical line-by-line translation.

use std::collections::HashSet;

use crate::property::boolean_property::BooleanProperty;
use crate::property::boolean_property_factory;
use crate::property::event::Event;
use crate::property::event_factory;
use crate::property::float_property::FloatProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::skin_property;
use crate::stretch_type::StretchType;
use crate::stubs::{Color, MainState, Rectangle, SkinOffset, SpriteBatch, TextureRegion};

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
            ref_prop: integer_property_factory::get_integer_property_by_id(type_id),
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
        if self.min < self.max {
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

/// Shared data for all SkinObject types.
pub struct SkinObjectData {
    pub offset: Vec<i32>,
    pub relative: bool,
    pub dsttimer: Option<Box<dyn TimerProperty>>,
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

static CENTERX: [f32; 10] = [0.5, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
static CENTERY: [f32; 10] = [0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0];

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

impl SkinObjectData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn get_all_destination(&self) -> &[SkinObjectDestination] {
        &self.dst
    }

    pub fn set_destination_with_int_timer_and_single_offset(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: i32,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: i32,
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        self.set_destination_with_timer_and_ops(
            time,
            x,
            y,
            w,
            h,
            acc,
            a,
            r,
            g,
            b,
            blend,
            filter,
            angle,
            center,
            loop_val,
            timer_prop,
            &[op1, op2, op3],
        );
        self.set_offset_id_single(offset);
    }

    pub fn set_destination_with_int_timer_and_offsets(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: i32,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: &[i32],
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        self.set_destination_with_timer_and_ops(
            time,
            x,
            y,
            w,
            h,
            acc,
            a,
            r,
            g,
            b,
            blend,
            filter,
            angle,
            center,
            loop_val,
            timer_prop,
            &[op1, op2, op3],
        );
        self.set_offset_id(offset);
    }

    pub fn set_destination_with_int_timer_ops(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: i32,
        op: &[i32],
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer_prop,
        );
        if self.dstop.is_empty() && self.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    pub fn set_destination_with_int_timer_draw(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: i32,
        draw_prop: Box<dyn BooleanProperty>,
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::get_timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer_prop,
        );
        self.dstdraw = vec![draw_prop];
    }

    pub fn set_destination_with_timer_ops_and_single_offset(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: i32,
    ) {
        self.set_destination_with_timer_and_ops(
            time,
            x,
            y,
            w,
            h,
            acc,
            a,
            r,
            g,
            b,
            blend,
            filter,
            angle,
            center,
            loop_val,
            timer,
            &[op1, op2, op3],
        );
        self.set_offset_id_single(offset);
    }

    pub fn set_destination_with_timer_ops_and_offsets(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        op1: i32,
        op2: i32,
        op3: i32,
        offset: &[i32],
    ) {
        self.set_destination_with_timer_and_ops(
            time,
            x,
            y,
            w,
            h,
            acc,
            a,
            r,
            g,
            b,
            blend,
            filter,
            angle,
            center,
            loop_val,
            timer,
            &[op1, op2, op3],
        );
        self.set_offset_id(offset);
    }

    pub fn set_destination_with_timer_and_ops(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        op: &[i32],
    ) {
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer,
        );
        if self.dstop.is_empty() && self.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    pub fn set_destination_with_timer_draw(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
        draw_prop: Box<dyn BooleanProperty>,
    ) {
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer,
        );
        self.dstdraw = vec![draw_prop];
    }

    fn set_destination_core(
        &mut self,
        time: i64,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        acc: i32,
        a: i32,
        r: i32,
        g: i32,
        b: i32,
        blend: i32,
        filter: i32,
        angle: i32,
        center: i32,
        loop_val: i32,
        timer: Option<Box<dyn TimerProperty>>,
    ) {
        let obj = SkinObjectDestination::new(
            time,
            Rectangle::new(x, y, w, h),
            Color::new(
                r as f32 / 255.0,
                g as f32 / 255.0,
                b as f32 / 255.0,
                a as f32 / 255.0,
            ),
            angle,
            acc,
        );
        if self.dst.is_empty() {
            self.fixr = Some(obj.region.clone());
            self.fixc = Some(obj.color.clone());
            self.fixa = obj.angle;
        } else {
            if let Some(ref fixr) = self.fixr
                && !obj.region.equals(fixr)
            {
                self.fixr = None;
            }
            if let Some(ref fixc) = self.fixc
                && !obj.color.equals(fixc)
            {
                self.fixc = None;
            }
            if self.fixa != obj.angle {
                self.fixa = i32::MIN;
            }
        }
        if self.acc == 0 {
            self.acc = acc;
        }
        if self.dstblend == 0 {
            self.dstblend = blend;
        }
        if self.dstfilter == 0 {
            self.dstfilter = filter;
        }
        if self.dstcenter == 0 && (0..10).contains(&center) {
            self.dstcenter = center;
            self.centerx = CENTERX[center as usize];
            self.centery = CENTERY[center as usize];
        }
        if self.dsttimer.is_none() {
            self.dsttimer = timer;
        }
        if self.dstloop == 0 {
            self.dstloop = loop_val;
        }
        for i in 0..self.dst.len() {
            if self.dst[i].time > time {
                self.dst.insert(i, obj);
                self.starttime = self.dst[0].time;
                self.endtime = self.dst[self.dst.len() - 1].time;
                return;
            }
        }
        self.dst.push(obj);
        self.starttime = self.dst[0].time;
        self.endtime = self.dst[self.dst.len() - 1].time;
    }

    pub fn get_draw_condition(&self) -> &[Box<dyn BooleanProperty>] {
        &self.dstdraw
    }

    pub fn get_option(&self) -> &[i32] {
        &self.dstop
    }

    pub fn set_option(&mut self, dstop: Vec<i32>) {
        self.dstop = dstop;
    }

    pub fn set_draw_condition_from_ops(&mut self, dstop: &[i32]) {
        let mut seen = HashSet::new();
        let mut op = Vec::new();
        let mut draw: Vec<Box<dyn BooleanProperty>> = Vec::new();
        for &i in dstop {
            if i != 0 && !seen.contains(&i) {
                if let Some(dc) = boolean_property_factory::get_boolean_property(i) {
                    draw.push(dc);
                } else {
                    op.push(i);
                }
                seen.insert(i);
            }
        }
        self.dstop = op;
        self.dstdraw = draw;
    }

    pub fn set_draw_condition(&mut self, dstdraw: Vec<Box<dyn BooleanProperty>>) {
        self.dstdraw = dstdraw;
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

    pub fn set_stretch(&mut self, stretch: StretchType) {
        self.stretch = stretch;
    }

    pub fn get_stretch(&self) -> StretchType {
        self.stretch
    }

    pub fn get_blend(&self) -> i32 {
        self.dstblend
    }

    pub fn prepare_region(&mut self, time: i64, state: Option<&dyn MainState>) {
        let mut time = time;

        if let Some(ref timer) = self.dsttimer
            && let Some(s) = state
        {
            if timer.is_off(s) {
                self.draw = false;
                return;
            }
            time -= timer.get(s);
        }

        let lasttime = self.endtime;
        if self.dstloop == -1 {
            if time > self.endtime {
                time = -1;
            }
        } else if lasttime > 0 && time > self.dstloop as i64 {
            if lasttime == self.dstloop as i64 {
                time = self.dstloop as i64;
            } else {
                time = (time - self.dstloop as i64) % (lasttime - self.dstloop as i64)
                    + self.dstloop as i64;
            }
        }
        if self.starttime > time {
            self.draw = false;
            return;
        }
        self.nowtime = time;
        self.rate = -1.0;
        self.index = -1;
        for i in 0..self.off.len() {
            self.off[i] = if let Some(s) = state {
                s.get_offset_value(self.offset[i]).cloned()
            } else {
                None
            };
        }

        if self.fixr.is_none() {
            self.get_rate();
            if self.rate == 0.0 {
                let idx = self.index as usize;
                self.region.set(&self.dst[idx].region);
            } else if self.acc == 3 {
                let idx = self.index as usize;
                let r1 = &self.dst[idx].region;
                self.region.x = r1.x;
                self.region.y = r1.y;
                self.region.width = r1.width;
                self.region.height = r1.height;
            } else {
                let idx = self.index as usize;
                let rate = self.rate;
                let r1x = self.dst[idx].region.x;
                let r1y = self.dst[idx].region.y;
                let r1w = self.dst[idx].region.width;
                let r1h = self.dst[idx].region.height;
                let r2x = self.dst[idx + 1].region.x;
                let r2y = self.dst[idx + 1].region.y;
                let r2w = self.dst[idx + 1].region.width;
                let r2h = self.dst[idx + 1].region.height;
                self.region.x = r1x + (r2x - r1x) * rate;
                self.region.y = r1y + (r2y - r1y) * rate;
                self.region.width = r1w + (r2w - r1w) * rate;
                self.region.height = r1h + (r2h - r1h) * rate;
            }

            for off in self.off.iter().flatten() {
                if !self.relative {
                    self.region.x += off.x - off.w / 2.0;
                    self.region.y += off.y - off.h / 2.0;
                }
                self.region.width += off.w;
                self.region.height += off.h;
            }
        } else if let Some(ref fixr) = self.fixr {
            if self.offset.is_empty() {
                self.region.set(fixr);
                return;
            }
            self.region.set(fixr);
            for off in self.off.iter().flatten() {
                if !self.relative {
                    self.region.x += off.x - off.w / 2.0;
                    self.region.y += off.y - off.h / 2.0;
                }
                self.region.width += off.w;
                self.region.height += off.h;
            }
        }
    }

    pub fn get_destination(&self, _time: i64, _state: &dyn MainState) -> Option<&Rectangle> {
        if self.draw { Some(&self.region) } else { None }
    }

    fn prepare_color(&mut self) {
        if let Some(ref fixc) = self.fixc.clone() {
            self.color.set(fixc);
            for off in self.off.iter().flatten() {
                let mut a = self.color.a + (off.a / 255.0);
                a = if a > 1.0 {
                    1.0
                } else if a < 0.0 {
                    0.0
                } else {
                    a
                };
                self.color.a = a;
            }
            return;
        }
        self.get_rate();
        if self.rate == 0.0 {
            let idx = self.index as usize;
            let c = self.dst[idx].color.clone();
            self.color.set(&c);
        } else if self.acc == 3 {
            let idx = self.index as usize;
            self.color.r = self.dst[idx].color.r;
            self.color.g = self.dst[idx].color.g;
            self.color.b = self.dst[idx].color.b;
            self.color.a = self.dst[idx].color.a;
            return;
        } else {
            let idx = self.index as usize;
            let rate = self.rate;
            let r1r = self.dst[idx].color.r;
            let r1g = self.dst[idx].color.g;
            let r1b = self.dst[idx].color.b;
            let r1a = self.dst[idx].color.a;
            let r2r = self.dst[idx + 1].color.r;
            let r2g = self.dst[idx + 1].color.g;
            let r2b = self.dst[idx + 1].color.b;
            let r2a = self.dst[idx + 1].color.a;
            self.color.r = r1r + (r2r - r1r) * rate;
            self.color.g = r1g + (r2g - r1g) * rate;
            self.color.b = r1b + (r2b - r1b) * rate;
            self.color.a = r1a + (r2a - r1a) * rate;
            return;
        }
        for off in self.off.iter().flatten() {
            let mut a = self.color.a + (off.a / 255.0);
            a = if a > 1.0 {
                1.0
            } else if a < 0.0 {
                0.0
            } else {
                a
            };
            self.color.a = a;
        }
    }

    pub fn get_color(&self) -> &Color {
        &self.color
    }

    fn prepare_angle(&mut self) {
        if self.fixa != i32::MIN {
            self.angle = self.fixa;
            for off in self.off.iter().flatten() {
                self.angle += off.r as i32;
            }
            return;
        }
        self.get_rate();
        let idx = self.index as usize;
        self.angle = if self.rate == 0.0 || self.acc == 3 {
            self.dst[idx].angle
        } else {
            (self.dst[idx].angle as f32
                + (self.dst[idx + 1].angle - self.dst[idx].angle) as f32 * self.rate)
                as i32
        };
        for off in self.off.iter().flatten() {
            self.angle += off.r as i32;
        }
    }

    fn get_rate(&mut self) {
        if self.rate != -1.0 {
            return;
        }
        let mut time2 = self.dst[self.dst.len() - 1].time;
        if self.nowtime == time2 {
            self.rate = 0.0;
            self.index = self.dst.len() as i32 - 1;
            return;
        }
        for i in (0..=(self.dst.len() as i32 - 2)).rev() {
            let i = i as usize;
            let time1 = self.dst[i].time;
            if time1 <= self.nowtime && time2 > self.nowtime {
                let mut rate = (self.nowtime - time1) as f32 / (time2 - time1) as f32;
                match self.acc {
                    1 => {
                        rate = rate * rate;
                    }
                    2 => {
                        rate = 1.0 - (rate - 1.0) * (rate - 1.0);
                    }
                    _ => {}
                }
                self.rate = rate;
                self.index = i as i32;
                return;
            }
            time2 = time1;
        }
        self.rate = 0.0;
        self.index = 0;
    }

    pub fn validate(&self) -> bool {
        !self.dst.is_empty()
    }

    pub fn load(&mut self) {
        // no-op by default
    }

    pub fn prepare(&mut self, time: i64, state: &dyn MainState) {
        self.prepare_with_offset(time, state, 0.0, 0.0);
    }

    pub fn prepare_with_offset(
        &mut self,
        time: i64,
        state: &dyn MainState,
        offset_x: f32,
        offset_y: f32,
    ) {
        for draw_prop in &self.dstdraw {
            if !draw_prop.get(state) {
                self.draw = false;
                return;
            }
        }
        self.draw = true;
        self.prepare_region(time, Some(state));
        self.region.x += offset_x;
        self.region.y += offset_y;
        if let Some(ref mouse_rect) = self.mouse_rect {
            let mx = state.get_main().get_input_processor().get_mouse_x() - self.region.x;
            let my = state.get_main().get_input_processor().get_mouse_y() - self.region.y;
            if !mouse_rect.contains(mx, my) {
                self.draw = false;
                return;
            }
        }

        self.prepare_color();
        self.prepare_angle();
    }

    pub fn draw_image(&mut self, sprite: &mut SkinObjectRenderer, image: &TextureRegion) {
        if self.color.a == 0.0 {
            return;
        }

        self.tmp_rect.set(&self.region);
        self.stretch
            .stretch_rect(&mut self.tmp_rect, &mut self.tmp_image, image);
        sprite.set_color(&self.color);
        sprite.set_blend(self.dstblend);
        sprite.set_type(
            if self.dstfilter != 0 && self.image_type == SkinObjectRenderer::TYPE_NORMAL {
                if self.tmp_rect.width == self.tmp_image.get_region_width() as f32
                    && self.tmp_rect.height == self.tmp_image.get_region_height() as f32
                {
                    SkinObjectRenderer::TYPE_NORMAL
                } else {
                    SkinObjectRenderer::TYPE_BILINEAR
                }
            } else {
                self.image_type
            },
        );

        if self.angle != 0 {
            sprite.draw_rotated(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
                self.centerx,
                self.centery,
                self.angle,
            );
        } else {
            sprite.draw(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
            );
        }
    }

    pub fn draw_image_at(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        image: &TextureRegion,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
    ) {
        let color = self.color.clone();
        let angle = self.angle;
        self.draw_image_at_with_color(sprite, image, x, y, width, height, &color, angle);
    }

    pub fn draw_image_at_with_color(
        &mut self,
        sprite: &mut SkinObjectRenderer,
        image: &TextureRegion,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        color: &Color,
        angle: i32,
    ) {
        if color.a == 0.0 {
            return;
        }
        self.tmp_rect.set_xywh(x, y, width, height);
        self.stretch
            .stretch_rect(&mut self.tmp_rect, &mut self.tmp_image, image);
        sprite.set_color(color);
        sprite.set_blend(self.dstblend);
        sprite.set_type(
            if self.dstfilter != 0 && self.image_type == SkinObjectRenderer::TYPE_NORMAL {
                if self.tmp_rect.width == self.tmp_image.get_region_width() as f32
                    && self.tmp_rect.height == self.tmp_image.get_region_height() as f32
                {
                    SkinObjectRenderer::TYPE_NORMAL
                } else {
                    SkinObjectRenderer::TYPE_BILINEAR
                }
            } else {
                self.image_type
            },
        );

        if angle != 0 {
            sprite.draw_rotated(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
                self.centerx,
                self.centery,
                angle,
            );
        } else {
            sprite.draw(
                &self.tmp_image,
                self.tmp_rect.x,
                self.tmp_rect.y,
                self.tmp_rect.width,
                self.tmp_rect.height,
            );
        }
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

    pub fn get_clickevent_id(&self) -> i32 {
        self.clickevent
            .as_ref()
            .map(|e| e.get_event_id())
            .unwrap_or(0)
    }

    pub fn get_clickevent(&self) -> Option<&dyn Event> {
        self.clickevent.as_deref()
    }

    pub fn set_clickevent_by_id(&mut self, clickevent: i32) {
        self.clickevent = event_factory::get_event_by_id(clickevent);
    }

    pub fn set_clickevent(&mut self, clickevent: Box<dyn Event>) {
        self.clickevent = Some(clickevent);
    }

    pub fn get_clickevent_type(&self) -> i32 {
        self.clickevent_type
    }

    pub fn set_clickevent_type(&mut self, clickevent_type: i32) {
        self.clickevent_type = clickevent_type;
    }

    pub fn is_relative(&self) -> bool {
        self.relative
    }

    pub fn set_relative(&mut self, relative: bool) {
        self.relative = relative;
    }

    pub fn get_offset_id(&self) -> &[i32] {
        &self.offset
    }

    pub fn set_offset_id_single(&mut self, offset: i32) {
        self.set_offset_id(&[offset]);
    }

    pub fn set_offset_id(&mut self, offset: &[i32]) {
        if !self.offset.is_empty() {
            return;
        }
        let mut seen = HashSet::new();
        for &o in offset {
            if o > 0 && o < skin_property::OFFSET_MAX + 1 {
                seen.insert(o);
            }
        }
        if !seen.is_empty() {
            self.offset = seen.into_iter().collect();
            self.off = vec![None; self.offset.len()];
        }
    }

    pub fn get_offsets(&self) -> &[Option<SkinOffset>] {
        &self.off
    }

    pub fn get_destination_timer(&self) -> Option<&dyn TimerProperty> {
        self.dsttimer.as_deref()
    }

    pub fn get_image_type(&self) -> i32 {
        self.image_type
    }

    pub fn set_image_type(&mut self, image_type: i32) {
        self.image_type = image_type;
    }

    pub fn get_filter(&self) -> i32 {
        self.dstfilter
    }

    pub fn set_filter(&mut self, filter: i32) {
        self.dstfilter = filter;
    }

    pub fn set_mouse_rect(&mut self, x2: f32, y2: f32, w2: f32, h2: f32) {
        self.mouse_rect = Some(Rectangle::new(x2, y2, w2, h2));
    }

    pub fn get_name(&self) -> Option<&str> {
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

/// SkinObjectRenderer (inner class of Skin, but used by all SkinObject draw calls)
pub struct SkinObjectRenderer {
    pub color: Color,
    pub blend: i32,
    pub obj_type: i32,
    pub sprite: SpriteBatch,
}

impl Default for SkinObjectRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl SkinObjectRenderer {
    pub const TYPE_NORMAL: i32 = 0;
    pub const TYPE_LINEAR: i32 = 1;
    pub const TYPE_BILINEAR: i32 = 2;
    pub const TYPE_FFMPEG: i32 = 3;
    pub const TYPE_LAYER: i32 = 4;
    pub const TYPE_DISTANCE_FIELD: i32 = 5;

    pub fn new() -> Self {
        Self {
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            blend: 0,
            obj_type: 0,
            sprite: SpriteBatch::new(),
        }
    }

    pub fn set_color(&mut self, color: &Color) {
        self.color.set(color);
    }

    pub fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.color.set_rgba(r, g, b, a);
    }

    pub fn get_color(&self) -> &Color {
        &self.color
    }

    pub fn set_blend(&mut self, blend: i32) {
        self.blend = blend;
    }

    pub fn get_blend(&self) -> i32 {
        self.blend
    }

    pub fn set_type(&mut self, t: i32) {
        self.obj_type = t;
    }

    pub fn get_type(&self) -> i32 {
        self.obj_type
    }

    /// Set texture filter based on current type.
    /// In Java: sets Linear filter for TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD.
    /// In Rust/wgpu, filtering is handled at sampler level, so this is a no-op stub.
    fn set_filter(&self, _image: &TextureRegion) {
        // In Java: if type == TYPE_LINEAR || TYPE_FFMPEG || TYPE_DISTANCE_FIELD,
        // sets TextureFilter.Linear on the texture.
        // In wgpu, filtering is configured on samplers, not textures directly.
    }

    /// Pre-draw setup: shader switching, blend mode, color.
    /// In Java: switches shader, sets blend function, saves/sets color.
    /// In Rust/wgpu, these would be handled by the render pipeline state.
    fn pre_draw(&mut self) {
        // In Java:
        // - switches shader if current != type
        // - sets blend function based on blend value (2=additive, 3=subtractive, 4=multiply, 9=invert)
        // - saves orgcolor and sets sprite color
        // Stubbed: wgpu pipeline handles these via render pipeline descriptors
    }

    /// Post-draw cleanup: restore color and blend mode.
    /// In Java: restores original color and resets blend to SRC_ALPHA/ONE_MINUS_SRC_ALPHA.
    fn post_draw(&mut self) {
        // In Java:
        // - restores orgcolor if it was saved
        // - resets blend to default (SRC_ALPHA, ONE_MINUS_SRC_ALPHA) if blend >= 2
        // Stubbed: wgpu pipeline handles these
    }

    /// Java: sprite.draw(image, x + 0.01f, y + 0.01f, w, h)
    /// The 0.01 offset is a workaround for a Windows TextureRegion rendering issue.
    pub fn draw(&mut self, image: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {
        self.set_filter(image);
        self.pre_draw();
        self.sprite.draw_region(image, x + 0.01, y + 0.01, w, h);
        self.post_draw();
    }

    /// Java: sprite.draw(image, x + 0.01f, y + 0.01f, cx * w, cy * h, w, h, 1, 1, angle)
    pub fn draw_rotated(
        &mut self,
        image: &TextureRegion,
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        cx: f32,
        cy: f32,
        angle: i32,
    ) {
        self.set_filter(image);
        self.pre_draw();
        self.sprite.draw_region_rotated(
            image,
            x + 0.01,
            y + 0.01,
            cx * w,
            cy * h,
            w,
            h,
            1.0,
            1.0,
            angle as f32,
        );
        self.post_draw();
    }
}
