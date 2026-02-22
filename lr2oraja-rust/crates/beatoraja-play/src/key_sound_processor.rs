use bms_model::bms_model::BMSModel;

/// BG lane autoplay thread state.
/// Corresponds to Java AutoplayThread inner class of KeySoundProcessor.
struct AutoplayThread {
    stop: bool,
    starttime: i64,
    /// Indices of timelines that have BG notes
    timeline_micro_times: Vec<i64>,
}

impl AutoplayThread {
    /// Create a new AutoplayThread filtering timelines with BG notes.
    /// Corresponds to Java AutoplayThread(BMSModel model, long starttime).
    fn new(model: &BMSModel, starttime: i64) -> Self {
        let mut timeline_micro_times = Vec::new();
        for tl in model.get_all_time_lines() {
            if !tl.get_back_ground_notes().is_empty() {
                timeline_micro_times.push(tl.get_micro_time());
            }
        }
        AutoplayThread {
            stop: false,
            starttime,
            timeline_micro_times,
        }
    }

    /// Run the autoplay thread.
    /// Corresponds to Java AutoplayThread.run().
    fn run(&mut self) {
        // TODO: Phase 7+ dependency - requires BMSPlayer.timer, AudioDriver, Config
        // In Java:
        // 1. Find starting position from starttime
        // 2. Loop while !stop
        // 3. Get current micro time from TIMER_PLAY
        // 4. Get volume (adjusted or config.bgvolume)
        // 5. Play all BG notes in timelines up to current time
        // 6. Sleep until next timeline
        // 7. Break when past last timeline time
    }
}

/// Key sound processor for BG lane playback
pub struct KeySoundProcessor {
    auto_thread: Option<AutoplayThread>,
}

impl Default for KeySoundProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl KeySoundProcessor {
    pub fn new() -> Self {
        KeySoundProcessor { auto_thread: None }
    }

    pub fn start_bg_play(&mut self, model: &BMSModel, starttime: i64) {
        self.auto_thread = Some(AutoplayThread::new(model, starttime));
        // TODO: Phase 7+ dependency - actually start the thread/task
    }

    pub fn stop_bg_play(&mut self) {
        if let Some(ref mut thread) = self.auto_thread {
            thread.stop = true;
        }
    }
}
