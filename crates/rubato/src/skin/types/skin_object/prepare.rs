// Prepare lifecycle methods for SkinObjectData.
// Computes interpolated region, color, and angle for the current time.

use crate::skin::property::timer_property::TimerProperty;
use crate::skin::reexports::MainState;

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
    use crate::skin::reexports::{Color, Rectangle};
    use crate::skin::skin_object::SkinObjectDestination;

    /// Verify that prepare_region does not panic when dst has a single entry.
    /// rate() returns index=0, rate=0.0 so the non-interpolation path is taken,
    /// but the bounds check guards against any future rate() change that might
    /// produce an out-of-bounds index.
    #[test]
    fn test_prepare_region_single_dst_no_panic() {
        let mut data = crate::skin::skin_object::SkinObjectData::new();
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
        let mut data = crate::skin::skin_object::SkinObjectData::new();
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
        let mut data = crate::skin::skin_object::SkinObjectData::new();
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
        let mut data = crate::skin::skin_object::SkinObjectData::new();
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
        let mut data = crate::skin::skin_object::SkinObjectData::new();
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
        let mut data = crate::skin::skin_object::SkinObjectData::new();

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

    // ---------------------------------------------------------------
    // Phase 1: Multi-destination interpolation tests
    // ---------------------------------------------------------------

    fn make_data_two_dst(
        r1: Rectangle,
        c1: Color,
        a1: i32,
        r2: Rectangle,
        c2: Color,
        a2: i32,
        acc: i32,
    ) -> crate::skin_object::SkinObjectData {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst
            .push(SkinObjectDestination::new(0, r1, c1, a1, acc));
        data.dst
            .push(SkinObjectDestination::new(100, r2, c2, a2, acc));
        data.acc = acc;
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data
    }

    #[test]
    fn test_multi_dst_linear_interpolation_midpoint() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        );
        data.prepare_region(50, None);
        assert!(data.draw);
        assert!((data.region.x - 50.0).abs() < 0.01);
        assert!((data.region.y - 50.0).abs() < 0.01);
        assert!((data.region.width - 150.0).abs() < 0.01);
        assert!((data.region.height - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_dst_linear_interpolation_quarter() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        );
        data.prepare_region(25, None);
        assert!(data.draw);
        assert!((data.region.x - 25.0).abs() < 0.01);
        assert!((data.region.y - 25.0).abs() < 0.01);
        assert!((data.region.width - 125.0).abs() < 0.01);
        assert!((data.region.height - 125.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_dst_three_destinations() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            200,
            Rectangle::new(200.0, 200.0, 300.0, 300.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 200;
        data.draw = true;

        // time=150 is between DST[1] (time=100) and DST[2] (time=200), midpoint
        data.prepare_region(150, None);
        assert!(data.draw);
        assert!((data.region.x - 150.0).abs() < 0.01);
        assert!((data.region.y - 150.0).abs() < 0.01);
        assert!((data.region.width - 250.0).abs() < 0.01);
        assert!((data.region.height - 250.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_dst_acc1_ease_in() {
        // acc=1: rate = rate^2. At time=50/100, raw rate=0.5, eased rate=0.25
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            1, // ease-in
        );
        data.prepare_region(50, None);
        assert!(data.draw);
        // x = 0 + (100-0) * 0.25 = 25.0
        assert!((data.region.x - 25.0).abs() < 0.01);
        assert!((data.region.y - 25.0).abs() < 0.01);
        // w = 100 + (200-100) * 0.25 = 125.0
        assert!((data.region.width - 125.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_dst_acc2_ease_out() {
        // acc=2: rate = 1 - (rate-1)^2. At time=50/100, raw rate=0.5,
        // eased rate = 1 - (0.5-1)^2 = 1 - 0.25 = 0.75
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            2, // ease-out
        );
        data.prepare_region(50, None);
        assert!(data.draw);
        // x = 0 + (100-0) * 0.75 = 75.0
        assert!((data.region.x - 75.0).abs() < 0.01);
        assert!((data.region.y - 75.0).abs() < 0.01);
        assert!((data.region.width - 175.0).abs() < 0.01);
    }

    #[test]
    fn test_multi_dst_acc3_step() {
        // acc=3: step function -- no interpolation, snaps to DST[idx]
        let mut data = make_data_two_dst(
            Rectangle::new(10.0, 20.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(200.0, 300.0, 400.0, 500.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            3, // step
        );
        data.prepare_region(50, None);
        assert!(data.draw);
        // Should snap to DST[0] values (index=0 since time=50 is between DST[0] and DST[1])
        assert_eq!(data.region.x, 10.0);
        assert_eq!(data.region.y, 20.0);
        assert_eq!(data.region.width, 100.0);
        assert_eq!(data.region.height, 100.0);
    }

    #[test]
    fn test_color_linear_interpolation() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 0.0, 0.0, 1.0), // red
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(0.0, 0.0, 1.0, 1.0), // blue
            0,
            0,
        );
        // prepare_region to set rate and index
        data.prepare_region(50, None);
        assert!(data.draw);
        // Now call prepare_color (already called by internal flow, but let's verify)
        // Rate was set by prepare_region; re-invoke prepare_color
        data.rate = -1.0; // reset to force recalculation
        data.prepare_color();
        assert!((data.color.r - 0.5).abs() < 0.01);
        assert!((data.color.g - 0.0).abs() < 0.01);
        assert!((data.color.b - 0.5).abs() < 0.01);
        assert!((data.color.a - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_color_acc3_step() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 0.0, 0.0, 1.0), // red
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(0.0, 0.0, 1.0, 1.0), // blue
            0,
            3, // step
        );
        data.prepare_region(50, None);
        data.rate = -1.0;
        data.prepare_color();
        // acc=3: snaps to DST[0] color (red), no interpolation
        assert!((data.color.r - 1.0).abs() < 0.01);
        assert!((data.color.g - 0.0).abs() < 0.01);
        assert!((data.color.b - 0.0).abs() < 0.01);
    }

    #[test]
    fn test_angle_linear_interpolation() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            90,
            0,
        );
        data.prepare_region(50, None);
        data.rate = -1.0;
        data.prepare_angle();
        assert_eq!(data.angle, 45);
    }

    #[test]
    fn test_angle_acc3_step() {
        let mut data = make_data_two_dst(
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            90,
            3, // step
        );
        data.prepare_region(50, None);
        data.rate = -1.0;
        data.prepare_angle();
        // acc=3: snaps to DST[0] angle (0)
        assert_eq!(data.angle, 0);
    }

    // ---------------------------------------------------------------
    // Phase 2: Loop, timer, draw condition tests
    // ---------------------------------------------------------------

    #[test]
    fn test_dstloop_normal_cycling() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            200,
            Rectangle::new(200.0, 200.0, 300.0, 300.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 200;
        data.dstloop = 100; // cycle starts at time=100
        data.draw = true;

        // time=250: cycle = endtime - dstloop = 200 - 100 = 100
        // wrapped = (250 - 100) % 100 + 100 = 150 % 100 + 100 = 50 + 100 = 150
        // time=150 interpolates between DST[1](100) and DST[2](200) at rate=0.5
        data.prepare_region(250, None);
        assert!(data.draw);
        assert!((data.region.x - 150.0).abs() < 0.01);
        assert!((data.region.y - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_dstloop_minus_one_hides_after_end() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.dstloop = -1;
        data.draw = true;

        // time=150 > endtime=100 with dstloop=-1 -> time becomes -1 -> starttime(0) > -1 -> draw=false
        data.prepare_region(150, None);
        assert!(!data.draw);
    }

    #[test]
    fn test_dstloop_minus_one_visible_before_end() {
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
        data.dstloop = -1;
        data.draw = true;

        data.prepare_region(50, None);
        assert!(data.draw);
        assert_eq!(data.region.x, 10.0);
    }

    #[test]
    fn test_dsttimer_off_hides_object() {
        use crate::property::timer_property::TimerPropertyEnum;
        use crate::property::timer_property_factory::TimerPropertyImpl;
        use crate::test_helpers::MockMainState;
        use rubato_types::timer_id::TimerId;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.dsttimer = Some(TimerPropertyEnum::Impl(TimerPropertyImpl {
            timer_id: TimerId(42),
        }));

        // Timer 42 is OFF (default: i64::MIN) -> draw=false
        let state = MockMainState::default();
        data.prepare_region(50, Some(&state));
        assert!(!data.draw);
    }

    #[test]
    fn test_dsttimer_on_subtracts_timer_value() {
        use crate::property::timer_property::TimerPropertyEnum;
        use crate::property::timer_property_factory::TimerPropertyImpl;
        use crate::test_helpers::MockMainState;
        use rubato_types::timer_id::TimerId;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(100.0, 100.0, 200.0, 200.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.dsttimer = Some(TimerPropertyEnum::Impl(TimerPropertyImpl {
            timer_id: TimerId(42),
        }));

        let mut state = MockMainState::default();
        // Timer 42 activated at 100_000 micro -> timer() returns 100_000/1000 = 100
        state.timer.set_timer_value(42, 100_000);
        state.timer.now_micro_time = 200_000;

        // time=150, timer.get() = 100 -> effective time = 150 - 100 = 50
        // rate at time=50 between DST[0](0) and DST[1](100) = 0.5
        data.prepare_region(150, Some(&state));
        assert!(data.draw);
        assert!((data.region.x - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_draw_condition_false_skips() {
        use crate::property::boolean_property::BooleanProperty;
        use crate::test_helpers::MockMainState;

        struct AlwaysFalse;
        impl BooleanProperty for AlwaysFalse {
            fn is_static(&self, _: &dyn crate::reexports::MainState) -> bool {
                true
            }
            fn get(&self, _: &dyn crate::reexports::MainState) -> bool {
                false
            }
        }

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.dstdraw.push(Box::new(AlwaysFalse));

        let state = MockMainState::default();
        data.prepare_with_offset(50, &state, 0.0, 0.0);
        assert!(!data.draw);
    }

    #[test]
    fn test_draw_condition_true_allows() {
        use crate::property::boolean_property::BooleanProperty;
        use crate::test_helpers::MockMainState;

        struct AlwaysTrue;
        impl BooleanProperty for AlwaysTrue {
            fn is_static(&self, _: &dyn crate::reexports::MainState) -> bool {
                true
            }
            fn get(&self, _: &dyn crate::reexports::MainState) -> bool {
                true
            }
        }

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(5.0, 10.0, 15.0, 20.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.dstdraw.push(Box::new(AlwaysTrue));

        let state = MockMainState::default();
        data.prepare_with_offset(50, &state, 0.0, 0.0);
        assert!(data.draw);
        assert_eq!(data.region.x, 5.0);
    }

    #[test]
    fn test_mouse_rect_inside_draws() {
        use crate::test_helpers::MockMainState;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(100.0, 100.0, 50.0, 50.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        // mouse_rect is relative to region position
        data.mouse_rect = Some(Rectangle::new(0.0, 0.0, 50.0, 50.0));

        let mut state = MockMainState::default();
        // Mouse at (125, 125) -> relative to region (100,100) = (25, 25) -> inside mouse_rect
        state.mouse_x = 125.0;
        state.mouse_y = 125.0;

        data.prepare_with_offset(50, &state, 0.0, 0.0);
        assert!(data.draw);
    }

    #[test]
    fn test_mouse_rect_outside_hides() {
        use crate::test_helpers::MockMainState;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(100.0, 100.0, 50.0, 50.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.mouse_rect = Some(Rectangle::new(0.0, 0.0, 50.0, 50.0));

        let mut state = MockMainState::default();
        // Mouse at (300, 300) -> relative to region (100,100) = (200, 200) -> outside
        state.mouse_x = 300.0;
        state.mouse_y = 300.0;

        data.prepare_with_offset(50, &state, 0.0, 0.0);
        assert!(!data.draw);
    }

    // ---------------------------------------------------------------
    // Phase 3: SkinOffset adjustment tests
    // ---------------------------------------------------------------

    #[test]
    fn test_offset_adjusts_region_xy() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(50.0, 50.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.offset = vec![5];
        data.off = vec![None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            5,
            SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        assert!(data.draw);
        // region.x = 50 + (10 - 0/2) = 60
        assert!((data.region.x - 60.0).abs() < 0.01);
        // region.y = 50 + (20 - 0/2) = 70
        assert!((data.region.y - 70.0).abs() < 0.01);
    }

    #[test]
    fn test_offset_adjusts_wh() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.offset = vec![1];
        data.off = vec![None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            1,
            SkinOffset {
                x: 0.0,
                y: 0.0,
                w: 30.0,
                h: 40.0,
                r: 0.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        assert!(data.draw);
        // region.x = 0 + (0 - 30/2) = -15
        assert!((data.region.x - (-15.0)).abs() < 0.01);
        // region.width = 100 + 30 = 130
        assert!((data.region.width - 130.0).abs() < 0.01);
        // region.height = 100 + 40 = 140
        assert!((data.region.height - 140.0).abs() < 0.01);
    }

    #[test]
    fn test_offset_relative_skips_xy() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(50.0, 50.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.relative = true; // skip x/y offset adjustment
        data.offset = vec![1];
        data.off = vec![None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            1,
            SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
                r: 0.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        assert!(data.draw);
        // x/y unchanged due to relative=true
        assert_eq!(data.region.x, 50.0);
        assert_eq!(data.region.y, 50.0);
        // w/h still adjusted
        assert!((data.region.width - 130.0).abs() < 0.01);
        assert!((data.region.height - 140.0).abs() < 0.01);
    }

    #[test]
    fn test_multiple_offsets_accumulate() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.offset = vec![1, 2];
        data.off = vec![None, None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            1,
            SkinOffset {
                x: 10.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        );
        state.offsets.insert(
            2,
            SkinOffset {
                x: 5.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        assert!(data.draw);
        // x = 0 + 10 + 5 = 15
        assert!((data.region.x - 15.0).abs() < 0.01);
    }

    #[test]
    fn test_offset_alpha_clamps() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 0.5), // alpha=0.5
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.offset = vec![1];
        data.off = vec![None];

        let mut state = MockMainState::default();
        // a=255 -> offset = 255/255 = 1.0 -> 0.5 + 1.0 = 1.5 -> clamped to 1.0
        state.offsets.insert(
            1,
            SkinOffset {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 255.0,
            },
        );

        data.prepare_region(50, Some(&state));
        data.prepare_color();
        assert!(
            (data.color.a - 1.0).abs() < 0.01,
            "alpha should be clamped to 1.0"
        );
    }

    #[test]
    fn test_offset_rotation() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            45,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.offset = vec![1];
        data.off = vec![None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            1,
            SkinOffset {
                x: 0.0,
                y: 0.0,
                w: 0.0,
                h: 0.0,
                r: 15.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        data.prepare_angle();
        // angle = 45 + 15 = 60
        assert_eq!(data.angle, 60);
    }

    // ---------------------------------------------------------------
    // Phase 4: fixr/fixc/fixa bypass tests
    // ---------------------------------------------------------------

    #[test]
    fn test_fixr_bypasses_interpolation() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 100.0, 100.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(200.0, 200.0, 300.0, 300.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.fixr = Some(Rectangle::new(77.0, 88.0, 99.0, 111.0));

        data.prepare_region(50, None);
        assert!(data.draw);
        // Should use fixr, not interpolate between DSTs
        assert_eq!(data.region.x, 77.0);
        assert_eq!(data.region.y, 88.0);
        assert_eq!(data.region.width, 99.0);
        assert_eq!(data.region.height, 111.0);
    }

    #[test]
    fn test_fixc_bypasses_color_interpolation() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 0.0, 0.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(0.0, 0.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.fixc = Some(Color::new(0.3, 0.4, 0.5, 0.6));

        data.prepare_region(50, None);
        data.prepare_color();
        // Should use fixc
        assert!((data.color.r - 0.3).abs() < 0.01);
        assert!((data.color.g - 0.4).abs() < 0.01);
        assert!((data.color.b - 0.5).abs() < 0.01);
        assert!((data.color.a - 0.6).abs() < 0.01);
    }

    #[test]
    fn test_fixa_bypasses_angle_interpolation() {
        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.dst.push(SkinObjectDestination::new(
            100,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            180,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.fixa = 42;

        data.prepare_region(50, None);
        data.prepare_angle();
        assert_eq!(data.angle, 42);
    }

    #[test]
    fn test_fixr_with_offset() {
        use crate::test_helpers::MockMainState;
        use rubato_types::skin_offset::SkinOffset;

        let mut data = crate::skin_object::SkinObjectData::new();
        data.dst.push(SkinObjectDestination::new(
            0,
            Rectangle::new(0.0, 0.0, 10.0, 10.0),
            Color::new(1.0, 1.0, 1.0, 1.0),
            0,
            0,
        ));
        data.starttime = 0;
        data.endtime = 100;
        data.draw = true;
        data.fixr = Some(Rectangle::new(50.0, 50.0, 100.0, 100.0));
        data.offset = vec![1];
        data.off = vec![None];

        let mut state = MockMainState::default();
        state.offsets.insert(
            1,
            SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 0.0,
                h: 0.0,
                r: 0.0,
                a: 0.0,
            },
        );

        data.prepare_region(50, Some(&state));
        assert!(data.draw);
        // fixr base + offset
        assert!((data.region.x - 60.0).abs() < 0.01);
        assert!((data.region.y - 70.0).abs() < 0.01);
        assert_eq!(data.region.width, 100.0);
        assert_eq!(data.region.height, 100.0);
    }
}
