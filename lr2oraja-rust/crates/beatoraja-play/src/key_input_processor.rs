use crate::bms_player::TIME_MARGIN;
use crate::lane_property::LaneProperty;

/// A single key state change to replay.
/// Produced by JudgeThread::tick() for the caller to apply.
#[derive(Clone, Debug, PartialEq)]
pub struct ReplayKeyEvent {
    pub keycode: i32,
    pub pressed: bool,
    pub time: i64,
}

/// Result of a single JudgeThread tick.
#[derive(Clone, Debug)]
pub struct JudgeTickResult {
    /// Key events to replay (may be empty).
    pub replay_events: Vec<ReplayKeyEvent>,
    /// Whether judge.update(mtime) should be called this tick.
    pub should_update_judge: bool,
    /// Whether the judge thread has finished (past last timeline).
    pub finished: bool,
    /// Whether keylog was present (for resetting key states on finish).
    pub has_keylog: bool,
}

/// Key input processing thread
pub struct KeyInputProccessor {
    prevtime: i64,
    scratch: Vec<f32>,
    scratch_key: Vec<i32>,
    scratch_tt_graphic_speed: Vec<f32>,
    lane_property: LaneProperty,
    is_judge_started: bool,
    key_beam_stop: bool,
    judge: Option<JudgeThread>,
}

/// Replay keylog entry — lightweight copy of the DTO fields needed for replay.
#[derive(Clone, Debug)]
struct ReplayKeylogEntry {
    /// Press time in microseconds
    time: i64,
    keycode: i32,
    pressed: bool,
}

/// Judge thread state — processes replay keylog and drives judge updates.
///
/// Corresponds to Java JudgeThread (inner class of KeyInputProccessor).
/// In Java this extends Thread and runs concurrently. In Rust, we use a
/// synchronous tick-based design where tick() is called each frame from
/// the game loop.
struct JudgeThread {
    stop: bool,
    micro_margin_time: i64,
    /// Micro time of last timeline + TIME_MARGIN * 1000
    last_time: i64,
    /// Replay keylog entries (None if no replay)
    keylog: Option<Vec<ReplayKeylogEntry>>,
    /// Current index into keylog
    index: usize,
    /// Previous tick time for performance tracking
    prevtime: i64,
    /// Max frame time (for logging)
    frametime: i64,
}

impl JudgeThread {
    /// Create a new JudgeThread.
    ///
    /// Corresponds to Java: new JudgeThread(timelines, keylog, milliMarginTime)
    ///
    /// # Arguments
    /// * `last_timeline_micro_time` - micro time of the last timeline
    /// * `keylog` - optional replay keylog entries (DTO KeyInputLog from ReplayData)
    /// * `milli_margin_time` - margin time in milliseconds
    fn new(
        last_timeline_micro_time: i64,
        keylog: Option<Vec<ReplayKeylogEntry>>,
        milli_margin_time: i64,
    ) -> Self {
        let last_time = last_timeline_micro_time + TIME_MARGIN as i64 * 1000;
        JudgeThread {
            stop: false,
            micro_margin_time: milli_margin_time * 1000,
            last_time,
            keylog,
            index: 0,
            prevtime: -1,
            frametime: 1,
        }
    }

