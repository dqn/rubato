use std::time::Instant;

/// TimerManager - manages timing for the application
///
/// All timing uses microseconds internally.
/// Java uses System.nanoTime() / 1000 for microsecond precision.
pub struct TimerManager {
    /// State start time (nanos from Instant)
    starttime: Instant,
    /// Current microsecond time (relative to starttime)
    nowmicrotime: i64,
    /// Whether time updates are frozen
    frozen: bool,
    /// Timer array, indexed by timer ID
    timer: Vec<i64>,
}

/// SkinProperty.TIMER_MAX + 1 (Java TIMER_MAX = 2999)
pub const TIMER_COUNT: usize = 3000;

impl TimerManager {
    pub fn new() -> Self {
        let mut timer = vec![i64::MIN; TIMER_COUNT];
        // Initialize all timers to OFF (Long.MIN_VALUE equivalent)
        for t in timer.iter_mut() {
            *t = i64::MIN;
        }
        Self {
            starttime: Instant::now(),
            nowmicrotime: 0,
            frozen: false,
            timer,
        }
    }

    pub fn get_start_time(&self) -> i64 {
        // starttime / 1000000 in Java (nanos to millis)
        // In Rust we track relative time, so this returns 0 conceptually
        0
    }

    pub fn get_start_micro_time(&self) -> i64 {
        // starttime / 1000 in Java (nanos to micros)
        0
    }

    pub fn get_now_time(&self) -> i64 {
        self.nowmicrotime / 1000
    }

    pub fn get_now_time_for_id(&self, id: i32) -> i64 {
        if self.is_timer_on(id) {
            (self.nowmicrotime - self.get_micro_timer(id)) / 1000
        } else {
            0
        }
    }

    pub fn get_now_micro_time(&self) -> i64 {
        self.nowmicrotime
    }

    pub fn get_now_micro_time_for_id(&self, id: i32) -> i64 {
        if self.is_timer_on(id) {
            self.nowmicrotime - self.get_micro_timer(id)
        } else {
            0
        }
    }

    pub fn get_timer(&self, id: i32) -> i64 {
        self.get_micro_timer(id) / 1000
    }

    pub fn get_micro_timer(&self, id: i32) -> i64 {
        if id >= 0 && (id as usize) < TIMER_COUNT {
            self.timer[id as usize]
        } else {
            // In Java: current.getSkin().getMicroCustomTimer(id)
            // Phase 5+ dependency - custom skin timers
            i64::MIN
        }
    }

    pub fn is_timer_on(&self, id: i32) -> bool {
        self.get_micro_timer(id) != i64::MIN
    }

    pub fn set_timer_on(&mut self, id: i32) {
        self.set_micro_timer(id, self.nowmicrotime);
    }

    pub fn set_timer_off(&mut self, id: i32) {
        self.set_micro_timer(id, i64::MIN);
    }

    pub fn set_micro_timer(&mut self, id: i32, microtime: i64) {
        if id >= 0 && (id as usize) < TIMER_COUNT {
            self.timer[id as usize] = microtime;
        } else {
            // In Java: current.getSkin().setMicroCustomTimer(id, microtime)
            // Phase 5+ dependency - custom skin timers
        }
    }

    pub fn switch_timer(&mut self, id: i32, on: bool) {
        if on {
            if self.get_micro_timer(id) == i64::MIN {
                let now = self.nowmicrotime;
                self.set_micro_timer(id, now);
            }
        } else {
            self.set_micro_timer(id, i64::MIN);
        }
    }

    pub fn set_main_state(&mut self) {
        // Reset all timers
        for t in self.timer.iter_mut() {
            *t = i64::MIN;
        }
        self.starttime = Instant::now();
        self.nowmicrotime = self.starttime.elapsed().as_micros() as i64;
    }

    pub fn set_frozen(&mut self, freeze: bool) {
        self.frozen = freeze;
    }

    pub fn update(&mut self) {
        if !self.frozen {
            self.nowmicrotime = self.starttime.elapsed().as_micros() as i64;
        }
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

impl beatoraja_types::timer_access::TimerAccess for TimerManager {
    fn get_now_time(&self) -> i64 {
        self.get_now_time()
    }

    fn get_now_micro_time(&self) -> i64 {
        self.get_now_micro_time()
    }

    fn get_micro_timer(&self, timer_id: i32) -> i64 {
        self.get_micro_timer(timer_id)
    }

    fn get_timer(&self, timer_id: i32) -> i64 {
        self.get_timer(timer_id)
    }

    fn get_now_time_for(&self, timer_id: i32) -> i64 {
        self.get_now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: i32) -> bool {
        self.is_timer_on(timer_id)
    }
}
