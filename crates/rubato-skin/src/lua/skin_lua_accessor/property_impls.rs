use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::property::boolean_property::BooleanProperty;
use crate::property::event::Event;
use crate::property::float_property::FloatProperty;
use crate::property::float_writer::FloatWriter;
use crate::property::integer_property::IntegerProperty;
use crate::property::string_property::StringProperty;
use crate::property::timer_property::TimerProperty;
use crate::reexports::MainState;
use rubato_types::sync_utils::lock_or_recover;

// ============================================================
// Lua-backed property implementations
// ============================================================

// SAFETY NOTE: These structs hold an Arc<Lua> that shares ownership of the Lua VM
// with the SkinLuaAccessor. The Arc ensures the VM cannot be dropped while any
// property is alive, preventing use-after-free. The Send+Sync impls are required
// by the property traits; Lua (without the "send" feature) is !Send, so we rely
// on the single-threaded access invariant. The creation_thread_id field enables
// debug_assert checks that panic in debug builds; in release builds, cross-thread
// access logs a warning and returns a safe default value instead of panicking.

pub(crate) struct LuaBooleanProperty {
    pub(super) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(super) lua: Arc<Lua>,
    pub(super) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: The Lua VM is accessed single-threaded in beatoraja's skin system.
// debug_assert in get() verifies this invariant at runtime in debug builds;
// release builds log a warning and return false on cross-thread access.
unsafe impl Send for LuaBooleanProperty {}
unsafe impl Sync for LuaBooleanProperty {}

impl BooleanProperty for LuaBooleanProperty {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        false
    }

    fn get(&self, _state: &dyn MainState) -> bool {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaBooleanProperty must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaBooleanProperty accessed from wrong thread");
            return false;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => match func.call::<LuaValue>(()) {
                Ok(val) => match val {
                    LuaValue::Boolean(b) => b,
                    LuaValue::Integer(i) => i != 0,
                    LuaValue::Number(f) => f != 0.0,
                    _ => false,
                },
                Err(e) => {
                    log::warn!("Lua runtime error (boolean property): {}", e);
                    false
                }
            },
            Err(e) => {
                log::warn!("Lua registry error (boolean property): {}", e);
                false
            }
        }
    }
}

pub(crate) struct LuaIntegerProperty {
    pub(super) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(super) lua: Arc<Lua>,
    pub(super) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaIntegerProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant in debug builds;
// release builds log a warning and return 0 on cross-thread access.
unsafe impl Send for LuaIntegerProperty {}
unsafe impl Sync for LuaIntegerProperty {}

impl IntegerProperty for LuaIntegerProperty {
    fn get(&self, _state: &dyn MainState) -> i32 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaIntegerProperty must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaIntegerProperty accessed from wrong thread");
            return 0;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => match func.call::<LuaValue>(()) {
                Ok(val) => match val {
                    LuaValue::Integer(i) => i as i32,
                    LuaValue::Number(f) => f as i32,
                    _ => 0,
                },
                Err(e) => {
                    log::warn!("Lua runtime error (integer property): {}", e);
                    0
                }
            },
            Err(e) => {
                log::warn!("Lua registry error (integer property): {}", e);
                0
            }
        }
    }
}

pub struct LuaFloatProperty {
    pub(crate) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(crate) lua: Arc<Lua>,
    pub(crate) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaFloatProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant in debug builds;
// release builds log a warning and return 0.0 on cross-thread access.
unsafe impl Send for LuaFloatProperty {}
unsafe impl Sync for LuaFloatProperty {}

impl FloatProperty for LuaFloatProperty {
    fn get(&self, _state: &dyn MainState) -> f32 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaFloatProperty must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaFloatProperty accessed from wrong thread");
            return 0.0;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => match func.call::<LuaValue>(()) {
                Ok(val) => match val {
                    LuaValue::Number(f) => f as f32,
                    LuaValue::Integer(i) => i as f32,
                    _ => 0.0,
                },
                Err(e) => {
                    log::warn!("Lua runtime error (float property): {}", e);
                    0.0
                }
            },
            Err(e) => {
                log::warn!("Lua registry error (float property): {}", e);
                0.0
            }
        }
    }
}

