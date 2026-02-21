use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::stubs::{BitmapFont, BitmapFontData, TextureRegion};

/// BitmapFont cache
///
/// Translated from BitmapFontCache.java
static CACHE_STORE: std::sync::LazyLock<Mutex<HashMap<PathBuf, CacheableBitmapFont>>> =
    std::sync::LazyLock::new(|| Mutex::new(HashMap::new()));

#[derive(Clone, Debug, Default)]
pub struct CacheableBitmapFont {
    pub font_data: BitmapFontData,
    pub regions: Vec<TextureRegion>,
    pub font: BitmapFont,
    pub original_size: f32,
    pub type_: i32,
    pub page_width: f32,
    pub page_height: f32,
}

pub fn has(path: Option<&PathBuf>) -> bool {
    if let Some(path) = path {
        let store = CACHE_STORE.lock().unwrap();
        store.contains_key(path)
    } else {
        false
    }
}

pub fn set(path: PathBuf, font: CacheableBitmapFont) {
    let mut store = CACHE_STORE.lock().unwrap();
    store.insert(path, font);
}

pub fn get(path: &PathBuf) -> Option<CacheableBitmapFont> {
    let store = CACHE_STORE.lock().unwrap();
    store.get(path).cloned()
}
