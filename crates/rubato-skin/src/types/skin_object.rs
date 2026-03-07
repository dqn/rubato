// SkinObject.java -> skin_object.rs
// Mechanical line-by-line translation.

use std::collections::HashSet;

use crate::core::stretch_type::StretchType;
use crate::property::boolean_property::BooleanProperty;
use crate::property::boolean_property_factory;
use crate::property::event::Event;
use crate::property::event_factory;
use crate::property::float_property::FloatProperty;
use crate::property::integer_property_factory;
use crate::property::timer_property::TimerProperty;
use crate::skin_property;
use crate::stubs::{
    BitmapFont, Color, GlyphLayout, MainState, Rectangle, SkinOffset, SpriteBatch, Texture,
    TextureRegion,
};

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

/// Destination configuration: keyframes, timing, offsets.
pub struct DestinationData {
    pub dst: Vec<SkinObjectDestination>,
    pub dstloop: i32,
    pub dstcenter: i32,
    pub centerx: f32,
    pub centery: f32,
    pub acc: i32,
    pub starttime: i64,
    pub endtime: i64,
    pub offset: Vec<i32>,
    pub relative: bool,
    pub off: Vec<Option<SkinOffset>>,
}

impl Default for DestinationData {
    fn default() -> Self {
        Self {
            dst: Vec::new(),
            dstloop: 0,
            dstcenter: 0,
            centerx: 0.0,
            centery: 0.0,
            acc: 0,
            starttime: 0,
            endtime: 0,
            offset: Vec::new(),
            relative: false,
            off: Vec::new(),
        }
    }
}

/// Timer, blend, filter, event, and draw-condition properties.
pub struct TimerData {
    pub dsttimer: Option<Box<dyn TimerProperty>>,
    pub dstblend: i32,
    pub dstfilter: i32,
    pub image_type: i32,
    pub clickevent: Option<Box<dyn Event>>,
    pub clickevent_type: i32,
    pub dstop: Vec<i32>,
    pub dstdraw: Vec<Box<dyn BooleanProperty>>,
    pub mouse_rect: Option<Rectangle>,
    pub stretch: StretchType,
    pub name: Option<String>,
}

impl Default for TimerData {
    fn default() -> Self {
        Self {
            dsttimer: None,
            dstblend: 0,
            dstfilter: 0,
            image_type: 0,
            clickevent: None,
            clickevent_type: 0,
            dstop: Vec::new(),
            dstdraw: Vec::new(),
            mouse_rect: None,
            stretch: StretchType::Stretch,
            name: None,
        }
    }
}

/// Mutable draw state: current frame's computed region, color, angle, temporaries.
pub struct DrawData {
    pub draw: bool,
    pub visible: bool,
    pub region: Rectangle,
    pub color: Color,
    pub angle: i32,
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

impl Default for DrawData {
    fn default() -> Self {
        Self {
            draw: false,
            visible: true,
            region: Rectangle::default(),
            color: Color::default(),
            angle: 0,
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

/// Shared data for all SkinObject types.
#[derive(Default)]
pub struct SkinObjectData {
    pub dest: DestinationData,
    pub timer: TimerData,
    pub draw_state: DrawData,
}

static CENTERX: [f32; 10] = [0.5, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0, 0.0, 0.5, 1.0];
static CENTERY: [f32; 10] = [0.5, 0.0, 0.0, 0.0, 0.5, 0.5, 0.5, 1.0, 1.0, 1.0];

impl SkinObjectData {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn all_destination(&self) -> &[SkinObjectDestination] {
        &self.dest.dst
    }

    #[allow(clippy::too_many_arguments)]
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
            crate::property::timer_property_factory::timer_property(timer)
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

    #[allow(clippy::too_many_arguments)]
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
            crate::property::timer_property_factory::timer_property(timer)
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

    #[allow(clippy::too_many_arguments)]
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
            crate::property::timer_property_factory::timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer_prop,
        );
        if self.timer.dstop.is_empty() && self.timer.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    #[allow(clippy::too_many_arguments)]
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
            crate::property::timer_property_factory::timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(
            time, x, y, w, h, acc, a, r, g, b, blend, filter, angle, center, loop_val, timer_prop,
        );
        self.timer.dstdraw = vec![draw_prop];
    }

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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

    #[allow(clippy::too_many_arguments)]
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
        if self.timer.dstop.is_empty() && self.timer.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    #[allow(clippy::too_many_arguments)]
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
        self.timer.dstdraw = vec![draw_prop];
    }

    #[allow(clippy::too_many_arguments)]
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
        if self.dest.dst.is_empty() {
            self.draw_state.fixr = Some(obj.region.clone());
            self.draw_state.fixc = Some(obj.color);
            self.draw_state.fixa = obj.angle;
        } else {
            if let Some(ref fixr) = self.draw_state.fixr
                && !obj.region.equals(fixr)
            {
                self.draw_state.fixr = None;
            }
            if let Some(ref fixc) = self.draw_state.fixc
                && !obj.color.equals(fixc)
            {
                self.draw_state.fixc = None;
            }
            if self.draw_state.fixa != obj.angle {
                self.draw_state.fixa = i32::MIN;
            }
        }
        if self.dest.acc == 0 {
            self.dest.acc = acc;
        }
        if self.timer.dstblend == 0 {
            self.timer.dstblend = blend;
        }
        if self.timer.dstfilter == 0 {
            self.timer.dstfilter = filter;
        }
        if self.dest.dstcenter == 0 && (0..10).contains(&center) {
            self.dest.dstcenter = center;
            self.dest.centerx = CENTERX[center as usize];
            self.dest.centery = CENTERY[center as usize];
        }
        if self.timer.dsttimer.is_none() {
            self.timer.dsttimer = timer;
        }
        if self.dest.dstloop == 0 {
            self.dest.dstloop = loop_val;
        }
        if let Some(pos) = self.dest.dst.iter().position(|d| d.time > time) {
            self.dest.dst.insert(pos, obj);
        } else {
            self.dest.dst.push(obj);
        }
        self.dest.starttime = self.dest.dst[0].time;
        self.dest.endtime = self.dest.dst[self.dest.dst.len() - 1].time;
    }

