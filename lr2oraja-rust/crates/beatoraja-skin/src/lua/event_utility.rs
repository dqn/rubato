use crate::stubs::MainState;

/// Event utility for Lua
///
/// Translated from EventUtility.java (207 lines)
/// Provides Lua utility functions for event operations:
/// - event_observe_turn_true: Execute action when boolean turns true
/// - event_observe_timer: Execute action when timer is set (changes)
/// - event_observe_timer_on: Execute action when timer turns ON
/// - event_observe_timer_off: Execute action when timer turns OFF
/// - event_min_interval: Throttle event execution to minimum interval
pub const TIMER_OFF_VALUE: i64 = i64::MIN;

pub struct EventUtility {
    // Would hold reference to MainState
}

impl EventUtility {
    pub fn new(_state: &dyn MainState) -> Self {
        Self {}
    }

    /// Export event utility functions to a Lua table
    pub fn export(&self, _table: &()) {
        // table.set("event_observe_turn_true", event_observe_turn_true function)
        //   - args: func (() -> boolean), action (() -> void)
        //   - returns: event function (() -> void)
        //   - impl: tracks previous boolean state, calls action when transitions to true
        //   - equivalent to:
        //     local isOn = false
        //     return function()
        //       local on = func()
        //       if isOn ~= on then
        //         isOn = on
        //         if isOn then action() end
        //       end
        //     end

        // table.set("event_observe_timer", event_observe_timer function)
        //   - args: timerFunc (() -> number), action (() -> void)
        //   - returns: event function
        //   - impl: tracks previous timer value, calls action when value changes (and not OFF)

        // table.set("event_observe_timer_on", event_observe_timer_on function)
        //   - args: timerFunc (() -> number), action (() -> void)
        //   - returns: event function
        //   - impl: calls action when timer transitions from OFF to ON

        // table.set("event_observe_timer_off", event_observe_timer_off function)
        //   - args: timerFunc (() -> number), action (() -> void)
        //   - returns: event function
        //   - impl: calls action when timer transitions from ON to OFF

        // table.set("event_min_interval", event_min_interval function)
        //   - args: minInterval (milliseconds), action (() -> void)
        //   - returns: event function
        //   - impl: throttles action to execute at most every minInterval milliseconds
        //   - NOTE: useful for throttling actions in event_observe_timer_on

        todo!("mlua integration: export event utility functions")
    }
}

/// State for event_observe_turn_true
pub struct EventObserveTurnTrueState {
    pub is_on: bool,
}

impl EventObserveTurnTrueState {
    pub fn new() -> Self {
        Self { is_on: false }
    }

    /// Update state and return whether action should be executed
    pub fn update(&mut self, on: bool) -> bool {
        if self.is_on != on {
            self.is_on = on;
            if self.is_on {
                return true;
            }
        }
        false
    }
}

impl Default for EventObserveTurnTrueState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for event_observe_timer
pub struct EventObserveTimerState {
    pub value: i64,
}

impl EventObserveTimerState {
    pub fn new() -> Self {
        Self {
            value: TIMER_OFF_VALUE,
        }
    }

    /// Update state and return whether action should be executed
    pub fn update(&mut self, new_value: i64) -> bool {
        if new_value != self.value && new_value != TIMER_OFF_VALUE {
            self.value = new_value;
            return true;
        }
        false
    }
}

impl Default for EventObserveTimerState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for event_observe_timer_on
pub struct EventObserveTimerOnState {
    pub is_on: bool,
}

impl EventObserveTimerOnState {
    pub fn new() -> Self {
        Self { is_on: false }
    }

    /// Update state and return whether action should be executed
    pub fn update(&mut self, timer_value: i64) -> bool {
        let on = timer_value != TIMER_OFF_VALUE;
        if self.is_on != on {
            self.is_on = on;
            if self.is_on {
                return true;
            }
        }
        false
    }
}

impl Default for EventObserveTimerOnState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for event_observe_timer_off
pub struct EventObserveTimerOffState {
    pub is_off: bool,
}

impl EventObserveTimerOffState {
    pub fn new() -> Self {
        Self { is_off: false }
    }

    /// Update state and return whether action should be executed
    pub fn update(&mut self, timer_value: i64) -> bool {
        let off = timer_value == TIMER_OFF_VALUE;
        if self.is_off != off {
            self.is_off = off;
            if self.is_off {
                return true;
            }
        }
        false
    }
}

impl Default for EventObserveTimerOffState {
    fn default() -> Self {
        Self::new()
    }
}

/// State for event_min_interval
pub struct EventMinIntervalState {
    pub last_execution: i64,
}

impl EventMinIntervalState {
    pub fn new() -> Self {
        Self {
            last_execution: TIMER_OFF_VALUE,
        }
    }

    /// Update state and return whether action should be executed
    /// interval_ms: minimum interval in milliseconds
    /// now_micro_time: current time in microseconds
    pub fn update(&mut self, interval_ms: i32, now_micro_time: i64) -> bool {
        if self.last_execution == TIMER_OFF_VALUE
            || (now_micro_time - self.last_execution) / 1000 >= interval_ms as i64
        {
            self.last_execution = now_micro_time;
            return true;
        }
        false
    }
}

impl Default for EventMinIntervalState {
    fn default() -> Self {
        Self::new()
    }
}
