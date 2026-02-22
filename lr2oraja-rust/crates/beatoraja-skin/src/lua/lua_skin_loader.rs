use std::collections::HashMap;
use std::path::Path;

use mlua::prelude::*;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinConfigProperty, SkinData, SkinHeaderData};
use crate::lua::skin_lua_accessor::SkinLuaAccessor;
use crate::stubs::MainState;

/// Lua skin loader
///
/// Translated from LuaSkinLoader.java (171 lines)
/// Loads Lua-based skins. Extends JSONSkinLoader with Lua scripting support.
/// Uses SkinLuaAccessor for Lua VM integration.
/// Converts Lua tables to JsonSkin.Skin structures via serde_json::Value intermediate.
pub struct LuaSkinLoader {
    pub lua: SkinLuaAccessor,
    pub json_loader: JSONSkinLoader,
}

impl LuaSkinLoader {
    /// Create a new LuaSkinLoader (header-only mode)
    /// Corresponds to Java: new LuaSkinLoader()
    pub fn new() -> Self {
        Self {
            lua: SkinLuaAccessor::new(false),
            json_loader: JSONSkinLoader::new(),
        }
    }

    /// Create a new LuaSkinLoader with MainState and Config
    /// Corresponds to Java: new LuaSkinLoader(MainState, Config)
    pub fn new_with_state(_state: &dyn MainState, config: &beatoraja_core::config::Config) -> Self {
        Self {
            lua: SkinLuaAccessor::new(false),
            json_loader: JSONSkinLoader::with_config(config),
        }
    }

    /// Load skin header from Lua file
    /// Corresponds to Java: LuaSkinLoader.loadHeader(Path)
    pub fn load_header(&mut self, p: &Path) -> Option<SkinHeaderData> {
        // lua.setDirectory(p.getParent())
        if let Some(parent) = p.parent() {
            self.lua.set_directory(parent);
        }
        // LuaValue value = lua.execFile(p)
        let value = self.lua.exec_file(p)?;
        // sk = fromLuaValue(JsonSkin.Skin.class, value)
        let sk = from_lua_value_to_skin(&value)?;
        self.json_loader.sk = Some(sk.clone());
        // header = loadJsonSkinHeader(sk, p)
        self.json_loader.load_header_from_skin(&sk, p)
    }

    /// Load skin from Lua file
    /// Corresponds to Java: LuaSkinLoader.loadSkin(Path, SkinType, Property)
    pub fn load_skin(
        &mut self,
        p: &Path,
        skin_type: &crate::skin_type::SkinType,
        property: &SkinConfigProperty,
    ) -> Option<SkinData> {
        self.load(p, skin_type, property)
    }

    /// Load skin implementation
    /// Corresponds to Java: LuaSkinLoader.load(Path, SkinType, Property)
    pub fn load(
        &mut self,
        p: &Path,
        skin_type: &crate::skin_type::SkinType,
        property: &SkinConfigProperty,
    ) -> Option<SkinData> {
        // 1. Load header
        let header = self.load_header(p)?;

        // 2. Set up file map from custom files
        let mut filemap: HashMap<String, String> = HashMap::new();
        for file in &header.custom_files {
            if let Some(ref selected) = file.selected_filename {
                filemap.insert(file.path.clone(), selected.clone());
            }
        }
        self.json_loader.filemap = filemap;

        // 3. Re-execute Lua with skin property exported
        // lua.exportSkinProperty(header, property, pathGetter)
        // LuaValue value = lua.execFile(p)
        let value = self.lua.exec_file(p)?;
        // sk = fromLuaValue(JsonSkin.Skin.class, value)
        let sk = from_lua_value_to_skin(&value)?;
        self.json_loader.sk = Some(sk.clone());

        // 4. Load JSON skin from the Lua-produced structure
        self.json_loader.load_skin(p, skin_type, property)
    }
}

impl Default for LuaSkinLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a Lua value to a serde_json::Value, then deserialize to JsonSkin.Skin.
/// This is the Rust equivalent of fromLuaValue(JsonSkin.Skin.class, value) in Java.
///
/// The Java version uses reflection to iterate fields and recursively convert.
/// In Rust, we convert the Lua table → serde_json::Value → serde deserialize.
fn from_lua_value_to_skin(lua_value: &LuaValue) -> Option<json_skin::Skin> {
    let json_value = lua_to_json(lua_value)?;
    match serde_json::from_value::<json_skin::Skin>(json_value) {
        Ok(skin) => Some(skin),
        Err(e) => {
            log::warn!("Failed to deserialize Lua value to JsonSkin.Skin: {}", e);
            None
        }
    }
}

