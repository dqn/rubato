// SkinLoader.java -> skin_loader.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rubato_core::config::Config;
use rubato_core::pixmap_resource_pool::PixmapResourcePool;
use rubato_core::player_config::PlayerConfig;

use crate::reexports::{MainState, Texture};
use crate::types::skin::Skin;
use crate::types::skin_type::SkinType;
use rubato_types::sync_utils::lock_or_recover;

/// Skin image resource pool
/// Translated from SkinLoader.java
///
/// SkinLoader is abstract in Java with static methods.
/// In Rust, we translate static state as module-level functions with a global resource pool.
static RESOURCE: std::sync::LazyLock<std::sync::Mutex<Option<PixmapResourcePool>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

fn push_unique_path(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if !paths.iter().any(|existing| existing == &path) {
        paths.push(path);
    }
}

fn skin_path_candidates(config: &Config, skin_path: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    push_unique_path(&mut candidates, skin_path.to_path_buf());

    if !config.paths.skinpath.is_empty() {
        let skin_root = PathBuf::from(&config.paths.skinpath);
        push_unique_path(&mut candidates, skin_root.join(skin_path));

        if let Ok(stripped) = skin_path.strip_prefix("skin")
            && !stripped.as_os_str().is_empty()
        {
            push_unique_path(&mut candidates, skin_root.join(stripped));
        }
    }

    candidates
}

fn resolve_skin_path(config: &Config, skin_path: &str) -> Option<PathBuf> {
    let requested = PathBuf::from(skin_path);
    if requested.is_absolute() {
        return requested.exists().then_some(requested);
    }

    let candidates = skin_path_candidates(config, &requested);

    for candidate in &candidates {
        if candidate.exists() {
            return Some(candidate.clone());
        }
    }

    let cwd = std::env::current_dir().ok()?;
    for ancestor in cwd.ancestors() {
        for candidate in &candidates {
            if candidate.is_absolute() {
                continue;
            }
            let resolved = ancestor.join(candidate);
            if resolved.exists() {
                return Some(resolved);
            }
        }
    }

    None
}

pub fn init_pixmap_resource_pool(generation: i32) {
    let mut resource = lock_or_recover(&RESOURCE);
    if let Some(r) = resource.as_ref() {
        r.dispose();
    }
    *resource = Some(PixmapResourcePool::with_maxgen(generation));
}

pub fn get_resource() -> std::sync::MutexGuard<'static, Option<PixmapResourcePool>> {
    let mut resource = lock_or_recover(&RESOURCE);
    if resource.is_none() {
        *resource = Some(PixmapResourcePool::new());
    }
    resource
}

pub fn skin_path_from_player_config(
    player_config: &PlayerConfig,
    skin_type_id: i32,
) -> Option<String> {
    player_config
        .skin
        .get(skin_type_id as usize)
        .and_then(|sc| sc.as_ref())
        .and_then(|sc| sc.path.clone())
        .or_else(|| rubato_types::skin_config::SkinConfig::default_for_id(skin_type_id).path)
}

/// Copies user-configured offset values from PlayerConfig into the Skin's offset map.
///
/// Matching is by name: for each entry in `Skin.offset`, we look for a
/// `PlayerConfig.skin[type_id].properties.offset` entry with the same name
/// and copy its x/y/w/h/r/a values.
pub fn apply_player_config_offsets(
    skin: &mut Skin,
    player_config: &PlayerConfig,
    skin_type_id: i32,
) {
    let pc_offsets = player_config
        .skin
        .get(skin_type_id as usize)
        .and_then(|sc| sc.as_ref())
        .and_then(|sc| sc.properties.as_ref())
        .map(|props| &props.offset);

    let pc_offsets = match pc_offsets {
        Some(offsets) => offsets,
        None => return,
    };

    for cfg_offset in skin.offset_mut().values_mut() {
        for pc_offset in pc_offsets.iter().flatten() {
            if let Some(ref pc_name) = pc_offset.name
                && *pc_name == cfg_offset.name
            {
                cfg_offset.x = pc_offset.x as f32;
                cfg_offset.y = pc_offset.y as f32;
                cfg_offset.w = pc_offset.w as f32;
                cfg_offset.h = pc_offset.h as f32;
                cfg_offset.r = pc_offset.r as f32;
                cfg_offset.a = pc_offset.a as f32;
                cfg_offset.enabled = true;
                break;
            }
        }
    }
}

