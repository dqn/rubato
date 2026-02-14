// PlayerResource — shared data container passed between states.
//
// Contains the currently loaded BMS model, score data, and play settings.
// Used to pass data between Decide -> Play -> Result states.

use std::path::PathBuf;

use bms_config::PlayerConfig;
use bms_database::CourseData;
use bms_ir::RankingData;
use bms_model::{BmsModel, PlayMode};
use bms_replay::replay_data::ReplayData;
use bms_rule::ScoreData;

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
    #[allow(dead_code)]
    pub player_config: PlayerConfig,
    /// Original gauge option (saved at Decide, may be modified during Play).
    pub org_gauge_option: i32,
    /// BMS file's parent directory (for resolving WAV paths).
    pub bms_dir: Option<PathBuf>,

    // --- Play result fields (populated by PlayState shutdown) ---
    /// Gauge log: per-gauge-type values recorded every 500ms during play.
    pub gauge_log: Vec<Vec<f32>>,
    /// Maximum combo achieved.
    pub maxcombo: i32,
    /// Whether this score should be saved (false for autoplay/replay).
    pub update_score: bool,
    /// Assist option flags.
    #[allow(dead_code)] // Reserved for assist mode system
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
}

impl PlayerResource {
    /// Whether we are currently in course mode.
    pub fn is_course(&self) -> bool {
        self.course_bms_models.is_some()
    }

    /// Total number of stages in the current course.
    pub fn course_total(&self) -> usize {
        self.course_bms_models.as_ref().map_or(0, |v| v.len())
    }

    /// Reset course-specific state for a new course play session.
    pub fn start_course(&mut self, course: CourseData, models: Vec<BmsModel>, dirs: Vec<PathBuf>) {
        self.course_data = Some(course);
        self.course_bms_models = Some(models);
        self.course_bms_dirs = dirs;
        self.course_index = 0;
        self.course_score_data = Some(Vec::new());
        self.course_replays.clear();
        self.course_gauges.clear();
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
            course_bms_models: None,
            course_bms_dirs: Vec::new(),
            course_index: 0,
            course_data: None,
            course_gauge_carry: None,
        }
    }
}
