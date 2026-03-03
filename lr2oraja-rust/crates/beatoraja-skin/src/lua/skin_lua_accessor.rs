use std::path::Path;
use std::sync::{Arc, Mutex};

use mlua::prelude::*;

use crate::lua::event_utility::EventUtility;
use crate::lua::main_state_accessor::MainStateAccessor;
use crate::lua::timer_utility::TimerUtility;
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
    /// The Lua VM instance, wrapped in Arc so Lua property types can share
    /// ownership and the VM cannot be dropped while properties are alive.
    lua: Arc<Lua>,
}

impl SkinLuaAccessor {
    pub fn new(is_global: bool) -> Self {
        // Arc<Lua> is intentional: Lua is !Send+!Sync, but property types share ownership
        // of the VM via Arc (not across threads). Thread-safety is enforced via creation_thread_id
        // assertions in each property type's get() method.
        #[allow(clippy::arc_with_non_send_sync)]
        let lua = Arc::new(Lua::new());

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
    pub fn load_boolean_property_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn BooleanProperty>> {
        self.load_boolean_property_from_lua_function(func)
    }

    fn load_boolean_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn BooleanProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaBooleanProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_integer_property_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn IntegerProperty>> {
        self.load_integer_property_from_lua_function(func)
    }

    fn load_integer_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn IntegerProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaIntegerProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_float_property_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatProperty>> {
        self.load_float_property_from_lua_function(func)
    }

    fn load_float_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaFloatProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_string_property_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn StringProperty>> {
        self.load_string_property_from_lua_function(func)
    }

    fn load_string_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn StringProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaStringProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_timer_property_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn TimerProperty>> {
        self.load_timer_property_from_lua_function(func)
    }

    fn load_timer_property_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn TimerProperty>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaTimerProperty {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_event_from_function(&self, func: LuaFunction) -> Option<Box<dyn Event>> {
        self.load_event_from_lua_function(func)
    }

    fn load_event_from_lua_function(&self, func: LuaFunction) -> Option<Box<dyn Event>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaEvent {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    pub fn load_float_writer_from_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatWriter>> {
        self.load_float_writer_from_lua_function(func)
    }

    fn load_float_writer_from_lua_function(
        &self,
        func: LuaFunction,
    ) -> Option<Box<dyn FloatWriter>> {
        let func_key = self.lua.create_registry_value(func).ok()?;
        Some(Box::new(LuaFloatWriter {
            func_key: Arc::new(Mutex::new(func_key)),
            lua: Arc::clone(&self.lua),
            creation_thread_id: std::thread::current().id(),
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
    ///
    /// # Safety
    /// `state` must point to a valid `dyn MainState` that outlives the Lua VM
    /// and any closures exported from it. The caller must ensure single-threaded
    /// access to the state while Lua callbacks are active.
    pub unsafe fn export_main_state_accessor(&self, state: *mut dyn MainState) {
        let accessor = unsafe { MainStateAccessor::new(state) };
        if self.is_global {
            let globals = self.lua.globals();
            accessor.export(&self.lua, &globals);
        } else {
            let result: Result<(), LuaError> = (|| {
                let table = self.lua.create_table()?;
                accessor.export(&self.lua, &table);
                let loaded: LuaTable = self
                    .lua
                    .globals()
                    .get::<LuaTable>("package")?
                    .get::<LuaTable>("loaded")?;
                loaded.set(MAIN_STATE, table)?;
                Ok(())
            })();
            if let Err(e) = result {
                log::warn!("Failed to export main_state module: {}", e);
            }
        }
    }

    /// Export utility functions (timer_util, event_util) to Lua
    pub fn export_utilities(&self, state: &dyn MainState) {
        let timer_util = TimerUtility::new(state);
        let event_util = EventUtility::new(state);
        if self.is_global {
            let globals = self.lua.globals();
            timer_util.export(&self.lua, &globals);
            event_util.export(&self.lua, &globals);
        } else {
            let result: Result<(), LuaError> = (|| {
                let loaded: LuaTable = self
                    .lua
                    .globals()
                    .get::<LuaTable>("package")?
                    .get::<LuaTable>("loaded")?;

                let timer_table = self.lua.create_table()?;
                timer_util.export(&self.lua, &timer_table);
                loaded.set(TIMER_UTIL, timer_table)?;

                let event_table = self.lua.create_table()?;
                event_util.export(&self.lua, &event_table);
                loaded.set(EVENT_UTIL, event_table)?;

                Ok(())
            })();
            if let Err(e) = result {
                log::warn!("Failed to export utility modules: {}", e);
            }
        }
    }

    /// Export skin property/configuration to Lua global variable skin_config
    pub fn export_skin_property(
        &self,
        header: &crate::lr2::lr2_skin_header_loader::LR2SkinHeaderData,
        property: &SkinConfigProperty,
        _file_path_getter: &dyn Fn(&str) -> String,
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

    /// Export skin_config to Lua from a SkinHeaderData (JSON/Lua skin pipeline).
    /// Corresponds to Java: SkinLuaAccessor.exportSkinProperty(SkinHeader, Property, pathGetter)
    pub fn export_skin_property_from_header_data(
        &self,
        header: &crate::json::json_skin_loader::SkinHeaderData,
        filemap: &std::collections::HashMap<String, String>,
    ) {
        let result: Result<(), LuaError> = (|| {
            let table = self.lua.create_table()?;

            // file_path table
            let file_path_table = self.lua.create_table()?;
            for file in &header.custom_files {
                if let Some(ref selected) = file.selected_filename {
                    file_path_table.set(file.path.as_str(), selected.as_str())?;
                }
            }
            table.set("file_path", file_path_table)?;

            // get_path function
            let filemap_clone = filemap.clone();
            let get_path_fn = self.lua.create_function(move |_, path: String| {
                let result = crate::skin_loader::get_path(&path, &filemap_clone);
                Ok(result.to_string_lossy().to_string())
            })?;
            table.set("get_path", get_path_fn)?;

            // options table and enabled_options array
            // Java: when selectedOption is RANDOM_VALUE (-1) or unset (0), pick first valid op.
            let options_table = self.lua.create_table()?;
            let enabled_options_table = self.lua.create_table()?;
            let mut idx = 1;
            for option in &header.custom_options {
                let opvalue = if option.option.contains(&option.selected_option) {
                    option.selected_option
                } else if !option.option.is_empty() {
                    option.option[0]
                } else {
                    option.selected_option
                };
                options_table.set(option.name.as_str(), opvalue)?;
                enabled_options_table.set(idx, opvalue)?;
                idx += 1;
            }
            table.set("option", options_table)?;
            table.set("enabled_options", enabled_options_table)?;

            // offsets table (all defaults -- actual values set by setSkinConfigProperty)
            let offsets_table = self.lua.create_table()?;
            for offset_def in &header.custom_offsets {
                let offset_table = self.lua.create_table()?;
                offset_table.set("x", 0.0)?;
                offset_table.set("y", 0.0)?;
                offset_table.set("w", 0.0)?;
                offset_table.set("h", 0.0)?;
                offset_table.set("r", 0.0)?;
                offset_table.set("a", 0.0)?;
                offsets_table.set(offset_def.name.as_str(), offset_table)?;
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

// SAFETY NOTE: These structs hold an Arc<Lua> that shares ownership of the Lua VM
// with the SkinLuaAccessor. The Arc ensures the VM cannot be dropped while any
// property is alive, preventing use-after-free. The Send+Sync impls are required
// by the property traits; Lua (without the "send" feature) is !Send, so we rely
// on the single-threaded access invariant. The creation_thread_id field enables
// debug_assert checks that detect cross-thread access at runtime in debug builds.

struct LuaBooleanProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: The Lua VM is accessed single-threaded in beatoraja's skin system.
// debug_assert in get() verifies this invariant at runtime in debug builds.
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
        let key = self.func_key.lock().unwrap();
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

struct LuaIntegerProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaIntegerProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaIntegerProperty {}
unsafe impl Sync for LuaIntegerProperty {}

impl IntegerProperty for LuaIntegerProperty {
    fn get(&self, _state: &dyn MainState) -> i32 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaIntegerProperty must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

struct LuaFloatProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaFloatProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaFloatProperty {}
unsafe impl Sync for LuaFloatProperty {}

impl FloatProperty for LuaFloatProperty {
    fn get(&self, _state: &dyn MainState) -> f32 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaFloatProperty must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

struct LuaStringProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaStringProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaStringProperty {}
unsafe impl Sync for LuaStringProperty {}

impl StringProperty for LuaStringProperty {
    fn get(&self, _state: &dyn MainState) -> String {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaStringProperty must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

struct LuaTimerProperty {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaTimerProperty contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in get_micro() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaTimerProperty {}
unsafe impl Sync for LuaTimerProperty {}

impl TimerProperty for LuaTimerProperty {
    fn get_micro(&self, _state: &dyn MainState) -> i64 {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaTimerProperty must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

struct LuaEvent {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaEvent contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in exec() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaEvent {}
unsafe impl Sync for LuaEvent {}

impl Event for LuaEvent {
    fn exec(&self, _state: &mut dyn MainState, arg1: i32, arg2: i32) {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaEvent must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

struct LuaFloatWriter {
    func_key: Arc<Mutex<LuaRegistryKey>>,
    lua: Arc<Lua>,
    creation_thread_id: std::thread::ThreadId,
}

// SAFETY: LuaFloatWriter contains Arc<Lua> which is !Send because mlua::Lua
// (without the "send" feature) is not thread-safe. Access is restricted to a single
// thread; debug_assert in set() verifies this invariant at runtime in debug builds.
unsafe impl Send for LuaFloatWriter {}
unsafe impl Sync for LuaFloatWriter {}

impl FloatWriter for LuaFloatWriter {
    fn set(&self, _state: &mut dyn MainState, value: f32) {
        debug_assert_eq!(
            std::thread::current().id(),
            self.creation_thread_id,
            "LuaFloatWriter must be accessed on the thread where it was created"
        );
        let key = self.func_key.lock().unwrap();
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};

    /// Minimal mock MainState for Lua property tests.
    struct MockMainState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
    }

    impl Default for MockMainState {
        fn default() -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
            }
        }
    }

    impl MainState for MockMainState {
        fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
            &self.timer
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
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
    }

    #[test]
    fn boolean_property_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        // load_boolean_property_from_script prepends "return ", so the script
        // becomes "return true" which evaluates to a boolean value directly.
        let prop = accessor
            .load_boolean_property_from_script("true")
            .expect("should load boolean property");
        let state = MockMainState::default();
        assert!(prop.get(&state));
    }

    #[test]
    fn integer_property_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let prop = accessor
            .load_integer_property_from_script("42")
            .expect("should load integer property");
        let state = MockMainState::default();
        assert_eq!(prop.get(&state), 42);
    }

    #[test]
    fn float_property_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let prop = accessor
            .load_float_property_from_script("3.14")
            .expect("should load float property");
        let state = MockMainState::default();
        assert!((prop.get(&state) - 3.14).abs() < 0.01);
    }

    #[test]
    fn string_property_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let prop = accessor
            .load_string_property_from_script("'hello'")
            .expect("should load string property");
        let state = MockMainState::default();
        assert_eq!(prop.get(&state), "hello");
    }

    #[test]
    fn timer_property_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        // Timer property has special handling: if the script returns a function,
        // that function is used as the timer function (trial call mechanism).
        let prop = accessor
            .load_timer_property_from_script("function() return 1000000 end")
            .expect("should load timer property");
        let state = MockMainState::default();
        assert_eq!(prop.get_micro(&state), 1000000);
    }

    #[test]
    fn event_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        // load_event_from_script loads the script directly (no "return " prefix).
        // The script must be a valid Lua chunk that compiles to a function.
        // Use "return function(a, b) end" so the chunk returns a callable function.
        let func = accessor
            .lua()
            .load("return function(a, b) end")
            .into_function()
            .expect("should compile event chunk");
        let result: LuaValue = func.call(()).expect("should call chunk");
        if let LuaValue::Function(inner) = result {
            let event = accessor
                .load_event_from_function(inner)
                .expect("should load event");
            let mut state = MockMainState::default();
            // Should not panic
            event.exec(&mut state, 1, 2);
        } else {
            panic!("Expected Lua function from chunk");
        }
    }

    #[test]
    fn float_writer_works_on_creation_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let func = accessor
            .lua()
            .load("return function(v) end")
            .into_function()
            .expect("should compile float writer chunk");
        let result: LuaValue = func.call(()).expect("should call chunk");
        if let LuaValue::Function(inner) = result {
            let writer = accessor
                .load_float_writer_from_function(inner)
                .expect("should load float writer");
            let mut state = MockMainState::default();
            // Should not panic
            writer.set(&mut state, 1.0);
        } else {
            panic!("Expected Lua function from chunk");
        }
    }

    /// Verify that the debug_assert fires when a Lua property is accessed from a
    /// different thread than where it was created. This test only runs in debug mode.
    #[test]
    #[cfg(debug_assertions)]
    fn boolean_property_panics_on_wrong_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let prop = accessor
            .load_boolean_property_from_script("true")
            .expect("should load boolean property");
        let state = MockMainState::default();

        // Access from a different thread should panic due to debug_assert
        let handle = std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prop.get(&state);
            }));
            assert!(
                result.is_err(),
                "Expected panic when accessing LuaBooleanProperty from wrong thread"
            );
        });
        handle.join().expect("thread should complete");
    }

    /// Verify that the debug_assert fires for integer property on wrong thread.
    #[test]
    #[cfg(debug_assertions)]
    fn integer_property_panics_on_wrong_thread() {
        let accessor = SkinLuaAccessor::new(true);
        let prop = accessor
            .load_integer_property_from_script("42")
            .expect("should load integer property");
        let state = MockMainState::default();

        let handle = std::thread::spawn(move || {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                prop.get(&state);
            }));
            assert!(
                result.is_err(),
                "Expected panic when accessing LuaIntegerProperty from wrong thread"
            );
        });
        handle.join().expect("thread should complete");
    }
}