    /// Process one tick of the judge thread.
    ///
    /// Corresponds to one iteration of Java JudgeThread.run() loop.
    /// Returns a JudgeTickResult describing what actions the caller should take.
    ///
    /// # Arguments
    /// * `mtime` - current micro time from TIMER_PLAY
    fn tick(&mut self, mtime: i64) -> JudgeTickResult {
        if self.stop {
            return JudgeTickResult {
                replay_events: Vec::new(),
                should_update_judge: false,
                finished: true,
                has_keylog: self.keylog.is_some(),
            };
        }

        // Check if past last timeline time
        if mtime >= self.last_time {
            return JudgeTickResult {
                replay_events: Vec::new(),
                should_update_judge: false,
                finished: true,
                has_keylog: self.keylog.is_some(),
            };
        }

        let mut replay_events = Vec::new();
        let mut should_update = false;

        if mtime != self.prevtime {
            // Replay keylog entries up to current time
            if let Some(ref keylog) = self.keylog {
                while self.index < keylog.len()
                    && keylog[self.index].time + self.micro_margin_time <= mtime
                {
                    let entry = &keylog[self.index];
                    replay_events.push(ReplayKeyEvent {
                        keycode: entry.keycode,
                        pressed: entry.pressed,
                        time: entry.time + self.micro_margin_time,
                    });
                    self.index += 1;
                }
            }

            should_update = true;

            // Track performance (max frame time)
            if self.prevtime != -1 {
                let nowtime = mtime - self.prevtime;
                if nowtime > self.frametime {
                    self.frametime = nowtime;
                }
            }

            self.prevtime = mtime;
        }
        // If mtime == prevtime, Java sleeps 0.5ms; in Rust single-threaded we just skip

        JudgeTickResult {
            replay_events,
            should_update_judge: should_update,
            finished: false,
            has_keylog: self.keylog.is_some(),
        }
    }

    /// Get the max frame time observed (for performance logging).
    fn get_frametime(&self) -> i64 {
        self.frametime
    }
}

impl KeyInputProccessor {
    pub fn new(lane_property: &LaneProperty) -> Self {
        let scratch_len = lane_property.get_scratch_key_assign().len();
        KeyInputProccessor {
            prevtime: -1,
            scratch: vec![0.0; scratch_len],
            scratch_key: vec![0; scratch_len],
            scratch_tt_graphic_speed: vec![0.0; scratch_len],
            lane_property: lane_property.clone(),
            is_judge_started: false,
            key_beam_stop: false,
            judge: None,
        }
    }

    /// Start the judge thread with model timelines and optional replay keylog.
    ///
    /// Corresponds to Java: keyinput.startJudge(model, replay.keylog, resource.getMarginTime())
    ///
    /// # Arguments
    /// * `last_timeline_micro_time` - micro time of the last timeline in the model
    /// * `keylog` - optional replay keylog from ReplayData (DTO with pub fields)
    /// * `milli_margin_time` - margin time in milliseconds
    pub fn start_judge(
        &mut self,
        last_timeline_micro_time: i64,
        keylog: Option<&[beatoraja_types::stubs::KeyInputLog]>,
        milli_margin_time: i64,
    ) {
        // Convert DTO KeyInputLog entries to internal ReplayKeylogEntry
        let entries = keylog.map(|logs| {
            logs.iter()
                .map(|k| ReplayKeylogEntry {
                    time: k.time,
                    keycode: k.keycode,
                    pressed: k.pressed,
                })
                .collect()
        });
        self.judge = Some(JudgeThread::new(
            last_timeline_micro_time,
            entries,
            milli_margin_time,
        ));
        self.is_judge_started = true;
    }

    /// Tick the judge thread. Returns None if judge is not started.
    ///
    /// The caller should:
    /// 1. Apply replay_events by calling input.set_key_state() for each
    /// 2. Call judge.update(mtime) if should_update_judge is true
    /// 3. If finished: call input.reset_all_key_state() if has_keylog, then stop_judge()
    pub fn tick_judge(&mut self, mtime: i64) -> Option<JudgeTickResult> {
        self.judge.as_mut().map(|j| j.tick(mtime))
    }

    pub fn input(&mut self) {
        // TODO: Phase 7+ dependency - requires BMSPlayer, MainController, BMSPlayerInputProcessor
        // This method handles key beam flags and scratch turntable animation
        self.prevtime = 0; // stub
    }

    pub fn input_key_on(&mut self, lane: usize) {
        let lane_skin_offset = self.lane_property.get_lane_skin_offset();
        if lane >= lane_skin_offset.len() {
            return;
        }
        if self.key_beam_stop {}
        // TODO: Phase 7+ dependency - requires BMSPlayer timer, SkinPropertyMapper
    }

