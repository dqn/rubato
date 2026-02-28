use beatoraja_core::timer_manager::TimerManager;

// SkinProperty timer constants for PM character animations
const TIMER_PM_CHARA_1P_NEUTRAL: i32 = 900;
const TIMER_PM_CHARA_1P_FEVER: i32 = 901;
const TIMER_PM_CHARA_1P_GREAT: i32 = 902;
const TIMER_PM_CHARA_1P_GOOD: i32 = 903;
const TIMER_PM_CHARA_1P_BAD: i32 = 904;
const TIMER_PM_CHARA_2P_NEUTRAL: i32 = 905;
const TIMER_PM_CHARA_2P_GREAT: i32 = 906;
const TIMER_PM_CHARA_2P_BAD: i32 = 907;
const TIMER_PM_CHARA_DANCE: i32 = 909;

/// PMS character animation timer processor
pub struct PomyuCharaProcessor {
    /// Motion cycle times: 0:1P_NEUTRAL 1:1P_FEVER 2:1P_GREAT 3:1P_GOOD 4:1P_BAD 5:2P_NEUTRAL 6:2P_GREAT 7:2P_BAD
    pm_chara_time: [i32; 8],
    /// Processed note count at neutral motion start {1P, 2P}
    pm_chara_lastnotes: [i32; 2],
    /// PMS character judge
    pub pm_chara_judge: i32,
}

impl Default for PomyuCharaProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl PomyuCharaProcessor {
    pub fn new() -> Self {
        PomyuCharaProcessor {
            pm_chara_time: [1, 1, 1, 1, 1, 1, 1, 1],
            pm_chara_lastnotes: [0, 0],
            pm_chara_judge: 0,
        }
    }

    pub fn init(&mut self) {
        self.pm_chara_lastnotes[0] = 0;
        self.pm_chara_lastnotes[1] = 0;
        self.pm_chara_judge = 0;
    }

    pub fn get_pm_chara_time(&self, index: i32) -> i32 {
        if index < 0 || index >= self.pm_chara_time.len() as i32 {
            return 1;
        }
        self.pm_chara_time[index as usize]
    }

    pub fn set_pm_chara_time(&mut self, index: i32, value: i32) {
        if index >= 0 && (index as usize) < self.pm_chara_time.len() && value >= 1 {
            self.pm_chara_time[index as usize] = value;
        }
    }

    /// Update PMS character animation timers.
    ///
    /// Translated from: Java PomyuCharaProcessor.updateTimer(BMSPlayer player)
    /// Uses past_notes and gauge_is_max from the player to determine character state.
    pub fn update_timer(&mut self, timer: &mut TimerManager, past_notes: i32, gauge_is_max: bool) {
        // 1P neutral check
        let neutral_time_1p = self.get_pm_chara_time(0);
        if timer.is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL)
            && timer.get_now_time_for_id(TIMER_PM_CHARA_1P_NEUTRAL) >= neutral_time_1p as i64
            && timer.get_now_time_for_id(TIMER_PM_CHARA_1P_NEUTRAL) % (neutral_time_1p as i64) < 17
            && self.pm_chara_lastnotes[0] != past_notes
            && self.pm_chara_judge > 0
        {
            if self.pm_chara_judge == 1 || self.pm_chara_judge == 2 {
                if gauge_is_max {
                    timer.set_timer_on(TIMER_PM_CHARA_1P_FEVER);
                } else {
                    timer.set_timer_on(TIMER_PM_CHARA_1P_GREAT);
                }
            } else if self.pm_chara_judge == 3 {
                timer.set_timer_on(TIMER_PM_CHARA_1P_GOOD);
            } else {
                timer.set_timer_on(TIMER_PM_CHARA_1P_BAD);
            }
            timer.set_timer_off(TIMER_PM_CHARA_1P_NEUTRAL);
        }

