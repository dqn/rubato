// Destination setup methods for SkinObjectData.
// Configures animation keyframes, draw conditions, and offset IDs.

use std::collections::HashSet;

use crate::property::boolean_property::BooleanProperty;
use crate::property::boolean_property_factory;
use crate::property::timer_property::TimerPropertyEnum;
use crate::reexports::{Color, Rectangle};
use crate::skin_property;

use super::{CENTERX, CENTERY, DestinationParams, SkinObjectData, SkinObjectDestination};

impl SkinObjectData {
    pub fn set_destination_with_int_timer_and_single_offset(
        &mut self,
        params: &DestinationParams,
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
        self.set_destination_with_timer_and_ops(params, timer_prop, &[op1, op2, op3]);
        self.set_offset_id_single(offset);
    }

    pub fn set_destination_with_int_timer_and_offsets(
        &mut self,
        params: &DestinationParams,
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
        self.set_destination_with_timer_and_ops(params, timer_prop, &[op1, op2, op3]);
        self.set_offset_id(offset);
    }

    pub fn set_destination_with_int_timer_ops(
        &mut self,
        params: &DestinationParams,
        timer: i32,
        op: &[i32],
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(params, timer_prop);
        if self.dstop.is_empty() && self.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    pub fn set_destination_with_int_timer_draw(
        &mut self,
        params: &DestinationParams,
        timer: i32,
        draw_prop: Box<dyn BooleanProperty>,
    ) {
        let timer_prop = if timer > 0 {
            crate::property::timer_property_factory::timer_property(timer)
        } else {
            None
        };
        self.set_destination_core(params, timer_prop);
        self.dstdraw = vec![draw_prop];
    }

    pub fn set_destination_with_timer_ops_and_single_offset(
        &mut self,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        ops: &[i32],
        offset: i32,
    ) {
        self.set_destination_with_timer_and_ops(params, timer, ops);
        self.set_offset_id_single(offset);
    }

    pub fn set_destination_with_timer_ops_and_offsets(
        &mut self,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        ops: &[i32],
        offset: &[i32],
    ) {
        self.set_destination_with_timer_and_ops(params, timer, ops);
        self.set_offset_id(offset);
    }

    pub fn set_destination_with_timer_and_ops(
        &mut self,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        op: &[i32],
    ) {
        self.set_destination_core(params, timer);
        if self.dstop.is_empty() && self.dstdraw.is_empty() {
            self.set_draw_condition_from_ops(op);
        }
    }

    pub fn set_destination_with_timer_draw(
        &mut self,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
        draw_prop: Box<dyn BooleanProperty>,
    ) {
        self.set_destination_core(params, timer);
        self.dstdraw = vec![draw_prop];
    }

    fn set_destination_core(
        &mut self,
        params: &DestinationParams,
        timer: Option<TimerPropertyEnum>,
    ) {
        let obj = SkinObjectDestination::new(
            params.time,
            Rectangle::new(params.x, params.y, params.w, params.h),
            Color::new(
                params.r as f32 / 255.0,
                params.g as f32 / 255.0,
                params.b as f32 / 255.0,
                params.a as f32 / 255.0,
            ),
            params.angle,
            params.acc,
        );
        if self.dst.is_empty() {
            self.fixr = Some(obj.region);
            self.fixc = Some(obj.color);
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
            self.acc = params.acc;
        }
        if self.dstblend == 0 {
            self.dstblend = params.blend;
        }
        if self.dstfilter == 0 {
            self.dstfilter = params.filter;
        }
        if self.dstcenter == 0 && (0..10).contains(&params.center) {
            self.dstcenter = params.center;
            self.centerx = CENTERX[params.center as usize];
            self.centery = CENTERY[params.center as usize];
        }
        if self.dsttimer.is_none() {
            self.dsttimer = timer;
        }
        if self.dstloop == 0 {
            self.dstloop = params.loop_val;
        }
        if let Some(pos) = self.dst.iter().position(|d| d.time > params.time) {
            self.dst.insert(pos, obj);
        } else {
            self.dst.push(obj);
        }
        self.starttime = self.dst[0].time;
        self.endtime = self.dst[self.dst.len() - 1].time;
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
        self.dstop = op;
        self.dstdraw = draw;
    }

    pub fn set_offset_id_single(&mut self, offset: i32) {
        self.set_offset_id(&[offset]);
    }

    pub fn set_offset_id(&mut self, offset: &[i32]) {
        if !self.offset.is_empty() {
            return;
        }
        let mut seen = Vec::new();
        for &o in offset {
            if o > 0 && o < skin_property::OFFSET_MAX + 1 && !seen.contains(&o) {
                seen.push(o);
            }
        }
        if !seen.is_empty() {
            self.offset = seen;
            self.off = vec![None; self.offset.len()];
        }
    }
}
