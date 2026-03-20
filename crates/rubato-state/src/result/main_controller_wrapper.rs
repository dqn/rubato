use rubato_audio::audio_driver::AudioDriver;
use rubato_core::play_data_accessor::PlayDataAccessor;
use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
use rubato_types::config::Config;
use rubato_types::player_config::PlayerConfig;
use rubato_types::sound_type::SoundType;

use super::ir_send_status::IRSendStatusMain;
use super::ir_status::IRStatus;
use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::sync_utils::lock_or_recover;

/// Wrapper for bms.player.beatoraja.MainController.
/// Delegates trait methods (config, player_config, change_state, save_last_recording)
/// to `Box<dyn MainControllerAccess>`.
/// Stores crate-local components whose types cannot go on the cross-crate trait:
/// AudioDriver, BMSPlayerInputProcessor, IRStatus/IRSendStatus, PlayDataAccessor,
/// RankingDataCache.
pub struct MainController {
    inner: Box<dyn MainControllerAccess>,
    audio: Option<Box<dyn AudioDriver>>,
    ir_statuses: Vec<IRStatus>,
    ir_send_statuses: std::sync::Arc<std::sync::Mutex<Vec<IRSendStatusMain>>>,
    pub input_processor: BMSPlayerInputProcessor,
    pub play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
}

impl MainController {
    pub fn new(inner: Box<dyn MainControllerAccess>) -> Self {
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
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
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
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
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
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

    pub fn with_audio_and_ir(
        inner: Box<dyn MainControllerAccess>,
        audio: Box<dyn AudioDriver>,
        ir_statuses: Vec<IRStatus>,
    ) -> Self {
        let config = inner.config();
        let player_config = inner.player_config();
        let input_processor = BMSPlayerInputProcessor::new(config, player_config);
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(rubato_ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            audio: Some(audio),
            ir_statuses,
            ir_send_statuses: crate::result::ir_resend::shared_ir_statuses(),
            input_processor,
            play_data_accessor,
            ranking_data_cache,
        }
    }

    // ---- Trait-delegated methods ----

    pub fn config(&self) -> &Config {
        self.inner.config()
    }

    pub fn player_config(&self) -> &PlayerConfig {
        self.inner.player_config()
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

    pub fn sound_path(&self, sound: &SoundType) -> Option<String> {
        self.inner.sound_path(sound)
    }

    pub fn update_audio_config(&self, audio: rubato_types::audio_config::AudioConfig) {
        self.inner.update_audio_config(audio);
    }

    pub fn offset_value(&self, id: i32) -> Option<&rubato_types::skin_offset::SkinOffset> {
        self.inner.offset_value(id)
    }

    // ---- Locally-stored components (types not on MainControllerAccess trait) ----

    pub fn input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        &mut self.input_processor
    }

    pub fn sync_input_from(&mut self, input: &BMSPlayerInputProcessor) {
        self.input_processor.sync_runtime_state_from(input);
    }

    pub fn sync_input_back_to(&mut self, input: &mut BMSPlayerInputProcessor) {
        input.sync_runtime_state_from(&self.input_processor);
    }

    pub fn ir_status(&self) -> &[IRStatus] {
        &self.ir_statuses
    }

    pub fn ir_send_status(&self) -> std::sync::MutexGuard<'_, Vec<IRSendStatusMain>> {
        lock_or_recover(&self.ir_send_statuses)
    }

