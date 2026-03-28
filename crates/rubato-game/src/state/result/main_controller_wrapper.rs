use crate::core::play_data_accessor::PlayDataAccessor;
use crate::core::system_sound_manager::SoundType;
use rubato_types::config::Config;

use super::ir_send_status::IRSendStatusMain;
use super::ir_status::IRStatus;
use rubato_types::sync_utils::lock_or_recover;

/// Wrapper for bms.player.beatoraja.MainController.
/// Holds a cloned `Config` and locally-stored components:
/// IRStatus/IRSendStatus, PlayDataAccessor, RankingDataCache, sound_paths.
/// Audio/state-change/recording side-effects are handled via pending outbox
/// fields on the owning state (MusicResult/CourseResult), not through this wrapper.
pub struct MainController {
    config: Config,
    ir_statuses: Vec<IRStatus>,
    ir_send_statuses: std::sync::Arc<std::sync::Mutex<Vec<IRSendStatusMain>>>,
    pub play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn crate::ranking_data_cache_access::RankingDataCacheAccess>,
    /// Pre-resolved sound paths for `has_sound` / `select_course_sound` lookups.
    sound_paths: std::collections::HashMap<SoundType, String>,
}

impl MainController {
    pub fn new(
        config: Config,
        ranking_data_cache: Box<
            dyn crate::ranking_data_cache_access::RankingDataCacheAccess,
        >,
    ) -> Self {
        let play_data_accessor = PlayDataAccessor::new(&config);
        Self {
            config,
            ir_statuses: Vec::new(),
            ir_send_statuses: crate::state::result::ir_resend::shared_ir_statuses(),
            play_data_accessor,
            ranking_data_cache,
            sound_paths: std::collections::HashMap::new(),
        }
    }

    pub fn with_ir_statuses(
        config: Config,
        ranking_data_cache: Box<
            dyn crate::ranking_data_cache_access::RankingDataCacheAccess,
        >,
        ir_statuses: Vec<IRStatus>,
    ) -> Self {
        let play_data_accessor = PlayDataAccessor::new(&config);
        Self {
            config,
            ir_statuses,
            ir_send_statuses: crate::state::result::ir_resend::shared_ir_statuses(),
            play_data_accessor,
            ranking_data_cache,
            sound_paths: std::collections::HashMap::new(),
        }
    }

    // ---- Config access ----

    pub fn config(&self) -> &Config {
        &self.config
    }

    /// Look up a pre-resolved sound path by SoundType.
    pub fn sound_path(&self, sound: &SoundType) -> Option<&String> {
        self.sound_paths.get(sound)
    }

    /// Populate sound paths from the controller's SystemSoundManager snapshot.
    pub fn set_sound_paths(&mut self, paths: std::collections::HashMap<SoundType, String>) {
        self.sound_paths = paths;
    }

    // ---- Locally-stored components ----

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
    ) -> &dyn crate::ranking_data_cache_access::RankingDataCacheAccess {
        &*self.ranking_data_cache
    }

    pub fn ranking_data_cache_mut(
        &mut self,
    ) -> &mut dyn crate::ranking_data_cache_access::RankingDataCacheAccess {
        &mut *self.ranking_data_cache
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::ranking_data::RankingData;
    use rubato_types::song_data::SongData;

    fn make_ranking_cache()
    -> Box<dyn crate::ranking_data_cache_access::RankingDataCacheAccess> {
        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
    }

    #[test]
    fn test_main_controller_reuses_ranking_cache() {
        let song = SongData::default();
        let mut cache = make_ranking_cache();
        cache.put_song_any(&song, 0, Box::new(RankingData::new()));

        let mc = MainController::new(Config::default(), cache);
        let cached = mc
            .ranking_data_cache()
            .song_any(&song, 0)
            .and_then(|any| any.downcast::<RankingData>().ok())
            .map(|ranking| *ranking);

        assert!(
            cached.is_some(),
            "result wrapper should reuse the ranking cache passed at construction"
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

        let mc =
            MainController::with_ir_statuses(Config::default(), make_ranking_cache(), ir_statuses);

        assert_eq!(
            mc.ir_status().len(),
            1,
            "with_ir_statuses should carry IR statuses"
        );
        assert_eq!(mc.ir_status()[0].player.id, "id1");
    }

    #[test]
    fn test_with_ir_statuses_empty_ir() {
        let mc =
            MainController::with_ir_statuses(Config::default(), make_ranking_cache(), Vec::new());

        assert!(mc.ir_status().is_empty());
    }

    #[test]
    fn test_sound_path_lookup() {
        let mut mc = MainController::new(Config::default(), make_ranking_cache());
        let mut paths = std::collections::HashMap::new();
        paths.insert(SoundType::ResultClear, "/audio/clear.wav".to_string());
        mc.set_sound_paths(paths);

        assert_eq!(
            mc.sound_path(&SoundType::ResultClear),
            Some(&"/audio/clear.wav".to_string())
        );
        assert!(mc.sound_path(&SoundType::ResultFail).is_none());
    }
}
