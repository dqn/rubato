use std::collections::HashMap;
use std::path::Path;

use mlua::prelude::*;

use crate::json::json_skin;
use crate::json::json_skin_loader::{JSONSkinLoader, SkinConfigProperty, SkinData, SkinHeaderData};
use crate::lua::skin_lua_accessor::SkinLuaAccessor;
use crate::reexports::MainState;

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

    /// Create a new LuaSkinLoader with MainState and Config.
    ///
    /// The caller must keep `state` alive while the loader/Lua VM is in use
    /// because exported Lua closures retain a raw pointer to it.
    pub fn new_with_state(state: &mut dyn MainState, config: &rubato_core::config::Config) -> Self {
        let loader = Self::new_without_state(config);
        let state_ptr: *mut dyn MainState =
            unsafe { std::mem::transmute(state as *mut dyn MainState) };
        // SAFETY: the caller keeps `state` alive for the loader's lifetime; this
        // matches MainStateAccessor's raw-pointer contract. The transmute only
        // erases the trait-object lifetime; mutability already comes from the
        // caller's `&mut dyn MainState`.
        unsafe {
            loader.lua.export_main_state_accessor(state_ptr);
        }
        loader.lua.export_utilities(state);
        loader
    }

    /// Create a new LuaSkinLoader with Config only (no MainState reference needed)
    pub fn new_without_state(config: &rubato_core::config::Config) -> Self {
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

        // 3. Export skin property and re-execute Lua
        self.lua
            .export_skin_property_from_header_data(&header, &self.json_loader.filemap);
        let value = self.lua.exec_file(p)?;
        let sk = from_lua_value_to_skin(&value)?;
        self.json_loader.sk = Some(sk.clone());

        // 4. Convert Lua-produced structure via JSON skin pipeline
        // Call load_json_skin directly — load_skin would re-parse the .luaskin file as JSON.
        self.json_loader.serializer =
            Some(crate::json::json_skin_serializer::JsonSkinSerializer::new());
        self.json_loader
            .load_json_skin(&header, &sk, skin_type, property, p)
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
    // Coerce numbers→strings and empty objects→arrays to match json_skin types.
    // Java's fromLuaValue uses reflection to call toString() on String fields;
    // Lua skins commonly use integers for id/src fields that json_skin expects as String.
    let json_value = coerce_json_for_skin(json_value);
    match serde_json::from_value::<json_skin::Skin>(json_value) {
        Ok(skin) => Some(skin),
        Err(e) => {
            log::warn!("Failed to deserialize Lua value to JsonSkin.Skin: {}", e);
            None
        }
    }
}

/// Keys whose values should always be JSON strings (Option<String> or String in json_skin).
/// "id" is included here; the 3 structs where id is i32 (Offset, CustomEvent, CustomTimer)
/// use a custom deserializer that accepts both strings and integers.
const STRING_FIELD_KEYS: &[&str] = &[
    "id",
    "src",
    "path",
    "name",
    "author",
    "font",
    "category",
    "def",
    "constantText",
];

/// Keys whose values should be arrays (Vec<String> in json_skin).
/// Lua skins sometimes produce empty tables `{}` instead of empty arrays `[]`.
/// Note: most empty-object cases are handled by removing the key (see coerce_json_for_skin),
/// but non-empty maps that should be arrays still need explicit handling.
const VEC_STRING_FIELD_KEYS: &[&str] = &[
    "hidden",
    "processed",
    "note",
    "lnstart",
    "lnend",
    "lnbody",
    "lnbodyActive",
    "lnactive",
    "hcnstart",
    "hcnend",
    "hcnbody",
    "hcnactive",
    "hcnbodyActive",
    "hcndamage",
    "hcnbodyMiss",
    "hcnreactive",
    "hcnbodyReactive",
    "mine",
    "images",
    "nodes",
    "item",
];

/// Keys whose values are f32 in json_skin and should NOT be truncated.
/// All other float values in objects are truncated to integers (matching Java's toint() behavior).
const F32_FIELD_KEYS: &[&str] = &[
    "gain",
    "alpha",
    "outlineWidth",
    "shadowOffsetX",
    "shadowOffsetY",
    "shadowSmoothness",
];

