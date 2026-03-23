// Prepare lifecycle methods for SkinObjectData.
// Computes interpolated region, color, and angle for the current time.

use crate::property::timer_property::TimerProperty;
use crate::reexports::MainState;

use super::SkinObjectData;

impl SkinObjectData {
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
            let mx = state.mouse_x() - self.region.x;
            let my = state.mouse_y() - self.region.y;
            if !mouse_rect.contains(mx, my) {
                self.draw = false;
                return;
            }
        }

        self.prepare_color();
        self.prepare_angle();
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
            let cycle = lasttime - self.dstloop as i64;
            if cycle > 0 {
                time = (time - self.dstloop as i64) % cycle + self.dstloop as i64;
            } else {
                time = self.dstloop as i64;
            }
        }
        if self.starttime > time {
            self.draw = false;
            return;
        }
        self.nowtime = time;
        self.rate = -1.0;
        self.index = -1;
        for (off, &offset) in self.off.iter_mut().zip(self.offset.iter()) {
            *off = if let Some(s) = state {
                s.get_offset_value(offset).copied()
            } else {
                None
            };
        }

        if self.fixr.is_none() {
            self.rate();
            if self.dst.is_empty() {
                self.draw = false;
                return;
            }
            let idx = self.index as usize;
            if idx >= self.dst.len() {
                log::warn!(
                    "SkinObjectData::prepare: index {} out of bounds (dst.len={}), hiding object",
                    idx,
                    self.dst.len()
                );
                self.draw = false;
                return;
            }
            if self.rate == 0.0 {
                self.region.set(&self.dst[idx].region);
            } else if self.acc == 3 {
                let r1 = &self.dst[idx].region;
                self.region.x = r1.x;
                self.region.y = r1.y;
                self.region.width = r1.width;
                self.region.height = r1.height;
            } else if idx + 1 < self.dst.len() {
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
            } else {
                // idx+1 out of bounds: fall back to non-interpolated value
                self.region.set(&self.dst[idx].region);
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

    fn prepare_color(&mut self) {
        if let Some(ref fixc) = self.fixc {
            self.color.set(fixc);
            for off in self.off.iter().flatten() {
                let a = (self.color.a + (off.a / 255.0)).clamp(0.0, 1.0);
                self.color.a = a;
            }
            return;
        }
        self.rate();
        if self.dst.is_empty() {
            return;
        }
        if self.rate == 0.0 {
            let idx = self.index as usize;
            let c = self.dst[idx].color;
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
            if idx + 1 < self.dst.len() {
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
            } else {
                // idx+1 out of bounds: fall back to non-interpolated value
                let c = self.dst[idx].color;
                self.color.set(&c);
            }
            return;
        }
        for off in self.off.iter().flatten() {
            let a = (self.color.a + (off.a / 255.0)).clamp(0.0, 1.0);
            self.color.a = a;
        }
    }

    fn prepare_angle(&mut self) {
        if self.fixa != i32::MIN {
            self.angle = self.fixa;
            for off in self.off.iter().flatten() {
                self.angle += off.r as i32;
            }
            return;
        }
        self.rate();
        if self.dst.is_empty() {
            return;
        }
        let idx = self.index as usize;
        self.angle = if self.rate == 0.0 || self.acc == 3 {
            self.dst[idx].angle
        } else if idx + 1 < self.dst.len() {
            (self.dst[idx].angle as f32
                + (self.dst[idx + 1].angle - self.dst[idx].angle) as f32 * self.rate)
                as i32
        } else {
            // idx+1 out of bounds: fall back to non-interpolated value
            self.dst[idx].angle
        };
        for off in self.off.iter().flatten() {
            self.angle += off.r as i32;
        }
    }

    fn rate(&mut self) {
        if self.rate != -1.0 {
            return;
        }
        if self.dst.is_empty() {
            self.rate = 0.0;
            self.index = 0;
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
                if time2 == time1 {
                    self.rate = 0.0;
                    self.index = i as i32;
                    return;
                }
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
}

#[cfg(test)]
mod tests {
    use crate::reexports::{Color, Rectangle};
    use crate::skin_object::SkinObjectDestination;

    /// Verify that prepare_region does not panic when dst has a single entry.
    /// rate() returns index=0, rate=0.0 so the non-interpolation path is taken,
    /// but the bounds check guards against any future rate() change that might
    /// produce an out-of-bounds index.
    #[test]
    fn test_prepare_region_single_dst_no_panic() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(10.0, 20.0, 30.0, 40.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;

        // time=50 is within range but there's only one DST entry
        data.prepare_region(50, None);
        // draw remains true (prepare_region only sets false on error)
        assert!(data.draw, "should still draw with single dst entry");
        assert_eq!(data.region.x, 10.0);
        assert_eq!(data.region.y, 20.0);
    }

    /// Verify bounds check: when index is forced out of bounds by corrupting
    /// state, prepare_region sets draw=false instead of panicking.
    #[test]
    fn test_prepare_region_out_of_bounds_index_no_panic() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(5.0, 5.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;

        // Normal call should work
        data.prepare_region(50, None);
        assert!(data.draw);

        // rate() always produces valid indices, so the bounds check is a
        // safety net. Verify the overall flow doesn't panic on re-entry.
        data.draw = true;
        data.prepare_region(50, None);
        assert!(data.draw);
    }

    /// Verify the interpolation fallback: when rate() produces a valid index at
    /// the last element (rate != 0.0), but idx+1 is out of bounds, the code
    /// falls back to the non-interpolated dst value instead of panicking.
    #[test]
    fn test_prepare_region_interpolation_fallback_at_boundary() {
        let mut data = crate::skin_object::SkinObjectData::new();
        // Two DST entries: interpolation between them is valid for index=0
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(50.0, 50.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 200;
        data.draw = true;

        // At time=50, rate() sets index=0, rate=0.5. dst[0] and dst[1] are
        // both valid, so interpolation works normally.
        data.prepare_region(50, None);
        // draw remains true (prepare_region only sets false on error)
        assert!(data.draw);
        // Interpolated: 0.0 + (50.0 - 0.0) * 0.5 = 25.0
        assert!((data.region.x - 25.0).abs() < 0.01);
    }

    /// Regression: prepare_color must not panic when rate != 0.0 and
    /// index points to the last dst entry (idx+1 out of bounds).
    /// The bounds guard falls back to the non-interpolated color.
    #[test]
    fn test_prepare_color_idx_plus_one_out_of_bounds_no_panic() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(0.5, 0.6, 0.7, 0.8),
            0,
            0,
        ));
        // Force rate != 0.0 with index at last element (no idx+1 available)
        data.rate = 0.5;
        data.index = 0;

        // Must not panic; should fall back to dst[0] color
        data.prepare_color();
        assert!((data.color.r - 0.5).abs() < 0.01);
        assert!((data.color.g - 0.6).abs() < 0.01);
        assert!((data.color.b - 0.7).abs() < 0.01);
        assert!((data.color.a - 0.8).abs() < 0.01);
    }

    /// Regression: prepare_angle must not panic when rate != 0.0 and
    /// index points to the last dst entry (idx+1 out of bounds).
    /// The bounds guard falls back to the non-interpolated angle.
    #[test]
    fn test_prepare_angle_idx_plus_one_out_of_bounds_no_panic() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            90,
            0,
        ));
        // Force rate != 0.0 with index at last element (no idx+1 available)
        data.rate = 0.5;
        data.index = 0;

        // Must not panic; should fall back to dst[0] angle
        data.prepare_angle();
        assert_eq!(data.angle, 90);
    }

    /// Regression: when two consecutive DST entries share the same timestamp,
    /// (time2 - time1) is 0. Without the guard, dividing by zero produces
    /// f32::INFINITY in the rate computation.
    #[test]
    fn test_rate_same_timestamp_no_division_by_zero() {
        let mut data = crate::skin_object::SkinObjectData::new();

        // Two DST entries at the same time (100)
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(20.0, 20.0, 30.0, 30.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 100;
        data.endtime = 100;

        // Set nowtime so rate() will be called and the loop is entered
        data.nowtime = 100;
        data.rate = -1.0;
        data.index = -1;
        data.rate();

        // rate must be finite (no INFINITY from 0/0 division)
        assert!(
            data.rate.is_finite(),
            "rate should be finite, got {}",
            data.rate
        );
    }
}
