use std::path::Path;
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::property::boolean_property::BooleanProperty;
use crate::property::event::Event;
use crate::property::float_property::FloatProperty;
use crate::property::float_writer::FloatWriter;
use crate::property::integer_property::IntegerProperty;
use crate::property::string_property::StringProperty;
use crate::property::timer_property::TimerProperty;
use crate::stubs::MainState;

/// Lua skin accessor
///
/// Translated from SkinLuaAccessor.java (379 lines)
/// Provides Lua scripting integration for skins.
/// Exports BooleanProperty, IntegerProperty, FloatProperty, StringProperty,
/// TimerProperty, Event, and FloatWriter from Lua scripts.
const MAIN_STATE: &str = "main_state";
const TIMER_UTIL: &str = "timer_util";
const EVENT_UTIL: &str = "event_util";

pub struct SkinLuaAccessor {
    /// Whether to export to global scope (true) or as modules (false)
    is_global: bool,
    /// The Lua VM instance
    lua: Lua,
}

impl SkinLuaAccessor {
    pub fn new(is_global: bool) -> Self {
        let lua = Lua::new();

        if !is_global {
            // Pre-register empty tables so require("main_state") etc. don't error during header loading
            lua.scope(|_scope| {
                let loaded: LuaTable = lua
                    .globals()
                    .get::<LuaTable>("package")
                    .and_then(|pkg| pkg.get::<LuaTable>("loaded"))
                    .unwrap();
                let _ = loaded.set(MAIN_STATE, lua.create_table().unwrap());
                let _ = loaded.set(TIMER_UTIL, lua.create_table().unwrap());
                let _ = loaded.set(EVENT_UTIL, lua.create_table().unwrap());
                Ok(())
            })
            .unwrap_or_else(|e| {
                log::warn!("Failed to initialize Lua module tables: {}", e);
            });
        }

        Self { is_global, lua }
    }

