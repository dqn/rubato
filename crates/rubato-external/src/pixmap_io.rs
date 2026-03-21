// Real implementations replacing LibGDX Pixmap/GdxGraphics/BufferUtils/PixmapIO.
//

/// Real RGBA8888 pixel buffer replacing LibGDX Pixmap.
/// Stores width x height RGBA pixel data for screenshot capture.
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
    pixels: Vec<u8>,
}

impl Pixmap {
    pub fn new(width: i32, height: i32) -> Self {
        let width = width.max(0);
        let height = height.max(0);
        let size = (width as usize) * (height as usize) * 4;
        Self {
            width,
            height,
            pixels: vec![0u8; size],
        }
    }

    /// Returns a mutable reference to the internal pixel buffer.
    /// Matches Java Pixmap.getPixels() which returns the internal ByteBuffer.
    pub fn pixels(&mut self) -> &mut Vec<u8> {
        &mut self.pixels
    }

    /// Returns a read-only reference to the pixel data.
    pub fn pixel_data(&self) -> &[u8] {
        &self.pixels
    }

    pub fn dispose(&mut self) {
        self.pixels.clear();
        self.pixels.shrink_to_fit();
    }
}

/// Global back buffer dimensions for Gdx.graphics.
/// Set by the rendering system when the window/surface is created or resized.
///
/// Width and height are packed into a single AtomicU64 to prevent torn reads
/// when a resize races with a consumer reading both dimensions.
pub struct GdxGraphics;

static BACK_BUFFER_SIZE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

impl GdxGraphics {
    /// Pack width and height into a single u64: upper 32 bits = width, lower 32 bits = height.
    fn pack_size(width: i32, height: i32) -> u64 {
        ((width as u32 as u64) << 32) | (height as u32 as u64)
    }

    /// Unpack a u64 into (width, height).
    fn unpack_size(packed: u64) -> (i32, i32) {
        let width = (packed >> 32) as u32 as i32;
        let height = packed as u32 as i32;
        (width, height)
    }

    /// Returns both back buffer dimensions atomically.
    pub fn back_buffer_size() -> (i32, i32) {
        Self::unpack_size(BACK_BUFFER_SIZE.load(std::sync::atomic::Ordering::Acquire))
    }

    pub fn back_buffer_width() -> i32 {
        Self::back_buffer_size().0
    }

    pub fn back_buffer_height() -> i32 {
        Self::back_buffer_size().1
    }

