// External dependency stubs for beatoraja-external crate

use beatoraja_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

//
// Stubs replaced with real types:
//   Config → pub use beatoraja_core::config::Config
//   PlayerConfig → pub use beatoraja_core::player_config::PlayerConfig
//   ScoreData → pub use beatoraja_core::score_data::ScoreData
//   SongData → pub use beatoraja_song::song_data::SongData
//   ReplayData → pub use beatoraja_core::replay_data::ReplayData
//
// Remaining stubs — Why they cannot be replaced:
//
// MainController:
//   Replaced with NullMainController from beatoraja-types (Phase 18e-2).
//
// PlayerResource:
//   Replaced with Box<dyn PlayerResourceAccess> wrapper (Phase 18e-2).
//   get_original_mode() is crate-local (Mode from bms-model, not on trait).
//
// MainState:
//   Real type is a trait (beatoraja_core::main_state::MainState), but external
//   code uses it as a struct with `state.resource` field access. Replacing
//   requires a wrapper struct or reworking all callers.
//
// MainStateListener:
//   Takes &MainState (struct) in stub vs &dyn MainState (trait) in real type.
//
// SongDatabaseAccessor:
//   Real type is a trait in beatoraja-song, stub is a struct. Callers instantiate
//   it as a concrete type.
//
// ScoreDatabaseAccessor:
//   Real type requires path in constructor (new(path) -> Result), stub is unit
//   struct. set_score_data signature differs (&ScoreData vs &[ScoreData]).
//
// TableData, TableFolder, TableDataAccessor, TableAccessor:
//   Replaced with pub use from beatoraja-core (Phase 18e-11).
//
// Mode:
//   Replaced with real bms_model::mode::Mode enum (Phase 18e-2).
//
// IntegerProperty, BooleanProperty, StringProperty traits + factories:
//   Real traits in beatoraja-skin reference beatoraja-skin's own MainState stub
//   trait, not this crate's MainState struct. Type mismatch.
//
// Twitter4j:
//   No real Rust equivalent exists (twitter4j has no Rust port).
//
// ClipboardHelper, Pixmap, GdxGraphics, BufferUtils, PixmapIO:
//   Replaced with real implementations using image crate / atomic globals.
//
// ImGuiNotify:
//   From beatoraja-modmenu (cannot depend on it).
//
// AbstractResult, ScreenType:
//   From beatoraja-result/beatoraja-play (cannot depend on them).

// ============================================================
// MainController — replaced with NullMainController from beatoraja-types (Phase 18e-2)
// ============================================================

pub use beatoraja_types::main_controller_access::NullMainController;

// ============================================================
// PlayerResource — replaced with Box<dyn PlayerResourceAccess> wrapper (Phase 18e-2)
// ============================================================

/// Wrapper for bms.player.beatoraja.PlayerResource.
/// Delegates to `Box<dyn PlayerResourceAccess>` for trait methods.
/// `get_original_mode()` is crate-local (not on trait, since Mode lives in bms-model).
pub struct PlayerResource {
    inner: Box<dyn PlayerResourceAccess>,
    original_mode: Mode,
}

impl PlayerResource {
    pub fn new(inner: Box<dyn PlayerResourceAccess>, original_mode: Mode) -> Self {
        Self {
            inner,
            original_mode,
        }
    }

    pub fn get_config(&self) -> &Config {
        self.inner.get_config()
    }

    pub fn get_songdata(&self) -> Option<&SongData> {
        self.inner.get_songdata()
    }

    pub fn get_replay_data(&self) -> Option<&ReplayData> {
        self.inner.get_replay_data()
    }

    pub fn get_reverse_lookup_levels(&self) -> Vec<String> {
        self.inner.get_reverse_lookup_levels()
    }

    pub fn get_original_mode(&self) -> &Mode {
        &self.original_mode
    }
}

impl Default for PlayerResource {
    fn default() -> Self {
        Self {
            inner: Box::new(NullPlayerResource::new()),
            original_mode: Mode::BEAT_7K,
        }
    }
}

// ============================================================
// Config — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::config::Config;

// ============================================================
// PlayerConfig — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::player_config::PlayerConfig;

// ============================================================
// SongData — replaced with real type from beatoraja-song
// ============================================================

pub use beatoraja_song::song_data::SongData;

// ============================================================
// SongDatabaseAccessor — replaced with real trait from beatoraja-types
// ============================================================

pub use beatoraja_types::song_database_accessor::SongDatabaseAccessor;

// ============================================================
// ScoreData — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::score_data::ScoreData;

// ============================================================
// ScoreDatabaseAccessor stub
// ============================================================

/// Stub for bms.player.beatoraja.ScoreDatabaseAccessor
pub struct ScoreDatabaseAccessor;

impl ScoreDatabaseAccessor {
    pub fn create_table(&self) {
        log::warn!("not yet implemented: ScoreDatabaseAccessor.createTable");
    }