    pub fn ir_send_status_mut(&self) -> std::sync::MutexGuard<'_, Vec<IRSendStatusMain>> {
        lock_or_recover(&self.ir_send_statuses)
    }

    pub fn play_data_accessor(&self) -> &PlayDataAccessor {
        &self.play_data_accessor
    }

    pub fn audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    pub fn ranking_data_cache(
        &self,
    ) -> &dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess {
        &*self.ranking_data_cache
    }

    pub fn ranking_data_cache_mut(
        &mut self,
    ) -> &mut dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess {
        &mut *self.ranking_data_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_audio::recording_audio_driver::RecordingAudioDriver;
    use rubato_ir::ranking_data::RankingData;
    use rubato_types::main_controller_access::NullMainController;
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use rubato_types::song_data::SongData;

    #[test]
    fn test_main_controller_new_has_no_audio() {
        let mut mc = MainController::new(Box::new(NullMainController));
        assert!(mc.audio_processor_mut().is_none());
    }

    #[test]
    fn test_main_controller_with_audio_has_audio() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
        );
        assert!(mc.audio_processor_mut().is_some());
    }

    #[test]
    fn test_main_controller_audio_stop_note() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
        );
        if let Some(audio) = mc.audio_processor_mut() {
            audio.stop_note(None);
        }
        // Verify the call went through (cannot inspect mock after borrow, but no panic = pass)
    }

    #[test]
    fn test_main_controller_audio_set_global_pitch() {
        let mut mc = MainController::with_audio(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
        );
        if let Some(audio) = mc.audio_processor_mut() {
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
        fn config(&self) -> &Config {
            &self.config
        }

        fn player_config(&self) -> &PlayerConfig {
            &self.player_config
        }

        fn change_state(&mut self, _state: rubato_core::main_state::MainStateType) {}

        fn save_config(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn exit(&self) -> anyhow::Result<()> {
            Ok(())
        }

        fn save_last_recording(&self, _tag: &str) {}

        fn update_song(&mut self, _path: Option<&str>) {}

        fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            None
        }

        fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
            None
        }

        fn ranking_data_cache(
            &self,
        ) -> Option<&dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess> {
            Some(&*self.ranking_data_cache)
        }

        fn ranking_data_cache_mut(
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
            .ranking_data_cache_mut()
            .expect("test access should expose ranking cache")
            .put_song_any(&song, 0, Box::new(RankingData::new()));

        let mc = MainController::new(Box::new(access));
        let cached = mc
            .ranking_data_cache()
            .song_any(&song, 0)
            .and_then(|any| any.downcast::<RankingData>().ok())
            .map(|ranking| *ranking);

        assert!(
            cached.is_some(),
            "result wrapper should reuse the ranking cache exposed by its inner controller access"
        );
    }

    #[test]
    fn test_with_audio_and_ir_has_both_audio_and_ir_statuses() {
        use rubato_ir::ir_player_data::IRPlayerData;
        use std::sync::Arc;

        struct MockIRConnection;
        impl rubato_ir::ir_connection::IRConnection for MockIRConnection {
            fn get_rivals(&self) -> rubato_ir::ir_response::IRResponse<Vec<IRPlayerData>> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_table_datas(
                &self,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_table_data::IRTableData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _chart: &rubato_ir::ir_chart_data::IRChartData,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_course_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _course: &rubato_ir::ir_course_data::IRCourseData,
            ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
            {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_play_data(
                &self,
                _model: &rubato_ir::ir_chart_data::IRChartData,
                _score: &rubato_ir::ir_score_data::IRScoreData,
            ) -> rubato_ir::ir_response::IRResponse<()> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_course_play_data(
                &self,
                _course: &rubato_ir::ir_course_data::IRCourseData,
                _score: &rubato_ir::ir_score_data::IRScoreData,
            ) -> rubato_ir::ir_response::IRResponse<()> {
                rubato_ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_song_url(
                &self,
                _chart: &rubato_ir::ir_chart_data::IRChartData,
            ) -> Option<String> {
                None
            }
            fn get_course_url(
                &self,
                _course: &rubato_ir::ir_course_data::IRCourseData,
            ) -> Option<String> {
                None
            }
            fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
                None
            }
            fn name(&self) -> &str {
                "MockIR"
            }
        }

        let ir_statuses = vec![IRStatus::new(
            rubato_core::ir_config::IRConfig::default(),
            Arc::new(MockIRConnection)
                as Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync>,
            IRPlayerData::new("id1".into(), "Player1".into(), "Rank1".into()),
        )];

        let mut mc = MainController::with_audio_and_ir(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
            ir_statuses,
        );

        assert!(
            mc.audio_processor_mut().is_some(),
            "with_audio_and_ir should provide audio"
        );
        assert_eq!(
            mc.ir_status().len(),
            1,
            "with_audio_and_ir should carry IR statuses"
        );
        assert_eq!(mc.ir_status()[0].player.id, "id1");
    }

    #[test]
    fn test_with_audio_and_ir_empty_ir_still_has_audio() {
        let mut mc = MainController::with_audio_and_ir(
            Box::new(NullMainController),
            Box::new(RecordingAudioDriver::new()),
            Vec::new(),
        );

        assert!(mc.audio_processor_mut().is_some());
        assert!(mc.ir_status().is_empty());
    }
}
