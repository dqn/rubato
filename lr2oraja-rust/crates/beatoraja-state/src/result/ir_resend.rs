// IR resend background loop
// Translated from: MainController.java lines 518-548

use std::sync::{Arc, Mutex, OnceLock};

use beatoraja_types::ir_resend_service::IrResendService;

use super::ir_send_status::IRSendStatusMain;

/// Global shared IR send status list.
/// Both MusicResult (producer) and the resend thread (consumer) access this.
static SHARED_IR_STATUSES: OnceLock<Arc<Mutex<Vec<IRSendStatusMain>>>> = OnceLock::new();

/// Get the shared IR send status list.
pub fn shared_ir_statuses() -> Arc<Mutex<Vec<IRSendStatusMain>>> {
    SHARED_IR_STATUSES
        .get_or_init(|| Arc::new(Mutex::new(Vec::new())))
        .clone()
}

/// Concrete implementation of IrResendService.
/// Starts the background resend thread using the shared status list.
pub struct IrResendServiceImpl {
    ir_send_count: i32,
}

impl IrResendServiceImpl {
    pub fn new(ir_send_count: i32) -> Self {
        Self { ir_send_count }
    }
}

impl IrResendService for IrResendServiceImpl {
    fn start(&self) {
        start_ir_resend_thread(shared_ir_statuses(), self.ir_send_count);
    }

    fn stop(&self) {
        // The resend thread is daemon-like (same as Java: Thread.setDaemon(true)).
        // It will terminate when the process exits.
    }
}

