//! Configurable test double for `IRConnection`.
//!
//! Replaces the various one-off mock IR implementations scattered across
//! rubato-state and rubato-play test modules with a single builder-based
//! `TestIRConnection` that covers all common patterns:
//!
//! - Stub (all failure) -- the default
//! - Configurable send success/failure with call tracking
//! - Configurable play data / course play data responses
//! - Configurable URL responses
//! - AtomicBool tracking for method-call assertions

use std::sync::atomic::{AtomicBool, Ordering};

use crate::ir::ir_chart_data::IRChartData;
use crate::ir::ir_connection::IRConnection;
use crate::ir::ir_course_data::IRCourseData;
use crate::ir::ir_player_data::IRPlayerData;
use crate::ir::ir_response::IRResponse;
use crate::ir::ir_score_data::IRScoreData;
use crate::ir::ir_table_data::IRTableData;

/// A configurable test double for [`IRConnection`].
///
/// By default every method returns failure (or `None` for URL methods).
/// Use the builder methods (`with_*`) to configure specific responses.
pub struct TestIRConnection {
    name: String,

    // Send behaviour
    send_play_data_success: bool,
    send_course_play_data_success: bool,

    // Data responses
    play_data_scores: Option<Vec<IRScoreData>>,
    course_play_data_scores: Option<Vec<IRScoreData>>,

    // URL responses
    song_url: Option<String>,
    course_url: Option<String>,
    player_url: Option<String>,

    // Tracking flags
    send_play_data_called: AtomicBool,
    send_course_play_data_called: AtomicBool,
    get_play_data_called: AtomicBool,
    get_course_play_data_called: AtomicBool,
}

impl TestIRConnection {
    /// Create a new `TestIRConnection` with all methods returning failure.
    pub fn new() -> Self {
        Self {
            name: "TestIR".to_string(),
            send_play_data_success: false,
            send_course_play_data_success: false,
            play_data_scores: None,
            course_play_data_scores: None,
            song_url: None,
            course_url: None,
            player_url: None,
            send_play_data_called: AtomicBool::new(false),
            send_course_play_data_called: AtomicBool::new(false),
            get_play_data_called: AtomicBool::new(false),
            get_course_play_data_called: AtomicBool::new(false),
        }
    }

    // ---- Builder methods ----

    /// Set a custom name returned by [`IRConnection::name`].
    pub fn with_name(mut self, name: impl Into<String>) -> Self {
        self.name = name.into();
        self
    }

    /// Configure whether [`IRConnection::send_play_data`] returns success.
    pub fn with_send_play_data_success(mut self, success: bool) -> Self {
        self.send_play_data_success = success;
        self
    }

    /// Configure whether [`IRConnection::send_course_play_data`] returns success.
    pub fn with_send_course_play_data_success(mut self, success: bool) -> Self {
        self.send_course_play_data_success = success;
        self
    }

    /// Configure the scores returned by [`IRConnection::get_play_data`].
    ///
    /// When set, `get_play_data` returns a success response containing
    /// the provided scores. When `None` (default), it returns failure.
    pub fn with_play_data_scores(mut self, scores: Vec<IRScoreData>) -> Self {
        self.play_data_scores = Some(scores);
        self
    }

    /// Configure the scores returned by [`IRConnection::get_course_play_data`].
    ///
    /// When set, `get_course_play_data` returns a success response containing
    /// the provided scores. When `None` (default), it returns failure.
    pub fn with_course_play_data_scores(mut self, scores: Vec<IRScoreData>) -> Self {
        self.course_play_data_scores = Some(scores);
        self
    }

    /// Configure the URL returned by [`IRConnection::get_song_url`].
    pub fn with_song_url(mut self, url: impl Into<String>) -> Self {
        self.song_url = Some(url.into());
        self
    }

    /// Configure the URL returned by [`IRConnection::get_course_url`].
    pub fn with_course_url(mut self, url: impl Into<String>) -> Self {
        self.course_url = Some(url.into());
        self
    }

    /// Configure the URL returned by [`IRConnection::get_player_url`].
    pub fn with_player_url(mut self, url: impl Into<String>) -> Self {
        self.player_url = Some(url.into());
        self
    }

    // ---- Tracking accessors ----

    /// Whether [`IRConnection::send_play_data`] was called at least once.
    pub fn send_play_data_called(&self) -> bool {
        self.send_play_data_called.load(Ordering::SeqCst)
    }

    /// Whether [`IRConnection::send_course_play_data`] was called at least once.
    pub fn send_course_play_data_called(&self) -> bool {
        self.send_course_play_data_called.load(Ordering::SeqCst)
    }

    /// Whether [`IRConnection::get_play_data`] was called at least once.
    pub fn get_play_data_called(&self) -> bool {
        self.get_play_data_called.load(Ordering::SeqCst)
    }

    /// Whether [`IRConnection::get_course_play_data`] was called at least once.
    pub fn get_course_play_data_called(&self) -> bool {
        self.get_course_play_data_called.load(Ordering::SeqCst)
    }
}