    pub fn draw_condition(&self) -> &[Box<dyn BooleanProperty>] {
        &self.timer.dstdraw
    }

    pub fn option(&self) -> &[i32] {
        &self.timer.dstop
    }

    pub fn set_draw_condition_from_ops(&mut self, dstop: &[i32]) {
        let mut seen = HashSet::new();
        let mut op = Vec::new();
        let mut draw: Vec<Box<dyn BooleanProperty>> = Vec::new();
        for &i in dstop {
            if i != 0 && !seen.contains(&i) {
                if let Some(dc) = boolean_property_factory::boolean_property(i) {
                    draw.push(dc);
                } else {
                    op.push(i);
                }
                seen.insert(i);
            }
        }
        self.timer.dstop = op;
        self.timer.dstdraw = draw;
    }

    pub fn set_stretch_by_id(&mut self, stretch: i32) {
        if stretch < 0 {
            return;
        }
        for st in StretchType::values() {
            if st.id() == stretch {
                self.timer.stretch = *st;
                return;
            }
        }
    }

    pub fn stretch(&self) -> StretchType {
        self.timer.stretch
    }

    pub fn blend(&self) -> i32 {
        self.timer.dstblend
    }

    pub fn prepare_region(&mut self, time: i64, state: Option<&dyn MainState>) {
        let mut time = time;

        if let Some(ref timer) = self.timer.dsttimer
            && let Some(s) = state
        {
            if timer.is_off(s) {
                self.draw_state.draw = false;
                return;
            }
            time -= timer.get(s);
        }

        let lasttime = self.dest.endtime;
        if self.dest.dstloop == -1 {
            if time > self.dest.endtime {
                time = -1;
            }
        } else if lasttime > 0 && time > self.dest.dstloop as i64 {
            if lasttime == self.dest.dstloop as i64 {
                time = self.dest.dstloop as i64;
            } else {
                time = (time - self.dest.dstloop as i64) % (lasttime - self.dest.dstloop as i64)
                    + self.dest.dstloop as i64;
            }
        }
        if self.dest.starttime > time {
            self.draw_state.draw = false;
            return;
        }
        self.draw_state.nowtime = time;
        self.draw_state.rate = -1.0;
        self.draw_state.index = -1;
        for (off, &offset) in self.dest.off.iter_mut().zip(self.dest.offset.iter()) {
            *off = if let Some(s) = state {
                s.get_offset_value(offset).copied()
            } else {
                None
            };
        }

        if self.draw_state.fixr.is_none() {
            self.rate();
            if self.dest.dst.is_empty() {
                self.draw_state.draw = false;
                return;
            }
            if self.draw_state.rate == 0.0 {
                let idx = self.draw_state.index as usize;
                self.draw_state.region.set(&self.dest.dst[idx].region);
            } else if self.dest.acc == 3 {
                let idx = self.draw_state.index as usize;
                let r1 = &self.dest.dst[idx].region;
                self.draw_state.region.x = r1.x;
                self.draw_state.region.y = r1.y;
                self.draw_state.region.width = r1.width;
                self.draw_state.region.height = r1.height;
            } else {
                let idx = self.draw_state.index as usize;
                let rate = self.draw_state.rate;
                let r1x = self.dest.dst[idx].region.x;
                let r1y = self.dest.dst[idx].region.y;
                let r1w = self.dest.dst[idx].region.width;
                let r1h = self.dest.dst[idx].region.height;
                let r2x = self.dest.dst[idx + 1].region.x;
                let r2y = self.dest.dst[idx + 1].region.y;
                let r2w = self.dest.dst[idx + 1].region.width;
                let r2h = self.dest.dst[idx + 1].region.height;
                self.draw_state.region.x = r1x + (r2x - r1x) * rate;
                self.draw_state.region.y = r1y + (r2y - r1y) * rate;
                self.draw_state.region.width = r1w + (r2w - r1w) * rate;
                self.draw_state.region.height = r1h + (r2h - r1h) * rate;
            }

            for off in self.dest.off.iter().flatten() {
                if !self.dest.relative {
                    self.draw_state.region.x += off.x - off.w / 2.0;
                    self.draw_state.region.y += off.y - off.h / 2.0;
                }
                self.draw_state.region.width += off.w;
                self.draw_state.region.height += off.h;
            }
        } else if let Some(ref fixr) = self.draw_state.fixr {
            if self.dest.offset.is_empty() {
                self.draw_state.region.set(fixr);
                return;
            }
            self.draw_state.region.set(fixr);
            for off in self.dest.off.iter().flatten() {
                if !self.dest.relative {
                    self.draw_state.region.x += off.x - off.w / 2.0;
                    self.draw_state.region.y += off.y - off.h / 2.0;
                }
                self.draw_state.region.width += off.w;
                self.draw_state.region.height += off.h;
            }
        }
    }

