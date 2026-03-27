use crate::core::play_data_accessor::PlayDataAccessor;
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
/// AudioDriver, IRStatus/IRSendStatus, PlayDataAccessor, RankingDataCache.
pub struct MainController {
    inner: Box<dyn MainControllerAccess>,
    ir_statuses: Vec<IRStatus>,
    ir_send_statuses: std::sync::Arc<std::sync::Mutex<Vec<IRSendStatusMain>>>,
    pub play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
}

impl MainController {
    pub fn new(inner: Box<dyn MainControllerAccess>) -> Self {
        let config = inner.config();
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(crate::ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            ir_statuses: Vec::new(),
            ir_send_statuses: crate::state::result::ir_resend::shared_ir_statuses(),
            play_data_accessor,
            ranking_data_cache,
        }
    }

    pub fn with_ir_statuses(
        inner: Box<dyn MainControllerAccess>,
        ir_statuses: Vec<IRStatus>,
    ) -> Self {
        let config = inner.config();
        let play_data_accessor = PlayDataAccessor::new(config);
        let ranking_data_cache = inner
            .ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(crate::ir::ranking_data_cache::RankingDataCache::new()));
        Self {
            inner,
            ir_statuses,
            ir_send_statuses: crate::state::result::ir_resend::shared_ir_statuses(),
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

    pub fn change_state(&mut self, state: crate::core::main_state::MainStateType) {
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

    pub fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        self.inner.play_audio_path(path, volume, loop_play);
    }

    pub fn stop_audio_path(&mut self, path: &str) {
        self.inner.stop_audio_path(path);
    }

    // ---- Locally-stored components (types not on MainControllerAccess trait) ----

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
    use crate::ir::ranking_data::RankingData;
    use rubato_types::main_controller_access::NullMainController;
    use rubato_types::player_resource_access::PlayerResourceAccess;
    use rubato_types::song_data::SongData;

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
                ranking_data_cache: Box::new(crate::ir::ranking_data_cache::RankingDataCache::new()),
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

        fn change_state(&mut self, _state: crate::core::main_state::MainStateType) {}

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
    fn test_with_ir_statuses_carries_ir_statuses() {
        use crate::ir::ir_player_data::IRPlayerData;
        use std::sync::Arc;

        struct MockIRConnection;
        impl crate::ir::ir_connection::IRConnection for MockIRConnection {
            fn get_rivals(&self) -> crate::ir::ir_response::IRResponse<Vec<IRPlayerData>> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_table_datas(
                &self,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_table_data::IRTableData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _chart: Option<&crate::ir::ir_chart_data::IRChartData>,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_course_play_data(
                &self,
                _player: Option<&IRPlayerData>,
                _course: &crate::ir::ir_course_data::IRCourseData,
            ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
            {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_play_data(
                &self,
                _model: &crate::ir::ir_chart_data::IRChartData,
                _score: &crate::ir::ir_score_data::IRScoreData,
            ) -> crate::ir::ir_response::IRResponse<()> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn send_course_play_data(
                &self,
                _course: &crate::ir::ir_course_data::IRCourseData,
                _score: &crate::ir::ir_score_data::IRScoreData,
            ) -> crate::ir::ir_response::IRResponse<()> {
                crate::ir::ir_response::IRResponse::failure("mock".to_string())
            }
            fn get_song_url(
                &self,
                _chart: &crate::ir::ir_chart_data::IRChartData,
            ) -> Option<String> {
                None
            }
            fn get_course_url(
                &self,
                _course: &crate::ir::ir_course_data::IRCourseData,
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
            crate::core::ir_config::IRConfig::default(),
            Arc::new(MockIRConnection)
                as Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync>,
            IRPlayerData::new("id1".into(), "Player1".into(), "Rank1".into()),
        )];

        let mc = MainController::with_ir_statuses(Box::new(NullMainController), ir_statuses);

        assert_eq!(
            mc.ir_status().len(),
            1,
            "with_ir_statuses should carry IR statuses"
        );
        assert_eq!(mc.ir_status()[0].player.id, "id1");
    }

    #[test]
    fn test_with_ir_statuses_empty_ir() {
        let mc = MainController::with_ir_statuses(Box::new(NullMainController), Vec::new());

        assert!(mc.ir_status().is_empty());
    }
}
