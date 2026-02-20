// PlayerResource — shared data container passed between states.
//
// Contains the currently loaded BMS model, score data, and play settings.
// Used to pass data between Decide -> Play -> Result states.

use std::path::PathBuf;

use bms_config::PlayerConfig;
use bms_database::{CourseData, CourseDataConstraint};
use bms_ir::RankingData;
use bms_model::{BmsDecoder, BmsModel, PlayMode};
use bms_replay::replay_data::ReplayData;
use bms_rule::ScoreData;

/// Player mode determined by CLI arguments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PlayerMode {
    /// Normal play mode (default).
    #[default]
    Play,
    /// Practice mode with looping.
    Practice,
    /// Autoplay mode (no score save).
    Autoplay,
    /// Replay mode with slot index (0-3).
    Replay(u8),
}

/// Data shared across game states during a play session.
#[derive(Debug, Clone)]
pub struct PlayerResource {
    /// Currently loaded BMS chart (None if nothing is loaded).
    pub bms_model: Option<BmsModel>,
    /// Score data for the current play session.
    pub score_data: ScoreData,
    /// Active play mode.
    pub play_mode: PlayMode,
    /// Player configuration snapshot.
    #[allow(dead_code)] // Used in tests
    pub player_config: PlayerConfig,
    /// Original gauge option (saved at Decide, may be modified during Play).
    pub org_gauge_option: i32,
    /// BMS file's parent directory (for resolving WAV paths).
    pub bms_dir: Option<PathBuf>,
    /// Path to the BMS file (for practice mode reload).
    pub bms_path: Option<PathBuf>,
    /// Whether the next play session is practice mode.
    pub is_practice: bool,
    /// Player mode from CLI arguments (Play/Practice/Autoplay/Replay).
    pub player_mode: PlayerMode,

    // --- Play result fields (populated by PlayState shutdown) ---
    /// Gauge log: per-gauge-type values recorded every 500ms during play.
    pub gauge_log: Vec<Vec<f32>>,
    /// Maximum combo achieved.
    pub maxcombo: i32,
    /// Whether this score should be saved (false for autoplay/replay).
    pub update_score: bool,
    /// Assist option flags.
    pub assist: i32,

    // --- Result state fields ---
    /// Previous best score from DB (loaded in ResultState::create).
    pub oldscore: ScoreData,
    /// Accumulated course scores per stage (None when not in course mode).
    pub course_score_data: Option<Vec<ScoreData>>,
    /// Accumulated course replays.
    pub course_replays: Vec<ReplayData>,
    /// Accumulated course gauge logs.
    pub course_gauges: Vec<Vec<f32>>,
    /// Current play's replay data.
    pub replay_data: Option<ReplayData>,
    /// Flag set by KeyConfig/SkinConfig shutdown to request config file save.
    pub config_save_requested: bool,
    /// IR ranking data for the current chart (populated by IR query).
    pub ranking_data: Option<RankingData>,
    /// Target/rival EX score for result comparison (populated by PlayState).
    pub target_exscore: Option<i32>,

    // --- Course mode fields ---
    /// BMS models for each stage of a course (None when not in course mode).
    pub course_bms_models: Option<Vec<BmsModel>>,
    /// BMS file directories for each course stage.
    pub course_bms_dirs: Vec<PathBuf>,
    /// Current stage index within the course (0-based).
    pub course_index: usize,
    /// Course data for the current course play (None when not in course mode).
    pub course_data: Option<CourseData>,
    /// Last gauge value from previous course stage (for carry-over).
    pub course_gauge_carry: Option<f32>,
    /// Grade-specific constraints (e.g., NoSpeed, NoGood, mirror-only).
    ///
    /// Populated when selecting a Grade bar; used by PlayState to enforce
    /// course constraints during play.
    pub course_constraints: Vec<CourseDataConstraint>,

    // --- Ghost battle fields ---
    /// Ghost battle settings for pattern sharing (set by LeaderBoardBar, consumed by PlayState).
    pub ghost_battle: Option<GhostBattleSettings>,

    // --- Trainer fields (M1-M3) ---
    /// Frequency trainer: playback speed percentage (100 = normal). 0 means disabled.
    pub freq_trainer_freq: i32,
    /// Random trainer: whether a fixed lane order should be applied.
    pub random_trainer_enabled: bool,
    /// Random trainer: fixed 1P lane order (1-indexed, 7 keys).
    pub random_trainer_lane_order: [u8; 7],
    /// Judge trainer: whether judge rank override is active.
    pub judge_trainer_active: bool,
    /// Judge trainer: judge rank to override (EASY=0, NORMAL=1, HARD=2, VERY_HARD=3).
    pub judge_trainer_rank: i32,

    // --- IR send suppression ---
    /// When true, IR submission is suppressed (e.g. freq trainer active).
    pub force_no_ir_send: bool,