/// Loads a skin from config parameters without requiring a MainState reference.
///
/// Resolves the skin path from PlayerConfig (with fallback to SkinConfig default),
/// dispatches to the appropriate loader (JSON or Lua), and converts SkinData to Skin.
pub fn load_skin_from_config(
    config: &Config,
    player_config: &PlayerConfig,
    skin_type_id: i32,
) -> Option<Skin> {
    let skin_type = SkinType::skin_type_by_id(skin_type_id)?;

    log::debug!(
        "load_skin_from_config: type_id={}, skin_type={:?}, player_config.skin.len={}",
        skin_type_id,
        skin_type,
        player_config.skin.len()
    );

    // Resolve skin path: player_config.skin[id] → fallback to default
    let skin_path = match skin_path_from_player_config(player_config, skin_type_id) {
        Some(ref p) if !p.is_empty() => p.clone(),
        _ => {
            log::warn!(
                "No skin path configured for skin type {} ({:?})",
                skin_type_id,
                skin_type
            );
            return None;
        }
    };
    log::debug!("load_skin_from_config: skin_path={:?}", skin_path);

    let path = match resolve_skin_path(config, &skin_path) {
        Some(path) => path,
        None => {
            log::warn!(
                "Skin path {:?} could not be resolved (skin root {:?})",
                skin_path,
                config.paths.skinpath
            );
            return None;
        }
    };
    let property = crate::json::json_skin_loader::SkinConfigProperty;

    let mut skin = if skin_path.ends_with(".json") {
        let mut loader = crate::json::json_skin_loader::JSONSkinLoader::with_config(config);
        let header = loader.load_header(&path)?;
        let data = loader.load(&path, &skin_type, &property)?;
        let skin = crate::skin_data_converter::convert_skin_data(
            &header,
            data,
            &mut loader.source_map,
            &path,
            loader.usecim,
            &loader.dstr,
            &loader.filemap,
        );

        {
            let guard = lock_or_recover(&RESOURCE);
            if let Some(ref r) = *guard {
                r.dispose_old();
            }
        }

        skin
    } else if skin_path.ends_with(".luaskin") {
        let mut loader = crate::lua::lua_skin_loader::LuaSkinLoader::new_without_state(config);
        let header = loader.load_header(&path)?;
        let data = loader.load(&path, &skin_type, &property)?;
        let skin = crate::skin_data_converter::convert_skin_data(
            &header,
            data,
            &mut loader.json_loader.source_map,
            &path,
            loader.json_loader.usecim,
            &loader.json_loader.dstr,
            &loader.json_loader.filemap,
        );

        {
            let guard = lock_or_recover(&RESOURCE);
            if let Some(ref r) = *guard {
                r.dispose_old();
            }
        }

        skin
    } else {
        // LR2 CSV skin
        let dst = crate::reexports::Resolution {
            width: config.display.window_width as f32,
            height: config.display.window_height as f32,
        };
        let skin = crate::lr2::lr2_skin_csv_loader::load_lr2_skin(&path, &skin_type, dst);

        {
            let guard = lock_or_recover(&RESOURCE);
            if let Some(ref r) = *guard {
                r.dispose_old();
            }
        }

        skin
    }?;

    // Populate skin offset values from PlayerConfig
    apply_player_config_offsets(&mut skin, player_config, skin_type_id);

    Some(skin)
}