    pub fn destination(&self, _time: i64, _state: &dyn MainState) -> Option<&Rectangle> {
        if self.draw_state.draw {
            Some(&self.draw_state.region)
        } else {
            None
        }
    }

    fn prepare_color(&mut self) {
        if let Some(ref fixc) = self.draw_state.fixc {
            self.draw_state.color.set(fixc);
            for off in self.dest.off.iter().flatten() {
                let a = (self.draw_state.color.a + (off.a / 255.0)).clamp(0.0, 1.0);
                self.draw_state.color.a = a;
            }
            return;
        }
        self.rate();
        if self.dest.dst.is_empty() {
            return;
        }
        if self.draw_state.rate == 0.0 {
            let idx = self.draw_state.index as usize;
            let c = self.dest.dst[idx].color;
            self.draw_state.color.set(&c);
        } else if self.dest.acc == 3 {
            let idx = self.draw_state.index as usize;
            self.draw_state.color.r = self.dest.dst[idx].color.r;
            self.draw_state.color.g = self.dest.dst[idx].color.g;
            self.draw_state.color.b = self.dest.dst[idx].color.b;
            self.draw_state.color.a = self.dest.dst[idx].color.a;
            return;
        } else {
            let idx = self.draw_state.index as usize;
            let rate = self.draw_state.rate;
            let r1r = self.dest.dst[idx].color.r;
            let r1g = self.dest.dst[idx].color.g;
            let r1b = self.dest.dst[idx].color.b;
            let r1a = self.dest.dst[idx].color.a;
            let r2r = self.dest.dst[idx + 1].color.r;
            let r2g = self.dest.dst[idx + 1].color.g;
            let r2b = self.dest.dst[idx + 1].color.b;
            let r2a = self.dest.dst[idx + 1].color.a;
            self.draw_state.color.r = r1r + (r2r - r1r) * rate;
            self.draw_state.color.g = r1g + (r2g - r1g) * rate;
            self.draw_state.color.b = r1b + (r2b - r1b) * rate;
            self.draw_state.color.a = r1a + (r2a - r1a) * rate;
            return;
        }
        for off in self.dest.off.iter().flatten() {
            let a = (self.draw_state.color.a + (off.a / 255.0)).clamp(0.0, 1.0);
            self.draw_state.color.a = a;
        }
    }

    pub fn color(&self) -> &Color {
        &self.draw_state.color
    }

    fn prepare_angle(&mut self) {
        if self.draw_state.fixa != i32::MIN {
            self.draw_state.angle = self.draw_state.fixa;
            for off in self.dest.off.iter().flatten() {
                self.draw_state.angle += off.r as i32;
            }
            return;
        }
        self.rate();
        if self.dest.dst.is_empty() {
            return;
        }
        let idx = self.draw_state.index as usize;
        self.draw_state.angle = if self.draw_state.rate == 0.0 || self.dest.acc == 3 {
            self.dest.dst[idx].angle
        } else {
            (self.dest.dst[idx].angle as f32
                + (self.dest.dst[idx + 1].angle - self.dest.dst[idx].angle) as f32
                    * self.draw_state.rate) as i32
        };
        for off in self.dest.off.iter().flatten() {
            self.draw_state.angle += off.r as i32;
        }
    }

    fn rate(&mut self) {
        if self.draw_state.rate != -1.0 {
            return;
        }
        if self.dest.dst.is_empty() {
            self.draw_state.rate = 0.0;
            self.draw_state.index = 0;
            return;
        }
        let mut time2 = self.dest.dst[self.dest.dst.len() - 1].time;
        if self.draw_state.nowtime == time2 {
            self.draw_state.rate = 0.0;
            self.draw_state.index = self.dest.dst.len() as i32 - 1;
            return;
        }
        for i in (0..=(self.dest.dst.len() as i32 - 2)).rev() {
            let i = i as usize;
            let time1 = self.dest.dst[i].time;
            if time1 <= self.draw_state.nowtime && time2 > self.draw_state.nowtime {
                let mut rate = (self.draw_state.nowtime - time1) as f32 / (time2 - time1) as f32;
                match self.dest.acc {
                    1 => {
                        rate = rate * rate;
                    }
                    2 => {
                        rate = 1.0 - (rate - 1.0) * (rate - 1.0);
                    }
                    _ => {}
                }
                self.draw_state.rate = rate;
                self.draw_state.index = i as i32;
                return;
            }
            time2 = time1;
        }
        self.draw_state.rate = 0.0;
        self.draw_state.index = 0;
    }

    pub fn validate(&self) -> bool {
        !self.dest.dst.is_empty()
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
        for draw_prop in &self.timer.dstdraw {
            if !draw_prop.get(state) {
                self.draw_state.draw = false;
                return;
            }
        }
        self.draw_state.draw = true;
        self.prepare_region(time, Some(state));
        self.draw_state.region.x += offset_x;
        self.draw_state.region.y += offset_y;
        if let Some(ref mouse_rect) = self.timer.mouse_rect {
            let mx = state.get_main().input_processor().mouse_x() - self.draw_state.region.x;
            let my = state.get_main().input_processor().mouse_y() - self.draw_state.region.y;
            if !mouse_rect.contains(mx, my) {
                self.draw_state.draw = false;
                return;
            }
        }

        self.prepare_color();
        self.prepare_angle();
    }

