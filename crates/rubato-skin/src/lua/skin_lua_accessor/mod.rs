mod loaders;
pub mod property_impls;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use mlua::StdLib;
use mlua::prelude::*;

use crate::lua::event_utility::EventUtility;
use crate::lua::main_state_accessor::MainStateAccessor;
use crate::lua::timer_utility::TimerUtility;
use crate::reexports::MainState;

pub use property_impls::{LuaFloatProperty, LuaTimerProperty};

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
    /// The initial Lua package.path value, captured at construction time.
    /// Used to reset package.path in set_directory() to prevent unbounded
    /// growth from repeated skin loads (Java creates a new VM per load).
    base_package_path: String,
}

impl SkinLuaAccessor {
    pub fn new(is_global: bool) -> Self {
        // Arc<Lua> is intentional: Lua is !Send+!Sync, but property types share ownership
        // of the VM via Arc (not across threads). Thread-safety is enforced via creation_thread_id
        // assertions in each property type's get() method.
        //
        // SECURITY: Only load safe library subset. Skin Lua scripts must not have access to
        // OS (command execution), IO (file system), or DEBUG (sandbox escape) libraries.
        // The skin code needs: base functions (auto-loaded), table, string, math, utf8,
        // coroutine, and package (for require/package.loaded module system).
        let safe_libs = StdLib::TABLE
            | StdLib::STRING
            | StdLib::MATH
            | StdLib::UTF8
            | StdLib::COROUTINE
            | StdLib::PACKAGE;
        #[allow(clippy::arc_with_non_send_sync)]
        let lua = Arc::new(
            Lua::new_with(safe_libs, mlua::LuaOptions::default())
                .expect("Failed to create sandboxed Lua VM"),
        );

        // Capture the initial package.path before any modifications
        let base_package_path = lua
            .globals()
            .get::<LuaTable>("package")
            .and_then(|pkg| pkg.get::<String>("path"))
            .unwrap_or_default();

        if !is_global {
            // Pre-register empty tables so require("main_state") etc. don't error during header loading
            lua.scope(|_scope| {
                let loaded: LuaTable = lua
                    .globals()
                    .get::<LuaTable>("package")
                    .and_then(|pkg| pkg.get::<LuaTable>("loaded"))
                    .expect("Lua package.loaded");
                let _ = loaded.set(MAIN_STATE, lua.create_table().expect("Lua table creation"));
                let _ = loaded.set(TIMER_UTIL, lua.create_table().expect("Lua table creation"));
                let _ = loaded.set(EVENT_UTIL, lua.create_table().expect("Lua table creation"));
                Ok(())
            })
            .unwrap_or_else(|e| {
                log::warn!("Failed to initialize Lua module tables: {}", e);
            });
        }

        Self {
            is_global,
            lua,
            base_package_path,
        }
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

    /// Execute a Lua file and return the result.
    /// Falls back to Shift_JIS decoding when the file is not valid UTF-8.
    pub fn exec_file(&self, path: &Path) -> Option<LuaValue> {
        let path_str = path.to_string_lossy();
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                // Fallback: read raw bytes and decode as Shift_JIS (CP932)
                let bytes = std::fs::read(path).ok()?;
                let (decoded, _, _) = encoding_rs::SHIFT_JIS.decode(&bytes);
                decoded.into_owned()
            }
        };
        match self
            .lua
            .load(source.as_str())
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

    /// Set the Lua package search directory.
    /// Resets package.path to the initial base value before appending the new
    /// directory, preventing unbounded growth from repeated skin loads.
    /// Java creates a new Lua VM per load, so this reset emulates that behavior.
    pub fn set_directory(&self, path: &Path) {
        let path_str = path.to_string_lossy();
        let result: Result<(), LuaError> = (|| {
            let pkg: LuaTable = self.lua.globals().get("package")?;
            let new_path = format!(
                "{};{}{}?.lua",
                self.base_package_path,
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

            // path function: resolve custom file paths
            // Build a filemap from property files (name -> path)
            let filemap: std::collections::HashMap<String, String> = property
                .files
                .iter()
                .map(|f| (f.name.clone(), f.path.clone()))
                .collect();
            let get_path_fn = self.lua.create_function(move |_, path: String| {
                let result = crate::skin_loader::path(&path, &filemap);
                Ok(result.to_string_lossy().to_string())
            })?;
            table.set("path", get_path_fn)?;

            // options table and enabled_options array
            let options_table = self.lua.create_table()?;
            let enabled_options_table = self.lua.create_table()?;
            let mut idx = 1;
            for option in &header.custom_options {
                let opvalue = option.selected_option();
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

            // path function
            let filemap_clone = filemap.clone();
            let get_path_fn = self.lua.create_function(move |_, path: String| {
                let result = crate::skin_loader::path(&path, &filemap_clone);
                Ok(result.to_string_lossy().to_string())
            })?;
            table.set("path", get_path_fn)?;

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
    pub offsets: Vec<crate::reexports::SkinConfigOffset>,
}

/// Placeholder for SkinConfig.FilePath
#[derive(Clone, Debug, Default)]
pub struct SkinConfigFilePath {
    pub name: String,
    pub path: String,
}