/// Recursively walk a serde_json::Value tree and coerce types to match json_skin expectations.
/// - Numbers in STRING_FIELD_KEYS positions → strings (matches Java's toString() behavior)
/// - Floats in i32 positions → truncated to integers (matches Java's toint() behavior)
/// - Empty objects `{}` → removed (let #[serde(default)] handle both Vec and Option fields)
fn coerce_json_for_skin(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut new_map = serde_json::Map::new();
            for (key, val) in map {
                // Remove empty objects entirely — Lua empty tables `{}` can't deserialize
                // as Vec<T>; removing them lets #[serde(default)] provide Vec::new() or None.
                if let serde_json::Value::Object(ref inner) = val
                    && inner.is_empty()
                {
                    continue;
                }
                let coerced = coerce_value_for_key(&key, val);
                new_map.insert(key, coerced);
            }
            serde_json::Value::Object(new_map)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.into_iter().map(coerce_json_for_skin).collect())
        }
        other => other,
    }
}

fn coerce_value_for_key(key: &str, value: serde_json::Value) -> serde_json::Value {
    // Convert numbers to strings for known string-typed fields
    if STRING_FIELD_KEYS.contains(&key)
        && let serde_json::Value::Number(ref n) = value
    {
        return serde_json::Value::String(n.to_string());
    }
    // Convert empty objects to empty arrays for known Vec<String> fields
    if VEC_STRING_FIELD_KEYS.contains(&key)
        && let serde_json::Value::Object(ref map) = value
        && map.is_empty()
    {
        return serde_json::Value::Array(vec![]);
    }
    // Convert float-stored numbers to integers for i32 fields.
    // Lua arithmetic produces floats (e.g. 1920/2 = 960.0, 595/3 = 198.333...);
    // Java's toint() truncates them. serde_json can't deserialize f64 as i32.
    if let serde_json::Value::Number(ref n) = value
        && n.as_i64().is_none()
        && n.as_u64().is_none()
        && !F32_FIELD_KEYS.contains(&key)
        && let Some(f) = n.as_f64()
    {
        return serde_json::json!(f as i64);
    }
    // Recurse into nested structures
    coerce_json_for_skin(value)
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
                // Check if all integer keys form a pure sequence
                let mut has_string_key = false;
                let mut max_key = 0i64;
                for (key, _) in table.clone().pairs::<LuaValue, LuaValue>().flatten() {
                    match key {
                        LuaValue::Integer(i) => {
                            if i > max_key {
                                max_key = i;
                            }
                        }
                        _ => {
                            has_string_key = true;
                        }
                    }
                }
                if max_key == len as i64 {
                    // Sequential integer keys exist — extract as array.
                    // For mixed tables (e.g. {anim1, anim2, loop=300}), Java's
                    // fromLuaValue extracts only the array portion; named keys
                    // are ignored. This matches that behavior.
                    let mut arr = Vec::with_capacity(len);
                    for i in 1..=len {
                        let val: LuaValue = table.raw_get(i).unwrap_or(LuaValue::Nil);
                        arr.push(lua_to_json(&val).unwrap_or(serde_json::Value::Null));
                    }
                    if has_string_key {
                        log::debug!(
                            "lua_to_json: mixed table with {} sequential + string keys; extracting array",
                            len
                        );
                    }
                    return Some(serde_json::Value::Array(arr));
                }
            }

            // Object: string keys only (or non-sequential integer keys)
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
    use std::path::PathBuf;

    use crate::objects::wiring_check::{Severity, WiringCheck};
    use crate::skin_type::SkinType;
    use crate::test_helpers::MockMainState;
    use rubato_core::config::Config;

    fn repo_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(relative)
    }

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
            lua_to_json(&LuaValue::Number(2.75)),
            Some(serde_json::json!(2.75))
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

    #[test]
    fn test_load_default_decide_lua_skin_without_state() {
        let mut loader = LuaSkinLoader::new_without_state(&Config::default());
        let path = repo_path("skin/default/decide/decide.luaskin");

        let header = loader
            .load_header(&path)
            .expect("default decide Lua skin header should load");
        assert_eq!(header.skin_type, crate::skin_type::SkinType::Decide.id());

        let skin = loader.load(
            &path,
            &crate::skin_type::SkinType::Decide,
            &SkinConfigProperty,
        );
        assert!(
            skin.is_some(),
            "default decide Lua skin should load fully without MainState"
        );
    }

    #[test]
    fn test_load_default_play_lua_skin_without_state() {
        let mut loader = LuaSkinLoader::new_without_state(&Config::default());
        let path = repo_path("skin/default/play/play7.luaskin");

        let header = loader
            .load_header(&path)
            .expect("default play Lua skin header should load");
        assert_eq!(header.skin_type, crate::skin_type::SkinType::Play7Keys.id());

        let skin = loader.load(
            &path,
            &crate::skin_type::SkinType::Play7Keys,
            &SkinConfigProperty,
        );
        assert!(
            skin.is_some(),
            "default play Lua skin should load fully without MainState"
        );
    }

    #[test]
    fn test_new_with_state_exports_main_state_number_module() {
        let mut state = MockMainState::default();
        let loader = LuaSkinLoader::new_with_state(&mut state, &Config::default());
        let value = loader
            .lua
            .exec(
                "local loaded = package.loaded['main_state']; return type(loaded.number) .. ':' .. type(require('main_state').number)",
            )
            .expect("Lua should execute");
        match value {
            LuaValue::String(s) => assert_eq!(
                s.to_str().expect("Lua string should be valid UTF-8"),
                "function:function",
                "new_with_state should export main_state.number for result/play Lua skins"
            ),
            other => panic!("expected Lua string result, got {other:?}"),
        }
    }

    #[test]
    fn test_load_ecfn_result_lua_skin_with_state() {
        let mut state = MockMainState::default();
        let mut loader = LuaSkinLoader::new_with_state(&mut state, &Config::default());
        let path = repo_path("skin/ECFN/RESULT/result.luaskin");

        let header = loader
            .load_header(&path)
            .expect("ECFN result Lua skin header should load");
        assert_eq!(header.skin_type, crate::skin_type::SkinType::Result.id());

        let skin = loader.load(
            &path,
            &crate::skin_type::SkinType::Result,
            &SkinConfigProperty,
        );
        assert!(
            skin.is_some(),
            "ECFN result Lua skin should load with stateful main_state access"
        );
    }

    #[test]
    fn test_ecfn_play_lua_skin_wires_judge_images() {
        let mut loader = LuaSkinLoader::new_without_state(&Config::default());
        let path = repo_path("skin/ECFN/play/play7.luaskin");

        let header = loader
            .load_header(&path)
            .expect("ECFN play Lua skin header should load");
        let data = loader
            .load(&path, &SkinType::Play7Keys, &SkinConfigProperty)
            .expect("ECFN play Lua skin should load into SkinData");
        let resolved_judge = data
            .objects
            .iter()
            .find_map(|obj| obj.resolved_judge.as_ref())
            .expect("ECFN play SkinData should contain a resolved judge object");
        assert!(
            resolved_judge
                .judge_images()
                .iter()
                .any(|image| image.is_some()),
            "ECFN play SkinData should wire judge child images before conversion"
        );
        let skin = crate::skin_data_converter::convert_skin_data(
            &header,
            data,
            &mut loader.json_loader.source_map,
            &path,
            loader.json_loader.usecim,
            &loader.json_loader.dstr,
        )
        .expect("ECFN play Lua skin should convert into runtime Skin");

        let judge = skin
            .objects()
            .iter()
            .find_map(|obj| match obj {
                crate::skin::SkinObject::Judge(judge) => Some(judge),
                _ => None,
            })
            .expect("ECFN play skin should contain a judge object");
        let issues = judge.check_wiring();

        assert!(
            !issues.iter().any(|issue| issue.severity == Severity::Error),
            "ECFN judge object should have its images wired, issues={issues:?}"
        );
    }

    // -----------------------------------------------------------------------
    // 3-layer coercion tests (numbers->strings, float->int, empty {}->remove)
    // -----------------------------------------------------------------------

    #[test]
    fn test_coerce_numbers_to_strings_for_id_field() {
        // Layer 1: numbers in STRING_FIELD_KEYS positions -> strings
        let input = serde_json::json!({"id": 42, "src": 99, "font": 3});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["id"], serde_json::json!("42"));
        assert_eq!(coerced["src"], serde_json::json!("99"));
        assert_eq!(coerced["font"], serde_json::json!("3"));
    }

    #[test]
    fn test_coerce_numbers_to_strings_preserves_existing_strings() {
        let input = serde_json::json!({"id": "bg_image", "src": "texture.png"});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["id"], serde_json::json!("bg_image"));
        assert_eq!(coerced["src"], serde_json::json!("texture.png"));
    }

    #[test]
    fn test_coerce_float_to_int_truncation() {
        // Layer 2: floats in non-F32_FIELD_KEYS positions -> truncated to integers
        // Lua arithmetic produces floats like 1920/2 = 960.0
        let input = serde_json::json!({"x": 960.0, "y": 198.333, "w": 100.99});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["x"], serde_json::json!(960));
        assert_eq!(coerced["y"], serde_json::json!(198));
        assert_eq!(coerced["w"], serde_json::json!(100));
    }

    #[test]
    fn test_coerce_float_preserves_f32_fields() {
        // F32_FIELD_KEYS should NOT be truncated
        let input = serde_json::json!({
            "gain": 0.75,
            "alpha": 0.5,
            "outlineWidth": 1.5,
            "shadowOffsetX": 2.3,
            "shadowOffsetY": -1.7,
            "shadowSmoothness": 0.8
        });
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["gain"], serde_json::json!(0.75));
        assert_eq!(coerced["alpha"], serde_json::json!(0.5));
        assert_eq!(coerced["outlineWidth"], serde_json::json!(1.5));
    }

    #[test]
    fn test_coerce_empty_object_removed() {
        // Layer 3: empty objects {} -> removed (let serde(default) handle it)
        let input = serde_json::json!({
            "name": "test",
            "source": {},
            "destination": {}
        });
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["name"], serde_json::json!("test"));
        assert!(
            coerced.get("source").is_none(),
            "Empty object should be removed"
        );
        assert!(
            coerced.get("destination").is_none(),
            "Empty object should be removed"
        );
    }

    #[test]
    fn test_coerce_non_empty_object_preserved() {
        let input = serde_json::json!({"source": {"id": "bg"}});
        let coerced = coerce_json_for_skin(input);
        assert!(coerced.get("source").is_some());
        assert_eq!(coerced["source"]["id"], serde_json::json!("bg"));
    }

    #[test]
    fn test_coerce_empty_object_removed_for_vec_fields() {
        // Empty objects for VEC_STRING_FIELD_KEYS are removed (not converted to []).
        // The general empty-object removal (line 200-203) runs BEFORE coerce_value_for_key,
        // so the VEC_STRING_FIELD_KEYS -> [] path only handles non-empty maps.
        // With #[serde(default)], removing the key and providing [] are equivalent.
        let input = serde_json::json!({"note": {}, "mine": {}, "images": {}});
        let coerced = coerce_json_for_skin(input);
        assert!(
            coerced.get("note").is_none(),
            "Empty object for Vec fields should be removed"
        );
        assert!(
            coerced.get("mine").is_none(),
            "Empty object for Vec fields should be removed"
        );
        assert!(
            coerced.get("images").is_none(),
            "Empty object for Vec fields should be removed"
        );
    }

    #[test]
    fn test_coerce_recursion_into_nested_objects() {
        // Coercion should recurse into nested objects and arrays
        let input = serde_json::json!({
            "destination": [
                {"x": 100.0, "y": 200.0, "id": 5}
            ]
        });
        let coerced = coerce_json_for_skin(input);
        let dest = &coerced["destination"][0];
        assert_eq!(dest["x"], serde_json::json!(100));
        assert_eq!(dest["y"], serde_json::json!(200));
        assert_eq!(dest["id"], serde_json::json!("5"));
    }

    #[test]
    fn test_coerce_integer_values_not_truncated() {
        // Integer values (no decimal point) should pass through unchanged
        let input = serde_json::json!({"x": 100, "y": 200});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["x"], serde_json::json!(100));
        assert_eq!(coerced["y"], serde_json::json!(200));
    }

    #[test]
    fn test_coerce_path_number_to_string() {
        // "path" is in STRING_FIELD_KEYS, so numeric values should be coerced to strings
        let input = serde_json::json!({"path": 123});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["path"], serde_json::json!("123"));
    }

    #[test]
    fn test_coerce_constant_text_number_to_string() {
        // "constantText" is in STRING_FIELD_KEYS
        let input = serde_json::json!({"constantText": 42});
        let coerced = coerce_json_for_skin(input);
        assert_eq!(coerced["constantText"], serde_json::json!("42"));
    }
}
