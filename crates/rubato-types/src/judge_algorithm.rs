// JudgeAlgorithm

use std::fmt;
use std::str::FromStr;

/// Judge algorithm for score evaluation
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JudgeAlgorithm {
    Combo,
    Duration,
    Lowest,
    Score,
}

impl FromStr for JudgeAlgorithm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Combo" => Ok(Self::Combo),
            "Duration" => Ok(Self::Duration),
            "Lowest" => Ok(Self::Lowest),
            "Score" => Ok(Self::Score),
            _ => anyhow::bail!("unknown JudgeAlgorithm: {}", s),
        }
    }
}

impl fmt::Display for JudgeAlgorithm {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Combo => write!(f, "Combo"),
            Self::Duration => write!(f, "Duration"),
            Self::Lowest => write!(f, "Lowest"),
            Self::Score => write!(f, "Score"),
        }
    }
}

impl JudgeAlgorithm {
    pub fn name(&self) -> &str {
        match self {
            JudgeAlgorithm::Combo => "Combo",
            JudgeAlgorithm::Duration => "Duration",
            JudgeAlgorithm::Lowest => "Lowest",
            JudgeAlgorithm::Score => "Score",
        }
    }

    pub fn index(name: &str) -> i32 {
        Self::values()
            .iter()
            .position(|v| v.to_string() == name)
            .map(|i| i as i32)
            .unwrap_or(-1)
    }

    pub fn values() -> &'static [JudgeAlgorithm] {
        &[
            JudgeAlgorithm::Combo,
            JudgeAlgorithm::Duration,
            JudgeAlgorithm::Lowest,
            JudgeAlgorithm::Score,
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
        assert_eq!(JudgeAlgorithm::Score.name(), "Score");
    }

    #[test]
    fn test_judge_algorithm_get_index() {
        assert_eq!(JudgeAlgorithm::index("Combo"), 0);
        assert_eq!(JudgeAlgorithm::index("Duration"), 1);
        assert_eq!(JudgeAlgorithm::index("Lowest"), 2);
        assert_eq!(JudgeAlgorithm::index("Score"), 3);
        assert_eq!(JudgeAlgorithm::index("Unknown"), -1);
    }

    #[test]
    fn test_judge_algorithm_values() {
        let values = JudgeAlgorithm::values();
        assert_eq!(values.len(), 4);
        assert_eq!(values[0], JudgeAlgorithm::Combo);
        assert_eq!(values[1], JudgeAlgorithm::Duration);
        assert_eq!(values[2], JudgeAlgorithm::Lowest);
        assert_eq!(values[3], JudgeAlgorithm::Score);
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
        let b = a;
        assert_eq!(a, b);
        assert_eq!(format!("{:?}", a), "Combo");
    }

    #[test]
    fn test_judge_algorithm_from_str() {
        assert_eq!(
            "Combo".parse::<JudgeAlgorithm>().unwrap(),
            JudgeAlgorithm::Combo
        );
        assert_eq!(
            "Duration".parse::<JudgeAlgorithm>().unwrap(),
            JudgeAlgorithm::Duration
        );
        assert_eq!(
            "Lowest".parse::<JudgeAlgorithm>().unwrap(),
            JudgeAlgorithm::Lowest
        );
        assert_eq!(
            "Score".parse::<JudgeAlgorithm>().unwrap(),
            JudgeAlgorithm::Score
        );
        assert!("Unknown".parse::<JudgeAlgorithm>().is_err());
        assert!("".parse::<JudgeAlgorithm>().is_err());
    }

    #[test]
    fn test_judge_algorithm_display() {
        assert_eq!(JudgeAlgorithm::Combo.to_string(), "Combo");
        assert_eq!(JudgeAlgorithm::Duration.to_string(), "Duration");
        assert_eq!(JudgeAlgorithm::Lowest.to_string(), "Lowest");
        assert_eq!(JudgeAlgorithm::Score.to_string(), "Score");
    }

    #[test]
    fn test_judge_algorithm_display_from_str_round_trip() {
        for alg in JudgeAlgorithm::values() {
            let s = alg.to_string();
            let parsed: JudgeAlgorithm = s.parse().unwrap();
            assert_eq!(*alg, parsed);
        }
    }

    #[test]
    fn test_judge_algorithm_index_matches_values_position() {
        for (i, alg) in JudgeAlgorithm::values().iter().enumerate() {
            assert_eq!(JudgeAlgorithm::index(&alg.to_string()), i as i32);
        }
    }

    #[test]
    fn test_judge_algorithm_from_str_case_sensitive() {
        // FromStr is case-sensitive
        assert!("combo".parse::<JudgeAlgorithm>().is_err());
        assert!("COMBO".parse::<JudgeAlgorithm>().is_err());
        assert!("duration".parse::<JudgeAlgorithm>().is_err());
    }

    #[test]
    fn test_judge_algorithm_index_empty_string() {
        assert_eq!(JudgeAlgorithm::index(""), -1);
    }

    #[test]
    fn test_judge_algorithm_serde_all_variants() {
        for alg in JudgeAlgorithm::values() {
            let json = serde_json::to_string(alg).unwrap();
            let back: JudgeAlgorithm = serde_json::from_str(&json).unwrap();
            assert_eq!(*alg, back);
        }
    }

    #[test]
    fn test_judge_algorithm_serde_from_json_string() {
        let alg: JudgeAlgorithm = serde_json::from_str("\"Score\"").unwrap();
        assert_eq!(alg, JudgeAlgorithm::Score);
    }

    #[test]
    fn test_judge_algorithm_serde_invalid_variant() {
        let result: Result<JudgeAlgorithm, _> = serde_json::from_str("\"Invalid\"");
        assert!(result.is_err());
    }
}
