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
