// SkinLoader.java -> skin_loader.rs
// Mechanical line-by-line translation.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::stubs::{MainState, Pixmap, PixmapFormat, Texture};

/// Skin image resource pool
/// Translated from SkinLoader.java
///
/// SkinLoader is abstract in Java with static methods.
/// In Rust, we translate static state as module-level functions with a global resource pool.
static RESOURCE: std::sync::LazyLock<std::sync::Mutex<Option<PixmapResourcePool>>> =
    std::sync::LazyLock::new(|| std::sync::Mutex::new(None));

/// Stub for PixmapResourcePool
pub struct PixmapResourcePool {
    generation: i32,
}

impl PixmapResourcePool {
    pub fn new(generation: i32) -> Self {
        Self { generation }
    }

    pub fn dispose(&mut self) {
        // stub
    }

    pub fn dispose_old(&mut self) {
        // stub
    }

    pub fn exists(&self, _path: &str) -> bool {
        false
    }

    pub fn get(&self, path: &str) -> Option<Pixmap> {
        match Pixmap::from_file(path) {
            Ok(pixmap) => Some(pixmap),
            Err(e) => {
                log::warn!("Failed to load image: {}", e);
                None
            }
        }
    }
}

pub fn init_pixmap_resource_pool(generation: i32) {
    let mut resource = RESOURCE.lock().unwrap();
    if let Some(ref mut r) = *resource {
        r.dispose();
    }
    *resource = Some(PixmapResourcePool::new(generation));
}

pub fn get_resource() -> std::sync::MutexGuard<'static, Option<PixmapResourcePool>> {
    let mut resource = RESOURCE.lock().unwrap();
    if resource.is_none() {
        *resource = Some(PixmapResourcePool::new(1));
    }
    resource
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
    _state: &dyn MainState,
    skin_type: &crate::skin_type::SkinType,
    skin_config_path: &str,
) -> Option<crate::json::json_skin_loader::SkinData> {
    let property = crate::json::json_skin_loader::SkinConfigProperty;

    if skin_config_path.ends_with(".json") {
        // JSONSkinLoader
        let config = _state.get_resource().get_config();
        let mut loader = crate::json::json_skin_loader::JSONSkinLoader::with_config(config);
        let result = loader.load_skin(Path::new(skin_config_path), skin_type, &property);
        // Dispose old resources after loading
        if let Ok(mut guard) = RESOURCE.lock()
            && let Some(ref mut r) = *guard
        {
            r.dispose_old();
        }
        result
    } else if skin_config_path.ends_with(".luaskin") {
        // LuaSkinLoader
        let config = _state.get_resource().get_config();
        let mut loader = crate::lua::lua_skin_loader::LuaSkinLoader::new_with_state(_state, config);
        let result = loader.load_skin(Path::new(skin_config_path), skin_type, &property);
        if let Ok(mut guard) = RESOURCE.lock()
            && let Some(ref mut r) = *guard
        {
            r.dispose_old();
        }
        result
    } else {
        // LR2SkinCSVLoader - not yet implemented
        log::warn!(
            "LR2 CSV skin loading not yet implemented for: {}",
            skin_config_path
        );
        None
    }
}

/// Resolves a file path with wildcard and file mapping support.
/// Corresponds to SkinLoader.getPath(String, ObjectMap<String, String>)
pub fn get_path(imagepath: &str, filemap: &HashMap<String, String>) -> PathBuf {
    let mut imagepath = imagepath.to_string();
    let mut imagefile = PathBuf::from(&imagepath);

    for (key, value) in filemap {
        if imagepath.starts_with(key.as_str()) {
            let foot = &imagepath[key.len()..];
            let last_star = imagepath.rfind('*').unwrap_or(0);
            imagefile = PathBuf::from(format!("{}{}{}", &imagepath[..last_star], value, foot));
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

        let last_slash = imagepath.rfind('/').unwrap_or(0);
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
pub fn get_texture(path: &str, usecim: bool) -> Option<Texture> {
    get_texture_with_mipmaps(path, usecim, false)
}

/// Gets a texture from a file path, with optional CIM cache and mipmaps.
/// Corresponds to SkinLoader.getTexture(String, boolean, boolean)
pub fn get_texture_with_mipmaps(path: &str, usecim: bool, use_mip_maps: bool) -> Option<Texture> {
    let resource_guard = get_resource();
    let resource = resource_guard.as_ref()?;

    if resource.exists(path)
        && let Some(pixmap) = resource.get(path)
    {
        return Some(Texture::from_pixmap_with_mipmaps(&pixmap, use_mip_maps));
    }

    // try { ... } catch (Throwable e) { ... }
    let modified_time = match std::fs::metadata(path) {
        Ok(meta) => match meta.modified() {
            Ok(t) => {
                t.duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs() as i64
                    / 1000
            }
            Err(_) => 0,
        },
        Err(_) => return None,
    };

    let last_dot = path.rfind('.').unwrap_or(path.len());
    let cim = format!("{}__{}.cim", &path[..last_dot], modified_time);

    if resource.exists(&cim)
        && let Some(pixmap) = resource.get(&cim)
    {
        return Some(Texture::from_pixmap_with_mipmaps(&pixmap, use_mip_maps));
    }

    let cim_path = Path::new(&cim);
    if cim_path.exists() {
        if let Some(pixmap) = resource.get(&cim) {
            return Some(Texture::from_pixmap_with_mipmaps(&pixmap, use_mip_maps));
        }
    } else if usecim {
        if let Some(pixmap) = resource.get(path) {
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

            // PixmapIO.writeCIM(Gdx.files.local(cim), pixmap);
            // CIM writing is a LibGDX-specific format, stubbed here

            return Some(Texture::from_pixmap_with_mipmaps(&pixmap, use_mip_maps));
        }
    } else if let Some(pixmap) = resource.get(path) {
        return Some(Texture::from_pixmap_with_mipmaps(&pixmap, use_mip_maps));
    }

    None
}
