// IR resend background loop
// Translated from: MainController.java lines 518-548

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::JoinHandle;

use rubato_types::ir_resend_service::IrResendService;

use super::ir_send_status::IRSendStatusMain;

/// Global shared IR send status list.
/// Both MusicResult (producer) and the resend thread (consumer) access this.
/// NOTE: Process-global OnceLock. Tests using service.start() share this queue;
/// tests that need isolation should pass a local Arc<Mutex<Vec>> to start_ir_resend_thread() directly.
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
    shutdown_flag: Arc<AtomicBool>,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl IrResendServiceImpl {
    pub fn new(ir_send_count: i32) -> Self {
        Self {
            ir_send_count,
            shutdown_flag: Arc::new(AtomicBool::new(false)),
            handle: Mutex::new(None),
        }
    }
}

impl IrResendService for IrResendServiceImpl {
    fn start(&self) {
        // Reset shutdown flag so a restarted service doesn't exit immediately.
        self.shutdown_flag.store(false, Ordering::Release);
        let handle = start_ir_resend_thread(
            shared_ir_statuses(),
            self.ir_send_count,
            &self.shutdown_flag,
        );
        let mut guard = rubato_types::sync_utils::lock_or_recover(&self.handle);
        *guard = Some(handle);
    }

    fn stop(&self) {
        self.shutdown_flag.store(true, Ordering::Release);
        let mut guard = rubato_types::sync_utils::lock_or_recover(&self.handle);
        if let Some(handle) = guard.take() {
            // The thread checks the shutdown flag every 100ms.
            // Join if already finished; otherwise detach to avoid
            // busy-waiting up to 5s on the calling thread.
            if handle.is_finished()
                && let Err(e) = handle.join()
            {
                log::warn!("IR resend thread panicked: {:?}", e);
            }
            // If not finished, detach (drop the JoinHandle).
            // The thread will observe the shutdown flag and exit
            // within ~100ms on its own.
        }
    }
}

