use crate::bms_player::TIME_MARGIN;

use super::KeyInputProccessor;

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

/// Replay keylog entry — lightweight copy of the DTO fields needed for replay.
#[derive(Clone, Copy, Debug)]
pub(super) struct ReplayKeylogEntry {
    /// Press time in microseconds
    pub(super) time: i64,
    pub(super) keycode: i32,
    pub(super) pressed: bool,
}

/// Judge thread state — processes replay keylog and drives judge updates.
///
/// Corresponds to Java JudgeThread (inner class of KeyInputProccessor).
/// In Java this extends Thread and runs concurrently. In Rust, we use a
/// synchronous tick-based design where tick() is called each frame from
/// the game loop.
pub(super) struct JudgeThread {
    pub(super) stop: bool,
    pub(super) micro_margin_time: i64,
    /// Micro time of last timeline + TIME_MARGIN * 1000
    pub(super) last_time: i64,
    /// Replay keylog entries (None if no replay)
    pub(super) keylog: Option<Vec<ReplayKeylogEntry>>,
    /// Current index into keylog
    pub(super) index: usize,
    /// Previous tick time for performance tracking
    pub(super) prevtime: i64,
    /// Max frame time (for logging)
    pub(super) frametime: i64,
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
    pub(super) fn new(
        last_timeline_micro_time: i64,
        keylog: Option<Vec<ReplayKeylogEntry>>,
        milli_margin_time: i64,
    ) -> Self {
        let last_time = last_timeline_micro_time + TIME_MARGIN * 1000;
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
    pub(super) fn tick(&mut self, mtime: i64) -> JudgeTickResult {
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
    pub(super) fn frametime(&self) -> i64 {
        self.frametime
    }
}

impl KeyInputProccessor {
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
        keylog: Option<&[rubato_types::KeyInputLog]>,
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
