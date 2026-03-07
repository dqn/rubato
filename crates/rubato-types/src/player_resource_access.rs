use std::any::Any;
use std::path::{Path, PathBuf};

use bms_model::bms_model::BMSModel;

use crate::config::Config;
use crate::course_data::{CourseData, CourseDataConstraint};
use crate::groove_gauge::GrooveGauge;
use crate::player_config::PlayerConfig;
use crate::player_data::PlayerData;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::song_data::SongData;

// ---- Sub-trait 1: PlayerConfigAccess ----

/// Config access methods for PlayerResource.
pub trait PlayerConfigAccess {
    /// Get config reference
    fn config(&self) -> &Config;
    /// Get player config reference
    fn player_config(&self) -> &PlayerConfig;
    /// Get mutable player config when the resource owns it.
    fn player_config_mut(&mut self) -> Option<&mut PlayerConfig> {
        None
    }
}

// ---- Sub-trait 2: ScoreDataAccess ----

/// Score data access methods for PlayerResource.
pub trait ScoreDataAccess {
    /// Get current score data
    fn score_data(&self) -> Option<&ScoreData>;
    /// Get mutable score data
    fn score_data_mut(&mut self) -> Option<&mut ScoreData>;
    /// Get rival score data
    fn rival_score_data(&self) -> Option<&ScoreData>;
    /// Get target score data
    fn target_score_data(&self) -> Option<&ScoreData>;
    /// Set target score data.
    /// Java: PlayerResource.setTargetScoreData(ScoreData)
    fn set_target_score_data(&mut self, _score: ScoreData) {
        // default no-op
    }
    /// Get course score data
    fn course_score_data(&self) -> Option<&ScoreData>;
    /// Set course score data
    fn set_course_score_data(&mut self, score: ScoreData);
}

// ---- Sub-trait 3: SongDataAccess ----

/// Song data and course data access methods for PlayerResource.
pub trait SongDataAccess {
    /// Get current song data
    fn songdata(&self) -> Option<&SongData>;
    /// Get mutable current song data
    fn songdata_mut(&mut self) -> Option<&mut SongData>;
    /// Set current song data (or clear with None)
    fn set_songdata(&mut self, data: Option<SongData>);
    /// Get course data
    fn course_data(&self) -> Option<&CourseData>;
    /// Get current course index
    fn course_index(&self) -> usize;
    /// Advance to next course stage
    fn next_course(&mut self) -> bool;
    /// Get course constraints
    fn constraint(&self) -> Vec<CourseDataConstraint>;
    /// Get course BMS models as song data (for course data setSong)
    fn course_song_data(&self) -> Vec<SongData>;
}

// ---- Sub-trait 4: ReplayAccess ----

/// Replay data access methods for PlayerResource.
pub trait ReplayAccess {
    /// Get replay data
    fn replay_data(&self) -> Option<&ReplayData>;
    /// Get mutable replay data
    fn replay_data_mut(&mut self) -> Option<&mut ReplayData>;
    /// Get course replays
    fn course_replay(&self) -> &[ReplayData];
    /// Get mutable course replay data
    fn course_replay_mut(&mut self) -> &mut Vec<ReplayData>;
    /// Add a course replay entry
    fn add_course_replay(&mut self, rd: ReplayData);
}

// ---- Sub-trait 5: GaugeAccess ----

/// Gauge data access methods for PlayerResource.
pub trait GaugeAccess {
    /// Get gauge transition log
    fn gauge(&self) -> Option<&Vec<Vec<f32>>>;
    /// Get groove gauge
    fn groove_gauge(&self) -> Option<&GrooveGauge>;
    /// Get course gauge history
    fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>>;
    /// Get mutable course gauge history
    fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>>;
    /// Add a course gauge entry
    fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>);
}

// ---- Sub-trait 6: PlayerStateQuery ----

/// Numeric and boolean state query methods for PlayerResource.
pub trait PlayerStateQuery {
    /// Get max combo count
    fn maxcombo(&self) -> i32;
    /// Get original gauge option
    fn org_gauge_option(&self) -> i32;
    /// Set original gauge option
    fn set_org_gauge_option(&mut self, val: i32);
    /// Get assist flag
    fn assist(&self) -> i32;
    /// Whether to update score
    fn is_update_score(&self) -> bool;
    /// Whether to update course score
    fn is_update_course_score(&self) -> bool;
    /// Whether IR send is forcibly disabled
    fn is_force_no_ir_send(&self) -> bool;
    /// Whether frequency trainer is on
    fn is_freq_on(&self) -> bool;
    /// Get reverse lookup data (table names for current song)
    fn reverse_lookup_data(&self) -> Vec<String>;
    /// Get reverse lookup levels (table levels for current song)
    fn reverse_lookup_levels(&self) -> Vec<String>;
}

// ---- Super-trait: PlayerResourceAccess ----

