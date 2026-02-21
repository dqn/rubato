use super::timer_property::TimerProperty;
use crate::stubs::MainState;

/// Returns a TimerProperty for the given timer ID.
/// Returns None if timer_id is negative.
pub fn get_timer_property(timer_id: i32) -> Option<Box<dyn TimerProperty>> {
    if timer_id < 0 {
        return None;
    }

    Some(Box::new(TimerPropertyImpl { timer_id }))
}

struct TimerPropertyImpl {
    timer_id: i32,
}

impl TimerProperty for TimerPropertyImpl {
    fn get_micro(&self, state: &dyn MainState) -> i64 {
        state.get_timer().get_micro_timer(self.timer_id)
    }

    fn get(&self, state: &dyn MainState) -> i64 {
        state.get_timer().get_timer(self.timer_id)
    }

    fn get_now_time(&self, state: &dyn MainState) -> i64 {
        state.get_timer().get_now_time_for(self.timer_id)
    }

    fn is_on(&self, state: &dyn MainState) -> bool {
        state.get_timer().is_timer_on(self.timer_id)
    }

    fn is_off(&self, state: &dyn MainState) -> bool {
        !state.get_timer().is_timer_on(self.timer_id)
    }

    fn get_timer_id(&self) -> i32 {
        self.timer_id
    }
}
