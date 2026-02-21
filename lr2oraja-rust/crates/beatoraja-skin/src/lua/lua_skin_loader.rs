use std::path::Path;

use crate::lua::skin_lua_accessor::{SkinConfigProperty, SkinLuaAccessor};
use crate::stubs::MainState;

/// Lua skin loader
///
/// Translated from LuaSkinLoader.java (171 lines)
/// Loads Lua-based skins. Extends JSONSkinLoader with Lua scripting support.
/// Uses SkinLuaAccessor for Lua VM integration.
/// Converts Lua tables to JsonSkin.Skin structures using reflection-like deserialization.
pub struct LuaSkinLoader {
    pub lua: SkinLuaAccessor,
    // state: Option<&dyn MainState>,
    // config: Option<Config>,
}

impl LuaSkinLoader {
    /// Create a new LuaSkinLoader (header-only mode)
    pub fn new() -> Self {
        Self {
            lua: SkinLuaAccessor::new(false),
        }
    }

    /// Create a new LuaSkinLoader with MainState and Config
    pub fn new_with_state(
        _state: &dyn MainState,
        _config: &beatoraja_core::config::Config,
    ) -> Self {
        Self {
            lua: SkinLuaAccessor::new(false),
        }
    }

    /// Load skin header from Lua file
    pub fn load_header(&mut self, p: &Path) -> Option<()> {
        // lua.setDirectory(p.getParent())
        // LuaValue value = lua.execFile(p)
        // sk = fromLuaValue(JsonSkin.Skin.class, value)
        // header = loadJsonSkinHeader(sk, p)
        todo!("mlua integration: load header from Lua file")
    }

    /// Load skin from Lua file
    pub fn load_skin(
        &mut self,
        p: &Path,
        _skin_type: &crate::skin_type::SkinType,
        _property: &SkinConfigProperty,
    ) -> Option<()> {
        self.load(p, _skin_type, _property)
    }

    /// Load skin implementation
    pub fn load(
        &mut self,
        p: &Path,
        _skin_type: &crate::skin_type::SkinType,
        _property: &SkinConfigProperty,
    ) -> Option<()> {
        // 1. Load header
        // let header = self.load_header(p)?;
        // header.setSkinConfigProperty(property)

        // 2. Set up file map from custom files
        // filemap = ObjectMap::new()
        // for customFile in header.getCustomFiles() {
        //     if customFile.getSelectedFilename().is_some() {
        //         filemap.put(customFile.path, customFile.getSelectedFilename())
        //     }
        // }

        // 3. Export skin property and re-execute Lua
        // lua.exportSkinProperty(header, property, |path| {
        //     getPath(p.parent().to_string() + "/" + path, filemap).getPath()
        // })
        // let value = lua.execFile(p)
        // sk = fromLuaValue(JsonSkin.Skin.class, value)

        // 4. Load JSON skin from the Lua-produced structure
        // skin = loadJsonSkin(header, sk, type, property, p)

        todo!("mlua integration: load skin from Lua file")
    }

    /// Deserialize a Lua value into a Rust type
    /// This is the Rust equivalent of fromLuaValue() in Java.
    /// Handles: bool, int, float, String, arrays, and struct types.
    /// Also handles BooleanProperty, IntegerProperty, FloatProperty,
    /// StringProperty, TimerProperty, FloatWriter, Event by dispatching
    /// to the appropriate SkinLuaAccessor method.
    pub fn from_lua_value<T>(&self, _lua_value: &()) -> Option<T> {
        // Uses serializerMap equivalent:
        // - bool/Boolean -> LuaValue::toboolean
        // - int/Integer -> LuaValue::toint
        // - float/Float -> LuaValue::tofloat
        // - String -> LuaValue::tojstring
        // - BooleanProperty -> serializeLuaScript(lv, lua::loadBooleanProperty, ...)
        // - IntegerProperty -> serializeLuaScript(lv, lua::loadIntegerProperty, ...)
        // - FloatProperty -> serializeLuaScript(lv, lua::loadFloatProperty, ...)
        // - StringProperty -> serializeLuaScript(lv, lua::loadStringProperty, ...)
        // - TimerProperty -> serializeLuaScript(lv, lua::loadTimerProperty, ...)
        // - FloatWriter -> serializeLuaScript(lv, lua::loadFloatWriter, ...)
        // - Event -> serializeLuaScript(lv, lua::loadEvent, ...)
        //
        // For arrays: LuaTable keys -> Array.newInstance
        // For structs: reflection over fields, matching Lua table keys to field names
        todo!("mlua integration: deserialize Lua value to Rust type")
    }
}

impl Default for LuaSkinLoader {
    fn default() -> Self {
        Self::new()
    }
}
