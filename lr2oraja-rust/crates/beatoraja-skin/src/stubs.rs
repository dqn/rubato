// External dependency stubs for Phase 6 Skin System
// These will be replaced with actual implementations when corresponding phases are translated.

use std::collections::HashMap;

// ============================================================
// LibGDX graphics types
// ============================================================

/// Stub for com.badlogic.gdx.graphics.g2d.TextureRegion
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TextureRegion {
    pub u: f32,
    pub v: f32,
    pub u2: f32,
    pub v2: f32,
    pub region_x: i32,
    pub region_y: i32,
    pub region_width: i32,
    pub region_height: i32,
    pub texture: Option<Texture>,
}

impl TextureRegion {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn from_texture(texture: Texture) -> Self {
        Self {
            region_width: texture.width,
            region_height: texture.height,
            texture: Some(texture),
            region_x: 0,
            region_y: 0,
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
        }
    }

    pub fn from_texture_region(texture: Texture, x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            region_x: x,
            region_y: y,
            region_width: width,
            region_height: height,
            texture: Some(texture),
            u: 0.0,
            v: 0.0,
            u2: 1.0,
            v2: 1.0,
        }
    }

    pub fn get_region_x(&self) -> i32 {
        self.region_x
    }

    pub fn get_region_y(&self) -> i32 {
        self.region_y
    }

    pub fn get_region_width(&self) -> i32 {
        self.region_width
    }

    pub fn get_region_height(&self) -> i32 {
        self.region_height
    }

    pub fn set_region_x(&mut self, x: i32) {
        self.region_x = x;
    }

    pub fn set_region_y(&mut self, y: i32) {
        self.region_y = y;
    }

    pub fn set_region_width(&mut self, width: i32) {
        self.region_width = width;
    }

    pub fn set_region_height(&mut self, height: i32) {
        self.region_height = height;
    }

    pub fn get_texture(&self) -> Option<&Texture> {
        self.texture.as_ref()
    }

    pub fn set_texture(&mut self, texture: Texture) {
        self.texture = Some(texture);
    }

    pub fn set_region_from(&mut self, x: i32, y: i32, width: i32, height: i32) {
        self.region_x = x;
        self.region_y = y;
        self.region_width = width;
        self.region_height = height;
    }

    pub fn flip(&mut self, _x: bool, _y: bool) {
        // stub
    }

    pub fn set_from(&mut self, other: &TextureRegion) {
        self.u = other.u;
        self.v = other.v;
        self.u2 = other.u2;
        self.v2 = other.v2;
        self.region_x = other.region_x;
        self.region_y = other.region_y;
        self.region_width = other.region_width;
        self.region_height = other.region_height;
        self.texture = other.texture.clone();
    }
}

/// Stub for com.badlogic.gdx.graphics.Texture
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Texture {
    pub width: i32,
    pub height: i32,
    pub disposed: bool,
}

impl Texture {
    pub fn new(_path: &str) -> Self {
        Self::default()
    }

    pub fn from_pixmap(pixmap: &Pixmap) -> Self {
        Self {
            width: pixmap.width,
            height: pixmap.height,
            disposed: false,
        }
    }

    pub fn from_pixmap_with_mipmaps(pixmap: &Pixmap, _use_mip_maps: bool) -> Self {
        Self {
            width: pixmap.width,
            height: pixmap.height,
            disposed: false,
        }
    }

    pub fn new_sized(width: i32, height: i32, _format: PixmapFormat) -> Self {
        Self {
            width,
            height,
            disposed: false,
        }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn set_filter(&mut self, _min: TextureFilter, _mag: TextureFilter) {
        // stub
    }

    pub fn draw_pixmap(&mut self, _pixmap: &Pixmap, _x: i32, _y: i32) {
        // stub - corresponds to Texture.draw(Pixmap, x, y)
    }

    pub fn dispose(&mut self) {
        self.disposed = true;
    }
}

/// Stub for com.badlogic.gdx.graphics.Texture.TextureFilter
#[derive(Clone, Debug, PartialEq)]
pub enum TextureFilter {
    Nearest,
    Linear,
    MipMap,
    MipMapNearestNearest,
    MipMapLinearNearest,
    MipMapNearestLinear,
    MipMapLinearLinear,
}

/// Stub for com.badlogic.gdx.graphics.g2d.SpriteBatch
#[derive(Debug, Default)]
pub struct SpriteBatch;

#[allow(unused_variables)]
impl SpriteBatch {
    pub fn new() -> Self {
        Self
    }

