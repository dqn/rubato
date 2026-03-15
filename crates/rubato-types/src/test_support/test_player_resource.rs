//! Configurable test double for `PlayerResourceAccess`.
//!
//! Replaces ad-hoc MockPlayerResource implementations across test modules with a
//! single builder-based struct that covers both "return canned data" and "record
//! mutations for assertions" patterns.

use std::any::Any;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use crate::config::Config;
use crate::course_data::{CourseData, CourseDataConstraint};
use crate::groove_gauge::GrooveGauge;
use crate::player_config::PlayerConfig;
use crate::player_data::PlayerData;
use crate::player_resource_access::{
    ConfigAccess, CourseAccess, GaugeAccess, MediaAccess, PlayerResourceAccess, PlayerStateAccess,
    ReplayAccess, ScoreAccess, SessionMutation, SongAccess,
};
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::song_data::SongData;

// ---------------------------------------------------------------------------
// Operation log
// ---------------------------------------------------------------------------

/// Records mutations performed on `TestPlayerResource` so tests can assert
/// which methods were called and with what arguments.
#[derive(Debug, Default)]
pub struct TestPlayerResourceLog {
    pub cleared: bool,
    pub bms_file_calls: Vec<(PathBuf, i32, i32)>,
    pub course_file_calls: Vec<Vec<PathBuf>>,
    pub tablenames: Vec<String>,
    pub tablelevels: Vec<String>,
    pub set_rival_scores: Vec<Option<ScoreData>>,
    pub set_chart_options: Vec<Option<ReplayData>>,
    pub set_course_datas: Vec<CourseData>,
    pub auto_play_songs: Vec<(Vec<PathBuf>, bool)>,
}

// ---------------------------------------------------------------------------
// TestPlayerResource
// ---------------------------------------------------------------------------

/// A configurable test double for [`PlayerResourceAccess`].
///
/// Construct via `TestPlayerResource::new()` and chain `.with_*()` builder
/// methods to set up canned return values. Mutations are recorded into an
/// internal [`TestPlayerResourceLog`] accessible via [`Self::log()`].
pub struct TestPlayerResource {
    // -- canned return data --
    config: Option<Config>,
    player_config: Option<PlayerConfig>,
    song_data: Option<SongData>,
    score_data: Option<ScoreData>,
    rival_score_data: Option<ScoreData>,
    target_score_data: Option<ScoreData>,
    course_data: Option<CourseData>,
    course_score_data: Option<ScoreData>,
    replay_data: Option<ReplayData>,
    gauge: Option<Vec<Vec<f32>>>,
    groove_gauge: Option<GrooveGauge>,
    course_song_data: Vec<SongData>,

    // -- configurable return values for mutation methods --
    bms_file_result: bool,
    course_files_result: bool,
    next_song_result: bool,

    // -- mutable storage (mirrors NullPlayerResource) --
    course_replay: Vec<ReplayData>,
    course_gauge: Vec<Vec<Vec<f32>>>,

    // -- operation log --
    log: Arc<Mutex<TestPlayerResourceLog>>,
}

impl TestPlayerResource {
    /// Create a new `TestPlayerResource` with all fields defaulting to
    /// `None`/empty/`false`.
    pub fn new() -> Self {
        Self {
            config: None,
            player_config: None,
            song_data: None,
            score_data: None,
            rival_score_data: None,
            target_score_data: None,
            course_data: None,
            course_score_data: None,
            replay_data: None,
            gauge: None,
            groove_gauge: None,
            course_song_data: Vec::new(),
            bms_file_result: false,
            course_files_result: false,
            next_song_result: false,
            course_replay: Vec::new(),
            course_gauge: Vec::new(),
            log: Arc::new(Mutex::new(TestPlayerResourceLog::default())),
        }
    }

    // -- builder methods --

    pub fn with_config(mut self, config: Config) -> Self {
        self.config = Some(config);
        self
    }

    pub fn with_player_config(mut self, pc: PlayerConfig) -> Self {
        self.player_config = Some(pc);
        self
    }

    pub fn with_song_data(mut self, song: SongData) -> Self {
        self.song_data = Some(song);
        self
    }

    pub fn with_score_data(mut self, score: ScoreData) -> Self {
        self.score_data = Some(score);
        self
    }

    pub fn with_rival_score_data(mut self, score: ScoreData) -> Self {
        self.rival_score_data = Some(score);
        self
    }

    pub fn with_target_score_data(mut self, score: ScoreData) -> Self {
        self.target_score_data = Some(score);
        self
    }

    pub fn with_course_data(mut self, course: CourseData) -> Self {
        self.course_data = Some(course);
        self
    }

