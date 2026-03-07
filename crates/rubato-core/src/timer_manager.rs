use std::time::Instant;

use rubato_types::timer_id::TimerId;

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
    pub frozen: bool,
    /// Timer array, indexed by timer ID
    timer: Vec<i64>,
    /// Current main state type, set by MainController before skin draw
    pub state_type: Option<rubato_types::main_state_type::MainStateType>,
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

    pub fn start_time(&self) -> i64 {
        // starttime / 1000000 in Java (nanos to millis)
        // In Rust we track relative time, so this returns 0 conceptually
        0
    }

    pub fn start_micro_time(&self) -> i64 {
        // starttime / 1000 in Java (nanos to micros)
        0
    }

    pub fn now_time(&self) -> i64 {
        self.nowmicrotime / 1000
    }

    pub fn now_time_for_id(&self, id: TimerId) -> i64 {
        if self.is_timer_on(id) {
            (self.nowmicrotime - self.micro_timer(id)) / 1000
        } else {
            0
        }
    }

    pub fn now_micro_time(&self) -> i64 {
        self.nowmicrotime
    }

    pub fn now_micro_time_for_id(&self, id: TimerId) -> i64 {
        if self.is_timer_on(id) {
            self.nowmicrotime - self.micro_timer(id)
        } else {
            0
        }
    }

    pub fn timer(&self, id: TimerId) -> i64 {
        self.micro_timer(id) / 1000
    }

    /// Export a clone of the timer array for creating skin Timer snapshots.
    pub fn export_timer_array(&self) -> Vec<i64> {
        self.timer.clone()
    }

    pub fn micro_timer(&self, id: TimerId) -> i64 {
        let raw = id.as_i32();
        if raw >= 0 && (raw as usize) < TIMER_COUNT {
            self.timer[raw as usize]
        } else {
            // In Java: current.getSkin().getMicroCustomTimer(id)
            // Phase 5+ dependency - custom skin timers
            i64::MIN
        }
    }

    pub fn is_timer_on(&self, id: TimerId) -> bool {
        self.micro_timer(id) != i64::MIN
    }

    pub fn set_timer_on(&mut self, id: TimerId) {
        self.set_micro_timer(id, self.nowmicrotime);
    }

    pub fn set_timer_off(&mut self, id: TimerId) {
        self.set_micro_timer(id, i64::MIN);
    }

    pub fn set_micro_timer(&mut self, id: TimerId, microtime: i64) {
        let raw = id.as_i32();
        if raw >= 0 && (raw as usize) < TIMER_COUNT {
            self.timer[raw as usize] = microtime;
        } else {
            // In Java: current.getSkin().setMicroCustomTimer(id, microtime)
            // Phase 5+ dependency - custom skin timers
        }
    }

    pub fn switch_timer(&mut self, id: TimerId, on: bool) {
        if on {
            if self.micro_timer(id) == i64::MIN {
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
    pub fn set_recent_judges(&mut self, index: usize, judges: &[i64]) {
        self.recent_judges_index = index;
        self.recent_judges.clear();
        self.recent_judges.extend_from_slice(judges);
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

impl rubato_types::timer_access::TimerAccess for TimerManager {
    fn now_time(&self) -> i64 {
        self.now_time()
    }

    fn now_micro_time(&self) -> i64 {
        self.now_micro_time()
    }

    fn micro_timer(&self, timer_id: TimerId) -> i64 {
        self.micro_timer(timer_id)
    }

    fn timer(&self, timer_id: TimerId) -> i64 {
        self.timer(timer_id)
    }

    fn now_time_for(&self, timer_id: TimerId) -> i64 {
        self.now_time_for_id(timer_id)
    }

    fn is_timer_on(&self, timer_id: TimerId) -> bool {
        self.is_timer_on(timer_id)
    }
}

impl rubato_types::skin_render_context::SkinEventHandler for TimerManager {}
impl rubato_types::skin_render_context::SkinAudioControl for TimerManager {}
impl rubato_types::skin_render_context::SkinConfigAccess for TimerManager {}
impl rubato_types::skin_render_context::SkinPropertyProvider for TimerManager {}

impl rubato_types::skin_render_context::SkinStateQuery for TimerManager {
    fn current_state_type(&self) -> Option<rubato_types::main_state_type::MainStateType> {
        self.state_type
    }

    fn recent_judges(&self) -> &[i64] {
        &self.recent_judges
    }

    fn recent_judges_index(&self) -> usize {
        self.recent_judges_index
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_state_timer_off() {
        let tm = TimerManager::new();
        assert!(!tm.is_timer_on(TimerId::new(0)));
        assert_eq!(tm.micro_timer(TimerId::new(0)), i64::MIN);
    }

    #[test]
    fn set_timer_on_then_off() {
        let mut tm = TimerManager::new();
        tm.set_timer_on(TimerId::new(0));
        assert!(tm.is_timer_on(TimerId::new(0)));

        tm.set_timer_off(TimerId::new(0));
        assert!(!tm.is_timer_on(TimerId::new(0)));
    }

    #[test]
    fn switch_timer_idempotency() {
        let mut tm = TimerManager::new();
        // Simulate time progression by setting nowmicrotime directly
        tm.nowmicrotime = 1000;
        tm.set_timer_on(TimerId::new(5)); // timer[5] = 1000

        tm.nowmicrotime = 5000;
        tm.switch_timer(TimerId::new(5), true); // should NOT reset timer[5]
        assert_eq!(tm.micro_timer(TimerId::new(5)), 1000); // still original value

        tm.switch_timer(TimerId::new(5), false);
        assert!(!tm.is_timer_on(TimerId::new(5)));
    }

    #[test]
    fn switch_timer_turns_on_when_off() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 3000;
        tm.switch_timer(TimerId::new(10), true);
        assert!(tm.is_timer_on(TimerId::new(10)));
        assert_eq!(tm.micro_timer(TimerId::new(10)), 3000);
    }

    #[test]
    fn get_micro_timer_negative_id() {
        let tm = TimerManager::new();
        assert_eq!(tm.micro_timer(TimerId::new(-1)), i64::MIN);
    }

    #[test]
    fn get_micro_timer_out_of_bounds() {
        let tm = TimerManager::new();
        // TIMER_COUNT = 3000, so index 3000 is out of bounds
        assert_eq!(tm.micro_timer(TimerId::new(3000)), i64::MIN);
    }

    #[test]
    fn get_micro_timer_max_valid_index() {
        let tm = TimerManager::new();
        // Index 2999 is valid but timer is off
        assert_eq!(tm.micro_timer(TimerId::new(2999)), i64::MIN);
    }

    #[test]
    fn get_now_time_for_id_timer_on() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 10000;
        tm.set_timer_on(TimerId::new(1)); // timer[1] = 10000
        tm.nowmicrotime = 15000;
        // (15000 - 10000) / 1000 = 5
        assert_eq!(tm.now_time_for_id(TimerId::new(1)), 5);
    }

    #[test]
    fn get_now_time_for_id_timer_off() {
        let tm = TimerManager::new();
        assert_eq!(tm.now_time_for_id(TimerId::new(2)), 0);
    }

    #[test]
    fn set_main_state_resets_all_timers() {
        let mut tm = TimerManager::new();
        tm.set_timer_on(TimerId::new(0));
        tm.set_timer_on(TimerId::new(100));
        tm.set_timer_on(TimerId::new(2999));
        assert!(tm.is_timer_on(TimerId::new(0)));
        assert!(tm.is_timer_on(TimerId::new(100)));
        assert!(tm.is_timer_on(TimerId::new(2999)));

        tm.set_main_state();
        assert!(!tm.is_timer_on(TimerId::new(0)));
        assert!(!tm.is_timer_on(TimerId::new(100)));
        assert!(!tm.is_timer_on(TimerId::new(2999)));
    }

    #[test]
    fn now_time() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 5000;
        assert_eq!(tm.now_time(), 5);
    }

    #[test]
    fn now_micro_time() {
        let mut tm = TimerManager::new();
        tm.nowmicrotime = 12345;
        assert_eq!(tm.now_micro_time(), 12345);
    }

    #[test]
    fn set_micro_timer_out_of_bounds_is_no_op() {
        let mut tm = TimerManager::new();
        // Should not panic
        tm.set_micro_timer(TimerId::new(-1), 100);
        tm.set_micro_timer(TimerId::new(3000), 100);
    }

    #[test]
    fn frozen_prevents_time_update() {
        let mut tm = TimerManager::new();
        tm.frozen = true;
        let before = tm.now_micro_time();
        tm.update();
        assert_eq!(tm.now_micro_time(), before);
    }

    #[test]
    fn default_matches_new() {
        let from_new = TimerManager::new();
        let from_default = TimerManager::default();
        assert_eq!(from_new.now_time(), from_default.now_time());
        assert_eq!(from_new.timer.len(), from_default.timer.len());
    }
}