    /// Load a BooleanProperty from a Lua script string
    pub fn load_boolean_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn BooleanProperty>> {
        let full_script = format!("return {}", script);
        match self.lua.load(&full_script).into_function() {
            Ok(func) => self.load_boolean_property_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (boolean property): {}", e);
                None
            }
        }
    }

    /// Load a BooleanProperty from a Lua function
    pub fn load_boolean_property_from_function(&self) -> Option<Box<dyn BooleanProperty>> {
        log::warn!("load_boolean_property_from_function: Lua function reference not yet wired");
        None
    }

    fn load_boolean_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn BooleanProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaBooleanProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load an IntegerProperty from a Lua script string
    pub fn load_integer_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn IntegerProperty>> {
        let full_script = format!("return {}", script);
        match self.lua.load(&full_script).into_function() {
            Ok(func) => self.load_integer_property_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (integer property): {}", e);
                None
            }
        }
    }

    /// Load an IntegerProperty from a Lua function
    pub fn load_integer_property_from_function(&self) -> Option<Box<dyn IntegerProperty>> {
        log::warn!("load_integer_property_from_function: Lua function reference not yet wired");
        None
    }

    fn load_integer_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn IntegerProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaIntegerProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load a FloatProperty from a Lua script string
    pub fn load_float_property_from_script(&self, script: &str) -> Option<Box<dyn FloatProperty>> {
        let full_script = format!("return {}", script);
        match self.lua.load(&full_script).into_function() {
            Ok(func) => self.load_float_property_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (float property): {}", e);
                None
            }
        }
    }

    /// Load a FloatProperty from a Lua function
    pub fn load_float_property_from_function(&self) -> Option<Box<dyn FloatProperty>> {
        log::warn!("load_float_property_from_function: Lua function reference not yet wired");
        None
    }

    fn load_float_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaFloatProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load a StringProperty from a Lua script string
    pub fn load_string_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn StringProperty>> {
        let full_script = format!("return {}", script);
        match self.lua.load(&full_script).into_function() {
            Ok(func) => self.load_string_property_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (string property): {}", e);
                None
            }
        }
    }

    /// Load a StringProperty from a Lua function
    pub fn load_string_property_from_function(&self) -> Option<Box<dyn StringProperty>> {
        log::warn!("load_string_property_from_function: Lua function reference not yet wired");
        None
    }

    fn load_string_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn StringProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaStringProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load a TimerProperty from a Lua script string
    /// If the script returns a function, that function is used as a timer function.
    /// Otherwise, the script itself is regarded as a timer function.
    /// A timer function returns start time in microseconds if on, or i64::MIN if off.
    pub fn load_timer_property_from_script(&self, script: &str) -> Option<Box<dyn TimerProperty>> {
        let full_script = format!("return {}", script);
        match self.lua.load(&full_script).into_function() {
            Ok(func) => {
                // Trial call: if the result is a function, use that instead
                match func.call::<LuaValue>(()) {
                    Ok(LuaValue::Function(inner_func)) => {
                        self.load_timer_property_from_lua_function(inner_func)
                    }
                    Ok(_) => {
                        // The script itself returns a number, use the script as timer function
                        self.load_timer_property_from_lua_function(func)
                    }
                    Err(e) => {
                        log::warn!("Lua parse error (timer property trial call): {}", e);
                        None
                    }
                }
            }
            Err(e) => {
                log::warn!("Lua parse error (timer property): {}", e);
                None
            }
        }
    }

    /// Load a TimerProperty from a Lua function
    pub fn load_timer_property_from_function(&self) -> Option<Box<dyn TimerProperty>> {
        log::warn!("load_timer_property_from_function: Lua function reference not yet wired");
        None
    }

    fn load_timer_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn TimerProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaTimerProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load an Event from a Lua script string
    pub fn load_event_from_script(&self, script: &str) -> Option<Box<dyn Event>> {
        match self.lua.load(script).into_function() {
            Ok(func) => self.load_event_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (event): {}", e);
                None
            }
        }
    }

    /// Load an Event from a Lua function
    pub fn load_event_from_function(&self) -> Option<Box<dyn Event>> {
        log::warn!("load_event_from_function: Lua function reference not yet wired");
        None
    }

    fn load_event_from_lua_function(&self, func: LuaFunction) -> Option<Box<dyn Event>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaEvent {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Load a FloatWriter from a Lua script string
    pub fn load_float_writer_from_script(&self, script: &str) -> Option<Box<dyn FloatWriter>> {
        match self.lua.load(script).into_function() {
            Ok(func) => self.load_float_writer_from_lua_function(func),
            Err(e) => {
                log::warn!("Lua parse error (float writer): {}", e);
                None
            }
        }
    }

    /// Load a FloatWriter from a Lua function
    pub fn load_float_writer_from_function(&self) -> Option<Box<dyn FloatWriter>> {
        log::warn!("load_float_writer_from_function: Lua function reference not yet wired");
        None
    }

    fn load_float_writer_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatWriter>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        let lua_ptr = &self.lua as *const Lua;
        Some(Box::new(LuaFloatWriter {
            func_key: Arc::new(Mutex::new(func_key)),
            lua_ptr,
        }))
    }

    /// Execute a Lua script and return the result
    pub fn exec(&self, script: &str) -> Option<LuaValue> {
        match self.lua.load(script).call::<LuaValue>(()) {
            Ok(val) => Some(val),
            Err(e) => {
                log::warn!("Lua exec error: {}", e);
                None
            }
        }
    }

    /// Execute a Lua file and return the result
    pub fn exec_file(&self, path: &Path) -> Option<LuaValue> {
        let path_str = path.to_string_lossy();
        match self
            .lua
            .load(std::fs::read_to_string(path).ok()?.as_str())
            .set_name(path_str.as_ref())
            .call::<LuaValue>(())
        {
            Ok(val) => Some(val),
            Err(e) => {
                log::warn!("Lua exec file error ({}): {}", path_str, e);
                None
            }
        }
    }

    /// Set the Lua package search directory
    pub fn set_directory(&self, path: &Path) {
        let path_str = path.to_string_lossy();
        let result: Result<(), LuaError> = (|| {
            let pkg: LuaTable = self.lua.globals().get("package")?;
            let current_path: String = pkg.get("path")?;
            let new_path = format!(
                "{};{}{}?.lua",
                current_path,
                path_str,
                std::path::MAIN_SEPARATOR
            );
            pkg.set("path", new_path)?;
            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("Failed to set Lua package.path: {}", e);
        }
    }

    /// Export MainState accessor functions to Lua
    /// When is_global is true, exported as global variables.
    /// When is_global is false, exported as module "main_state".
    pub fn export_main_state_accessor(&self, _state: &dyn MainState) {
        log::warn!("Lua state export not yet wired: export_main_state_accessor");
    }

    /// Export utility functions (timer_util, event_util) to Lua
    pub fn export_utilities(&self, _state: &dyn MainState) {
        log::warn!("Lua state export not yet wired: export_utilities");
    }

    /// Export skin property/configuration to Lua global variable skin_config
    pub fn export_skin_property(
        &self,
        header: &crate::lr2::lr2_skin_header_loader::LR2SkinHeaderData,
        property: &SkinConfigProperty,
        file_path_getter: &dyn Fn(&str) -> String,
    ) {
        let result: Result<(), LuaError> = (|| {
            let table = self.lua.create_table()?;

            // file_path table
            let file_path_table = self.lua.create_table()?;
            for file in &property.files {
                file_path_table.set(file.name.as_str(), file.path.as_str())?;
            }
            table.set("file_path", file_path_table)?;

            // get_path function
            // NOTE: We cannot capture file_path_getter into a Lua closure directly
            // because it borrows from the caller. Instead, log + skip for now.
            log::warn!(
                "Lua skin property: get_path function not fully wired (requires closure capture)"
            );

            // options table and enabled_options array
            let options_table = self.lua.create_table()?;
            let enabled_options_table = self.lua.create_table()?;
            let mut idx = 1;
            for option in &header.custom_options {
                let opvalue = option.get_selected_option();
                options_table.set(option.name.as_str(), opvalue)?;
                enabled_options_table.set(idx, opvalue)?;
                idx += 1;
            }
            table.set("option", options_table)?;
            table.set("enabled_options", enabled_options_table)?;

            // offsets table
            let offsets_table = self.lua.create_table()?;
            for offset_def in &header.custom_offsets {
                let ofs = property.offsets.iter().find(|o| o.name == offset_def.name);
                let offset_table = self.lua.create_table()?;
                if let Some(ofs) = ofs {
                    offset_table.set("x", ofs.x)?;
                    offset_table.set("y", ofs.y)?;
                    offset_table.set("w", ofs.w)?;
                    offset_table.set("h", ofs.h)?;
                    offset_table.set("r", ofs.r)?;
                    offset_table.set("a", ofs.a)?;
                    offsets_table.set(ofs.name.as_str(), offset_table)?;
                } else {
                    offset_table.set("x", 0.0)?;
                    offset_table.set("y", 0.0)?;
                    offset_table.set("w", 0.0)?;
                    offset_table.set("h", 0.0)?;
                    offset_table.set("r", 0.0)?;
                    offset_table.set("a", 0.0)?;
                    offsets_table.set(offset_def.name.as_str(), offset_table)?;
                }
            }
            table.set("offset", offsets_table)?;

            self.lua.globals().set("skin_config", table)?;
            Ok(())
        })();
        if let Err(e) = result {
            log::warn!("Failed to export skin property to Lua: {}", e);
        }
    }

    /// Get a reference to the underlying Lua VM
    pub fn lua(&self) -> &Lua {
        &self.lua
    }
}

