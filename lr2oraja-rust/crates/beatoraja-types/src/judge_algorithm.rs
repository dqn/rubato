// JudgeAlgorithm - moved from stubs.rs (Phase 30a)

/// Judge algorithm for score evaluation
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JudgeAlgorithm {
    Combo,
    Duration,
    Lowest,
    Timing,
}

impl JudgeAlgorithm {
    pub fn name(&self) -> &str {
        match self {
            JudgeAlgorithm::Combo => "Combo",
            JudgeAlgorithm::Duration => "Duration",
            JudgeAlgorithm::Lowest => "Lowest",
            JudgeAlgorithm::Timing => "Timing",
        }
    }

    pub fn get_index(name: &str) -> i32 {
        match name {
            "Combo" => 0,
            "Duration" => 1,
            "Lowest" => 2,
            "Timing" => 3,
            _ => -1,
        }
    }

    pub fn values() -> &'static [JudgeAlgorithm] {
        &[
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Timing,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_judge_algorithm_name() {
        assert_eq!(JudgeAlgorithm::Combo.name(), "Combo");
        assert_eq!(JudgeAlgorithm::Duration.name(), "Duration");
        assert_eq!(JudgeAlgorithm::Lowest.name(), "Lowest");
        assert_eq!(JudgeAlgorithm::Timing.name(), "Timing");
    }

    #[test]
    fn test_judge_algorithm_get_index() {
        assert_eq!(JudgeAlgorithm::get_index("Combo"), 0);
        assert_eq!(JudgeAlgorithm::get_index("Duration"), 1);
        assert_eq!(JudgeAlgorithm::get_index("Lowest"), 2);
        assert_eq!(JudgeAlgorithm::get_index("Timing"), 3);
        assert_eq!(JudgeAlgorithm::get_index("Unknown"), -1);
    }

    #[test]
    fn test_judge_algorithm_values() {
        let values = JudgeAlgorithm::values();
        assert_eq!(values.len(), 4);
        assert_eq!(values[0], JudgeAlgorithm::Combo);
        assert_eq!(values[1], JudgeAlgorithm::Duration);
        assert_eq!(values[2], JudgeAlgorithm::Lowest);
        assert_eq!(values[3], JudgeAlgorithm::Timing);
    }

    #[test]
    fn test_judge_algorithm_serde_round_trip() {
        let alg = JudgeAlgorithm::Duration;
        let json = serde_json::to_string(&alg).unwrap();
        let deserialized: JudgeAlgorithm = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, alg);
    }

    #[test]
    fn test_judge_algorithm_clone_debug_eq() {
        let a = JudgeAlgorithm::Combo;
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(format!("{:?}", a), "Combo");
    }
}
