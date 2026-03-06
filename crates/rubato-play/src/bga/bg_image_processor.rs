use std::path::Path;

use rubato_render::texture::Texture;

/// Image file extensions supported for BGA
pub static PIC_EXTENSION: &[&str] = &["jpg", "jpeg", "gif", "bmp", "png", "tga"];

/// BG image resource manager
pub struct BGImageProcessor {
    bgamap: Vec<Option<Texture>>,
    bgacache_ids: Vec<i32>,
    cache_size: usize,
}

impl BGImageProcessor {
    pub fn new(size: usize, _maxgen: i32) -> Self {
        BGImageProcessor {
            bgamap: vec![None; 1000],
            bgacache_ids: vec![-1; size],
            cache_size: size,
        }
    }

    pub fn put(&mut self, id: usize, path: &Path) {
        if id >= self.bgamap.len() {
            self.bgamap.resize_with(id + 1, || None);
        }
        let path_str = path.to_str().unwrap_or("");
        let tex = Texture::new(path_str);
        if tex.width > 0 && tex.height > 0 {
            self.bgamap[id] = Some(tex);
        } else {
            log::warn!("Failed to load BGA image: {}", path_str);
            self.bgamap[id] = None;
        }
    }

    pub fn clear(&mut self) {
        for item in self.bgamap.iter_mut() {
            *item = None;
        }
    }

    pub fn dispose_old(&mut self) {
        // Evict textures not in the active cache window.
        // Cache IDs track which BGA IDs are actively in use by the current timelines.
        // Textures outside this set can be released to save memory.
        for (id, slot) in self.bgamap.iter_mut().enumerate() {
            if slot.is_some() && !self.bgacache_ids.contains(&(id as i32)) {
                *slot = None;
            }
        }
    }

    pub fn prepare(&mut self, timelines: &[i32]) {
        // Pre-cache: mark the upcoming BGA IDs in the cache
        for id in self.bgacache_ids.iter_mut() {
            *id = -1;
        }
        for (i, &bga_id) in timelines.iter().enumerate() {
            if bga_id >= 0 && i < self.cache_size {
                self.bgacache_ids[i % self.cache_size] = bga_id;
            }
        }
    }

    /// Get the texture for the given BGA id, updating the cache.
    /// Returns a reference to the texture if it exists.
    pub fn texture(&mut self, id: usize) -> Option<&Texture> {
        let cid = id % self.cache_size;
        if self.bgacache_ids[cid] == id as i32 {
            return self.bgamap.get(id).and_then(|t| t.as_ref());
        }
        if id < self.bgamap.len() && self.bgamap[id].is_some() {
            self.bgacache_ids[cid] = id as i32;
            return self.bgamap.get(id).and_then(|t| t.as_ref());
        }
        None
    }

    /// Directly insert a texture at the given id (for testing).
    #[cfg(test)]
    pub fn put_texture(&mut self, id: usize, tex: Texture) {
        if id >= self.bgamap.len() {
            self.bgamap.resize_with(id + 1, || None);
        }
        self.bgamap[id] = Some(tex);
    }

    pub fn dispose(&mut self) {
        self.bgamap.clear();
        self.bgacache_ids.clear();
    }
}
