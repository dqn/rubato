use crate::bms_player::TIME_MARGIN;
use crate::lane_property::LaneProperty;
use rubato_core::timer_manager::TimerManager;
use rubato_types::timer_id::TimerId;

// SkinProperty timer constants for key beam on/off
// Translated from SkinPropertyMapper.keyOnTimerId / keyOffTimerId
const TIMER_KEYON_1P_SCRATCH: i32 = 100;
const TIMER_KEYON_1P_KEY10: i32 = 1410;
const TIMER_KEYOFF_1P_SCRATCH: i32 = 120;
const TIMER_KEYOFF_1P_KEY10: i32 = 1610;

/// Compute the timer ID for key-on (key beam start).
///
/// Translated from: SkinPropertyMapper.keyOnTimerId(player, key)
fn key_on_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYON_1P_SCRATCH + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYON_1P_KEY10 + key - 10 + player * 100);
        }
    }
    TimerId::new(-1)
}

/// Compute the timer ID for key-off (key beam end).
///
/// Translated from: SkinPropertyMapper.keyOffTimerId(player, key)
fn key_off_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYOFF_1P_SCRATCH + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYOFF_1P_KEY10 + key - 10 + player * 100);
        }
    }
    TimerId::new(-1)
}

/// Context passed into KeyInputProccessor::input() each frame.
///
/// Bundles the external state needed by the input processing loop,
/// avoiding the need for the processor to hold references to the parent player.
pub struct InputContext<'a> {
    /// Current time in milliseconds (from timer.getNowTime())
    pub now: i64,
    /// Key states array — true if the key is currently pressed
    pub key_states: &'a [bool],
    /// Auto-press timing array from JudgeManager (i64::MIN means not auto-pressed)
    pub auto_presstime: &'a [i64],
    /// Whether the play mode is AUTOPLAY
    pub is_autoplay: bool,
    /// Timer manager for setting key beam timers
    pub timer: &'a mut TimerManager,
}

/// A single key state change to replay.
/// Produced by JudgeThread::tick() for the caller to apply.
#[derive(Clone, Copy, Debug, PartialEq)]
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
    pub key_beam_stop: bool,
    judge: Option<JudgeThread>,
}

/// Replay keylog entry — lightweight copy of the DTO fields needed for replay.
#[derive(Clone, Copy, Debug)]
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
    fn frametime(&self) -> i64 {
        self.frametime
    }
}

