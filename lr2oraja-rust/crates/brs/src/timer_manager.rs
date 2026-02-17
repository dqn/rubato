// TimerManager — ported from Java TimerManager.java.
//
// Manages per-state timers using integer microseconds (i64).
// Standard timers use array indices 0..TIMER_COUNT, custom timers
// (id >= 3000) are deferred to skin layer in later sub-phases.

use std::time::Instant;

use bms_skin::property_id::TIMER_MAX;

/// Number of standard timer slots (0..=TIMER_MAX).
const TIMER_COUNT: usize = (TIMER_MAX + 1) as usize;

/// Sentinel value indicating a timer is OFF (matches Java Long.MIN_VALUE).
pub const TIMER_OFF: i64 = i64::MIN;

/// Manages timers for the current game state.
///
/// Each state transition resets all timers and the start time.
/// Timer values store the absolute microsecond time at which they were activated.
pub struct TimerManager {
    start_instant: Instant,
    now_micro_time: i64,
    frozen: bool,
    timers: Box<[i64; TIMER_COUNT]>,
}

impl TimerManager {
    pub fn new() -> Self {
        let mut timers = Box::new([0i64; TIMER_COUNT]);
        timers.fill(TIMER_OFF);
        Self {
            start_instant: Instant::now(),
            now_micro_time: 0,
            frozen: false,
            timers,
        }
    }

    /// Resets all timers and the start time. Called on state transition.
    /// Corresponds to Java `setMainState()`.
    pub fn reset(&mut self) {
        self.timers.fill(TIMER_OFF);
        self.start_instant = Instant::now();
        self.now_micro_time = 0;
    }

    /// Updates `now_micro_time` from elapsed wall-clock time.
    /// Has no effect when frozen.
    pub fn update(&mut self) {
        if !self.frozen {
            self.now_micro_time = self.start_instant.elapsed().as_micros() as i64;
        }
    }

    /// Returns the current time in milliseconds since state start.
    pub fn now_time(&self) -> i64 {
        self.now_micro_time / 1000
    }

    /// Returns the current time in microseconds since state start.
    pub fn now_micro_time(&self) -> i64 {
        self.now_micro_time
    }

    /// Returns elapsed milliseconds since the given timer was activated.
    /// Returns 0 if the timer is OFF.
    pub fn now_time_of(&self, id: i32) -> i64 {
        if self.is_timer_on(id) {
            (self.now_micro_time - self.micro_timer(id)) / 1000
        } else {
            0
        }
    }

    /// Returns elapsed microseconds since the given timer was activated.
    /// Returns 0 if the timer is OFF.
    #[allow(dead_code)] // Used in tests
    pub fn now_micro_time_of(&self, id: i32) -> i64 {
        if self.is_timer_on(id) {
            self.now_micro_time - self.micro_timer(id)
        } else {
            0
        }
    }

    /// Returns the raw timer value (absolute microsecond time of activation).
    /// Returns `TIMER_OFF` for inactive timers or out-of-range IDs.
    pub fn micro_timer(&self, id: i32) -> i64 {
        if id >= 0 && (id as usize) < TIMER_COUNT {
            self.timers[id as usize]
        } else {
            // Custom timers (id >= TIMER_COUNT) delegated to skin in later phases
            TIMER_OFF
        }
    }

    /// Returns true if the timer is active (not `TIMER_OFF`).
    pub fn is_timer_on(&self, id: i32) -> bool {
        self.micro_timer(id) != TIMER_OFF
    }

    /// Activates a timer, setting it to the current microsecond time.
    pub fn set_timer_on(&mut self, id: i32) {
        self.set_micro_timer(id, self.now_micro_time);
    }

    /// Deactivates a timer, setting it to `TIMER_OFF`.
    #[allow(dead_code)] // Used in tests
    pub fn set_timer_off(&mut self, id: i32) {
        self.set_micro_timer(id, TIMER_OFF);
    }

    /// Sets a timer to a specific microsecond value.
    pub fn set_micro_timer(&mut self, id: i32, micro_time: i64) {
        if id >= 0 && (id as usize) < TIMER_COUNT {
            self.timers[id as usize] = micro_time;
        }
        // Custom timer delegation deferred to later sub-phases
    }

    /// Activates a timer only if it is currently OFF (when `on` is true).
    /// Deactivates unconditionally when `on` is false.
    pub fn switch_timer(&mut self, id: i32, on: bool) {
        if on {
            if self.micro_timer(id) == TIMER_OFF {
                self.set_micro_timer(id, self.now_micro_time);
            }
        } else {
            self.set_micro_timer(id, TIMER_OFF);
        }
    }

    /// Sets or clears the frozen state. When frozen, `update()` does not advance time.
    #[allow(dead_code)] // TODO: integrate with pause/resume system
    pub fn set_frozen(&mut self, frozen: bool) {
        self.frozen = frozen;
    }

