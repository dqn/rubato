use crate::stubs::MainState;

pub trait TimerProperty: Send + Sync {
    fn get_micro(&self, state: &dyn MainState) -> i64;

    fn get(&self, state: &dyn MainState) -> i64 {
        self.get_micro(state) / 1000
    }

    fn get_now_time(&self, state: &dyn MainState) -> i64 {
        let time = self.get_micro(state);
        if time == i64::MIN {
            0
        } else {
            state.get_timer().get_now_time() - time / 1000
        }
    }

    fn is_on(&self, state: &dyn MainState) -> bool {
        self.get_micro(state) != i64::MIN
    }

    fn is_off(&self, state: &dyn MainState) -> bool {
        self.get_micro(state) == i64::MIN
    }

    /// Returns the timer ID.
    /// For script-defined timers, returns `i32::MIN`.
    fn get_timer_id(&self) -> i32 {
        i32::MIN
    }
}
