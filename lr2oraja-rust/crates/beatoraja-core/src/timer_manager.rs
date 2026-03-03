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
    /// Current main state type, set by MainController before skin draw
    state_type: Option<beatoraja_types::main_state_type::MainStateType>,
    /// Recent judge timing offsets (milliseconds), set by BMSPlayer::render()
    recent_judges: Vec<i64>,
    /// Current write index into recent_judges circular buffer
    recent_judges_index: usize,
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
            state_type: None,
            recent_judges: Vec::new(),
            recent_judges_index: 0,
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

    pub fn timer_values(&self) -> &[i64] {
        &self.timer
    }

    pub fn set_main_state(&mut self) {
        // Reset all timers
        for t in self.timer.iter_mut() {
            *t = i64::MIN;
        }
        self.starttime = Instant::now();
        self.nowmicrotime = self.starttime.elapsed().as_micros() as i64;
    }

    pub fn set_state_type(
        &mut self,
        state_type: Option<beatoraja_types::main_state_type::MainStateType>,
    ) {
        self.state_type = state_type;
    }

    pub fn set_recent_judges(&mut self, index: usize, judges: &[i64]) {
        self.recent_judges_index = index;
        self.recent_judges.clear();
        self.recent_judges.extend_from_slice(judges);
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

impl beatoraja_types::skin_render_context::SkinRenderContext for TimerManager {
    fn current_state_type(&self) -> Option<beatoraja_types::main_state_type::MainStateType> {
        self.state_type
    }

    fn get_recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }

    fn get_recent_judges_index(&self) -> usize {
        self.recent_judges_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_timer_off() {
        let tm = TimerManager::new();
        assert!(!tm.is_timer_on(0));
        assert_eq!(tm.get_micro_timer(0), i64::MIN);
    }

    #[test]
    fn set_timer_on_then_off() {
        let mut tm = TimerManager::new();
        tm.set_timer_on(0);
        assert!(tm.is_timer_on(0));

        tm.set_timer_off(0);
        assert!(!tm.is_timer_on(0));
    }

    #[test]
    fn switch_timer_idempotency() {
        let mut tm = TimerManager::new();
        // Simulate time progression by setting nowmicrotime directly
        tm.nowmicrotime = 1000;
        tm.set_timer_on(5); // timer[5] = 1000

        tm.nowmicrotime = 5000;
        tm.switch_timer(5, true); // should NOT reset timer[5]
        assert_eq!(tm.get_micro_timer(5), 1000); // still original value

        tm.switch_timer(5, false);
        assert!(!tm.is_timer_on(5));
    }

    #[test]
    fn switch_timer_turns_on_when_off() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 3000;
        tm.switch_timer(10, true);
        assert!(tm.is_timer_on(10));
        assert_eq!(tm.get_micro_timer(10), 3000);
    }

    #[test]
    fn get_micro_timer_negative_id() {
        let tm = TimerManager::new();
        assert_eq!(tm.get_micro_timer(-1), i64::MIN);
    }

    #[test]
    fn get_micro_timer_out_of_bounds() {
        let tm = TimerManager::new();
        // TIMER_COUNT = 3000, so index 3000 is out of bounds
        assert_eq!(tm.get_micro_timer(3000), i64::MIN);
    }

    #[test]
    fn get_micro_timer_max_valid_index() {
        let tm = TimerManager::new();
        // Index 2999 is valid but timer is off
        assert_eq!(tm.get_micro_timer(2999), i64::MIN);
    }

    #[test]
    fn get_now_time_for_id_timer_on() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 10000;
        tm.set_timer_on(1); // timer[1] = 10000
        tm.nowmicrotime = 15000;
        // (15000 - 10000) / 1000 = 5
        assert_eq!(tm.get_now_time_for_id(1), 5);
    }

    #[test]
    fn get_now_time_for_id_timer_off() {
        let tm = TimerManager::new();
        assert_eq!(tm.get_now_time_for_id(2), 0);
    }

    #[test]
    fn set_main_state_resets_all_timers() {
        let mut tm = TimerManager::new();
        tm.set_timer_on(0);
        tm.set_timer_on(100);
        tm.set_timer_on(2999);
        assert!(tm.is_timer_on(0));
        assert!(tm.is_timer_on(100));
        assert!(tm.is_timer_on(2999));

        tm.set_main_state();
        assert!(!tm.is_timer_on(0));
        assert!(!tm.is_timer_on(100));
        assert!(!tm.is_timer_on(2999));
    }

    #[test]
    fn get_now_time() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 5000;
        assert_eq!(tm.get_now_time(), 5);
    }

    #[test]
    fn get_now_micro_time() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 12345;
        assert_eq!(tm.get_now_micro_time(), 12345);
    }

    #[test]
    fn set_micro_timer_out_of_bounds_is_no_op() {
        let mut tm = TimerManager::new();
        // Should not panic
        tm.set_micro_timer(-1, 100);
        tm.set_micro_timer(3000, 100);
    }

    #[test]
    fn frozen_prevents_time_update() {
        let mut tm = TimerManager::new();
        tm.set_frozen(true);
        let before = tm.get_now_micro_time();
        tm.update();
        assert_eq!(tm.get_now_micro_time(), before);
    }

    #[test]
    fn default_matches_new() {
        let from_new = TimerManager::new();
        let from_default = TimerManager::default();
        assert_eq!(from_new.get_now_time(), from_default.get_now_time());
        assert_eq!(from_new.timer.len(), from_default.timer.len());
    }
}