    /// Returns whether the timer is frozen.
    #[allow(dead_code)] // Used in tests
    pub fn is_frozen(&self) -> bool {
        self.frozen
    }

    /// Sets `now_micro_time` directly. Used for testing.
    #[cfg(test)]
    pub(crate) fn set_now_micro_time(&mut self, us: i64) {
        self.now_micro_time = us;
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_skin::property_id::{TIMER_FADEOUT, TIMER_STARTINPUT};

    #[test]
    fn new_timers_are_off() {
        let tm = TimerManager::new();
        assert!(!tm.is_timer_on(TIMER_STARTINPUT));
        assert!(!tm.is_timer_on(TIMER_FADEOUT));
        assert_eq!(tm.micro_timer(0), TIMER_OFF);
    }

    #[test]
    fn set_timer_on_and_off() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(5000);
        tm.set_timer_on(TIMER_STARTINPUT);
        assert!(tm.is_timer_on(TIMER_STARTINPUT));
        assert_eq!(tm.micro_timer(TIMER_STARTINPUT), 5000);

        tm.set_timer_off(TIMER_STARTINPUT);
        assert!(!tm.is_timer_on(TIMER_STARTINPUT));
    }

    #[test]
    fn switch_timer_only_sets_once() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(1000);
        tm.switch_timer(TIMER_FADEOUT, true);
        assert_eq!(tm.micro_timer(TIMER_FADEOUT), 1000);

        // Switching on again should NOT update the value
        tm.set_now_micro_time(2000);
        tm.switch_timer(TIMER_FADEOUT, true);
        assert_eq!(tm.micro_timer(TIMER_FADEOUT), 1000);

        // Switching off deactivates
        tm.switch_timer(TIMER_FADEOUT, false);
        assert!(!tm.is_timer_on(TIMER_FADEOUT));
    }

    #[test]
    fn now_time_of_returns_elapsed() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(10_000);
        tm.set_timer_on(TIMER_STARTINPUT);

        tm.set_now_micro_time(15_000);
        assert_eq!(tm.now_micro_time_of(TIMER_STARTINPUT), 5_000);
        assert_eq!(tm.now_time_of(TIMER_STARTINPUT), 5);
    }

    #[test]
    fn now_time_of_returns_zero_when_off() {
        let tm = TimerManager::new();
        assert_eq!(tm.now_time_of(TIMER_STARTINPUT), 0);
        assert_eq!(tm.now_micro_time_of(TIMER_STARTINPUT), 0);
    }

    #[test]
    fn now_time_returns_ms() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(12_345);
        assert_eq!(tm.now_time(), 12);
        assert_eq!(tm.now_micro_time(), 12_345);
    }

    #[test]
    fn frozen_prevents_update() {
        let mut tm = TimerManager::new();
        tm.set_frozen(true);
        assert!(tm.is_frozen());

        let before = tm.now_micro_time();
        tm.update();
        assert_eq!(tm.now_micro_time(), before);
    }

    #[test]
    fn update_advances_time() {
        let mut tm = TimerManager::new();
        std::thread::sleep(std::time::Duration::from_millis(5));
        tm.update();
        assert!(tm.now_micro_time() > 0);
    }

    #[test]
    fn reset_clears_all_timers() {
        let mut tm = TimerManager::new();
        tm.set_now_micro_time(1000);
        tm.set_timer_on(TIMER_STARTINPUT);
        tm.set_timer_on(TIMER_FADEOUT);

        tm.reset();
        assert!(!tm.is_timer_on(TIMER_STARTINPUT));
        assert!(!tm.is_timer_on(TIMER_FADEOUT));
        assert_eq!(tm.now_micro_time(), 0);
    }

    #[test]
    fn out_of_range_id_is_off() {
        let tm = TimerManager::new();
        assert!(!tm.is_timer_on(-1));
        assert!(!tm.is_timer_on(TIMER_MAX + 1));
        assert_eq!(tm.micro_timer(-1), TIMER_OFF);
    }

    #[test]
    fn set_micro_timer_out_of_range_is_no_op() {
        let mut tm = TimerManager::new();
        tm.set_micro_timer(-1, 1000);
        assert_eq!(tm.micro_timer(-1), TIMER_OFF);
        tm.set_micro_timer(TIMER_MAX + 1, 1000);
        assert_eq!(tm.micro_timer(TIMER_MAX + 1), TIMER_OFF);
    }

    #[test]
    fn set_micro_timer_direct() {
        let mut tm = TimerManager::new();
        tm.set_micro_timer(100, 42_000);
        assert!(tm.is_timer_on(100));
        assert_eq!(tm.micro_timer(100), 42_000);
    }
}