    pub fn draw_image(&mut self, sprite: &mut SkinObjectRenderer, image: &TextureRegion) {
        if self.draw_state.color.a == 0.0 {
            return;
        }

        self.draw_state.tmp_rect.set(&self.draw_state.region);
        self.timer.stretch.stretch_rect(
            &mut self.draw_state.tmp_rect,
            &mut self.draw_state.tmp_image,
            image,
        );
        sprite.set_color(&self.draw_state.color);
        sprite.blend = self.timer.dstblend;
        sprite.obj_type = if self.timer.dstfilter != 0
            && self.timer.image_type == SkinObjectRenderer::TYPE_NORMAL
        {
            if self.draw_state.tmp_rect.width == self.draw_state.tmp_image.region_width as f32
                && self.draw_state.tmp_rect.height == self.draw_state.tmp_image.region_height as f32
            {
                SkinObjectRenderer::TYPE_NORMAL
            } else {
                SkinObjectRenderer::TYPE_BILINEAR
            }
        } else {
            self.timer.image_type
        };

        if self.draw_state.angle != 0 {
            sprite.draw_rotated(
                &self.draw_state.tmp_image,
                self.draw_state.tmp_rect.x,
                self.draw_state.tmp_rect.y,
                self.draw_state.tmp_rect.width,
                self.draw_state.tmp_rect.height,
                self.dest.centerx,
                self.dest.centery,
                self.draw_state.angle,
            );
        } else {
            sprite.draw(
                &self.draw_state.tmp_image,
                self.draw_state.tmp_rect.x,
                self.draw_state.tmp_rect.y,
                self.draw_state.tmp_rect.width,
                self.draw_state.tmp_rect.height,
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
        let color = self.draw_state.color;
        let angle = self.draw_state.angle;
        self.draw_image_at_with_color(sprite, image, x, y, width, height, &color, angle);
    }

    #[allow(clippy::too_many_arguments)]
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
        self.draw_state.tmp_rect.set_xywh(x, y, width, height);
        self.timer.stretch.stretch_rect(
            &mut self.draw_state.tmp_rect,
            &mut self.draw_state.tmp_image,
            image,
        );
        sprite.set_color(color);
        sprite.blend = self.timer.dstblend;
        sprite.obj_type = if self.timer.dstfilter != 0
            && self.timer.image_type == SkinObjectRenderer::TYPE_NORMAL
        {
            if self.draw_state.tmp_rect.width == self.draw_state.tmp_image.region_width as f32
                && self.draw_state.tmp_rect.height == self.draw_state.tmp_image.region_height as f32
            {
                SkinObjectRenderer::TYPE_NORMAL
            } else {
                SkinObjectRenderer::TYPE_BILINEAR
            }
        } else {
            self.timer.image_type
        };

        if angle != 0 {
            sprite.draw_rotated(
                &self.draw_state.tmp_image,
                self.draw_state.tmp_rect.x,
                self.draw_state.tmp_rect.y,
                self.draw_state.tmp_rect.width,
                self.draw_state.tmp_rect.height,
                self.dest.centerx,
                self.dest.centery,
                angle,
            );
        } else {
            sprite.draw(
                &self.draw_state.tmp_image,
                self.draw_state.tmp_rect.x,
                self.draw_state.tmp_rect.y,
                self.draw_state.tmp_rect.width,
                self.draw_state.tmp_rect.height,
            );
        }
    }

    pub fn mouse_pressed(&self, state: &mut dyn MainState, button: i32, x: i32, y: i32) -> bool {
        if let Some(ref clickevent) = self.timer.clickevent {
            let r = &self.draw_state.region;
            let button_events: [i32; 5] = [1, -1, 1, 1, -1];
            let inc = if button >= 0 && (button as usize) < button_events.len() {
                button_events[button as usize]
            } else {
                0
            };
            match self.timer.clickevent_type {
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
        self.timer
            .clickevent
            .as_ref()
            .map(|e| e.get_event_id().as_i32())
            .unwrap_or(0)
    }

    pub fn clickevent(&self) -> Option<&dyn Event> {
        self.timer.clickevent.as_deref()
    }

    pub fn set_clickevent_by_id(&mut self, clickevent: i32) {
        self.timer.clickevent = event_factory::event_by_id(clickevent);
    }

    pub fn set_clickevent(&mut self, clickevent: Box<dyn Event>) {
        self.timer.clickevent = Some(clickevent);
    }

    pub fn clickevent_type(&self) -> i32 {
        self.timer.clickevent_type
    }

    pub fn is_relative(&self) -> bool {
        self.dest.relative
    }

    pub fn offset_id(&self) -> &[i32] {
        &self.dest.offset
    }

    pub fn set_offset_id_single(&mut self, offset: i32) {
        self.set_offset_id(&[offset]);
    }

    pub fn set_offset_id(&mut self, offset: &[i32]) {
        if !self.dest.offset.is_empty() {
            return;
        }
        let mut seen = HashSet::new();
        for &o in offset {
            if o > 0 && o < skin_property::OFFSET_MAX + 1 {
                seen.insert(o);
            }
        }
        if !seen.is_empty() {
            self.dest.offset = seen.into_iter().collect();
            self.dest.off = vec![None; self.dest.offset.len()];
        }
    }

    pub fn offsets(&self) -> &[Option<SkinOffset>] {
        &self.dest.off
    }

    pub fn destination_timer(&self) -> Option<&dyn TimerProperty> {
        self.timer.dsttimer.as_deref()
    }

    pub fn image_type(&self) -> i32 {
        self.timer.image_type
    }

    pub fn filter(&self) -> i32 {
        self.timer.dstfilter
    }

    pub fn set_mouse_rect(&mut self, x2: f32, y2: f32, w2: f32, h2: f32) {
        self.timer.mouse_rect = Some(Rectangle::new(x2, y2, w2, h2));
    }

    pub fn name(&self) -> Option<&str> {
        self.timer.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) {
        self.timer.name = Some(name);
    }

    pub fn is_disposed(&self) -> bool {
        self.draw_state.disposed
    }

    pub fn set_disposed(&mut self) {
        self.draw_state.disposed = true;
    }
}

/// SkinObjectRenderer (inner class of Skin, but used by all SkinObject draw calls)
/// Corresponds to Skin.SkinObjectRenderer in Java.
///
/// Manages shader switching, blend state, and color for sprite draw calls.
/// Java: holds SpriteBatch + ShaderProgram[] + blend/type/color state.
pub struct SkinObjectRenderer {
    pub color: Color,
    pub blend: i32,
    pub obj_type: i32,
    /// Current active shader type (tracks which shader is set on the sprite batch)
    current_shader: i32,
    /// Saved color before pre_draw, restored in post_draw
    orgcolor: Option<Color>,
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

    // GL blend constants (matching Java)
    const GL_SRC_ALPHA: i32 = 0x0302;
    const GL_ONE: i32 = 1;
    const GL_ONE_MINUS_SRC_ALPHA: i32 = 0x0303;
    const GL_ZERO: i32 = 0;
    const GL_SRC_COLOR: i32 = 0x0300;
    const GL_ONE_MINUS_DST_COLOR: i32 = 0x0307;

    pub fn new() -> Self {
        let mut sprite = SpriteBatch::new();
        // Java: sprite.setShader(shaders[current]); sprite.setColor(Color.WHITE);
        sprite.shader_type = Self::TYPE_NORMAL;
        sprite.set_color(&Color::new(1.0, 1.0, 1.0, 1.0));
        Self {
            color: Color::new(1.0, 1.0, 1.0, 1.0),
            blend: 0,
            obj_type: 0,
            current_shader: Self::TYPE_NORMAL,
            orgcolor: None,
            sprite,
        }
    }

    pub fn set_color(&mut self, color: &Color) {
        self.color.set(color);
    }

    pub fn set_color_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.color.set_rgba(r, g, b, a);
    }

    pub fn color(&self) -> &Color {
        &self.color
    }

    pub fn blend(&self) -> i32 {
        self.blend
    }

    pub fn toast_type(&self) -> i32 {
        self.obj_type
    }

    /// Set texture filter based on current type.
    /// Java: sets Linear filter for TYPE_LINEAR, TYPE_FFMPEG, TYPE_DISTANCE_FIELD.
    /// In wgpu, filtering is handled by sampler selection in the render pipeline.
    fn set_filter(&self, _image: &TextureRegion) {
        // In wgpu, filtering is configured on samplers via SpriteRenderPipeline::get_sampler().
        // The sampler is selected based on shader_type when creating texture bind groups.
    }

    /// Pre-draw setup: shader switching, blend mode, color.
    /// Java: Skin.java lines 496-537
    fn pre_draw(&mut self) {
        // Java: if(shaders[current] != shaders[type]) { sprite.setShader(shaders[type]); current = type; }
        if self.current_shader != self.obj_type {
            self.sprite.shader_type = self.obj_type;
            self.current_shader = self.obj_type;
        }

        // Java: switch(blend) — set blend function
        match self.blend {
            2 => {
                // Additive: SRC_ALPHA, ONE
                self.sprite
                    .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE);
            }
            3 => {
                // Subtractive: SRC_ALPHA, ONE (with GL_FUNC_SUBTRACT equation)
                // In wgpu, this is handled by the BlendMode::Subtractive pipeline
                self.sprite
                    .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE);
            }
            4 => {
                // Multiply: ZERO, SRC_COLOR
                self.sprite
                    .set_blend_function(Self::GL_ZERO, Self::GL_SRC_COLOR);
            }
            9 => {
                // Inversion: ONE_MINUS_DST_COLOR, ZERO
                self.sprite
                    .set_blend_function(Self::GL_ONE_MINUS_DST_COLOR, Self::GL_ZERO);
            }
            _ => {}
        }

        // Java: orgcolor = sprite.getColor(); sprite.setColor(color);
        self.orgcolor = Some(self.sprite.color());
        self.sprite.set_color(&self.color);
    }

