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
                t2.get_state() == 0
                    && t1.get_micro_time() < ptime + judgetable[2][0]
                    && t2.get_micro_time() <= ptime + judgetable[2][1]
            }
            JudgeAlgorithm::Duration => {
                (t1.get_micro_time() - ptime).abs() > (t2.get_micro_time() - ptime).abs()
                    && t2.get_state() == 0
            }
            JudgeAlgorithm::Lowest => false,
            JudgeAlgorithm::Score => {
                t2.get_state() == 0
                    && t1.get_micro_time() < ptime + judgetable[1][0]
                    && t2.get_micro_time() <= ptime + judgetable[1][1]
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

    pub fn get_index(algorithm: &str) -> i32 {
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
        assert_eq!(JudgeAlgorithm::get_index("Combo"), 0);
        assert_eq!(JudgeAlgorithm::get_index("Duration"), 1);
        assert_eq!(JudgeAlgorithm::get_index("Lowest"), 2);
        assert_eq!(JudgeAlgorithm::get_index("Score"), 3);
    }

    #[test]
    fn get_index_returns_negative_one_for_unknown() {
        assert_eq!(JudgeAlgorithm::get_index("Unknown"), -1);
        assert_eq!(JudgeAlgorithm::get_index(""), -1);
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
}
