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
        } else {
            (self.dst[idx].angle as f32
                + (self.dst[idx + 1].angle - self.dst[idx].angle) as f32 * self.rate)
                as i32
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