pub(crate) struct LuaStringProperty {
    pub(super) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(super) lua: Arc<Lua>,
    pub(super) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaStringProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant in debug builds;
// release builds log a warning and return "" on cross-thread access.
unsafe impl Send for LuaStringProperty {}
unsafe impl Sync for LuaStringProperty {}

impl StringProperty for LuaStringProperty {
    fn get(&self, _state: &dyn MainState) -> String {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaStringProperty must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaStringProperty accessed from wrong thread");
            return String::new();
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => match func.call::<LuaValue>(()) {
                Ok(val) => match val {
                    LuaValue::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
                    _ => val.to_string().unwrap_or_default(),
                },
                Err(e) => {
                    log::warn!("Lua runtime error (string property): {}", e);
                    String::new()
                }
            },
            Err(e) => {
                log::warn!("Lua registry error (string property): {}", e);
                String::new()
            }
        }
    }
}

#[derive(Clone)]
pub struct LuaTimerProperty {
    pub(crate) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(crate) lua: Arc<Lua>,
    pub(crate) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaTimerProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get_micro() verifies this invariant in debug builds;
// release builds log a warning and return i64::MIN on cross-thread access.
unsafe impl Send for LuaTimerProperty {}
unsafe impl Sync for LuaTimerProperty {}

impl TimerProperty for LuaTimerProperty {
    fn get_micro(&self, _state: &dyn MainState) -> i64 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaTimerProperty must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaTimerProperty accessed from wrong thread");
            return i64::MIN;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => match func.call::<LuaValue>(()) {
                Ok(val) => match val {
                    LuaValue::Integer(i) => i,
                    LuaValue::Number(f) => f as i64,
                    _ => i64::MIN,
                },
                Err(e) => {
                    log::warn!("Lua runtime error (timer property): {}", e);
                    i64::MIN
                }
            },
            Err(e) => {
                log::warn!("Lua registry error (timer property): {}", e);
                i64::MIN
            }
        }
    }
}

pub(crate) struct LuaEvent {
    pub(super) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(super) lua: Arc<Lua>,
    pub(super) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaEvent contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in exec() verifies this invariant in debug builds;
// release builds log a warning and return early on cross-thread access.
unsafe impl Send for LuaEvent {}
unsafe impl Sync for LuaEvent {}

impl Event for LuaEvent {
    fn exec(&self, _state: &mut dyn MainState, arg1: i32, arg2: i32) {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaEvent must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaEvent accessed from wrong thread");
            return;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => {
                // Pass both args; Lua functions ignore extra args
                if let Err(e) = func.call::<LuaValue>((arg1, arg2)) {
                    log::warn!("Lua runtime error (event): {}", e);
                }
            }
            Err(e) => {
                log::warn!("Lua registry error (event): {}", e);
            }
        }
    }
}

pub(crate) struct LuaFloatWriter {
    pub(super) func_key: Arc<Mutex<LuaRegistryKey>>,
    pub(super) lua: Arc<Lua>,
    pub(super) creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaFloatWriter contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in set() verifies this invariant in debug builds;
// release builds log a warning and return early on cross-thread access.
unsafe impl Send for LuaFloatWriter {}
unsafe impl Sync for LuaFloatWriter {}

impl FloatWriter for LuaFloatWriter {
    fn set(&self, _state: &mut dyn MainState, value: f32) {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaFloatWriter must be accessed on the thread where it was created"
        );
        if std::thread::current().id() != self.creation_thread_id {
            log::warn!("LuaFloatWriter accessed from wrong thread");
            return;
        }
        let key = lock_or_recover(&self.func_key);
        match self.lua.registry_value::<LuaFunction>(&key) {
            Ok(func) => {
                if let Err(e) = func.call::<LuaValue>(value) {
                    log::warn!("Lua runtime error (float writer): {}", e);
                }
            }
            Err(e) => {
                log::warn!("Lua registry error (float writer): {}", e);
            }
        }
    }
}
