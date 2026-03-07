use bms_model::mode::Mode;
use rubato_play::bms_player_rule::BMSPlayerRule;

use std::sync::Mutex;

pub const JUDGE_OPTIONS: [&str; 4] = ["EASY", "NORMAL", "HARD", "VERY_HARD"];

static ACTIVE: Mutex<bool> = Mutex::new(false);
static JUDGE_RANK: Mutex<i32> = Mutex::new(0);

pub struct JudgeTrainer;

impl JudgeTrainer {
    pub fn is_active() -> bool {
        *ACTIVE.lock().expect("ACTIVE lock poisoned")
    }

    pub fn set_active(active: bool) {
        *ACTIVE.lock().expect("ACTIVE lock poisoned") = active;
    }

    pub fn get_judge_rank() -> i32 {
        *JUDGE_RANK.lock().expect("JUDGE_RANK lock poisoned")
    }

    pub fn set_judge_rank(judge_rank: i32) {
        *JUDGE_RANK.lock().expect("JUDGE_RANK lock poisoned") = judge_rank;
    }

    pub fn judge_window_rate(mode: &Mode) -> i32 {
        // NOTE: The order of the rule is from VERY-HARD to VERY-EASY:
        // VERY-HARD | HARD | NORMAL | EASY | VERY-EASY
        //     0     |  1   |   2    |  3   |     4
        // However, the order defined here is completely reversed and VERY-EASY is not an option
        // (LR2 doesn't support VERY-EASY and LR2oraja considers it as EASY directly).
        // Therefore, we need a transformation:
        // EASY 0 -> 3 | NORMAL: 1 -> 2 | HARD: 2 -> 1 | VERY-HARD: 3 -> 0
        // We can observe that the sum is always 3
        let judge_rank = *JUDGE_RANK.lock().expect("JUDGE_RANK lock poisoned");
        let rule = BMSPlayerRule::for_mode(mode);
        rule.judge.windowrule.judgerank[(3 - judge_rank) as usize]
    }
}
