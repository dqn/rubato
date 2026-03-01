use beatoraja_render::color::Rectangle;
use beatoraja_render::texture::TextureRegion;

/// BGA skin object.
/// Translated from: SkinBGA.java
pub struct SkinBGA {
    bga_expand: i32,
    time: i64,
    /// Color RGBA (from SkinObject destination)
    pub color: (f32, f32, f32, f32),
    /// Blend mode (from SkinObject destination)
    pub blend: i32,
    /// Draw region (from SkinObject destination)
    pub region: Rectangle,
    /// Whether this object should be drawn (from SkinObject destination)
    pub draw: bool,
}

/// BGA expand modes (from Config)
pub const BGAEXPAND_FULL: i32 = 0;
pub const BGAEXPAND_KEEP_ASPECT_RATIO: i32 = 1;
pub const BGAEXPAND_OFF: i32 = 2;

/// BGA stretch types for aspect-ratio correction.
/// Subset of the full StretchType in beatoraja-skin, covering only the 3 modes
/// used by BGA expand config.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StretchType {
    Stretch,
    KeepAspectRatioFitInner,
    KeepAspectRatioNoExpanding,
}

impl StretchType {
    /// Modify the rectangle and image region to apply the stretch type.
    /// Translated from: Java StretchType.stretchRect(Rectangle, TextureRegion, TextureRegion)
    pub fn stretch_rect(&self, rectangle: &mut Rectangle, image: &mut TextureRegion) {
        match self {
            StretchType::Stretch => {
                // No modification — stretch to fill
            }
            StretchType::KeepAspectRatioFitInner => {
                let img_w = image.get_region_width() as f32;
                let img_h = image.get_region_height() as f32;
                if img_w > 0.0 && img_h > 0.0 {
                    let scale_x = rectangle.width / img_w;
                    let scale_y = rectangle.height / img_h;
                    if scale_x <= scale_y {
                        // Fit width, adjust height
                        let new_h = img_h * scale_x;
                        let cy = rectangle.y + rectangle.height * 0.5;
                        rectangle.height = new_h;
                        rectangle.y = cy - new_h * 0.5;
                    } else {
                        // Fit height, adjust width
                        let new_w = img_w * scale_y;
                        let cx = rectangle.x + rectangle.width * 0.5;
                        rectangle.width = new_w;
                        rectangle.x = cx - new_w * 0.5;
                    }
                }
            }
            StretchType::KeepAspectRatioNoExpanding => {
                let img_w = image.get_region_width() as f32;
                let img_h = image.get_region_height() as f32;
                if img_w > 0.0 && img_h > 0.0 {
                    let scale = 1.0f32.min((rectangle.width / img_w).min(rectangle.height / img_h));
                    let new_w = img_w * scale;
                    let new_h = img_h * scale;
                    let cx = rectangle.x + rectangle.width * 0.5;
                    let cy = rectangle.y + rectangle.height * 0.5;
                    rectangle.width = new_w;
                    rectangle.x = cx - new_w * 0.5;
                    rectangle.height = new_h;
                    rectangle.y = cy - new_h * 0.5;
                }
            }
        }
    }
}

impl SkinBGA {
    pub fn new(bga_expand: i32) -> Self {
        SkinBGA {
            bga_expand,
            time: 0,
            color: (1.0, 1.0, 1.0, 1.0),
            blend: 0,
            region: Rectangle::default(),
            draw: false,
        }
    }

    pub fn get_stretch_type(&self) -> StretchType {
        match self.bga_expand {
            BGAEXPAND_FULL => StretchType::Stretch,
            BGAEXPAND_KEEP_ASPECT_RATIO => StretchType::KeepAspectRatioFitInner,
            BGAEXPAND_OFF => StretchType::KeepAspectRatioNoExpanding,
            _ => StretchType::Stretch,
        }
    }

    /// Get the current time (set during prepare).
    pub fn get_time(&self) -> i64 {
        self.time
    }

    pub fn prepare(&mut self, time: i64) {
        self.time = time;
        // The caller (skin rendering system) is responsible for:
        // 1. Calling SkinObjectData.prepare() to set draw/region/color/blend
        // 2. Calling BGAProcessor.prepare_bga() with the play timer value
    }

    pub fn draw(&self) {
        // Drawing is handled by beatoraja_skin::skin_bga_object::SkinBgaObject.
        // The skin-level SkinBgaObject holds Arc<Mutex<dyn BgaDraw>> and implements
        // the full BGA rendering logic via BGAProcessor.draw_bga().
        // In practice mode, it delegates to PracticeConfiguration.draw().
        // This play-side struct exists for stretch type and time tracking only.
    }