    pub fn set_transform_matrix(&mut self, matrix: &Matrix4) {}
    pub fn set_shader(&mut self, shader: Option<&ShaderProgram>) {}
    pub fn set_color(&mut self, color: &Color) {}
    pub fn get_color(&self) -> Color {
        Color::WHITE
    }
    pub fn set_blend_function(&mut self, src: i32, dst: i32) {}
    pub fn flush(&mut self) {}
    pub fn draw_texture(&mut self, texture: &Texture, x: f32, y: f32, w: f32, h: f32) {}
    pub fn draw_region(&mut self, region: &TextureRegion, x: f32, y: f32, w: f32, h: f32) {}
    pub fn draw_region_rotated(
        &mut self,
        region: &TextureRegion,
        x: f32,
        y: f32,
        cx: f32,
        cy: f32,
        w: f32,
        h: f32,
        sx: f32,
        sy: f32,
        angle: f32,
    ) {
    }
}

/// Stub for com.badlogic.gdx.graphics.Color
#[derive(Clone, Debug)]
pub struct Color {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl Default for Color {
    fn default() -> Self {
        Self {
            r: 1.0,
            g: 1.0,
            b: 1.0,
            a: 1.0,
        }
    }
}

impl Color {
    pub const WHITE: Color = Color {
        r: 1.0,
        g: 1.0,
        b: 1.0,
        a: 1.0,
    };
    pub const BLACK: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    pub const CLEAR: Color = Color {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 0.0,
    };

    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }

    /// Parses a hex color string (e.g. "FF0000FF") into a Color.
    /// Corresponds to com.badlogic.gdx.graphics.Color.valueOf(String)
    pub fn value_of(hex: &str) -> Self {
        let hex = hex.trim();
        let len = hex.len();
        if len < 6 {
            return Color::new(1.0, 0.0, 0.0, 1.0); // fallback red
        }
        let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(255) as f32 / 255.0;
        let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0) as f32 / 255.0;
        let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0) as f32 / 255.0;
        let a = if len >= 8 {
            u8::from_str_radix(&hex[6..8], 16).unwrap_or(255) as f32 / 255.0
        } else {
            1.0
        };
        Color::new(r, g, b, a)
    }

    /// Packs r, g, b, a into an integer (Color.rgba8888 equivalent)
    pub fn rgba8888(r: f32, g: f32, b: f32, a: f32) -> i32 {
        ((255.0 * r) as i32) << 24
            | ((255.0 * g) as i32) << 16
            | ((255.0 * b) as i32) << 8
            | ((255.0 * a) as i32)
    }

    /// Corresponds to com.badlogic.gdx.graphics.Color.toIntBits(a, b, g, r)
    /// Note: LibGDX's toIntBits packs as ABGR
    pub fn to_int_bits(a: i32, b: i32, g: i32, r: i32) -> i32 {
        (a << 24) | (b << 16) | (g << 8) | r
    }

    pub fn set(&mut self, other: &Color) {
        self.r = other.r;
        self.g = other.g;
        self.b = other.b;
        self.a = other.a;
    }

    pub fn set_rgba(&mut self, r: f32, g: f32, b: f32, a: f32) {
        self.r = r;
        self.g = g;
        self.b = b;
        self.a = a;
    }

    pub fn equals(&self, other: &Color) -> bool {
        (self.r - other.r).abs() < f32::EPSILON
            && (self.g - other.g).abs() < f32::EPSILON
            && (self.b - other.b).abs() < f32::EPSILON
            && (self.a - other.a).abs() < f32::EPSILON
    }
}

