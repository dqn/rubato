/// Timer data carrier for skin rendering -- implements TimerAccess from rubato-types.
///
/// Holds current time and per-timer-id activation times (snapshot from TimerManager).
/// Previously returned 0 for all per-timer queries (frozen animations).
#[derive(Clone, Debug, Default)]
pub struct Timer {
    pub now_time: i64,
    pub now_micro_time: i64,
    /// Per-timer-id activation times. Index = timer_id, value = micro-time when set
    /// (i64::MIN = OFF). Populated from TimerManager's timer array.
    timers: Vec<i64>,
}

impl Timer {
    /// Create a Timer with time values and a timer array snapshot.
    pub fn with_timers(now_time: i64, now_micro_time: i64, timers: Vec<i64>) -> Self {
        Self {
            now_time,
            now_micro_time,
            timers,
        }
    }

    /// Set the activation time for a specific timer ID.
    /// Grows the timers array as needed (new entries default to i64::MIN = OFF).
    pub fn set_timer_value(&mut self, timer_id: i32, micro_time: i64) {
        if timer_id < 0 {
            return;
        }
        let idx = timer_id as usize;
        if idx >= self.timers.len() {
            self.timers.resize(idx + 1, i64::MIN);
        }
        self.timers[idx] = micro_time;
    }

    pub fn now_time(&self) -> i64 {
        self.now_time
    }

    pub fn now_micro_time(&self) -> i64 {
        self.now_micro_time
    }

    pub fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        let raw = timer_id.as_i32();
        if raw >= 0 && (raw as usize) < self.timers.len() {
            self.timers[raw as usize]
        } else {
            i64::MIN
        }
    }

    pub fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        self.micro_timer(timer_id) / 1000
    }

    pub fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        if self.is_timer_on(timer_id) {
            (self.now_micro_time - self.micro_timer(timer_id)) / 1000
        } else {
            0
        }
    }

    pub fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        self.micro_timer(timer_id) != i64::MIN
    }
}

impl rubato_types::skin_render_context::SkinRenderContext for Timer {}

impl rubato_types::timer_access::TimerAccess for Timer {
    fn now_time(&self) -> i64 {
        self.now_time
    }
    fn now_micro_time(&self) -> i64 {
        self.now_micro_time
    }
    fn micro_timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::micro_timer(self, timer_id)
    }
    fn timer(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::timer(self, timer_id)
    }
    fn now_time_for(&self, timer_id: rubato_types::timer_id::TimerId) -> i64 {
        Timer::now_time_for(self, timer_id)
    }
    fn is_timer_on(&self, timer_id: rubato_types::timer_id::TimerId) -> bool {
        Timer::is_timer_on(self, timer_id)
    }
}