/// Start the IR resend background thread.
///
/// Translated from: MainController.java lines 518-548
/// In Java this is an infinite loop in a daemon thread that:
/// 1. Checks each pending IR send for exponential backoff timing
/// 2. Retries the send
/// 3. Removes successful sends and those exceeding retry limit
/// 4. Sleeps 3 seconds between iterations
///
/// `ir_send_status` is the shared list of pending sends.
/// `ir_send_count` is the maximum retry count from config.
pub fn start_ir_resend_thread(
    ir_send_status: Arc<Mutex<Vec<IRSendStatusMain>>>,
    ir_send_count: i32,
) {
    std::thread::spawn(move || {
        loop {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            match ir_send_status.lock() {
                Ok(mut statuses) => {
                    // Java: List<IRSendStatus> removeIrSendStatus = new ArrayList<>();
                    let mut remove_indices: Vec<usize> = Vec::new();

                    for (i, score) in statuses.iter_mut().enumerate() {
                        // Java: long timeUntilNextTry = (long)(Math.pow(4, score.retry) * 1000);
                        let time_until_next_try = (4_i64.pow(score.retry as u32)) * 1000;
                        // Java: if (score.retry != 0 && now - score.lastTry >= timeUntilNextTry)
                        if score.retry != 0 && now - score.last_try >= time_until_next_try {
                            score.send();
                        }
                        // Java: if(score.isSent)
                        if score.is_sent {
                            remove_indices.push(i);
                        }
                        // Java: if(score.retry > getConfig().getIrSendCount())
                        if score.retry > ir_send_count {
                            remove_indices.push(i);
                            log::error!(
                                "Failed to send a score for {} {}",
                                score.songdata.get_title(),
                                score.songdata.get_subtitle()
                            );
                        }
                    }

                    // Remove in reverse order to preserve indices
                    remove_indices.sort_unstable();
                    remove_indices.dedup();
                    for &i in remove_indices.iter().rev() {
                        statuses.remove(i);
                    }
                }
                Err(e) => {
                    log::error!("Failed to lock ir_send_status: {}", e);
                }
            }

            // Java: Thread.sleep(3000, 0);
            std::thread::sleep(std::time::Duration::from_millis(3000));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_ir::ir_chart_data::IRChartData;
    use beatoraja_ir::ir_connection::IRConnection;
    use beatoraja_ir::ir_course_data::IRCourseData;
    use beatoraja_ir::ir_player_data::IRPlayerData;
    use beatoraja_ir::ir_response::IRResponse;
    use beatoraja_ir::ir_score_data::IRScoreData;
    use beatoraja_ir::ir_table_data::IRTableData;
    use beatoraja_types::song_data::SongData;
    use std::sync::atomic::{AtomicI32, Ordering};

    struct MockIRSuccess {
        send_count: AtomicI32,
    }

    impl MockIRSuccess {
        fn new() -> Self {
            Self {
                send_count: AtomicI32::new(0),
            }
        }
    }

    impl IRConnection for MockIRSuccess {
        fn get_rivals(&self) -> IRResponse<Vec<IRPlayerData>> {
            IRResponse::failure("".to_string())
        }
        fn get_table_datas(&self) -> IRResponse<Vec<IRTableData>> {
            IRResponse::failure("".to_string())
        }
        fn get_play_data(
            &self,
            _: Option<&IRPlayerData>,
            _: &IRChartData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("".to_string())
        }
        fn get_course_play_data(
            &self,
            _: Option<&IRPlayerData>,
            _: &IRCourseData,
        ) -> IRResponse<Vec<IRScoreData>> {
            IRResponse::failure("".to_string())
        }
        fn send_play_data(&self, _: &IRChartData, _: &IRScoreData) -> IRResponse<()> {
            self.send_count.fetch_add(1, Ordering::SeqCst);
            IRResponse::success("OK".to_string(), ())
        }
        fn send_course_play_data(&self, _: &IRCourseData, _: &IRScoreData) -> IRResponse<()> {
            IRResponse::failure("".to_string())
        }
        fn get_song_url(&self, _: &IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(&self, _: &IRCourseData) -> Option<String> {
            None
        }
        fn get_player_url(&self, _: &IRPlayerData) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIR"
        }
    }

    #[test]
    fn test_ir_send_status_removed_after_success() {
        // Pre-set a status that already called send() once (retry=1) with last_try=0
        // so the backoff condition (now - 0 >= 4^1 * 1000 = 4000ms) is met
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRSuccess::new());
        let mut song = SongData::default();
        song.title = "Test".to_string();
        let score = beatoraja_core::score_data::ScoreData::default();
        let mut status = IRSendStatusMain::new(conn, &song, &score);
        status.retry = 1;
        status.last_try = 0; // long ago, so backoff is satisfied

        let ir_send_status = Arc::new(Mutex::new(vec![status]));

        // Simulate one iteration of the resend loop manually
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;

        {
            let mut statuses = ir_send_status.lock().unwrap();
            let mut remove_indices: Vec<usize> = Vec::new();

            for (i, score) in statuses.iter_mut().enumerate() {
                let time_until_next_try = (4_i64.pow(score.retry as u32)) * 1000;
                if score.retry != 0 && now - score.last_try >= time_until_next_try {
                    score.send();
                }
                if score.is_sent {
                    remove_indices.push(i);
                }
            }

            remove_indices.sort_unstable();
            remove_indices.dedup();
            for &i in remove_indices.iter().rev() {
                statuses.remove(i);
            }
        }

        assert!(ir_send_status.lock().unwrap().is_empty());
    }

    #[test]
    fn test_ir_send_status_removed_after_max_retries() {
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRSuccess::new());
        let mut song = SongData::default();
        song.title = "Test".to_string();
        let score = beatoraja_core::score_data::ScoreData::default();
        let mut status = IRSendStatusMain::new(conn, &song, &score);
        // Set retry count above the limit
        status.retry = 6;
        let ir_send_count = 5;

        let ir_send_status = Arc::new(Mutex::new(vec![status]));

        {
            let mut statuses = ir_send_status.lock().unwrap();
            let mut remove_indices: Vec<usize> = Vec::new();

            for (i, score) in statuses.iter_mut().enumerate() {
                if score.retry > ir_send_count {
                    remove_indices.push(i);
                }
            }

            remove_indices.sort_unstable();
            remove_indices.dedup();
            for &i in remove_indices.iter().rev() {
                statuses.remove(i);
            }
        }

        assert!(ir_send_status.lock().unwrap().is_empty());
    }

    #[test]
    fn test_exponential_backoff_calculation() {
        // Java: Math.pow(4, retry) * 1000
        // retry=0: 1 * 1000 = 1000ms (1s)
        // retry=1: 4 * 1000 = 4000ms (4s)
        // retry=2: 16 * 1000 = 16000ms (16s)
        // retry=3: 64 * 1000 = 64000ms (64s)
        assert_eq!((4_i64.pow(0)) * 1000, 1000);
        assert_eq!((4_i64.pow(1)) * 1000, 4000);
        assert_eq!((4_i64.pow(2)) * 1000, 16000);
        assert_eq!((4_i64.pow(3)) * 1000, 64000);
        assert_eq!((4_i64.pow(4)) * 1000, 256000);
    }
}