/// Stub for com.badlogic.gdx.math.Rectangle
#[derive(Clone, Debug, Default)]
pub struct Rectangle {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

impl Rectangle {
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub fn set(&mut self, other: &Rectangle) {
        self.x = other.x;
        self.y = other.y;
        self.width = other.width;
        self.height = other.height;
    }

    pub fn set_xywh(&mut self, x: f32, y: f32, w: f32, h: f32) {
        self.x = x;
        self.y = y;
        self.width = w;
        self.height = h;
    }

    pub fn contains(&self, x: f32, y: f32) -> bool {
        x >= 0.0 && x <= self.width && y >= 0.0 && y <= self.height
    }

    pub fn equals(&self, other: &Rectangle) -> bool {
        (self.x - other.x).abs() < f32::EPSILON
            && (self.y - other.y).abs() < f32::EPSILON
            && (self.width - other.width).abs() < f32::EPSILON
            && (self.height - other.height).abs() < f32::EPSILON
    }
}

/// Stub for com.badlogic.gdx.math.Matrix4
#[derive(Clone, Debug, Default)]
pub struct Matrix4 {
    pub values: [f32; 16],
}

impl Matrix4 {
    pub fn new() -> Self {
        Self::default()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn set(
        &mut self,
        _tx: f32,
        _ty: f32,
        _tz: f32,
        _qx: f32,
        _qy: f32,
        _qz: f32,
        _qw: f32,
        _sx: f32,
        _sy: f32,
        _sz: f32,
    ) {
        // stub
    }
}

/// Stub for com.badlogic.gdx.graphics.glutils.ShaderProgram
#[derive(Clone, Debug, Default)]
pub struct ShaderProgram;

/// Stub for com.badlogic.gdx.graphics.Pixmap
#[derive(Clone, Debug, Default)]
pub struct Pixmap {
    pub width: i32,
    pub height: i32,
}

impl Pixmap {
    pub fn new(width: i32, height: i32, _format: PixmapFormat) -> Self {
        Self { width, height }
    }

    pub fn get_width(&self) -> i32 {
        self.width
    }

    pub fn get_height(&self) -> i32 {
        self.height
    }

    pub fn draw_pixmap(
        &mut self,
        _src: &Pixmap,
        _sx: i32,
        _sy: i32,
        _sw: i32,
        _sh: i32,
        _dx: i32,
        _dy: i32,
        _dw: i32,
        _dh: i32,
    ) {
    }

    pub fn set_color_rgba(&mut self, _r: f32, _g: f32, _b: f32, _a: f32) {}

    pub fn set_color(&mut self, _color: &Color) {}

    pub fn fill(&mut self) {}

    pub fn fill_rectangle(&mut self, _x: i32, _y: i32, _width: i32, _height: i32) {}

    pub fn draw_line(&mut self, _x1: i32, _y1: i32, _x2: i32, _y2: i32) {}

    pub fn draw_pixel(&mut self, _x: i32, _y: i32, _color: i32) {}

    pub fn set_color_int(&mut self, _color: i32) {}

    pub fn get_pixel(&self, _x: i32, _y: i32) -> i32 {
        0
    }

    pub fn fill_triangle(&mut self, _x1: i32, _y1: i32, _x2: i32, _y2: i32, _x3: i32, _y3: i32) {}

    pub fn dispose(&mut self) {}
}

#[derive(Clone, Debug)]
pub enum PixmapFormat {
    RGBA8888,
    RGB888,
    Alpha,
}

/// Stub for com.badlogic.gdx.graphics.g2d.BitmapFont.BitmapFontData
#[derive(Clone, Debug, Default)]
pub struct BitmapFontData;

/// Stub for com.badlogic.gdx.graphics.g2d.BitmapFont
#[derive(Clone, Debug, Default)]
pub struct BitmapFont;

#[allow(unused_variables)]
impl BitmapFont {
    pub fn new() -> Self {
        Self
    }

    pub fn get_regions(&self) -> Vec<TextureRegion> {
        vec![]
    }