/// Loads a skin for a stateful caller using an explicit skin path.
///
/// Lua skins loaded through this path receive the live `main_state` accessor,
/// which result/select/decide Lua skins may require at load time.
pub fn load_skin_from_path_with_state(
    state: &mut dyn MainState,
    skin_type_id: i32,
    skin_path: &str,
) -> Option<Skin> {
    let skin_type = SkinType::skin_type_by_id(skin_type_id)?;
    let config = state
        .config_ref()
        .cloned()
        .expect("config required for skin loading");
    let path = resolve_skin_path(&config, skin_path)?;
    let property = crate::json::json_skin_loader::SkinConfigProperty;

    let mut skin = if skin_path.ends_with(".json") {
        let mut loader = crate::json::json_skin_loader::JSONSkinLoader::with_config(&config);
        let header = loader.load_header(&path)?;
        let data = loader.load(&path, &skin_type, &property)?;
        crate::skin_data_converter::convert_skin_data(
            &header,
            data,
            &mut loader.source_map,
            &path,
            loader.usecim,
            &loader.dstr,
            &loader.filemap,
        )
    } else if skin_path.ends_with(".luaskin") {
        let mut loader = crate::lua::lua_skin_loader::LuaSkinLoader::new_with_state(state, &config);
        let header = loader.load_header(&path)?;
        let data = loader.load(&path, &skin_type, &property)?;
        crate::skin_data_converter::convert_skin_data(
            &header,
            data,
            &mut loader.json_loader.source_map,
            &path,
            loader.json_loader.usecim,
            &loader.json_loader.dstr,
            &loader.json_loader.filemap,
        )
    } else {
        let dst = crate::reexports::Resolution {
            width: config.display.window_width as f32,
            height: config.display.window_height as f32,
        };
        crate::lr2::lr2_skin_csv_loader::load_lr2_skin(&path, &skin_type, dst)
    };

    {
        let guard = lock_or_recover(&RESOURCE);
        if let Some(ref r) = *guard {
            r.dispose_old();
        }
    }

    // Apply player-configured skin offsets (parity with load_skin_from_config).
    // Select, Result, and Decide states load skins through this path and their
    // user-configured offsets were previously silently ignored.
    if let Some(ref mut s) = skin
        && let Some(pc) = state.player_config_ref()
    {
        apply_player_config_offsets(s, pc, skin_type_id);
    }

    skin
}

/// Loads a skin for the given state and skin type.
/// Corresponds to SkinLoader.load(MainState, SkinType)
pub fn load(
    _state: &dyn MainState,
    skin_type: &crate::skin_type::SkinType,
) -> Option<crate::json::json_skin_loader::SkinData> {
    // In Java:
    // Skin skin = load(state, skinType, state.resource.getPlayerConfig().getSkin()[skinType.getId()]);
    // if(skin == null) { fallback to default }
    // return skin;
    log::warn!(
        "SkinLoader.load: requires SkinConfig from PlayerConfig (skin type {:?})",
        skin_type
    );
    None
}

/// Loads a skin with a specific skin config path.
/// Corresponds to SkinLoader.load(MainState, SkinType, SkinConfig)
///
/// Dispatches to JSONSkinLoader (.json), LuaSkinLoader (.luaskin), or LR2SkinCSVLoader.
pub fn load_with_config(
    _state: &mut dyn MainState,
    skin_type: &crate::skin_type::SkinType,
    skin_config_path: &str,
) -> Option<crate::json::json_skin_loader::SkinData> {
    let property = crate::json::json_skin_loader::SkinConfigProperty;

    if skin_config_path.ends_with(".json") {
        // JSONSkinLoader
        let config = _state
            .config_ref()
            .cloned()
            .expect("config required for skin loading");
        let mut loader = crate::json::json_skin_loader::JSONSkinLoader::with_config(&config);
        let result = loader.load_skin(Path::new(skin_config_path), skin_type, &property);
        // Dispose old resources after loading
        {
            let guard = lock_or_recover(&RESOURCE);
            if let Some(ref r) = *guard {
                r.dispose_old();
            }
        }
        result
    } else if skin_config_path.ends_with(".luaskin") {
        // LuaSkinLoader
        let config = _state
            .config_ref()
            .cloned()
            .expect("config required for skin loading");
        let mut loader =
            crate::lua::lua_skin_loader::LuaSkinLoader::new_with_state(_state, &config);
        let result = loader.load_skin(Path::new(skin_config_path), skin_type, &property);
        {
            let guard = lock_or_recover(&RESOURCE);
            if let Some(ref r) = *guard {
                r.dispose_old();
            }
        }
        result
    } else {
        // LR2 CSV produces Skin directly (not SkinData).
        // Use load_skin_from_config() or load_lr2_skin() instead.
        None
    }
}

