use bms_model::note::Note;

/// Judge algorithm
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JudgeAlgorithm {
    /// Combo priority
    Combo,
    /// Duration priority
    Duration,
    /// Lowest note priority
    Lowest,
    /// Score priority
    Score,
}

pub static DEFAULT_ALGORITHM: &[JudgeAlgorithm] = &[
    JudgeAlgorithm::Combo,
    JudgeAlgorithm::Duration,
    JudgeAlgorithm::Lowest,
];

impl JudgeAlgorithm {
    /// Compare two notes. Returns true if t2 is preferred over t1.
    pub fn compare(&self, t1: &Note, t2: &Note, ptime: i64, judgetable: &[Vec<i64>]) -> bool {
        match self {
            JudgeAlgorithm::Combo => {
                t2.state() == 0
                    && t1.micro_time() < ptime + judgetable[2][0]
                    && t2.micro_time() <= ptime + judgetable[2][1]
            }
            JudgeAlgorithm::Duration => {
                (t1.micro_time() - ptime).abs() > (t2.micro_time() - ptime).abs() && t2.state() == 0
            }
            JudgeAlgorithm::Lowest => false,
            JudgeAlgorithm::Score => {
                t2.state() == 0
                    && t1.micro_time() < ptime + judgetable[1][0]
                    && t2.micro_time() <= ptime + judgetable[1][1]
            }
        }
    }

    /// Compare two notes using raw time/state values and `[i64; 2]` judge table.
    /// Used by `JudgeManager::update()` where only `JudgeNote` (no mutable `Note`) is available.
    /// Returns true if t2 is preferred over t1.
    pub fn compare_times(
        &self,
        t1_time: i64,
        t2_time: i64,
        t2_state: i32,
        ptime: i64,
        judgetable: &[[i64; 2]],
    ) -> bool {
        match self {
            JudgeAlgorithm::Combo => {
                t2_state == 0
                    && t1_time < ptime + judgetable[2][0]
                    && t2_time <= ptime + judgetable[2][1]
            }
            JudgeAlgorithm::Duration => {
                (t1_time - ptime).abs() > (t2_time - ptime).abs() && t2_state == 0
            }
            JudgeAlgorithm::Lowest => false,
            JudgeAlgorithm::Score => {
                t2_state == 0
                    && t1_time < ptime + judgetable[1][0]
                    && t2_time <= ptime + judgetable[1][1]
            }
        }
    }

