// PatternModifyLog DTO - moved from stubs.rs (Phase 30b)
// NOTE: This is the DTO version with pub fields for JSON serde.
// The runtime version exists in beatoraja-pattern with a different field layout.

/// Stub for beatoraja.pattern.PatternModifyLog (field layout differs from beatoraja-pattern)
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PatternModifyLog {
    pub old_lane: i32,
    pub new_lane: i32,
}

impl PatternModifyLog {
    pub fn validate(&self) -> bool {
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_modify_log_default() {
        let log = PatternModifyLog::default();
        assert_eq!(log.old_lane, 0);
        assert_eq!(log.new_lane, 0);
    }

    #[test]
    fn test_pattern_modify_log_validate() {
        let log = PatternModifyLog {
            old_lane: 2,
            new_lane: 5,
        };
        assert!(log.validate());
    }

    #[test]
    fn test_pattern_modify_log_serde_round_trip() {
        let log = PatternModifyLog {
            old_lane: 3,
            new_lane: 7,
        };
        let json = serde_json::to_string(&log).unwrap();
        let deserialized: PatternModifyLog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.old_lane, 3);
        assert_eq!(deserialized.new_lane, 7);
    }

    #[test]
    fn test_pattern_modify_log_clone() {
        let log = PatternModifyLog {
            old_lane: 1,
            new_lane: 4,
        };
        let cloned = log.clone();
        assert_eq!(cloned.old_lane, log.old_lane);
        assert_eq!(cloned.new_lane, log.new_lane);
    }
}