    pub fn dispose(&mut self) {
        // no resources to dispose in Rust translation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stretch_type_from_expand_config() {
        assert_eq!(
            SkinBGA::new(BGAEXPAND_FULL).get_stretch_type(),
            StretchType::Stretch
        );
        assert_eq!(
            SkinBGA::new(BGAEXPAND_KEEP_ASPECT_RATIO).get_stretch_type(),
            StretchType::KeepAspectRatioFitInner
        );
        assert_eq!(
            SkinBGA::new(BGAEXPAND_OFF).get_stretch_type(),
            StretchType::KeepAspectRatioNoExpanding
        );
        // Invalid defaults to Stretch
        assert_eq!(SkinBGA::new(99).get_stretch_type(), StretchType::Stretch);
    }

    #[test]
    fn test_stretch_type_stretch_no_modification() {
        let mut rect = Rectangle::new(10.0, 20.0, 200.0, 150.0);
        let mut image = TextureRegion::from_texture(beatoraja_render::texture::Texture {
            width: 100,
            height: 100,
            disposed: false,
            ..Default::default()
        });
        let orig_rect = rect.clone();
        StretchType::Stretch.stretch_rect(&mut rect, &mut image);
        assert_eq!(rect.x, orig_rect.x);
        assert_eq!(rect.y, orig_rect.y);
        assert_eq!(rect.width, orig_rect.width);
        assert_eq!(rect.height, orig_rect.height);
    }

    #[test]
    fn test_stretch_type_keep_aspect_ratio_fit_inner_wider() {
        // Rectangle is wider than image aspect
        let mut rect = Rectangle::new(0.0, 0.0, 400.0, 200.0);
        let mut image = TextureRegion::from_texture(beatoraja_render::texture::Texture {
            width: 100,
            height: 100,
            disposed: false,
            ..Default::default()
        });
        StretchType::KeepAspectRatioFitInner.stretch_rect(&mut rect, &mut image);
        // Should fit to height (200), width becomes 200, centered
        assert!((rect.width - 200.0).abs() < 0.01);
        assert!((rect.height - 200.0).abs() < 0.01);
        assert!((rect.x - 100.0).abs() < 0.01); // centered: (400-200)/2 = 100
    }

    #[test]
    fn test_stretch_type_keep_aspect_ratio_fit_inner_taller() {
        // Rectangle is taller than image aspect
        let mut rect = Rectangle::new(0.0, 0.0, 200.0, 400.0);
        let mut image = TextureRegion::from_texture(beatoraja_render::texture::Texture {
            width: 100,
            height: 100,
            disposed: false,
            ..Default::default()
        });
        StretchType::KeepAspectRatioFitInner.stretch_rect(&mut rect, &mut image);
        // Should fit to width (200), height becomes 200, centered
        assert!((rect.width - 200.0).abs() < 0.01);
        assert!((rect.height - 200.0).abs() < 0.01);
        assert!((rect.y - 100.0).abs() < 0.01); // centered: (400-200)/2 = 100
    }

    #[test]
    fn test_stretch_type_no_expanding_small_image() {
        // Image is smaller than rectangle — should not expand
        let mut rect = Rectangle::new(0.0, 0.0, 400.0, 400.0);
        let mut image = TextureRegion::from_texture(beatoraja_render::texture::Texture {
            width: 100,
            height: 100,
            disposed: false,
            ..Default::default()
        });
        StretchType::KeepAspectRatioNoExpanding.stretch_rect(&mut rect, &mut image);
        // scale = min(1.0, min(4.0, 4.0)) = 1.0 — image stays at 100x100
        assert!((rect.width - 100.0).abs() < 0.01);
        assert!((rect.height - 100.0).abs() < 0.01);
        // centered in 400x400: (400-100)/2 = 150
        assert!((rect.x - 150.0).abs() < 0.01);
        assert!((rect.y - 150.0).abs() < 0.01);
    }

    #[test]
    fn test_stretch_type_no_expanding_large_image() {
        // Image is larger than rectangle — should scale down
        let mut rect = Rectangle::new(0.0, 0.0, 200.0, 100.0);
        let mut image = TextureRegion::from_texture(beatoraja_render::texture::Texture {
            width: 400,
            height: 400,
            disposed: false,
            ..Default::default()
        });
        StretchType::KeepAspectRatioNoExpanding.stretch_rect(&mut rect, &mut image);
        // scale = min(1.0, min(0.5, 0.25)) = 0.25
        // new_w = 400 * 0.25 = 100, new_h = 400 * 0.25 = 100
        assert!((rect.width - 100.0).abs() < 0.01);
        assert!((rect.height - 100.0).abs() < 0.01);
    }

    #[test]
    fn test_skin_bga_prepare_sets_time() {
        let mut bga = SkinBGA::new(BGAEXPAND_FULL);
        assert_eq!(bga.get_time(), 0);
        bga.prepare(12345);
        assert_eq!(bga.get_time(), 12345);
    }
}
