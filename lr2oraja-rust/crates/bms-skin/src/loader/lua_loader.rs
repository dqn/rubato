// Lua skin loader.
//
// Executes Lua scripts that produce a skin data table, converts the result
// to JSON, then delegates to the JSON skin loader for actual skin building.
//
// This matches the Java architecture where LuaSkinLoader extends JSONSkinLoader
// and converts Lua tables to JsonSkin.Skin objects.
//
// The Lua environment provides:
// - `skin_config` table with custom options, offsets, and file paths
// - `skin_property` table with property ID constants
// - Standard Lua libraries (math, string, table, etc.)
//
// Ported from LuaSkinLoader.java and SkinLuaAccessor.java.

use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::rc::Rc;

use anyhow::{Context, Result};
use mlua::prelude::*;
use serde_json::Value;

use bms_config::resolution::Resolution;
use bms_config::skin_config::Offset;

use crate::property_mapper;
use crate::skin::Skin;
use crate::skin_header::SkinHeader;

use super::json_loader;
use super::lua_event_utility;
use super::lua_state_provider::{LuaStateProvider, TIMER_OFF};
use super::lua_timer_utility;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Loads a SkinHeader from a Lua skin script.
///
/// `lua_source` is the Lua script content (UTF-8).
/// The script should return a table matching the beatoraja JSON skin format,
/// or a `{header, main}` table where `header` contains the skin metadata.
pub fn load_lua_header(lua_source: &str, path: Option<&Path>) -> Result<SkinHeader> {
    let lua = create_lua_env(path, None)?;
    let value = exec_lua(&lua, lua_source, path)?;
    let resolved = resolve_for_header(&value);
    let json = lua_value_to_json(&resolved);
    let json_str =
        serde_json::to_string(&json).context("Failed to serialize Lua result to JSON")?;
    json_loader::load_header(&json_str)
}

/// Converts a Lua skin script to a JSON string.
///
/// Executes the Lua script and converts the resulting table to a JSON string.
/// Handles the `{header, main}` pattern by calling `main()` to get the skin data.
/// This is useful for tools that need the intermediate JSON representation
/// (e.g., screenshot harness that reuses JSON image loading paths).
///
/// If `state_provider` is `Some`, `main_state` is backed by the Rust provider
/// (with `timer_util` and `event_util` also registered). If `None`, the
/// existing Lua stub is used for backward compatibility.
pub fn lua_to_json_string(
    lua_source: &str,
    path: Option<&Path>,
    enabled_options: &HashSet<i32>,
    offsets: &[(i32, Offset)],
    state_provider: Option<Rc<RefCell<dyn LuaStateProvider>>>,
) -> Result<String> {
    let lua = create_lua_env(path, state_provider)?;
    let header_probe = exec_lua(&lua, lua_source, path)?;
    let option_selection = extract_option_selection(&header_probe, enabled_options)?;
    export_skin_config(&lua, enabled_options, offsets)?;
    apply_option_selection(&lua, &option_selection)?;
    let value = exec_lua(&lua, lua_source, path)?;
    let resolved = resolve_for_skin(&lua, &value)?;
    let json = lua_value_to_json(&resolved);
    // Lua division always produces floats (e.g. 595/3 = 198.333).
    // The JSON skin schema uses i32 for coordinates, so truncate floats
    // to integers to match Java's LuaSkinLoader truncation behavior.
    let json = truncate_floats_to_ints(json);
    serde_json::to_string(&json).context("Failed to serialize Lua result to JSON")
}

/// Loads a full Skin from a Lua skin script.
///
/// `lua_source` is the Lua script content (UTF-8).
/// `enabled_options`: set of enabled option IDs (from user's skin config).
/// `dest_resolution`: the display resolution to scale to.
/// `offsets`: custom offset values from user's skin config.
///
/// If `state_provider` is `Some`, `main_state` is backed by the Rust provider
/// (with `timer_util` and `event_util` also registered). If `None`, the
/// existing Lua stub is used for backward compatibility.
pub fn load_lua_skin(
    lua_source: &str,
    enabled_options: &HashSet<i32>,
    dest_resolution: Resolution,
    path: Option<&Path>,
    offsets: &[(i32, Offset)],
    state_provider: Option<Rc<RefCell<dyn LuaStateProvider>>>,
) -> Result<Skin> {
    let lua = create_lua_env(path, state_provider)?;
    let header_probe = exec_lua(&lua, lua_source, path)?;
    let option_selection = extract_option_selection(&header_probe, enabled_options)?;

    // Export skin_config with options and offsets
    export_skin_config(&lua, enabled_options, offsets)?;
    apply_option_selection(&lua, &option_selection)?;

    let value = exec_lua(&lua, lua_source, path)?;
    let resolved = resolve_for_skin(&lua, &value)?;
    let json = lua_value_to_json(&resolved);
    let json = truncate_floats_to_ints(json);
    let json_str =
        serde_json::to_string(&json).context("Failed to serialize Lua result to JSON")?;
    json_loader::load_skin(&json_str, enabled_options, dest_resolution, path)
}

// ---------------------------------------------------------------------------
// Lua environment setup
// ---------------------------------------------------------------------------