/// Resolves a file path with wildcard and file mapping support.
/// Corresponds to SkinLoader.getPath(String, ObjectMap<String, String>)
pub fn path(imagepath: &str, filemap: &HashMap<String, String>) -> PathBuf {
    let mut imagepath = imagepath.to_string();
    let mut imagefile = PathBuf::from(&imagepath);

    for (key, value) in filemap {
        if imagepath.starts_with(key.as_str()) {
            let foot = &imagepath[key.len()..];
            imagefile = PathBuf::from(format!("{}{}", value, foot));
            imagepath = String::new();
            break;
        }
    }

    if imagepath.contains('*') {
        let last_star = imagepath.rfind('*').unwrap_or(0);
        let mut ext = imagepath[last_star + 1..].to_string();
        if imagepath.contains('|') {
            let pipe_pos = imagepath.rfind('|').unwrap_or(0);
            if imagepath.len() > pipe_pos + 1 {
                let star_to_pipe =
                    &imagepath[last_star + 1..imagepath.find('|').unwrap_or(imagepath.len())];
                ext = format!("{}{}", star_to_pipe, &imagepath[pipe_pos + 1..]);
            } else {
                ext = imagepath[last_star + 1..imagepath.find('|').unwrap_or(imagepath.len())]
                    .to_string();
            }
        }

        let last_slash = imagepath.rfind(['/', '\\']).unwrap_or(0);
        let imagedir = Path::new(&imagepath[..last_slash]);
        if imagedir.exists() && imagedir.is_dir() {
            let mut candidates: Vec<PathBuf> = Vec::new();
            if let Ok(entries) = std::fs::read_dir(imagedir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if let Some(path_str) = path.to_str()
                        && path_str.to_lowercase().ends_with(&ext.to_lowercase())
                    {
                        candidates.push(path);
                    }
                }
            }
            if !candidates.is_empty() {
                let idx = (rand::random::<f64>() * candidates.len() as f64) as usize;
                let idx = idx.min(candidates.len() - 1);
                imagefile = candidates[idx].clone();
            }
        }
    }

    imagefile
}

/// Gets a texture from a file path, optionally using CIM cache.
/// Corresponds to SkinLoader.getTexture(String, boolean)
pub fn texture(path: &str, usecim: bool) -> Option<Texture> {
    texture_with_mipmaps(path, usecim, false)
}

