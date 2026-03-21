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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_params() -> DestinationParams {
        DestinationParams {
            time: 0,
            x: 0.0,
            y: 0.0,
            w: 100.0,
            h: 100.0,
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
        }
    }

    /// Regression: DST command values[18-20] must wire as draw conditions (dstdraw),
    /// and read_offset values must wire as offset IDs.
    /// A prior bug swapped these parameters, causing draw conditions to be lost
    /// and offsets to be misinterpreted.
    #[test]
    fn set_destination_wires_draw_conditions_and_offsets() {
        let mut data = SkinObjectData::default();

        // op1=1 (OPTION_FOLDERBAR), op2=2 (OPTION_SONGBAR), op3=0 (no condition)
        // These are known boolean property IDs that should produce dstdraw entries.
        let op1 = 1;
        let op2 = 2;
        let op3 = 0;
        let offsets = vec![10, 30]; // OFFSET_ALL, OFFSET_NOTES_1P

        data.set_destination_with_int_timer_and_offsets(
            &default_params(),
            0,
            op1,
            op2,
            op3,
            &offsets,
        );

        assert_eq!(
            data.dstdraw.len(),
            2,
            "expected 2 draw conditions from op1={op1}, op2={op2}; got {}",
            data.dstdraw.len()
        );

        assert_eq!(
            data.offset,
            vec![10, 30],
            "expected offset IDs [10, 30]; got {:?}",
            data.offset
        );
        assert_eq!(data.off.len(), 2, "off slots must match offset count");
    }

    #[test]
    fn set_draw_condition_deduplicates_ops() {
        let mut data = SkinObjectData::default();
        data.set_draw_condition_from_ops(&[1, 1, 2]);
        assert_eq!(
            data.dstdraw.len(),
            2,
            "duplicate op IDs should be deduplicated"
        );
    }

    #[test]
    fn set_offset_id_rejects_out_of_range() {
        let mut data = SkinObjectData::default();
        data.set_offset_id(&[0, -1, 200, 50]);
        assert_eq!(data.offset, vec![50]);
        assert_eq!(data.off.len(), 1);
    }

    #[test]
    fn set_offset_id_does_not_overwrite() {
        let mut data = SkinObjectData::default();
        data.set_offset_id(&[10]);
        assert_eq!(data.offset, vec![10]);

        data.set_offset_id(&[20, 30]);
        assert_eq!(data.offset, vec![10], "offset should not be overwritten");
    }

    #[test]
    fn set_draw_condition_negative_id_produces_negated_property() {
        let mut data = SkinObjectData::default();
        data.set_draw_condition_from_ops(&[-1]);
        assert_eq!(
            data.dstdraw.len(),
            1,
            "negative ID should produce a negated draw condition"
        );
    }
}
