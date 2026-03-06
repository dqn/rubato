// External dependency stubs for beatoraja-result crate
// These will be replaced with actual implementations when corresponding phases are translated.

use rubato_audio::audio_driver::AudioDriver;
use rubato_types::config::Config;
use rubato_types::player_config::PlayerConfig;
use rubato_types::sound_type::SoundType;

// ============================================================
// Re-exports from real crates (Phase 11 stub replacements)
// ============================================================

pub use rubato_core::timer_manager::TimerManager;
pub use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
pub use rubato_input::key_command::KeyCommand;
pub use rubato_input::keyboard_input_processor::ControlKeys;
pub use rubato_skin::skin::Skin;
pub use rubato_skin::skin_header::SkinHeader;
pub use rubato_skin::skin_object::SkinObjectRenderer;
pub use rubato_skin::stubs::Color;
pub use rubato_skin::stubs::Pixmap;
pub use rubato_skin::stubs::PixmapFormat;
pub use rubato_skin::stubs::Rectangle;
pub use rubato_skin::stubs::Texture;
pub use rubato_skin::stubs::TextureRegion;
use rubato_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};

// PlayDataAccessor: replaced by pub use from rubato_core (Phase 18e-4)
pub use rubato_core::play_data_accessor::PlayDataAccessor;

// ============================================================
// MainController wrapper (Phase 41b — delegates to MainControllerAccess trait)
// ============================================================

// MainControllerAccess: real trait from beatoraja-types (Phase 41b)
pub use rubato_types::main_controller_access::{MainControllerAccess, NullMainController};

/// Wrapper for bms.player.beatoraja.MainController.
/// Delegates trait methods (get_config, get_player_config, change_state, save_last_recording)
/// to `Box<dyn MainControllerAccess>`.
/// Stores crate-local components whose types cannot go on the cross-crate trait:
/// AudioDriver, BMSPlayerInputProcessor, IRStatus/IRSendStatus, PlayDataAccessor,
/// RankingDataCache.
pub struct MainController {
    inner: Box<dyn MainControllerAccess>,
    audio: Option<Box<dyn AudioDriver>>,
    ir_statuses: Vec<IRStatus>,
    ir_send_statuses: std::sync::Arc<std::sync::Mutex<Vec<IRSendStatusMain>>>,
    input_processor: BMSPlayerInputProcessor,
    play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
}

impl MainController {
    pub fn new(inner: Box<dyn MainControllerAccess>) -> Self {
        let config = inner.get_config();
        let player_config = inner.get_player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .get_ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(rubato_ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            audio: None,
            ir_statuses: Vec::new(),
            ir_send_statuses: crate::result::ir_resend::shared_ir_statuses(),
            input_processor,
            play_data_accessor,
            ranking_data_cache,
        }
    }

    pub fn with_audio(inner: Box<dyn MainControllerAccess>, audio: Box<dyn AudioDriver>) -> Self {
        let config = inner.get_config();
        let player_config = inner.get_player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .get_ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(rubato_ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            audio: Some(audio),
            ir_statuses: Vec::new(),
            ir_send_statuses: crate::result::ir_resend::shared_ir_statuses(),
            input_processor,
            play_data_accessor,
            ranking_data_cache,
        }
    }

    pub fn with_ir_statuses(
        inner: Box<dyn MainControllerAccess>,
        ir_statuses: Vec<IRStatus>,
    ) -> Self {
        let config = inner.get_config();
        let player_config = inner.get_player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .get_ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(rubato_ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            audio: None,
            ir_statuses,
            ir_send_statuses: crate::result::ir_resend::shared_ir_statuses(),
            input_processor,
            play_data_accessor,
            ranking_data_cache,
        }
    }

    /// Set a pre-configured input processor (replaces the default).
    pub fn set_input_processor(&mut self, processor: BMSPlayerInputProcessor) {
        self.input_processor = processor;
    }

    /// Set a pre-configured play data accessor (replaces the default).
    pub fn set_play_data_accessor(&mut self, accessor: PlayDataAccessor) {
        self.play_data_accessor = accessor;
    }

    // ---- Trait-delegated methods ----

    pub fn get_config(&self) -> &Config {
        self.inner.get_config()
    }

    pub fn get_player_config(&self) -> &PlayerConfig {
        self.inner.get_player_config()
    }

    pub fn change_state(&mut self, state: rubato_core::main_state::MainStateType) {
        self.inner.change_state(state);
    }

    pub fn save_last_recording(&self, tag: &str) {
        self.inner.save_last_recording(tag);
    }

