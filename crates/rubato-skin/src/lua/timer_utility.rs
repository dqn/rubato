use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::reexports::MainState;
use rubato_types::sync_utils::lock_or_recover;

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

/// Wrapper for raw MainState pointer to implement Send/Sync.
///
/// # Accepted design trade-off: lifetime erasure via transmute
///
/// See EventUtility::StatePtr for the full rationale. The raw pointer's lifetime
/// is erased to 'static because Lua closures cannot carry non-'static references.
/// Safety invariant: MainState must outlive the Arc<Lua> VM and all exported closures.
#[derive(Clone, Copy)]
struct StatePtr(*const dyn MainState);
// SAFETY: StatePtr contains a *const dyn MainState raw pointer, which is !Send and !Sync
// by default. The pointer is only dereferenced within Lua closures that run on a single
// thread (beatoraja's skin system is single-threaded). The caller of TimerUtility::new()
// guarantees the MainState outlives the TimerUtility and all Lua closures exported from it.
unsafe impl Send for StatePtr {}
unsafe impl Sync for StatePtr {}

pub struct TimerUtility {
    state_ptr: StatePtr,
}

impl TimerUtility {
    pub fn new(state: &dyn MainState) -> Self {
        let ptr: *const dyn MainState = state;
        // SAFETY: Transmute erases the trait object lifetime ('a -> 'static).
        // The caller guarantees that state outlives TimerUtility and its Lua closures.
        // Only read access (*const) is used -- no aliasing violations.
        let ptr: *const dyn MainState = unsafe { std::mem::transmute(ptr) };
        Self {
            state_ptr: StatePtr(ptr),
        }
    }

    /// Export timer utility functions to a Lua table
    pub fn export(&self, lua: &Lua, table: &LuaTable) {
        let result: Result<(), LuaError> = (|| {
            let sp = self.state_ptr;

            // now_timer(timer_value) -> elapsed micro sec (0 if OFF)
            let now_timer_func = lua.create_function(move |_, timer_value: i64| {
                let state = unsafe { &*sp.0 };
                Ok(now_timer(timer_value, state.now_micro_time()))
            })?;
            table.set("now_timer", now_timer_func)?;

            // is_timer_on(timer_value) -> boolean
            let is_timer_on_func =
                lua.create_function(|_, timer_value: i64| Ok(is_timer_on(timer_value)))?;
            table.set("is_timer_on", is_timer_on_func)?;

            // is_timer_off(timer_value) -> boolean
            let is_timer_off_func =
                lua.create_function(|_, timer_value: i64| Ok(is_timer_off(timer_value)))?;
            table.set("is_timer_off", is_timer_off_func)?;

            // timer_function(timer_id) -> function() -> number
            let sp = self.state_ptr;
            let timer_function_func = lua.create_function(move |lua, timer_id: i32| {
                let tid = rubato_types::timer_id::TimerId::new(timer_id);
                let timer_func = lua.create_function(move |_, ()| {
                    let state = unsafe { &*sp.0 };
                    Ok(state.micro_timer(tid))
                })?;
                Ok(timer_func)
            })?;
            table.set("timer_function", timer_function_func)?;

            // timer_observe_boolean(func) -> function() -> number (timer function)
            let sp = self.state_ptr;
            let timer_observe_boolean_func =
                lua.create_function(move |lua, func: LuaFunction| {
                    let observe_state = Arc::new(Mutex::new(TimerObserveBooleanState::new()));
                    let timer_func = lua.create_function(move |_, ()| {
                        let state = unsafe { &*sp.0 };
                        let on: bool = func.call(()).unwrap_or(false);
                        let mut obs = lock_or_recover(&observe_state);
                        Ok(obs.update(on, state.now_micro_time()))
                    })?;
                    Ok(timer_func)
                })?;
            table.set("timer_observe_boolean", timer_observe_boolean_func)?;

            // new_passive_timer() -> table { timer, turn_on, turn_on_reset, turn_off }
            let sp = self.state_ptr;
            let new_passive_timer_func = lua.create_function(move |lua, ()| {
                let passive_state = Arc::new(Mutex::new(PassiveTimerState::new()));
                let tbl = lua.create_table()?;

                // timer() -> number
                let ps = passive_state.clone();
                let timer_func = lua.create_function(move |_, ()| {
                    let ps = lock_or_recover(&ps);
                    Ok(ps.timer())
                })?;
                tbl.set("timer", timer_func)?;

                // turn_on() -> true
                let ps = passive_state.clone();
                let turn_on_func = lua.create_function(move |_, ()| {
                    let state = unsafe { &*sp.0 };
                    let mut ps = lock_or_recover(&ps);
                    ps.turn_on(state.now_micro_time());
                    Ok(true)
                })?;
                tbl.set("turn_on", turn_on_func)?;

                // turn_on_reset() -> true
                let ps = passive_state.clone();
                let turn_on_reset_func = lua.create_function(move |_, ()| {
                    let state = unsafe { &*sp.0 };
                    let mut ps = lock_or_recover(&ps);
                    ps.turn_on_reset(state.now_micro_time());
                    Ok(true)
                })?;
                tbl.set("turn_on_reset", turn_on_reset_func)?;

                // turn_off() -> true
                let ps = passive_state.clone();
                let turn_off_func = lua.create_function(move |_, ()| {
                    let mut ps = lock_or_recover(&ps);
                    ps.turn_off();
                    Ok(true)
                })?;
                tbl.set("turn_off", turn_off_func)?;

                Ok(tbl)
            })?;
            table.set("new_passive_timer", new_passive_timer_func)?;

            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("TimerUtility::export failed: {}", e);
        }
    }
}

/// now_timer: Get elapsed time from timer value
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

    pub fn timer(&self) -> i64 {
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