    /// Post-draw cleanup: restore color and blend mode.
    /// Java: Skin.java lines 539-547
    fn post_draw(&mut self) {
        // Java: if(orgcolor != null) { sprite.setColor(orgcolor); }
        if let Some(ref orgcolor) = self.orgcolor.take() {
            self.sprite.set_color(orgcolor);
        }

        // Java: if (blend >= 2) { sprite.setBlendFunction(GL_SRC_ALPHA, GL_ONE_MINUS_SRC_ALPHA); }
        if self.blend >= 2 {
            self.sprite
                .set_blend_function(Self::GL_SRC_ALPHA, Self::GL_ONE_MINUS_SRC_ALPHA);
        }
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
    #[allow(clippy::too_many_arguments)]
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

    /// Draw a full Texture at (x, y) with size (w, h).
    /// Java: SkinObjectRenderer.draw(Texture image, float x, float y, float w, float h)
    pub fn draw_texture(&mut self, image: &Texture, x: f32, y: f32, w: f32, h: f32) {
        // Java: setFilter(image)
        // In wgpu, filtering is configured on samplers via SpriteRenderPipeline::get_sampler().
        self.pre_draw();
        self.sprite.draw_texture(image, x, y, w, h);
        self.post_draw();
    }

    /// Draw text using a BitmapFont with color.
    /// Java: SkinObjectRenderer.draw(BitmapFont font, String s, float x, float y, Color c)
    ///
    /// Sets the font color, then delegates to font.draw(sprite, text, x, y) which
    /// rasterizes glyphs and submits quads to the SpriteBatch.
    pub fn draw_font(&mut self, font: &mut BitmapFont, text: &str, x: f32, y: f32, color: &Color) {
        // Java: for (TextureRegion region : font.getRegions()) { setFilter(region); }
        // In wgpu, filtering is handled by sampler selection based on shader_type.
        self.pre_draw();
        font.set_color(color);
        font.draw(&mut self.sprite, text, x, y);
        self.post_draw();
    }

    /// Draw pre-laid-out text using a BitmapFont and GlyphLayout.
    /// Java: SkinObjectRenderer.draw(BitmapFont font, GlyphLayout layout, float x, float y)
    pub fn draw_font_layout(&mut self, font: &BitmapFont, layout: &GlyphLayout, x: f32, y: f32) {
        self.pre_draw();
        font.draw_layout(&mut self.sprite, layout, x, y);
        self.post_draw();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_skin_object_renderer_new() {
        let renderer = SkinObjectRenderer::new();
        assert_eq!(renderer.blend, 0);
        assert_eq!(renderer.obj_type, 0);
        // Default color is white
        assert_eq!(renderer.color.r, 1.0);
        assert_eq!(renderer.color.g, 1.0);
        assert_eq!(renderer.color.b, 1.0);
        assert_eq!(renderer.color.a, 1.0);
    }

    #[test]
    fn test_skin_object_renderer_type_constants() {
        // Match Java SkinObjectRenderer constants
        assert_eq!(SkinObjectRenderer::TYPE_NORMAL, 0);
        assert_eq!(SkinObjectRenderer::TYPE_LINEAR, 1);
        assert_eq!(SkinObjectRenderer::TYPE_BILINEAR, 2);
        assert_eq!(SkinObjectRenderer::TYPE_FFMPEG, 3);
        assert_eq!(SkinObjectRenderer::TYPE_LAYER, 4);
        assert_eq!(SkinObjectRenderer::TYPE_DISTANCE_FIELD, 5);
    }

    #[test]
    fn test_skin_object_renderer_set_color() {
        let mut renderer = SkinObjectRenderer::new();
        let red = Color::new(1.0, 0.0, 0.0, 0.5);
        renderer.set_color(&red);
        assert_eq!(renderer.color().r, 1.0);
        assert_eq!(renderer.color().g, 0.0);
        assert_eq!(renderer.color().a, 0.5);
    }

    #[test]
    fn test_skin_object_renderer_set_blend() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.blend = 2;
        assert_eq!(renderer.blend(), 2);
    }

    #[test]
    fn test_skin_object_renderer_set_type() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.obj_type = SkinObjectRenderer::TYPE_BILINEAR;
        assert_eq!(renderer.toast_type(), SkinObjectRenderer::TYPE_BILINEAR);
    }

