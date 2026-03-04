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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_helpers::MockMainState;

    /// Timer property must return real timer values when a timer is set.
    /// Before the fix, get_micro_timer always returned 0 (frozen animations).
    #[test]
    fn test_timer_property_returns_real_micro_timer_value() {
        let timer_id = 10;
        let prop = get_timer_property(timer_id).unwrap();

        // Set up a state where timer 10 is ON at micro-time 500_000
        let mut state = MockMainState::default();
        state.timer.now_time = 1000;
        state.timer.now_micro_time = 1_000_000;
        state.timer.set_timer_value(timer_id, 500_000);

        // Timer 10 should be ON
        assert!(prop.is_on(&state), "Timer {} should be ON", timer_id);
        assert!(!prop.is_off(&state), "Timer {} should not be OFF", timer_id);

        // get_micro should return the activation time (500_000), not 0
        assert_eq!(prop.get_micro(&state), 500_000);

        // get should return activation time / 1000 = 500
        assert_eq!(prop.get(&state), 500);

        // get_now_time should return elapsed time: (now - activation) / 1000 = 500
        assert_eq!(prop.get_now_time(&state), 500);
    }

    /// Timer property for an OFF timer must return i64::MIN and report off.
    #[test]
    fn test_timer_property_off_timer_returns_min() {
        let timer_id = 42;
        let prop = get_timer_property(timer_id).unwrap();

        let state = MockMainState::default();
        // Timer 42 is never set, should be OFF (i64::MIN)

        assert!(prop.is_off(&state), "Unset timer should be OFF");
        assert!(!prop.is_on(&state), "Unset timer should not be ON");
        assert_eq!(prop.get_micro(&state), i64::MIN);
        assert_eq!(prop.get_now_time(&state), 0);
    }
}
