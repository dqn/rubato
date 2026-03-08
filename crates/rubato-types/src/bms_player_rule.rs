// BMSPlayerRule - moved from stubs.rs (Phase 30a)

use bms_model::bms_model::BMSModel;
use bms_model::bms_model::{JudgeRankType, TotalType};
use bms_model::mode::Mode;

/// BMS player rule (LR2 or Beatoraja)
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum BMSPlayerRule {
    LR2,
    Beatoraja,
}

/// Judgerank lookup table for a given mode.
/// These values mirror JudgeWindowRule.judgerank from beatoraja-play.
///
/// Java: BMSPlayerRule.getBMSPlayerRule(mode).judge.windowrule.judgerank
fn judgerank_table(mode: Option<&Mode>) -> &'static [i32; 5] {
    match mode {
        Some(m) if *m == Mode::POPN_5K || *m == Mode::POPN_9K => {
            // PMS rule: [33, 50, 70, 100, 133]
            &[33, 50, 70, 100, 133]
        }
        _ => {
            // Normal rule (5K, 7K, 24K, etc.): [25, 50, 75, 100, 125]
            &[25, 50, 75, 100, 125]
        }
    }
}

/// Calculate default TOTAL value.
///
/// Java: BMSPlayerRule.calculateDefaultTotal(Mode, int)
fn calculate_default_total(total_notes: i32) -> f64 {
    let notes = total_notes as f64;
    160.0 + (notes + (notes - 400.0).clamp(0.0, 200.0)) * 0.16
}

impl BMSPlayerRule {
    /// Validate and normalize judgerank and total values in a BMS model.
    ///
    /// Converts judgerank from BMS_RANK/BMS_DEFEXRANK/BMSON_JUDGERANK to a
    /// normalized BMSON_JUDGERANK value. Sets default TOTAL if unspecified.
    ///
    /// Java: BMSPlayerRule.validate(BMSModel)
    pub fn validate(model: &mut BMSModel) {
        let judgerank = model.judgerank;
        let table = judgerank_table(model.mode());

        match &model.judgerank_type {
            JudgeRankType::BmsRank => {
                let idx = if (0..5).contains(&judgerank) {
                    judgerank as usize
                } else {
                    2 // default to rank 2 (NORMAL)
                };
                model.judgerank = table[idx];
            }
            JudgeRankType::BmsDefexrank => {
                if judgerank > 0 {
                    model.judgerank = judgerank * table[2] / 100;
                } else {
                    model.judgerank = table[2];
                }
            }
            JudgeRankType::BmsonJudgerank => {
                if judgerank <= 0 {
                    model.judgerank = 100;
                }
            }
        }
        model.judgerank_type = JudgeRankType::BmsonJudgerank;

        match model.total_type {
            TotalType::Bms => {
                if model.total <= 0.0 {
                    model.total = calculate_default_total(model.total_notes());
                }
            }
            TotalType::Bmson => {
                let default_total = calculate_default_total(model.total_notes());
                if model.total > 0.0 {
                    model.total = model.total / 100.0 * default_total;
                } else {
                    model.total = default_total;
                }
            }
        }
        model.total_type = TotalType::Bms;
    }
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

    #[test]
    fn test_calculate_default_total() {
        // 100 notes: 160 + (100 + 0) * 0.16 = 160 + 16 = 176
        assert!((calculate_default_total(100) - 176.0).abs() < 0.001);

        // 500 notes: 160 + (500 + min(max(100, 0), 200)) * 0.16 = 160 + 600*0.16 = 256
        assert!((calculate_default_total(500) - 256.0).abs() < 0.001);

        // 700 notes: 160 + (700 + min(max(300, 0), 200)) * 0.16 = 160 + 900*0.16 = 304
        assert!((calculate_default_total(700) - 304.0).abs() < 0.001);
    }

    #[test]
    fn test_validate_bms_rank() {
        let mut model = BMSModel::new();
        model.judgerank = 2; // NORMAL rank
        model.judgerank_type = JudgeRankType::BmsRank;
        model.total = 300.0;
        model.total_type = TotalType::Bms;

        BMSPlayerRule::validate(&mut model);

        // Rank 2 with normal table → 75
        assert_eq!(model.judgerank, 75);
        assert_eq!(model.judgerank_type, JudgeRankType::BmsonJudgerank);
        assert_eq!(model.total_type, TotalType::Bms);
        // Total was > 0, should be unchanged
        assert!((model.total - 300.0).abs() < 0.001);
    }

    #[test]
    fn test_validate_bms_rank_out_of_range_uses_default() {
        let mut model = BMSModel::new();
        model.judgerank = 10; // out of range
        model.judgerank_type = JudgeRankType::BmsRank;
        model.total = 100.0;
        model.total_type = TotalType::Bms;

        BMSPlayerRule::validate(&mut model);

        // Out of range → defaults to index 2 → 75
        assert_eq!(model.judgerank, 75);
    }

    #[test]
    fn test_validate_defexrank() {
        let mut model = BMSModel::new();
        model.judgerank = 150; // 150% of normal
        model.judgerank_type = JudgeRankType::BmsDefexrank;
        model.total = 100.0;
        model.total_type = TotalType::Bms;

        BMSPlayerRule::validate(&mut model);

        // 150 * 75 / 100 = 112
        assert_eq!(model.judgerank, 112);
    }

    #[test]
    fn test_validate_bmson_judgerank_zero() {
        let mut model = BMSModel::new();
        model.judgerank = 0;
        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        model.total = 100.0;
        model.total_type = TotalType::Bms;

        BMSPlayerRule::validate(&mut model);

        // 0 → default to 100
        assert_eq!(model.judgerank, 100);
    }

    #[test]
    fn test_validate_total_bms_unset() {
        let mut model = BMSModel::new();
        model.judgerank = 100;
        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        model.total = 0.0; // unset
        model.total_type = TotalType::Bms;

        BMSPlayerRule::validate(&mut model);

        // Should calculate default total
        assert!(model.total > 0.0);
    }

    #[test]
    fn test_validate_total_bmson_percentage() {
        let mut model = BMSModel::new();
        model.judgerank = 100;
        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        model.total = 200.0; // 200% of default
        model.total_type = TotalType::Bmson;

        BMSPlayerRule::validate(&mut model);

        let default_total = calculate_default_total(model.total_notes());
        let expected = 200.0 / 100.0 * default_total;
        assert!((model.total - expected).abs() < 0.001);
        assert_eq!(model.total_type, TotalType::Bms);
    }
}