    #[test]
    fn test_skin_object_renderer_draw_generates_vertices() {
        let mut renderer = SkinObjectRenderer::new();
        let region = TextureRegion::new();
        renderer.draw(&region, 10.0, 20.0, 100.0, 50.0);
        // draw calls pre_draw + sprite.draw_region + post_draw
        // sprite should have 6 vertices for one quad
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_renderer_pre_draw_sets_blend_additive() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.blend = 2; // Additive
        let region = TextureRegion::new();
        renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
        // After post_draw, blend should be reset to Normal
        // (post_draw resets blend to SRC_ALPHA/ONE_MINUS_SRC_ALPHA when blend >= 2)
        let color = renderer.sprite.color();
        // Color should be restored to white (default)
        assert_eq!(color.r, 1.0);
        assert_eq!(color.g, 1.0);
        assert_eq!(color.b, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_skin_object_renderer_pre_draw_shader_switching() {
        let mut renderer = SkinObjectRenderer::new();
        // Initially TYPE_NORMAL
        assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_NORMAL);
        // Set type to FFMPEG
        renderer.obj_type = SkinObjectRenderer::TYPE_FFMPEG;
        let region = TextureRegion::new();
        renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
        // After pre_draw, current_shader should match obj_type
        assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_FFMPEG);
        assert_eq!(
            renderer.sprite.shader_type(),
            SkinObjectRenderer::TYPE_FFMPEG
        );
    }