/// Recursively convert a Lua value to a serde_json::Value.
/// Handles: nil, boolean, integer, number, string, table (array or object).
fn lua_to_json(value: &LuaValue) -> Option<serde_json::Value> {
    match value {
        LuaValue::Nil => Some(serde_json::Value::Null),
        LuaValue::Boolean(b) => Some(serde_json::Value::Bool(*b)),
        LuaValue::Integer(i) => Some(serde_json::json!(*i)),
        LuaValue::Number(f) => Some(serde_json::json!(*f)),
        LuaValue::String(s) => {
            let s = s.to_str().map(|s| s.to_string()).unwrap_or_default();
            Some(serde_json::Value::String(s))
        }
        LuaValue::Table(table) => {
            // Determine if this is an array (sequential integer keys starting at 1)
            // or an object (string keys).
            let len = table.raw_len();
            if len > 0 {
                // Check if it's a pure sequence
                let mut is_array = true;
                let mut max_key = 0i64;
                for (key, _) in table.clone().pairs::<LuaValue, LuaValue>().flatten() {
                    match key {
                        LuaValue::Integer(i) => {
                            if i > max_key {
                                max_key = i;
                            }
                        }
                        _ => {
                            is_array = false;
                            break;
                        }
                    }
                }
                if is_array && max_key == len as i64 {
                    // Pure array
                    let mut arr = Vec::with_capacity(len);
                    for i in 1..=len {
                        let val: LuaValue = table.raw_get(i).unwrap_or(LuaValue::Nil);
                        arr.push(lua_to_json(&val).unwrap_or(serde_json::Value::Null));
                    }
                    return Some(serde_json::Value::Array(arr));
                }
            }

            // Object or mixed table: convert to JSON object
            let mut map = serde_json::Map::new();
            for (key, val) in table.clone().pairs::<LuaValue, LuaValue>().flatten() {
                let key_str = match &key {
                    LuaValue::String(s) => s.to_str().map(|s| s.to_string()).unwrap_or_default(),
                    LuaValue::Integer(i) => i.to_string(),
                    _ => continue,
                };
                if let Some(json_val) = lua_to_json(&val) {
                    map.insert(key_str, json_val);
                }
            }
            Some(serde_json::Value::Object(map))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lua_to_json_primitives() {
        assert_eq!(lua_to_json(&LuaValue::Nil), Some(serde_json::Value::Null));
        assert_eq!(
            lua_to_json(&LuaValue::Boolean(true)),
            Some(serde_json::Value::Bool(true))
        );
        assert_eq!(
            lua_to_json(&LuaValue::Integer(42)),
            Some(serde_json::json!(42))
        );
        assert_eq!(
            lua_to_json(&LuaValue::Number(3.14)),
            Some(serde_json::json!(3.14))
        );
    }

    #[test]
    fn test_lua_to_json_table_as_object() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("name", "test_skin").unwrap();
        table.set("w", 1920).unwrap();
        table.set("h", 1080).unwrap();

        let json = lua_to_json(&LuaValue::Table(table)).unwrap();
        assert_eq!(json["name"], "test_skin");
        assert_eq!(json["w"], 1920);
        assert_eq!(json["h"], 1080);
    }

    #[test]
    fn test_lua_to_json_table_as_array() {
        let lua = Lua::new();
        let table = lua.create_sequence_from([10, 20, 30]).unwrap();

        let json = lua_to_json(&LuaValue::Table(table)).unwrap();
        assert!(json.is_array());
        let arr = json.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0], 10);
        assert_eq!(arr[1], 20);
        assert_eq!(arr[2], 30);
    }

    #[test]
    fn test_from_lua_value_to_skin_minimal() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("type", 0i32).unwrap();
        table.set("name", "TestSkin").unwrap();
        table.set("w", 1920i32).unwrap();
        table.set("h", 1080i32).unwrap();

        let skin = from_lua_value_to_skin(&LuaValue::Table(table));
        assert!(skin.is_some());
        let skin = skin.unwrap();
        assert_eq!(skin.skin_type, 0);
        assert_eq!(skin.name, Some("TestSkin".to_string()));
        assert_eq!(skin.w, 1920);
        assert_eq!(skin.h, 1080);
    }

    #[test]
    fn test_from_lua_value_to_skin_with_source() {
        let lua = Lua::new();
        let table = lua.create_table().unwrap();
        table.set("type", 5i32).unwrap();
        table.set("w", 1920i32).unwrap();
        table.set("h", 1080i32).unwrap();

        // Add a source array
        let source = lua.create_table().unwrap();
        source.set("id", "bg").unwrap();
        source.set("path", "bg.png").unwrap();
        let sources = lua.create_sequence_from([source]).unwrap();
        table.set("source", sources).unwrap();

        let skin = from_lua_value_to_skin(&LuaValue::Table(table));
        assert!(skin.is_some());
        let skin = skin.unwrap();
        assert_eq!(skin.source.len(), 1);
        assert_eq!(skin.source[0].id, Some("bg".to_string()));
        assert_eq!(skin.source[0].path, Some("bg.png".to_string()));
    }

    #[test]
    fn test_lua_loader_default() {
        let loader = LuaSkinLoader::new();
        assert!(loader.json_loader.sk.is_none());
    }
}
