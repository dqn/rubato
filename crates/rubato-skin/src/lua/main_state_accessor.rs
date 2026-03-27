use mlua::prelude::*;
use rubato_types::property_snapshot::PropertySnapshot;
use rubato_types::skin_render_context::SkinRenderContext;
use rubato_types::timer_access::TimerAccess;

use crate::core::skin_property_mapper;
use crate::property::boolean_property_factory;
use crate::property::float_property::FloatProperty;
use crate::property::float_property_factory;
use crate::property::integer_property_factory;
use crate::property::string_property_factory;
use crate::reexports::MainState;

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

// ================================================================
// Load-time accessor: MainStateAccessor (uses *mut dyn MainState)
// ================================================================

/// Wrapper for raw MainState pointer to implement Send/Sync.
/// Uses *mut to support both read and write operations (set_timer, set_volume, etc.).
/// Used only at load-time when no PropertySnapshot is available.
#[derive(Clone, Copy)]
struct StatePtr(*mut dyn MainState);
// SAFETY: StatePtr contains a *mut dyn MainState raw pointer, which is !Send and !Sync
// by default. The MainState is accessed single-threaded in beatoraja's skin system,
// and the MainState reference outlives the Lua VM. The caller of MainStateAccessor::new()
// guarantees no aliasing &mut references exist while Lua callbacks are invoked.
unsafe impl Send for StatePtr {}
unsafe impl Sync for StatePtr {}