    pub fn set_color(&mut self, color: &Color) {}

    pub fn draw(&self, batch: &mut SpriteBatch, text: &str, x: f32, y: f32) {}

    pub fn draw_layout(&self, batch: &mut SpriteBatch, layout: &GlyphLayout, x: f32, y: f32) {}

    pub fn dispose(&mut self) {}
}

/// Stub for com.badlogic.gdx.graphics.g2d.GlyphLayout
#[derive(Clone, Debug, Default)]
pub struct GlyphLayout {
    pub width: f32,
    pub height: f32,
}

impl GlyphLayout {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Stub for com.badlogic.gdx.graphics.g2d.freetype.FreeTypeFontGenerator
#[derive(Clone, Debug, Default)]
pub struct FreeTypeFontGenerator;

impl FreeTypeFontGenerator {
    pub fn new(_font_file: &str) -> Self {
        Self
    }

    pub fn generate_font(&self, _param: &FreeTypeFontParameter) -> BitmapFont {
        BitmapFont::new()
    }

    pub fn dispose(&mut self) {}
}

#[derive(Clone, Debug, Default)]
pub struct FreeTypeFontParameter {
    pub size: i32,
    pub border_width: f32,
    pub border_color: Color,
    pub color: Color,
    pub characters: String,
}

// ============================================================
// LibGDX file types
// ============================================================

/// Stub for com.badlogic.gdx.files.FileHandle
#[derive(Clone, Debug, Default)]
pub struct FileHandle {
    pub path: String,
}

impl FileHandle {
    pub fn new(path: &str) -> Self {
        Self {
            path: path.to_string(),
        }
    }

    pub fn exists(&self) -> bool {
        std::path::Path::new(&self.path).exists()
    }

    pub fn name(&self) -> &str {
        std::path::Path::new(&self.path)
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
    }

    pub fn extension(&self) -> &str {
        std::path::Path::new(&self.path)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn parent(&self) -> FileHandle {
        let p = std::path::Path::new(&self.path);
        FileHandle {
            path: p
                .parent()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default(),
        }
    }

    pub fn child(&self, name: &str) -> FileHandle {
        let p = std::path::Path::new(&self.path).join(name);
        FileHandle {
            path: p.to_string_lossy().into_owned(),
        }
    }

    pub fn sibling(&self, name: &str) -> FileHandle {
        self.parent().child(name)
    }

    pub fn list(&self) -> Vec<FileHandle> {
        vec![]
    }

    pub fn read_string(&self) -> String {
        std::fs::read_to_string(&self.path).unwrap_or_default()
    }
}

// ============================================================
// Gdx global
// ============================================================

pub struct Gdx;

impl Gdx {
    pub fn files_internal(path: &str) -> FileHandle {
        FileHandle::new(path)
    }
}

// ============================================================
// OpenGL constants
// ============================================================

pub mod gl11 {
    pub const GL_SRC_ALPHA: i32 = 0x0302;
    pub const GL_ONE: i32 = 1;
    pub const GL_ONE_MINUS_SRC_ALPHA: i32 = 0x0303;
    pub const GL_ZERO: i32 = 0;
    pub const GL_SRC_COLOR: i32 = 0x0300;
    pub const GL_ONE_MINUS_DST_COLOR: i32 = 0x0307;
}

pub mod gl20 {
    pub const GL_FUNC_ADD: i32 = 0x8006;
    pub const GL_FUNC_SUBTRACT: i32 = 0x800A;
}

// ============================================================
// beatoraja types (from other crates, stubbed for phase independence)
// ============================================================

/// Stub for beatoraja.MainState
pub trait MainState {
    fn get_timer(&self) -> &Timer;
    fn get_offset_value(&self, id: i32) -> Option<&SkinOffset>;
    fn get_main(&self) -> &MainController;
    fn get_image(&self, id: i32) -> Option<TextureRegion>;
    fn get_resource(&self) -> &PlayerResource;
}

/// Stub for beatoraja.MainController
pub struct MainController {
    pub debug: bool,
}

impl MainController {
    pub fn get_input_processor(&self) -> &InputProcessor {
        todo!("Phase 7+ dependency")
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        todo!("Phase 7+ dependency")
    }
}

/// Stub for input processor
pub struct InputProcessor;

impl InputProcessor {
    pub fn get_mouse_x(&self) -> f32 {
        0.0
    }
    pub fn get_mouse_y(&self) -> f32 {
        0.0
    }
}

/// Stub for SkinOffset (shared between Skin and SkinObject)
#[derive(Clone, Debug, Default)]
pub struct SkinOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
}

/// Stub for beatoraja.Timer
#[derive(Clone, Debug, Default)]
pub struct Timer {
    pub now_time: i64,
    pub now_micro_time: i64,
}

impl Timer {
    pub fn get_now_time(&self) -> i64 {
        self.now_time
    }

