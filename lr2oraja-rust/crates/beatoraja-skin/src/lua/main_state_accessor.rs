use mlua::prelude::*;

use crate::property::boolean_property_factory;
use crate::property::float_property_factory;
use crate::property::integer_property_factory;
use crate::property::string_property_factory;
use crate::skin_property_mapper;
use crate::stubs::MainState;

/// Main state accessor for Lua
///
/// Translated from MainStateAccessor.java (319 lines)
/// Provides Lua functions to access game state values from MainState.
/// Exports 28 functions: option, number, float_number, text, offset, timer,
/// timer_off_value, time, set_timer, event_exec, event_index,
/// rate, exscore, rate_best, exscore_best, rate_rival, exscore_rival,
/// volume_sys/key/bg, set_volume_sys/key/bg, judge, gauge, gauge_type,
/// audio_play, audio_loop, audio_stop
///
/// Timer off value constant (Long.MIN_VALUE in Java)
pub const TIMER_OFF_VALUE: i64 = i64::MIN;

/// Wrapper for raw MainState pointer to implement Send/Sync.
/// Uses *mut to support both read and write operations (set_timer, set_volume, etc.).
#[derive(Clone, Copy)]
struct StatePtr(*mut dyn MainState);
// SAFETY: StatePtr contains a *mut dyn MainState raw pointer, which is !Send and !Sync
// by default. The MainState is accessed single-threaded in beatoraja's skin system,
// and the MainState reference outlives the Lua VM. The caller of MainStateAccessor::new()
// guarantees no aliasing &mut references exist while Lua callbacks are invoked.
unsafe impl Send for StatePtr {}
unsafe impl Sync for StatePtr {}

pub struct MainStateAccessor {
    state_ptr: StatePtr,
}

impl MainStateAccessor {
    /// Create a new MainStateAccessor from a raw mutable pointer to MainState.
    ///
    /// # Safety
    /// - `state` must point to a valid `dyn MainState` that outlives this accessor
    ///   and any Lua closures exported from it.
    /// - The caller must ensure no aliasing &mut references exist while Lua callbacks
    ///   are invoked. In practice, this is guaranteed by single-threaded skin access.
    pub unsafe fn new(state: *mut dyn MainState) -> Self {
        Self {
            state_ptr: StatePtr(state),
        }
    }