    // --- E2E / headless testing ---
    /// Whether to exit the app after the Result screen completes (from --exit-after-result CLI flag).
    pub exit_after_result: bool,
    /// Whether the app should exit (set by ResultState shutdown when exit_after_result is true).
    pub request_app_exit: bool,
}

/// Settings for ghost battle pattern sharing.
///
/// When a ghost battle starts, the leader board provides the random seed and lane
/// sequence so that both players share the same note pattern.
#[derive(Debug, Clone)]
pub struct GhostBattleSettings {
    /// RNG seed for pattern generation (same seed as the opponent's play).
    pub random_seed: i64,
    /// Lane sequence identifier used by the pattern modifier.
    ///
    /// Encoded as a decimal integer where each digit represents a lane (1-7),
    /// e.g. 1234567 for default, 7654321 for mirror.
    /// Set from IR ghost data's `lane_order`.
    pub lane_sequence: i32,
}

impl PlayerResource {
    /// Re-decode the BMS file from `bms_path` and replace `bms_model`.
    /// Used by practice mode to get a fresh model for each loop iteration.
    pub fn reload_bms(&mut self) -> anyhow::Result<()> {
        if let Some(path) = &self.bms_path {
            let model = BmsDecoder::decode(path)?;
            self.play_mode = model.mode;
            self.bms_model = Some(model);
        }
        Ok(())
    }

    /// Whether we are currently in course mode.
    pub fn is_course(&self) -> bool {
        self.course_bms_models.is_some()
    }

    /// Total number of stages in the current course.
    pub fn course_total(&self) -> usize {
        self.course_bms_models.as_ref().map_or(0, |v| v.len())
    }

    /// Reset course-specific state for a new course play session.
    pub fn start_course(
        &mut self,
        course: CourseData,
        models: Vec<BmsModel>,
        dirs: Vec<PathBuf>,
        constraints: Vec<CourseDataConstraint>,
    ) {
        self.course_data = Some(course);
        self.course_bms_models = Some(models);
        self.course_bms_dirs = dirs;
        self.course_index = 0;
        self.course_score_data = Some(Vec::new());
        self.course_replays.clear();
        self.course_gauges.clear();
        self.course_constraints = constraints;
        self.course_gauge_carry = None;
    }

    /// Clear course mode state (when leaving course mode).
    pub fn clear_course(&mut self) {
        self.course_data = None;
        self.course_bms_models = None;
        self.course_bms_dirs.clear();
        self.course_index = 0;
        self.course_score_data = None;
        self.course_replays.clear();
        self.course_gauges.clear();
        self.course_gauge_carry = None;
        self.course_constraints.clear();
    }

    /// Load the next course stage BMS model into bms_model.
    /// Returns true if successful, false if no more stages.
    pub fn load_course_stage(&mut self) -> bool {
        if let Some(models) = &self.course_bms_models
            && self.course_index < models.len()
        {
            let model = models[self.course_index].clone();
            self.play_mode = model.mode;
            self.bms_dir = self.course_bms_dirs.get(self.course_index).cloned();
            self.bms_model = Some(model);
            return true;
        }
        false
    }
}

impl Default for PlayerResource {
    fn default() -> Self {
        Self {
            bms_model: None,
            score_data: ScoreData::default(),
            play_mode: PlayMode::Beat7K,
            player_config: PlayerConfig::default(),
            org_gauge_option: 0,
            bms_dir: None,
            bms_path: None,
            is_practice: false,
            player_mode: PlayerMode::default(),
            gauge_log: Vec::new(),
            maxcombo: 0,
            update_score: false,
            assist: 0,
            oldscore: ScoreData::default(),
            course_score_data: None,
            course_replays: Vec::new(),
            course_gauges: Vec::new(),
            replay_data: None,
            config_save_requested: false,
            ranking_data: None,
            target_exscore: None,
            course_bms_models: None,
            course_bms_dirs: Vec::new(),
            course_index: 0,
            course_data: None,
            course_gauge_carry: None,
            course_constraints: Vec::new(),
            ghost_battle: None,
            freq_trainer_freq: 0,
            random_trainer_enabled: false,
            random_trainer_lane_order: [1, 2, 3, 4, 5, 6, 7],
            judge_trainer_active: false,
            judge_trainer_rank: 0,
            force_no_ir_send: false,
            exit_after_result: false,
            request_app_exit: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghost_battle_default_is_none() {
        let res = PlayerResource::default();
        assert!(res.ghost_battle.is_none());
    }

    #[test]
    fn ghost_battle_set_and_take() {
        let mut res = PlayerResource::default();
        res.ghost_battle = Some(GhostBattleSettings {
            random_seed: 42,
            lane_sequence: 3,
        });

        // First take returns Some with the correct values.
        let settings = res.ghost_battle.take();
        assert!(settings.is_some());
        let settings = settings.unwrap();
        assert_eq!(settings.random_seed, 42);
        assert_eq!(settings.lane_sequence, 3);

        // Second take returns None (consumed).
        assert!(res.ghost_battle.is_none());
    }
}