    pub fn get_score_data(&self, _sha256: &str, _mode: i32) -> Option<ScoreData> {
        log::warn!("not yet implemented: ScoreDatabaseAccessor.getScoreData");
        None
    }

    pub fn set_score_data(&self, _scores: &[ScoreData]) {
        log::warn!("not yet implemented: ScoreDatabaseAccessor.setScoreData");
    }
}

// ============================================================
// MainState stub (for ScreenShotExporter)
// ============================================================

/// Stub for bms.player.beatoraja.MainState
pub struct MainState {
    pub main: NullMainController,
    pub resource: PlayerResource,
}

// ============================================================
// Screen type stubs (for instanceof checks)
// ============================================================

/// Enum to represent the current screen state type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ScreenType {
    MusicSelector,
    MusicDecide,
    BMSPlayer,
    MusicResult,
    CourseResult,
    KeyConfiguration,
    Other,
}

// ============================================================
// AbstractResult stub
// ============================================================

/// Stub for bms.player.beatoraja.result.AbstractResult
pub struct AbstractResult {
    pub new_score: ScoreData,
    pub old_score: ScoreData,
    pub ir_rank: i32,
    pub ir_total_player: i32,
    pub old_ir_rank: i32,
}

impl AbstractResult {
    pub fn get_new_score(&self) -> &ScoreData {
        &self.new_score
    }

    pub fn get_old_score(&self) -> &ScoreData {
        &self.old_score
    }

    pub fn get_ir_rank(&self) -> i32 {
        self.ir_rank
    }

    pub fn get_ir_total_player(&self) -> i32 {
        self.ir_total_player
    }

    pub fn get_old_ir_rank(&self) -> i32 {
        self.old_ir_rank
    }
}

// ============================================================
// ReplayData — replaced with real type from beatoraja-core
// ============================================================

pub use beatoraja_core::replay_data::ReplayData;

// ============================================================
// Mode — replaced with real type from bms-model
// ============================================================

pub use bms_model::mode::Mode;

// ============================================================
// TableData and related types — replaced with real types from beatoraja-core (Phase 18e-11)
// ============================================================

pub use beatoraja_core::table_data::{TableData, TableFolder};
pub use beatoraja_core::table_data_accessor::{TableAccessor, TableDataAccessor};

// ============================================================
// LibGDX stubs
// ============================================================

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

// ============================================================
// ImGuiNotify — real type re-export (replaced from stubs)
// ============================================================

pub use beatoraja_types::imgui_notify::ImGuiNotify;

// ============================================================
// IntegerProperty / BooleanProperty / StringProperty stubs
// ============================================================

/// Stub for bms.player.beatoraja.skin.property.IntegerProperty
pub trait IntegerProperty {
    fn get(&self, state: &MainState) -> i32;
}

/// Stub for bms.player.beatoraja.skin.property.BooleanProperty
pub trait BooleanProperty {
    fn get(&self, state: &MainState) -> bool;
}

/// Stub for bms.player.beatoraja.skin.property.StringProperty
pub trait StringProperty {
    fn get(&self, state: &MainState) -> String;
}

/// Stub for IntegerPropertyFactory
pub struct IntegerPropertyFactory;

/// Default integer property returning 0
struct DefaultIntegerProperty;
impl IntegerProperty for DefaultIntegerProperty {
    fn get(&self, _state: &MainState) -> i32 {
        0
    }
}

impl IntegerPropertyFactory {
    pub fn get_integer_property(_id: i32) -> Box<dyn IntegerProperty> {
        log::warn!("not yet implemented: IntegerPropertyFactory.getIntegerProperty");
        Box::new(DefaultIntegerProperty)
    }
}

/// Stub for BooleanPropertyFactory
pub struct BooleanPropertyFactory;

/// Default boolean property returning false
struct DefaultBooleanProperty;
impl BooleanProperty for DefaultBooleanProperty {
    fn get(&self, _state: &MainState) -> bool {
        false
    }
}

impl BooleanPropertyFactory {
    pub fn get_boolean_property(_id: i32) -> Box<dyn BooleanProperty> {
        log::warn!("not yet implemented: BooleanPropertyFactory.getBooleanProperty");
        Box::new(DefaultBooleanProperty)
    }
}

/// Stub for StringPropertyFactory
pub struct StringPropertyFactory;

/// Default string property returning empty string
struct DefaultStringProperty;
impl StringProperty for DefaultStringProperty {
    fn get(&self, _state: &MainState) -> String {
        String::new()
    }
}

impl StringPropertyFactory {
    pub fn get_string_property(_id: i32) -> Box<dyn StringProperty> {
        log::warn!("not yet implemented: StringPropertyFactory.getStringProperty");
        Box::new(DefaultStringProperty)
    }
}

// ============================================================
// SkinProperty constants (re-exported from beatoraja-skin)
// ============================================================