/// Trait interface for PlayerResource access.
///
/// Downstream crates use `&dyn PlayerResourceAccess` instead of concrete PlayerResource stubs.
/// The real implementation in beatoraja-core implements this trait.
///
/// Methods that return types not available in beatoraja-types (e.g., BMSModel, RankingData,
/// BMSPlayerMode) are NOT included here. Downstream crates that need those methods should
/// keep local extension stubs until the types are unified.
///
/// This is a super-trait of focused sub-traits:
/// - [`PlayerConfigAccess`] - Config access
/// - [`ScoreDataAccess`] - Score data access
/// - [`SongDataAccess`] - Song/course data access
/// - [`ReplayAccess`] - Replay data access
/// - [`GaugeAccess`] - Gauge data access
/// - [`PlayerStateQuery`] - Numeric/boolean state queries
pub trait PlayerResourceAccess:
    PlayerConfigAccess
    + ScoreDataAccess
    + SongDataAccess
    + ReplayAccess
    + GaugeAccess
    + PlayerStateQuery
    + Send
{
    /// Convert a boxed trait object into `Box<dyn Any + Send>` for type-erased
    /// take/restore of the underlying concrete type (e.g., core::PlayerResource).
    fn into_any_send(self: Box<Self>) -> Box<dyn Any + Send>;

    // ---- Lifecycle / mutation methods ----

    /// Clear session state (course, scores, gauge, combo, table info).
    /// Corresponds to Java PlayerResource.clear()
    fn clear(&mut self);

    /// Set BMS file for play. Returns true if loading succeeded.
    /// `mode_type`: 0=Play, 1=Practice, 2=Autoplay, 3=Replay
    /// `mode_id`: replay slot index (0 for non-replay modes)
    /// Corresponds to Java PlayerResource.setBMSFile(Path, BMSPlayerMode)
    fn set_bms_file(&mut self, path: &Path, mode_type: i32, mode_id: i32) -> bool;

    /// Set course BMS files. Returns true if all files loaded successfully.
    /// Corresponds to Java PlayerResource.setCourseBMSFiles(Path[])
    fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool;

    /// Set table name for current song
    fn set_tablename(&mut self, name: &str);

    /// Set table level for current song
    fn set_tablelevel(&mut self, level: &str);

    /// Set rival score data (Option variant for clearing)
    fn set_rival_score_data_option(&mut self, score: Option<ScoreData>);

    /// Set chart option (replay data for chart replication)
    fn set_chart_option_data(&mut self, option: Option<ReplayData>);

    /// Set course data
    fn set_course_data(&mut self, data: CourseData);

    /// Clear course data (set to None)
    fn clear_course_data(&mut self);

    /// Reload the current BMS file from disk.
    /// Preserves tablename and tablelevel across clear().
    fn reload_bms_file(&mut self) {
        // default no-op -- only the real PlayerResource has BMS loading capability
    }

    /// Set the player config gauge option.
    fn set_player_config_gauge(&mut self, _gauge: i32) {
        // default no-op
    }

    /// Set auto-play song paths for directory autoplay.
    /// Corresponds to Java PlayerResource.setAutoPlaySongs(Path[], boolean)
    fn set_auto_play_songs(&mut self, _paths: Vec<PathBuf>, _loop_play: bool) {
        // default no-op
    }

    /// Advance to the next song in auto-play or course mode.
    /// Returns true if a valid next song was loaded.
    /// Corresponds to Java PlayerResource.nextSong()
    fn next_song(&mut self) -> bool {
        false
    }

    /// Get BMS model reference.
    /// Java: PlayerResource.getBMSModel()
    fn bms_model(&self) -> Option<&BMSModel> {
        None
    }

    /// Set player data.
    /// Java: PlayerResource.setPlayerData(PlayerData)
    fn set_player_data(&mut self, _player_data: PlayerData) {
        // default no-op
    }

    /// Set banner pixmap on BMSResource from raw RGBA8888 data.
    /// Pass None to clear the banner.
    /// Java: PlayerResource.getBMSResource().setBanner(Pixmap)
    fn set_bms_banner_raw(&mut self, _data: Option<(i32, i32, Vec<u8>)>) {
        // default no-op
    }

    /// Set stagefile pixmap on BMSResource from raw RGBA8888 data.
    /// Pass None to clear the stagefile.
    /// Java: PlayerResource.getBMSResource().setStagefile(Pixmap)
    fn set_bms_stagefile_raw(&mut self, _data: Option<(i32, i32, Vec<u8>)>) {
        // default no-op
    }

    // ---- BGA processor (type-erased) ----

    /// Get the type-erased BGA processor for reuse across plays.
    ///
    /// The concrete type is `Arc<Mutex<BGAProcessor>>` from beatoraja-play, but it is stored
    /// as `Box<dyn Any + Send>` here because beatoraja-types cannot depend on beatoraja-play.
    /// The caller (LauncherStateFactory) downcasts to the concrete type.
    ///
    /// Java: PlayerResource.getBGAManager() -> BMSResource.getBGAProcessor()
    fn bga_any(&self) -> Option<&(dyn Any + Send)> {
        None
    }

    /// Store the type-erased BGA processor for reuse in subsequent plays.
    ///
    /// The caller passes `Box<Arc<Mutex<BGAProcessor>>>` erased to `Box<dyn Any + Send>`.
    /// Java: the BGAProcessor lives in BMSResource and is created once per PlayerResource.
    fn set_bga_any(&mut self, _bga: Box<dyn Any + Send>) {
        // default no-op
    }

    /// Set ranking data (type-erased).
    ///
    /// The `data` parameter should be a `Box<RankingData>` from beatoraja-ir.
    /// Pass `None` to clear. Callers downcast via `data.downcast::<RankingData>()`.
    /// Java: PlayerResource.setRankingData(RankingData)
    fn set_ranking_data_any(&mut self, _data: Option<Box<dyn Any + Send + Sync>>) {
        // default no-op
    }
}