/// Creates a new Lua VM with standard libraries and the skin module path.
///
/// Sets up `package.path` for the script's directory and registers
/// `main_state` module. If `state_provider` is `Some`, the module is
/// backed by the Rust provider with full game state access, and
/// `timer_util` / `event_util` are also registered. If `None`, a pure-Lua
/// stub returning default values is used (backward compatible).
fn create_lua_env(
    path: Option<&Path>,
    state_provider: Option<Rc<RefCell<dyn LuaStateProvider>>>,
) -> Result<Lua> {
    let lua = Lua::new();

    // Add the script's directory to the Lua package path
    if let Some(p) = path
        && let Some(dir) = p.parent()
    {
        let dir_str = dir.to_string_lossy();
        lua.load(format!("package.path = package.path .. ';{dir_str}/?.lua'"))
            .exec()
            .map_err(|e| anyhow::anyhow!("Failed to set Lua package path: {}", e))?;
    }

    if let Some(provider) = state_provider {
        // Register Rust-backed main_state with real provider
        register_main_state(&lua, provider)?;
    } else {
        // Register main_state stub module via package.preload.
        // This is checked before file searchers, matching how beatoraja Java
        // provides main_state programmatically via SkinLuaAccessor.
        lua.load(
            r#"
package.preload["main_state"] = function()
    local M = {}
    M.timer_off_value = -9223372036854775808
    function M.number(_) return 0 end
    function M.option(_) return false end
    function M.text(_) return "" end
    function M.timer(_) return M.timer_off_value end
    function M.float_number(_) return 0.0 end
    function M.slider(_) return 0.0 end
    return M
end
"#,
        )
        .exec()
        .map_err(|e| anyhow::anyhow!("Failed to register main_state stub: {}", e))?;
    }

    Ok(lua)
}

