use crate::stubs::MainState;

/// Timer utility for Lua
///
/// Translated from TimerUtility.java (164 lines)
/// Provides Lua utility functions for timer operations:
/// - now_timer: Get elapsed time from timer value
/// - is_timer_on: Check if timer is ON
/// - is_timer_off: Check if timer is OFF
/// - timer_function: Create timer function from timer ID
/// - timer_observe_boolean: Create timer that observes a boolean function
/// - new_passive_timer: Create passive timer with on/off controls
pub const TIMER_OFF_VALUE: i64 = i64::MIN;

pub struct TimerUtility {
    // Would hold reference to MainState
}

impl TimerUtility {
    pub fn new(_state: &dyn MainState) -> Self {
        Self {}
    }

    /// Export timer utility functions to a Lua table
    pub fn export(&self, _table: &()) {
        // table.set("now_timer", now_timer function)
        //   - arg: timer value (NOT timer ID)
        //   - returns: elapsed time (micro sec) if ON, 0 if OFF
        //   - impl: time != MIN ? getNowMicroTime() - time : 0

        // table.set("is_timer_on", is_timer_on function)
        //   - arg: timer value
        //   - returns: value != i64::MIN

        // table.set("is_timer_off", is_timer_off function)
        //   - arg: timer value
        //   - returns: value == i64::MIN

        // table.set("timer_function", timer_function function)
        //   - arg: timer ID (TIMER_* or custom)
        //   - returns: function () -> number
        //   - impl: creates closure that calls state.timer.getMicroTimer(id)

        // table.set("timer_observe_boolean", timer_observe_boolean function)
        //   - arg: func (() -> boolean)
        //   - returns: function () -> number (timer function)
        //   - impl: tracks previous boolean state, sets timer ON when true, OFF when false

        // table.set("new_passive_timer", new_passive_timer function)
        //   - returns: table { timer, turn_on, turn_on_reset, turn_off }
        //   - timer: () -> number (returns timer value)
        //   - turn_on: () -> true (sets timer ON if not already)
        //   - turn_on_reset: () -> true (sets/resets timer ON)
        //   - turn_off: () -> true (sets timer OFF)

        log::warn!(
            "TimerUtility::export: Lua timer utility export not yet wired (requires MainState lifetime bridging)"
        );
    }
}

/// now_timer: Get elapsed time from timer value
/// arg: timer value (NOT timer ID)
/// returns: elapsed time (micro sec) if ON, 0 if OFF
pub fn now_timer(timer_value: i64, now_micro_time: i64) -> i64 {
    if timer_value != TIMER_OFF_VALUE {
        now_micro_time - timer_value
    } else {
        0
    }
}

/// is_timer_on: Check if timer is ON
pub fn is_timer_on(timer_value: i64) -> bool {
    timer_value != TIMER_OFF_VALUE
}

/// is_timer_off: Check if timer is OFF
pub fn is_timer_off(timer_value: i64) -> bool {
    timer_value == TIMER_OFF_VALUE
}

/// State for timer_observe_boolean
pub struct TimerObserveBooleanState {
    pub timer_value: i64,
}

impl TimerObserveBooleanState {
    pub fn new() -> Self {
        Self {
            timer_value: TIMER_OFF_VALUE,
        }
    }

    /// Update state based on observed boolean value
    pub fn update(&mut self, on: bool, now_micro_time: i64) -> i64 {
        if on && self.timer_value == TIMER_OFF_VALUE {
            self.timer_value = now_micro_time;
        } else if !on && self.timer_value != TIMER_OFF_VALUE {
            self.timer_value = TIMER_OFF_VALUE;
        }
        self.timer_value
    }
}

impl Default for TimerObserveBooleanState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for new_passive_timer
pub struct PassiveTimerState {
    pub timer_value: i64,
}

impl PassiveTimerState {
    pub fn new() -> Self {
        Self {
            timer_value: TIMER_OFF_VALUE,
        }
    }

    pub fn get_timer(&self) -> i64 {
        self.timer_value
    }

    pub fn turn_on(&mut self, now_micro_time: i64) {
        if self.timer_value == TIMER_OFF_VALUE {
            self.timer_value = now_micro_time;
        }
    }

    pub fn turn_on_reset(&mut self, now_micro_time: i64) {
        self.timer_value = now_micro_time;
    }

    pub fn turn_off(&mut self) {
        self.timer_value = TIMER_OFF_VALUE;
    }
}

impl Default for PassiveTimerState {
    fn default() -> Self {
        Self::new()
    }
}
