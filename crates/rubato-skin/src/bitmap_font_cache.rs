use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

use crate::reexports::{BitmapFont, BitmapFontData, TextureRegion};
use rubato_types::sync_utils::lock_or_recover;

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
        let store = lock_or_recover(&CACHE_STORE);
        store.contains_key(path)
    } else {
        false
    }
}

pub fn set(path: PathBuf, font: CacheableBitmapFont) {
    let mut store = lock_or_recover(&CACHE_STORE);
    store.insert(path, font);
}

pub fn get(path: &PathBuf) -> Option<CacheableBitmapFont> {
    let store = lock_or_recover(&CACHE_STORE);
    store.get(path).cloned()
}

/// Remove all cached bitmap fonts, releasing their texture pixel data.
///
/// Called during skin disposal to match Java's garbage collection behavior
/// where font cache entries become unreachable when the skin is disposed.
pub fn clear() {
    let mut store = lock_or_recover(&CACHE_STORE);
    store.clear();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clear_removes_all_cached_entries() {
        // Insert two entries into the global cache.
        let path_a = PathBuf::from("/tmp/test_font_a.fnt");
        let path_b = PathBuf::from("/tmp/test_font_b.fnt");
        set(path_a.clone(), CacheableBitmapFont::default());
        set(path_b.clone(), CacheableBitmapFont::default());

        assert!(has(Some(&path_a)));
        assert!(has(Some(&path_b)));

        // Clear must remove all entries.
        clear();

        assert!(!has(Some(&path_a)));
        assert!(!has(Some(&path_b)));
        assert!(get(&path_a).is_none());
        assert!(get(&path_b).is_none());
    }
}
