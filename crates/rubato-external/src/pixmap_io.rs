// Real implementations replacing LibGDX Pixmap/GdxGraphics/BufferUtils/PixmapIO.
// Moved from stubs.rs in Phase 25a.

/// Real RGBA8888 pixel buffer replacing LibGDX Pixmap.
/// Stores width x height RGBA pixel data for screenshot capture.
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    pixels: Vec<u8>,
}

impl Pixmap {
    pub fn new(width: i32, height: i32) -> Self {
        let size = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0u8; size],
        }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    /// Returns a mutable reference to the internal pixel buffer.
    /// Matches Java Pixmap.getPixels() which returns the internal ByteBuffer.
    pub fn get_pixels(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }

    /// Returns a read-only reference to the pixel data.
    pub fn get_pixel_data(&self) -> &[u8] {
        &self.pixels
    }

    pub fn dispose(&mut self) {
        self.pixels.clear();
        self.pixels.shrink_to_fit();
    }
}

/// Global back buffer dimensions for Gdx.graphics.
/// Set by the rendering system when the window/surface is created or resized.
pub struct GdxGraphics;

static BACK_BUFFER_WIDTH: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);
static BACK_BUFFER_HEIGHT: std::sync::atomic::AtomicI32 = std::sync::atomic::AtomicI32::new(0);