    #[test]
    fn test_skin_object_renderer_draw_rotated_generates_vertices() {
        let mut renderer = SkinObjectRenderer::new();
        let region = TextureRegion::new();
        renderer.draw_rotated(&region, 10.0, 20.0, 100.0, 50.0, 0.5, 0.5, 45);
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_renderer_pre_draw_saves_and_restores_color() {
        let mut renderer = SkinObjectRenderer::new();
        // Set sprite color to something specific
        let blue = Color::new(0.0, 0.0, 1.0, 1.0);
        renderer.sprite.set_color(&blue);
        // Set renderer color to red
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        renderer.set_color(&red);
        // Draw: pre_draw saves blue, sets red; post_draw restores blue
        let region = TextureRegion::new();
        renderer.draw(&region, 0.0, 0.0, 10.0, 10.0);
        let restored = renderer.sprite.color();
        assert_eq!(restored.r, 0.0);
        assert_eq!(restored.g, 0.0);
        assert_eq!(restored.b, 1.0);
        assert_eq!(restored.a, 1.0);
    }

    #[test]
    fn test_skin_object_destination_new() {
        let dst = SkinObjectDestination::new(
            1000,
            Rectangle::new(10.0, 20.0, 100.0, 50.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            45,
            1,
        );
        assert_eq!(dst.time, 1000);
        assert_eq!(dst.region.x, 10.0);
        assert_eq!(dst.angle, 45);
        assert_eq!(dst.acc, 1);
    }

    #[test]
    fn test_skin_object_data_validate() {
        let data = SkinObjectData::new();
        assert!(!data.validate());

        let mut data = SkinObjectData::new();
        data.dest.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::default(),
            Color::default(),
            0,
            0,
        ));
        assert!(data.validate());
    }

    // =========================================================================
    // Phase 40a: Two-phase prepare/draw lifecycle tests
    // =========================================================================

    /// Helper: set up a single-destination SkinObjectData so prepare() sets draw=true.
    fn setup_data(data: &mut SkinObjectData, x: f32, y: f32, w: f32, h: f32) {
        data.set_destination_with_int_timer_ops(
            0,
            x,
            y,
            w,
            h,
            0,
            255,
            255,
            255,
            255,
            0,
            0,
            0,
            0,
            0,
            0,
            &[0],
        );
    }

    #[test]
    fn test_skin_object_data_prepare_sets_draw_and_region() {
        // Phase 40a: verify prepare(&mut self) mutates internal state
        let mut data = SkinObjectData::new();
        setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

        // Before prepare: draw is false
        assert!(!data.draw_state.draw);

        let state = crate::test_helpers::MockMainState::default();
        data.prepare(0, &state);

        // After prepare: draw is true, region is populated
        assert!(data.draw_state.draw);
        assert_eq!(data.draw_state.region.x, 10.0);
        assert_eq!(data.draw_state.region.y, 20.0);
        assert_eq!(data.draw_state.region.width, 100.0);
        assert_eq!(data.draw_state.region.height, 50.0);
    }