    pub fn get_now_micro_time(&self) -> i64 {
        self.now_micro_time
    }

    pub fn get_micro_timer(&self, _timer_id: i32) -> i64 {
        todo!("Phase 7+ dependency: Timer.getMicroTimer")
    }

    pub fn get_timer(&self, _timer_id: i32) -> i64 {
        todo!("Phase 7+ dependency: Timer.getTimer")
    }

    pub fn get_now_time_for(&self, _timer_id: i32) -> i64 {
        todo!("Phase 7+ dependency: Timer.getNowTime(timerId)")
    }

    pub fn is_timer_on(&self, _timer_id: i32) -> bool {
        todo!("Phase 7+ dependency: Timer.isTimerOn")
    }
}

/// Stub for beatoraja.DisposableObject
pub trait DisposableObject {
    fn dispose(&mut self);
    fn is_disposed(&self) -> bool;
}

/// Stub for beatoraja.Validatable
pub trait Validatable {
    fn validate(&self) -> bool;
}

/// Stub for beatoraja.ShaderManager
pub struct ShaderManager;

impl ShaderManager {
    pub fn get_shader(_name: &str) -> Option<ShaderProgram> {
        None
    }
}

/// Stub for beatoraja.Resolution
#[derive(Clone, Debug, Default)]
pub struct Resolution {
    pub width: f32,
    pub height: f32,
}

/// Stub for beatoraja.SkinConfig.Offset
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SkinConfigOffset {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
    pub enabled: bool,
}

// ============================================================
// beatoraja.play types (stubs)
// ============================================================

/// Stub for beatoraja.play.BMSPlayer
pub struct BMSPlayer {
    pub judge_manager: JudgeManager,
}

impl BMSPlayer {
    pub fn get_skin_type(&self) -> crate::skin_type::SkinType {
        crate::skin_type::SkinType::Play7Keys
    }

    pub fn get_past_notes(&self) -> i32 {
        0
    }

    pub fn get_judge_manager(&self) -> &JudgeManager {
        &self.judge_manager
    }
}

/// Stub for beatoraja.play.JudgeManager (minimal for visualizers)
pub struct JudgeManager {
    pub recent_judges: Vec<i64>,
    pub recent_judges_index: usize,
}

impl JudgeManager {
    pub fn get_recent_judges_index(&self) -> usize {
        self.recent_judges_index
    }

    pub fn get_recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }
}

/// Stub for beatoraja.result.MusicResult
pub struct MusicResult {
    pub resource: MusicResultResource,
}

impl MusicResult {
    pub fn get_timing_distribution(&self) -> &TimingDistribution {
        todo!("Phase 7+ dependency: MusicResult.getTimingDistribution")
    }
}

/// Stub for PlayerResource within MusicResult context
pub struct MusicResultResource;

impl MusicResultResource {
    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        todo!("Phase 7+ dependency")
    }

    pub fn get_original_mode(&self) -> bms_model::mode::Mode {
        todo!("Phase 7+ dependency")
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        todo!("Phase 7+ dependency")
    }

    pub fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
        vec![]
    }
}

