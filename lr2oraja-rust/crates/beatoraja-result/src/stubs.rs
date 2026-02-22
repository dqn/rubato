// External dependency stubs for beatoraja-result crate
// These will be replaced with actual implementations when corresponding phases are translated.

use std::sync::Arc;

// ============================================================
// Re-exports from real crates (Phase 11 stub replacements)
// ============================================================

pub use beatoraja_core::timer_manager::TimerManager;
pub use beatoraja_input::key_command::KeyCommand;
pub use beatoraja_input::keyboard_input_processor::ControlKeys;
pub use beatoraja_skin::skin::Skin;
pub use beatoraja_skin::skin_header::SkinHeader;
pub use beatoraja_skin::skin_object::SkinObjectRenderer;
pub use beatoraja_skin::stubs::Color;
pub use beatoraja_skin::stubs::Pixmap;
pub use beatoraja_skin::stubs::PixmapFormat;
pub use beatoraja_skin::stubs::Rectangle;
pub use beatoraja_skin::stubs::Texture;
pub use beatoraja_skin::stubs::TextureRegion;
use beatoraja_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};
use beatoraja_types::song_data::SongData;

// PlayDataAccessor: replaced by pub use from beatoraja_core (Phase 18e-4)
pub use beatoraja_core::play_data_accessor::PlayDataAccessor;

// ============================================================
// MainController stub
// ============================================================

/// Stub for bms.player.beatoraja.MainController
pub struct MainController;

impl MainController {
    pub fn get_input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        log::warn!("not yet implemented: MainController.getInputProcessor");
        // Leak a boxed value to get a &'static mut reference - stub only
        Box::leak(Box::new(BMSPlayerInputProcessor))
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        log::warn!("not yet implemented: MainController.getIRStatus");
        &[]
    }

    pub fn save_last_recording(&self, _tag: &str) {
        log::warn!("not yet implemented: MainController.saveLastRecording");
    }

    pub fn ir_send_status(&self) -> &Vec<IRSendStatusMain> {
        log::warn!("not yet implemented: MainController.irSendStatus");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(Vec::new()))
    }

    pub fn ir_send_status_mut(&mut self) -> &mut Vec<IRSendStatusMain> {
        log::warn!("not yet implemented: MainController.irSendStatus_mut");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(Vec::new()))
    }

    pub fn get_play_data_accessor(&self) -> &PlayDataAccessor {
        log::warn!("not yet implemented: MainController.getPlayDataAccessor");
        // Leak a boxed null instance - stub only, will be replaced with real implementation
        Box::leak(Box::new(PlayDataAccessor::null()))
    }
}

// ============================================================
// IR (Internet Ranking) stubs
// ============================================================

/// Stub for bms.player.beatoraja.MainController.IRStatus
pub struct IRStatus {
    pub connection: Arc<dyn IRConnection>,
    pub config: IRConfig,
}

// IRConnection: replaced by pub use from beatoraja_ir (trait)
pub use beatoraja_ir::ir_connection::IRConnection;

// IRConfig: replaced by pub use from beatoraja_core
pub use beatoraja_core::ir_config::IRConfig;

// IRScoreData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ir_score_data::IRScoreData;

// IRCourseData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ir_course_data::IRCourseData;

// RankingData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ranking_data::RankingData;

// ============================================================
// MainController.IRSendStatus (for MusicResult)
// ============================================================

/// Stub for bms.player.beatoraja.MainController.IRSendStatus
pub struct IRSendStatusMain {
    pub connection: Arc<dyn IRConnection>,
    pub songdata: SongData,
    pub score: beatoraja_core::score_data::ScoreData,
    pub retry: i32,
}

impl IRSendStatusMain {
    pub fn new(
        connection: Arc<dyn IRConnection>,
        songdata: &SongData,
        score: &beatoraja_core::score_data::ScoreData,
    ) -> Self {
        Self {
            connection,
            songdata: songdata.clone(),
            score: score.clone(),
            retry: 0,
        }
    }

    pub fn send(&mut self) -> bool {
        log::warn!("not yet implemented: IRSendStatus.send");
        false
    }
}

// ============================================================
// Input stubs
// ============================================================

/// Stub for bms.player.beatoraja.input.BMSPlayerInputProcessor
pub struct BMSPlayerInputProcessor;

impl BMSPlayerInputProcessor {
    pub fn get_scroll(&self) -> i32 {
        0
    }

    pub fn reset_scroll(&mut self) {
        // stub
    }

    pub fn get_key_state(&self, _index: i32) -> bool {
        false
    }

    pub fn reset_key_changed_time(&mut self, _index: i32) -> bool {
        false
    }

    pub fn reset_all_key_changed_time(&mut self) {
        // stub
    }

    pub fn is_control_key_pressed(&self, _key: ControlKeys) -> bool {
        false
    }

    pub fn is_activated(&self, _command: KeyCommand) -> bool {
        false
    }
}

// GrooveGauge: replaced by real type from beatoraja-types
pub use beatoraja_types::groove_gauge::GrooveGauge;

// GdxArray: replaced by Vec<T> — callers updated to use Vec directly

// ============================================================
// PlayerResource — replaced with Box<dyn PlayerResourceAccess> wrapper (Phase 18e-2)
// ============================================================

/// Wrapper for bms.player.beatoraja.PlayerResource.
/// Delegates to `Box<dyn PlayerResourceAccess>` for trait methods.
/// Crate-local methods provide access to non-trait types (BMSModel, BMSPlayerMode, RankingData).
pub struct PlayerResource {
    inner: Box<dyn PlayerResourceAccess>,
    bms_model: bms_model::bms_model::BMSModel,
    course_bms_models: Option<Vec<bms_model::bms_model::BMSModel>>,
    play_mode: BMSPlayerMode,
    ranking_data: Option<RankingData>,
}

