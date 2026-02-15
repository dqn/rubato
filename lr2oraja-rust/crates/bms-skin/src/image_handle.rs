// Image handle abstraction for skin system.
//
// Actual GPU texture management is deferred to Phase 10 (Bevy).
// This module provides opaque handles and a loader trait.

use std::path::Path;

/// Opaque handle to a loaded image/texture.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct ImageHandle(pub u32);

impl ImageHandle {
    /// A sentinel value representing "no image".
    pub const NONE: Self = Self(u32::MAX);

    /// Embedded texture handle for judgedetail.png.
    /// 0xFFF0 (65520) is far above normal LR2 skin image indices (0-255).
    pub const EMBEDDED_JUDGEDETAIL: Self = Self(0xFFF0);

    /// Returns true if this is a valid handle (not NONE).
    pub fn is_valid(self) -> bool {
        self != Self::NONE
    }
}

/// A rectangular region within an image.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageRegion {
    pub handle: ImageHandle,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl Default for ImageRegion {
    fn default() -> Self {
        Self {
            handle: ImageHandle::NONE,
            x: 0.0,
            y: 0.0,
            w: 0.0,
            h: 0.0,
        }
    }
}

impl ImageRegion {
    pub fn full(handle: ImageHandle, width: f32, height: f32) -> Self {
        Self {
            handle,
            x: 0.0,
            y: 0.0,
            w: width,
            h: height,
        }
    }
}

/// Trait for loading images into handles.
///
/// Implemented by the rendering backend (Phase 10). During Phase 9 testing,
/// `StubImageLoader` provides ID-only allocation.
pub trait ImageLoader {
    /// Loads an image from a file path and returns a handle.
    fn load(&mut self, path: &Path) -> Option<ImageHandle>;

    /// Loads an image with color key transparency.
    ///
    /// The bottom-right pixel's RGB color is used as the transparent color:
    /// all pixels matching that color have their alpha set to 0.
    /// Used by PomyuChara (.chp) images.
    fn load_with_color_key(&mut self, path: &Path) -> Option<ImageHandle> {
        self.load(path)
    }

    /// Returns the dimensions (width, height) of a loaded image.
    fn dimensions(&self, handle: ImageHandle) -> Option<(f32, f32)>;
}

/// Stub image loader for testing. Assigns sequential IDs without loading real images.
#[derive(Debug, Default)]
pub struct StubImageLoader {
    next_id: u32,
}

impl ImageLoader for StubImageLoader {
    fn load(&mut self, _path: &Path) -> Option<ImageHandle> {
        let handle = ImageHandle(self.next_id);
        self.next_id += 1;
        Some(handle)
    }

    fn dimensions(&self, _handle: ImageHandle) -> Option<(f32, f32)> {
        // Stub returns a default size
        Some((256.0, 256.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_image_handle_none() {
        assert!(!ImageHandle::NONE.is_valid());
        assert!(ImageHandle(0).is_valid());
        assert!(ImageHandle(42).is_valid());
    }

    #[test]
    fn test_stub_loader() {
        let mut loader = StubImageLoader::default();
        let h1 = loader.load(Path::new("test1.png")).unwrap();
        let h2 = loader.load(Path::new("test2.png")).unwrap();
        assert_eq!(h1, ImageHandle(0));
        assert_eq!(h2, ImageHandle(1));
        assert_eq!(loader.dimensions(h1), Some((256.0, 256.0)));
    }

    #[test]
    fn test_image_region_full() {
        let r = ImageRegion::full(ImageHandle(0), 100.0, 200.0);
        assert_eq!(r.x, 0.0);
        assert_eq!(r.y, 0.0);
        assert_eq!(r.w, 100.0);
        assert_eq!(r.h, 200.0);
    }
}