    pub fn with_course_score_data(mut self, score: ScoreData) -> Self {
        self.course_score_data = Some(score);
        self
    }

    pub fn with_replay_data(mut self, replay: ReplayData) -> Self {
        self.replay_data = Some(replay);
        self
    }

    pub fn with_gauge(mut self, gauge: Vec<Vec<f32>>) -> Self {
        self.gauge = Some(gauge);
        self
    }

    pub fn with_groove_gauge(mut self, gg: GrooveGauge) -> Self {
        self.groove_gauge = Some(gg);
        self
    }

    pub fn with_bms_file_result(mut self, result: bool) -> Self {
        self.bms_file_result = result;
        self
    }

    pub fn with_course_files_result(mut self, result: bool) -> Self {
        self.course_files_result = result;
        self
    }

    pub fn with_next_song_result(mut self, result: bool) -> Self {
        self.next_song_result = result;
        self
    }

    pub fn with_course_song_data(mut self, songs: Vec<SongData>) -> Self {
        self.course_song_data = songs;
        self
    }

    // -- log access --

    /// Returns a shared handle to the operation log. Clone the `Arc` and
    /// inspect after exercising the resource under test.
    pub fn log(&self) -> Arc<Mutex<TestPlayerResourceLog>> {
        Arc::clone(&self.log)
    }

    // -- static defaults (same pattern as NullPlayerResource) --

    fn default_config() -> &'static Config {
        static CONFIG: OnceLock<Config> = OnceLock::new();
        CONFIG.get_or_init(Config::default)
    }

    fn default_player_config() -> &'static PlayerConfig {
        static PCONFIG: OnceLock<PlayerConfig> = OnceLock::new();
        PCONFIG.get_or_init(PlayerConfig::default)
    }
}

impl Default for TestPlayerResource {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// PlayerResourceAccess impl
// ---------------------------------------------------------------------------

impl ConfigAccess for TestPlayerResource {
    fn config(&self) -> &Config {
        self.config
            .as_ref()
            .unwrap_or_else(|| Self::default_config())
    }

    fn player_config(&self) -> &PlayerConfig {
        self.player_config
            .as_ref()
            .unwrap_or_else(|| Self::default_player_config())
    }

    fn player_config_mut(&mut self) -> Option<&mut PlayerConfig> {
        self.player_config.as_mut()
    }
}

impl ScoreAccess for TestPlayerResource {
    fn score_data(&self) -> Option<&ScoreData> {
        self.score_data.as_ref()
    }

    fn rival_score_data(&self) -> Option<&ScoreData> {
        self.rival_score_data.as_ref()
    }

    fn target_score_data(&self) -> Option<&ScoreData> {
        self.target_score_data.as_ref()
    }

    fn set_target_score_data(&mut self, score: ScoreData) {
        self.target_score_data = Some(score);
    }

    fn course_score_data(&self) -> Option<&ScoreData> {
        self.course_score_data.as_ref()
    }

    fn set_course_score_data(&mut self, score: ScoreData) {
        self.course_score_data = Some(score);
    }

    fn score_data_mut(&mut self) -> Option<&mut ScoreData> {
        self.score_data.as_mut()
    }
}

impl SongAccess for TestPlayerResource {
    fn songdata(&self) -> Option<&SongData> {
        self.song_data.as_ref()
    }

    fn songdata_mut(&mut self) -> Option<&mut SongData> {
        self.song_data.as_mut()
    }

    fn set_songdata(&mut self, data: Option<SongData>) {
        self.song_data = data;
    }

    fn course_song_data(&self) -> Vec<SongData> {
        self.course_song_data.clone()
    }
}

impl ReplayAccess for TestPlayerResource {
    fn replay_data(&self) -> Option<&ReplayData> {
        self.replay_data.as_ref()
    }

    fn replay_data_mut(&mut self) -> Option<&mut ReplayData> {
        self.replay_data.as_mut()
    }

    fn course_replay(&self) -> &[ReplayData] {
        &self.course_replay
    }

    fn add_course_replay(&mut self, rd: ReplayData) {
        self.course_replay.push(rd);
    }

    fn course_replay_mut(&mut self) -> &mut Vec<ReplayData> {
        &mut self.course_replay
    }
}

impl CourseAccess for TestPlayerResource {
    fn course_data(&self) -> Option<&CourseData> {
        self.course_data.as_ref()
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

    fn set_course_data(&mut self, data: CourseData) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .set_course_datas
            .push(data.clone());
        self.course_data = Some(data);
    }