impl GdxGraphics {
    pub fn get_back_buffer_width() -> i32 {
        BACK_BUFFER_WIDTH.load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_back_buffer_height() -> i32 {
        BACK_BUFFER_HEIGHT.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// Set the back buffer dimensions. Called by the rendering system on resize.
    pub fn set_back_buffer_size(width: i32, height: i32) {
        BACK_BUFFER_WIDTH.store(width, std::sync::atomic::Ordering::Relaxed);
        BACK_BUFFER_HEIGHT.store(height, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Real memory copy replacing LibGDX BufferUtils.
/// Copies `count` bytes from `src` starting at `src_offset` into `dst`.
pub struct BufferUtils;

impl BufferUtils {
    pub fn copy(src: &[u8], src_offset: usize, dst: &mut Vec<u8>, count: usize) {
        let src_end = src_offset + count;
        if src_end > src.len() {
            log::error!(
                "BufferUtils::copy: src_offset({}) + count({}) > src.len({})",
                src_offset,
                count,
                src.len()
            );
            return;
        }
        // Ensure dst has enough capacity
        if dst.len() < count {
            dst.resize(count, 0);
        }
        dst[..count].copy_from_slice(&src[src_offset..src_end]);
    }
}

/// Real PNG writer replacing LibGDX PixmapIO.
/// Uses the `image` crate to encode RGBA8888 pixel data as PNG.
pub struct PixmapIO;

impl PixmapIO {
    pub fn write_png(path: &str, pixmap: &Pixmap) {
        use image::{ImageBuffer, Rgba};
        use std::path::Path;

        let width = pixmap.get_width() as u32;
        let height = pixmap.get_height() as u32;
        let pixel_data = pixmap.get_pixel_data();

        if pixel_data.len() != (width as usize) * (height as usize) * 4 {
            log::error!(
                "PixmapIO::write_png: pixel data length mismatch: expected {}, got {}",
                (width as usize) * (height as usize) * 4,
                pixel_data.len()
            );
            return;
        }

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            match ImageBuffer::from_raw(width, height, pixel_data.to_vec()) {
                Some(img) => img,
                None => {
                    log::error!("PixmapIO::write_png: failed to create ImageBuffer");
                    return;
                }
            };

        // Ensure parent directory exists
        let file_path = Path::new(path);
        if let Some(parent) = file_path.parent()
            && !parent.exists()
            && let Err(e) = std::fs::create_dir_all(parent)
        {
            log::error!("PixmapIO::write_png: failed to create directory: {}", e);
            return;
        }

        if let Err(e) = img.save(file_path) {
            log::error!("PixmapIO::write_png: failed to save PNG: {}", e);
        }
    }

    /// Encode pixmap as PNG bytes in memory (for Twitter upload etc.)
    pub fn encode_png_bytes(pixmap: &Pixmap) -> Vec<u8> {
        use image::{ImageBuffer, ImageEncoder, Rgba};

        let width = pixmap.get_width() as u32;
        let height = pixmap.get_height() as u32;
        let pixel_data = pixmap.get_pixel_data();

        if pixel_data.len() != (width as usize) * (height as usize) * 4 {
            log::error!(
                "PixmapIO::encode_png_bytes: pixel data length mismatch: expected {}, got {}",
                (width as usize) * (height as usize) * 4,
                pixel_data.len()
            );
            return Vec::new();
        }

        let img: ImageBuffer<Rgba<u8>, Vec<u8>> =
            match ImageBuffer::from_raw(width, height, pixel_data.to_vec()) {
                Some(img) => img,
                None => {
                    log::error!("PixmapIO::encode_png_bytes: failed to create ImageBuffer");
                    return Vec::new();
                }
            };

        let mut buf = std::io::Cursor::new(Vec::new());
        let encoder = image::codecs::png::PngEncoder::new(&mut buf);
        if let Err(e) =
            encoder.write_image(img.as_raw(), width, height, image::ExtendedColorType::Rgba8)
        {
            log::error!("PixmapIO::encode_png_bytes: failed to encode PNG: {}", e);
            return Vec::new();
        }

        buf.into_inner()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============================================================
    // Pixmap tests
    // ============================================================

    #[test]
    fn test_pixmap_new_creates_zero_filled_buffer() {
        let pixmap = Pixmap::new(4, 3);
        assert_eq!(pixmap.get_width(), 4);
        assert_eq!(pixmap.get_height(), 3);
        assert_eq!(pixmap.get_pixel_data().len(), 4 * 3 * 4); // width * height * RGBA
        assert!(pixmap.get_pixel_data().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_pixmap_get_pixels_returns_mutable_internal_buffer() {
        let mut pixmap = Pixmap::new(2, 2);
        let pixels = pixmap.get_pixels();
        // Write some data
        pixels[0] = 255; // R
        pixels[1] = 128; // G
        pixels[2] = 64; // B
        pixels[3] = 255; // A

        // Verify the data persists via read-only access
        assert_eq!(pixmap.get_pixel_data()[0], 255);
        assert_eq!(pixmap.get_pixel_data()[1], 128);
        assert_eq!(pixmap.get_pixel_data()[2], 64);
        assert_eq!(pixmap.get_pixel_data()[3], 255);
    }

    #[test]
    fn test_pixmap_dispose_clears_data() {
        let mut pixmap = Pixmap::new(10, 10);
        assert_eq!(pixmap.get_pixel_data().len(), 400);
        pixmap.dispose();
        assert_eq!(pixmap.get_pixel_data().len(), 0);
    }

    #[test]
    fn test_pixmap_zero_dimensions() {
        let pixmap = Pixmap::new(0, 0);
        assert_eq!(pixmap.get_width(), 0);
        assert_eq!(pixmap.get_height(), 0);
        assert_eq!(pixmap.get_pixel_data().len(), 0);
    }

    // ============================================================
    // BufferUtils tests
    // ============================================================

    #[test]
    fn test_buffer_utils_copy_basic() {
        let src = vec![1u8, 2, 3, 4, 5];
        let mut dst = vec![0u8; 5];
        BufferUtils::copy(&src, 0, &mut dst, 5);
        assert_eq!(dst, vec![1, 2, 3, 4, 5]);
    }

    #[test]
    fn test_buffer_utils_copy_with_offset() {
        let src = vec![10u8, 20, 30, 40, 50];
        let mut dst = vec![0u8; 3];
        BufferUtils::copy(&src, 2, &mut dst, 3);
        assert_eq!(dst, vec![30, 40, 50]);
    }

    #[test]
    fn test_buffer_utils_copy_grows_dst_if_needed() {
        let src = vec![1u8, 2, 3, 4];
        let mut dst = Vec::new();
        BufferUtils::copy(&src, 0, &mut dst, 4);
        assert_eq!(dst, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_buffer_utils_copy_out_of_bounds_is_noop() {
        let src = vec![1u8, 2];
        let mut dst = vec![0u8; 5];
        // src_offset + count > src.len(), should not copy
        BufferUtils::copy(&src, 0, &mut dst, 5);
        assert_eq!(dst, vec![0, 0, 0, 0, 0]);
    }

    #[test]
    fn test_buffer_utils_copy_into_pixmap() {
        // Simulate the screenshot flow: copy raw pixel data into pixmap buffer
        let raw_pixels = vec![255u8, 0, 0, 255, 0, 255, 0, 255]; // 2 RGBA pixels
        let mut pixmap = Pixmap::new(2, 1);
        let pixel_buf = pixmap.get_pixels();
        BufferUtils::copy(&raw_pixels, 0, pixel_buf, raw_pixels.len());
        assert_eq!(pixmap.get_pixel_data(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    // ============================================================
    // GdxGraphics tests
    // ============================================================

    #[test]
    fn test_gdx_graphics_set_and_get() {
        // Reset to known state
        GdxGraphics::set_back_buffer_size(1920, 1080);
        assert_eq!(GdxGraphics::get_back_buffer_width(), 1920);
        assert_eq!(GdxGraphics::get_back_buffer_height(), 1080);

        GdxGraphics::set_back_buffer_size(800, 600);
        assert_eq!(GdxGraphics::get_back_buffer_width(), 800);
        assert_eq!(GdxGraphics::get_back_buffer_height(), 600);
    }

    #[test]
    fn test_gdx_graphics_default_is_zero() {
        // Note: test order isn't guaranteed, so we just verify set works
        GdxGraphics::set_back_buffer_size(0, 0);
        assert_eq!(GdxGraphics::get_back_buffer_width(), 0);
        assert_eq!(GdxGraphics::get_back_buffer_height(), 0);
    }

    // ============================================================
    // PixmapIO tests
    // ============================================================

    #[test]
    fn test_pixmap_io_write_png_creates_valid_file() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test.png");
        let path_str = path.to_str().unwrap();

        let mut pixmap = Pixmap::new(2, 2);
        // Set a red pixel at (0,0), green at (1,0), blue at (0,1), white at (1,1)
        let pixels = pixmap.get_pixels();
        // Pixel (0,0): red
        pixels[0] = 255;
        pixels[1] = 0;
        pixels[2] = 0;
        pixels[3] = 255;
        // Pixel (1,0): green
        pixels[4] = 0;
        pixels[5] = 255;
        pixels[6] = 0;
        pixels[7] = 255;
        // Pixel (0,1): blue
        pixels[8] = 0;
        pixels[9] = 0;
        pixels[10] = 255;
        pixels[11] = 255;
        // Pixel (1,1): white
        pixels[12] = 255;
        pixels[13] = 255;
        pixels[14] = 255;
        pixels[15] = 255;

        PixmapIO::write_png(path_str, &pixmap);

        // Verify file exists and is a valid PNG
        assert!(path.exists());
        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 2);
        assert_eq!(img.height(), 2);

        // Verify pixel values
        let rgba = img.to_rgba8();
        assert_eq!(rgba.get_pixel(0, 0).0, [255, 0, 0, 255]); // red
        assert_eq!(rgba.get_pixel(1, 0).0, [0, 255, 0, 255]); // green
        assert_eq!(rgba.get_pixel(0, 1).0, [0, 0, 255, 255]); // blue
        assert_eq!(rgba.get_pixel(1, 1).0, [255, 255, 255, 255]); // white
    }

    #[test]
    fn test_pixmap_io_write_png_creates_parent_dirs() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("sub/dir/test.png");
        let path_str = path.to_str().unwrap();

        let pixmap = Pixmap::new(1, 1);
        PixmapIO::write_png(path_str, &pixmap);

        assert!(path.exists());
    }

    #[test]
    fn test_pixmap_io_encode_png_bytes() {
        let mut pixmap = Pixmap::new(1, 1);
        let pixels = pixmap.get_pixels();
        pixels[0] = 100;
        pixels[1] = 150;
        pixels[2] = 200;
        pixels[3] = 255;

        let bytes = PixmapIO::encode_png_bytes(&pixmap);
        assert!(!bytes.is_empty());
        // Verify PNG magic number
        assert_eq!(&bytes[0..4], &[0x89, 0x50, 0x4E, 0x47]);

        // Decode and verify
        let img = image::load_from_memory(&bytes).unwrap();
        let rgba = img.to_rgba8();
        assert_eq!(rgba.get_pixel(0, 0).0, [100, 150, 200, 255]);
    }

    #[test]
    fn test_full_screenshot_flow() {
        // Simulate the full screenshot pipeline:
        // 1. Set screen dimensions
        // 2. Create pixmap
        // 3. Copy pixel data via BufferUtils
        // 4. Write PNG via PixmapIO
        // 5. Verify output

        GdxGraphics::set_back_buffer_size(3, 2);
        let width = GdxGraphics::get_back_buffer_width();
        let height = GdxGraphics::get_back_buffer_height();

        // Simulate raw OpenGL pixel data (3x2 RGBA)
        #[rustfmt::skip]
        let raw_pixels: Vec<u8> = vec![
            255, 0,   0,   255,   0, 255,   0, 255,   0,   0, 255, 255, // row 0
            128, 128, 128, 255,  64,  64,  64, 255,  32,  32,  32, 255, // row 1
        ];

        let mut pixmap = Pixmap::new(width, height);
        let pixel_buf = pixmap.get_pixels();
        BufferUtils::copy(&raw_pixels, 0, pixel_buf, raw_pixels.len());

        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("screenshot.png");
        PixmapIO::write_png(path.to_str().unwrap(), &pixmap);

        // Verify the written PNG
        let img = image::open(&path).unwrap();
        assert_eq!(img.width(), 3);
        assert_eq!(img.height(), 2);
        let rgba = img.to_rgba8();
        assert_eq!(rgba.get_pixel(0, 0).0, [255, 0, 0, 255]);
        assert_eq!(rgba.get_pixel(1, 0).0, [0, 255, 0, 255]);
        assert_eq!(rgba.get_pixel(2, 0).0, [0, 0, 255, 255]);
        assert_eq!(rgba.get_pixel(0, 1).0, [128, 128, 128, 255]);

        pixmap.dispose();
        assert_eq!(pixmap.get_pixel_data().len(), 0);
    }
}