impl Drop for IrResendServiceImpl {
    fn drop(&mut self) {
        self.stop();
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
/// `shutdown_flag` is checked each iteration to allow graceful shutdown.
///
/// Returns the `JoinHandle` for the spawned thread.
pub fn start_ir_resend_thread(
    ir_send_status: Arc<Mutex<Vec<IRSendStatusMain>>>,
    ir_send_count: i32,
    shutdown_flag: &Arc<AtomicBool>,
) -> JoinHandle<()> {
    let shutdown = Arc::clone(shutdown_flag);
    std::thread::spawn(move || {
        loop {
            if shutdown.load(Ordering::Acquire) {
                break;
            }

            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;

            // Phase 1: Take all entries out of the shared list so we can release the
            // lock before performing blocking HTTP sends.
            let mut snapshot: Vec<IRSendStatusMain> = {
                let mut statuses = rubato_types::sync_utils::lock_or_recover(&ir_send_status);
                statuses.drain(..).collect()
            };

            // Phase 2: Perform blocking HTTP sends outside the lock.
            for score in &mut snapshot {
                // Java: long timeUntilNextTry = (long)(Math.pow(4, score.retry) * 1000);
                let time_until_next_try = 4_i64
                    .checked_pow(score.retry as u32)
                    .unwrap_or(i64::MAX / 1000)
                    .saturating_mul(1000);
                // Java: if (score.retry != 0 && now - score.lastTry >= timeUntilNextTry)
                if score.retry != 0 && now - score.last_try >= time_until_next_try {
                    score.send();
                }
            }

            // Phase 3: Re-acquire lock. Remove completed/exhausted entries, put back
            // the rest. Any new entries added by other threads while we were sending
            // are already in the vec and will be preserved.
            {
                let mut keep: Vec<IRSendStatusMain> = Vec::new();
                for score in snapshot {
                    if score.is_sent {
                        // Successfully sent -- discard.
                        continue;
                    }
                    if score.retry > ir_send_count {
                        log::error!(
                            "Failed to send a score for {} {}",
                            score.songdata.metadata.title,
                            score.songdata.metadata.subtitle
                        );
                        continue;
                    }
                    keep.push(score);
                }
                {
                    let mut statuses = rubato_types::sync_utils::lock_or_recover(&ir_send_status);
                    // Prepend kept entries before any newly added ones.
                    let new_entries: Vec<IRSendStatusMain> = statuses.drain(..).collect();
                    statuses.extend(keep);
                    statuses.extend(new_entries);
                }
            }

            // Java: Thread.sleep(3000, 0);
            // Sleep in small increments so we can respond to shutdown quickly.
            for _ in 0..30 {
                if shutdown.load(Ordering::Acquire) {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(100));
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_ir::ir_chart_data::IRChartData;
    use rubato_ir::ir_connection::IRConnection;
    use rubato_ir::ir_course_data::IRCourseData;
    use rubato_ir::ir_player_data::IRPlayerData;
    use rubato_ir::ir_response::IRResponse;
    use rubato_ir::ir_score_data::IRScoreData;
    use rubato_ir::ir_table_data::IRTableData;
    use rubato_types::song_data::SongData;
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
        song.metadata.title = "Test".to_string();
        let score = rubato_core::score_data::ScoreData::default();
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
            let mut statuses = ir_send_status.lock().expect("mutex poisoned");
            let mut remove_indices: Vec<usize> = Vec::new();

            for (i, score) in statuses.iter_mut().enumerate() {
                let time_until_next_try = 4_i64
                    .checked_pow(score.retry as u32)
                    .unwrap_or(i64::MAX / 1000)
                    .saturating_mul(1000);
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

        assert!(ir_send_status.lock().expect("mutex poisoned").is_empty());
    }

    #[test]
    fn test_ir_send_status_removed_after_max_retries() {
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRSuccess::new());
        let mut song = SongData::default();
        song.metadata.title = "Test".to_string();
        let score = rubato_core::score_data::ScoreData::default();
        let mut status = IRSendStatusMain::new(conn, &song, &score);
        // Set retry count above the limit
        status.retry = 6;
        let ir_send_count = 5;

        let ir_send_status = Arc::new(Mutex::new(vec![status]));

        {
            let mut statuses = ir_send_status.lock().expect("mutex poisoned");
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

        assert!(ir_send_status.lock().expect("mutex poisoned").is_empty());
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

    // -- Thread lifecycle tests --

    #[test]
    fn test_start_ir_resend_thread_spawns_and_shuts_down() {
        // Verify the thread spawns and can be shut down via the flag.
        let ir_send_status = Arc::new(Mutex::new(Vec::<IRSendStatusMain>::new()));
        let ir_send_count = 5;
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = start_ir_resend_thread(Arc::clone(&ir_send_status), ir_send_count, &shutdown);

        // The thread is running in the background; verify the shared list is still usable
        let statuses = ir_send_status.lock().expect("mutex poisoned");
        assert!(statuses.is_empty());
        drop(statuses);

        // Signal shutdown and join
        shutdown.store(true, Ordering::Release);
        handle.join().expect("thread should not panic");
    }

    #[test]
    fn test_resend_thread_removes_success() {
        // Create a status that will succeed on first resend attempt.
        // retry=1 with last_try=0 so the backoff condition is immediately met.
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRSuccess::new());
        let mut song = SongData::default();
        song.metadata.title = "ThreadSuccessTest".to_string();
        let score = rubato_core::score_data::ScoreData::default();
        let mut status = IRSendStatusMain::new(conn, &song, &score);
        status.retry = 1;
        status.last_try = 0;

        let ir_send_status = Arc::new(Mutex::new(vec![status]));
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = start_ir_resend_thread(Arc::clone(&ir_send_status), 5, &shutdown);

        // Wait for the thread to process (first iteration runs immediately)
        std::thread::sleep(std::time::Duration::from_millis(500));

        let statuses = ir_send_status.lock().expect("mutex poisoned");
        assert!(
            statuses.is_empty(),
            "successful status should be removed by the resend thread"
        );
        drop(statuses);

        shutdown.store(true, Ordering::Release);
        handle.join().expect("thread should not panic");
    }

    #[test]
    fn test_resend_thread_removes_exhausted_retries() {
        // Create a status whose retry count already exceeds the max.
        // The thread should remove it on the first iteration without attempting to send.
        let conn: Arc<dyn IRConnection + Send + Sync> = Arc::new(MockIRSuccess::new());
        let mut song = SongData::default();
        song.metadata.title = "ExhaustedRetryTest".to_string();
        song.metadata.subtitle = "sub".to_string();
        let score = rubato_core::score_data::ScoreData::default();
        let mut status = IRSendStatusMain::new(conn, &song, &score);
        status.retry = 10; // well above the limit of 5
        status.last_try = 0;

        let ir_send_status = Arc::new(Mutex::new(vec![status]));
        let shutdown = Arc::new(AtomicBool::new(false));

        let handle = start_ir_resend_thread(Arc::clone(&ir_send_status), 5, &shutdown);

        // Wait for the thread to process the first iteration
        std::thread::sleep(std::time::Duration::from_millis(500));

        let statuses = ir_send_status.lock().expect("mutex poisoned");
        assert!(
            statuses.is_empty(),
            "exhausted-retry status should be removed by the resend thread"
        );
        drop(statuses);

        shutdown.store(true, Ordering::Release);
        handle.join().expect("thread should not panic");
    }

    #[test]
    fn test_ir_resend_service_impl_stop_joins_thread() {
        let service = IrResendServiceImpl::new(5);
        service.start();
        // Thread should be running
        assert!(service.handle.lock().unwrap().is_some());

        service.stop();

        // After stop, the shutdown flag should be set and handle consumed
        assert!(service.shutdown_flag.load(Ordering::Acquire));
        assert!(service.handle.lock().unwrap().is_none());
    }

    #[test]
    fn test_ir_resend_service_impl_drop_shuts_down() {
        let flag = {
            let service = IrResendServiceImpl::new(5);
            service.start();
            let flag = Arc::clone(&service.shutdown_flag);
            assert!(!flag.load(Ordering::Acquire));
            flag
            // service dropped here
        };
        // After drop, the shutdown flag should be set
        assert!(flag.load(Ordering::Acquire));
    }
}
