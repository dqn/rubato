use std::path::Path;

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
///
/// Uses todo!("mlua integration") for Lua VM calls.
const MAIN_STATE: &str = "main_state";
const TIMER_UTIL: &str = "timer_util";
const EVENT_UTIL: &str = "event_util";

pub struct SkinLuaAccessor {
    /// Whether to export to global scope (true) or as modules (false)
    is_global: bool,
    // globals: LuaVM - would be mlua::Lua in actual implementation
}

impl SkinLuaAccessor {
    pub fn new(is_global: bool) -> Self {
        // globals = JsePlatform.standardGlobals()
        // if !isGlobal, set empty tables for main_state, timer_util, event_util
        Self { is_global }
    }

    /// Load a BooleanProperty from a Lua script string
    pub fn load_boolean_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn BooleanProperty>> {
        todo!("mlua integration: load boolean property from script")
    }

    /// Load a BooleanProperty from a Lua function
    pub fn load_boolean_property_from_function(&self) -> Option<Box<dyn BooleanProperty>> {
        todo!("mlua integration: load boolean property from function")
    }

    /// Load an IntegerProperty from a Lua script string
    pub fn load_integer_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn IntegerProperty>> {
        todo!("mlua integration: load integer property from script")
    }

    /// Load an IntegerProperty from a Lua function
    pub fn load_integer_property_from_function(&self) -> Option<Box<dyn IntegerProperty>> {
        todo!("mlua integration: load integer property from function")
    }

    /// Load a FloatProperty from a Lua script string
    pub fn load_float_property_from_script(&self, script: &str) -> Option<Box<dyn FloatProperty>> {
        todo!("mlua integration: load float property from script")
    }

    /// Load a FloatProperty from a Lua function
    pub fn load_float_property_from_function(&self) -> Option<Box<dyn FloatProperty>> {
        todo!("mlua integration: load float property from function")
    }

    /// Load a StringProperty from a Lua script string
    pub fn load_string_property_from_script(
        &self,
        script: &str,
    ) -> Option<Box<dyn StringProperty>> {
        todo!("mlua integration: load string property from script")
    }

    /// Load a StringProperty from a Lua function
    pub fn load_string_property_from_function(&self) -> Option<Box<dyn StringProperty>> {
        todo!("mlua integration: load string property from function")
    }

    /// Load a TimerProperty from a Lua script string
    /// If the script returns a function, that function is used as a timer function.
    /// Otherwise, the script itself is regarded as a timer function.
    /// A timer function returns start time in microseconds if on, or i64::MIN if off.
    pub fn load_timer_property_from_script(&self, script: &str) -> Option<Box<dyn TimerProperty>> {
        todo!("mlua integration: load timer property from script")
    }

    /// Load a TimerProperty from a Lua function
    pub fn load_timer_property_from_function(&self) -> Option<Box<dyn TimerProperty>> {
        todo!("mlua integration: load timer property from function")
    }

    /// Load an Event from a Lua script string
    pub fn load_event_from_script(&self, script: &str) -> Option<Box<dyn Event>> {
        todo!("mlua integration: load event from script")
    }

    /// Load an Event from a Lua function
    pub fn load_event_from_function(&self) -> Option<Box<dyn Event>> {
        todo!("mlua integration: load event from function")
    }

    /// Load a FloatWriter from a Lua script string
    pub fn load_float_writer_from_script(&self, script: &str) -> Option<Box<dyn FloatWriter>> {
        todo!("mlua integration: load float writer from script")
    }

    /// Load a FloatWriter from a Lua function
    pub fn load_float_writer_from_function(&self) -> Option<Box<dyn FloatWriter>> {
        todo!("mlua integration: load float writer from function")
    }

    /// Execute a Lua script and return the result
    pub fn exec(&self, script: &str) {
        todo!("mlua integration: exec script")
    }

    /// Execute a Lua file and return the result
    pub fn exec_file(&self, path: &Path) {
        todo!("mlua integration: exec file")
    }

    /// Set the Lua package search directory
    pub fn set_directory(&self, path: &Path) {
        todo!("mlua integration: set directory for package.path")
    }

    /// Export MainState accessor functions to Lua
    /// When is_global is true, exported as global variables.
    /// When is_global is false, exported as module "main_state".
    pub fn export_main_state_accessor(&self, _state: &dyn MainState) {
        todo!("mlua integration: export main state accessor")
    }

    /// Export utility functions (timer_util, event_util) to Lua
    pub fn export_utilities(&self, _state: &dyn MainState) {
        todo!("mlua integration: export utilities")
    }

    /// Export skin property/configuration to Lua global variable skin_config
    pub fn export_skin_property(
        &self,
        _header: &crate::lr2::lr2_skin_header_loader::LR2SkinHeaderData,
        _property: &SkinConfigProperty,
        _file_path_getter: &dyn Fn(&str) -> String,
    ) {
        todo!("mlua integration: export skin property")
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