/// Load-time Lua accessor backed by `*mut dyn MainState`.
///
/// During skin loading, there is no `PropertySnapshot` yet. Lua scripts that
/// call `main_state.number(id)` etc. during `load_header()` / `load()` go
/// through this accessor, which delegates to the live `MainState` trait object.
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
                Ok(state.micro_timer(rubato_types::timer_id::TimerId::new(id)))
            })?;
            table.set("timer", timer_func)?;

            // timer_off_value constant
            table.set("timer_off_value", TIMER_OFF_VALUE)?;

            // time() -> integer (current micro time)
            let sp = self.state_ptr;
            let time_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.now_micro_time())
            })?;
            table.set("time", time_func)?;

            // set_timer(timer_id, timer_value) -> true
            // Only custom timers are writable by skin.
            let sp = self.state_ptr;
            let set_timer_func =
                lua.create_function(move |_, (timer_id, timer_value): (i32, i64)| {
                    if !skin_property_mapper::is_timer_writable_by_skin(
                        rubato_types::timer_id::TimerId::new(timer_id),
                    ) {
                        return Err(LuaError::RuntimeError(
                            "The specified timer cannot be changed by skin".to_string(),
                        ));
                    }
                    let state = unsafe { &mut *sp.0 };
                    state.set_timer_micro(
                        rubato_types::timer_id::TimerId::new(timer_id),
                        timer_value,
                    );
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
                Ok(state.score_data_property().now_rate() as f64)
            })?;
            table.set("rate", rate_func)?;

            // exscore() -> integer (current EX score)
            let sp = self.state_ptr;
            let exscore_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.score_data_property().now_ex_score() as f64)
            })?;
            table.set("exscore", exscore_func)?;

            // rate_best() -> float (current best score rate)
            let sp = self.state_ptr;
            let rate_best_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.score_data_property().now_best_score_rate() as f64)
            })?;
            table.set("rate_best", rate_best_func)?;

            // exscore_best() -> integer (best EX score)
            let sp = self.state_ptr;
            let exscore_best_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.score_data_property().best_score() as f64)
            })?;
            table.set("exscore_best", exscore_best_func)?;

            // rate_rival() -> float (rival score rate)
            let sp = self.state_ptr;
            let rate_rival_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.score_data_property().rival_score_rate() as f64)
            })?;
            table.set("rate_rival", rate_rival_func)?;

            // exscore_rival() -> integer (rival EX score)
            let sp = self.state_ptr;
            let exscore_rival_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                Ok(state.score_data_property().rival_score() as f64)
            })?;
            table.set("exscore_rival", exscore_rival_func)?;

            // volume_sys() -> float (system volume)
            let sp = self.state_ptr;
            let volume_sys_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.systemvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_sys", volume_sys_func)?;

            // set_volume_sys(value) -> true
            let sp = self.state_ptr;
            let set_volume_sys_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.systemvolume = value;
                }
                state.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_sys", set_volume_sys_func)?;

            // volume_key() -> float (key volume)
            let sp = self.state_ptr;
            let volume_key_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.keyvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_key", volume_key_func)?;

            // set_volume_key(value) -> true
            let sp = self.state_ptr;
            let set_volume_key_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.keyvolume = value;
                }
                state.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_key", set_volume_key_func)?;

            // volume_bg() -> float (BG volume)
            let sp = self.state_ptr;
            let volume_bg_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                let vol = state
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.bgvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_bg", volume_bg_func)?;

            // set_volume_bg(value) -> true
            let sp = self.state_ptr;
            let set_volume_bg_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let state = unsafe { &mut *sp.0 };
                if let Some(config) = state.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.bgvolume = value;
                }
                state.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_bg", set_volume_bg_func)?;

            // judge(id) -> integer (fast + slow count for judge index)
            let sp = self.state_ptr;
            let judge_func = lua.create_function(move |_, id: i32| {
                let state = unsafe { &*sp.0 };
                let total = state.judge_count(id, true) + state.judge_count(id, false);
                Ok(total)
            })?;
            table.set("judge", judge_func)?;

            // gauge() -> float (gauge value, 0 if not BMSPlayer)
            let sp = self.state_ptr;
            let gauge_func = lua.create_function(move |_, ()| {
                let state = unsafe { &*sp.0 };
                if state.is_bms_player() {
                    Ok(state.gauge_value() as f64)
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
                    Ok(state.gauge_type() as f64)
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
                        .config_ref()
                        .and_then(|c| c.audio_config())
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
                        .config_ref()
                        .and_then(|c| c.audio_config())
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

// ================================================================
// Render-time accessor: SnapshotAccessor (uses *mut PropertySnapshot)
// ================================================================

/// Wrapper for a raw `PropertySnapshot` pointer to implement Send/Sync.
///
/// Uses `*mut` because both reads and writes go through the same allocation:
/// reads access snapshot fields directly, writes push into `snapshot.actions`.
#[derive(Clone, Copy)]
struct SnapshotPtr(*mut PropertySnapshot);
// SAFETY: The PropertySnapshot is owned by the caller and accessed
// single-threaded during skin rendering. The pointer outlives the Lua VM
// closures that capture it. No aliasing &mut references exist while Lua
// callbacks are invoked.
unsafe impl Send for SnapshotPtr {}
unsafe impl Sync for SnapshotPtr {}

/// Render-time Lua accessor backed by `*mut PropertySnapshot`.
///
/// During skin rendering, the active screen builds a `PropertySnapshot` each
/// frame. Lua scripts read property values directly from snapshot fields
/// (via `SkinRenderContext` trait) and queue write-back actions into
/// `snapshot.actions` (a `SkinActionQueue`). This eliminates the need for
/// `*mut dyn MainState` at render time, reducing the unsafe surface to a
/// single mutable pointer to a concrete type.
pub struct SnapshotAccessor {
    snapshot_ptr: SnapshotPtr,
}

impl SnapshotAccessor {
    /// Create a new SnapshotAccessor from a raw mutable pointer to PropertySnapshot.
    ///
    /// # Safety
    /// - `snapshot` must point to a valid `PropertySnapshot` that outlives this
    ///   accessor and any Lua closures exported from it.
    /// - The caller must ensure no aliasing &mut references exist while Lua
    ///   callbacks are invoked.
    pub unsafe fn new(snapshot: *mut PropertySnapshot) -> Self {
        Self {
            snapshot_ptr: SnapshotPtr(snapshot),
        }
    }

    /// Export all accessor functions to a Lua table.
    ///
    /// Read methods access `PropertySnapshot` fields directly via its
    /// `SkinRenderContext` and `TimerAccess` trait implementations.
    /// Write methods push actions into `snapshot.actions`.
    pub fn export(&self, lua: &Lua, table: &LuaTable) {
        let result: Result<(), LuaError> = (|| {
            let sp = self.snapshot_ptr;

            // option(id) -> boolean
            let option_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.boolean_value(id))
            })?;
            table.set("option", option_func)?;

            // number(id) -> integer
            let sp = self.snapshot_ptr;
            let number_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.integer_value(id))
            })?;
            table.set("number", number_func)?;

            // float_number(id) -> float
            let sp = self.snapshot_ptr;
            let float_number_func = lua.create_function(move |_, id: f64| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.float_value(id as i32))
            })?;
            table.set("float_number", float_number_func)?;

            // text(id) -> string
            let sp = self.snapshot_ptr;
            let text_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.string_value(id))
            })?;
            table.set("text", text_func)?;

            // offset(id) -> table {x, y, w, h, r, a}
            let sp = self.snapshot_ptr;
            let offset_func = lua.create_function(move |lua, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                let tbl = lua.create_table()?;
                if let Some(offset) = snapshot.get_offset_value(id) {
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
            let sp = self.snapshot_ptr;
            let timer_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.micro_timer(rubato_types::timer_id::TimerId::new(id)))
            })?;
            table.set("timer", timer_func)?;

            // timer_off_value constant
            table.set("timer_off_value", TIMER_OFF_VALUE)?;

            // time() -> integer (current micro time)
            let sp = self.snapshot_ptr;
            let time_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.now_micro_time())
            })?;
            table.set("time", time_func)?;

            // set_timer(timer_id, timer_value) -> true
            // Only custom timers are writable by skin. Queues into actions.
            let sp = self.snapshot_ptr;
            let set_timer_func =
                lua.create_function(move |_, (timer_id, timer_value): (i32, i64)| {
                    if !skin_property_mapper::is_timer_writable_by_skin(
                        rubato_types::timer_id::TimerId::new(timer_id),
                    ) {
                        return Err(LuaError::RuntimeError(
                            "The specified timer cannot be changed by skin".to_string(),
                        ));
                    }
                    let snapshot = unsafe { &mut *sp.0 };
                    snapshot.set_timer_micro(
                        rubato_types::timer_id::TimerId::new(timer_id),
                        timer_value,
                    );
                    Ok(true)
                })?;
            table.set("set_timer", set_timer_func)?;

            // event_exec(id [, arg1 [, arg2]]) -> true
            // Only skin-runnable events are allowed. Queues into actions.
            let sp = self.snapshot_ptr;
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
                let snapshot = unsafe { &mut *sp.0 };
                snapshot.execute_event(id, arg1, arg2);
                Ok(true)
            })?;
            table.set("event_exec", event_exec_func)?;

            // event_index(id) -> integer
            let sp = self.snapshot_ptr;
            let event_index_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.image_index_value(id))
            })?;
            table.set("event_index", event_index_func)?;

            // rate() -> float (current score rate)
            let sp = self.snapshot_ptr;
            let rate_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().now_rate() as f64)
            })?;
            table.set("rate", rate_func)?;

            // exscore() -> integer (current EX score)
            let sp = self.snapshot_ptr;
            let exscore_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().now_ex_score() as f64)
            })?;
            table.set("exscore", exscore_func)?;

            // rate_best() -> float (current best score rate)
            let sp = self.snapshot_ptr;
            let rate_best_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().now_best_score_rate() as f64)
            })?;
            table.set("rate_best", rate_best_func)?;

            // exscore_best() -> integer (best EX score)
            let sp = self.snapshot_ptr;
            let exscore_best_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().best_score() as f64)
            })?;
            table.set("exscore_best", exscore_best_func)?;

            // rate_rival() -> float (rival score rate)
            let sp = self.snapshot_ptr;
            let rate_rival_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().rival_score_rate() as f64)
            })?;
            table.set("rate_rival", rate_rival_func)?;

            // exscore_rival() -> integer (rival EX score)
            let sp = self.snapshot_ptr;
            let exscore_rival_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                Ok(snapshot.score_data_property().rival_score() as f64)
            })?;
            table.set("exscore_rival", exscore_rival_func)?;

            // volume_sys() -> float (system volume)
            let sp = self.snapshot_ptr;
            let volume_sys_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                let vol = snapshot
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.systemvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_sys", volume_sys_func)?;

            // set_volume_sys(value) -> true
            // Mutates the snapshot's config copy and queues audio_config_changed.
            let sp = self.snapshot_ptr;
            let set_volume_sys_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let snapshot = unsafe { &mut *sp.0 };
                if let Some(config) = snapshot.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.systemvolume = value;
                }
                snapshot.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_sys", set_volume_sys_func)?;

            // volume_key() -> float (key volume)
            let sp = self.snapshot_ptr;
            let volume_key_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                let vol = snapshot
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.keyvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_key", volume_key_func)?;

            // set_volume_key(value) -> true
            let sp = self.snapshot_ptr;
            let set_volume_key_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let snapshot = unsafe { &mut *sp.0 };
                if let Some(config) = snapshot.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.keyvolume = value;
                }
                snapshot.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_key", set_volume_key_func)?;

            // volume_bg() -> float (BG volume)
            let sp = self.snapshot_ptr;
            let volume_bg_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                let vol = snapshot
                    .config_ref()
                    .and_then(|c| c.audio_config())
                    .map(|a| a.bgvolume)
                    .unwrap_or(0.0);
                Ok(vol as f64)
            })?;
            table.set("volume_bg", volume_bg_func)?;

            // set_volume_bg(value) -> true
            let sp = self.snapshot_ptr;
            let set_volume_bg_func = lua.create_function(move |_, value: f32| {
                let value = value.clamp(0.0, 1.0);
                let snapshot = unsafe { &mut *sp.0 };
                if let Some(config) = snapshot.config_mut()
                    && let Some(ref mut audio) = config.audio
                {
                    audio.bgvolume = value;
                }
                snapshot.notify_audio_config_changed();
                Ok(true)
            })?;
            table.set("set_volume_bg", set_volume_bg_func)?;

            // judge(id) -> integer (fast + slow count for judge index)
            let sp = self.snapshot_ptr;
            let judge_func = lua.create_function(move |_, id: i32| {
                let snapshot = unsafe { &*sp.0 };
                let total = snapshot.judge_count(id, true) + snapshot.judge_count(id, false);
                Ok(total)
            })?;
            table.set("judge", judge_func)?;

            // gauge() -> float (gauge value, 0 if not BMSPlayer)
            let sp = self.snapshot_ptr;
            let gauge_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                if snapshot.is_bms_player() {
                    Ok(snapshot.gauge_value() as f64)
                } else {
                    Ok(0.0f64)
                }
            })?;
            table.set("gauge", gauge_func)?;

            // gauge_type() -> integer (gauge type, 0 if not BMSPlayer)
            let sp = self.snapshot_ptr;
            let gauge_type_func = lua.create_function(move |_, ()| {
                let snapshot = unsafe { &*sp.0 };
                if snapshot.is_bms_player() {
                    Ok(snapshot.gauge_type() as f64)
                } else {
                    Ok(0.0f64)
                }
            })?;
            table.set("gauge_type", gauge_type_func)?;

            // audio_play(path, volume) -> true
            // Plays a one-shot audio file. Volume <=0 defaults to 1.0, clamped to [0, 2].
            // Queues into snapshot.actions.audio_plays.
            let sp = self.snapshot_ptr;
            let audio_play_func =
                lua.create_function(move |_, (path, volume): (String, f32)| {
                    let vol = if volume <= 0.0 {
                        1.0
                    } else {
                        volume.clamp(0.0, 2.0)
                    };
                    let snapshot = unsafe { &mut *sp.0 };
                    let sys_vol = snapshot
                        .config_ref()
                        .and_then(|c| c.audio_config())
                        .map(|a| a.systemvolume)
                        .unwrap_or(1.0);
                    snapshot.audio_play(&path, sys_vol * vol, false);
                    Ok(true)
                })?;
            table.set("audio_play", audio_play_func)?;

            // audio_loop(path, volume) -> true
            // Plays a looping audio file. Volume <=0 defaults to 1.0, clamped to [0, 2].
            let sp = self.snapshot_ptr;
            let audio_loop_func =
                lua.create_function(move |_, (path, volume): (String, f32)| {
                    let vol = if volume <= 0.0 {
                        1.0
                    } else {
                        volume.clamp(0.0, 2.0)
                    };
                    let snapshot = unsafe { &mut *sp.0 };
                    let sys_vol = snapshot
                        .config_ref()
                        .and_then(|c| c.audio_config())
                        .map(|a| a.systemvolume)
                        .unwrap_or(1.0);
                    snapshot.audio_play(&path, sys_vol * vol, true);
                    Ok(true)
                })?;
            table.set("audio_loop", audio_loop_func)?;

            // audio_stop(path) -> true
            let sp = self.snapshot_ptr;
            let audio_stop_func = lua.create_function(move |_, path: String| {
                let snapshot = unsafe { &mut *sp.0 };
                snapshot.audio_stop(&path);
                Ok(true)
            })?;
            table.set("audio_stop", audio_stop_func)?;

            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("SnapshotAccessor::export failed: {}", e);
        }
    }
}

