// JudgeAlgorithm - moved from stubs.rs (Phase 30a)

use std::fmt;
use std::str::FromStr;

/// Judge algorithm for score evaluation
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JudgeAlgorithm {
    Combo,
    Duration,
    Lowest,
    Timing,
}

impl FromStr for JudgeAlgorithm {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "Combo" => Ok(Self::Combo),
            "Duration" => Ok(Self::Duration),
            "Lowest" => Ok(Self::Lowest),
            "Timing" => Ok(Self::Timing),
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
            Self::Timing => write!(f, "Timing"),
        }
    }
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
        assert_eq!(JudgeAlgorithm::index("Combo"), 0);
        assert_eq!(JudgeAlgorithm::index("Duration"), 1);
        assert_eq!(JudgeAlgorithm::index("Lowest"), 2);
        assert_eq!(JudgeAlgorithm::index("Timing"), 3);
        assert_eq!(JudgeAlgorithm::index("Unknown"), -1);
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
            "Timing".parse::<JudgeAlgorithm>().unwrap(),
            JudgeAlgorithm::Timing
        );
        assert!("Unknown".parse::<JudgeAlgorithm>().is_err());
        assert!("".parse::<JudgeAlgorithm>().is_err());
    }

    #[test]
    fn test_judge_algorithm_display() {
        assert_eq!(JudgeAlgorithm::Combo.to_string(), "Combo");
        assert_eq!(JudgeAlgorithm::Duration.to_string(), "Duration");
        assert_eq!(JudgeAlgorithm::Lowest.to_string(), "Lowest");
        assert_eq!(JudgeAlgorithm::Timing.to_string(), "Timing");
    }

    #[test]
    fn test_judge_algorithm_display_from_str_round_trip() {
        for alg in JudgeAlgorithm::values() {
            let s = alg.to_string();
            let parsed: JudgeAlgorithm = s.parse().unwrap();
            assert_eq!(*alg, parsed);
        }
    }
}