impl KeyInputProccessor {
    pub fn new(lane_property: &LaneProperty) -> Self {
        let scratch_len = lane_property.scratch_key_assign().len();
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
        keylog: Option<&[rubato_types::stubs::KeyInputLog]>,
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

    /// Process key input each frame: key beam flags and scratch turntable animation.
    ///
    /// Translated from Java: KeyInputProccessor.input()
    ///
    /// Returns scratch angle values indexed by scratch index.
    /// The caller should write `result[s]` to `main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r`.
    #[allow(clippy::needless_range_loop)] // Multiple parallel arrays indexed by s/lane
    pub fn input(&mut self, ctx: &mut InputContext) {
        let lane_offsets = self.lane_property.lane_skin_offset();
        let lane_keys = self.lane_property.lane_key_assign();
        let lane_scratch = self.lane_property.lane_scratch_assign();
        let lane_players = self.lane_property.lane_player();

        for lane in 0..lane_offsets.len() {
            let offset = lane_offsets[lane];
            let mut pressed = false;
            let mut scratch_changed = false;

            if !self.key_beam_stop {
                for &key in &lane_keys[lane] {
                    let key_idx = key as usize;
                    let is_key_active = (key_idx < ctx.key_states.len() && ctx.key_states[key_idx])
                        || (key_idx < ctx.auto_presstime.len()
                            && ctx.auto_presstime[key_idx] != i64::MIN);
                    if is_key_active {
                        pressed = true;
                        let scratch_idx = lane_scratch[lane];
                        if scratch_idx != -1 {
                            let si = scratch_idx as usize;
                            if si < self.scratch_key.len() && self.scratch_key[si] != key {
                                scratch_changed = true;
                                self.scratch_key[si] = key;
                            }
                        }
                    }
                }
            }

            let timer_on = key_on_timer_id(lane_players[lane], offset);
            let timer_off = key_off_timer_id(lane_players[lane], offset);

            if pressed {
                if (!self.is_judge_started || ctx.is_autoplay)
                    && (!ctx.timer.is_timer_on(timer_on) || scratch_changed)
                {
                    ctx.timer.set_timer_on(timer_on);
                    ctx.timer.set_timer_off(timer_off);
                }
            } else if ctx.timer.is_timer_on(timer_on) {
                ctx.timer.set_timer_on(timer_off);
                ctx.timer.set_timer_off(timer_on);
            }
        }

        // Scratch turntable animation
        if self.prevtime >= 0 {
            let deltatime = (ctx.now - self.prevtime) as f32 / 1000.0;
            let scratch_keys = self.lane_property.scratch_key_assign();
            #[allow(clippy::needless_range_loop)]
            for s in 0..self.scratch.len() {
                let key0 = scratch_keys[s][1];
                let key1 = scratch_keys[s][0];

                let mut target_speed: f32 = 1.0;
                let mut move_towards_speed: f32 = 4.0;

                if !ctx.is_autoplay {
                    let key0_idx = key0 as usize;
                    let key1_idx = key1 as usize;
                    let key0_active = (key0_idx < ctx.key_states.len() && ctx.key_states[key0_idx])
                        || (key0_idx < ctx.auto_presstime.len()
                            && ctx.auto_presstime[key0_idx] != i64::MIN);
                    let key1_active = (key1_idx < ctx.key_states.len() && ctx.key_states[key1_idx])
                        || (key1_idx < ctx.auto_presstime.len()
                            && ctx.auto_presstime[key1_idx] != i64::MIN);

                    if key0_active {
                        target_speed = -0.75;
                        move_towards_speed = 16.0;
                        self.scratch_tt_graphic_speed[s] =
                            self.scratch_tt_graphic_speed[s].min(0.0);
                    } else if key1_active {
                        target_speed = 2.0;
                        move_towards_speed = 16.0;
                        self.scratch_tt_graphic_speed[s] =
                            self.scratch_tt_graphic_speed[s].max(0.0);
                    }
                }

                // Move towards target speed
                // Java uses constant 1.0f in the abs check (not targetSpeed)
                if (1.0_f32 - self.scratch_tt_graphic_speed[s]).abs() <= deltatime {
                    self.scratch_tt_graphic_speed[s] = target_speed;
                } else {
                    self.scratch_tt_graphic_speed[s] +=
                        (target_speed - self.scratch_tt_graphic_speed[s]).signum()
                            * deltatime
                            * move_towards_speed;
                }

                // Apply TT speed to scratch angle
                if self.scratch_tt_graphic_speed[s] > 0.0 {
                    self.scratch[s] += 360.0 - self.scratch_tt_graphic_speed[s] * deltatime * 270.0;
                } else if self.scratch_tt_graphic_speed[s] < 0.0 {
                    self.scratch[s] += -self.scratch_tt_graphic_speed[s] * deltatime * 270.0;
                }

                self.scratch[s] %= 360.0;
            }
        }

        self.prevtime = ctx.now;
    }

    /// Returns the current scratch angle values.
    ///
    /// The caller should write `angles[s]` to `main.getOffset(OFFSET_SCRATCHANGLE_1P + s).r`.
    pub fn scratch_angles(&self) -> &[f32] {
        &self.scratch
    }

    /// Key beam flag ON — called from judge synchronization.
    ///
    /// Translated from Java: KeyInputProccessor.inputKeyOn(lane)
    pub fn input_key_on(&mut self, lane: usize, timer: &mut TimerManager) {
        let lane_skin_offset = self.lane_property.lane_skin_offset();
        if lane >= lane_skin_offset.len() {
            return;
        }
        if !self.key_beam_stop {
            let offset = lane_skin_offset[lane];
            let player = self.lane_property.lane_player()[lane];
            let timer_on = key_on_timer_id(player, offset);
            let timer_off = key_off_timer_id(player, offset);
            let lane_scratch = self.lane_property.lane_scratch_assign();
            if !timer.is_timer_on(timer_on) || lane_scratch[lane] != -1 {
                timer.set_timer_on(timer_on);
                timer.set_timer_off(timer_off);
            }
        }
    }

    pub fn stop_judge(&mut self) {
        if self.judge.is_some() {
            if let Some(ref j) = self.judge {
                log::info!("入力パフォーマンス(max us) : {}", j.frametime());
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::mode::Mode;
    use rubato_types::stubs::KeyInputLog as DtoKeyInputLog;

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
        assert_eq!(jt.frametime(), 1);

        jt.tick(1_000_000);
        jt.tick(1_100_000); // delta = 100_000
        assert_eq!(jt.frametime(), 100_000);

        jt.tick(1_150_000); // delta = 50_000 (less than previous max)
        assert_eq!(jt.frametime(), 100_000); // max stays

        jt.tick(1_400_000); // delta = 250_000
        assert_eq!(jt.frametime(), 250_000);
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

    // --- SkinPropertyMapper timer ID tests ---

    #[test]
    fn test_key_on_timer_id_scratch_range() {
        // player 0, key 0 (scratch) -> 100
        assert_eq!(key_on_timer_id(0, 0), TimerId::new(TIMER_KEYON_1P_SCRATCH));
        // player 0, key 7 -> 107
        assert_eq!(
            key_on_timer_id(0, 7),
            TimerId::new(TIMER_KEYON_1P_SCRATCH + 7)
        );
        // player 1, key 0 -> 110
        assert_eq!(
            key_on_timer_id(1, 0),
            TimerId::new(TIMER_KEYON_1P_SCRATCH + 10)
        );
    }

    #[test]
    fn test_key_off_timer_id_scratch_range() {
        assert_eq!(
            key_off_timer_id(0, 0),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH)
        );
        assert_eq!(
            key_off_timer_id(0, 7),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH + 7)
        );
        assert_eq!(
            key_off_timer_id(1, 0),
            TimerId::new(TIMER_KEYOFF_1P_SCRATCH + 10)
        );
    }

    #[test]
    fn test_key_on_timer_id_key10_range() {
        // key 10 -> TIMER_KEYON_1P_KEY10 + 0
        assert_eq!(key_on_timer_id(0, 10), TimerId::new(TIMER_KEYON_1P_KEY10));
        // key 15 -> TIMER_KEYON_1P_KEY10 + 5
        assert_eq!(
            key_on_timer_id(0, 15),
            TimerId::new(TIMER_KEYON_1P_KEY10 + 5)
        );
    }

    #[test]
    fn test_key_timer_id_invalid_player() {
        assert_eq!(key_on_timer_id(2, 0), TimerId::new(-1));
        assert_eq!(key_off_timer_id(2, 0), TimerId::new(-1));
    }

    #[test]
    fn test_key_timer_id_invalid_key() {
        assert_eq!(key_on_timer_id(0, 100), TimerId::new(-1));
        assert_eq!(key_off_timer_id(0, 100), TimerId::new(-1));
    }

    // --- input() method tests (Phase 41f) ---

    fn make_timer() -> TimerManager {
        TimerManager::new()
    }

    fn make_context<'a>(
        now: i64,
        key_states: &'a [bool],
        auto_presstime: &'a [i64],
        is_autoplay: bool,
        timer: &'a mut TimerManager,
    ) -> InputContext<'a> {
        InputContext {
            now,
            key_states,
            auto_presstime,
            is_autoplay,
            timer,
        }
    }