/// Null implementation of PlayerResourceAccess for stub contexts.
/// All methods log a warning and return defaults.
#[derive(Default)]
pub struct NullPlayerResource {
    course_replay: Vec<ReplayData>,
    course_gauge: Vec<Vec<Vec<f32>>>,
}

impl NullPlayerResource {
    pub fn new() -> Self {
        Self::default()
    }

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

impl PlayerConfigAccess for NullPlayerResource {
    fn config(&self) -> &Config {
        log::warn!("NullPlayerResource::config called -- returning default");
        Self::null_config()
    }
    fn player_config(&self) -> &PlayerConfig {
        log::warn!("NullPlayerResource::player_config called -- returning default");
        Self::null_player_config()
    }
}

impl ScoreDataAccess for NullPlayerResource {
    fn score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
        None
    }
    fn rival_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn target_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn course_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn set_course_score_data(&mut self, _score: ScoreData) {}
}

impl SongDataAccess for NullPlayerResource {
    fn songdata(&self) -> Option<&SongData> {
        None
    }
    fn songdata_mut(&mut self) -> Option<&mut SongData> {
        None
    }
    fn set_songdata(&mut self, _data: Option<SongData>) {}
    fn course_data(&self) -> Option<&CourseData> {
        None
    }
    fn course_index(&self) -> usize {
        0
    }
    fn next_course(&mut self) -> bool {
        false
    }
    fn constraint(&self) -> Vec<CourseDataConstraint> {
        vec![]
    }
    fn course_song_data(&self) -> Vec<SongData> {
        vec![]
    }
}

impl ReplayAccess for NullPlayerResource {
    fn replay_data(&self) -> Option<&ReplayData> {
        None
    }
    fn replay_data_mut(&mut self) -> Option<&mut ReplayData> {
        None
    }
    fn course_replay(&self) -> &[ReplayData] {
        &[]
    }
    fn course_replay_mut(&mut self) -> &mut Vec<ReplayData> {
        &mut self.course_replay
    }
    fn add_course_replay(&mut self, _rd: ReplayData) {}
}

impl GaugeAccess for NullPlayerResource {
    fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }
    fn groove_gauge(&self) -> Option<&GrooveGauge> {
        None
    }
    fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        // Return a static empty vec
        static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
        &EMPTY
    }
    fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        &mut self.course_gauge
    }
    fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
}

impl PlayerStateQuery for NullPlayerResource {
    fn maxcombo(&self) -> i32 {
        0
    }
    fn org_gauge_option(&self) -> i32 {
        0
    }
    fn set_org_gauge_option(&mut self, _val: i32) {}
    fn assist(&self) -> i32 {
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
    fn reverse_lookup_data(&self) -> Vec<String> {
        vec![]
    }
    fn reverse_lookup_levels(&self) -> Vec<String> {
        vec![]
    }
}

impl PlayerResourceAccess for NullPlayerResource {
    fn into_any_send(self: Box<Self>) -> Box<dyn Any + Send> {
        self
    }
    fn clear(&mut self) {}
    fn set_bms_file(&mut self, _path: &Path, _mode_type: i32, _mode_id: i32) -> bool {
        log::warn!("NullPlayerResource::set_bms_file called -- returning false");
        false
    }
    fn set_course_bms_files(&mut self, _files: &[PathBuf]) -> bool {
        log::warn!("NullPlayerResource::set_course_bms_files called -- returning false");
        false
    }
    fn set_tablename(&mut self, _name: &str) {}
    fn set_tablelevel(&mut self, _level: &str) {}
    fn set_rival_score_data_option(&mut self, _score: Option<ScoreData>) {}
    fn set_chart_option_data(&mut self, _option: Option<ReplayData>) {}
    fn set_course_data(&mut self, _data: CourseData) {}
    fn clear_course_data(&mut self) {}
}
