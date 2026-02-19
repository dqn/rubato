use serde::{Deserialize, Serialize};

use crate::judge_property::JudgeWindowTable;

/// Algorithm for selecting which note to judge when multiple notes
/// are within the judge window at the same time.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum JudgeAlgorithm {
    /// Combo-first: prefer the note that maintains combo (selects t2 if it
    /// is unjudged and t1 falls within the GOOD window from press time).
    #[default]
    Combo,
    /// Duration-first: prefer the note closer in time to the press moment,
    /// as long as t2 is unjudged.
    Duration,
    /// Lowest-first: always prefer the earliest (lowest) note.
    /// Never switches to t2, so compare always returns false.
    Lowest,
    /// Score-first: prefer the note that maximizes score (selects t2 if it
    /// is unjudged and t1 falls within the GREAT window from press time).
    Score,
}

/// The default set of algorithms used in standard play.
///
/// The first element is used as the primary note selection algorithm
/// in JudgeManager initialization.
pub const DEFAULT_ALGORITHMS: [JudgeAlgorithm; 3] = [
    JudgeAlgorithm::Combo,
    JudgeAlgorithm::Duration,
    JudgeAlgorithm::Lowest,
];

impl JudgeAlgorithm {
    /// Compare two notes and decide whether to switch from t1 to t2.
    ///
    /// Returns `true` if t2 should be selected over t1, `false` otherwise.
    ///
    /// # Arguments
    /// * `t1_time` - Time of the currently selected note (microseconds)
    /// * `t2_time` - Time of the candidate replacement note (microseconds)
    /// * `t2_state` - State of t2 (0 = unjudged, non-zero = already judged)
    /// * `press_time` - Time of the key press (microseconds)
    /// * `judge_table` - Judge window table [PG, GR, GD, BD, MS]
    pub fn compare(
        self,
        t1_time: i64,
        t2_time: i64,
        t2_state: i32,
        press_time: i64,
        judge_table: &JudgeWindowTable,
    ) -> bool {
        match self {
            Self::Combo => {
                // Select t2 if: t2 is unjudged AND t1 is past the GOOD late limit
                // AND t2 is within the GOOD early limit
                t2_state == 0
                    && t1_time < press_time + judge_table[2][0]
                    && t2_time <= press_time + judge_table[2][1]
            }
            Self::Duration => {
                // Select t2 if: t2 is closer to press_time AND t2 is unjudged
                (t1_time - press_time).abs() > (t2_time - press_time).abs() && t2_state == 0
            }
            Self::Lowest => {
                // Always prefer the earliest note; never switch
                false
            }
            Self::Score => {
                // Select t2 if: t2 is unjudged AND t1 is past the GREAT late limit
                // AND t2 is within the GREAT early limit
                t2_state == 0
                    && t1_time < press_time + judge_table[1][0]
                    && t2_time <= press_time + judge_table[1][1]
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Create a standard SEVENKEYS-like judge table for testing.
    fn test_judge_table() -> JudgeWindowTable {
        vec![
            [-20000, 20000],   // PG
            [-60000, 60000],   // GR
            [-150000, 150000], // GD
            [-280000, 220000], // BD
            [-150000, 500000], // MS
        ]
    }

    // --- Combo algorithm tests ---

    #[test]
    fn combo_selects_t2_when_t1_past_good_and_t2_within() {
        let table = test_judge_table();
        // t1 is before (press_time + GD late limit), meaning t1 has passed the GOOD window
        // t2 is within GOOD early limit
        let t1_time = 0;
        let t2_time = 200000;
        let press_time = 200000; // t1 < 200000 + (-150000) = 50000 => 0 < 50000 is true
        // t2 <= 200000 + 150000 = 350000 => 200000 <= 350000 is true

        assert!(JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn combo_rejects_when_t2_already_judged() {
        let table = test_judge_table();
        let t1_time = 0;
        let t2_time = 200000;
        let press_time = 200000;

        assert!(!JudgeAlgorithm::Combo.compare(t1_time, t2_time, 1, press_time, &table));
    }

    #[test]
    fn combo_rejects_when_t1_within_good_window() {
        let table = test_judge_table();
        // t1 is within the GOOD window (not past it)
        let t1_time = 100000;
        let t2_time = 200000;
        let press_time = 100000; // t1 < 100000 + (-150000) = -50000 => 100000 < -50000 is false

        assert!(!JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn combo_rejects_when_t2_past_good_early() {
        let table = test_judge_table();
        let t1_time = 0;
        let press_time = 200000;
        let t2_time = 400000; // t2 <= 200000 + 150000 = 350000 => 400000 <= 350000 is false

        assert!(!JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));
    }

    // --- Duration algorithm tests ---

    #[test]
    fn duration_selects_closer_note() {
        let table = test_judge_table();
        // t2 is closer to press_time than t1
        let t1_time = 0;
        let t2_time = 90000;
        let press_time = 100000;
        // |0 - 100000| = 100000 > |90000 - 100000| = 10000

        assert!(JudgeAlgorithm::Duration.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn duration_rejects_farther_note() {
        let table = test_judge_table();
        let t1_time = 90000;
        let t2_time = 0;
        let press_time = 100000;
        // |90000 - 100000| = 10000 > |0 - 100000| = 100000 is false

        assert!(!JudgeAlgorithm::Duration.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn duration_rejects_equal_distance() {
        let table = test_judge_table();
        let t1_time = 50000;
        let t2_time = 150000;
        let press_time = 100000;
        // |50000 - 100000| = 50000 > |150000 - 100000| = 50000 is false (equal)

        assert!(!JudgeAlgorithm::Duration.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn duration_rejects_when_t2_already_judged() {
        let table = test_judge_table();
        let t1_time = 0;
        let t2_time = 90000;
        let press_time = 100000;

        assert!(!JudgeAlgorithm::Duration.compare(t1_time, t2_time, 1, press_time, &table));
    }

    // --- Lowest algorithm tests ---

    #[test]
    fn lowest_always_returns_false() {
        let table = test_judge_table();

        // Regardless of inputs, Lowest never switches
        assert!(!JudgeAlgorithm::Lowest.compare(0, 100000, 0, 50000, &table));
        assert!(!JudgeAlgorithm::Lowest.compare(100000, 0, 0, 50000, &table));
        assert!(!JudgeAlgorithm::Lowest.compare(0, 0, 0, 0, &table));
    }

    // --- Score algorithm tests ---

    #[test]
    fn score_selects_t2_when_t1_past_great_and_t2_within() {
        let table = test_judge_table();
        // t1 is past the GREAT late limit, t2 is within GREAT early limit
        let t1_time = 0;
        let t2_time = 200000;
        let press_time = 200000; // t1 < 200000 + (-60000) = 140000 => 0 < 140000 is true
        // t2 <= 200000 + 60000 = 260000 => 200000 <= 260000 is true

        assert!(JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn score_rejects_when_t2_already_judged() {
        let table = test_judge_table();
        let t1_time = 0;
        let t2_time = 200000;
        let press_time = 200000;

        assert!(!JudgeAlgorithm::Score.compare(t1_time, t2_time, 1, press_time, &table));
    }

    #[test]
    fn score_rejects_when_t1_within_great_window() {
        let table = test_judge_table();
        let t1_time = 150000;
        let t2_time = 250000;
        let press_time = 150000; // t1 < 150000 + (-60000) = 90000 => 150000 < 90000 is false

        assert!(!JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn score_rejects_when_t2_past_great_early() {
        let table = test_judge_table();
        let t1_time = 0;
        let press_time = 200000;
        let t2_time = 300000; // t2 <= 200000 + 60000 = 260000 => 300000 <= 260000 is false

        assert!(!JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));
    }

    // --- Default algorithm tests ---

    #[test]
    fn default_algorithm_is_combo() {
        assert_eq!(JudgeAlgorithm::default(), JudgeAlgorithm::Combo);
    }

    #[test]
    fn default_algorithms_are_combo_duration_lowest() {
        assert_eq!(
            DEFAULT_ALGORITHMS,
            [
                JudgeAlgorithm::Combo,
                JudgeAlgorithm::Duration,
                JudgeAlgorithm::Lowest
            ]
        );
    }

    // --- Boundary value tests ---

    #[test]
    fn combo_boundary_at_good_late_limit() {
        let table = test_judge_table();
        // Exactly at the boundary: t1_time == press_time + judge_table[2][0]
        let press_time = 200000;
        let t1_time = press_time + table[2][0]; // 200000 + (-150000) = 50000
        let t2_time = 250000; // within early limit

        // t1 < 50000 is false (t1 == 50000), so should NOT switch
        assert!(!JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));

        // Just past the boundary
        let t1_time = t1_time - 1;
        assert!(JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn combo_boundary_at_good_early_limit() {
        let table = test_judge_table();
        let press_time = 0;
        let t1_time = -200000; // well past good window
        let t2_time = press_time + table[2][1]; // 0 + 150000 = 150000

        // t2 <= 150000 is true, so should switch
        assert!(JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));

        // Just past the boundary
        let t2_time = t2_time + 1;
        assert!(!JudgeAlgorithm::Combo.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn score_boundary_at_great_late_limit() {
        let table = test_judge_table();
        let press_time = 200000;
        let t1_time = press_time + table[1][0]; // 200000 + (-60000) = 140000
        let t2_time = 250000;

        // t1 < 140000 is false (t1 == 140000)
        assert!(!JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));

        // Just past the boundary
        let t1_time = t1_time - 1;
        assert!(JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));
    }

    #[test]
    fn score_boundary_at_great_early_limit() {
        let table = test_judge_table();
        let press_time = 0;
        let t1_time = -200000;
        let t2_time = press_time + table[1][1]; // 0 + 60000 = 60000

        // t2 <= 60000 is true
        assert!(JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));

        // Just past the boundary
        let t2_time = t2_time + 1;
        assert!(!JudgeAlgorithm::Score.compare(t1_time, t2_time, 0, press_time, &table));
    }

    // --- Serde roundtrip test ---

    #[test]
    fn serde_roundtrip() {
        let algorithms = [
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Score,
        ];

        for algo in &algorithms {
            let json = serde_json::to_string(algo).unwrap();
            let deserialized: JudgeAlgorithm = serde_json::from_str(&json).unwrap();
            assert_eq!(*algo, deserialized);
        }
    }
}