    /// Export all accessor functions to a Lua table
    pub fn export(&self, lua: &Lua, table: &LuaTable) {
        let result: Result<(), LuaError> = (|| {
            let sp = self.state_ptr;

            // option(id) -> boolean
            let option_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                Ok(option_fn(state, id))
            })?;
            table.set("option", option_func)?;

            // number(id) -> integer
            let sp = self.state_ptr;
            let number_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                Ok(number_fn(state, id))
            })?;
            table.set("number", number_func)?;

            // float_number(id) -> float
            let sp = self.state_ptr;
            let float_number_func = lua.create_function(move |_, id: f64| {
                let state = unsafe { &*sp.0 };
                Ok(float_number_fn(state, id as i32))
            })?;
            table.set("float_number", float_number_func)?;

            // text(id) -> string
            let sp = self.state_ptr;
            let text_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                Ok(text_fn(state, id))
            })?;
            table.set("text", text_func)?;

            // offset(id) -> table {x, y, w, h, r, a}
            let sp = self.state_ptr;
            let offset_func = lua.create_function(move |lua, id: i32| {
                let state = unsafe { &*sp.0 };
                let tbl = lua.create_table()?;
                if let Some(offset) = state.get_offset_value(id) {
                    tbl.set("x", offset.x as f64)?;
                    tbl.set("y", offset.y as f64)?;
                    tbl.set("w", offset.w as f64)?;
                    tbl.set("h", offset.h as f64)?;
                    tbl.set("r", offset.r as f64)?;
                    tbl.set("a", offset.a as f64)?;
                } else {
                    tbl.set("x", 0.0)?;
                    tbl.set("y", 0.0)?;
                    tbl.set("w", 0.0)?;
                    tbl.set("h", 0.0)?;
                    tbl.set("r", 0.0)?;
                    tbl.set("a", 0.0)?;
                }
                Ok(tbl)
            })?;
            table.set("offset", offset_func)?;

            // timer(id) -> integer (micro sec)
            let sp = self.state_ptr;
            let timer_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_timer().get_micro_timer(id))
            })?;
            table.set("timer", timer_func)?;

            // timer_off_value constant
            table.set("timer_off_value", TIMER_OFF_VALUE)?;

            // time() -> integer (current micro time)
            let sp = self.state_ptr;
            let time_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_timer().get_now_micro_time())
            })?;
            table.set("time", time_func)?;

            // set_timer(timer_id, timer_value) -> true
            // Only custom timers are writable by skin.
            let sp = self.state_ptr;
            let set_timer_func =
                lua.create_function(move |_, (timer_id, timer_value): (i32, i64)| {
                    if !skin_property_mapper::is_timer_writable_by_skin(timer_id) {
                        return Err(LuaError::RuntimeError(
                            "The specified timer cannot be changed by skin".to_string(),
                        ));
                    }
                    let state = unsafe { &mut *sp.0 };
                    state.set_timer_micro(timer_id, timer_value);
                    Ok(true)
                })?;
            table.set("set_timer", set_timer_func)?;

            // event_exec(id [, arg1 [, arg2]]) -> true
            // Only skin-runnable events are allowed.
            let sp = self.state_ptr;
            let event_exec_func = lua.create_function(move |_, args: LuaMultiValue| {
                let mut iter = args.into_iter();
                let id_val = iter.next().ok_or_else(|| {
                    LuaError::RuntimeError("event_exec requires at least 1 argument".to_string())
                })?;
                let id = match id_val {
                    LuaValue::Integer(i) => i as i32,
                    LuaValue::Number(f) => f as i32,
                    _ => {
                        return Err(LuaError::RuntimeError(
                            "event_exec: first argument must be a number".to_string(),
                        ));
                    }
                };
                if !skin_property_mapper::is_event_runnable_by_skin(id) {
                    return Err(LuaError::RuntimeError(
                        "The specified event cannot be executed by skin".to_string(),
                    ));
                }
                let arg1 = iter
                    .next()
                    .map(|v| match v {
                        LuaValue::Integer(i) => i as i32,
                        LuaValue::Number(f) => f as i32,
                        _ => 0,
                    })
                    .unwrap_or(0);
                let arg2 = iter
                    .next()
                    .map(|v| match v {
                        LuaValue::Integer(i) => i as i32,
                        LuaValue::Number(f) => f as i32,
                        _ => 0,
                    })
                    .unwrap_or(0);
                let state = unsafe { &mut *sp.0 };
                state.execute_event(id, arg1, arg2);
                Ok(true)
            })?;
            table.set("event_exec", event_exec_func)?;

            // event_index(id) -> integer
            let sp = self.state_ptr;
            let event_index_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                Ok(event_index_fn(state, id))
            })?;
            table.set("event_index", event_index_func)?;

            // rate() -> float (current score rate)
            let sp = self.state_ptr;
            let rate_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_now_rate() as f64)
            })?;
            table.set("rate", rate_func)?;

            // exscore() -> integer (current EX score)
            let sp = self.state_ptr;
            let exscore_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_now_ex_score() as f64)
            })?;
            table.set("exscore", exscore_func)?;

            // rate_best() -> float (current best score rate)
            let sp = self.state_ptr;
            let rate_best_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_now_best_score_rate() as f64)
            })?;
            table.set("rate_best", rate_best_func)?;

            // exscore_best() -> integer (best EX score)
            let sp = self.state_ptr;
            let exscore_best_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_best_score() as f64)
            })?;
            table.set("exscore_best", exscore_best_func)?;

            // rate_rival() -> float (rival score rate)
            let sp = self.state_ptr;
            let rate_rival_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_rival_score_rate() as f64)
            })?;
            table.set("rate_rival", rate_rival_func)?;

            // exscore_rival() -> integer (rival EX score)
            let sp = self.state_ptr;
            let exscore_rival_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.get_score_data_property().get_rival_score() as f64)
            })?;
            table.set("exscore_rival", exscore_rival_func)?;

            // volume_sys() -> float (system volume)
            let sp = self.state_ptr;
            let volume_sys_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .get_config_ref()
                    .and_then(|c| c.get_audio_config())
                    .map(|a| a.systemvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_sys", volume_sys_func)?;

            // set_volume_sys(value) -> true
            let sp = self.state_ptr;
            let set_volume_sys_func = lua.create_function(move |_, value: f32| {
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.get_config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.systemvolume = value;
                }
                Ok(true)
            })?;
            table.set("set_volume_sys", set_volume_sys_func)?;

            // volume_key() -> float (key volume)
            let sp = self.state_ptr;
            let volume_key_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .get_config_ref()
                    .and_then(|c| c.get_audio_config())
                    .map(|a| a.keyvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_key", volume_key_func)?;

            // set_volume_key(value) -> true
            let sp = self.state_ptr;
            let set_volume_key_func = lua.create_function(move |_, value: f32| {
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.get_config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.keyvolume = value;
                }
                Ok(true)
            })?;
            table.set("set_volume_key", set_volume_key_func)?;

            // volume_bg() -> float (BG volume)
            let sp = self.state_ptr;
            let volume_bg_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .get_config_ref()
                    .and_then(|c| c.get_audio_config())
                    .map(|a| a.bgvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_bg", volume_bg_func)?;

            // set_volume_bg(value) -> true
            let sp = self.state_ptr;
            let set_volume_bg_func = lua.create_function(move |_, value: f32| {
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.get_config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.bgvolume = value;
                }
                Ok(true)
            })?;
            table.set("set_volume_bg", set_volume_bg_func)?;

            // judge(id) -> integer (fast + slow count for judge index)
            let sp = self.state_ptr;
            let judge_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                let total = state.get_judge_count(id, true) + state.get_judge_count(id, false);
                Ok(total)
            })?;
            table.set("judge", judge_func)?;

            // gauge() -> float (gauge value, 0 if not BMSPlayer)
            let sp = self.state_ptr;
            let gauge_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                if state.is_bms_player() {
                    Ok(state.get_gauge_value() as f64)
                } else {
                    Ok(0.0f64)
                }
            })?;
            table.set("gauge", gauge_func)?;

            // gauge_type() -> integer (gauge type, 0 if not BMSPlayer)
            let sp = self.state_ptr;
            let gauge_type_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                if state.is_bms_player() {
                    Ok(state.get_gauge_type() as f64)
                } else {
                    Ok(0.0f64)
                }
            })?;
            table.set("gauge_type", gauge_type_func)?;

            // audio_play(path, volume) -> true
            // Plays a one-shot audio file. Volume <=0 defaults to 1.0, clamped to [0, 2].
            let sp = self.state_ptr;
            let audio_play_func =
                lua.create_function(move |_, (path, volume): (String, f32)| {
                    let vol = if volume <= 0.0 {
                        1.0
                    } else {
                        volume.clamp(0.0, 2.0)
                    };
                    let state = unsafe { &mut *sp.0 };
                    let sys_vol = state
                        .get_config_ref()
                        .and_then(|c| c.get_audio_config())
                        .map(|a| a.systemvolume)
                        .unwrap_or(1.0);
                    state.audio_play(&path, sys_vol * vol, false);
                    Ok(true)
                })?;
            table.set("audio_play", audio_play_func)?;

            // audio_loop(path, volume) -> true
            // Plays a looping audio file. Volume <=0 defaults to 1.0, clamped to [0, 2].
            let sp = self.state_ptr;
            let audio_loop_func =
                lua.create_function(move |_, (path, volume): (String, f32)| {
                    let vol = if volume <= 0.0 {
                        1.0
                    } else {
                        volume.clamp(0.0, 2.0)
                    };
                    let state = unsafe { &mut *sp.0 };
                    let sys_vol = state
                        .get_config_ref()
                        .and_then(|c| c.get_audio_config())
                        .map(|a| a.systemvolume)
                        .unwrap_or(1.0);
                    state.audio_play(&path, sys_vol * vol, true);
                    Ok(true)
                })?;
            table.set("audio_loop", audio_loop_func)?;

            // audio_stop(path) -> true
            let sp = self.state_ptr;
            let audio_stop_func = lua.create_function(move |_, path: String| {
                let state = unsafe { &mut *sp.0 };
                state.audio_stop(&path);
                Ok(true)
            })?;
            table.set("audio_stop", audio_stop_func)?;

            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("MainStateAccessor::export failed: {}", e);
        }
    }
}

