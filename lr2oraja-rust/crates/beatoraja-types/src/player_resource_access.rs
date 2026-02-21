use crate::config::Config;
use crate::course_data::{CourseData, CourseDataConstraint};
use crate::groove_gauge::GrooveGauge;
use crate::player_config::PlayerConfig;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::song_data::SongData;

/// Trait interface for PlayerResource access.
///
/// Downstream crates use `&dyn PlayerResourceAccess` instead of concrete PlayerResource stubs.
/// The real implementation in beatoraja-core implements this trait.
///
/// Methods that return types not available in beatoraja-types (e.g., BMSModel, RankingData,
/// BMSPlayerMode) are NOT included here. Downstream crates that need those methods should
/// keep local extension stubs until the types are unified.
pub trait PlayerResourceAccess {
    // ---- Config access ----

    /// Get config reference
    fn get_config(&self) -> &Config;

    /// Get player config reference
    fn get_player_config(&self) -> &PlayerConfig;

    // ---- Score data ----

    /// Get current score data
    fn get_score_data(&self) -> Option<&ScoreData>;

    /// Get rival score data
    fn get_rival_score_data(&self) -> Option<&ScoreData>;

    /// Get target score data
    fn get_target_score_data(&self) -> Option<&ScoreData>;

    /// Get course score data
    fn get_course_score_data(&self) -> Option<&ScoreData>;

    /// Set course score data
    fn set_course_score_data(&mut self, score: ScoreData);

    // ---- Song data ----

    /// Get current song data
    fn get_songdata(&self) -> Option<&SongData>;

    // ---- Replay data ----

    /// Get replay data
    fn get_replay_data(&self) -> Option<&ReplayData>;

    /// Get course replays
    fn get_course_replay(&self) -> &[ReplayData];

    /// Add a course replay entry
    fn add_course_replay(&mut self, rd: ReplayData);

    // ---- Course data ----

    /// Get course data
    fn get_course_data(&self) -> Option<&CourseData>;

    /// Get current course index
    fn get_course_index(&self) -> usize;

    /// Advance to next course stage
    fn next_course(&mut self) -> bool;

    /// Get course constraints
    fn get_constraint(&self) -> Vec<CourseDataConstraint>;

    // ---- Gauge data ----

    /// Get gauge transition log
    fn get_gauge(&self) -> Option<&Vec<Vec<f32>>>;

    /// Get groove gauge
    fn get_groove_gauge(&self) -> Option<&GrooveGauge>;

    /// Get course gauge history
    fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>>;

    /// Add a course gauge entry
    fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>);

    // ---- Numeric state ----

    /// Get max combo count
    fn get_maxcombo(&self) -> i32;

    /// Get original gauge option
    fn get_org_gauge_option(&self) -> i32;

    /// Set original gauge option
    fn set_org_gauge_option(&mut self, val: i32);

    /// Get assist flag
    fn get_assist(&self) -> i32;

    // ---- Boolean state ----

    /// Whether to update score
    fn is_update_score(&self) -> bool;

    /// Whether to update course score
    fn is_update_course_score(&self) -> bool;

    /// Whether IR send is forcibly disabled
    fn is_force_no_ir_send(&self) -> bool;

    /// Whether frequency trainer is on
    fn is_freq_on(&self) -> bool;

    // ---- Reverse lookup ----

    /// Get reverse lookup data (table names for current song)
    fn get_reverse_lookup_data(&self) -> Vec<String>;

    /// Get reverse lookup levels (table levels for current song)
    fn get_reverse_lookup_levels(&self) -> Vec<String>;
}

/// Null implementation of PlayerResourceAccess for stub contexts.
/// All methods log a warning and return defaults.
pub struct NullPlayerResource;

impl NullPlayerResource {
    fn null_config() -> &'static Config {
        use std::sync::OnceLock;
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(Config::default)
    }

    fn null_player_config() -> &'static PlayerConfig {
        use std::sync::OnceLock;
        static PCONFIG: OnceLock<PlayerConfig> = OnceLock::new();
        PCONFIG.get_or_init(PlayerConfig::default)
    }
}

impl PlayerResourceAccess for NullPlayerResource {
    fn get_config(&self) -> &Config {
        log::warn!("NullPlayerResource::get_config called — returning default");
        Self::null_config()
    }
    fn get_player_config(&self) -> &PlayerConfig {
        log::warn!("NullPlayerResource::get_player_config called — returning default");
        Self::null_player_config()
    }
    fn get_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_rival_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_target_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_course_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn set_course_score_data(&mut self, _score: ScoreData) {}
    fn get_songdata(&self) -> Option<&SongData> {
        None
    }
    fn get_replay_data(&self) -> Option<&ReplayData> {
        None
    }
    fn get_course_replay(&self) -> &[ReplayData] {
        &[]
    }
    fn add_course_replay(&mut self, _rd: ReplayData) {}
    fn get_course_data(&self) -> Option<&CourseData> {
        None
    }
    fn get_course_index(&self) -> usize {
        0
    }
    fn next_course(&mut self) -> bool {
        false
    }
    fn get_constraint(&self) -> Vec<CourseDataConstraint> {
        vec![]
    }
    fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }
    fn get_groove_gauge(&self) -> Option<&GrooveGauge> {
        None
    }
    fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        // Return a static empty vec
        static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
        &EMPTY
    }
    fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
    fn get_maxcombo(&self) -> i32 {
        0
    }
    fn get_org_gauge_option(&self) -> i32 {
        0
    }
    fn set_org_gauge_option(&mut self, _val: i32) {}
    fn get_assist(&self) -> i32 {
        0
    }
    fn is_update_score(&self) -> bool {
        false
    }
    fn is_update_course_score(&self) -> bool {
        false
    }
    fn is_force_no_ir_send(&self) -> bool {
        false
    }
    fn is_freq_on(&self) -> bool {
        false
    }
    fn get_reverse_lookup_data(&self) -> Vec<String> {
        vec![]
    }
    fn get_reverse_lookup_levels(&self) -> Vec<String> {
        vec![]
    }
}