    #[test]
    fn test_skin_object_data_prepare_then_draw_image() {
        // Phase 40a: verify two-phase pattern — prepare() then draw_image()
        let mut data = SkinObjectData::new();
        setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

        let state = crate::test_helpers::MockMainState::default();
        data.prepare(0, &state);
        assert!(data.draw_state.draw);

        // Phase 2: draw reads pre-computed state (region, color, angle)
        let mut renderer = SkinObjectRenderer::new();
        let image = TextureRegion::new();
        data.draw_image(&mut renderer, &image);

        // Verify vertices were generated
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_data_prepare_color_and_angle_cached() {
        // Phase 40a: verify prepare() caches color and angle for later draw use
        let mut data = SkinObjectData::new();
        // Set up with specific color (128, 64, 32, 200) and angle=45
        data.set_destination_with_int_timer_ops(
            0,
            0.0,
            0.0,
            50.0,
            50.0,
            0,
            200,
            128,
            64,
            32,
            0,
            0,
            45,
            0,
            0,
            0,
            &[0],
        );

        let state = crate::test_helpers::MockMainState::default();
        data.prepare(0, &state);

        // Color should be cached
        assert!((data.draw_state.color.r - 128.0 / 255.0).abs() < 0.01);
        assert!((data.draw_state.color.g - 64.0 / 255.0).abs() < 0.01);
        assert!((data.draw_state.color.b - 32.0 / 255.0).abs() < 0.01);
        assert!((data.draw_state.color.a - 200.0 / 255.0).abs() < 0.01);
        // Angle should be cached
        assert_eq!(data.draw_state.angle, 45);
    }

    #[test]
    fn test_skin_object_data_draw_without_prepare_does_not_draw() {
        // Phase 40a: verify draw skips when prepare hasn't been called
        let mut data = SkinObjectData::new();
        setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

        // draw is false by default (no prepare called)
        assert!(!data.draw_state.draw);

        // Attempting to use draw_image would still work mechanically,
        // but the caller checks data.draw before calling draw methods.
        // This test verifies the flag is false.
    }

    #[test]
    fn test_skin_object_data_prepare_with_offset_modifies_region() {
        // Phase 40a: verify prepare_with_offset() adds offset to region
        let mut data = SkinObjectData::new();
        setup_data(&mut data, 10.0, 20.0, 100.0, 50.0);

        let state = crate::test_helpers::MockMainState::default();
        data.prepare_with_offset(0, &state, 5.0, 3.0);

        assert!(data.draw_state.draw);
        assert_eq!(data.draw_state.region.x, 15.0); // 10 + 5
        assert_eq!(data.draw_state.region.y, 23.0); // 20 + 3
    }

    #[test]
    fn test_skin_object_data_two_phase_separate_calls() {
        // Phase 40a: The key invariant — prepare and draw are separate calls.
        // The caller can inspect state between prepare and draw.
        let mut data = SkinObjectData::new();
        setup_data(&mut data, 50.0, 60.0, 200.0, 150.0);

        let state = crate::test_helpers::MockMainState::default();

        // Phase 1: prepare (mutable)
        data.prepare(0, &state);
        assert!(data.draw_state.draw);

        // Between phases: caller can read the cached state
        let cached_region = data.draw_state.region.clone();
        let _cached_color = data.draw_state.color;
        assert_eq!(cached_region.x, 50.0);
        assert_eq!(cached_region.width, 200.0);

        // Phase 2: draw (also mutable for scratch-space)
        let mut renderer = SkinObjectRenderer::new();
        let image = TextureRegion::new();
        data.draw_image_at(
            &mut renderer,
            &image,
            cached_region.x,
            cached_region.y,
            cached_region.width,
            cached_region.height,
        );

        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_data_empty_dst_does_not_panic() {
        // Regression: rate() panicked on `self.dest.dst.len() - 1` when dst is empty.
        let mut data = SkinObjectData::new();
        assert!(data.dest.dst.is_empty());

        let state = crate::test_helpers::MockMainState::default();
        // prepare() calls rate(), prepare_color(), prepare_angle() — all must survive empty dst.
        data.prepare(0, &state);

        // With empty dst, draw should remain false (no destination to render).
        assert!(!data.draw_state.draw);
    }

    // =========================================================================
    // draw_texture / draw_font / draw_font_layout tests
    // =========================================================================

    #[test]
    fn test_skin_object_renderer_draw_texture_generates_vertices() {
        let mut renderer = SkinObjectRenderer::new();
        let tex = Texture::default();
        renderer.draw_texture(&tex, 10.0, 20.0, 100.0, 50.0);
        // 1 quad = 6 vertices
        assert_eq!(renderer.sprite.vertices().len(), 6);
    }

    #[test]
    fn test_skin_object_renderer_draw_texture_applies_blend() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.blend = 2; // Additive
        let tex = Texture::default();
        renderer.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        // After post_draw, blend is reset to Normal
        let color = renderer.sprite.color();
        assert_eq!(color.r, 1.0);
        assert_eq!(color.a, 1.0);
    }

    #[test]
    fn test_skin_object_renderer_draw_texture_shader_switching() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.obj_type = SkinObjectRenderer::TYPE_LINEAR;
        let tex = Texture::default();
        renderer.draw_texture(&tex, 0.0, 0.0, 10.0, 10.0);
        assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_LINEAR);
        assert_eq!(
            renderer.sprite.shader_type(),
            SkinObjectRenderer::TYPE_LINEAR
        );
    }

    #[test]
    fn test_skin_object_renderer_draw_font_no_crash() {
        let mut renderer = SkinObjectRenderer::new();
        let mut font = BitmapFont::new();
        let white = Color::new(1.0, 1.0, 1.0, 1.0);
        // BitmapFont without a loaded font file will just be a no-op
        renderer.draw_font(&mut font, "Hello", 10.0, 20.0, &white);
        // No crash is the success criterion; font has no loaded font file
        // so no vertices are generated
    }

    #[test]
    fn test_skin_object_renderer_draw_font_saves_restores_color() {
        let mut renderer = SkinObjectRenderer::new();
        let blue = Color::new(0.0, 0.0, 1.0, 1.0);
        renderer.sprite.set_color(&blue);
        let red = Color::new(1.0, 0.0, 0.0, 1.0);
        renderer.set_color(&red);

        let mut font = BitmapFont::new();
        let green = Color::new(0.0, 1.0, 0.0, 1.0);
        renderer.draw_font(&mut font, "Test", 0.0, 0.0, &green);

        // After post_draw, sprite color should be restored to blue
        let restored = renderer.sprite.color();
        assert_eq!(restored.r, 0.0);
        assert_eq!(restored.g, 0.0);
        assert_eq!(restored.b, 1.0);
        assert_eq!(restored.a, 1.0);
    }

    #[test]
    fn test_skin_object_renderer_draw_font_layout_no_crash() {
        let mut renderer = SkinObjectRenderer::new();
        let font = BitmapFont::new();
        let layout = GlyphLayout::new();
        renderer.draw_font_layout(&font, &layout, 10.0, 20.0);
        // No crash is the success criterion
    }

    #[test]
    fn test_skin_object_renderer_draw_font_shader_switching() {
        let mut renderer = SkinObjectRenderer::new();
        renderer.obj_type = SkinObjectRenderer::TYPE_LINEAR;
        let mut font = BitmapFont::new();
        let white = Color::new(1.0, 1.0, 1.0, 1.0);
        renderer.draw_font(&mut font, "Test", 0.0, 0.0, &white);
        // After draw_font, shader should have been switched to TYPE_LINEAR
        assert_eq!(renderer.current_shader, SkinObjectRenderer::TYPE_LINEAR);
        assert_eq!(
            renderer.sprite.shader_type(),
            SkinObjectRenderer::TYPE_LINEAR
        );
    }
}