    pub fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.inner.play_sound(sound, loop_sound);
    }

    pub fn stop_sound(&mut self, sound: &SoundType) {
        self.inner.stop_sound(sound);
    }

    pub fn get_sound_path(&self, sound: &SoundType) -> Option<String> {
        self.inner.get_sound_path(sound)
    }

    // ---- Locally-stored components (types not on MainControllerAccess trait) ----

    pub fn get_input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        &mut self.input_processor
    }

    pub fn sync_input_from(&mut self, input: &BMSPlayerInputProcessor) {
        self.input_processor.sync_runtime_state_from(input);
    }

    pub fn sync_input_back_to(&mut self, input: &mut BMSPlayerInputProcessor) {
        input.sync_runtime_state_from(&self.input_processor);
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        &self.ir_statuses
    }

    pub fn ir_send_status(&self) -> std::sync::MutexGuard<'_, Vec<IRSendStatusMain>> {
        self.ir_send_statuses.lock().unwrap()
    }

    pub fn ir_send_status_mut(&self) -> std::sync::MutexGuard<'_, Vec<IRSendStatusMain>> {
        self.ir_send_statuses.lock().unwrap()
    }

    pub fn get_play_data_accessor(&self) -> &PlayDataAccessor {
        &self.play_data_accessor
    }

    pub fn get_audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    pub fn get_ranking_data_cache(
        &self,
    ) -> &dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess {
        &*self.ranking_data_cache
    }

    pub fn get_ranking_data_cache_mut(
        &mut self,
    ) -> &mut dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess {
        &mut *self.ranking_data_cache
    }
}

// ============================================================
// IR (Internet Ranking) — real implementations (Phase 20)
// ============================================================

// IRStatus: real type with connection, config, player (Phase 20)
pub use super::ir_status::IRStatus;

// IRConnection: real trait from rubato_ir
pub use rubato_ir::ir_connection::IRConnection;

// IRConfig: real type from rubato_core
pub use rubato_core::ir_config::IRConfig;

// IRScoreData: real type from rubato_ir
pub use rubato_ir::ir_score_data::IRScoreData;

// IRCourseData: real type from rubato_ir
pub use rubato_ir::ir_course_data::IRCourseData;

// RankingData: real type from rubato_ir
pub use rubato_ir::ranking_data::RankingData;

// ============================================================
// MainController.IRSendStatus — real implementation (Phase 20)
// ============================================================

// IRSendStatusMain: real type with send() implementation (Phase 20)
pub use super::ir_send_status::IRSendStatusMain;

// BMSPlayerInputProcessor: replaced by pub use from rubato_input (Phase 18e-9)

