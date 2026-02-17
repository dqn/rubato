// PomyuChara runtime processor.
//
// Manages timer-based state transitions for PMS character animations
// based on judge results.
//
// Ported from Java: PomyuCharaProcessor.java

use bms_skin::property_id::{
    TIMER_PM_CHARA_1P_BAD, TIMER_PM_CHARA_1P_FEVER, TIMER_PM_CHARA_1P_GOOD,
    TIMER_PM_CHARA_1P_GREAT, TIMER_PM_CHARA_1P_NEUTRAL, TIMER_PM_CHARA_2P_BAD,
    TIMER_PM_CHARA_2P_GREAT, TIMER_PM_CHARA_2P_NEUTRAL, TIMER_PM_CHARA_DANCE,
};

/// Timer check threshold (17ms ~= 1 frame at 60fps).
#[allow(dead_code)] // Used in tests (via update_timer)
const TIMER_THRESHOLD: i64 = 17;

/// Runtime state for PomyuChara animation transitions.
#[allow(dead_code)] // Used in tests
pub struct PomyuCharaState {
    /// Cycle times per motion: [1P_NEUTRAL, 1P_FEVER, 1P_GREAT, 1P_GOOD, 1P_BAD, 2P_NEUTRAL, 2P_GREAT, 2P_BAD]
    pub cycle_times: [i32; 8],
    /// Note count at last neutral start: [1P, 2P]
    pub last_notes: [i32; 2],
    /// Latest judge result (1=PG, 2=GR, 3=GD, 4=BD, 5=PR)
    pub judge: i32,
}

impl Default for PomyuCharaState {
    fn default() -> Self {
        Self {
            cycle_times: [1; 8],
            last_notes: [0; 2],
            judge: 0,
        }
    }
}

impl PomyuCharaState {
    /// Initialize from skin's pomyu_chara_times.
    #[allow(dead_code)] // Used in tests
    pub fn from_skin_times(times: &[i32; 8]) -> Self {
        Self {
            cycle_times: *times,
            ..Default::default()
        }
    }

    /// Get cycle time for a timer index (0-7).
    #[allow(dead_code)] // Used in tests (via update_timer)
    fn get_cycle_time(&self, index: i32) -> i32 {
        if index >= 0 && (index as usize) < self.cycle_times.len() {
            self.cycle_times[index as usize].max(1)
        } else {
            1
        }
    }

    /// Update timers based on judge results and motion completion.
    ///
    /// `timer_on`: closure to check if timer is on
    /// `timer_now`: closure to get elapsed time for a timer
    /// `set_timer_on`: closure to turn on a timer
    /// `set_timer_off`: closure to turn off a timer
    /// `past_notes`: total processed notes
    /// `gauge_is_max`: whether the gauge is at max value
    #[allow(dead_code)] // Used in tests
    pub fn update_timer(
        &mut self,
        timer_on: &impl Fn(i32) -> bool,
        timer_now: &impl Fn(i32) -> i64,
        set_timer_on: &mut impl FnMut(i32),
        set_timer_off: &mut impl FnMut(i32),
        past_notes: i32,
        gauge_is_max: bool,
    ) {
        // 1P neutral completion check
        if timer_on(TIMER_PM_CHARA_1P_NEUTRAL) {
            let elapsed = timer_now(TIMER_PM_CHARA_1P_NEUTRAL);
            let cycle = self.get_cycle_time(0) as i64;
            if elapsed >= cycle
                && elapsed % cycle < TIMER_THRESHOLD
                && self.last_notes[0] != past_notes
                && self.judge > 0
            {
                if self.judge == 1 || self.judge == 2 {
                    if gauge_is_max {
                        set_timer_on(TIMER_PM_CHARA_1P_FEVER);
                    } else {
                        set_timer_on(TIMER_PM_CHARA_1P_GREAT);
                    }
                } else if self.judge == 3 {
                    set_timer_on(TIMER_PM_CHARA_1P_GOOD);
                } else {
                    set_timer_on(TIMER_PM_CHARA_1P_BAD);
                }
                set_timer_off(TIMER_PM_CHARA_1P_NEUTRAL);
            }
        }

        // 2P neutral completion check
        if timer_on(TIMER_PM_CHARA_2P_NEUTRAL) {
            let elapsed = timer_now(TIMER_PM_CHARA_2P_NEUTRAL);
            let cycle =
                self.get_cycle_time(TIMER_PM_CHARA_2P_NEUTRAL - TIMER_PM_CHARA_1P_NEUTRAL) as i64;
            if elapsed >= cycle
                && elapsed % cycle < TIMER_THRESHOLD
                && self.last_notes[1] != past_notes
                && self.judge > 0
            {
                // 2P: GREAT for PG/GR/GD, BAD for BD/PR (reversed from 1P)
                if self.judge >= 1 && self.judge <= 3 {
                    set_timer_on(TIMER_PM_CHARA_2P_BAD);
                } else {
                    set_timer_on(TIMER_PM_CHARA_2P_GREAT);
                }
                set_timer_off(TIMER_PM_CHARA_2P_NEUTRAL);
            }
        }

        // Judge motion completion -> return to neutral
        for timer_id in TIMER_PM_CHARA_1P_FEVER..=TIMER_PM_CHARA_2P_BAD {
            if timer_id == TIMER_PM_CHARA_2P_NEUTRAL {
                continue;
            }
            if timer_on(timer_id) {
                let elapsed = timer_now(timer_id);
                let cycle = self.get_cycle_time(timer_id - TIMER_PM_CHARA_1P_NEUTRAL) as i64;
                if elapsed >= cycle {
                    if timer_id <= TIMER_PM_CHARA_1P_BAD {
                        set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                        self.last_notes[0] = past_notes;
                    } else {
                        set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                        self.last_notes[1] = past_notes;
                    }
                    set_timer_off(timer_id);
                }
            }
        }

        // Dance timer is always active
        if !timer_on(TIMER_PM_CHARA_DANCE) {
            set_timer_on(TIMER_PM_CHARA_DANCE);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    struct MockTimerState {
        timers: RefCell<HashMap<i32, i64>>,
    }

    impl MockTimerState {
        fn new() -> Self {
            Self {
                timers: RefCell::new(HashMap::new()),
            }
        }

        fn set_on(&self, id: i32, time: i64) {
            self.timers.borrow_mut().insert(id, time);
        }

        fn is_on(&self, id: i32) -> bool {
            self.timers.borrow().contains_key(&id)
        }

        fn elapsed(&self, id: i32) -> i64 {
            *self.timers.borrow().get(&id).unwrap_or(&0)
        }
    }

    #[test]
    fn test_default_state() {
        let state = PomyuCharaState::default();
        assert_eq!(state.cycle_times, [1; 8]);
        assert_eq!(state.last_notes, [0; 2]);
        assert_eq!(state.judge, 0);
    }

    #[test]
    fn test_from_skin_times() {
        let times = [100, 200, 300, 400, 500, 100, 200, 300];
        let state = PomyuCharaState::from_skin_times(&times);
        assert_eq!(state.cycle_times, times);
    }

    #[test]
    fn test_dance_timer_always_on() {
        let mut state = PomyuCharaState::default();
        let mock = MockTimerState::new();

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| {
                on_timers.push(id);
                mock.set_on(id, 0);
            },
            &mut |id| {
                off_timers.push(id);
            },
            0,
            false,
        );

        assert!(on_timers.contains(&TIMER_PM_CHARA_DANCE));
    }

    #[test]
    fn test_neutral_to_great_transition() {
        let mut state = PomyuCharaState::default();
        state.cycle_times = [100, 100, 100, 100, 100, 100, 100, 100];
        state.judge = 1; // PERFECT

        let mock = MockTimerState::new();
        mock.set_on(TIMER_PM_CHARA_1P_NEUTRAL, 100); // elapsed >= cycle

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| on_timers.push(id),
            &mut |id| off_timers.push(id),
            1, // past_notes != last_notes[0]
            false,
        );

        assert!(on_timers.contains(&TIMER_PM_CHARA_1P_GREAT));
        assert!(off_timers.contains(&TIMER_PM_CHARA_1P_NEUTRAL));
    }

