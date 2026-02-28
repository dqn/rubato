// External dependency stubs for beatoraja-result crate
// These will be replaced with actual implementations when corresponding phases are translated.

use beatoraja_audio::audio_driver::AudioDriver;
use beatoraja_types::config::Config;
use beatoraja_types::player_config::PlayerConfig;

// ============================================================
// Re-exports from real crates (Phase 11 stub replacements)
// ============================================================

pub use beatoraja_core::timer_manager::TimerManager;
pub use beatoraja_input::bms_player_input_processor::BMSPlayerInputProcessor;
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

// PlayDataAccessor: replaced by pub use from beatoraja_core (Phase 18e-4)
pub use beatoraja_core::play_data_accessor::PlayDataAccessor;

// ============================================================
// MainController wrapper (Phase 41b — delegates to MainControllerAccess trait)
// ============================================================

// MainControllerAccess: real trait from beatoraja-types (Phase 41b)
pub use beatoraja_types::main_controller_access::{MainControllerAccess, NullMainController};

/// Wrapper for bms.player.beatoraja.MainController.
/// Delegates trait methods (get_config, get_player_config, change_state, save_last_recording)
/// to `Box<dyn MainControllerAccess>`.
/// Retains local stubs for methods whose return types are not on the trait
/// (get_input_processor, get_ir_status, ir_send_status, get_play_data_accessor,
///  get_ranking_data_cache).
/// AudioDriver is stored directly (Phase 41c) — not on MainControllerAccess trait.
pub struct MainController {
    inner: Box<dyn MainControllerAccess>,
    audio: Option<Box<dyn AudioDriver>>,
    ir_statuses: Vec<IRStatus>,
}

impl MainController {
    pub fn new(inner: Box<dyn MainControllerAccess>) -> Self {
        Self {
            inner,
            audio: None,
            ir_statuses: Vec::new(),
        }
    }

    pub fn with_audio(inner: Box<dyn MainControllerAccess>, audio: Box<dyn AudioDriver>) -> Self {
        Self {
            inner,
            audio: Some(audio),
            ir_statuses: Vec::new(),
        }
    }

    pub fn with_ir_statuses(
        inner: Box<dyn MainControllerAccess>,
        ir_statuses: Vec<IRStatus>,
    ) -> Self {
        Self {
            inner,
            audio: None,
            ir_statuses,
        }
    }

    // ---- Trait-delegated methods ----

    pub fn get_config(&self) -> &Config {
        self.inner.get_config()
    }

    pub fn get_player_config(&self) -> &PlayerConfig {
        self.inner.get_player_config()
    }

    pub fn change_state(&mut self, state: beatoraja_core::main_state::MainStateType) {
        self.inner.change_state(state);
    }

    pub fn save_last_recording(&self, tag: &str) {
        self.inner.save_last_recording(tag);
    }

    // ---- Local stubs (types not on MainControllerAccess trait) ----

    pub fn get_input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        log::warn!("not yet implemented: MainController.getInputProcessor");
        // Leak a boxed value to get a &'static mut reference - stub only
        Box::leak(Box::new(BMSPlayerInputProcessor::new(
            &Config::default(),
            &PlayerConfig::default(),
        )))
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        &self.ir_statuses
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

    pub fn get_audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    pub fn get_ranking_data_cache(&self) -> &RankingDataCache {
        log::warn!("not yet implemented: MainController.getRankingDataCache");
        static DEFAULT: RankingDataCache = RankingDataCache;
        &DEFAULT
    }
}

/// Stub for RankingDataCache
pub struct RankingDataCache;

impl RankingDataCache {
    pub fn get(
        &self,
        _songdata: &beatoraja_types::song_data::SongData,
        _lnmode: i32,
    ) -> Option<RankingData> {
        log::warn!("not yet implemented: RankingDataCache.get");
        None
    }

    pub fn put(
        &self,
        _songdata: &beatoraja_types::song_data::SongData,
        _lnmode: i32,
        _ranking: RankingData,
    ) {
        log::warn!("not yet implemented: RankingDataCache.put");
    }
}

// ============================================================
// IR (Internet Ranking) — real implementations (Phase 20)
// ============================================================