    fn clear_course_data(&mut self) {
        self.course_data = None;
    }
}

impl GaugeAccess for TestPlayerResource {
    fn gauge(&self) -> Option<&Vec<Vec<f32>>> {
        self.gauge.as_ref()
    }

    fn groove_gauge(&self) -> Option<&GrooveGauge> {
        self.groove_gauge.as_ref()
    }

    fn course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        &self.course_gauge
    }

    fn add_course_gauge(&mut self, gauge: Vec<Vec<f32>>) {
        self.course_gauge.push(gauge);
    }

    fn course_gauge_mut(&mut self) -> &mut Vec<Vec<Vec<f32>>> {
        &mut self.course_gauge
    }
}

impl PlayerStateAccess for TestPlayerResource {
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
        true
    }

    fn is_update_course_score(&self) -> bool {
        true
    }

    fn is_force_no_ir_send(&self) -> bool {
        false
    }

    fn is_freq_on(&self) -> bool {
        false
    }
}

impl SessionMutation for TestPlayerResource {
    fn clear(&mut self) {
        self.log.lock().expect("mutex poisoned").cleared = true;
    }

    fn set_bms_file(&mut self, path: &Path, mode_type: i32, mode_id: i32) -> bool {
        self.log
            .lock()
            .expect("mutex poisoned")
            .bms_file_calls
            .push((path.to_path_buf(), mode_type, mode_id));
        self.bms_file_result
    }

    fn set_course_bms_files(&mut self, files: &[PathBuf]) -> bool {
        self.log
            .lock()
            .expect("mutex poisoned")
            .course_file_calls
            .push(files.to_vec());
        self.course_files_result
    }

    fn set_tablename(&mut self, name: &str) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .tablenames
            .push(name.to_string());
    }

    fn set_tablelevel(&mut self, level: &str) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .tablelevels
            .push(level.to_string());
    }

    fn set_rival_score_data_option(&mut self, score: Option<ScoreData>) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .set_rival_scores
            .push(score);
    }

    fn set_chart_option_data(&mut self, option: Option<ReplayData>) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .set_chart_options
            .push(option);
    }

    fn set_auto_play_songs(&mut self, paths: Vec<PathBuf>, loop_play: bool) {
        self.log
            .lock()
            .expect("mutex poisoned")
            .auto_play_songs
            .push((paths, loop_play));
    }

    fn next_song(&mut self) -> bool {
        self.next_song_result
    }
}

impl MediaAccess for TestPlayerResource {
    fn reverse_lookup_data(&self) -> Vec<String> {
        vec![]
    }

    fn reverse_lookup_levels(&self) -> Vec<String> {
        vec![]
    }

    fn set_player_data(&mut self, _player_data: PlayerData) {
        // no-op for tests
    }
}

impl PlayerResourceAccess for TestPlayerResource {
    fn into_any_send(self: Box<Self>) -> Box<dyn Any + Send> {
        self
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_returns_none_for_optional_data() {
        let res = TestPlayerResource::new();
        assert!(res.score_data().is_none());
        assert!(res.rival_score_data().is_none());
        assert!(res.target_score_data().is_none());
        assert!(res.songdata().is_none());
        assert!(res.replay_data().is_none());
        assert!(res.course_data().is_none());
        assert!(res.course_score_data().is_none());
        assert!(res.gauge().is_none());
        assert!(res.groove_gauge().is_none());
    }

    #[test]
    fn builder_sets_score_data() {
        let score = ScoreData::default();
        let res = TestPlayerResource::new().with_score_data(score.clone());
        assert!(res.score_data().is_some());
    }

    #[test]
    fn builder_sets_song_data() {
        let song = SongData::default();
        let res = TestPlayerResource::new().with_song_data(song);
        assert!(res.songdata().is_some());
    }

    #[test]
    fn builder_sets_course_data() {
        let course = CourseData::default();
        let res = TestPlayerResource::new().with_course_data(course);
        assert!(res.course_data().is_some());
    }

    #[test]
    fn set_bms_file_records_call_and_returns_configured_result() {
        let mut res = TestPlayerResource::new().with_bms_file_result(true);
        let log = res.log();

        let ok = res.set_bms_file(Path::new("/tmp/test.bms"), 0, 0);
        assert!(ok);

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.bms_file_calls.len(), 1);
        assert_eq!(l.bms_file_calls[0].0, PathBuf::from("/tmp/test.bms"));
        assert_eq!(l.bms_file_calls[0].1, 0);
        assert_eq!(l.bms_file_calls[0].2, 0);
    }