pub use beatoraja_skin::skin_property::{
    NUMBER_CLEAR, NUMBER_MAXSCORE, NUMBER_PLAYLEVEL, OPTION_RESULT_A_1P, OPTION_RESULT_AA_1P,
    OPTION_RESULT_AAA_1P, OPTION_RESULT_B_1P, OPTION_RESULT_C_1P, OPTION_RESULT_D_1P,
    OPTION_RESULT_E_1P, OPTION_RESULT_F_1P, STRING_FULLTITLE, STRING_TABLE_LEVEL,
    STRING_TABLE_NAME,
};

// ============================================================
// Twitter4j stubs (entirely stubbed - no Rust equivalent)
// ============================================================

/// Stub for twitter4j.Twitter — Twitter API not supported in Rust port
pub struct Twitter;

impl Twitter {
    pub fn upload_media(&self, _name: &str, _input: &[u8]) -> anyhow::Result<UploadedMedia> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }

    pub fn update_status(&self, _update: &StatusUpdate) -> anyhow::Result<Status> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }
}

/// Stub for twitter4j.TwitterFactory
pub struct TwitterFactory;

impl TwitterFactory {
    pub fn new(_config: TwitterConfiguration) -> Self {
        Self
    }

    pub fn get_instance(&self) -> Twitter {
        Twitter
    }
}

/// Stub for twitter4j.conf.ConfigurationBuilder
pub struct TwitterConfigurationBuilder {
    pub consumer_key: String,
    pub consumer_secret: String,
    pub access_token: String,
    pub access_token_secret: String,
}

impl Default for TwitterConfigurationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TwitterConfigurationBuilder {
    pub fn new() -> Self {
        Self {
            consumer_key: String::new(),
            consumer_secret: String::new(),
            access_token: String::new(),
            access_token_secret: String::new(),
        }
    }

    pub fn set_o_auth_consumer_key(mut self, key: &str) -> Self {
        self.consumer_key = key.to_string();
        self
    }

    pub fn set_o_auth_consumer_secret(mut self, secret: &str) -> Self {
        self.consumer_secret = secret.to_string();
        self
    }

    pub fn set_o_auth_access_token(mut self, token: &str) -> Self {
        self.access_token = token.to_string();
        self
    }

    pub fn set_o_auth_access_token_secret(mut self, secret: &str) -> Self {
        self.access_token_secret = secret.to_string();
        self
    }

    pub fn build(self) -> TwitterConfiguration {
        TwitterConfiguration
    }
}

/// Stub for twitter4j.conf.Configuration
pub struct TwitterConfiguration;

/// Stub for twitter4j.UploadedMedia
pub struct UploadedMedia {
    pub media_id: i64,
}

impl std::fmt::Display for UploadedMedia {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "UploadedMedia(id={})", self.media_id)
    }
}

impl UploadedMedia {
    pub fn get_media_id(&self) -> i64 {
        self.media_id
    }
}

/// Stub for twitter4j.StatusUpdate
pub struct StatusUpdate {
    pub text: String,
    pub media_ids: Vec<i64>,
}

impl StatusUpdate {
    pub fn new(text: String) -> Self {
        Self {
            text,
            media_ids: Vec::new(),
        }
    }

    pub fn set_media_ids(&mut self, ids: Vec<i64>) {
        self.media_ids = ids;
    }
}

/// Stub for twitter4j.Status
pub struct Status;

impl std::fmt::Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Status")
    }
}

// ============================================================
// AWT Clipboard stubs
// ============================================================

/// Cross-platform clipboard helper using arboard crate
pub struct ClipboardHelper;

impl ClipboardHelper {
    /// Copy an image file to the system clipboard.
    /// Uses arboard for cross-platform clipboard access.
    pub fn copy_image_to_clipboard(path: &str) -> anyhow::Result<()> {
        use arboard::{Clipboard, ImageData};
        use std::borrow::Cow;

        let img = image::open(path)
            .map_err(|e| anyhow::anyhow!("Failed to open image {}: {}", path, e))?;
        let rgba = img.to_rgba8();
        let (width, height) = rgba.dimensions();
        let bytes = rgba.into_raw();

        let img_data = ImageData {
            width: width as usize,
            height: height as usize,
            bytes: Cow::Owned(bytes),
        };

        let mut clipboard =
            Clipboard::new().map_err(|e| anyhow::anyhow!("Failed to access clipboard: {}", e))?;
        clipboard
            .set_image(img_data)
            .map_err(|e| anyhow::anyhow!("Failed to copy image to clipboard: {}", e))?;
        Ok(())
    }
}

// ============================================================
// MainStateListener stub (re-export)
// ============================================================

/// Stub for bms.player.beatoraja.MainStateListener
pub trait MainStateListener {
    fn update(&mut self, state: &MainState, status: i32);
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
