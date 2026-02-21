/// Judge property configuration
#[derive(Clone, Debug)]
pub struct JudgeProperty {
    /// Normal note judge windows: PG, GR, GD, BD, MS order, {LATE lower, EARLY upper} pairs
    note: Vec<[i64; 2]>,
    /// Scratch note judge windows
    scratch: Vec<[i64; 2]>,
    /// Long note end judge windows
    longnote: Vec<[i64; 2]>,
    pub longnote_margin: i64,
    /// Long scratch end judge windows
    longscratch: Vec<[i64; 2]>,
    pub longscratch_margin: i64,
    /// Combo continuation per judge
    pub combo: Vec<bool>,
    /// Miss condition
    pub miss: MissCondition,
    /// Whether each judge causes note to vanish
    pub judge_vanish: Vec<bool>,
    pub windowrule: JudgeWindowRule,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MissCondition {
    One,
    Always,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteType {
    Note,
    LongnoteEnd,
    Scratch,
    LongscratchEnd,
}

#[derive(Clone, Debug)]
pub struct JudgeWindowRule {
    pub judgerank: Vec<i32>,
    pub fixjudge: Vec<bool>,
    pub rule_type: JudgeWindowRuleType,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JudgeWindowRuleType {
    Normal,
    Pms,
    Lr2,
}

static LR2_SCALING: [[i64; 5]; 4] = [
    [0, 0, 0, 0, 0],
    [0, 8000, 15000, 18000, 21000],
    [0, 24000, 30000, 40000, 60000],
    [0, 40000, 60000, 100000, 120000],
];

fn lr2_judge_scaling(mut base: i64, judgerank: i32) -> i64 {
    let mut sign: i64 = 1;
    if base < 0 {
        base = -base;
        sign = -1;
    }
    if judgerank >= 100 {
        return sign * base * judgerank as i64 / 100;
    }
    let last = LR2_SCALING[0].len() - 1;
    let judgeindex = judgerank as usize / 25;
    let mut s: usize = 0;
    while s < LR2_SCALING.len() && base >= LR2_SCALING[s][last] {
        s += 1;
    }
    let (x1, x2, d): (i64, i64, i64);
    if s < LR2_SCALING.len() {
        let n = base - LR2_SCALING[s - 1][last];
        d = LR2_SCALING[s][last] - LR2_SCALING[s - 1][last];
        x1 = d * LR2_SCALING[s - 1][judgeindex]
            + n * (LR2_SCALING[s][judgeindex] - LR2_SCALING[s - 1][judgeindex]);
        x2 = d * LR2_SCALING[s - 1][judgeindex + 1]
            + n * (LR2_SCALING[s][judgeindex + 1] - LR2_SCALING[s - 1][judgeindex + 1]);
    } else {
        let n = base;
        d = LR2_SCALING[s - 1][last];
        x1 = n * LR2_SCALING[s - 1][judgeindex];
        x2 = n * LR2_SCALING[s - 1][judgeindex + 1];
    }
    sign * (x1 + (judgerank as i64 - judgeindex as i64 * 25) * (x2 - x1) / 25) / d
}

fn create_lr2(org: &[[i64; 2]], judgerank: i32, judge_window_rate: &[i32]) -> Vec<[i64; 2]> {
    let mut judge: Vec<[i64; 2]> = org.to_vec();

    // Only change pgreat, great, good
    let fixmax = 3;
    for i in 0..fixmax {
        for j in 0..2 {
            judge[i][j] = lr2_judge_scaling(org[i][j], judgerank);
        }
    }

    // Correction if we exceed the bad windows
    for i in (0..fixmax).rev() {
        for j in 0..2 {
            if judge[i][j].abs() > judge[i + 1][j].abs() {
                judge[i][j] = judge[i + 1][j];
            }
        }
    }

    // judgeWindowRate correction
    let limit = std::cmp::min(org.len(), 3);
    for i in 0..limit {
        for j in 0..2 {
            judge[i][j] = judge[i][j] * judge_window_rate[i] as i64 / 100;
            if judge[i][j].abs() > judge[3][j].abs() {
                judge[i][j] = judge[3][j];
            }
            if i > 0 && judge[i][j].abs() < judge[i - 1][j].abs() {
                judge[i][j] = judge[i - 1][j];
            }
        }
    }

    judge
}

impl JudgeWindowRule {
    fn create_normal(
        &self,
        org: &[[i64; 2]],
        judgerank: i32,
        judge_window_rate: &[i32],
    ) -> Vec<[i64; 2]> {
        let mut judge: Vec<[i64; 2]> = vec![[0, 0]; org.len()];
        for i in 0..org.len() {
            for j in 0..2 {
                judge[i][j] = if self.fixjudge[i] {
                    org[i][j]
                } else {
                    org[i][j] * judgerank as i64 / 100
                };
            }
        }

        let mut fixmin: i32 = -1;
        let limit = std::cmp::min(org.len(), 4);
        for i in 0..limit {
            if self.fixjudge[i] {
                fixmin = i as i32;
                continue;
            }
            let mut fixmax: i32 = -1;
            for j2 in (i + 1)..4 {
                if self.fixjudge[j2] {
                    fixmax = j2 as i32;
                    break;
                }
            }

            for j in 0..2 {
                if fixmin != -1 && judge[i][j].abs() < judge[fixmin as usize][j].abs() {
                    judge[i][j] = judge[fixmin as usize][j];
                }
                if fixmax != -1 && judge[i][j].abs() > judge[fixmax as usize][j].abs() {
                    judge[i][j] = judge[fixmax as usize][j];
                }
            }
        }

        // judgeWindowRate correction
        let limit2 = std::cmp::min(org.len(), 3);
        for i in 0..limit2 {
            for j in 0..2 {
                judge[i][j] = judge[i][j] * judge_window_rate[i] as i64 / 100;
                if judge[i][j].abs() > judge[3][j].abs() {
                    judge[i][j] = judge[3][j];
                }
                if i > 0 && judge[i][j].abs() < judge[i - 1][j].abs() {
                    judge[i][j] = judge[i - 1][j];
                }
            }
        }

        judge
    }

    pub fn create(
        &self,
        org: &[[i64; 2]],
        judgerank: i32,
        judge_window_rate: &[i32],
    ) -> Vec<[i64; 2]> {
        match self.rule_type {
            JudgeWindowRuleType::Lr2 => create_lr2(org, judgerank, judge_window_rate),
            _ => self.create_normal(org, judgerank, judge_window_rate),
        }
    }
}

fn convert_milli(judge: &[[i64; 2]]) -> Vec<Vec<i32>> {
    let mut mjudge: Vec<Vec<i32>> = Vec::with_capacity(judge.len());
    for row in judge {
        let mut mrow = Vec::with_capacity(row.len());
        for &val in row {
            mrow.push((val / 1000) as i32);
        }
        mjudge.push(mrow);
    }
    mjudge
}

impl JudgeProperty {
    pub fn get_note_judge(&self, judgerank: i32, judge_window_rate: &[i32]) -> Vec<Vec<i32>> {
        convert_milli(
            &self
                .windowrule
                .create(&self.note, judgerank, judge_window_rate),
        )
    }

    pub fn get_long_note_end_judge(
        &self,
        judgerank: i32,
        judge_window_rate: &[i32],
    ) -> Vec<Vec<i32>> {
        convert_milli(
            &self
                .windowrule
                .create(&self.longnote, judgerank, judge_window_rate),
        )
    }

    pub fn get_scratch_judge(&self, judgerank: i32, judge_window_rate: &[i32]) -> Vec<Vec<i32>> {
        convert_milli(
            &self
                .windowrule
                .create(&self.scratch, judgerank, judge_window_rate),
        )
    }

    pub fn get_long_scratch_end_judge(
        &self,
        judgerank: i32,
        judge_window_rate: &[i32],
    ) -> Vec<Vec<i32>> {
        convert_milli(
            &self
                .windowrule
                .create(&self.longscratch, judgerank, judge_window_rate),
        )
    }

    pub fn get_judge(
        &self,
        notetype: NoteType,
        judgerank: i32,
        judge_window_rate: &[i32],
    ) -> Vec<[i64; 2]> {
        match notetype {
            NoteType::Note => self
                .windowrule
                .create(&self.note, judgerank, judge_window_rate),
            NoteType::LongnoteEnd => {
                self.windowrule
                    .create(&self.longnote, judgerank, judge_window_rate)
            }
            NoteType::Scratch => {
                self.windowrule
                    .create(&self.scratch, judgerank, judge_window_rate)
            }
            NoteType::LongscratchEnd => {
                self.windowrule
                    .create(&self.longscratch, judgerank, judge_window_rate)
            }
        }
    }
}

// Pre-defined JudgeWindowRules
fn rule_normal() -> JudgeWindowRule {
    JudgeWindowRule {
        judgerank: vec![25, 50, 75, 100, 125],
        fixjudge: vec![false, false, false, false, true],
        rule_type: JudgeWindowRuleType::Normal,
    }
}

fn rule_pms() -> JudgeWindowRule {
    JudgeWindowRule {
        judgerank: vec![33, 50, 70, 100, 133],
        fixjudge: vec![true, false, false, true, true],
        rule_type: JudgeWindowRuleType::Pms,
    }
}

fn rule_lr2() -> JudgeWindowRule {
    JudgeWindowRule {
        judgerank: vec![25, 50, 75, 100, 75],
        fixjudge: vec![false, false, false, true, true],
        rule_type: JudgeWindowRuleType::Lr2,
    }
}

// Pre-defined JudgeProperty variants
pub fn fivekeys() -> JudgeProperty {
    JudgeProperty {
        note: vec![
            [-20000, 20000],
            [-50000, 50000],
            [-100000, 100000],
            [-150000, 150000],
            [-150000, 500000],
        ],
        scratch: vec![
            [-30000, 30000],
            [-60000, 60000],
            [-110000, 110000],
            [-160000, 160000],
            [-160000, 500000],
        ],
        longnote: vec![
            [-120000, 120000],
            [-150000, 150000],
            [-200000, 200000],
            [-250000, 250000],
        ],
        longnote_margin: 0,
        longscratch: vec![
            [-130000, 130000],
            [-160000, 160000],
            [-110000, 110000],
            [-260000, 260000],
        ],
        longscratch_margin: 0,
        combo: vec![true, true, true, false, false, false],
        miss: MissCondition::Always,
        judge_vanish: vec![true, true, true, true, true, false],
        windowrule: rule_normal(),
    }
}

pub fn sevenkeys() -> JudgeProperty {
    JudgeProperty {
        note: vec![
            [-20000, 20000],
            [-60000, 60000],
            [-150000, 150000],
            [-280000, 220000],
            [-150000, 500000],
        ],
        scratch: vec![
            [-30000, 30000],
            [-70000, 70000],
            [-160000, 160000],
            [-290000, 230000],
            [-160000, 500000],
        ],
        longnote: vec![
            [-120000, 120000],
            [-160000, 160000],
            [-200000, 200000],
            [-280000, 220000],
        ],
        longnote_margin: 0,
        longscratch: vec![
            [-130000, 130000],
            [-170000, 170000],
            [-210000, 210000],
            [-290000, 230000],
        ],
        longscratch_margin: 0,
        combo: vec![true, true, true, false, false, true],
        miss: MissCondition::Always,
        judge_vanish: vec![true, true, true, true, true, false],
        windowrule: rule_normal(),
    }
}

pub fn pms() -> JudgeProperty {
    JudgeProperty {
        note: vec![
            [-20000, 20000],
            [-50000, 50000],
            [-117000, 117000],
            [-183000, 183000],
            [-175000, 500000],
        ],
        scratch: vec![],
        longnote: vec![
            [-120000, 120000],
            [-150000, 150000],
            [-217000, 217000],
            [-283000, 283000],
        ],
        longnote_margin: 200000,
        longscratch: vec![],
        longscratch_margin: 0,
        combo: vec![true, true, true, false, false, false],
        miss: MissCondition::One,
        judge_vanish: vec![true, true, true, false, true, false],
        windowrule: rule_pms(),
    }
}

pub fn keyboard() -> JudgeProperty {
    JudgeProperty {
        note: vec![
            [-30000, 30000],
            [-90000, 90000],
            [-200000, 200000],
            [-320000, 240000],
            [-200000, 650000],
        ],
        scratch: vec![],
        longnote: vec![
            [-160000, 25000],
            [-200000, 75000],
            [-260000, 140000],
            [-320000, 240000],
        ],
        longnote_margin: 0,
        longscratch: vec![],
        longscratch_margin: 0,
        combo: vec![true, true, true, false, false, true],
        miss: MissCondition::Always,
        judge_vanish: vec![true, true, true, true, true, false],
        windowrule: rule_normal(),
    }
}

pub fn lr2() -> JudgeProperty {
    JudgeProperty {
        note: vec![
            [-21000, 21000],
            [-60000, 60000],
            [-120000, 120000],
            [-200000, 200000],
            [0, 1000000],
        ],
        scratch: vec![
            [-21000, 21000],
            [-60000, 60000],
            [-120000, 120000],
            [-200000, 200000],
            [0, 1000000],
        ],
        longnote: vec![
            [-120000, 120000],
            [-120000, 120000],
            [-120000, 120000],
            [-200000, 200000],
        ],
        longnote_margin: 0,
        longscratch: vec![
            [-120000, 120000],
            [-120000, 120000],
            [-120000, 120000],
            [-200000, 200000],
        ],
        longscratch_margin: 0,
        combo: vec![true, true, true, false, false, true],
        miss: MissCondition::Always,
        judge_vanish: vec![true, true, true, true, true, false],
        windowrule: rule_lr2(),
    }
}

/// Enum-like accessor for JudgeProperty variants
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JudgePropertyType {
    FiveKeys,
    SevenKeys,
    Pms,
    Keyboard,
    Lr2,
}

impl JudgePropertyType {
    pub fn get(&self) -> JudgeProperty {
        match self {
            JudgePropertyType::FiveKeys => fivekeys(),
            JudgePropertyType::SevenKeys => sevenkeys(),
            JudgePropertyType::Pms => pms(),
            JudgePropertyType::Keyboard => keyboard(),
            JudgePropertyType::Lr2 => lr2(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- JudgePropertyType tests ---

    #[test]
    fn judge_property_type_get_returns_correct_variant() {
        // Just verify each type returns a JudgeProperty without panicking
        let _ = JudgePropertyType::FiveKeys.get();
        let _ = JudgePropertyType::SevenKeys.get();
        let _ = JudgePropertyType::Pms.get();
        let _ = JudgePropertyType::Keyboard.get();
        let _ = JudgePropertyType::Lr2.get();
    }

    // --- JudgeWindowRule pre-defined rule tests ---

    #[test]
    fn rule_normal_has_correct_judgerank() {
        let rule = rule_normal();
        assert_eq!(rule.judgerank, vec![25, 50, 75, 100, 125]);
        assert_eq!(rule.fixjudge, vec![false, false, false, false, true]);
        assert_eq!(rule.rule_type, JudgeWindowRuleType::Normal);
    }

    #[test]
    fn rule_pms_has_correct_judgerank() {
        let rule = rule_pms();
        assert_eq!(rule.judgerank, vec![33, 50, 70, 100, 133]);
        assert_eq!(rule.fixjudge, vec![true, false, false, true, true]);
        assert_eq!(rule.rule_type, JudgeWindowRuleType::Pms);
    }

    #[test]
    fn rule_lr2_has_correct_judgerank() {
        let rule = rule_lr2();
        assert_eq!(rule.judgerank, vec![25, 50, 75, 100, 75]);
        assert_eq!(rule.fixjudge, vec![false, false, false, true, true]);
        assert_eq!(rule.rule_type, JudgeWindowRuleType::Lr2);
    }

    // --- Seven keys note judge window tests ---

    #[test]
    fn sevenkeys_note_windows() {
        let jp = sevenkeys();
        assert_eq!(jp.note.len(), 5);
        // PGREAT window
        assert_eq!(jp.note[0], [-20000, 20000]);
        // GREAT window
        assert_eq!(jp.note[1], [-60000, 60000]);
        // GOOD window
        assert_eq!(jp.note[2], [-150000, 150000]);
        // BAD window
        assert_eq!(jp.note[3], [-280000, 220000]);
        // POOR window (miss)
        assert_eq!(jp.note[4], [-150000, 500000]);
    }

    #[test]
    fn sevenkeys_scratch_windows() {
        let jp = sevenkeys();
        assert_eq!(jp.scratch.len(), 5);
        // Scratch has wider windows than keys
        assert_eq!(jp.scratch[0], [-30000, 30000]);
        assert_eq!(jp.scratch[1], [-70000, 70000]);
    }

    #[test]
    fn sevenkeys_combo_conditions() {
        let jp = sevenkeys();
        // PG, GR, GD continue combo; BD does not; PR does not; MS does
        assert_eq!(jp.combo, vec![true, true, true, false, false, true]);
    }

    // --- Five keys note judge window tests ---

    #[test]
    fn fivekeys_note_windows() {
        let jp = fivekeys();
        assert_eq!(jp.note.len(), 5);
        assert_eq!(jp.note[0], [-20000, 20000]);
        assert_eq!(jp.note[1], [-50000, 50000]);
        assert_eq!(jp.note[2], [-100000, 100000]);
        assert_eq!(jp.note[3], [-150000, 150000]);
        assert_eq!(jp.note[4], [-150000, 500000]);
    }

    #[test]
    fn fivekeys_combo_conditions() {
        let jp = fivekeys();
        // PG, GR, GD continue combo; BD, PR, MS do not
        assert_eq!(jp.combo, vec![true, true, true, false, false, false]);
    }

    // --- LR2 judge property tests ---

    #[test]
    fn lr2_note_windows() {
        let jp = lr2();
        assert_eq!(jp.note.len(), 5);
        assert_eq!(jp.note[0], [-21000, 21000]);
        assert_eq!(jp.note[1], [-60000, 60000]);
        assert_eq!(jp.note[2], [-120000, 120000]);
        assert_eq!(jp.note[3], [-200000, 200000]);
        assert_eq!(jp.note[4], [0, 1000000]);
    }

    #[test]
    fn lr2_scratch_same_as_note() {
        let jp = lr2();
        // In LR2, scratch windows are the same as note windows
        assert_eq!(jp.note, jp.scratch);
    }

    #[test]
    fn lr2_longnote_windows() {
        let jp = lr2();
        assert_eq!(jp.longnote.len(), 4);
        assert_eq!(jp.longnote[0], [-120000, 120000]);
        assert_eq!(jp.longnote[3], [-200000, 200000]);
    }

    #[test]
    fn lr2_uses_lr2_window_rule() {
        let jp = lr2();
        assert_eq!(jp.windowrule.rule_type, JudgeWindowRuleType::Lr2);
    }

    // --- PMS judge property tests ---

    #[test]
    fn pms_has_no_scratch() {
        let jp = pms();
        assert!(jp.scratch.is_empty());
        assert!(jp.longscratch.is_empty());
    }

    #[test]
    fn pms_has_longnote_margin() {
        let jp = pms();
        assert_eq!(jp.longnote_margin, 200000);
    }

    #[test]
    fn pms_miss_condition_is_one() {
        let jp = pms();
        assert_eq!(jp.miss, MissCondition::One);
    }

    // --- Keyboard judge property tests ---

    #[test]
    fn keyboard_has_no_scratch() {
        let jp = keyboard();
        assert!(jp.scratch.is_empty());
        assert!(jp.longscratch.is_empty());
    }

    #[test]
    fn keyboard_has_wider_note_windows() {
        let jp = keyboard();
        // Keyboard PGREAT window is wider than 7keys
        assert_eq!(jp.note[0], [-30000, 30000]);
        assert_eq!(jp.note[1], [-90000, 90000]);
    }

    // --- LR2 judge scaling tests ---

    #[test]
    fn lr2_judge_scaling_at_rank_100_returns_base() {
        // When judgerank >= 100, returns base * judgerank / 100
        assert_eq!(lr2_judge_scaling(21000, 100), 21000);
        assert_eq!(lr2_judge_scaling(60000, 100), 60000);
    }

    #[test]
    fn lr2_judge_scaling_at_rank_200_doubles() {
        assert_eq!(lr2_judge_scaling(21000, 200), 42000);
    }

    #[test]
    fn lr2_judge_scaling_negative_base() {
        // Negative base should produce negative result
        let result = lr2_judge_scaling(-21000, 100);
        assert_eq!(result, -21000);
    }

    #[test]
    fn lr2_judge_scaling_zero_base() {
        assert_eq!(lr2_judge_scaling(0, 50), 0);
    }

    #[test]
    fn lr2_judge_scaling_rank_50() {
        // Rank 50 should be roughly half the window
        let result = lr2_judge_scaling(21000, 50);
        assert!(result < 21000, "rank 50 should narrow the window");
        assert!(result > 0, "rank 50 should still be positive");
    }

    #[test]
    fn lr2_judge_scaling_rank_75() {
        let result = lr2_judge_scaling(21000, 75);
        assert!(result < 21000, "rank 75 should narrow the window");
        assert!(
            result > lr2_judge_scaling(21000, 50),
            "rank 75 wider than rank 50"
        );
    }

    // --- JudgeWindowRule create tests ---

    #[test]
    fn normal_rule_create_at_rank_100_preserves_windows() {
        let rule = rule_normal();
        let org = &[
            [-20000i64, 20000],
            [-60000, 60000],
            [-150000, 150000],
            [-280000, 220000],
            [-150000, 500000],
        ];
        let rate = [100, 100, 100];
        let result = rule.create(org, 100, &rate);
        // At rank 100, windows should be 100% of original
        assert_eq!(result[0], [-20000, 20000]);
        assert_eq!(result[1], [-60000, 60000]);
        assert_eq!(result[2], [-150000, 150000]);
    }

    #[test]
    fn normal_rule_create_at_rank_50_narrows_windows() {
        let rule = rule_normal();
        let org = &[
            [-20000i64, 20000],
            [-60000, 60000],
            [-150000, 150000],
            [-280000, 220000],
            [-150000, 500000],
        ];
        let rate = [100, 100, 100];
        let result = rule.create(org, 50, &rate);
        // At rank 50, non-fixed windows should be ~50% of original
        assert_eq!(result[0], [-10000, 10000]);
        assert_eq!(result[1], [-30000, 30000]);
    }

    #[test]
    fn lr2_rule_create_dispatches_to_lr2() {
        let rule = rule_lr2();
        let org = &[
            [-21000i64, 21000],
            [-60000, 60000],
            [-120000, 120000],
            [-200000, 200000],
            [0, 1000000],
        ];
        let rate = [100, 100, 100];
        let result = rule.create(org, 100, &rate);
        // At rank 100 in LR2, windows should be unchanged
        assert_eq!(result[0], [-21000, 21000]);
    }

    // --- get_judge and convert_milli tests ---

    #[test]
    fn get_note_judge_converts_to_milliseconds() {
        let jp = sevenkeys();
        let rate = [100, 100, 100];
        let result = jp.get_note_judge(100, &rate);
        // Original PGREAT: [-20000, 20000] micros => [-20, 20] millis
        assert_eq!(result[0], vec![-20, 20]);
        // GREAT: [-60000, 60000] => [-60, 60]
        assert_eq!(result[1], vec![-60, 60]);
    }

    #[test]
    fn get_judge_returns_correct_note_type() {
        let jp = sevenkeys();
        let rate = [100, 100, 100];
        let note_judge = jp.get_judge(NoteType::Note, 100, &rate);
        let scratch_judge = jp.get_judge(NoteType::Scratch, 100, &rate);
        // Note and Scratch should differ for 7keys
        assert_ne!(note_judge[0], scratch_judge[0]);
    }

    #[test]
    fn get_scratch_judge_converts_to_milliseconds() {
        let jp = sevenkeys();
        let rate = [100, 100, 100];
        let result = jp.get_scratch_judge(100, &rate);
        // Scratch PGREAT: [-30000, 30000] => [-30, 30]
        assert_eq!(result[0], vec![-30, 30]);
    }

    #[test]
    fn judge_vanish_flags() {
        let jp = sevenkeys();
        // PG through PR cause vanish, MS does not
        assert_eq!(jp.judge_vanish, vec![true, true, true, true, true, false]);
    }

    #[test]
    fn miss_condition_always_for_sevenkeys() {
        let jp = sevenkeys();
        assert_eq!(jp.miss, MissCondition::Always);
    }

    // --- Judge window rate tests ---

    #[test]
    fn judge_window_rate_scales_windows() {
        let jp = sevenkeys();
        // 50% rate for PG, 100% for GR, 100% for GD
        let rate = [50, 100, 100];
        let result = jp.get_judge(NoteType::Note, 100, &rate);
        // PG window should be halved: [-20000, 20000] * 50% = [-10000, 10000]
        assert_eq!(result[0], [-10000, 10000]);
        // GR window should be unchanged: [-60000, 60000]
        assert_eq!(result[1], [-60000, 60000]);
    }

    // --- create_lr2 function tests ---

    #[test]
    fn create_lr2_at_rank_100_preserves_windows() {
        let org = &[
            [-21000i64, 21000],
            [-60000, 60000],
            [-120000, 120000],
            [-200000, 200000],
            [0, 1000000],
        ];
        let rate = [100, 100, 100];
        let result = create_lr2(org, 100, &rate);
        // At rank 100, LR2 scaling: base * 100 / 100 = base
        assert_eq!(result[0], [-21000, 21000]);
        assert_eq!(result[1], [-60000, 60000]);
        assert_eq!(result[2], [-120000, 120000]);
        // BAD and POOR windows are never scaled
        assert_eq!(result[3], [-200000, 200000]);
        assert_eq!(result[4], [0, 1000000]);
    }

    #[test]
    fn create_lr2_at_rank_50_narrows_pg_gr_gd() {
        let org = &[
            [-21000i64, 21000],
            [-60000, 60000],
            [-120000, 120000],
            [-200000, 200000],
            [0, 1000000],
        ];
        let rate = [100, 100, 100];
        let result = create_lr2(org, 50, &rate);
        // Windows should be narrower at rank 50
        assert!(result[0][1] < 21000);
        assert!(result[1][1] < 60000);
        // BAD and POOR should be unchanged
        assert_eq!(result[3], [-200000, 200000]);
        assert_eq!(result[4], [0, 1000000]);
    }
}