/// Placeholder for SkinConfig.Property
#[derive(Clone, Debug, Default)]
pub struct SkinConfigProperty {
    pub files: Vec<SkinConfigFilePath>,
    pub offsets: Vec<crate::stubs::SkinConfigOffset>,
}

/// Placeholder for SkinConfig.FilePath
#[derive(Clone, Debug, Default)]
pub struct SkinConfigFilePath {
    pub name: String,
    pub path: String,
}

// ============================================================
// Lua-backed property implementations
// ============================================================

// SAFETY NOTE: These structs hold a raw pointer to the Lua VM that owns the
// registry keys. They are only valid as long as the SkinLuaAccessor (and its Lua)
// is alive. In beatoraja, properties are always used within the lifetime of their
// SkinLuaAccessor, so this is safe in practice. The Send+Sync impls are required
// by the property traits.

struct LuaBooleanProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

// SAFETY: The Lua VM is accessed single-threaded in beatoraja's skin system
unsafe impl Send for LuaBooleanProperty {}
unsafe impl Sync for LuaBooleanProperty {}

impl BooleanProperty for LuaBooleanProperty {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        false
    }

    fn get(&self, _state: &dyn MainState) -> bool {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaIntegerProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaIntegerProperty {}
unsafe impl Sync for LuaIntegerProperty {}

impl IntegerProperty for LuaIntegerProperty {
    fn get(&self, _state: &dyn MainState) -> i32 {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaFloatProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaFloatProperty {}
unsafe impl Sync for LuaFloatProperty {}

impl FloatProperty for LuaFloatProperty {
    fn get(&self, _state: &dyn MainState) -> f32 {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaStringProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaStringProperty {}
unsafe impl Sync for LuaStringProperty {}

impl StringProperty for LuaStringProperty {
    fn get(&self, _state: &dyn MainState) -> String {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaTimerProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaTimerProperty {}
unsafe impl Sync for LuaTimerProperty {}

impl TimerProperty for LuaTimerProperty {
    fn get_micro(&self, _state: &dyn MainState) -> i64 {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaEvent {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaEvent {}
unsafe impl Sync for LuaEvent {}

impl Event for LuaEvent {
    fn exec(&self, _state: &mut dyn MainState, arg1: i32, arg2: i32) {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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

struct LuaFloatWriter {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua_ptr: *const Lua,
}

unsafe impl Send for LuaFloatWriter {}
unsafe impl Sync for LuaFloatWriter {}

impl FloatWriter for LuaFloatWriter {
    fn set(&self, _state: &mut dyn MainState, value: f32) {
        let lua = unsafe { &*self.lua_ptr };
        let key = self.func_key.lock().unwrap();
        match lua.registry_value::<LuaFunction>(&key) {
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