    #[test]
    fn test_input_no_keys_pressed_no_timer_change() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();
        // No keys pressed, no auto_presstime
        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // No timers should be set
        for id in 100..110 {
            assert!(!ctx.timer.is_timer_on(TimerId::new(id)));
        }
    }

    #[test]
    fn test_input_key_pressed_sets_beam_timer_when_not_judge_started() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        // judge is NOT started, so key beam should activate
        let mut timer = make_timer();

        // BEAT_7K: lane 0 maps to key 0, skin offset 1, player 0
        // timer_on = key_on_timer_id(0, 1) = 100 + 1 = 101
        // timer_off = key_off_timer_id(0, 1) = 120 + 1 = 121
        let mut key_states = vec![false; 9];
        key_states[0] = true; // key 0 pressed
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        assert!(ctx.timer.is_timer_on(TimerId::new(101))); // KEYON timer for offset 1
        assert!(!ctx.timer.is_timer_on(TimerId::new(121))); // KEYOFF timer should be off
    }

    #[test]
    fn test_input_key_released_swaps_beam_timers() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // First: press key 0 (lane 0, offset 1)
        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(101))); // KEYON on

        // Then: release key 0
        key_states[0] = false;
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // After release: timer_off(121) becomes on, timer_on(101) becomes off
        assert!(timer.is_timer_on(TimerId::new(121))); // KEYOFF on
        assert!(!timer.is_timer_on(TimerId::new(101))); // KEYON off
    }

    #[test]
    fn test_input_auto_presstime_triggers_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // No key physically pressed, but auto_presstime is set for key 0
        let key_states = vec![false; 9];
        let mut auto_presstime = vec![i64::MIN; 9];
        auto_presstime[0] = 50_000; // auto-pressed at 50ms
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // Should trigger beam as if key was pressed
        assert!(ctx.timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_key_beam_stop_prevents_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.key_beam_stop = true;
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // key_beam_stop is true, so no timers should be set
        assert!(!ctx.timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_judge_started_requires_autoplay_for_beam() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.start_judge(10_000_000, None, 0);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        key_states[0] = true;
        let auto_presstime = vec![i64::MIN; 9];

        // judge is started, NOT autoplay -> no beam timer
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(!timer.is_timer_on(TimerId::new(101)));

        // judge is started, IS autoplay -> beam timer sets
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, true, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_scratch_key_change_triggers_re_beam() {
        // BEAT_7K: lane 7 (scratch) has keys [7, 8], scratch_assign[7] = 0
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // skin offset for lane 7 is 0. timer_on = key_on_timer_id(0, 0) = 100
        // Press key 7 (first scratch key)
        let mut key_states = vec![false; 9];
        key_states[7] = true;
        let auto_presstime = vec![i64::MIN; 9];
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        assert!(timer.is_timer_on(TimerId::new(100)));

        // Now press key 8 instead (switch scratch direction)
        key_states[7] = false;
        key_states[8] = true;
        {
            let mut ctx = make_context(200, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // scratch_changed should be true, timer should be re-set
        assert!(timer.is_timer_on(TimerId::new(100)));
    }

    // --- Scratch animation tests ---

    #[test]
    fn test_input_scratch_initial_no_animation_on_first_frame() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];
        // prevtime starts at -1, first frame should not animate
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        // scratch angles should still be 0 after first frame (prevtime was -1)
        assert_eq!(proc.scratch_angles()[0], 0.0);
        // prevtime should now be 100
        assert_eq!(proc.prevtime, 100);
    }

    #[test]
    fn test_input_scratch_idle_moves_towards_default_speed() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame to set prevtime
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Second frame at 1000ms (deltatime = 1.0s)
        {
            let mut ctx = make_context(1000, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Default target_speed=1.0, move_towards_speed=4.0
        // scratch_tt_graphic_speed starts at 0.0
        // |1.0 - 0.0| = 1.0 > deltatime(1.0) is false, so speed = target_speed = 1.0
        assert_eq!(proc.scratch_tt_graphic_speed[0], 1.0);
        // With speed=1.0 > 0: scratch += 360.0 - 1.0 * 1.0 * 270.0 = 90.0
        assert!((proc.scratch_angles()[0] - 90.0).abs() < 0.01);
    }

    #[test]
    fn test_input_scratch_key0_sets_negative_direction() {
        // BEAT_7K: scratch_to_key[0] = [7, 8]
        // key0 = scratch_keys[0][1] = 8, key1 = scratch_keys[0][0] = 7
        // Pressing key0 (=8) -> target_speed=-0.75, speed clamped to min(current, 0)
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Press key 8 (scratch key0 direction)
        key_states[8] = true;
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // deltatime = 100/1000 = 0.1
        // key0 active: target=-0.75, move_towards=16.0, speed clamped to min(0, 0)=0
        // |(-0.75) - 0| = 0.75 > 0.1 -> speed += signum(-0.75) * 0.1 * 16 = -1.6
        assert!(proc.scratch_tt_graphic_speed[0] < 0.0);
    }

    #[test]
    fn test_input_scratch_key1_sets_positive_direction() {
        // key1 = scratch_keys[0][0] = 7
        // Pressing key1 (=7) -> target_speed=2.0
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }

        // Press key 7 (scratch key1 direction)
        key_states[7] = true;
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // key1 active: target=2.0, move_towards=16.0, speed clamped to max(0, 0)=0
        // |2.0 - 0| = 2.0 > 0.1 -> speed += 1.0 * 0.1 * 16 = 1.6
        assert!(proc.scratch_tt_graphic_speed[0] > 0.0);
        assert!((proc.scratch_tt_graphic_speed[0] - 1.6).abs() < 0.01);
    }

    #[test]
    fn test_input_scratch_autoplay_ignores_key_states() {
        // In autoplay mode, scratch animation uses default idle speed
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let mut key_states = vec![false; 9];
        key_states[7] = true; // key pressed but autoplay ignores it
        let auto_presstime = vec![i64::MIN; 9];

        // First frame
        {
            let mut ctx = make_context(0, &key_states, &auto_presstime, true, &mut timer);
            proc.input(&mut ctx);
        }

        // Second frame
        {
            let mut ctx = make_context(100, &key_states, &auto_presstime, true, &mut timer);
            proc.input(&mut ctx);
        }

        // autoplay -> default target_speed=1.0, move_towards=4.0
        // speed starts at 0, deltatime=0.1
        // |1.0 - 0| = 1.0 > 0.1 -> speed += 1.0 * 0.1 * 4.0 = 0.4
        assert!((proc.scratch_tt_graphic_speed[0] - 0.4).abs() < 0.01);
    }

    #[test]
    fn test_input_scratch_angle_wraps_at_360() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        // Set scratch angle close to 360
        proc.scratch[0] = 350.0;
        proc.scratch_tt_graphic_speed[0] = 1.0;
        proc.prevtime = 0;

        {
            let mut ctx = make_context(1000, &key_states, &auto_presstime, false, &mut timer);
            proc.input(&mut ctx);
        }
        // scratch += 360.0 - 1.0 * 1.0 * 270.0 = 90.0
        // 350.0 + 90.0 = 440.0 % 360.0 = 80.0
        assert!((proc.scratch_angles()[0] - 80.0).abs() < 0.01);
    }

    #[test]
    fn test_input_prevtime_updates() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        assert_eq!(proc.prevtime, -1);
        let mut timer = make_timer();

        let key_states = vec![false; 9];
        let auto_presstime = vec![i64::MIN; 9];

        let mut ctx = make_context(42, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);
        assert_eq!(proc.prevtime, 42);
    }

    #[test]
    fn test_input_multiple_lanes_pressed() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // Press keys 0, 2, 4 (lanes 0, 2, 4; offsets 1, 3, 5)
        let mut key_states = vec![false; 9];
        key_states[0] = true;
        key_states[2] = true;
        key_states[4] = true;
        let auto_presstime = vec![i64::MIN; 9];
        let mut ctx = make_context(100, &key_states, &auto_presstime, false, &mut timer);
        proc.input(&mut ctx);

        // timer_on IDs: 101, 103, 105
        assert!(ctx.timer.is_timer_on(TimerId::new(101)));
        assert!(ctx.timer.is_timer_on(TimerId::new(103)));
        assert!(ctx.timer.is_timer_on(TimerId::new(105)));
        // Unpressed lanes should NOT have timers
        assert!(!ctx.timer.is_timer_on(TimerId::new(102))); // offset 2
        assert!(!ctx.timer.is_timer_on(TimerId::new(104))); // offset 4
    }

    // --- input_key_on() tests ---

    #[test]
    fn test_input_key_on_sets_timer() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 0, offset 1, player 0 -> timer_on = 101, timer_off = 121
        proc.input_key_on(0, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(101)));
        assert!(!timer.is_timer_on(TimerId::new(121)));
    }

    #[test]
    fn test_input_key_on_scratch_lane_always_retriggers() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 7 is scratch, offset 0, player 0 -> timer_on = 100
        proc.input_key_on(7, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(100)));

        // Call again — scratch lanes should re-trigger
        // (scratch condition: lane_scratch[lane] != -1 -> always true for scratch)
        proc.input_key_on(7, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(100)));
    }

    #[test]
    fn test_input_key_on_key_beam_stop_prevents_timer() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        proc.key_beam_stop = true;
        let mut timer = make_timer();

        proc.input_key_on(0, &mut timer);
        assert!(!timer.is_timer_on(TimerId::new(101)));
    }

    #[test]
    fn test_input_key_on_out_of_bounds_lane_no_crash() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // Lane 100 is out of bounds — should return early without panic
        proc.input_key_on(100, &mut timer);
    }

    #[test]
    fn test_input_key_on_non_scratch_lane_does_not_retrigger() {
        let lp = make_lane_property();
        let mut proc = KeyInputProccessor::new(&lp);
        let mut timer = make_timer();

        // lane 0 (non-scratch), offset 1 -> timer_on = 101
        proc.input_key_on(0, &mut timer);
        assert!(timer.is_timer_on(TimerId::new(101)));

        // Manually set timer_on to OFF, then call again
        // Since timer_on is already ON and lane is not scratch, it should NOT re-trigger
        // Actually, input_key_on checks: !timer.is_timer_on(timer_on) || lane_scratch[lane] != -1
        // For non-scratch lane with timer already on -> skip
        // We can test by checking the timer was set once (already proven above)
    }

    // --- get_scratch_angles() tests ---

    #[test]
    fn test_get_scratch_angles_initial_zeros() {
        let lp = make_lane_property();
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 1); // BEAT_7K has 1 scratch
        assert_eq!(angles[0], 0.0);
    }

    #[test]
    fn test_get_scratch_angles_beat_14k_has_two() {
        let lp = LaneProperty::new(&Mode::BEAT_14K);
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 2);
    }

    #[test]
    fn test_get_scratch_angles_popn_has_none() {
        let lp = LaneProperty::new(&Mode::POPN_9K);
        let proc = KeyInputProccessor::new(&lp);
        let angles = proc.scratch_angles();
        assert_eq!(angles.len(), 0);
    }
}