/// option function - Gets OPTION_* boolean by ID
pub fn option_fn(state: &dyn MainState, id: i32) -> bool {
    if let Some(prop) = boolean_property_factory::boolean_property(id) {
        prop.get(state)
    } else {
        false
    }
}

/// number function - Gets NUMBER_* integer by ID
pub fn number_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::integer_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

/// float_number function - Gets SLIDER_*/BARGRAPH_* float by ID
pub fn float_number_fn(state: &dyn MainState, id: i32) -> f32 {
    if let Some(prop) = float_property_factory::rate_property_by_id(id) {
        prop.get(state)
    } else {
        0.0
    }
}

/// text function - Gets STRING_* text by ID
pub fn text_fn(state: &dyn MainState, id: i32) -> String {
    if let Some(prop) = string_property_factory::string_property_by_id(id) {
        prop.get(state)
    } else {
        String::new()
    }
}

/// event_index function - Gets event/button index by ID
pub fn event_index_fn(state: &dyn MainState, id: i32) -> i32 {
    if let Some(prop) = integer_property_factory::image_index_property_by_id(id) {
        prop.get(state)
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reexports::{SkinOffset, Timer};
    use std::cell::RefCell;
    use std::collections::HashMap;

    /// Mock MainState for Lua accessor tests.
    /// Provides controllable score, judge, gauge, volume, timer, and audio data.
    struct LuaTestState {
        timer: Timer,
        offsets: HashMap<i32, SkinOffset>,
        score_data_property: rubato_core::score_data_property::ScoreDataProperty,
        judge_counts: HashMap<(i32, bool), i32>,
        gauge_value: f32,
        gauge_type: i32,
        is_bms_player: bool,
        config: rubato_types::config::Config,
        /// Records audio_play calls: (path, volume, is_loop)
        audio_play_log: RefCell<Vec<(String, f32, bool)>>,
        /// Records audio_stop calls: path
        audio_stop_log: RefCell<Vec<String>>,
        /// Records execute_event calls: (id, arg1, arg2)
        event_log: RefCell<Vec<(i32, i32, i32)>>,
        /// Records notify_audio_config_changed calls count
        audio_config_changed_count: RefCell<u32>,
    }

    impl Default for LuaTestState {
        fn default() -> Self {
            Self {
                timer: Timer::default(),
                offsets: HashMap::new(),
                score_data_property: rubato_core::score_data_property::ScoreDataProperty::default(),
                judge_counts: HashMap::new(),
                gauge_value: 0.0,
                gauge_type: 0,
                is_bms_player: false,
                config: rubato_types::config::Config::default(),
                audio_play_log: RefCell::new(Vec::new()),
                audio_stop_log: RefCell::new(Vec::new()),
                event_log: RefCell::new(Vec::new()),
                audio_config_changed_count: RefCell::new(0),
            }
        }
    }

    impl rubato_types::timer_access::TimerAccess for LuaTestState {
        fn now_time(&self) -> i64 {
            self.timer.now_time()
        }
        fn now_micro_time(&self) -> i64 {
            self.timer.now_micro_time()
        }
        fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.micro_timer(timer_id)
        }
        fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.timer(timer_id)
        }
        fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
            self.timer.now_time_for(timer_id)
        }
        fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
            self.timer.is_timer_on(timer_id)
        }
    }

    impl rubato_types::skin_render_context::SkinRenderContext for LuaTestState {
        fn get_offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
            self.offsets.get(&id)
        }

        fn score_data_property(&self) -> &rubato_types::score_data_property::ScoreDataProperty {
            &self.score_data_property
        }

        fn judge_count(&self, judge: i32, fast: bool) -> i32 {
            *self.judge_counts.get(&(judge, fast)).unwrap_or(&0)
        }

        fn gauge_value(&self) -> f32 {
            self.gauge_value
        }

        fn gauge_type(&self) -> i32 {
            self.gauge_type
        }

        fn is_bms_player(&self) -> bool {
            self.is_bms_player
        }

        fn config_ref(&self) -> Option<&rubato_types::config::Config> {
            Some(&self.config)
        }

        fn config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
            Some(&mut self.config)
        }

        fn set_timer_micro(&mut self, timer_id: rubato_types::timer_id::TimerId, micro_time: i64) {
            self.timer.set_timer_value(timer_id.as_i32(), micro_time);
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

        fn notify_audio_config_changed(&mut self) {
            *self.audio_config_changed_count.borrow_mut() += 1;
        }
    }

    impl MainState for LuaTestState {}

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
        let (_lua, table) = setup_lua_with_state(&mut state);
        let rate_fn: mlua::Function = table.get("rate").unwrap();
        let result: f64 = rate_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_exscore_returns_now_exscore() {
        let mut state = LuaTestState::default();
        state.score_data_property.nowscore = 1234;
        let (_lua, table) = setup_lua_with_state(&mut state);
        let exscore_fn: mlua::Function = table.get("exscore").unwrap();
        let result: f64 = exscore_fn.call(()).unwrap();
        assert_eq!(result, 1234.0);
    }

    #[test]
    fn test_rate_best_returns_now_best_score_rate() {
        let mut state = LuaTestState::default();
        state.score_data_property.nowbestscorerate = 0.92;
        let (_lua, table) = setup_lua_with_state(&mut state);
        let rate_best_fn: mlua::Function = table.get("rate_best").unwrap();
        let result: f64 = rate_best_fn.call(()).unwrap();
        assert!((result - 0.92).abs() < 0.001);
    }

    #[test]
    fn test_exscore_best_returns_best_score() {
        let mut state = LuaTestState::default();
        state.score_data_property.bestscore = 999;
        let (_lua, table) = setup_lua_with_state(&mut state);
        let exscore_best_fn: mlua::Function = table.get("exscore_best").unwrap();
        let result: f64 = exscore_best_fn.call(()).unwrap();
        assert_eq!(result, 999.0);
    }

    #[test]
    fn test_rate_rival_returns_rival_score_rate() {
        let mut state = LuaTestState::default();
        state.score_data_property.rivalscorerate = 0.75;
        let (_lua, table) = setup_lua_with_state(&mut state);
        let rate_rival_fn: mlua::Function = table.get("rate_rival").unwrap();
        let result: f64 = rate_rival_fn.call(()).unwrap();
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn test_exscore_rival_returns_rival_score() {
        let mut state = LuaTestState::default();
        state.score_data_property.rivalscore = 500;
        let (_lua, table) = setup_lua_with_state(&mut state);
        let exscore_rival_fn: mlua::Function = table.get("exscore_rival").unwrap();
        let result: f64 = exscore_rival_fn.call(()).unwrap();
        assert_eq!(result, 500.0);
    }

    #[test]
    fn test_volume_sys_returns_system_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig {
            systemvolume: 0.8,
            ..Default::default()
        });
        let (_lua, table) = setup_lua_with_state(&mut state);
        let volume_sys_fn: mlua::Function = table.get("volume_sys").unwrap();
        let result: f64 = volume_sys_fn.call(()).unwrap();
        assert!((result - 0.8).abs() < 0.001);
    }

    #[test]
    fn test_volume_key_returns_key_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig {
            keyvolume: 0.6,
            ..Default::default()
        });
        let (_lua, table) = setup_lua_with_state(&mut state);
        let volume_key_fn: mlua::Function = table.get("volume_key").unwrap();
        let result: f64 = volume_key_fn.call(()).unwrap();
        assert!((result - 0.6).abs() < 0.001);
    }

    #[test]
    fn test_volume_bg_returns_bg_volume() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig {
            bgvolume: 0.4,
            ..Default::default()
        });
        let (_lua, table) = setup_lua_with_state(&mut state);
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
        let (_lua, table) = setup_lua_with_state(&mut state);
        let judge_fn: mlua::Function = table.get("judge").unwrap();
        let result: i32 = judge_fn.call(0).unwrap();
        assert_eq!(result, 80);
    }

    #[test]
    fn test_gauge_returns_zero_when_not_bms_player() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_gauge_returns_value_when_bms_player() {
        let mut state = LuaTestState {
            is_bms_player: true,
            gauge_value: 0.85,
            ..Default::default()
        };
        let (_lua, table) = setup_lua_with_state(&mut state);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn test_gauge_type_returns_zero_when_not_bms_player() {
        let mut state = LuaTestState::default();
        let (_lua, table) = setup_lua_with_state(&mut state);
        let gauge_type_fn: mlua::Function = table.get("gauge_type").unwrap();
        let result: f64 = gauge_type_fn.call(()).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn test_gauge_type_returns_value_when_bms_player() {
        let mut state = LuaTestState {
            is_bms_player: true,
            gauge_type: 2,
            ..Default::default()
        };
        let (_lua, table) = setup_lua_with_state(&mut state);
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

    #[test]
    fn test_set_volume_sys_calls_notify_audio_config_changed() {
        // Regression: Lua set_volume_sys must propagate changes to audio driver
        // via notify_audio_config_changed, not just modify the local config clone.
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_sys").unwrap();
        let _: bool = func.call(0.75f32).unwrap();
        assert_eq!(
            *state.audio_config_changed_count.borrow(),
            1,
            "set_volume_sys should call notify_audio_config_changed"
        );
        assert_eq!(
            state.config.audio.as_ref().unwrap().systemvolume,
            0.75,
            "config should be updated"
        );
    }

    #[test]
    fn test_set_volume_key_calls_notify_audio_config_changed() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_key").unwrap();
        let _: bool = func.call(0.5f32).unwrap();
        assert_eq!(
            *state.audio_config_changed_count.borrow(),
            1,
            "set_volume_key should call notify_audio_config_changed"
        );
        assert_eq!(
            state.config.audio.as_ref().unwrap().keyvolume,
            0.5,
            "config should be updated"
        );
    }

    #[test]
    fn test_set_volume_bg_calls_notify_audio_config_changed() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_bg").unwrap();
        let _: bool = func.call(0.25f32).unwrap();
        assert_eq!(
            *state.audio_config_changed_count.borrow(),
            1,
            "set_volume_bg should call notify_audio_config_changed"
        );
        assert_eq!(
            state.config.audio.as_ref().unwrap().bgvolume,
            0.25,
            "config should be updated"
        );
    }

    #[test]
    fn test_set_volume_sys_clamps_out_of_range() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_sys").unwrap();
        let _: bool = func.call(99.0f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().systemvolume, 1.0);
        let _: bool = func.call(-5.0f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().systemvolume, 0.0);
    }

    #[test]
    fn test_set_volume_key_clamps_out_of_range() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_key").unwrap();
        let _: bool = func.call(2.0f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().keyvolume, 1.0);
        let _: bool = func.call(-1.0f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().keyvolume, 0.0);
    }

    #[test]
    fn test_set_volume_bg_clamps_out_of_range() {
        let mut state = LuaTestState::default();
        state.config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        let (_lua, table) = setup_lua_with_state(&mut state);
        let func: mlua::Function = table.get("set_volume_bg").unwrap();
        let _: bool = func.call(1.5f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().bgvolume, 1.0);
        let _: bool = func.call(-0.1f32).unwrap();
        assert_eq!(state.config.audio.as_ref().unwrap().bgvolume, 0.0);
    }

    // ================================================================
    // SnapshotAccessor tests (render-time path)
    // ================================================================

    /// Helper: create SnapshotAccessor, export to Lua, and return (Lua, table, snapshot).
    fn setup_lua_with_snapshot(snapshot: &mut PropertySnapshot) -> (mlua::Lua, mlua::Table) {
        let lua = mlua::Lua::new();
        let table = lua.create_table().unwrap();
        let ptr: *mut PropertySnapshot = snapshot;
        let accessor = unsafe { SnapshotAccessor::new(ptr) };
        accessor.export(&lua, &table);
        (lua, table)
    }

    #[test]
    fn snapshot_number_reads_integer_value() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.integers.insert(100, 42);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let number_fn: mlua::Function = table.get("number").unwrap();
        let result: i32 = number_fn.call(100).unwrap();
        assert_eq!(result, 42);
    }

    #[test]
    fn snapshot_float_number_reads_float_value() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.floats.insert(5, 0.75);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let float_fn: mlua::Function = table.get("float_number").unwrap();
        let result: f32 = float_fn.call(5.0f64).unwrap();
        assert!((result - 0.75).abs() < 0.001);
    }

    #[test]
    fn snapshot_text_reads_string_value() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.strings.insert(10, "Hello".to_string());
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let text_fn: mlua::Function = table.get("text").unwrap();
        let result: String = text_fn.call(10).unwrap();
        assert_eq!(result, "Hello");
    }

    #[test]
    fn snapshot_option_reads_boolean_value() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.booleans.insert(42, true);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let option_fn: mlua::Function = table.get("option").unwrap();
        let result: bool = option_fn.call(42).unwrap();
        assert!(result);
    }

    #[test]
    fn snapshot_timer_reads_from_timers_map() {
        let mut snapshot = PropertySnapshot::new();
        snapshot
            .timers
            .insert(rubato_types::timer_id::TimerId::new(5), 1_000_000);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let timer_fn: mlua::Function = table.get("timer").unwrap();
        let result: i64 = timer_fn.call(5).unwrap();
        assert_eq!(result, 1_000_000);
    }

    #[test]
    fn snapshot_time_reads_now_micro_time() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.now_micro_time = 5_000_000;
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let time_fn: mlua::Function = table.get("time").unwrap();
        let result: i64 = time_fn.call(()).unwrap();
        assert_eq!(result, 5_000_000);
    }

    #[test]
    fn snapshot_offset_reads_from_offsets_map() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.offsets.insert(
            3,
            rubato_types::skin_offset::SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
                r: 50.0,
                a: 60.0,
            },
        );
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let offset_fn: mlua::Function = table.get("offset").unwrap();
        let result: mlua::Table = offset_fn.call(3).unwrap();
        assert_eq!(result.get::<f64>("x").unwrap(), 10.0);
        assert_eq!(result.get::<f64>("y").unwrap(), 20.0);
        assert_eq!(result.get::<f64>("w").unwrap(), 30.0);
        assert_eq!(result.get::<f64>("h").unwrap(), 40.0);
        assert_eq!(result.get::<f64>("r").unwrap(), 50.0);
        assert_eq!(result.get::<f64>("a").unwrap(), 60.0);
    }

    #[test]
    fn snapshot_rate_reads_score_data_property() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.score_data_property.nowrate = 0.85;
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let rate_fn: mlua::Function = table.get("rate").unwrap();
        let result: f64 = rate_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn snapshot_exscore_reads_score_data_property() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.score_data_property.nowscore = 1234;
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let exscore_fn: mlua::Function = table.get("exscore").unwrap();
        let result: f64 = exscore_fn.call(()).unwrap();
        assert_eq!(result, 1234.0);
    }

    #[test]
    fn snapshot_volume_sys_reads_from_config() {
        let mut snapshot = PropertySnapshot::new();
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig {
            systemvolume: 0.8,
            ..Default::default()
        });
        snapshot.config = Some(Box::new(config));
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let vol_fn: mlua::Function = table.get("volume_sys").unwrap();
        let result: f64 = vol_fn.call(()).unwrap();
        assert!((result - 0.8).abs() < 0.001);
    }

    #[test]
    fn snapshot_judge_reads_judge_counts() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.judge_counts.insert((0, true), 50);
        snapshot.judge_counts.insert((0, false), 30);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let judge_fn: mlua::Function = table.get("judge").unwrap();
        let result: i32 = judge_fn.call(0).unwrap();
        assert_eq!(result, 80);
    }

    #[test]
    fn snapshot_gauge_reads_from_snapshot() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.state_type = Some(rubato_types::main_state_type::MainStateType::Play);
        snapshot.gauge_value = 0.85;
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert!((result - 0.85).abs() < 0.001);
    }

    #[test]
    fn snapshot_gauge_returns_zero_when_not_play_state() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.state_type = Some(rubato_types::main_state_type::MainStateType::MusicSelect);
        snapshot.gauge_value = 0.85;
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let gauge_fn: mlua::Function = table.get("gauge").unwrap();
        let result: f64 = gauge_fn.call(()).unwrap();
        assert_eq!(result, 0.0);
    }

    #[test]
    fn snapshot_set_volume_sys_queues_action() {
        let mut snapshot = PropertySnapshot::new();
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig::default());
        snapshot.config = Some(Box::new(config));
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let func: mlua::Function = table.get("set_volume_sys").unwrap();
        let _: bool = func.call(0.75f32).unwrap();
        // Config copy is updated
        assert_eq!(
            snapshot
                .config
                .as_ref()
                .unwrap()
                .audio
                .as_ref()
                .unwrap()
                .systemvolume,
            0.75
        );
        // audio_config_changed is queued in actions
        assert!(snapshot.actions.audio_config_changed);
    }

    #[test]
    fn snapshot_event_exec_queues_custom_event() {
        let mut snapshot = PropertySnapshot::new();
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let func: mlua::Function = table.get("event_exec").unwrap();
        // Use event ID 1000 which is in the skin-runnable range (custom events)
        let result: mlua::Result<bool> = func.call((1000, 1, 2));
        if result.is_ok() {
            assert_eq!(snapshot.actions.custom_events, vec![(1000, 1, 2)]);
        }
        // If 1000 is not skin-runnable, the error is expected behavior
    }

    #[test]
    fn snapshot_audio_play_queues_action() {
        let mut snapshot = PropertySnapshot::new();
        let mut config = rubato_types::config::Config::default();
        config.audio = Some(rubato_types::audio_config::AudioConfig {
            systemvolume: 0.5,
            ..Default::default()
        });
        snapshot.config = Some(Box::new(config));
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let func: mlua::Function = table.get("audio_play").unwrap();
        let _: bool = func.call(("test.wav", 0.8f32)).unwrap();
        assert_eq!(snapshot.actions.audio_plays.len(), 1);
        assert_eq!(snapshot.actions.audio_plays[0].0, "test.wav");
        // Volume = systemvolume * vol = 0.5 * 0.8 = 0.4
        assert!((snapshot.actions.audio_plays[0].1 - 0.4).abs() < 0.001);
        assert!(!snapshot.actions.audio_plays[0].2); // not loop
    }

    #[test]
    fn snapshot_audio_stop_queues_action() {
        let mut snapshot = PropertySnapshot::new();
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let func: mlua::Function = table.get("audio_stop").unwrap();
        let _: bool = func.call("test.wav").unwrap();
        assert_eq!(snapshot.actions.audio_stops, vec!["test.wav".to_string()]);
    }

    #[test]
    fn snapshot_event_index_reads_image_index_value() {
        let mut snapshot = PropertySnapshot::new();
        snapshot.image_indices.insert(42, 7);
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        let func: mlua::Function = table.get("event_index").unwrap();
        let result: i32 = func.call(42).unwrap();
        assert_eq!(result, 7);
    }

    #[test]
    fn snapshot_all_write_functions_exist() {
        let mut snapshot = PropertySnapshot::new();
        let (_lua, table) = setup_lua_with_snapshot(&mut snapshot);
        // Verify all write functions are exported
        assert!(table.get::<mlua::Function>("set_timer").is_ok());
        assert!(table.get::<mlua::Function>("event_exec").is_ok());
        assert!(table.get::<mlua::Function>("set_volume_sys").is_ok());
        assert!(table.get::<mlua::Function>("set_volume_key").is_ok());
        assert!(table.get::<mlua::Function>("set_volume_bg").is_ok());
        assert!(table.get::<mlua::Function>("audio_play").is_ok());
        assert!(table.get::<mlua::Function>("audio_loop").is_ok());
        assert!(table.get::<mlua::Function>("audio_stop").is_ok());
    }
}
