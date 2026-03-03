use std::sync::{Arc, Mutex};

use mlua::prelude::*;

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

/// Wrapper for raw MainState pointer to implement Send/Sync.
#[derive(Clone, Copy)]
struct StatePtr(*const dyn MainState);
// SAFETY: StatePtr contains a *const dyn MainState raw pointer, which is !Send and !Sync
// by default. The pointer is only dereferenced within Lua closures that run on a single
// thread (beatoraja's skin system is single-threaded). The caller of EventUtility::new()
// guarantees the MainState outlives the EventUtility and all Lua closures exported from it.
unsafe impl Send for StatePtr {}
unsafe impl Sync for StatePtr {}

pub struct EventUtility {
    state_ptr: StatePtr,
}

impl EventUtility {
    pub fn new(state: &dyn MainState) -> Self {
        let ptr: *const dyn MainState = state;
        // SAFETY: Transmute erases the trait object lifetime ('a -> 'static).
        // The caller guarantees that state outlives EventUtility and its Lua closures.
        // Only read access (*const) is used -- no aliasing violations.
        let ptr: *const dyn MainState = unsafe { std::mem::transmute(ptr) };
        Self {
            state_ptr: StatePtr(ptr),
        }
    }

    /// Export event utility functions to a Lua table
    pub fn export(&self, lua: &Lua, table: &LuaTable) {
        let result: Result<(), LuaError> = (|| {
            // event_observe_turn_true(func, action) -> event function
            let event_observe_turn_true_func =
                lua.create_function(|lua, (func, action): (LuaFunction, LuaFunction)| {
                    let observe_state = Arc::new(Mutex::new(EventObserveTurnTrueState::new()));
                    let event_func = lua.create_function(move |_, ()| {
                        let on: bool = func.call(()).unwrap_or(false);
                        let mut obs = observe_state.lock().unwrap();
                        if obs.update(on)
                            && let Err(e) = action.call::<LuaValue>(())
                        {
                            log::warn!("Lua callback error: {e}");
                        }
                        Ok(())
                    })?;
                    Ok(event_func)
                })?;
            table.set("event_observe_turn_true", event_observe_turn_true_func)?;

            // event_observe_timer(timer_func, action) -> event function
            let event_observe_timer_func =
                lua.create_function(|lua, (timer_func, action): (LuaFunction, LuaFunction)| {
                    let observe_state = Arc::new(Mutex::new(EventObserveTimerState::new()));
                    let event_func = lua.create_function(move |_, ()| {
                        let value: i64 = timer_func.call(()).unwrap_or(TIMER_OFF_VALUE);
                        let mut obs = observe_state.lock().unwrap();
                        if obs.update(value)
                            && let Err(e) = action.call::<LuaValue>(())
                        {
                            log::warn!("Lua callback error: {e}");
                        }
                        Ok(())
                    })?;
                    Ok(event_func)
                })?;
            table.set("event_observe_timer", event_observe_timer_func)?;

            // event_observe_timer_on(timer_func, action) -> event function
            let event_observe_timer_on_func =
                lua.create_function(|lua, (timer_func, action): (LuaFunction, LuaFunction)| {
                    let observe_state = Arc::new(Mutex::new(EventObserveTimerOnState::new()));
                    let event_func = lua.create_function(move |_, ()| {
                        let value: i64 = timer_func.call(()).unwrap_or(TIMER_OFF_VALUE);
                        let mut obs = observe_state.lock().unwrap();
                        if obs.update(value)
                            && let Err(e) = action.call::<LuaValue>(())
                        {
                            log::warn!("Lua callback error: {e}");
                        }
                        Ok(())
                    })?;
                    Ok(event_func)
                })?;
            table.set("event_observe_timer_on", event_observe_timer_on_func)?;

            // event_observe_timer_off(timer_func, action) -> event function
            let event_observe_timer_off_func =
                lua.create_function(|lua, (timer_func, action): (LuaFunction, LuaFunction)| {
                    let observe_state = Arc::new(Mutex::new(EventObserveTimerOffState::new()));
                    let event_func = lua.create_function(move |_, ()| {
                        let value: i64 = timer_func.call(()).unwrap_or(TIMER_OFF_VALUE);
                        let mut obs = observe_state.lock().unwrap();
                        if obs.update(value)
                            && let Err(e) = action.call::<LuaValue>(())
                        {
                            log::warn!("Lua callback error: {e}");
                        }
                        Ok(())
                    })?;
                    Ok(event_func)
                })?;
            table.set("event_observe_timer_off", event_observe_timer_off_func)?;

            // event_min_interval(min_interval_ms, action) -> event function
            let sp = self.state_ptr;
            let event_min_interval_func =
                lua.create_function(move |lua, (min_interval, action): (i32, LuaFunction)| {
                    let interval_state = Arc::new(Mutex::new(EventMinIntervalState::new()));
                    let event_func = lua.create_function(move |_, ()| {
                        let state = unsafe { &*sp.0 };
                        let now = state.get_timer().get_now_micro_time();
                        let mut is = interval_state.lock().unwrap();
                        if is.update(min_interval, now)
                            && let Err(e) = action.call::<LuaValue>(())
                        {
                            log::warn!("Lua callback error: {e}");
                        }
                        Ok(())
                    })?;
                    Ok(event_func)
                })?;
            table.set("event_min_interval", event_min_interval_func)?;

            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("EventUtility::export failed: {}", e);
        }
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
