use crate::validatable::Validatable;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PatternModifyLog {
    pub section: f64,
    pub modify: Option<Vec<i32>>,
}

impl Default for PatternModifyLog {
    fn default() -> Self {
        PatternModifyLog {
            section: -1.0,
            modify: None,
        }
    }
}

impl PatternModifyLog {
    pub fn new(section: f64, modify: Vec<i32>) -> Self {
        PatternModifyLog {
            section,
            modify: Some(modify),
        }
    }
}

impl Validatable for PatternModifyLog {
    fn validate(&mut self) -> bool {
        self.section >= 0.0 && self.modify.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_has_negative_section_and_no_modify() {
        let log = PatternModifyLog::default();
        assert_eq!(log.section, -1.0);
        assert!(log.modify.is_none());
    }

    #[test]
    fn new_sets_section_and_modify() {
        let modify = vec![0, 2, 1, 3];
        let log = PatternModifyLog::new(1.0, modify.clone());
        assert_eq!(log.section, 1.0);
        assert_eq!(log.modify, Some(modify));
    }

    #[test]
    fn new_with_empty_modify_vec() {
        let log = PatternModifyLog::new(0.0, Vec::new());
        assert_eq!(log.section, 0.0);
        assert_eq!(log.modify, Some(Vec::new()));
    }

    #[test]
    fn validate_returns_false_for_default() {
        let mut log = PatternModifyLog::default();
        assert!(!log.validate());
    }

    #[test]
    fn validate_returns_false_for_negative_section() {
        let mut log = PatternModifyLog::new(-1.0, vec![0, 1]);
        assert!(!log.validate());
    }

    #[test]
    fn validate_returns_false_for_none_modify() {
        let mut log = PatternModifyLog {
            section: 1.0,
            modify: None,
        };
        assert!(!log.validate());
    }

    #[test]
    fn validate_returns_true_for_valid_log() {
        let mut log = PatternModifyLog::new(0.0, vec![0, 1, 2]);
        assert!(log.validate());
    }

    #[test]
    fn validate_returns_true_for_section_zero() {
        let mut log = PatternModifyLog::new(0.0, vec![0]);
        assert!(log.validate());
    }

    #[test]
    fn validate_returns_true_for_positive_section() {
        let mut log = PatternModifyLog::new(5.5, vec![3, 2, 1, 0]);
        assert!(log.validate());
    }

    #[test]
    fn serialization_roundtrip() {
        let log = PatternModifyLog::new(2.5, vec![1, 0, 3, 2]);
        let json = serde_json::to_string(&log).unwrap();
        let deserialized: PatternModifyLog = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.section, 2.5);
        assert_eq!(deserialized.modify, Some(vec![1, 0, 3, 2]));
    }

    #[test]
    fn clone_is_independent() {
        let log = PatternModifyLog::new(1.0, vec![0, 1]);
        let mut cloned = log.clone();
        cloned.section = 2.0;
        assert_eq!(log.section, 1.0);
    }
}