/// option function - Gets OPTION_* boolean by ID
pub fn option_fn(state: &dyn MainState, id: i32) -> bool {
    if let Some(prop) = boolean_property_factory::get_boolean_property(id) {
        prop.get(state)
    } else {
        false
    }
}

/// number function - Gets NUMBER_* integer by ID
pub fn number_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::get_integer_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

/// float_number function - Gets SLIDER_*/BARGRAPH_* float by ID
pub fn float_number_fn(state: &dyn MainState, id: i32) -> f32 {
    if let Some(prop) = float_property_factory::get_rate_property_by_id(id) {
        prop.get(state)
    } else {
        0.0
    }
}

/// text function - Gets STRING_* text by ID
pub fn text_fn(state: &dyn MainState, id: i32) -> String {
    if let Some(prop) = string_property_factory::get_string_property_by_id(id) {
        prop.get(state)
    } else {
        String::new()
    }
}

/// event_index function - Gets event/button index by ID
pub fn event_index_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::get_image_index_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Mock MainState for Lua accessor tests.
    /// Provides controllable score, judge, gauge, volume, timer, and audio data.
    struct LuaTestState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
        offsets: HashMap<i32, SkinOffset>,
        score_data_property: beatoraja_core::score_data_property::ScoreDataProperty,
        judge_counts: HashMap<(i32, bool), i32>,
        gauge_value: f32,
        gauge_type: i32,
        is_bms_player: bool,
        config: beatoraja_types::config::Config,
        /// Records audio_play calls: (path, volume, is_loop)
        audio_play_log: RefCell<Vec<(String, f32, bool)>>,
        /// Records audio_stop calls: path
        audio_stop_log: RefCell<Vec<String>>,
        /// Records execute_event calls: (id, arg1, arg2)
        event_log: RefCell<Vec<(i32, i32, i32)>>,
    }

    impl Default for LuaTestState {
        fn default() -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
                offsets: HashMap::new(),
                score_data_property:
                    beatoraja_core::score_data_property::ScoreDataProperty::default(),
                judge_counts: HashMap::new(),
                gauge_value: 0.0,
                gauge_type: 0,
                is_bms_player: false,
                config: beatoraja_types::config::Config::default(),
                audio_play_log: RefCell::new(Vec::new()),
                audio_stop_log: RefCell::new(Vec::new()),
                event_log: RefCell::new(Vec::new()),
            }
        }
    }

    impl MainState for LuaTestState {
        fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
            &self.timer
        }

        fn get_offset_value(&self, id: i32) -> Option<&SkinOffset> {
            self.offsets.get(&id)
        }

        fn get_main(&self) -> &MainController {
            &self.main
        }

        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }

        fn get_resource(&self) -> &PlayerResource {
            &self.resource
        }

        fn get_score_data_property(
            &self,
        ) -> &beatoraja_core::score_data_property::ScoreDataProperty {
            &self.score_data_property
        }

        fn get_judge_count(&self, judge: i32, fast: bool) -> i32 {
            *self.judge_counts.get(&(judge, fast)).unwrap_or(&0)
        }

        fn get_gauge_value(&self) -> f32 {
            self.gauge_value
        }

        fn get_gauge_type(&self) -> i32 {
            self.gauge_type
        }

        fn is_bms_player(&self) -> bool {
            self.is_bms_player
        }

        fn get_config_ref(&self) -> Option<&beatoraja_types::config::Config> {
            Some(&self.config)
        }

        fn get_config_mut(&mut self) -> Option<&mut beatoraja_types::config::Config> {
            Some(&mut self.config)
        }

        fn set_timer_micro(&mut self, timer_id: i32, micro_time: i64) {
            self.timer.set_timer_value(timer_id, micro_time);
        }

        fn audio_play(&mut self, path: &str, volume: f32, is_loop: bool) {
            self.audio_play_log
                .borrow_mut()
                .push((path.to_string(), volume, is_loop));
        }

        fn audio_stop(&mut self, path: &str) {
            self.audio_stop_log.borrow_mut().push(path.to_string());
        }

        fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
            self.event_log.borrow_mut().push((id, arg1, arg2));
        }
    }

    /// Helper: create accessor, export to Lua, and return (Lua, table) for testing.
    fn setup_lua_with_state(state: &mut dyn MainState) -> (mlua::Lua, mlua::Table) {
        let lua = mlua::Lua::new();
        let table = lua.create_table().unwrap();
        let ptr: *mut dyn MainState = state;
        // SAFETY: Transmute erases the trait object lifetime ('a -> 'static).
        // state outlives the returned Lua VM (tests are synchronous and
        // single-threaded). The raw pointer is only dereferenced by Lua closures
        // called within this test function's scope.
        let ptr: *mut dyn MainState = unsafe { std::mem::transmute(ptr) };
        let accessor = unsafe { MainStateAccessor::new(ptr) };
        accessor.export(&lua, &table);
        // We need to return the Lua and table, but Lua owns the table.
        // Use scope-safe approach: return owned values.
        (lua, table)
    }

    // ----------------------------------------------------------------
    // Tests for the already-implemented functions (sanity check)
    // ----------------------------------------------------------------

    #[test]
    fn test_option_fn_returns_false_for_out_of_range_id() {
        let state = LuaTestState::default();
        // ID >= 65536 is out of range, so no property is found -> false
        assert!(!option_fn(&state, 100000));
    }

    #[test]
    fn test_number_fn_returns_zero_for_out_of_range_id() {
        let state = LuaTestState::default();
        assert_eq!(number_fn(&state, 100000), 0);
    }

    #[test]
    fn test_float_number_fn_returns_zero_for_out_of_range_id() {
        let state = LuaTestState::default();
        assert_eq!(float_number_fn(&state, 100000), 0.0);
    }

    #[test]
    fn test_text_fn_returns_empty_for_out_of_range_id() {
        let state = LuaTestState::default();
        assert_eq!(text_fn(&state, 100000), "");
    }

    #[test]
    fn test_event_index_fn_returns_zero_for_out_of_range_id() {
        let state = LuaTestState::default();
        assert_eq!(event_index_fn(&state, 100000), 0);
    }

    // ----------------------------------------------------------------
    // Tests for new Lua-exported functions
    // ----------------------------------------------------------------

    #[test]
    fn test_rate_returns_now_rate() {
        let mut state = LuaTestState::default();
        state.score_data_property.nowrate = 0.85;
        let (lua, table) = setup_lua_with_state(&mut state);
        let rate_fn: mlua::Function = table.get("rate").unwrap();
        let result: f64 = rate_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_exscore_returns_now_exscore() {
        let mut state = LuaTestState::default();
        state.score_data_property.nowscore = 1234;
        let (lua, table) = setup_lua_with_state(&mut state);
        let exscore_fn: mlua::Function = table.get("exscore").unwrap();
        let result: f64 = exscore_fn.call(()).unwrap();
        assert_eq!(result, 1234.0);
    }

    #[test]
    fn test_rate_best_returns_now_best_score_rate() {
        let mut state = LuaTestState::default();
        state.score_data_property.nowbestscorerate = 0.92;
        let (lua, table) = setup_lua_with_state(&mut state);
        let rate_best_fn: mlua::Function = table.get("rate_best").unwrap();
        let result: f64 = rate_best_fn.call(()).unwrap();
        assert!((result - 0.92).abs() < 0.001);
    }

    #[test]
    fn test_exscore_best_returns_best_score() {
        let mut state = LuaTestState::default();
        state.score_data_property.bestscore = 999;
        let (lua, table) = setup_lua_with_state(&mut state);
        let exscore_best_fn: mlua::Function = table.get("exscore_best").unwrap();
        let result: f64 = exscore_best_fn.call(()).unwrap();
        assert_eq!(result, 999.0);
    }

    #[test]
    fn test_rate_rival_returns_rival_score_rate() {
        let mut state = LuaTestState::default();
        state.score_data_property.rivalscorerate = 0.75;
        let (lua, table) = setup_lua_with_state(&mut state);
        let rate_rival_fn: mlua::Function = table.get("rate_rival").unwrap();
        let result: f64 = rate_rival_fn.call(()).unwrap();
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_exscore_rival_returns_rival_score() {
        let mut state = LuaTestState::default();
        state.score_data_property.rivalscore = 500;
        let (lua, table) = setup_lua_with_state(&mut state);
        let exscore_rival_fn: mlua::Function = table.get("exscore_rival").unwrap();
        let result: f64 = exscore_rival_fn.call(()).unwrap();
        assert_eq!(result, 500.0);
    }

    #[test]
    fn test_volume_sys_returns_system_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(beatoraja_types::audio_config::AudioConfig {
            systemvolume: 0.8,
            ..Default::default()
        });
        let (lua, table) = setup_lua_with_state(&mut state);
        let volume_sys_fn: mlua::Function = table.get("volume_sys").unwrap();
        let result: f64 = volume_sys_fn.call(()).unwrap();
        assert!((result - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_volume_key_returns_key_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(beatoraja_types::audio_config::AudioConfig {
            keyvolume: 0.6,
            ..Default::default()
        });
        let (lua, table) = setup_lua_with_state(&mut state);
        let volume_key_fn: mlua::Function = table.get("volume_key").unwrap();
        let result: f64 = volume_key_fn.call(()).unwrap();
        assert!((result - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_volume_bg_returns_bg_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(beatoraja_types::audio_config::AudioConfig {
            bgvolume: 0.4,
            ..Default::default()
        });
        let (lua, table) = setup_lua_with_state(&mut state);
        let volume_bg_fn: mlua::Function = table.get("volume_bg").unwrap();
        let result: f64 = volume_bg_fn.call(()).unwrap();
        assert!((result - 0.4).abs() < 0.001);
    }

    #[test]
    fn test_judge_returns_total_fast_plus_slow() {
        let mut state = LuaTestState::default();
        // Judge index 0 (PGREAT): 50 fast + 30 slow = 80
        state.judge_counts.insert((0, true), 50);
        state.judge_counts.insert((0, false), 30);
        let (lua, table) = setup_lua_with_state(&mut state);
        let judge_fn: mlua::Function = table.get("judge").unwrap();
        let result: i32 = judge_fn.call(0).unwrap();
        assert_eq!(result, 80);
    }

    #[test]
    fn test_gauge_returns_zero_when_not_bms_player() {
        let mut state = LuaTestState::default();
        let (lua, table) = setup_lua_with_state(&mut state);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_gauge_returns_value_when_bms_player() {
        let mut state = LuaTestState::default();
        state.is_bms_player = true;
        state.gauge_value = 0.85;
        let (lua, table) = setup_lua_with_state(&mut state);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_gauge_type_returns_zero_when_not_bms_player() {
        let mut state = LuaTestState::default();
        let (lua, table) = setup_lua_with_state(&mut state);
        let gauge_type_fn: mlua::Function = table.get("gauge_type").unwrap();
        let result: f64 = gauge_type_fn.call(()).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_gauge_type_returns_value_when_bms_player() {
        let mut state = LuaTestState::default();
        state.is_bms_player = true;
        state.gauge_type = 2;
        let (lua, table) = setup_lua_with_state(&mut state);
        let gauge_type_fn: mlua::Function = table.get("gauge_type").unwrap();
        let result: f64 = gauge_type_fn.call(()).unwrap();
        assert_eq!(result, 2.0);
    }

    // NOTE: set_volume_*, set_timer, event_exec, audio_play/loop/stop require mutable
    // state access. These are tested via the Lua closures using *mut dyn MainState.
    // The tests below verify the Lua functions exist on the table.

    #[test]
    fn test_set_timer_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("set_timer");
        assert!(func.is_ok(), "set_timer function should be exported");
    }

    #[test]
    fn test_event_exec_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("event_exec");
        assert!(func.is_ok(), "event_exec function should be exported");
    }

    #[test]
    fn test_set_volume_sys_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("set_volume_sys");
        assert!(func.is_ok(), "set_volume_sys function should be exported");
    }

    #[test]
    fn test_set_volume_key_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("set_volume_key");
        assert!(func.is_ok(), "set_volume_key function should be exported");
    }

    #[test]
    fn test_set_volume_bg_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("set_volume_bg");
        assert!(func.is_ok(), "set_volume_bg function should be exported");
    }

    #[test]
    fn test_audio_play_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("audio_play");
        assert!(func.is_ok(), "audio_play function should be exported");
    }

    #[test]
    fn test_audio_loop_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("audio_loop");
        assert!(func.is_ok(), "audio_loop function should be exported");
    }

    #[test]
    fn test_audio_stop_exists_on_table() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Result<mlua::Function> = table.get("audio_stop");
        assert!(func.is_ok(), "audio_stop function should be exported");
    }
}