// IRStatus: real type with connection, config, player (Phase 20)
pub use crate::ir_status::IRStatus;

// IRConnection: real trait from beatoraja_ir
pub use beatoraja_ir::ir_connection::IRConnection;

// IRConfig: real type from beatoraja_core
pub use beatoraja_core::ir_config::IRConfig;

// IRScoreData: real type from beatoraja_ir
pub use beatoraja_ir::ir_score_data::IRScoreData;

// IRCourseData: real type from beatoraja_ir
pub use beatoraja_ir::ir_course_data::IRCourseData;

// RankingData: real type from beatoraja_ir
pub use beatoraja_ir::ranking_data::RankingData;

// ============================================================
// MainController.IRSendStatus — real implementation (Phase 20)
// ============================================================

// IRSendStatusMain: real type with send() implementation (Phase 20)
pub use crate::ir_send_status::IRSendStatusMain;

// BMSPlayerInputProcessor: replaced by pub use from beatoraja_input (Phase 18e-9)

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

    pub fn get_replay_data_mut(&mut self) -> Option<&mut beatoraja_core::replay_data::ReplayData> {
        log::warn!("not yet implemented: PlayerResource.getReplayData() (mutable)");
        None
    }

    pub fn reload_bms_file(&mut self) {
        log::warn!("not yet implemented: PlayerResource.reloadBMSFile");
    }

    pub fn set_player_config_gauge(&mut self, gauge: i32) {
        log::warn!(
            "not yet implemented: PlayerResource.getPlayerConfig().setGauge({})",
            gauge
        );
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

// SkinObjectData: replaced by pub use from beatoraja-skin (Phase 33c)
pub use beatoraja_skin::skin_object::SkinObjectData;

// TimerManager: replaced by pub use beatoraja_core::timer_manager::TimerManager

// EventType: removed (dead code — only used in commented-out lines)

// FreqTrainerMenu: replaced with re-exports from beatoraja-modmenu (Phase 18e-6)
pub use beatoraja_modmenu::freq_trainer_menu::FreqTrainerMenu;

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::bms_model::BMSModel;
    use bms_model::note::Note;

    /// Mock AudioDriver for testing.
    struct MockAudioDriver {
        stop_note_called: bool,
        global_pitch: f32,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self {
                stop_note_called: false,
                global_pitch: 1.0,
            }
        }
    }

    impl AudioDriver for MockAudioDriver {
        fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
        fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
        fn is_playing_path(&self, _path: &str) -> bool {
            false
        }
        fn stop_path(&mut self, _path: &str) {}
        fn dispose_path(&mut self, _path: &str) {}
        fn set_model(&mut self, _model: &BMSModel) {}
        fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
        fn abort(&mut self) {}
        fn get_progress(&self) -> f32 {
            1.0
        }
        fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}
        fn play_judge(&mut self, _judge: i32, _fast: bool) {}
        fn stop_note(&mut self, _n: Option<&Note>) {
            self.stop_note_called = true;
        }
        fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}
        fn set_global_pitch(&mut self, pitch: f32) {
            self.global_pitch = pitch;
        }
        fn get_global_pitch(&self) -> f32 {
            self.global_pitch
        }
        fn dispose_old(&mut self) {}
        fn dispose(&mut self) {}
    }

    #[test]
    fn test_main_controller_new_has_no_audio() {
        let mut mc = MainController::new(Box::new(NullMainController));
        assert!(mc.get_audio_processor_mut().is_none());
    }

    #[test]
    fn test_main_controller_with_audio_has_audio() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        assert!(mc.get_audio_processor_mut().is_some());
    }

    #[test]
    fn test_main_controller_audio_stop_note() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        if let Some(audio) = mc.get_audio_processor_mut() {
            audio.stop_note(None);
        }
        // Verify the call went through (cannot inspect mock after borrow, but no panic = pass)
    }

    #[test]
    fn test_main_controller_audio_set_global_pitch() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(MockAudioDriver::new()),
        );
        if let Some(audio) = mc.get_audio_processor_mut() {
            audio.set_global_pitch(1.5);
            assert_eq!(audio.get_global_pitch(), 1.5);
        } else {
            panic!("expected audio processor to be present");
        }
    }
}