impl Default for TestIRConnection {
    fn default() -> Self {
        Self::new()
    }
}

impl IRConnection for TestIRConnection {
    fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
        IRResponse::failure("TestIRConnection: get_rivals not configured".to_string())
    }

    fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
        IRResponse::failure("TestIRConnection: get_table_datas not configured".to_string())
    }

    fn get_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        _chart: Option<&IRChartData>,
    ) -> IRResponse<Vec<IRScoreData>> {
        self.get_play_data_called.store(true, Ordering::SeqCst);
        match &self.play_data_scores {
            Some(scores) => IRResponse::success("OK".to_string(), scores.clone()),
            None => {
                IRResponse::failure("TestIRConnection: get_play_data not configured".to_string())
            }
        }
    }

    fn get_course_play_data(
        &self,
        _player: Option<&IRPlayerData>,
        _course: &IRCourseData,
    ) -> IRResponse<Vec<IRScoreData>> {
        self.get_course_play_data_called
            .store(true, Ordering::SeqCst);
        match &self.course_play_data_scores {
            Some(scores) => IRResponse::success("OK".to_string(), scores.clone()),
            None => IRResponse::failure(
                "TestIRConnection: get_course_play_data not configured".to_string(),
            ),
        }
    }

    fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreData) -> IRResponse<()> {
        self.send_play_data_called.store(true, Ordering::SeqCst);
        if self.send_play_data_success {
            IRResponse::success("OK".to_string(), ())
        } else {
            IRResponse::failure("TestIRConnection: send_play_data configured to fail".to_string())
        }
    }

    fn send_course_play_data(
        &self,
        _course: &IRCourseData,
        _score: &IRScoreData,
    ) -> IRResponse<()> {
        self.send_course_play_data_called
            .store(true, Ordering::SeqCst);
        if self.send_course_play_data_success {
            IRResponse::success("OK".to_string(), ())
        } else {
            IRResponse::failure(
                "TestIRConnection: send_course_play_data configured to fail".to_string(),
            )
        }
    }

    fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
        self.song_url.clone()
    }

    fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
        self.course_url.clone()
    }

    fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
        self.player_url.clone()
    }

    fn name(&self) -> &str {
        &self.name
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::clear_type::ClearType;

    fn make_chart() -> IRChartData {
        IRChartData {
            sha256: "test_sha256".to_string(),
            ..Default::default()
        }
    }

    fn make_score(player: &str, exscore_pgs: i32) -> IRScoreData {
        IRScoreData {
            sha256: String::new(),
            lntype: 0,
            player: player.to_string(),
            clear: ClearType::Normal,
            date: 0,
            epg: exscore_pgs,
            lpg: 0,
            egr: 0,
            lgr: 0,
            egd: 0,
            lgd: 0,
            ebd: 0,
            lbd: 0,
            epr: 0,
            lpr: 0,
            ems: 0,
            lms: 0,
            avgjudge: 0,
            maxcombo: 0,
            notes: 0,
            passnotes: 0,
            minbp: 0,
            option: 0,
            seed: 0,
            assist: 0,
            gauge: 0,
            device_type: None,
            judge_algorithm: None,
            rule: None,
            skin: None,
        }
    }

    fn make_course() -> IRCourseData {
        IRCourseData {
            name: "Test Course".to_string(),
            charts: vec![],
            constraint: vec![],
            trophy: vec![],
            lntype: -1,
        }
    }

    fn make_player() -> IRPlayerData {
        IRPlayerData::new("id".to_string(), "name".to_string(), "rank".to_string())
    }

    // ---- Default behaviour ----

    #[test]
    fn default_returns_failure_for_all_methods() {
        let ir = TestIRConnection::new();
        let chart = make_chart();
        let course = make_course();
        let player = make_player();
        let score = make_score("p", 0);

        assert!(!ir.get_rivals().is_succeeded());
        assert!(!ir.get_table_datas().is_succeeded());
        assert!(!ir.get_play_data(None, Some(&chart)).is_succeeded());
        assert!(!ir.get_course_play_data(None, &course).is_succeeded());
        assert!(!ir.send_play_data(&chart, &score).is_succeeded());
        assert!(!ir.send_course_play_data(&course, &score).is_succeeded());
        assert!(ir.get_song_url(&chart).is_none());
        assert!(ir.get_course_url(&course).is_none());
        assert!(ir.get_player_url(&player).is_none());
    }

    #[test]
    fn default_name_is_test_ir() {
        let ir = TestIRConnection::new();
        assert_eq!(ir.name(), "TestIR");
    }

    // ---- Builder configuration ----

    #[test]
    fn custom_name() {
        let ir = TestIRConnection::new().with_name("CustomIR");
        assert_eq!(ir.name(), "CustomIR");
    }

    #[test]
    fn send_play_data_success_configured() {
        let ir = TestIRConnection::new().with_send_play_data_success(true);
        let chart = make_chart();
        let score = make_score("p", 0);

        let resp = ir.send_play_data(&chart, &score);
        assert!(resp.is_succeeded());
    }

    #[test]
    fn send_play_data_failure_configured() {
        let ir = TestIRConnection::new().with_send_play_data_success(false);
        let chart = make_chart();
        let score = make_score("p", 0);

        let resp = ir.send_play_data(&chart, &score);
        assert!(!resp.is_succeeded());
    }

    #[test]
    fn send_course_play_data_success_configured() {
        let ir = TestIRConnection::new().with_send_course_play_data_success(true);
        let course = make_course();
        let score = make_score("p", 0);

        let resp = ir.send_course_play_data(&course, &score);
        assert!(resp.is_succeeded());
    }

    #[test]
    fn play_data_scores_configured() {
        let scores = vec![make_score("Alice", 100), make_score("Bob", 80)];
        let ir = TestIRConnection::new().with_play_data_scores(scores);
        let chart = make_chart();

        let resp = ir.get_play_data(None, Some(&chart));
        assert!(resp.is_succeeded());
        let data = resp.data().unwrap();
        assert_eq!(data.len(), 2);
        assert_eq!(data[0].player, "Alice");
        assert_eq!(data[0].epg, 100);
        assert_eq!(data[1].player, "Bob");
    }

    #[test]
    fn course_play_data_scores_configured() {
        let scores = vec![make_score("Charlie", 50)];
        let ir = TestIRConnection::new().with_course_play_data_scores(scores);
        let course = make_course();

        let resp = ir.get_course_play_data(None, &course);
        assert!(resp.is_succeeded());
        let data = resp.data().unwrap();
        assert_eq!(data.len(), 1);
        assert_eq!(data[0].player, "Charlie");
    }

    #[test]
    fn url_methods_configured() {
        let ir = TestIRConnection::new()
            .with_song_url("https://example.com/song")
            .with_course_url("https://example.com/course")
            .with_player_url("https://example.com/player");

        let chart = make_chart();
        let course = make_course();
        let player = make_player();

        assert_eq!(
            ir.get_song_url(&chart),
            Some("https://example.com/song".to_string())
        );
        assert_eq!(
            ir.get_course_url(&course),
            Some("https://example.com/course".to_string())
        );
        assert_eq!(
            ir.get_player_url(&player),
            Some("https://example.com/player".to_string())
        );
    }

    // ---- Tracking flags ----

    #[test]
    fn tracking_flags_initially_false() {
        let ir = TestIRConnection::new();
        assert!(!ir.send_play_data_called());
        assert!(!ir.send_course_play_data_called());
        assert!(!ir.get_play_data_called());
        assert!(!ir.get_course_play_data_called());
    }

    #[test]
    fn send_play_data_sets_tracking_flag() {
        let ir = TestIRConnection::new();
        let chart = make_chart();
        let score = make_score("p", 0);

        assert!(!ir.send_play_data_called());
        ir.send_play_data(&chart, &score);
        assert!(ir.send_play_data_called());
    }

    #[test]
    fn send_course_play_data_sets_tracking_flag() {
        let ir = TestIRConnection::new();
        let course = make_course();
        let score = make_score("p", 0);

        assert!(!ir.send_course_play_data_called());
        ir.send_course_play_data(&course, &score);
        assert!(ir.send_course_play_data_called());
    }

    #[test]
    fn get_play_data_sets_tracking_flag() {
        let ir = TestIRConnection::new();
        let chart = make_chart();

        assert!(!ir.get_play_data_called());
        ir.get_play_data(None, Some(&chart));
        assert!(ir.get_play_data_called());
    }

    #[test]
    fn get_course_play_data_sets_tracking_flag() {
        let ir = TestIRConnection::new();
        let course = make_course();

        assert!(!ir.get_course_play_data_called());
        ir.get_course_play_data(None, &course);
        assert!(ir.get_course_play_data_called());
    }

    // ---- Combined configuration ----

    #[test]
    fn full_builder_chain() {
        let scores = vec![make_score("Rival", 200)];
        let ir = TestIRConnection::new()
            .with_name("FullTestIR")
            .with_send_play_data_success(true)
            .with_send_course_play_data_success(true)
            .with_play_data_scores(scores)
            .with_song_url("https://ir.example.com/song/1");

        assert_eq!(ir.name(), "FullTestIR");

        let chart = make_chart();
        let score = make_score("me", 0);

        assert!(ir.send_play_data(&chart, &score).is_succeeded());
        assert!(ir.send_play_data_called());

        let play_resp = ir.get_play_data(None, Some(&chart));
        assert!(play_resp.is_succeeded());
        assert_eq!(play_resp.data().unwrap()[0].player, "Rival");

        assert_eq!(
            ir.get_song_url(&chart),
            Some("https://ir.example.com/song/1".to_string())
        );
    }

    #[test]
    fn default_trait_impl() {
        let ir = TestIRConnection::default();
        assert_eq!(ir.name(), "TestIR");
        assert!(!ir.send_play_data_called());
    }
}