    /// Set the back buffer dimensions. Called by the rendering system on resize.
    pub fn set_back_buffer_size(width: i32, height: i32) {
        BACK_BUFFER_SIZE.store(
            Self::pack_size(width, height),
            std::sync::atomic::Ordering::Release,
        );
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
    /// Write a PNG file. Errors are logged but not propagated, matching the Java
    /// fire-and-forget screenshot pattern where the caller always shows a "saved"
    /// notification regardless of I/O outcome.
    pub fn write_png(path: &str, pixmap: &Pixmap) {
        use image::{ImageBuffer, Rgba};
        use std::path::Path;

        let width = pixmap.width as u32;
        let height = pixmap.height as u32;
        let pixel_data = pixmap.pixel_data();

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

        let width = pixmap.width as u32;
        let height = pixmap.height as u32;
        let pixel_data = pixmap.pixel_data();

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
        assert_eq!(pixmap.width, 4);
        assert_eq!(pixmap.height, 3);
        assert_eq!(pixmap.pixel_data().len(), 4 * 3 * 4); // width * height * RGBA
        assert!(pixmap.pixel_data().iter().all(|&b| b == 0));
    }

    #[test]
    fn test_pixmap_pixels_returns_mutable_internal_buffer() {
        let mut pixmap = Pixmap::new(2, 2);
        let pixels = pixmap.pixels();
        // Write some data
        pixels[0] = 255; // R
        pixels[1] = 128; // G
        pixels[2] = 64; // B
        pixels[3] = 255; // A

        // Verify the data persists via read-only access
        assert_eq!(pixmap.pixel_data()[0], 255);
        assert_eq!(pixmap.pixel_data()[1], 128);
        assert_eq!(pixmap.pixel_data()[2], 64);
        assert_eq!(pixmap.pixel_data()[3], 255);
    }

    #[test]
    fn test_pixmap_dispose_clears_data() {
        let mut pixmap = Pixmap::new(10, 10);
        assert_eq!(pixmap.pixel_data().len(), 400);
        pixmap.dispose();
        assert_eq!(pixmap.pixel_data().len(), 0);
    }

    #[test]
    fn test_pixmap_zero_dimensions() {
        let pixmap = Pixmap::new(0, 0);
        assert_eq!(pixmap.width, 0);
        assert_eq!(pixmap.height, 0);
        assert_eq!(pixmap.pixel_data().len(), 0);
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
        let pixel_buf = pixmap.pixels();
        BufferUtils::copy(&raw_pixels, 0, pixel_buf, raw_pixels.len());
        assert_eq!(pixmap.pixel_data(), &[255, 0, 0, 255, 0, 255, 0, 255]);
    }

    // ============================================================
    // GdxGraphics tests
    // ============================================================

    #[test]
    fn test_gdx_graphics_set_and_get() {
        // Reset to known state
        GdxGraphics::set_back_buffer_size(1920, 1080);
        assert_eq!(GdxGraphics::back_buffer_width(), 1920);
        assert_eq!(GdxGraphics::back_buffer_height(), 1080);

        GdxGraphics::set_back_buffer_size(800, 600);
        assert_eq!(GdxGraphics::back_buffer_width(), 800);
        assert_eq!(GdxGraphics::back_buffer_height(), 600);
    }

    #[test]
    fn test_gdx_graphics_default_is_zero() {
        // Note: test order isn't guaranteed, so we just verify set works
        GdxGraphics::set_back_buffer_size(0, 0);
        assert_eq!(GdxGraphics::back_buffer_width(), 0);
        assert_eq!(GdxGraphics::back_buffer_height(), 0);
    }

    #[test]
    fn test_gdx_graphics_back_buffer_size_returns_consistent_pair() {
        // Regression: width and height must be read atomically as a pair.
        // Previously they were stored in two separate AtomicI32 values,
        // allowing a resize between the two loads to produce mismatched dimensions.
        GdxGraphics::set_back_buffer_size(1920, 1080);
        let (w, h) = GdxGraphics::back_buffer_size();
        assert_eq!((w, h), (1920, 1080));

        GdxGraphics::set_back_buffer_size(3840, 2160);
        let (w, h) = GdxGraphics::back_buffer_size();
        assert_eq!((w, h), (3840, 2160));
    }

    #[test]
    fn test_gdx_graphics_pack_unpack_roundtrip() {
        // Verify pack/unpack helpers preserve values across the i32 range.
        let cases: &[(i32, i32)] = &[
            (0, 0),
            (1920, 1080),
            (3840, 2160),
            (i32::MAX, i32::MAX),
            (-1, -1),
            (i32::MIN, i32::MIN),
        ];
        for &(w, h) in cases {
            let packed = GdxGraphics::pack_size(w, h);
            let (uw, uh) = GdxGraphics::unpack_size(packed);
            assert_eq!((uw, uh), (w, h), "roundtrip failed for ({}, {})", w, h);
        }
    }

    #[test]
    fn test_gdx_graphics_packed_atomic_prevents_torn_reads() {
        // Stress test: a writer thread rapidly alternates between two packed
        // sizes on a local AtomicU64 while the reader thread checks that every
        // observed pair is one of the two valid combinations. With the old
        // two-AtomicI32 design, a torn read could produce (1920, 600) or
        // (800, 1080). Using a single AtomicU64 guarantees consistency.
        //
        // Uses a local AtomicU64 to avoid interfering with other tests that
        // read the global BACK_BUFFER_SIZE.
        use std::sync::Arc;
        use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

        let packed = Arc::new(AtomicU64::new(GdxGraphics::pack_size(800, 600)));
        let packed_w = Arc::clone(&packed);
        let stop = Arc::new(AtomicBool::new(false));
        let stop_w = Arc::clone(&stop);

        let size_a = GdxGraphics::pack_size(1920, 1080);
        let size_b = GdxGraphics::pack_size(800, 600);

        let writer = std::thread::spawn(move || {
            let mut toggle = false;
            while !stop_w.load(Ordering::Relaxed) {
                packed_w.store(if toggle { size_a } else { size_b }, Ordering::Relaxed);
                toggle = !toggle;
            }
        });

        for _ in 0..100_000 {
            let (w, h) = GdxGraphics::unpack_size(packed.load(Ordering::Relaxed));
            assert!(
                (w == 1920 && h == 1080) || (w == 800 && h == 600),
                "torn read detected: ({}, {})",
                w,
                h,
            );
        }

        stop.store(true, Ordering::Relaxed);
        writer.join().unwrap();
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
        let pixels = pixmap.pixels();
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
        let pixels = pixmap.pixels();
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
        let (width, height) = GdxGraphics::back_buffer_size();

        // Simulate raw OpenGL pixel data (3x2 RGBA)
        #[rustfmt::skip]
        let raw_pixels: Vec<u8> = vec![
            255, 0,   0,   255,   0, 255,   0, 255,   0,   0, 255, 255, // row 0
            128, 128, 128, 255,  64,  64,  64, 255,  32,  32,  32, 255, // row 1
        ];

        let mut pixmap = Pixmap::new(width, height);
        let pixel_buf = pixmap.pixels();
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
        assert_eq!(pixmap.pixel_data().len(), 0);
    }
}
