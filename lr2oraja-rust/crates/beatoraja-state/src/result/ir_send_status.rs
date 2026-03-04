use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use beatoraja_ir::ir_chart_data::IRChartData;
use beatoraja_ir::ir_connection::IRConnection;
use beatoraja_ir::ir_score_data::IRScoreData;
use beatoraja_types::song_data::SongData;

/// MainController.IRSendStatus — handles IR score submission
///
/// Translated from: MainController.IRSendStatus (Java inner class)
pub struct IRSendStatusMain {
    pub connection: Arc<dyn IRConnection + Send + Sync>,
    pub songdata: SongData,
    pub score: beatoraja_core::score_data::ScoreData,
    pub retry: i32,
    pub last_try: i64,
    pub is_sent: bool,
}

impl IRSendStatusMain {
    pub fn new(
        connection: Arc<dyn IRConnection + Send + Sync>,
        songdata: &SongData,
        score: &beatoraja_core::score_data::ScoreData,
    ) -> Self {
        Self {
            connection,
            songdata: songdata.clone(),
            score: score.clone(),
            retry: 0,
            last_try: 0,
            is_sent: false,
        }
    }

    /// Send play data to IR.
    ///
    /// Translated from: MainController.IRSendStatus.send()
    pub fn send(&mut self) -> bool {
        log::info!("IRへスコア送信中 : {}", self.songdata.get_title());
        self.last_try = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        let chart_data = IRChartData::new(&self.songdata);
        let score_data = IRScoreData::new(&self.score);
        let send1 = self.connection.send_play_data(&chart_data, &score_data);
        self.retry += 1;
        if send1.is_succeeded() {
            log::info!("IRスコア送信完了 : {}", self.songdata.get_title());
            self.is_sent = true;
            true
        } else {
            log::warn!("IRスコア送信失敗 : {}", send1.get_message());
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_ir::ir_account::IRAccount;
    use beatoraja_ir::ir_chart_data::IRChartData;
    use beatoraja_ir::ir_course_data::IRCourseData;
    use beatoraja_ir::ir_player_data::IRPlayerData;
    use beatoraja_ir::ir_response::IRResponse;
    use beatoraja_ir::ir_score_data::IRScoreData;
    use beatoraja_ir::ir_table_data::IRTableData;
    use std::sync::atomic::{AtomicBool, Ordering};

    /// Mock IR connection that returns success
    struct MockIRConnectionSuccess {
        send_called: AtomicBool,
    }

    impl MockIRConnectionSuccess {
        fn new() -> Self {
            Self {
                send_called: AtomicBool::new(false),
            }
        }
    }

    impl IRConnection for MockIRConnectionSuccess {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: &IRChartData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _course: &IRCourseData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("mock".to_string())
        }
        fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreData) -> IRResponse<()> {
            self.send_called.store(true, Ordering::SeqCst);
            IRResponse::success("OK".to_string(), ())
        }
        fn send_course_play_data(
            &self,
            _course: &IRCourseData,
            _score: &IRScoreData,
        ) -> IRResponse<()> {
            IRResponse::failure("mock".to_string())
        }
        fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
            None
        }
        fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIR"
        }
    }

    /// Mock IR connection that returns failure
    struct MockIRConnectionFailure;

    impl IRConnection for MockIRConnectionFailure {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _chart: &IRChartData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("mock".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&IRPlayerData>,
            _course: &IRCourseData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("mock".to_string())
        }
        fn send_play_data(&self, _model: &IRChartData, _score: &IRScoreData) -> IRResponse<()> {
            IRResponse::failure("Server error".to_string())
        }
        fn send_course_play_data(
            &self,
            _course: &IRCourseData,
            _score: &IRScoreData,
        ) -> IRResponse<()> {
            IRResponse::failure("mock".to_string())
        }
        fn get_song_url(&self, _chart: &IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(&self, _course: &IRCourseData) -> Option<String> {
            None
        }
        fn get_player_url(&self, _player: &IRPlayerData) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIRFail"
        }
    }

    fn make_test_song() -> SongData {
        let mut song = SongData::default();
        song.title = "Test Song".to_string();
        song.subtitle = "Test Sub".to_string();
        song
    }

    fn make_test_score() -> beatoraja_core::score_data::ScoreData {
        beatoraja_core::score_data::ScoreData::default()
    }

    #[test]
    fn test_ir_send_status_new_defaults() {
        let conn = Arc::new(MockIRConnectionSuccess::new());
        let song = make_test_song();
        let score = make_test_score();
        let status = IRSendStatusMain::new(conn, &song, &score);

        assert_eq!(status.retry, 0);
        assert_eq!(status.last_try, 0);
        assert!(!status.is_sent);
    }

    #[test]
    fn test_ir_send_status_send_success() {
        let conn = Arc::new(MockIRConnectionSuccess::new());
        let song = make_test_song();
        let score = make_test_score();
        let mut status = IRSendStatusMain::new(conn.clone(), &song, &score);

        let result = status.send();

        assert!(result);
        assert!(status.is_sent);
        assert_eq!(status.retry, 1);
        assert!(status.last_try > 0);
        assert!(conn.send_called.load(Ordering::SeqCst));
    }

    #[test]
    fn test_ir_send_status_send_failure() {
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRConnectionFailure);
        let song = make_test_song();
        let score = make_test_score();
        let mut status = IRSendStatusMain::new(conn, &song, &score);

        let result = status.send();

        assert!(!result);
        assert!(!status.is_sent);
        assert_eq!(status.retry, 1);
        assert!(status.last_try > 0);
    }

    #[test]
    fn test_ir_send_status_retry_increments() {
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRConnectionFailure);
        let song = make_test_song();
        let score = make_test_score();
        let mut status = IRSendStatusMain::new(conn, &song, &score);

        status.send();
        assert_eq!(status.retry, 1);

        status.send();
        assert_eq!(status.retry, 2);

        status.send();
        assert_eq!(status.retry, 3);
    }
}