    #[test]
    fn test_neutral_to_fever_when_max_gauge() {
        let mut state = PomyuCharaState::default();
        state.cycle_times = [100; 8];
        state.judge = 1; // PERFECT

        let mock = MockTimerState::new();
        mock.set_on(TIMER_PM_CHARA_1P_NEUTRAL, 100);

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| on_timers.push(id),
            &mut |id| off_timers.push(id),
            1,
            true, // gauge is max
        );

        assert!(on_timers.contains(&TIMER_PM_CHARA_1P_FEVER));
        assert!(off_timers.contains(&TIMER_PM_CHARA_1P_NEUTRAL));
    }

    #[test]
    fn test_neutral_to_bad() {
        let mut state = PomyuCharaState::default();
        state.cycle_times = [100; 8];
        state.judge = 4; // BAD

        let mock = MockTimerState::new();
        mock.set_on(TIMER_PM_CHARA_1P_NEUTRAL, 100);

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| on_timers.push(id),
            &mut |id| off_timers.push(id),
            1,
            false,
        );

        assert!(on_timers.contains(&TIMER_PM_CHARA_1P_BAD));
    }

    #[test]
    fn test_judge_motion_returns_to_neutral() {
        let mut state = PomyuCharaState::default();
        state.cycle_times = [100; 8];

        let mock = MockTimerState::new();
        mock.set_on(TIMER_PM_CHARA_1P_GREAT, 100); // motion complete

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| on_timers.push(id),
            &mut |id| off_timers.push(id),
            5,
            false,
        );

        assert!(on_timers.contains(&TIMER_PM_CHARA_1P_NEUTRAL));
        assert!(off_timers.contains(&TIMER_PM_CHARA_1P_GREAT));
        assert_eq!(state.last_notes[0], 5);
    }

    #[test]
    fn test_no_transition_when_no_judge() {
        let mut state = PomyuCharaState::default();
        state.cycle_times = [100; 8];
        state.judge = 0; // no judge

        let mock = MockTimerState::new();
        mock.set_on(TIMER_PM_CHARA_1P_NEUTRAL, 100);

        let mut on_timers: Vec<i32> = Vec::new();
        let mut off_timers: Vec<i32> = Vec::new();

        state.update_timer(
            &|id| mock.is_on(id),
            &|id| mock.elapsed(id),
            &mut |id| on_timers.push(id),
            &mut |id| off_timers.push(id),
            1,
            false,
        );

        // Should not transition from neutral (judge == 0)
        assert!(!on_timers.contains(&TIMER_PM_CHARA_1P_GREAT));
        assert!(!on_timers.contains(&TIMER_PM_CHARA_1P_BAD));
        assert!(!off_timers.contains(&TIMER_PM_CHARA_1P_NEUTRAL));
    }
}