/// Registers a Rust-backed `main_state` module in the Lua environment.
///
/// Captures an `Rc<RefCell<dyn LuaStateProvider>>` and exposes all methods
/// as Lua functions. Also registers `timer_util` and `event_util` modules
/// with `main_state.time()` as the clock source.
fn register_main_state(lua: &Lua, provider: Rc<RefCell<dyn LuaStateProvider>>) -> Result<()> {
    let ms = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create main_state table: {}", e))?;

    // timer_off_value constant
    ms.set("timer_off_value", TIMER_OFF)
        .map_err(|e| anyhow::anyhow!("Failed to set timer_off_value: {}", e))?;

    // Read-only state queries
    {
        let p = provider.clone();
        ms.set(
            "option",
            lua.create_function(move |_, id: i32| Ok(p.borrow().option(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "number",
            lua.create_function(move |_, id: i32| Ok(p.borrow().number(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "float_number",
            lua.create_function(move |_, id: i32| Ok(p.borrow().float_number(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "text",
            lua.create_function(move |_, id: i32| Ok(p.borrow().text(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "timer",
            lua.create_function(move |_, id: i32| Ok(p.borrow().timer(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "time",
            lua.create_function(move |_, ()| Ok(p.borrow().time()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "slider",
            lua.create_function(move |_, id: i32| Ok(p.borrow().slider(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "offset",
            lua.create_function(move |lua, id: i32| {
                let off = p.borrow().offset(id);
                let t = lua.create_table()?;
                t.set("x", off.x)?;
                t.set("y", off.y)?;
                t.set("w", off.w)?;
                t.set("h", off.h)?;
                t.set("r", off.r)?;
                t.set("a", off.a)?;
                Ok(t)
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Concrete accessors
    {
        let p = provider.clone();
        ms.set(
            "rate",
            lua.create_function(move |_, ()| Ok(p.borrow().rate()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "exscore",
            lua.create_function(move |_, ()| Ok(p.borrow().exscore()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "rate_best",
            lua.create_function(move |_, ()| Ok(p.borrow().rate_best()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "exscore_best",
            lua.create_function(move |_, ()| Ok(p.borrow().exscore_best()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "rate_rival",
            lua.create_function(move |_, ()| Ok(p.borrow().rate_rival()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "exscore_rival",
            lua.create_function(move |_, ()| Ok(p.borrow().exscore_rival()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Volume accessors
    {
        let p = provider.clone();
        ms.set(
            "volume_sys",
            lua.create_function(move |_, ()| Ok(p.borrow().volume_sys()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "volume_key",
            lua.create_function(move |_, ()| Ok(p.borrow().volume_key()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "volume_bg",
            lua.create_function(move |_, ()| Ok(p.borrow().volume_bg()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Judge / gauge
    {
        let p = provider.clone();
        ms.set(
            "judge",
            lua.create_function(move |_, id: i32| Ok(p.borrow().judge(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "gauge",
            lua.create_function(move |_, ()| Ok(p.borrow().gauge()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "gauge_type",
            lua.create_function(move |_, ()| Ok(p.borrow().gauge_type()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "event_index",
            lua.create_function(move |_, id: i32| Ok(p.borrow().event_index(id)))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Writable: set_timer (only custom timers)
    {
        let p = provider.clone();
        ms.set(
            "set_timer",
            lua.create_function(move |_, (id, value): (i32, i64)| {
                if property_mapper::is_timer_writable_by_skin(id) {
                    p.borrow_mut().set_timer(id, value);
                }
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Writable: volume setters
    {
        let p = provider.clone();
        ms.set(
            "set_volume_sys",
            lua.create_function(move |_, value: f32| {
                p.borrow_mut().set_volume_sys(value);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "set_volume_key",
            lua.create_function(move |_, value: f32| {
                p.borrow_mut().set_volume_key(value);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "set_volume_bg",
            lua.create_function(move |_, value: f32| {
                p.borrow_mut().set_volume_bg(value);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Writable: event_exec (variadic args, 0-2 integers)
    {
        let p = provider.clone();
        ms.set(
            "event_exec",
            lua.create_function(move |_, (id, args): (i32, mlua::Variadic<i32>)| {
                if property_mapper::is_event_runnable_by_skin(id) {
                    let args_vec: Vec<i32> = args.into_iter().collect();
                    p.borrow_mut().event_exec(id, &args_vec);
                }
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Audio control (matches Java MainStateAccessor signatures)
    {
        let p = provider.clone();
        ms.set(
            "audio_play",
            lua.create_function(move |_, (path, volume): (String, f32)| {
                p.borrow_mut().audio_play(&path, volume);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "audio_loop",
            lua.create_function(move |_, (path, volume): (String, f32)| {
                p.borrow_mut().audio_loop(&path, volume);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }
    {
        let p = provider.clone();
        ms.set(
            "audio_stop",
            lua.create_function(move |_, path: String| {
                p.borrow_mut().audio_stop(&path);
                Ok(())
            })
            .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    }

    // Register main_state via package.preload
    let ms_clone = ms.clone();
    lua.load("package.preload['main_state'] = ...")
        .into_function()
        .ok(); // discard — we set it manually below
    let preload = lua
        .globals()
        .get::<mlua::Table>("package")
        .and_then(|pkg| pkg.get::<mlua::Table>("preload"))
        .map_err(|e| anyhow::anyhow!("Failed to access package.preload: {}", e))?;
    preload
        .set(
            "main_state",
            lua.create_function(move |_, ()| Ok(ms_clone.clone()))
                .map_err(|e| anyhow::anyhow!("{e}"))?,
        )
        .map_err(|e| anyhow::anyhow!("Failed to register main_state: {}", e))?;

    // Register timer_util using main_state.time() as clock source
    let timer_util_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create timer_util table: {}", e))?;
    {
        let p = provider.clone();
        let get_now = lua
            .create_function(move |_, ()| Ok(p.borrow().time()))
            .map_err(|e| anyhow::anyhow!("{e}"))?;
        lua_timer_utility::register_timer_utilities(lua, &timer_util_table, get_now)?;
    }
    lua.globals()
        .set("timer_util", timer_util_table)
        .map_err(|e| anyhow::anyhow!("Failed to set timer_util global: {}", e))?;

    // Register event_util
    let event_util_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create event_util table: {}", e))?;
    lua_event_utility::register_event_utilities(lua, &event_util_table)?;
    lua.globals()
        .set("event_util", event_util_table)
        .map_err(|e| anyhow::anyhow!("Failed to set event_util global: {}", e))?;

    Ok(())
}

/// Exports the `skin_config` global table to the Lua environment.
///
/// The table contains:
/// - `option`: table mapping option names to selected indices
/// - `offset`: table mapping offset IDs to {x, y, w, h, r, a}
/// - `enabled_options`: array of enabled option IDs
fn export_skin_config(
    lua: &Lua,
    enabled_options: &HashSet<i32>,
    offsets: &[(i32, Offset)],
) -> Result<()> {
    let config = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create skin_config: {}", e))?;

    // Enabled options as array
    let opt_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create table: {}", e))?;
    for (i, &id) in enabled_options.iter().enumerate() {
        opt_table
            .set(i + 1, id)
            .map_err(|e| anyhow::anyhow!("Failed to set option: {}", e))?;
    }
    config
        .set("enabled_options", opt_table)
        .map_err(|e| anyhow::anyhow!("Failed to set enabled_options: {}", e))?;

    // Option table: maps property names to selected option IDs.
    // In beatoraja Java, SkinLuaAccessor populates this from the user's
    // skin configuration. Empty table allows skins to access it without errors.
    let option_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create table: {}", e))?;
    config
        .set("option", option_table)
        .map_err(|e| anyhow::anyhow!("Failed to set option: {}", e))?;

    // Offsets
    let offset_table = lua
        .create_table()
        .map_err(|e| anyhow::anyhow!("Failed to create table: {}", e))?;
    for &(id, ref off) in offsets {
        let ot = lua
            .create_table()
            .map_err(|e| anyhow::anyhow!("Failed to create table: {}", e))?;
        ot.set("x", off.x)
            .map_err(|e| anyhow::anyhow!("Failed to set x: {}", e))?;
        ot.set("y", off.y)
            .map_err(|e| anyhow::anyhow!("Failed to set y: {}", e))?;
        ot.set("w", off.w)
            .map_err(|e| anyhow::anyhow!("Failed to set w: {}", e))?;
        ot.set("h", off.h)
            .map_err(|e| anyhow::anyhow!("Failed to set h: {}", e))?;
        ot.set("r", off.r)
            .map_err(|e| anyhow::anyhow!("Failed to set r: {}", e))?;
        ot.set("a", off.a)
            .map_err(|e| anyhow::anyhow!("Failed to set a: {}", e))?;
        offset_table
            .set(id, ot)
            .map_err(|e| anyhow::anyhow!("Failed to set offset: {}", e))?;
    }
    config
        .set("offset", offset_table)
        .map_err(|e| anyhow::anyhow!("Failed to set offset: {}", e))?;

    lua.globals()
        .set("skin_config", config)
        .map_err(|e| anyhow::anyhow!("Failed to set skin_config: {}", e))?;

    Ok(())
}

/// Executes a Lua script and returns the result value.
fn exec_lua(lua: &Lua, source: &str, path: Option<&Path>) -> Result<mlua::Value> {
    let chunk = if let Some(p) = path {
        lua.load(source).set_name(p.to_string_lossy())
    } else {
        lua.load(source).set_name("<lua skin>")
    };
    chunk
        .eval()
        .map_err(|e| anyhow::anyhow!("Lua execution failed: {}", e))
}

/// Resolves the `{header, main}` return pattern for skin loading.
///
/// If the Lua result is a table with a `main` function:
/// 1. Extracts default option values from `header.property`
/// 2. Populates `skin_config.option` with defaults (matching beatoraja behavior)
/// 3. Calls `main()` and returns the skin data table
///
/// Otherwise returns the original value.
fn resolve_for_skin(lua: &Lua, value: &mlua::Value) -> Result<mlua::Value> {
    if let mlua::Value::Table(t) = value
        && let Ok(main_fn) = t.get::<mlua::Function>("main")
    {
        // Before calling main(), populate skin_config.option with defaults
        // from header.property. This matches beatoraja where the launcher
        // pre-selects the first option of each property by default.
        populate_default_options(lua, t)?;

        return main_fn
            .call::<mlua::Value>(())
            .map_err(|e| anyhow::anyhow!("Failed to call skin main(): {}", e));
    }
    Ok(value.clone())
}

/// Extracts default options from `header.property` and sets them in `skin_config.option`.
///
/// Each property has a `name` and `item` array. The first item's `op` value
/// is used as the default, matching beatoraja's behavior when no user selection exists.
fn populate_default_options(lua: &Lua, result_table: &mlua::Table) -> Result<()> {
    let header = match result_table.get::<mlua::Value>("header") {
        Ok(mlua::Value::Table(h)) => h,
        _ => return Ok(()),
    };
    let property = match header.get::<mlua::Value>("property") {
        Ok(mlua::Value::Table(p)) => p,
        _ => return Ok(()),
    };

    let globals = lua.globals();
    let skin_config = match globals.get::<mlua::Value>("skin_config") {
        Ok(mlua::Value::Table(c)) => c,
        _ => return Ok(()),
    };
    let option_table = match skin_config.get::<mlua::Value>("option") {
        Ok(mlua::Value::Table(o)) => o,
        _ => return Ok(()),
    };

    // Iterate properties and set default (first item's op value)
    for pair in property.pairs::<mlua::Value, mlua::Value>() {
        let (_, prop) = pair.map_err(|e| anyhow::anyhow!("Failed to read property: {}", e))?;
        if let mlua::Value::Table(prop_table) = prop {
            let name = match prop_table.get::<mlua::Value>("name") {
                Ok(mlua::Value::String(s)) => s,
                _ => continue,
            };
            let items = match prop_table.get::<mlua::Value>("item") {
                Ok(mlua::Value::Table(i)) => i,
                _ => continue,
            };
            // First item's op value is the default
            if let Ok(mlua::Value::Table(first_item)) = items.get::<mlua::Value>(1)
                && let Ok(op) = first_item.get::<mlua::Value>("op")
                && matches!(
                    option_table.get::<mlua::Value>(name.clone()),
                    Ok(mlua::Value::Nil)
                )
            {
                option_table
                    .set(name, op)
                    .map_err(|e| anyhow::anyhow!("Failed to set option default: {}", e))?;
            }
        }
    }

    Ok(())
}

/// Extracts option selections from header property metadata.
///
/// For each option group, this selects:
/// 1. A matching ID from `enabled_options` if present.
/// 2. Otherwise the first item's `op` value as default.
fn extract_option_selection(
    value: &mlua::Value,
    enabled_options: &HashSet<i32>,
) -> Result<Vec<(String, i32)>> {
    let header = resolve_for_header(value);
    let header_table = match header {
        mlua::Value::Table(t) => t,
        _ => return Ok(Vec::new()),
    };
    let property = match header_table.get::<mlua::Value>("property") {
        Ok(mlua::Value::Table(p)) => p,
        _ => return Ok(Vec::new()),
    };

    let mut selections = Vec::new();

    for pair in property.pairs::<mlua::Value, mlua::Value>() {
        let (_, prop) = pair.map_err(|e| anyhow::anyhow!("Failed to read property: {}", e))?;
        let prop_table = match prop {
            mlua::Value::Table(t) => t,
            _ => continue,
        };
        let name = match prop_table.get::<mlua::Value>("name") {
            Ok(mlua::Value::String(s)) => s.to_str().ok().map(|v| v.to_string()),
            _ => None,
        };
        let Some(name) = name else {
            continue;
        };
        let items = match prop_table.get::<mlua::Value>("item") {
            Ok(mlua::Value::Table(t)) => t,
            _ => continue,
        };

        let mut first_op: Option<i32> = None;
        let mut selected_op: Option<i32> = None;

        for item in items.sequence_values::<mlua::Table>() {
            let item = item.map_err(|e| anyhow::anyhow!("Failed to read item: {}", e))?;
            let op = match item.get::<mlua::Value>("op") {
                Ok(mlua::Value::Integer(i)) => i32::try_from(i).ok(),
                Ok(mlua::Value::Number(n)) => Some(n as i32),
                _ => None,
            };
            let Some(op) = op else {
                continue;
            };
            if first_op.is_none() {
                first_op = Some(op);
            }
            if enabled_options.contains(&op) {
                selected_op = Some(op);
                break;
            }
        }

        if let Some(op) = selected_op.or(first_op) {
            selections.push((name, op));
        }
    }

    Ok(selections)
}

fn apply_option_selection(lua: &Lua, option_selection: &[(String, i32)]) -> Result<()> {
    if option_selection.is_empty() {
        return Ok(());
    }

    let globals = lua.globals();
    let skin_config = match globals.get::<mlua::Value>("skin_config") {
        Ok(mlua::Value::Table(c)) => c,
        _ => return Ok(()),
    };
    let option_table = match skin_config.get::<mlua::Value>("option") {
        Ok(mlua::Value::Table(o)) => o,
        _ => return Ok(()),
    };

    for (name, op) in option_selection {
        option_table
            .set(name.as_str(), *op)
            .map_err(|e| anyhow::anyhow!("Failed to set option selection: {}", e))?;
    }
    Ok(())
}

/// Resolves the `{header, main}` return pattern for header loading.
///
/// If the Lua result is a table with a `header` sub-table, returns that
/// sub-table. Otherwise returns the original value as-is.
fn resolve_for_header(value: &mlua::Value) -> mlua::Value {
    if let mlua::Value::Table(t) = value
        && let Ok(header @ mlua::Value::Table(_)) = t.get::<mlua::Value>("header")
    {
        return header;
    }
    value.clone()
}

// ---------------------------------------------------------------------------
// Lua value → JSON conversion
// ---------------------------------------------------------------------------

/// Recursively converts a Lua value to a serde_json Value.
///
/// Lua tables are detected as either arrays (consecutive integer keys from 1)
/// or objects (string keys). Mixed tables are treated as objects with string
/// keys only (numeric keys are converted to strings).
fn lua_value_to_json(value: &mlua::Value) -> Value {
    match value {
        mlua::Value::Nil => Value::Null,
        mlua::Value::Boolean(b) => Value::Bool(*b),
        mlua::Value::Integer(n) => Value::Number(serde_json::Number::from(*n)),
        mlua::Value::Number(n) => {
            serde_json::Number::from_f64(*n).map_or(Value::Null, Value::Number)
        }
        mlua::Value::String(s) => {
            let str_result = s.to_str();
            match str_result {
                Ok(str_ref) => Value::String(str_ref.to_string()),
                Err(_) => Value::String(String::new()),
            }
        }
        mlua::Value::Table(t) => lua_table_to_json(t),
        // Lua functions → sentinel string so PropertyRef deserializes as Script.
        // This preserves the "draw field is present" semantics: in Java, a Lua
        // function in dst.draw becomes a BooleanProperty, preventing op from
        // being used as option_conditions. Without this sentinel, the function
        // would become null/None and op would incorrectly take over.
        mlua::Value::Function(_) => Value::String("__lua_function__".to_string()),
        _ => Value::Null, // userdata, thread, etc. → null
    }
}

/// Converts a Lua table to a JSON value (array or object).
///
/// Tables with consecutive integer keys 1..n are treated as arrays, even if
/// extra string keys are present (mixed tables). This matches Java's libGDX
/// Json deserializer behavior, where array elements are read by index and
/// stray string keys (e.g., `loop` accidentally placed inside a `dst` array)
/// are silently ignored.
///
/// Empty tables without string keys are treated as empty arrays.
/// Tables with only string keys are treated as objects.
fn lua_table_to_json(table: &mlua::Table) -> Value {
    let len = table.raw_len() as i64;

    // Sequence detection: if raw_len > 0 and all integer keys 1..n exist,
    // treat as array regardless of extra string keys (mixed table tolerance).
    if len > 0 && has_sequence_keys(table, len) {
        let mut arr = Vec::with_capacity(len as usize);
        for i in 1..=len {
            if let Ok(val) = table.raw_get::<mlua::Value>(i) {
                arr.push(lua_value_to_json(&val));
            }
        }
        return Value::Array(arr);
    }

    // Empty table → treat as array by default (common Lua/JSON convention)
    // This matches beatoraja's behavior where empty {} tables are used as empty arrays
    if len == 0 {
        // Check if there are any string keys
        let mut has_string_keys = false;
        for (key, _) in table.clone().pairs::<mlua::Value, mlua::Value>().flatten() {
            if matches!(key, mlua::Value::String(_)) {
                has_string_keys = true;
                break;
            }
        }
        if !has_string_keys {
            return Value::Array(Vec::new());
        }
    }

    // Object/map table
    let mut map = serde_json::Map::new();
    for (key, val) in table.clone().pairs::<mlua::Value, mlua::Value>().flatten() {
        let key_str = match &key {
            mlua::Value::String(s) => match s.to_str() {
                Ok(str_ref) => str_ref.to_string(),
                Err(_) => continue,
            },
            mlua::Value::Integer(n) => n.to_string(),
            mlua::Value::Number(n) => n.to_string(),
            _ => continue,
        };
        map.insert(key_str, lua_value_to_json(&val));
    }
    Value::Object(map)
}

/// Recursively truncates float values to integers in a JSON value.
///
/// In Lua, division always produces floats (e.g., `595/3 = 198.333`).
/// The beatoraja JSON skin schema uses `i32` for coordinates and sizes.
/// Java's LuaSkinLoader truncates these via `Coercions.toint()`.
/// This function replicates that behavior.
fn truncate_floats_to_ints(value: Value) -> Value {
    match value {
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                // If the float is representable as i64, truncate it
                let truncated = f as i64;
                if (truncated as f64 - f).abs() < 1.0 {
                    return Value::Number(serde_json::Number::from(truncated));
                }
            }
            Value::Number(n)
        }
        Value::Array(arr) => Value::Array(arr.into_iter().map(truncate_floats_to_ints).collect()),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(k, v)| (k, truncate_floats_to_ints(v)))
                .collect(),
        ),
        other => other,
    }
}

/// Checks if a Lua table has consecutive integer keys 1..n.
///
/// Unlike the previous `is_sequence`, this does NOT require that the table
/// has no extra string keys. Mixed tables (array + string keys) are common
/// in beatoraja Lua skins where a stray field like `loop` is placed inside
/// a `dst` array table by mistake. Java's libGDX handles this gracefully by
/// reading only the array portion; we replicate that here.
fn has_sequence_keys(table: &mlua::Table, len: i64) -> bool {
    for i in 1..=len {
        if table.raw_get::<mlua::Value>(i).is_err() {
            return false;
        }
    }
    true
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- Lua → JSON conversion --

    #[test]
    fn test_nil_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("nil").eval().unwrap();
        assert_eq!(lua_value_to_json(&val), Value::Null);
    }

    #[test]
    fn test_boolean_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("true").eval().unwrap();
        assert_eq!(lua_value_to_json(&val), Value::Bool(true));
    }

    #[test]
    fn test_integer_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("42").eval().unwrap();
        assert_eq!(lua_value_to_json(&val), serde_json::json!(42));
    }

    #[test]
    fn test_float_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("3.14").eval().unwrap();
        let json = lua_value_to_json(&val);
        assert!(json.is_number());
    }

    #[test]
    fn test_string_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("'hello'").eval().unwrap();
        assert_eq!(lua_value_to_json(&val), Value::String("hello".to_string()));
    }

    #[test]
    fn test_array_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("{10, 20, 30}").eval().unwrap();
        assert_eq!(lua_value_to_json(&val), serde_json::json!([10, 20, 30]));
    }

    #[test]
    fn test_object_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("{name = 'Test', type = 6}").eval().unwrap();
        let json = lua_value_to_json(&val);
        assert_eq!(json["name"], "Test");
        assert_eq!(json["type"], 6);
    }

    #[test]
    fn test_nested_table_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua
            .load("{items = {1, 2, 3}, meta = {x = 10}}")
            .eval()
            .unwrap();
        let json = lua_value_to_json(&val);
        assert_eq!(json["items"], serde_json::json!([1, 2, 3]));
        assert_eq!(json["meta"]["x"], 10);
    }

    #[test]
    fn test_function_to_sentinel() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("function() return 1 end").eval().unwrap();
        assert_eq!(
            lua_value_to_json(&val),
            Value::String("__lua_function__".to_string())
        );
    }

    // -- Lua skin loading --

    #[test]
    fn test_load_lua_header() {
        let lua_src = r#"
return {
    type = 6,
    name = "Lua Test Skin",
    author = "Test Author",
    w = 1280,
    h = 720,
    property = {},
    filepath = {},
    offset = {},
    destination = {}
}
"#;
        let header = load_lua_header(lua_src, None).unwrap();
        assert_eq!(header.name, "Lua Test Skin");
        assert_eq!(header.author, "Test Author");
    }

    #[test]
    fn test_load_lua_skin() {
        let lua_src = r#"
return {
    type = 6,
    name = "Lua Skin",
    w = 1280,
    h = 720,
    fadeout = 500,
    scene = 5000,
    image = {
        {id = "bg", src = "0", x = 0, y = 0, w = 1280, h = 720}
    },
    destination = {
        {id = "bg", dst = {{x = 0, y = 0, w = 1280, h = 720}}}
    }
}
"#;
        let skin =
            load_lua_skin(lua_src, &HashSet::new(), Resolution::Hd, None, &[], None).unwrap();
        assert_eq!(skin.fadeout, 500);
        assert_eq!(skin.scene, 5000);
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_load_lua_with_computation() {
        // Lua can compute values dynamically
        let lua_src = r#"
local w = 1280
local h = 720
return {
    type = 6,
    name = "Computed Skin",
    w = w,
    h = h,
    fadeout = math.floor(w / 2),
    destination = {}
}
"#;
        let skin =
            load_lua_skin(lua_src, &HashSet::new(), Resolution::Hd, None, &[], None).unwrap();
        assert_eq!(skin.fadeout, 640);
    }

    #[test]
    fn test_load_lua_with_options() {
        // Skin uses skin_config to check enabled options
        let lua_src = r#"
local opts = skin_config and skin_config.enabled_options or {}
local show_bg = false
for _, v in ipairs(opts) do
    if v == 901 then show_bg = true end
end

local dsts = {}
if show_bg then
    table.insert(dsts, {id = "bg", dst = {{x = 0, y = 0, w = 1280, h = 720}}})
end

return {
    type = 6,
    name = "Opt Skin",
    w = 1280,
    h = 720,
    image = {{id = "bg", src = "0"}},
    destination = dsts
}
"#;
        // Without option 901
        let skin =
            load_lua_skin(lua_src, &HashSet::new(), Resolution::Hd, None, &[], None).unwrap();
        assert_eq!(skin.object_count(), 0);

        // With option 901
        let skin = load_lua_skin(
            lua_src,
            &HashSet::from([901]),
            Resolution::Hd,
            None,
            &[],
            None,
        )
        .unwrap();
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_lua_error_reporting() {
        let lua_src = "this is not valid lua!!!";
        let result = load_lua_header(lua_src, None);
        assert!(result.is_err());
    }

    #[test]
    fn test_skin_config_offsets() {
        let lua_src = r#"
local off = skin_config and skin_config.offset or {}
local x_off = off[10] and off[10].x or 0

return {
    type = 6,
    name = "Offset Skin",
    w = 1280,
    h = 720,
    fadeout = math.floor(x_off),
    destination = {}
}
"#;
        let offsets = vec![(
            10,
            Offset {
                name: String::new(),
                x: 42,
                y: 0,
                w: 0,
                h: 0,
                r: 0,
                a: 0,
            },
        )];
        let skin = load_lua_skin(
            lua_src,
            &HashSet::new(),
            Resolution::Hd,
            None,
            &offsets,
            None,
        )
        .unwrap();
        assert_eq!(skin.fadeout, 42);
    }

    #[test]
    fn test_empty_table_to_json() {
        let lua = Lua::new();
        let val: mlua::Value = lua.load("{}").eval().unwrap();
        // Empty table should be an empty object (not array)
        let json = lua_value_to_json(&val);
        assert!(json.is_object() || json.is_array());
    }

    #[test]
    fn test_mixed_table_to_json() {
        let lua = Lua::new();
        // Lua table with both integer and string keys (common in beatoraja
        // skins, e.g., `dst = { {time=0}, {time=200}, loop=300 }`).
        // The integer-key portion is treated as an array; string keys are
        // silently discarded to match Java's libGDX behavior.
        let val: mlua::Value = lua
            .load("{[1] = 'a', [2] = 'b', name = 'test'}")
            .eval()
            .unwrap();
        let json = lua_value_to_json(&val);
        // Mixed table with sequence portion → treated as array
        assert!(json.is_array());
        assert_eq!(json, serde_json::json!(["a", "b"]));
    }

    // -- LuaStateProvider integration tests --

    use crate::loader::lua_state_provider::{LuaStateProvider, StubLuaStateProvider, TIMER_OFF};
    use crate::skin_object::SkinOffset;

    /// Shared mutation tracker for the mock provider.
    #[derive(Default)]
    struct MockMutations {
        last_set_timer: Option<(i32, i64)>,
        last_event_exec: Option<(i32, Vec<i32>)>,
    }

    /// Mock provider that returns specific values for testing.
    struct MockLuaStateProvider {
        number_val: i32,
        option_val: bool,
        timer_val: i64,
        time_val: i64,
        text_val: String,
        float_number_val: f64,
        slider_val: f64,
        offset_val: SkinOffset,
        rate_val: f64,
        exscore_val: i32,
        volume_sys_val: f32,
        judge_val: i32,
        gauge_val: f64,
        gauge_type_val: i32,
        mutations: Rc<RefCell<MockMutations>>,
    }

    impl MockLuaStateProvider {
        fn new_with_mutations() -> (Self, Rc<RefCell<MockMutations>>) {
            let mutations = Rc::new(RefCell::new(MockMutations::default()));
            let provider = Self {
                mutations: mutations.clone(),
                ..Default::default()
            };
            (provider, mutations)
        }
    }

    impl Default for MockLuaStateProvider {
        fn default() -> Self {
            Self {
                number_val: 0,
                option_val: false,
                timer_val: TIMER_OFF,
                time_val: 0,
                text_val: String::new(),
                float_number_val: 0.0,
                slider_val: 0.0,
                offset_val: SkinOffset::default(),
                rate_val: 0.0,
                exscore_val: 0,
                volume_sys_val: 0.0,
                judge_val: 0,
                gauge_val: 0.0,
                gauge_type_val: 0,
                mutations: Rc::new(RefCell::new(MockMutations::default())),
            }
        }
    }

    impl LuaStateProvider for MockLuaStateProvider {
        fn option(&self, _id: i32) -> bool {
            self.option_val
        }
        fn number(&self, _id: i32) -> i32 {
            self.number_val
        }
        fn float_number(&self, _id: i32) -> f64 {
            self.float_number_val
        }
        fn text(&self, _id: i32) -> String {
            self.text_val.clone()
        }
        fn timer(&self, _id: i32) -> i64 {
            self.timer_val
        }
        fn time(&self) -> i64 {
            self.time_val
        }
        fn slider(&self, _id: i32) -> f64 {
            self.slider_val
        }
        fn offset(&self, _id: i32) -> SkinOffset {
            self.offset_val
        }
        fn rate(&self) -> f64 {
            self.rate_val
        }
        fn exscore(&self) -> i32 {
            self.exscore_val
        }
        fn rate_best(&self) -> f64 {
            0.0
        }
        fn exscore_best(&self) -> i32 {
            0
        }
        fn rate_rival(&self) -> f64 {
            0.0
        }
        fn exscore_rival(&self) -> i32 {
            0
        }
        fn volume_sys(&self) -> f32 {
            self.volume_sys_val
        }
        fn volume_key(&self) -> f32 {
            0.0
        }
        fn volume_bg(&self) -> f32 {
            0.0
        }
        fn judge(&self, _id: i32) -> i32 {
            self.judge_val
        }
        fn gauge(&self) -> f64 {
            self.gauge_val
        }
        fn gauge_type(&self) -> i32 {
            self.gauge_type_val
        }
        fn event_index(&self, _id: i32) -> i32 {
            0
        }
        fn set_timer(&mut self, id: i32, value: i64) {
            self.mutations.borrow_mut().last_set_timer = Some((id, value));
        }
        fn set_volume_sys(&mut self, _value: f32) {}
        fn set_volume_key(&mut self, _value: f32) {}
        fn set_volume_bg(&mut self, _value: f32) {}
        fn event_exec(&mut self, id: i32, args: &[i32]) {
            self.mutations.borrow_mut().last_event_exec = Some((id, args.to_vec()));
        }
        fn audio_play(&mut self, _path: &str, _volume: f32) {}
        fn audio_loop(&mut self, _path: &str, _volume: f32) {}
        fn audio_stop(&mut self, _path: &str) {}
    }

    #[test]
    fn test_stub_provider_number_returns_zero() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();
        let result: i32 = lua
            .load("local ms = require('main_state'); return ms.number(42)")
            .eval()
            .unwrap();
        assert_eq!(result, 0);
    }

    #[test]
    fn test_stub_provider_option_returns_false() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();
        let result: bool = lua
            .load("local ms = require('main_state'); return ms.option(100)")
            .eval()
            .unwrap();
        assert!(!result);
    }

    #[test]
    fn test_stub_provider_timer_returns_off() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();
        let result: i64 = lua
            .load("local ms = require('main_state'); return ms.timer(10)")
            .eval()
            .unwrap();
        assert_eq!(result, TIMER_OFF);
    }

    #[test]
    fn test_custom_provider_returns_values() {
        let mock = MockLuaStateProvider {
            number_val: 42,
            option_val: true,
            text_val: "hello".to_string(),
            float_number_val: 314_f64 / 100.0,
            slider_val: 0.75,
            rate_val: 95.5,
            exscore_val: 1234,
            ..Default::default()
        };
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        let n: i32 = lua
            .load("local ms = require('main_state'); return ms.number(0)")
            .eval()
            .unwrap();
        assert_eq!(n, 42);

        let o: bool = lua
            .load("local ms = require('main_state'); return ms.option(0)")
            .eval()
            .unwrap();
        assert!(o);

        let t: String = lua
            .load("local ms = require('main_state'); return ms.text(0)")
            .eval()
            .unwrap();
        assert_eq!(t, "hello");

        let f: f64 = lua
            .load("local ms = require('main_state'); return ms.float_number(0)")
            .eval()
            .unwrap();
        assert!((f - (314_f64 / 100.0)).abs() < 0.001);

        let s: f64 = lua
            .load("local ms = require('main_state'); return ms.slider(0)")
            .eval()
            .unwrap();
        assert!((s - 0.75).abs() < 0.001);

        let r: f64 = lua
            .load("local ms = require('main_state'); return ms.rate()")
            .eval()
            .unwrap();
        assert!((r - 95.5).abs() < 0.001);

        let e: i32 = lua
            .load("local ms = require('main_state'); return ms.exscore()")
            .eval()
            .unwrap();
        assert_eq!(e, 1234);
    }

    #[test]
    fn test_timer_value_from_provider() {
        let mock = MockLuaStateProvider {
            timer_val: 5_000_000,
            ..Default::default()
        };
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        let v: i64 = lua
            .load("local ms = require('main_state'); return ms.timer(41)")
            .eval()
            .unwrap();
        assert_eq!(v, 5_000_000);
    }

    #[test]
    fn test_offset_returns_table() {
        let mock = MockLuaStateProvider {
            offset_val: SkinOffset {
                x: 10.0,
                y: 20.0,
                w: 30.0,
                h: 40.0,
                r: 50.0,
                a: 60.0,
            },
            ..Default::default()
        };
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        lua.load(
            r#"
            local ms = require('main_state')
            local off = ms.offset(0)
            assert(off.x == 10.0, "x=" .. off.x)
            assert(off.y == 20.0, "y=" .. off.y)
            assert(off.w == 30.0, "w=" .. off.w)
            assert(off.h == 40.0, "h=" .. off.h)
            assert(off.r == 50.0, "r=" .. off.r)
            assert(off.a == 60.0, "a=" .. off.a)
            "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn test_set_timer_custom_allowed() {
        let (mock, mutations) = MockLuaStateProvider::new_with_mutations();
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        // Custom timer ID 10000 should be writable
        lua.load("local ms = require('main_state'); ms.set_timer(10000, 999)")
            .exec()
            .unwrap();

        assert_eq!(mutations.borrow().last_set_timer, Some((10000, 999)));
    }

    #[test]
    fn test_set_timer_builtin_rejected() {
        let (mock, mutations) = MockLuaStateProvider::new_with_mutations();
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        // Built-in timer ID (e.g. TIMER_PLAY = 41) should NOT be writable
        lua.load("local ms = require('main_state'); ms.set_timer(41, 999)")
            .exec()
            .unwrap();

        assert_eq!(
            mutations.borrow().last_set_timer,
            None,
            "Built-in timer should not be set"
        );
    }

    #[test]
    fn test_timer_off_value_constant() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        let v: i64 = lua
            .load("local ms = require('main_state'); return ms.timer_off_value")
            .eval()
            .unwrap();
        assert_eq!(v, i64::MIN);
    }

    #[test]
    fn test_timer_util_registered_with_provider() {
        let mock = MockLuaStateProvider {
            time_val: 10_000_000,
            ..Default::default()
        };
        let provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        // timer_util should be available and now_timer should work
        let elapsed: i64 = lua
            .load("return timer_util.now_timer(5000000)")
            .eval()
            .unwrap();
        assert_eq!(elapsed, 5_000_000); // 10M - 5M

        let off_elapsed: i64 = lua
            .load(
                "local ms = require('main_state'); return timer_util.now_timer(ms.timer_off_value)",
            )
            .eval()
            .unwrap();
        assert_eq!(off_elapsed, 0);
    }

    #[test]
    fn test_event_util_registered_with_provider() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        // event_util should be available
        lua.load(
            r#"
            local counter = 0
            local state = false
            local observe = event_util.observe_turn_true(
                function() return state end,
                function() counter = counter + 1 end
            )
            observe()
            assert(counter == 0, "initial: expected 0, got " .. counter)
            state = true
            observe()
            assert(counter == 1, "after true: expected 1, got " .. counter)
            "#,
        )
        .exec()
        .unwrap();
    }

    #[test]
    fn test_skin_conditional_with_provider_option() {
        // Skin checks main_state.option() to decide structure
        let lua_src = r#"
local ms = require('main_state')
local dsts = {}
if ms.option(100) then
    table.insert(dsts, {id = "bg", dst = {{x = 0, y = 0, w = 1280, h = 720}}})
end
return {
    type = 6,
    name = "Provider Skin",
    w = 1280,
    h = 720,
    image = {{id = "bg", src = "0"}},
    destination = dsts
}
"#;
        // With StubLuaStateProvider: option returns false -> 0 objects
        let stub_provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let skin = load_lua_skin(
            lua_src,
            &HashSet::new(),
            Resolution::Hd,
            None,
            &[],
            Some(stub_provider),
        )
        .unwrap();
        assert_eq!(skin.object_count(), 0);

        // With MockLuaStateProvider(option=true): 1 object
        let mock = MockLuaStateProvider {
            option_val: true,
            ..Default::default()
        };
        let mock_provider: Rc<RefCell<dyn LuaStateProvider>> = Rc::new(RefCell::new(mock));
        let skin = load_lua_skin(
            lua_src,
            &HashSet::new(),
            Resolution::Hd,
            None,
            &[],
            Some(mock_provider),
        )
        .unwrap();
        assert_eq!(skin.object_count(), 1);
    }

    #[test]
    fn test_audio_functions_with_correct_signatures() {
        let provider: Rc<RefCell<dyn LuaStateProvider>> =
            Rc::new(RefCell::new(StubLuaStateProvider));
        let lua = create_lua_env(None, Some(provider)).unwrap();

        // audio_play(path, volume) should not error
        lua.load("local ms = require('main_state'); ms.audio_play('test.wav', 0.8)")
            .exec()
            .unwrap();

        // audio_loop(path, volume) should not error
        lua.load("local ms = require('main_state'); ms.audio_loop('bgm.ogg', 0.5)")
            .exec()
            .unwrap();

        // audio_stop(path) should not error
        lua.load("local ms = require('main_state'); ms.audio_stop('test.wav')")
            .exec()
            .unwrap();
    }
}
