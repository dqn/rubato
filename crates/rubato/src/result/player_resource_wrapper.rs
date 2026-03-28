use crate::ir::ranking_data::RankingData;
use rubato_skin::groove_gauge::GrooveGauge;
use rubato_skin::player_resource_access::{
    ConfigAccess, GaugeAccess, PlayerStateAccess, ReplayAccess, ScoreAccess, SessionMutation,
};

use crate::core::bms_player_mode::BMSPlayerMode;
use crate::core::bms_player_mode::Mode as BMSPlayerModeType;
use crate::core::player_resource::PlayerResource as CorePlayerResource;

/// Wrapper for bms.player.beatoraja.PlayerResource.
/// Delegates to concrete `CorePlayerResource` for trait methods.
/// Crate-local methods provide access to non-trait types (BMSModel, BMSPlayerMode, RankingData).
///
/// NOTE: `bms_model` is a snapshot taken at construction time. After `next_course()` updates the
/// inner PlayerResource's model, this local field becomes stale. Use `inner.songdata()` for
/// post-next_course data; the local `bms_model` must not be relied on after course advancement.
pub struct PlayerResource {
    inner: CorePlayerResource,
    pub bms_model: bms::model::bms_model::BMSModel,
    pub course_bms_models: Option<Vec<bms::model::bms_model::BMSModel>>,
    play_mode: BMSPlayerMode,
    pub ranking_data: Option<RankingData>,
}

impl PlayerResource {
    pub fn new(inner: CorePlayerResource, play_mode: BMSPlayerMode) -> Self {
        Self {
            inner,
            bms_model: bms::model::bms_model::BMSModel::default(),
            course_bms_models: None,
            play_mode,
            ranking_data: None,
        }
    }

    // ---- Trait-delegated methods ----

    pub fn config(&self) -> &crate::core::config::Config {
        self.inner.config()
    }

    pub fn player_config(&self) -> &crate::core::player_config::PlayerConfig {
        self.inner.player_config()
    }

    pub fn player_config_mut(&mut self) -> Option<&mut crate::core::player_config::PlayerConfig> {
        self.inner.player_config_mut()
    }

    pub fn score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.inner.score_data()
    }

    pub fn score_data_mut(&mut self) -> Option<&mut crate::core::score_data::ScoreData> {
        self.inner.score_data_mut()
    }

    pub fn target_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.inner.target_score_data()
    }

    pub fn course_score_data(&self) -> Option<&crate::core::score_data::ScoreData> {
        self.inner.course_score_data()
    }

    pub fn set_course_score_data(&mut self, score: crate::core::score_data::ScoreData) {
        self.inner.set_course_score_data(score);
    }

    pub fn songdata(&self) -> Option<&rubato_skin::song_data::SongData> {
        self.inner.songdata()
    }

    pub fn replay_data(&self) -> Option<&crate::core::replay_data::ReplayData> {
        self.inner.replay_data()
    }

    pub fn course_replay(&self) -> &[crate::core::replay_data::ReplayData] {
        self.inner.course_replay()
    }

    pub fn course_replay_mut(&mut self) -> &mut Vec<crate::core::replay_data::ReplayData> {
        self.inner.course_replay_mut()
    }

    pub fn add_course_replay(&mut self, replay: crate::core::replay_data::ReplayData) {
        self.inner.add_course_replay(replay);
    }

    pub fn course_data(&self) -> Option<&crate::core::course_data::CourseData> {
        self.inner.course_data()
    }

    pub fn course_index(&self) -> usize {
        self.inner.course_index()
    }

    pub fn next_course(&mut self) -> bool {
        self.inner.next_course()
    }

    pub fn constraint(&self) -> Vec<crate::core::course_data::CourseDataConstraint> {
        self.inner.constraint()
    }

    pub fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
        self.inner.gauge()
    }

    pub fn groove_gauge(&self) -> Option<&GrooveGauge> {
        self.inner.groove_gauge()
    }

    pub fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        self.inner.course_gauge()
    }

    pub fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        self.inner.course_gauge_mut()
    }

    pub fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
        self.inner.add_course_gauge(gauge);
    }

    pub fn maxcombo(&self) -> i32 {
        self.inner.maxcombo()
    }

    pub fn org_gauge_option(&self) -> i32 {
        self.inner.org_gauge_option()
    }

    pub fn assist(&self) -> i32 {
        self.inner.assist()
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

    pub fn recent_judges(&self) -> &[i64] {
        self.inner.recent_judges()
    }

    pub fn recent_judges_index(&self) -> usize {
        self.inner.recent_judges_index()
    }

    pub fn player_data(&self) -> &rubato_skin::player_data::PlayerData {
        self.inner.player_data()
    }

    // ---- Crate-local methods (not on trait -- types cause circular deps) ----

    pub fn bms_model(&self) -> &bms::model::bms_model::BMSModel {
        // Verify local field and inner trait agree when both are available.
        debug_assert!(
            self.inner
                .bms_model()
                .is_none_or(|inner_model| { inner_model.sha256 == self.bms_model.sha256 }),
            "PlayerResource: local bms_model sha256 ({}) diverges from inner ({})",
            self.bms_model.sha256,
            self.inner
                .bms_model()
                .map_or("<none>", |m| m.sha256.as_str()),
        );
        &self.bms_model
    }

    pub fn course_bms_models(&self) -> Option<&[bms::model::bms_model::BMSModel]> {
        self.course_bms_models.as_deref()
    }

    pub fn play_mode(&self) -> &BMSPlayerMode {
        &self.play_mode
    }

    pub fn ranking_data(&self) -> Option<&RankingData> {
        self.ranking_data.as_ref()
    }

    /// Take the inner CorePlayerResource, replacing it with a default.
    /// Used during state transition to return the resource to MainController.
    pub fn take_inner(&mut self) -> Option<CorePlayerResource> {
        let default = CorePlayerResource::new(
            rubato_skin::config::Config::default(),
            rubato_skin::player_config::PlayerConfig::default(),
        );
        Some(std::mem::replace(&mut self.inner, default))
    }

    pub fn replay_data_mut(&mut self) -> Option<&mut crate::core::replay_data::ReplayData> {
        self.inner.replay_data_mut()
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
            inner: CorePlayerResource::new(
                rubato_skin::config::Config::default(),
                rubato_skin::player_config::PlayerConfig::default(),
            ),
            bms_model: bms::model::bms_model::BMSModel::default(),
            course_bms_models: None,
            play_mode: BMSPlayerMode::new(BMSPlayerModeType::Play),
            ranking_data: None,
        }
    }
}