/// Gets a texture from a file path, with optional CIM cache and mipmaps.
/// Corresponds to SkinLoader.getTexture(String, boolean, boolean)
pub fn texture_with_mipmaps(path: &str, usecim: bool, use_mip_maps: bool) -> Option<Texture> {
    let resource_guard = get_resource();
    let resource = resource_guard.as_ref()?;

    // Cache hit — already loaded
    if resource.exists(path) {
        return resource.get_and_use(path, |pixmap| {
            Texture::from_pixmap_with_mipmaps(pixmap, use_mip_maps)
        });
    }

    // try { ... } catch (Throwable e) { ... }
    let modified_time = match std::fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(t) => t
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64,
            Err(_) => 0,
        },
        Err(_) => return None,
    };

    let last_dot = path.rfind('.').unwrap_or(path.len());
    let cim = format!("{}__{}.cim", &path[..last_dot], modified_time);

    // CIM cache hit
    if resource.exists(&cim) {
        return resource.get_and_use(&cim, |pixmap| {
            Texture::from_pixmap_with_mipmaps(pixmap, use_mip_maps)
        });
    }

    let cim_path = Path::new(&cim);
    if cim_path.exists() {
        resource.get_and_use(&cim, |pixmap| {
            Texture::from_pixmap_with_mipmaps(pixmap, use_mip_maps)
        })
    } else if usecim {
        let result = resource.get_and_use(path, |pixmap| {
            Texture::from_pixmap_with_mipmaps(pixmap, use_mip_maps)
        });

        // Delete old CIM files
        let parent = Path::new(path).parent();
        let prefix = format!("{}__", &path[..last_dot]);
        if let Some(parent) = parent
            && let Ok(entries) = std::fs::read_dir(parent)
        {
            for entry in entries.flatten() {
                let filename = entry.path().to_string_lossy().to_string();
                if filename.starts_with(&prefix) && filename.ends_with(".cim") {
                    let _ = std::fs::remove_file(entry.path());
                    break;
                }
            }
        }

        // CIM cache writing skipped: LibGDX-specific optimization not needed in Rust.
        // Existing CIM caches are still read (see cache hit logic above).

        result
    } else {
        resource.get_and_use(path, |pixmap| {
            Texture::from_pixmap_with_mipmaps(pixmap, use_mip_maps)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use rubato_core::player_config::PlayerConfig;
    use rubato_types::test_support::CurrentDirGuard;

    static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn load_skin_from_config_resolves_default_decide_skin_from_package_dir() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let package_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let _cwd = CurrentDirGuard::set(&package_dir);

        let skin = load_skin_from_config(
            &Config::default(),
            &PlayerConfig::default(),
            SkinType::Decide.id(),
        );

        assert!(
            skin.is_some(),
            "default decide skin should resolve even when the process starts in a crate directory"
        );
    }

    #[test]
    fn path_filemap_replaces_prefix_without_wildcard() {
        let mut filemap = HashMap::new();
        filemap.insert("theme/".to_string(), "/custom/".to_string());
        let result = path("theme/bg.png", &filemap);
        assert_eq!(result, PathBuf::from("/custom/bg.png"));
    }

    #[test]
    fn path_filemap_replaces_prefix_with_wildcard() {
        // Regression: previously the code duplicated the segment between key.len() and star_pos.
        let mut filemap = HashMap::new();
        filemap.insert("theme/".to_string(), "/custom/".to_string());
        let result = path("theme/bg*.png", &filemap);
        assert_eq!(result, PathBuf::from("/custom/bg*.png"));
    }

    #[test]
    fn path_filemap_wildcard_in_foot_preserved() {
        let mut filemap = HashMap::new();
        filemap.insert("images/".to_string(), "/replaced/".to_string());
        let result = path("images/sub/bg*.jpg", &filemap);
        assert_eq!(result, PathBuf::from("/replaced/sub/bg*.jpg"));
    }

    #[test]
    fn path_no_filemap_match_passes_through() {
        let filemap = HashMap::new();
        let result = path("other/file.png", &filemap);
        assert_eq!(result, PathBuf::from("other/file.png"));
    }

    #[test]
    fn path_wildcard_resolves_with_backslash_separator() {
        // LR2 skin files from Windows use backslash separators.
        // Previously rfind('/') missed backslashes, causing wildcard resolution
        // to silently fail (imagedir became "" instead of the actual directory).
        let tmp = std::env::temp_dir().join("rubato_test_backslash_wildcard");
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        std::fs::write(tmp.join("bg01.png"), b"").unwrap();

        let filemap = HashMap::new();
        let wildcard = format!("{}\\*.png", tmp.display());
        let result = path(&wildcard, &filemap);

        // With the fix, the backslash is recognized as a separator and the
        // directory is correctly extracted, allowing wildcard resolution to
        // find bg01.png. Without the fix, imagedir is "" and resolution fails.
        assert_eq!(result, tmp.join("bg01.png"));

        let _ = std::fs::remove_dir_all(&tmp);
    }

    // -- apply_player_config_offsets tests --

    fn make_skin_with_offset(id: i32, name: &str) -> Skin {
        use crate::skin_config_offset::SkinConfigOffset;
        use crate::skin_header::SkinHeader;

        let mut header = SkinHeader::new();
        header.set_skin_type(SkinType::Play7Keys);
        let mut skin = Skin::new(header);
        skin.offset.insert(
            id,
            SkinConfigOffset {
                name: name.to_string(),
                ..Default::default()
            },
        );
        skin
    }

    #[test]
    fn apply_player_config_offsets_copies_values_by_name() {
        use rubato_types::skin_config::{SkinConfig, SkinOffset, SkinProperty};

        let mut skin = make_skin_with_offset(crate::skin_property::OFFSET_ALL, "All offset(%)");

        let mut pc = PlayerConfig::default();
        // Ensure skin vec is large enough for skin_type_id 0
        while pc.skin.len() <= 0 {
            pc.skin.push(None);
        }
        pc.skin[0] = Some(SkinConfig {
            path: Some("test.json".to_string()),
            properties: Some(SkinProperty {
                option: vec![],
                file: vec![],
                offset: vec![Some(SkinOffset {
                    name: Some("All offset(%)".to_string()),
                    x: 5,
                    y: 10,
                    w: 20,
                    h: 15,
                    r: 0,
                    a: 0,
                })],
            }),
        });

        apply_player_config_offsets(&mut skin, &pc, 0);

        let cfg = skin
            .offset()
            .get(&crate::skin_property::OFFSET_ALL)
            .unwrap();
        assert_eq!(cfg.x, 5.0);
        assert_eq!(cfg.y, 10.0);
        assert_eq!(cfg.w, 20.0);
        assert_eq!(cfg.h, 15.0);
        assert!(cfg.enabled);
    }

    #[test]
    fn apply_player_config_offsets_no_match_leaves_defaults() {
        use rubato_types::skin_config::{SkinConfig, SkinOffset, SkinProperty};

        let mut skin = make_skin_with_offset(crate::skin_property::OFFSET_ALL, "All offset(%)");

        let mut pc = PlayerConfig::default();
        while pc.skin.len() <= 0 {
            pc.skin.push(None);
        }
        pc.skin[0] = Some(SkinConfig {
            path: Some("test.json".to_string()),
            properties: Some(SkinProperty {
                option: vec![],
                file: vec![],
                offset: vec![Some(SkinOffset {
                    name: Some("Different name".to_string()),
                    x: 99,
                    y: 99,
                    w: 99,
                    h: 99,
                    r: 0,
                    a: 0,
                })],
            }),
        });

        apply_player_config_offsets(&mut skin, &pc, 0);

        let cfg = skin
            .offset()
            .get(&crate::skin_property::OFFSET_ALL)
            .unwrap();
        assert_eq!(cfg.x, 0.0, "unmatched offset should remain at default");
        assert_eq!(cfg.y, 0.0);
    }

    #[test]
    fn apply_player_config_offsets_no_properties_is_noop() {
        let mut skin = make_skin_with_offset(crate::skin_property::OFFSET_ALL, "All offset(%)");
        let pc = PlayerConfig::default();

        apply_player_config_offsets(&mut skin, &pc, 0);

        let cfg = skin
            .offset()
            .get(&crate::skin_property::OFFSET_ALL)
            .unwrap();
        assert_eq!(cfg.x, 0.0, "no properties should leave offsets unchanged");
    }
}