    pub fn values() -> &'static [JudgeAlgorithm] {
        &[
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Score,
        ]
    }

    pub fn name(&self) -> &str {
        match self {
            JudgeAlgorithm::Combo => "Combo",
            JudgeAlgorithm::Duration => "Duration",
            JudgeAlgorithm::Lowest => "Lowest",
            JudgeAlgorithm::Score => "Score",
        }
    }

    pub fn index(algorithm: &str) -> i32 {
        for (i, v) in Self::values().iter().enumerate() {
            if v.name() == algorithm {
                return i as i32;
            }
        }
        -1
    }

    pub fn from_name(name: &str) -> Option<JudgeAlgorithm> {
        for v in Self::values() {
            if v.name() == name {
                return Some(*v);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_returns_all_four_variants() {
        let vals = JudgeAlgorithm::values();
        assert_eq!(vals.len(), 4);
        assert_eq!(vals[0], JudgeAlgorithm::Combo);
        assert_eq!(vals[1], JudgeAlgorithm::Duration);
        assert_eq!(vals[2], JudgeAlgorithm::Lowest);
        assert_eq!(vals[3], JudgeAlgorithm::Score);
    }

    #[test]
    fn name_returns_correct_strings() {
        assert_eq!(JudgeAlgorithm::Combo.name(), "Combo");
        assert_eq!(JudgeAlgorithm::Duration.name(), "Duration");
        assert_eq!(JudgeAlgorithm::Lowest.name(), "Lowest");
        assert_eq!(JudgeAlgorithm::Score.name(), "Score");
    }

    #[test]
    fn get_index_returns_correct_index() {
        assert_eq!(JudgeAlgorithm::index("Combo"), 0);
        assert_eq!(JudgeAlgorithm::index("Duration"), 1);
        assert_eq!(JudgeAlgorithm::index("Lowest"), 2);
        assert_eq!(JudgeAlgorithm::index("Score"), 3);
    }

    #[test]
    fn get_index_returns_negative_one_for_unknown() {
        assert_eq!(JudgeAlgorithm::index("Unknown"), -1);
        assert_eq!(JudgeAlgorithm::index(""), -1);
    }

    #[test]
    fn from_name_returns_correct_variant() {
        assert_eq!(
            JudgeAlgorithm::from_name("Combo"),
            Some(JudgeAlgorithm::Combo)
        );
        assert_eq!(
            JudgeAlgorithm::from_name("Duration"),
            Some(JudgeAlgorithm::Duration)
        );
        assert_eq!(
            JudgeAlgorithm::from_name("Lowest"),
            Some(JudgeAlgorithm::Lowest)
        );
        assert_eq!(
            JudgeAlgorithm::from_name("Score"),
            Some(JudgeAlgorithm::Score)
        );
    }

    #[test]
    fn from_name_returns_none_for_unknown() {
        assert_eq!(JudgeAlgorithm::from_name("Unknown"), None);
        assert_eq!(JudgeAlgorithm::from_name(""), None);
        assert_eq!(JudgeAlgorithm::from_name("combo"), None); // case-sensitive
    }

    #[test]
    fn default_algorithm_has_three_entries() {
        assert_eq!(DEFAULT_ALGORITHM.len(), 3);
        assert_eq!(DEFAULT_ALGORITHM[0], JudgeAlgorithm::Combo);
        assert_eq!(DEFAULT_ALGORITHM[1], JudgeAlgorithm::Duration);
        assert_eq!(DEFAULT_ALGORITHM[2], JudgeAlgorithm::Lowest);
    }

    #[test]
    fn lowest_compare_always_returns_false() {
        let t1 = Note::new_normal(1);
        let t2 = Note::new_normal(2);
        let judgetable = vec![vec![0i64; 2]; 5];
        assert!(!JudgeAlgorithm::Lowest.compare(&t1, &t2, 0, &judgetable));
    }

    #[test]
    fn duration_compare_prefers_closer_note() {
        // t1 is at time 100000, t2 is at time 50000, ptime is 60000
        // |t1 - ptime| = 40000, |t2 - ptime| = 10000
        // t2 is closer => should return true (if t2 state == 0)
        let mut t1 = Note::new_normal(1);
        t1.set_micro_time(100000);
        let mut t2 = Note::new_normal(2);
        t2.set_micro_time(50000);
        let judgetable = vec![vec![0i64; 2]; 5];
        assert!(JudgeAlgorithm::Duration.compare(&t1, &t2, 60000, &judgetable));
    }

    #[test]
    fn duration_compare_rejects_when_t2_state_nonzero() {
        let mut t1 = Note::new_normal(1);
        t1.set_micro_time(100000);
        let mut t2 = Note::new_normal(2);
        t2.set_micro_time(50000);
        t2.set_state(1); // already judged
        let judgetable = vec![vec![0i64; 2]; 5];
        assert!(!JudgeAlgorithm::Duration.compare(&t1, &t2, 60000, &judgetable));
    }

    #[test]
    fn duration_compare_rejects_when_t1_is_closer() {
        // t1 is at 50000, t2 at 100000, ptime is 60000
        // |t1 - ptime| = 10000, |t2 - ptime| = 40000
        // t1 is closer => should return false
        let mut t1 = Note::new_normal(1);
        t1.set_micro_time(50000);
        let mut t2 = Note::new_normal(2);
        t2.set_micro_time(100000);
        let judgetable = vec![vec![0i64; 2]; 5];
        assert!(!JudgeAlgorithm::Duration.compare(&t1, &t2, 60000, &judgetable));
    }

    #[test]
    fn combo_compare_basic() {
        // judgetable[2] = [late_lower, early_upper] for GOOD window
        let mut t1 = Note::new_normal(1);
        t1.set_micro_time(50000);
        let mut t2 = Note::new_normal(2);
        t2.set_micro_time(60000);
        // ptime = 55000
        // t1.time < ptime + judgetable[2][0] => 50000 < 55000 + (-100000) = -45000 => false
        // So with typical negative late values, combo condition depends on window bounds
        let judgetable = vec![
            vec![-20000i64, 20000],
            vec![-60000, 60000],
            vec![-150000, 150000],
            vec![-280000, 220000],
            vec![-150000, 500000],
        ];
        // t1.time(50000) < ptime(55000) + judgetable[2][0](-150000) = -95000 => false
        assert!(!JudgeAlgorithm::Combo.compare(&t1, &t2, 55000, &judgetable));
    }

    #[test]
    fn score_compare_uses_judgetable_index_1() {
        let mut t1 = Note::new_normal(1);
        t1.set_micro_time(50000);
        let mut t2 = Note::new_normal(2);
        t2.set_micro_time(60000);
        // judgetable[1] = GREAT window
        let judgetable = vec![
            vec![-20000i64, 20000],
            vec![-60000, 60000],
            vec![-150000, 150000],
            vec![-280000, 220000],
            vec![-150000, 500000],
        ];
        // t1.time(50000) < ptime(55000) + judgetable[1][0](-60000) = -5000 => false
        assert!(!JudgeAlgorithm::Score.compare(&t1, &t2, 55000, &judgetable));
    }

    // ---------------------------------------------------------------
    // compare_times() tests
    // ---------------------------------------------------------------

    /// Typical judge table (microseconds):
    ///   [0] PGREAT: [-20_000,  20_000]
    ///   [1] GREAT:  [-60_000,  60_000]
    ///   [2] GOOD:   [-150_000, 150_000]
    ///   [3] BAD:    [-280_000, 220_000]
    ///   [4] MISS:   [-150_000, 500_000]
    fn make_judgetable() -> Vec<[i64; 2]> {
        vec![
            [-20_000, 20_000],   // PGREAT
            [-60_000, 60_000],   // GREAT
            [-150_000, 150_000], // GOOD
            [-280_000, 220_000], // BAD
            [-150_000, 500_000], // MISS
        ]
    }

    // --- Lowest: always false ---

    #[test]
    fn compare_times_lowest_always_false() {
        let jt = make_judgetable();
        // Even with t2 unjudged and closer, Lowest always returns false.
        assert!(!JudgeAlgorithm::Lowest.compare_times(100_000, 50_000, 0, 60_000, &jt));
        assert!(!JudgeAlgorithm::Lowest.compare_times(50_000, 100_000, 0, 60_000, &jt));
        assert!(!JudgeAlgorithm::Lowest.compare_times(0, 0, 0, 0, &jt));
    }

    // --- Duration ---

    #[test]
    fn compare_times_duration_prefers_closer_note() {
        let jt = make_judgetable();
        // t1 at 100_000, t2 at 50_000, ptime 60_000
        // |t1 - ptime| = 40_000, |t2 - ptime| = 10_000 => t2 closer => true
        assert!(JudgeAlgorithm::Duration.compare_times(100_000, 50_000, 0, 60_000, &jt));
    }

    #[test]
    fn compare_times_duration_rejects_farther_note() {
        let jt = make_judgetable();
        // |t1 - ptime| = 10_000, |t2 - ptime| = 40_000 => t1 closer => false
        assert!(!JudgeAlgorithm::Duration.compare_times(50_000, 100_000, 0, 60_000, &jt));
    }

    #[test]
    fn compare_times_duration_equal_distance_returns_false() {
        let jt = make_judgetable();
        // Both same distance => abs comparison is not strictly greater => false
        assert!(!JudgeAlgorithm::Duration.compare_times(70_000, 50_000, 0, 60_000, &jt));
    }

    #[test]
    fn compare_times_duration_rejects_nonzero_t2_state() {
        let jt = make_judgetable();
        // t2 is closer but state != 0 (already judged)
        assert!(!JudgeAlgorithm::Duration.compare_times(100_000, 50_000, 1, 60_000, &jt));
        assert!(!JudgeAlgorithm::Duration.compare_times(100_000, 50_000, -1, 60_000, &jt));
        assert!(!JudgeAlgorithm::Duration.compare_times(100_000, 50_000, 42, 60_000, &jt));
    }

    #[test]
    fn compare_times_duration_negative_times() {
        let jt = make_judgetable();
        // t1 at -100_000, t2 at -50_000, ptime = -60_000
        // |t1 - ptime| = 40_000, |t2 - ptime| = 10_000 => t2 closer => true
        assert!(JudgeAlgorithm::Duration.compare_times(-100_000, -50_000, 0, -60_000, &jt));
    }

    #[test]
    fn compare_times_duration_t2_exactly_at_ptime() {
        let jt = make_judgetable();
        // t2 is exactly at ptime => distance 0, t1 is elsewhere => true
        assert!(JudgeAlgorithm::Duration.compare_times(100_000, 60_000, 0, 60_000, &jt));
    }

    #[test]
    fn compare_times_duration_both_at_ptime() {
        let jt = make_judgetable();
        // Both exactly at ptime => equal distance (0 == 0), not strictly greater => false
        assert!(!JudgeAlgorithm::Duration.compare_times(60_000, 60_000, 0, 60_000, &jt));
    }

    // --- Combo (uses judgetable[2] = GOOD window) ---

    #[test]
    fn compare_times_combo_t2_preferred_within_good_window() {
        let jt = make_judgetable();
        // ptime = 0
        // Condition: t2_state == 0
        //   && t1_time < ptime + judgetable[2][0] (= 0 + (-150_000) = -150_000)
        //   && t2_time <= ptime + judgetable[2][1] (= 0 + 150_000 = 150_000)
        // t1 at -200_000 (past GOOD late boundary): -200_000 < -150_000 => true
        // t2 at 100_000 (within GOOD early boundary): 100_000 <= 150_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(-200_000, 100_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_t1_at_exact_good_late_boundary() {
        let jt = make_judgetable();
        // ptime = 0, judgetable[2][0] = -150_000
        // t1_time == ptime + judgetable[2][0] = -150_000
        // Condition: t1_time < -150_000 => -150_000 < -150_000 => false (not strictly less)
        assert!(!JudgeAlgorithm::Combo.compare_times(-150_000, 100_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_t1_just_past_good_late_boundary() {
        let jt = make_judgetable();
        // t1 = -150_001, boundary = -150_000 => -150_001 < -150_000 => true
        // t2 = 0 (within early boundary) => 0 <= 150_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(-150_001, 0, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_t2_at_exact_good_early_boundary() {
        let jt = make_judgetable();
        // ptime = 0, judgetable[2][1] = 150_000
        // t2_time == 150_000 <= 150_000 => true (less-or-equal)
        // t1_time = -200_000 < -150_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(-200_000, 150_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_t2_just_past_good_early_boundary() {
        let jt = make_judgetable();
        // t2_time = 150_001 <= 150_000 => false
        assert!(!JudgeAlgorithm::Combo.compare_times(-200_000, 150_001, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_t1_within_good_window_rejects() {
        let jt = make_judgetable();
        // t1 = -100_000, boundary = -150_000 => -100_000 < -150_000 => false
        // t1 is NOT past the GOOD late boundary, so combo does not prefer t2
        assert!(!JudgeAlgorithm::Combo.compare_times(-100_000, 0, 0, 0, &jt));
    }

    #[test]
    fn compare_times_combo_rejects_nonzero_t2_state() {
        let jt = make_judgetable();
        // All timing conditions met, but t2_state != 0
        assert!(!JudgeAlgorithm::Combo.compare_times(-200_000, 0, 1, 0, &jt));
        assert!(!JudgeAlgorithm::Combo.compare_times(-200_000, 0, -1, 0, &jt));
    }

    #[test]
    fn compare_times_combo_with_nonzero_ptime() {
        let jt = make_judgetable();
        // ptime = 500_000
        // GOOD late boundary: 500_000 + (-150_000) = 350_000
        // GOOD early boundary: 500_000 + 150_000 = 650_000
        // t1 = 300_000 < 350_000 => true, t2 = 600_000 <= 650_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(300_000, 600_000, 0, 500_000, &jt));
        // t1 = 350_000 < 350_000 => false (exact boundary)
        assert!(!JudgeAlgorithm::Combo.compare_times(350_000, 600_000, 0, 500_000, &jt));
    }

    #[test]
    fn compare_times_combo_both_conditions_must_hold() {
        let jt = make_judgetable();
        // t1 past boundary but t2 past early boundary => false
        assert!(!JudgeAlgorithm::Combo.compare_times(-200_000, 200_000, 0, 0, &jt));
        // t1 NOT past boundary but t2 within early boundary => false
        assert!(!JudgeAlgorithm::Combo.compare_times(-100_000, 50_000, 0, 0, &jt));
    }

    // --- Score (uses judgetable[1] = GREAT window) ---

    #[test]
    fn compare_times_score_t2_preferred_within_great_window() {
        let jt = make_judgetable();
        // ptime = 0
        // GREAT late boundary: 0 + (-60_000) = -60_000
        // GREAT early boundary: 0 + 60_000 = 60_000
        // t1 = -70_000 < -60_000 => true, t2 = 50_000 <= 60_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(-70_000, 50_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_t1_at_exact_great_late_boundary() {
        let jt = make_judgetable();
        // t1 = -60_000, boundary = -60_000 => -60_000 < -60_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(-60_000, 0, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_t1_just_past_great_late_boundary() {
        let jt = make_judgetable();
        // t1 = -60_001 < -60_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(-60_001, 0, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_t2_at_exact_great_early_boundary() {
        let jt = make_judgetable();
        // t2 = 60_000 <= 60_000 => true
        // t1 = -70_000 < -60_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(-70_000, 60_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_t2_just_past_great_early_boundary() {
        let jt = make_judgetable();
        // t2 = 60_001 <= 60_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(-70_000, 60_001, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_t1_within_great_window_rejects() {
        let jt = make_judgetable();
        // t1 = -50_000, boundary = -60_000 => -50_000 < -60_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(-50_000, 0, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_rejects_nonzero_t2_state() {
        let jt = make_judgetable();
        assert!(!JudgeAlgorithm::Score.compare_times(-70_000, 50_000, 1, 0, &jt));
        assert!(!JudgeAlgorithm::Score.compare_times(-70_000, 50_000, 2, 0, &jt));
    }

    #[test]
    fn compare_times_score_with_nonzero_ptime() {
        let jt = make_judgetable();
        // ptime = 1_000_000
        // GREAT late boundary: 1_000_000 + (-60_000) = 940_000
        // GREAT early boundary: 1_000_000 + 60_000 = 1_060_000
        // t1 = 900_000 < 940_000 => true, t2 = 1_050_000 <= 1_060_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(900_000, 1_050_000, 0, 1_000_000, &jt));
        // t1 = 940_000 < 940_000 => false (exact boundary)
        assert!(!JudgeAlgorithm::Score.compare_times(940_000, 1_050_000, 0, 1_000_000, &jt));
    }

    #[test]
    fn compare_times_score_both_conditions_must_hold() {
        let jt = make_judgetable();
        // t1 past but t2 past early => false
        assert!(!JudgeAlgorithm::Score.compare_times(-70_000, 70_000, 0, 0, &jt));
        // t1 NOT past but t2 within => false
        assert!(!JudgeAlgorithm::Score.compare_times(-50_000, 30_000, 0, 0, &jt));
    }

    // --- Score vs Combo use different judge table indices ---

    #[test]
    fn compare_times_score_and_combo_use_different_windows() {
        let jt = make_judgetable();
        // t1 = -100_000, t2 = 100_000, ptime = 0
        //
        // Score uses [1] GREAT: late=-60_000, early=60_000
        //   t1(-100_000) < -60_000 => true, t2(100_000) <= 60_000 => false => Score=false
        //
        // Combo uses [2] GOOD: late=-150_000, early=150_000
        //   t1(-100_000) < -150_000 => false => Combo=false
        assert!(!JudgeAlgorithm::Score.compare_times(-100_000, 100_000, 0, 0, &jt));
        assert!(!JudgeAlgorithm::Combo.compare_times(-100_000, 100_000, 0, 0, &jt));

        // t1 = -160_000, t2 = 100_000, ptime = 0
        // Score: t1(-160_000) < -60_000 => true, t2(100_000) <= 60_000 => false => false
        // Combo: t1(-160_000) < -150_000 => true, t2(100_000) <= 150_000 => true => true
        assert!(!JudgeAlgorithm::Score.compare_times(-160_000, 100_000, 0, 0, &jt));
        assert!(JudgeAlgorithm::Combo.compare_times(-160_000, 100_000, 0, 0, &jt));
    }

    // --- Equal timing cases ---

    #[test]
    fn compare_times_duration_equal_times() {
        let jt = make_judgetable();
        // t1 == t2 in time => equal distance => not strictly greater => false
        assert!(!JudgeAlgorithm::Duration.compare_times(50_000, 50_000, 0, 60_000, &jt));
    }

    #[test]
    fn compare_times_combo_equal_times() {
        let jt = make_judgetable();
        // t1 = t2 = -200_000, ptime = 0
        // t1(-200_000) < -150_000 => true
        // t2(-200_000) <= 150_000 => true
        // Both conditions met => true
        assert!(JudgeAlgorithm::Combo.compare_times(-200_000, -200_000, 0, 0, &jt));
    }

    #[test]
    fn compare_times_score_equal_times() {
        let jt = make_judgetable();
        // t1 = t2 = -70_000, ptime = 0
        // t1(-70_000) < -60_000 => true
        // t2(-70_000) <= 60_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(-70_000, -70_000, 0, 0, &jt));
    }

    // --- t2_state edge values ---

    #[test]
    fn compare_times_t2_state_zero_is_required() {
        let jt = make_judgetable();
        // Verify all state-dependent algorithms require t2_state == 0
        let combos = [
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Score,
        ];
        for algo in &combos {
            // Setup times so that timing conditions pass for Combo/Score/Duration
            let (t1, t2, ptime) = match algo {
                JudgeAlgorithm::Combo => (-200_000i64, 0i64, 0i64),
                JudgeAlgorithm::Score => (-70_000, 0, 0),
                JudgeAlgorithm::Duration => (100_000, 50_000, 60_000),
                _ => unreachable!(),
            };
            // state = 0 => condition met (timing is satisfied)
            assert!(
                algo.compare_times(t1, t2, 0, ptime, &jt),
                "{:?} should return true with state=0",
                algo
            );
            // state != 0 => always false regardless of timing
            for state in [1, -1, 2, 100, i32::MAX, i32::MIN] {
                assert!(
                    !algo.compare_times(t1, t2, state, ptime, &jt),
                    "{:?} should return false with state={}",
                    algo,
                    state
                );
            }
        }
    }

    // --- Tight judge table (narrow PGREAT-like windows) ---

    #[test]
    fn compare_times_combo_tight_table() {
        // Use a tight table where GOOD window is tiny
        let jt: Vec<[i64; 2]> = vec![
            [-1_000, 1_000],  // PGREAT
            [-3_000, 3_000],  // GREAT
            [-5_000, 5_000],  // GOOD
            [-10_000, 8_000], // BAD
            [-5_000, 20_000], // MISS
        ];
        // ptime = 100_000
        // GOOD late: 100_000 + (-5_000) = 95_000
        // GOOD early: 100_000 + 5_000 = 105_000
        // t1 = 94_999 < 95_000 => true, t2 = 105_000 <= 105_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(94_999, 105_000, 0, 100_000, &jt));
        // t1 = 95_000 < 95_000 => false
        assert!(!JudgeAlgorithm::Combo.compare_times(95_000, 105_000, 0, 100_000, &jt));
        // t2 = 105_001 <= 105_000 => false
        assert!(!JudgeAlgorithm::Combo.compare_times(94_999, 105_001, 0, 100_000, &jt));
    }

    #[test]
    fn compare_times_score_tight_table() {
        let jt: Vec<[i64; 2]> = vec![
            [-1_000, 1_000],  // PGREAT
            [-3_000, 3_000],  // GREAT
            [-5_000, 5_000],  // GOOD
            [-10_000, 8_000], // BAD
            [-5_000, 20_000], // MISS
        ];
        // ptime = 100_000
        // GREAT late: 100_000 + (-3_000) = 97_000
        // GREAT early: 100_000 + 3_000 = 103_000
        // t1 = 96_999 < 97_000 => true, t2 = 103_000 <= 103_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(96_999, 103_000, 0, 100_000, &jt));
        // t1 = 97_000 < 97_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(97_000, 103_000, 0, 100_000, &jt));
        // t2 = 103_001 <= 103_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(96_999, 103_001, 0, 100_000, &jt));
    }

    // --- Asymmetric judge table ---

    #[test]
    fn compare_times_asymmetric_judgetable() {
        // Asymmetric windows (late window larger than early window)
        let jt: Vec<[i64; 2]> = vec![
            [-20_000, 10_000],   // PGREAT
            [-80_000, 40_000],   // GREAT
            [-200_000, 80_000],  // GOOD
            [-300_000, 200_000], // BAD
            [-200_000, 500_000], // MISS
        ];
        // ptime = 0
        // Score (GREAT): late = -80_000, early = 40_000
        // t1 = -80_001 < -80_000 => true, t2 = 40_000 <= 40_000 => true
        assert!(JudgeAlgorithm::Score.compare_times(-80_001, 40_000, 0, 0, &jt));
        // t2 = 40_001 <= 40_000 => false
        assert!(!JudgeAlgorithm::Score.compare_times(-80_001, 40_001, 0, 0, &jt));

        // Combo (GOOD): late = -200_000, early = 80_000
        // t1 = -200_001 < -200_000 => true, t2 = 80_000 <= 80_000 => true
        assert!(JudgeAlgorithm::Combo.compare_times(-200_001, 80_000, 0, 0, &jt));
        // t2 = 80_001 <= 80_000 => false
        assert!(!JudgeAlgorithm::Combo.compare_times(-200_001, 80_001, 0, 0, &jt));
    }

    // --- Zero-width judge table ---

    #[test]
    fn compare_times_zero_width_judgetable() {
        let jt: Vec<[i64; 2]> = vec![
            [0, 0], // PGREAT
            [0, 0], // GREAT
            [0, 0], // GOOD
            [0, 0], // BAD
            [0, 0], // MISS
        ];
        // ptime = 100
        // Score (GREAT): late = 100 + 0 = 100, early = 100 + 0 = 100
        // t1 = 99 < 100 => true, t2 = 100 <= 100 => true
        assert!(JudgeAlgorithm::Score.compare_times(99, 100, 0, 100, &jt));
        // t1 = 100 < 100 => false
        assert!(!JudgeAlgorithm::Score.compare_times(100, 100, 0, 100, &jt));

        // Combo (GOOD): same zero-width
        assert!(JudgeAlgorithm::Combo.compare_times(99, 100, 0, 100, &jt));
        assert!(!JudgeAlgorithm::Combo.compare_times(100, 100, 0, 100, &jt));

        // Duration doesn't use judgetable, still works
        assert!(JudgeAlgorithm::Duration.compare_times(200, 100, 0, 100, &jt));
    }

    // --- Large timing values ---

    #[test]
    fn compare_times_large_values() {
        let jt = make_judgetable();
        let large = 1_000_000_000i64; // 1000 seconds in microseconds
        // Duration: |t1 - ptime| = 200_000, |t2 - ptime| = 100_000 => true
        assert!(JudgeAlgorithm::Duration.compare_times(
            large + 200_000,
            large + 100_000,
            0,
            large,
            &jt
        ));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    /// Valid 5-element judge table for property tests.
    fn make_judgetable() -> Vec<[i64; 2]> {
        vec![
            [-20_000, 20_000],
            [-60_000, 60_000],
            [-150_000, 150_000],
            [-280_000, 220_000],
            [-150_000, 500_000],
        ]
    }

    proptest! {
        /// Lowest always returns false regardless of inputs.
        #[test]
        fn lowest_always_false(t1 in any::<i64>(), t2 in any::<i64>(), t2_state in any::<i32>(), ptime in any::<i64>()) {
            let jt = make_judgetable();
            prop_assert!(!JudgeAlgorithm::Lowest.compare_times(t1, t2, t2_state, ptime, &jt));
        }

        /// When t2_state is nonzero, Duration/Combo/Score all return false.
        /// Timing values are bounded to avoid i64 subtraction overflow in Duration's
        /// `(t1_time - ptime).abs()` which is evaluated before the short-circuit check.
        #[test]
        fn nonzero_t2_state_always_false(
            t1 in -1_000_000_000i64..1_000_000_000,
            t2 in -1_000_000_000i64..1_000_000_000,
            t2_state in 1..=i32::MAX,
            ptime in -1_000_000_000i64..1_000_000_000,
        ) {
            let jt = make_judgetable();
            prop_assert!(!JudgeAlgorithm::Duration.compare_times(t1, t2, t2_state, ptime, &jt));
            prop_assert!(!JudgeAlgorithm::Combo.compare_times(t1, t2, t2_state, ptime, &jt));
            prop_assert!(!JudgeAlgorithm::Score.compare_times(t1, t2, t2_state, ptime, &jt));
        }

        /// Duration is antisymmetric: if t2 is strictly closer to ptime than t1,
        /// Duration returns true; swapping t1/t2 returns false.
        #[test]
        fn duration_antisymmetry(
            t1_delta in 1i64..1_000_000,
            t2_delta in 1i64..1_000_000,
            ptime in -500_000i64..500_000,
        ) {
            prop_assume!(t1_delta != t2_delta);
            let jt = make_judgetable();
            let (far_delta, close_delta) = if t1_delta > t2_delta {
                (t1_delta, t2_delta)
            } else {
                (t2_delta, t1_delta)
            };
            // t1 is farther, t2 is closer => Duration prefers t2 => true
            let t1_far = ptime + far_delta;
            let t2_close = ptime + close_delta;
            prop_assert!(JudgeAlgorithm::Duration.compare_times(t1_far, t2_close, 0, ptime, &jt));
            // Swap: t1 is closer, t2 is farther => false
            prop_assert!(!JudgeAlgorithm::Duration.compare_times(t2_close, t1_far, 0, ptime, &jt));
        }

        /// If Combo returns true, both sub-conditions must hold:
        ///   t1_time < ptime + judgetable[2][0]
        ///   t2_time <= ptime + judgetable[2][1]
        #[test]
        fn combo_requires_both_conditions(
            t1 in any::<i64>(),
            t2 in any::<i64>(),
            ptime in -1_000_000i64..1_000_000,
        ) {
            let jt = make_judgetable();
            if JudgeAlgorithm::Combo.compare_times(t1, t2, 0, ptime, &jt) {
                prop_assert!(t1 < ptime + jt[2][0], "t1 must be past GOOD late boundary");
                prop_assert!(t2 <= ptime + jt[2][1], "t2 must be within GOOD early boundary");
            }
        }
    }
}