    pub fn stop_judge(&mut self) {
        if self.judge.is_some() {
            if let Some(ref j) = self.judge {
                log::info!("入力パフォーマンス(max us) : {}", j.get_frametime());
            }
            self.key_beam_stop = true;
            self.is_judge_started = false;
            self.judge = None;
        }
    }

    /// Returns whether the judge has been started and is still active.
    pub fn is_judge_started(&self) -> bool {
        self.is_judge_started
    }

    pub fn set_key_beam_stop(&mut self, input_stop: bool) {
        self.key_beam_stop = input_stop;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::stubs::KeyInputLog as DtoKeyInputLog;
    use bms_model::mode::Mode;

    fn make_lane_property() -> LaneProperty {
        LaneProperty::new(&Mode::BEAT_7K)
    }

    // --- JudgeThread tests ---

    #[test]
    fn test_judge_thread_new_sets_fields() {
        let jt = JudgeThread::new(1_000_000, None, 500);
        assert!(!jt.stop);
        assert_eq!(jt.micro_margin_time, 500_000); // 500ms * 1000
        assert_eq!(jt.last_time, 1_000_000 + TIME_MARGIN as i64 * 1000);
        assert!(jt.keylog.is_none());
        assert_eq!(jt.index, 0);
        assert_eq!(jt.prevtime, -1);
    }

    #[test]
    fn test_judge_thread_tick_no_keylog_updates_judge() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());
        assert!(result.should_update_judge);
        assert!(!result.finished);
        assert!(!result.has_keylog);
    }

    #[test]
    fn test_judge_thread_tick_same_time_skips_update() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        let _ = jt.tick(1_000_000);
        // Same time again
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());
        assert!(!result.should_update_judge);
        assert!(!result.finished);
    }

    #[test]
    fn test_judge_thread_tick_past_last_time_finishes() {
        let last_tl_time = 1_000_000i64;
        let mut jt = JudgeThread::new(last_tl_time, None, 0);
        let past_time = last_tl_time + TIME_MARGIN as i64 * 1000;
        let result = jt.tick(past_time);
        assert!(result.finished);
        assert!(!result.should_update_judge);
    }

    #[test]
    fn test_judge_thread_tick_stopped_returns_finished() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        jt.stop = true;
        let result = jt.tick(1_000_000);
        assert!(result.finished);
        assert!(!result.should_update_judge);
    }

    #[test]
    fn test_judge_thread_tick_replays_keylog_entries() {
        let keylog = vec![
            ReplayKeylogEntry {
                time: 1_000_000,
                keycode: 0,
                pressed: true,
            },
            ReplayKeylogEntry {
                time: 2_000_000,
                keycode: 1,
                pressed: true,
            },
            ReplayKeylogEntry {
                time: 3_000_000,
                keycode: 0,
                pressed: false,
            },
        ];
        let mut jt = JudgeThread::new(10_000_000, Some(keylog), 0);

        // At 1.5s, should replay first entry only
        let result = jt.tick(1_500_000);
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].keycode, 0);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[0].time, 1_000_000);
        assert!(result.should_update_judge);
        assert!(result.has_keylog);

        // At 3.5s, should replay entries 2 and 3
        let result = jt.tick(3_500_000);
        assert_eq!(result.replay_events.len(), 2);
        assert_eq!(result.replay_events[0].keycode, 1);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[1].keycode, 0);
        assert!(!result.replay_events[1].pressed);
    }

    #[test]
    fn test_judge_thread_tick_with_margin_time() {
        let keylog = vec![ReplayKeylogEntry {
            time: 1_000_000,
            keycode: 0,
            pressed: true,
        }];
        // margin_time = 500ms = 500_000us
        let mut jt = JudgeThread::new(10_000_000, Some(keylog), 500);

        // At 1.0s: keylog[0].time(1_000_000) + margin(500_000) = 1_500_000 > 1_000_000
        // So entry should NOT be replayed yet
        let result = jt.tick(1_000_000);
        assert!(result.replay_events.is_empty());

        // At 1.5s: 1_000_000 + 500_000 = 1_500_000 <= 1_500_000 -> replay
        let result = jt.tick(1_500_000);
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].time, 1_500_000); // time + margin
    }

    #[test]
    fn test_judge_thread_frametime_tracking() {
        let mut jt = JudgeThread::new(10_000_000, None, 0);
        assert_eq!(jt.get_frametime(), 1);

        jt.tick(1_000_000);
        jt.tick(1_100_000); // delta = 100_000
        assert_eq!(jt.get_frametime(), 100_000);

        jt.tick(1_150_000); // delta = 50_000 (less than previous max)
        assert_eq!(jt.get_frametime(), 100_000); // max stays

        jt.tick(1_400_000); // delta = 250_000
        assert_eq!(jt.get_frametime(), 250_000);
    }

    // --- KeyInputProccessor tests ---

    #[test]
    fn test_start_judge_sets_state() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert!(!proc.is_judge_started());
        proc.start_judge(10_000_000, None, 0);
        assert!(proc.is_judge_started());
        assert!(proc.judge.is_some());
    }

    #[test]
    fn test_start_judge_with_keylog() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 1_000_000,
                keycode: 0,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 2_000_000,
                keycode: 0,
                pressed: false,
            },
        ];
        proc.start_judge(10_000_000, Some(&keylog), 100);
        assert!(proc.is_judge_started());
        // Verify keylog was converted
        let judge = proc.judge.as_ref().unwrap();
        assert!(judge.keylog.is_some());
        assert_eq!(judge.keylog.as_ref().unwrap().len(), 2);
        assert_eq!(judge.micro_margin_time, 100_000);
    }

    #[test]
    fn test_stop_judge_clears_state() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        assert!(proc.is_judge_started());

        proc.stop_judge();
        assert!(!proc.is_judge_started());
        assert!(proc.judge.is_none());
        assert!(proc.key_beam_stop);
    }

    #[test]
    fn test_tick_judge_returns_none_when_not_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert!(proc.tick_judge(1_000_000).is_none());
    }

    #[test]
    fn test_tick_judge_returns_result_when_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        let result = proc.tick_judge(1_000_000);
        assert!(result.is_some());
        let result = result.unwrap();
        assert!(result.should_update_judge);
        assert!(!result.finished);
    }

    #[test]
    fn test_tick_judge_replays_dto_keylog() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 500_000,
                keycode: 2,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 1_500_000,
                keycode: 3,
                pressed: false,
            },
        ];
        proc.start_judge(10_000_000, Some(&keylog), 0);

        let result = proc.tick_judge(1_000_000).unwrap();
        assert_eq!(result.replay_events.len(), 1);
        assert_eq!(result.replay_events[0].keycode, 2);
        assert!(result.replay_events[0].pressed);
        assert_eq!(result.replay_events[0].time, 500_000);
    }

    #[test]
    fn test_full_replay_sequence() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let keylog = vec![
            DtoKeyInputLog {
                time: 100_000,
                keycode: 0,
                pressed: true,
            },
            DtoKeyInputLog {
                time: 200_000,
                keycode: 0,
                pressed: false,
            },
        ];
        // last timeline at 500_000us
        proc.start_judge(500_000, Some(&keylog), 0);

        // Tick at 150_000: replay first entry
        let r = proc.tick_judge(150_000).unwrap();
        assert_eq!(r.replay_events.len(), 1);
        assert!(r.should_update_judge);
        assert!(!r.finished);
        assert!(r.has_keylog);

        // Tick at 250_000: replay second entry
        let r = proc.tick_judge(250_000).unwrap();
        assert_eq!(r.replay_events.len(), 1);
        assert!(!r.replay_events[0].pressed);

        // Tick past end: last_time = 500_000 + TIME_MARGIN * 1000 = 5_500_000
        let r = proc.tick_judge(5_500_000).unwrap();
        assert!(r.finished);
        assert!(r.has_keylog);
    }
}