impl PlayerResource {
    pub fn new(inner: Box<dyn PlayerResourceAccess>, play_mode: BMSPlayerMode) -> Self {
        Self {
            inner,
            bms_model: bms_model::bms_model::BMSModel::default(),
            course_bms_models: None,
            play_mode,
            ranking_data: None,
        }
    }

    // ---- Trait-delegated methods ----

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        self.inner.get_config()
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        self.inner.get_player_config()
    }

    pub fn get_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        self.inner.get_score_data()
    }

    pub fn get_score_data_mut(&mut self) -> Option<&mut beatoraja_core::score_data::ScoreData> {
        self.inner.get_score_data_mut()
    }

    pub fn get_target_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        self.inner.get_target_score_data()
    }

    pub fn get_course_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        self.inner.get_course_score_data()
    }

    pub fn set_course_score_data(&mut self, score: beatoraja_core::score_data::ScoreData) {
        self.inner.set_course_score_data(score);
    }

    pub fn get_songdata(&self) -> Option<&beatoraja_types::song_data::SongData> {
        self.inner.get_songdata()
    }

    pub fn get_replay_data(&self) -> Option<&beatoraja_core::replay_data::ReplayData> {
        self.inner.get_replay_data()
    }

    pub fn get_course_replay(&self) -> &[beatoraja_core::replay_data::ReplayData] {
        self.inner.get_course_replay()
    }

    pub fn get_course_replay_mut(&mut self) -> &mut Vec<beatoraja_core::replay_data::ReplayData> {
        self.inner.get_course_replay_mut()
    }

    pub fn add_course_replay(&mut self, replay: beatoraja_core::replay_data::ReplayData) {
        self.inner.add_course_replay(replay);
    }

    pub fn get_course_data(&self) -> Option<&beatoraja_core::course_data::CourseData> {
        self.inner.get_course_data()
    }

    pub fn get_course_index(&self) -> usize {
        self.inner.get_course_index()
    }

    pub fn next_course(&mut self) -> bool {
        self.inner.next_course()
    }

    pub fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
        self.inner.get_constraint()
    }

    pub fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
        self.inner.get_gauge()
    }

    pub fn get_groove_gauge(&self) -> Option<&GrooveGauge> {
        self.inner.get_groove_gauge()
    }

    pub fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        self.inner.get_course_gauge()
    }

    pub fn get_course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        self.inner.get_course_gauge_mut()
    }

    pub fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
        self.inner.add_course_gauge(gauge);
    }

    pub fn get_maxcombo(&self) -> i32 {
        self.inner.get_maxcombo()
    }

    pub fn get_org_gauge_option(&self) -> i32 {
        self.inner.get_org_gauge_option()
    }

    pub fn get_assist(&self) -> i32 {
        self.inner.get_assist()
    }

    pub fn is_update_score(&self) -> bool {
        self.inner.is_update_score()
    }

    pub fn is_update_course_score(&self) -> bool {
        self.inner.is_update_course_score()
    }

    pub fn is_force_no_ir_send(&self) -> bool {
        self.inner.is_force_no_ir_send()
    }

    pub fn is_freq_on(&self) -> bool {
        self.inner.is_freq_on()
    }

    // ---- Crate-local methods (not on trait — types cause circular deps) ----

    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        &self.bms_model
    }

    pub fn get_course_bms_models(&self) -> Option<&[bms_model::bms_model::BMSModel]> {
        self.course_bms_models.as_deref()
    }

    pub fn get_play_mode(&self) -> &BMSPlayerMode {
        &self.play_mode
    }

    pub fn get_ranking_data(&self) -> Option<&RankingData> {
        self.ranking_data.as_ref()
    }

    pub fn set_ranking_data(&mut self, data: Option<RankingData>) {
        self.ranking_data = data;
    }
}

impl Default for PlayerResource {
    fn default() -> Self {
        Self {
            inner: Box::new(NullPlayerResource::new()),
            bms_model: bms_model::bms_model::BMSModel::default(),
            course_bms_models: None,
            play_mode: BMSPlayerMode::new(BMSPlayerModeType::Play),
            ranking_data: None,
        }
    }
}

// BMSPlayerMode: replaced by pub use from beatoraja_core (Phase 18e-5)
pub use beatoraja_core::bms_player_mode::BMSPlayerMode;
// Alias Mode as BMSPlayerModeType to avoid naming conflict with bms_model::mode::Mode
pub use beatoraja_core::bms_player_mode::Mode as BMSPlayerModeType;

// FloatArray: replaced by Vec<f32> — callers updated to use Vec directly

// IntArray: replaced by Vec<i32> — callers updated to use Vec directly (Phase 18e-4)

// Skin: replaced by pub use beatoraja_skin::skin::Skin
// SkinHeader: replaced by pub use beatoraja_skin::skin_header::SkinHeader
// Color: replaced by pub use beatoraja_skin::stubs::Color
// Rectangle: replaced by pub use beatoraja_skin::stubs::Rectangle
// SkinObjectRenderer: replaced by pub use beatoraja_skin::skin_object::SkinObjectRenderer

// TextureRegion, Texture, Pixmap: replaced by pub use beatoraja_skin::stubs::*

/// Stub for SkinObject base (partial — only what SkinGaugeGraphObject needs)
pub struct SkinObjectData {
    pub region: Rectangle,
}

// TimerManager: replaced by pub use beatoraja_core::timer_manager::TimerManager

// EventType: removed (dead code — only used in commented-out lines)

// ============================================================
// FreqTrainerMenu stub
// ============================================================

pub fn is_freq_trainer_enabled() -> bool {
    false
}

pub fn is_freq_negative() -> bool {
    false
}
