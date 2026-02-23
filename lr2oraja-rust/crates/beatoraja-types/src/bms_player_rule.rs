// BMSPlayerRule - moved from stubs.rs (Phase 30a)

/// BMS player rule (LR2 or Beatoraja)
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BMSPlayerRule {
    LR2,
    Beatoraja,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bms_player_rule_variants() {
        let lr2 = BMSPlayerRule::LR2;
        let beatoraja = BMSPlayerRule::Beatoraja;
        assert_ne!(lr2, beatoraja);
    }

    #[test]
    fn test_bms_player_rule_serde_round_trip() {
        let rule = BMSPlayerRule::LR2;
        let json = serde_json::to_string(&rule).unwrap();
        let deserialized: BMSPlayerRule = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, rule);

        let rule2 = BMSPlayerRule::Beatoraja;
        let json2 = serde_json::to_string(&rule2).unwrap();
        let deserialized2: BMSPlayerRule = serde_json::from_str(&json2).unwrap();
        assert_eq!(deserialized2, rule2);
    }

    #[test]
    fn test_bms_player_rule_clone_debug_eq() {
        let a = BMSPlayerRule::Beatoraja;
        let b = a.clone();
        assert_eq!(a, b);
        assert_eq!(format!("{:?}", a), "Beatoraja");
    }
}