        // 2P neutral check
        let neutral_time_2p =
            self.get_pm_chara_time(TIMER_PM_CHARA_2P_NEUTRAL - TIMER_PM_CHARA_1P_NEUTRAL);
        if timer.is_timer_on(TIMER_PM_CHARA_2P_NEUTRAL)
            && timer.get_now_time_for_id(TIMER_PM_CHARA_2P_NEUTRAL) >= neutral_time_2p as i64
            && timer.get_now_time_for_id(TIMER_PM_CHARA_2P_NEUTRAL) % (neutral_time_2p as i64) < 17
            && self.pm_chara_lastnotes[1] != past_notes
            && self.pm_chara_judge > 0
        {
            if self.pm_chara_judge >= 1 && self.pm_chara_judge <= 3 {
                timer.set_timer_on(TIMER_PM_CHARA_2P_BAD);
            } else {
                timer.set_timer_on(TIMER_PM_CHARA_2P_GREAT);
            }
            timer.set_timer_off(TIMER_PM_CHARA_2P_NEUTRAL);
        }

        // Non-neutral timer cycling
        for i in TIMER_PM_CHARA_1P_FEVER..=TIMER_PM_CHARA_2P_BAD {
            if i != TIMER_PM_CHARA_2P_NEUTRAL
                && timer.is_timer_on(i)
                && timer.get_now_time_for_id(i)
                    >= self.get_pm_chara_time(i - TIMER_PM_CHARA_1P_NEUTRAL) as i64
            {
                if i <= TIMER_PM_CHARA_1P_BAD {
                    timer.set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);
                    self.pm_chara_lastnotes[0] = past_notes;
                } else {
                    timer.set_timer_on(TIMER_PM_CHARA_2P_NEUTRAL);
                    self.pm_chara_lastnotes[1] = past_notes;
                }
                timer.set_timer_off(i);
            }
        }

        timer.switch_timer(TIMER_PM_CHARA_DANCE, true);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_timer_enables_dance_timer() {
        let mut pomyu = PomyuCharaProcessor::new();
        let mut timer = TimerManager::new();
        timer.update();

        pomyu.update_timer(&mut timer, 0, false);

        assert!(timer.is_timer_on(TIMER_PM_CHARA_DANCE));
    }

    #[test]
    fn update_timer_with_judge_and_notes_change_enables_great() {
        let mut pomyu = PomyuCharaProcessor::new();
        pomyu.pm_chara_judge = 1; // PGREAT
        pomyu.pm_chara_lastnotes[0] = 0;

        let mut timer = TimerManager::new();
        timer.update();
        timer.set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);

        // We need time to pass so the neutral timer check works.
        // pm_chara_time[0] = 1 (very short), so we need getNowTime >= 1.
        std::thread::sleep(std::time::Duration::from_millis(2));
        timer.update();

        // past_notes changed from 0 to 5
        pomyu.update_timer(&mut timer, 5, false);

        // With judge=1 and not gauge_is_max, should enable GREAT
        assert!(timer.is_timer_on(TIMER_PM_CHARA_1P_GREAT));
        assert!(!timer.is_timer_on(TIMER_PM_CHARA_1P_NEUTRAL));
    }

    #[test]
    fn update_timer_fever_when_gauge_is_max() {
        let mut pomyu = PomyuCharaProcessor::new();
        pomyu.pm_chara_judge = 2; // GREAT
        pomyu.pm_chara_lastnotes[0] = 0;

        let mut timer = TimerManager::new();
        timer.update();
        timer.set_timer_on(TIMER_PM_CHARA_1P_NEUTRAL);

        std::thread::sleep(std::time::Duration::from_millis(2));
        timer.update();

        pomyu.update_timer(&mut timer, 3, true); // gauge_is_max = true

        assert!(timer.is_timer_on(TIMER_PM_CHARA_1P_FEVER));
    }

    #[test]
    fn init_resets_state() {
        let mut pomyu = PomyuCharaProcessor::new();
        pomyu.pm_chara_lastnotes = [10, 20];
        pomyu.pm_chara_judge = 3;

        pomyu.init();

        assert_eq!(pomyu.pm_chara_lastnotes, [0, 0]);
        assert_eq!(pomyu.pm_chara_judge, 0);
    }
}
