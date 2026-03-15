use crate::property::timer_property::{TimerProperty, TimerPropertyEnum};
use crate::reexports::MainState;

/// Custom timer definition
///
/// Translated from CustomTimer.java
pub struct CustomTimer {
    pub id: i32,
    timer_func: Option<TimerPropertyEnum>,
    time: i64,
}

impl CustomTimer {
    pub fn new(id: i32, timer_func: Option<TimerPropertyEnum>) -> Self {
        Self {
            id,
            timer_func,
            time: i64::MIN,
        }
    }

    pub fn is_passive(&self) -> bool {
        self.timer_func.is_none()
    }

    pub fn micro_timer(&self) -> i64 {
        self.time
    }

    pub fn set_micro_timer(&mut self, time: i64) {
        if self.timer_func.is_some() {
            return;
        }
        self.time = time;
    }

    pub fn update(&mut self, state: &dyn MainState) {
        if let Some(ref timer_func) = self.timer_func {
            self.time = timer_func.get_micro(state);
        }
    }
}
