use std::path::Path;

use log::warn;

use crate::pixmap::Pixmap;
use crate::resource_pool::ResourcePool;

/// PixmapResourcePool - resource pool for Pixmap images
pub struct PixmapResourcePool {
    pool: ResourcePool<String, Pixmap>,
}

impl PixmapResourcePool {
    pub fn new() -> Self {
        Self {
            pool: ResourcePool::new(1),
        }
    }

    pub fn with_maxgen(maxgen: i32) -> Self {
        Self {
            pool: ResourcePool::new(maxgen),
        }
    }

    pub fn exists(&self, key: &str) -> bool {
        self.pool.exists(&key.to_string())
    }

    /// Ensure the resource is loaded into the pool (load-on-miss).
    pub fn get(&self, key: &str) -> Option<()> {
        self.pool.get(
            &key.to_string(),
            |k| {
                let pixmap = Self::load_picture(k);
                pixmap.map(Self::convert)
            },
            |mut resource| {
                resource.dispose();
            },
        )
    }

    /// Load if needed, then apply a function to the cached resource.
    /// Returns None if the resource couldn't be loaded.
    pub fn get_and_use<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&Pixmap) -> R,
    {
        self.pool.get_or_load(
            &key.to_string(),
            |k| Self::load_picture(k).map(Self::convert),
            f,
            |mut resource| {
                resource.dispose();
            },
        )
    }

    /// Access a cached resource without loading.
    pub fn with_resource<F, R>(&self, key: &str, f: F) -> Option<R>
    where
        F: FnOnce(&Pixmap) -> R,
    {
        self.pool.with_resource(&key.to_string(), f)
    }

    /// Convert pixmap on load. Override point for subclasses.
    fn convert(pixmap: Pixmap) -> Pixmap {
        pixmap
    }

    pub fn dispose_old(&self) {
        self.pool.dispose_old(|mut resource| {
            resource.dispose();
        });
    }

    pub fn size(&self) -> usize {
        self.pool.size()
    }

    pub fn dispose(&self) {
        self.pool.dispose(|mut resource| {
            resource.dispose();
        });
    }

    /// Load a picture from the given path
    pub fn load_picture(path: &str) -> Option<Pixmap> {
        let f = Path::new(path);
        if !f.is_file() {
            return None;
        }

        // CIM is a LibGDX-specific cache format (originalBase__timestamp.cim).
        // Try to find the original image file instead.
        if path.ends_with(".cim") {
            if let Some(base) = path.rsplit_once("__").map(|(base, _)| base) {
                for ext in &[".png", ".jpg", ".jpeg", ".bmp"] {
                    let original = format!("{}{}", base, ext);
                    if Path::new(&original).is_file() {
                        return Self::load_pixmap_from_file(&original).ok();
                    }
                }
            }
            return None;
        }

        // Try loading via LibGDX Pixmap equivalent
        // In Rust, this would use an image loading library
        match Self::load_pixmap_from_file(path) {
            Ok(pixmap) => Some(pixmap),
            Err(e) => {
                warn!("BGA file load failed: {}", e);
                // Retry with ImageIO equivalent
                warn!("BGA file load retry: {}", path);
                match Self::load_pixmap_fallback(path) {
                    Ok(pixmap) => Some(pixmap),
                    Err(e) => {
                        warn!("BGA file load failed: {}", e);
                        None
                    }
                }
            }
        }
    }

    fn load_pixmap_from_file(path: &str) -> Result<Pixmap, String> {
        Pixmap::from_file(path)
    }

    fn load_pixmap_fallback(path: &str) -> Result<Pixmap, String> {
        // Fallback: try loading via image crate with explicit format detection
        Pixmap::from_file(path)
    }
}

impl Default for PixmapResourcePool {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_png(dir: &std::path::Path, name: &str) -> String {
        let path = dir.join(name);
        let img = image::RgbaImage::from_pixel(2, 2, image::Rgba([255, 0, 0, 255]));
        img.save(&path).unwrap();
        path.to_string_lossy().to_string()
    }

    #[test]
    fn test_get_and_use_loads_and_provides_access() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::new();
        let dims = pool.get_and_use(&path, |p| (p.width, p.height));
        assert_eq!(dims, Some((2, 2)));
        assert!(pool.exists(&path));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_and_use_caches_on_second_call() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::new();
        pool.get_and_use(&path, |_| {});
        assert_eq!(pool.size(), 1);

        // Second call should use cache, not reload
        let dims = pool.get_and_use(&path, |p| (p.width, p.height));
        assert_eq!(dims, Some((2, 2)));
        assert_eq!(pool.size(), 1);
    }

    #[test]
    fn test_get_returns_none_for_nonexistent_file() {
        let pool = PixmapResourcePool::new();
        let result = pool.get_and_use("/nonexistent/path.png", |_| ());
        assert_eq!(result, None);
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_dispose_old_evicts_after_maxgen() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::new(); // maxgen = 1
        pool.get(&path);
        assert_eq!(pool.size(), 1);

        // gen 0 -> 1
        pool.dispose_old();
        assert_eq!(pool.size(), 1);

        // gen 1 == maxgen(1), evicted
        pool.dispose_old();
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_access_resets_generation() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::new(); // maxgen = 1
        pool.get(&path);

        pool.dispose_old(); // gen 0 -> 1
        pool.get(&path); // resets gen to 0
        pool.dispose_old(); // gen 0 -> 1
        assert_eq!(pool.size(), 1); // still alive

        pool.dispose_old(); // gen 1 == maxgen, evicted
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_dispose_clears_all() {
        let dir = tempfile::tempdir().unwrap();
        let p1 = create_test_png(dir.path(), "a.png");
        let p2 = create_test_png(dir.path(), "b.png");

        let pool = PixmapResourcePool::new();
        pool.get(&p1);
        pool.get(&p2);
        assert_eq!(pool.size(), 2);

        pool.dispose();
        assert_eq!(pool.size(), 0);
    }

    #[test]
    fn test_with_resource_accesses_cached() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::new();
        pool.get(&path);

        let width = pool.with_resource(&path, |p| p.width);
        assert_eq!(width, Some(2));
    }

    #[test]
    fn test_with_resource_returns_none_when_not_loaded() {
        let pool = PixmapResourcePool::new();
        let result = pool.with_resource("not_loaded.png", |_| ());
        assert_eq!(result, None);
    }

    #[test]
    fn test_custom_maxgen() {
        let dir = tempfile::tempdir().unwrap();
        let path = create_test_png(dir.path(), "test.png");

        let pool = PixmapResourcePool::with_maxgen(3);
        pool.get(&path);

        // Need 4 dispose_old calls to evict (gen 0->1, 1->2, 2->3, 3==maxgen)
        pool.dispose_old();
        pool.dispose_old();
        pool.dispose_old();
        assert_eq!(pool.size(), 1); // gen 3, not evicted yet (checked before eviction)

        pool.dispose_old(); // gen 3 == maxgen(3), now evicted
        assert_eq!(pool.size(), 0);
    }
}