/// Stub for beatoraja.result.AbstractResult.TimingDistribution
pub struct TimingDistribution {
    pub distribution: Vec<i32>,
    pub array_center: i32,
    pub average: f32,
    pub std_dev: f32,
}

impl TimingDistribution {
    pub fn get_timing_distribution(&self) -> &[i32] {
        &self.distribution
    }

    pub fn get_array_center(&self) -> i32 {
        self.array_center
    }

    pub fn get_average(&self) -> f32 {
        self.average
    }

    pub fn get_std_dev(&self) -> f32 {
        self.std_dev
    }
}

// ============================================================
// beatoraja.song types (stubs)
// ============================================================

/// Stub for beatoraja.song.SongData
#[derive(Clone, Debug, Default)]
pub struct SongData {
    pub length: i32,
}

impl SongData {
    pub fn get_bms_model(&self) -> Option<&bms_model::bms_model::BMSModel> {
        None
    }

    pub fn get_information(&self) -> Option<&SongInformation> {
        None
    }

    pub fn get_length(&self) -> i32 {
        self.length
    }
}

/// Stub for beatoraja.song.SongInformation
#[derive(Clone, Debug, Default)]
pub struct SongInformation {
    pub mainbpm: f64,
}

impl SongInformation {
    pub fn get_speedchange_values(&self) -> Vec<[f64; 2]> {
        vec![]
    }

    pub fn get_distribution_values(&self) -> Vec<Vec<i32>> {
        vec![]
    }

    pub fn get_mainbpm(&self) -> f64 {
        self.mainbpm
    }
}

/// Stub for beatoraja.PlayerResource
pub struct PlayerResource;

impl PlayerResource {
    pub fn get_songdata(&self) -> Option<&SongData> {
        None
    }

    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        todo!("Phase 7+ dependency: PlayerResource.getBMSModel")
    }

    pub fn get_original_mode(&self) -> bms_model::mode::Mode {
        todo!("Phase 7+ dependency: PlayerResource.getOriginalMode")
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        todo!("Phase 7+ dependency: PlayerResource.getPlayerConfig")
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        todo!("Phase 7+ dependency: PlayerResource.getConfig")
    }

    pub fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
        vec![]
    }
}

/// Stub for beatoraja.play.PlaySkin
pub struct PlaySkinStub {
    pub pomyu: beatoraja_play::pomyu_chara_processor::PomyuCharaProcessor,
}

impl Default for PlaySkinStub {
    fn default() -> Self {
        Self::new()
    }
}

impl PlaySkinStub {
    pub fn new() -> Self {
        Self {
            pomyu: beatoraja_play::pomyu_chara_processor::PomyuCharaProcessor::new(),
        }
    }

    pub fn add(&mut self, _obj: crate::skin_image::SkinImage) {
        // stub
    }
}

/// Stub for beatoraja.skin.SkinLoader (static methods)
pub struct SkinLoaderStub;

impl SkinLoaderStub {
    pub fn get_texture(_path: &str, _usecim: bool) -> Option<Texture> {
        todo!("Image loading")
    }
}

// ============================================================
// Video / Movie types (stubs)
// ============================================================

/// Stub for bms.player.beatoraja.play.bga.FFmpegProcessor
pub struct FFmpegProcessor {
    pub disposed: bool,
}

impl FFmpegProcessor {
    pub fn new(_parallel: i32) -> Self {
        Self { disposed: false }
    }

    pub fn create(&mut self, _path: &str) {
        // stub
    }

    pub fn play(&mut self, _time: i64, _loop_play: bool) {
        // stub
    }

    pub fn get_frame(&self, _time: i64) -> Option<Texture> {
        None
    }

    pub fn dispose(&mut self) {
        self.disposed = true;
    }
}

/// Stub for video player
pub struct VideoProcessor;

impl Default for VideoProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl VideoProcessor {
    pub fn new() -> Self {
        Self
    }

    pub fn play(&mut self, _path: &str) {}
    pub fn stop(&mut self) {}
    pub fn get_texture(&self) -> Option<Texture> {
        None
    }
    pub fn dispose(&mut self) {}
}