    #[test]
    fn set_course_bms_files_records_call() {
        let mut res = TestPlayerResource::new().with_course_files_result(true);
        let log = res.log();

        let files = vec![PathBuf::from("/a.bms"), PathBuf::from("/b.bms")];
        let ok = res.set_course_bms_files(&files);
        assert!(ok);

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.course_file_calls.len(), 1);
        assert_eq!(l.course_file_calls[0], files);
    }

    #[test]
    fn clear_sets_log_flag() {
        let mut res = TestPlayerResource::new();
        let log = res.log();

        assert!(!log.lock().expect("mutex poisoned").cleared);
        res.clear();
        assert!(log.lock().expect("mutex poisoned").cleared);
    }

    #[test]
    fn set_tablename_and_tablelevel_recorded() {
        let mut res = TestPlayerResource::new();
        let log = res.log();

        res.set_tablename("Normal");
        res.set_tablelevel("12");

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.tablenames, vec!["Normal".to_string()]);
        assert_eq!(l.tablelevels, vec!["12".to_string()]);
    }

    #[test]
    fn set_rival_score_data_option_recorded() {
        let mut res = TestPlayerResource::new();
        let log = res.log();

        res.set_rival_score_data_option(None);
        res.set_rival_score_data_option(Some(ScoreData::default()));

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.set_rival_scores.len(), 2);
        assert!(l.set_rival_scores[0].is_none());
        assert!(l.set_rival_scores[1].is_some());
    }

    #[test]
    fn course_replay_mutable_storage() {
        let mut res = TestPlayerResource::new();
        assert!(res.course_replay().is_empty());

        res.add_course_replay(ReplayData::default());
        assert_eq!(res.course_replay().len(), 1);

        res.course_replay_mut().clear();
        assert!(res.course_replay().is_empty());
    }

    #[test]
    fn course_gauge_mutable_storage() {
        let mut res = TestPlayerResource::new();
        assert!(res.course_gauge().is_empty());

        res.add_course_gauge(vec![vec![1.0, 2.0]]);
        assert_eq!(res.course_gauge().len(), 1);

        res.course_gauge_mut().clear();
        assert!(res.course_gauge().is_empty());
    }

    #[test]
    fn into_any_send_downcast_roundtrip() {
        let res = TestPlayerResource::new().with_bms_file_result(true);
        let boxed: Box<dyn PlayerResourceAccess> = Box::new(res);
        let any = boxed.into_any_send();
        let recovered = any.downcast::<TestPlayerResource>();
        assert!(recovered.is_ok());
        assert!(recovered.unwrap().bms_file_result);
    }

    #[test]
    fn config_returns_default_when_not_configured() {
        let res = TestPlayerResource::new();
        // Should not panic -- returns static default.
        let _cfg = res.config();
        let _pc = res.player_config();
    }

    #[test]
    fn config_returns_configured_value() {
        let mut cfg = Config::default();
        cfg.display.max_frame_per_second = 120;
        let res = TestPlayerResource::new().with_config(cfg);
        assert_eq!(res.config().display.max_frame_per_second, 120);
    }

    #[test]
    fn set_auto_play_songs_recorded() {
        let mut res = TestPlayerResource::new();
        let log = res.log();

        let paths = vec![PathBuf::from("/a.bms"), PathBuf::from("/b.bms")];
        res.set_auto_play_songs(paths.clone(), true);

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.auto_play_songs.len(), 1);
        assert_eq!(l.auto_play_songs[0].0, paths);
        assert!(l.auto_play_songs[0].1);
    }

    #[test]
    fn next_song_returns_configured_result() {
        let mut res_false = TestPlayerResource::new();
        assert!(!res_false.next_song());

        let mut res_true = TestPlayerResource::new().with_next_song_result(true);
        assert!(res_true.next_song());
    }

    #[test]
    fn set_course_data_updates_state_and_log() {
        let mut res = TestPlayerResource::new();
        let log = res.log();

        assert!(res.course_data().is_none());

        let course = CourseData {
            name: Some("Test Course".to_string()),
            ..Default::default()
        };
        res.set_course_data(course.clone());

        assert!(res.course_data().is_some());
        assert_eq!(
            res.course_data().unwrap().name,
            Some("Test Course".to_string())
        );

        let l = log.lock().expect("mutex poisoned");
        assert_eq!(l.set_course_datas.len(), 1);
    }

    #[test]
    fn player_config_mut_returns_some_when_configured() {
        let mut res = TestPlayerResource::new().with_player_config(PlayerConfig::default());
        assert!(res.player_config_mut().is_some());
    }

    #[test]
    fn player_config_mut_returns_none_when_not_configured() {
        let mut res = TestPlayerResource::new();
        assert!(res.player_config_mut().is_none());
    }
}
