// External dependency stubs for Phase 6 Skin System
// Rendering stubs (LibGDX types) are in rendering_stubs.rs, re-exported here for compatibility.

// Re-export all rendering stubs (LibGDX graphics types, file types, GL constants)
pub use crate::rendering_stubs::*;

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

    /// Returns the integer property value for the given ID.
    /// Used by IntegerPropertyFactory delegate to look up pre-computed values.
    fn integer_value(&self, _id: i32) -> i32 {
        0
    }

    /// Returns the string property value for the given ID.
    /// Used by StringPropertyFactory delegate to look up pre-computed values.
    fn string_value(&self, _id: i32) -> String {
        String::new()
    }
}

/// Stub for beatoraja.MainController
pub struct MainController {
    pub debug: bool,
}

impl MainController {
    pub fn get_input_processor(&self) -> &InputProcessor {
        static INPUT: std::sync::OnceLock<InputProcessor> = std::sync::OnceLock::new();
        INPUT.get_or_init(|| InputProcessor)
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        static CONFIG: std::sync::OnceLock<beatoraja_core::config::Config> =
            std::sync::OnceLock::new();
        CONFIG.get_or_init(beatoraja_core::config::Config::default)
    }
}

/// Stub for input processor
pub struct InputProcessor;

// SAFETY: InputProcessor is a stateless unit struct.
unsafe impl Send for InputProcessor {}
unsafe impl Sync for InputProcessor {}

impl InputProcessor {
    pub fn get_mouse_x(&self) -> f32 {
        0.0
    }
    pub fn get_mouse_y(&self) -> f32 {
        0.0
    }
}

// SkinOffset — re-exported from beatoraja-types (Phase 25d-2)
pub use beatoraja_types::skin_offset::SkinOffset;

/// Stub for beatoraja.Timer — implements TimerAccess from beatoraja-types.
///
/// This struct is kept for backward compatibility. New code should use
/// `&dyn beatoraja_types::timer_access::TimerAccess` directly.
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
        0
    }

    pub fn get_timer(&self, _timer_id: i32) -> i64 {
        0
    }

    pub fn get_now_time_for(&self, _timer_id: i32) -> i64 {
        0
    }

    pub fn is_timer_on(&self, _timer_id: i32) -> bool {
        false
    }
}

impl beatoraja_types::timer_access::TimerAccess for Timer {
    fn get_now_time(&self) -> i64 {
        self.now_time
    }
    fn get_now_micro_time(&self) -> i64 {
        self.now_micro_time
    }
    fn get_micro_timer(&self, _timer_id: i32) -> i64 {
        0
    }
    fn get_timer(&self, _timer_id: i32) -> i64 {
        0
    }
    fn get_now_time_for(&self, _timer_id: i32) -> i64 {
        0
    }
    fn is_timer_on(&self, _timer_id: i32) -> bool {
        false
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
        static DEFAULT: std::sync::OnceLock<TimingDistribution> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(|| TimingDistribution {
            distribution: vec![],
            array_center: 0,
            average: 0.0,
            std_dev: 0.0,
        })
    }
}

/// Stub for PlayerResource within MusicResult context
pub struct MusicResultResource;

impl MusicResultResource {
    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        static MODEL: std::sync::OnceLock<bms_model::bms_model::BMSModel> =
            std::sync::OnceLock::new();
        MODEL.get_or_init(bms_model::bms_model::BMSModel::default)
    }

    pub fn get_original_mode(&self) -> bms_model::mode::Mode {
        bms_model::mode::Mode::BEAT_7K
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<beatoraja_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(beatoraja_core::player_config::PlayerConfig::default)
    }

    pub fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
        vec![]
    }
}

// TimingDistribution — re-exported from beatoraja-types (Phase 25d-2)
pub use beatoraja_types::timing_distribution::TimingDistribution;

// beatoraja.song types (re-exports)
pub use beatoraja_song::song_data::SongData;
pub use beatoraja_song::song_information::SongInformation;

/// Stub for beatoraja.PlayerResource
pub struct PlayerResource;

impl PlayerResource {
    pub fn get_songdata(&self) -> Option<&SongData> {
        None
    }

    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        static MODEL: std::sync::OnceLock<bms_model::bms_model::BMSModel> =
            std::sync::OnceLock::new();
        MODEL.get_or_init(bms_model::bms_model::BMSModel::default)
    }

    pub fn get_original_mode(&self) -> bms_model::mode::Mode {
        bms_model::mode::Mode::BEAT_7K
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        static PC: std::sync::OnceLock<beatoraja_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        PC.get_or_init(beatoraja_core::player_config::PlayerConfig::default)
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        static CFG: std::sync::OnceLock<beatoraja_core::config::Config> =
            std::sync::OnceLock::new();
        CFG.get_or_init(beatoraja_core::config::Config::default)
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
    pub fn get_texture(path: &str, usecim: bool) -> Option<Texture> {
        crate::skin_loader::get_texture(path, usecim)
    }
}