// GrooveGauge: replaced by real type from beatoraja-types
pub use rubato_types::groove_gauge::GrooveGauge;

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

    pub fn get_config(&self) -> &rubato_core::config::Config {
        self.inner.get_config()
    }

    pub fn get_player_config(&self) -> &rubato_core::player_config::PlayerConfig {
        self.inner.get_player_config()
    }

    pub fn get_player_config_mut(
        &mut self,
    ) -> Option<&mut rubato_core::player_config::PlayerConfig> {
        self.inner.get_player_config_mut()
    }

    pub fn get_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.inner.get_score_data()
    }

    pub fn get_score_data_mut(&mut self) -> Option<&mut rubato_core::score_data::ScoreData> {
        self.inner.get_score_data_mut()
    }

    pub fn get_target_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.inner.get_target_score_data()
    }

    pub fn get_course_score_data(&self) -> Option<&rubato_core::score_data::ScoreData> {
        self.inner.get_course_score_data()
    }

    pub fn set_course_score_data(&mut self, score: rubato_core::score_data::ScoreData) {
        self.inner.set_course_score_data(score);
    }

    pub fn get_songdata(&self) -> Option<&rubato_types::song_data::SongData> {
        self.inner.get_songdata()
    }

    pub fn get_replay_data(&self) -> Option<&rubato_core::replay_data::ReplayData> {
        self.inner.get_replay_data()
    }

    pub fn get_course_replay(&self) -> &[rubato_core::replay_data::ReplayData] {
        self.inner.get_course_replay()
    }

    pub fn get_course_replay_mut(&mut self) -> &mut Vec<rubato_core::replay_data::ReplayData> {
        self.inner.get_course_replay_mut()
    }

    pub fn add_course_replay(&mut self, replay: rubato_core::replay_data::ReplayData) {
        self.inner.add_course_replay(replay);
    }

    pub fn get_course_data(&self) -> Option<&rubato_core::course_data::CourseData> {
        self.inner.get_course_data()
    }

    pub fn get_course_index(&self) -> usize {
        self.inner.get_course_index()
    }

    pub fn next_course(&mut self) -> bool {
        self.inner.next_course()
    }

    pub fn get_constraint(&self) -> Vec<rubato_core::course_data::CourseDataConstraint> {
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

    pub fn set_bms_model(&mut self, model: bms_model::bms_model::BMSModel) {
        self.bms_model = model;
    }

    pub fn set_course_bms_models(&mut self, models: Option<Vec<bms_model::bms_model::BMSModel>>) {
        self.course_bms_models = models;
    }

    /// Take the inner PlayerResourceAccess, replacing it with a NullPlayerResource.
    /// Used during state transition to return the resource to MainController.
    pub fn take_inner(&mut self) -> Option<Box<dyn PlayerResourceAccess>> {
        let null: Box<dyn PlayerResourceAccess> = Box::new(NullPlayerResource::new());
        Some(std::mem::replace(&mut self.inner, null))
    }

    pub fn get_replay_data_mut(&mut self) -> Option<&mut rubato_core::replay_data::ReplayData> {
        self.inner.get_replay_data_mut()
    }

    pub fn reload_bms_file(&mut self) {
        self.inner.reload_bms_file();
    }

    pub fn set_player_config_gauge(&mut self, gauge: i32) {
        self.inner.set_player_config_gauge(gauge);
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

// BMSPlayerMode: replaced by pub use from rubato_core (Phase 18e-5)
pub use rubato_core::bms_player_mode::BMSPlayerMode;
// Alias Mode as BMSPlayerModeType to avoid naming conflict with bms_model::mode::Mode
pub use rubato_core::bms_player_mode::Mode as BMSPlayerModeType;

// FloatArray: replaced by Vec<f32> — callers updated to use Vec directly

// IntArray: replaced by Vec<i32> — callers updated to use Vec directly (Phase 18e-4)

// Skin: replaced by pub use rubato_skin::skin::Skin
// SkinHeader: replaced by pub use rubato_skin::skin_header::SkinHeader
// Color: replaced by pub use rubato_skin::stubs::Color
// Rectangle: replaced by pub use rubato_skin::stubs::Rectangle
// SkinObjectRenderer: replaced by pub use rubato_skin::skin_object::SkinObjectRenderer

// TextureRegion, Texture, Pixmap: replaced by pub use rubato_skin::stubs::*

// SkinObjectData: replaced by pub use from beatoraja-skin (Phase 33c)
pub use rubato_skin::skin_object::SkinObjectData;

// TimerManager: replaced by pub use rubato_core::timer_manager::TimerManager

// EventType: removed (dead code — only used in commented-out lines)

// FreqTrainerMenu: replaced with re-exports from beatoraja-modmenu (Phase 18e-6)
pub use crate::modmenu::freq_trainer_menu::FreqTrainerMenu;

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::bms_model::BMSModel;
    use bms_model::note::Note;
    use rubato_types::song_data::SongData;

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

    struct CacheBackedMainControllerAccess {
        config: Config,
        player_config: PlayerConfig,
        ranking_data_cache:
            Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
    }

    impl CacheBackedMainControllerAccess {
        fn new() -> Self {
            Self {
                config: Config::default(),
                player_config: PlayerConfig::default(),
                ranking_data_cache: Box::new(rubato_ir::ranking_data_cache::RankingDataCache::new()),
            }
        }
    }

    impl MainControllerAccess for CacheBackedMainControllerAccess {
        fn get_config(&self) -> &Config {
            &self.config
        }

        fn get_player_config(&self) -> &PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, _state: rubato_core::main_state::MainStateType) {}

        fn save_config(&self) {}

        fn exit(&self) {}

        fn save_last_recording(&self, _tag: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            None
        }

        fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            None
        }

        fn get_ranking_data_cache(
            &self,
        ) -> Option<&dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess> {
            Some(&*self.ranking_data_cache)
        }

        fn get_ranking_data_cache_mut(
            &mut self,
        ) -> Option<
            &mut (dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess + 'static),
        > {
            Some(&mut *self.ranking_data_cache)
        }
    }

    #[test]
    fn test_main_controller_reuses_inner_ranking_cache() {
        let song = SongData::default();
        let mut access = CacheBackedMainControllerAccess::new();
        access
            .get_ranking_data_cache_mut()
            .expect("test access should expose ranking cache")
            .put_song_any(&song, 0, Box::new(RankingData::new()));

        let mc = MainController::new(Box::new(access));
        let cached = mc
            .get_ranking_data_cache()
            .get_song_any(&song, 0)
            .and_then(|any| any.downcast::<RankingData>().ok())
            .map(|ranking| *ranking);

        assert!(
            cached.is_some(),
            "result wrapper should reuse the ranking cache exposed by its inner controller access"
        );
    }
}
