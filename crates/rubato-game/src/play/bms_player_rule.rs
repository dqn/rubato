use crate::play::gauge_property::GaugeProperty;
use crate::play::judge::property::{JudgeProperty, JudgePropertyType};
use bms::model::bms_model::{BMSModel, JudgeRankType, TotalType};
use bms::model::mode::Mode;

/// Player rule
#[derive(Clone, Debug)]
pub struct BMSPlayerRule {
    /// Gauge specification
    pub gauge: GaugeProperty,
    /// Judge specification
    pub judge: JudgeProperty,
    /// Target modes. Empty means all modes
    pub mode: Vec<Mode>,
}

impl BMSPlayerRule {
    fn new(gauge: GaugeProperty, judge_type: JudgePropertyType, modes: Vec<Mode>) -> Self {
        BMSPlayerRule {
            gauge,
            judge: judge_type.get(),
            mode: modes,
        }
    }

    pub fn for_mode(mode: &Mode) -> BMSPlayerRule {
        let ruleset = bms_player_rule_set_lr2();
        for rule in &ruleset {
            if rule.mode.is_empty() {
                return rule.clone();
            }
            for m in &rule.mode {
                if m == mode {
                    return rule.clone();
                }
            }
        }
        // fallback: LR2
        BMSPlayerRule::new(GaugeProperty::Lr2, JudgePropertyType::Lr2, vec![])
    }

    pub fn validate(model: &mut BMSModel) {
        let mode = model.mode().copied().unwrap_or(Mode::BEAT_7K);
        let rule = Self::for_mode(&mode);
        let judgerank = model.judgerank;
        match &model.judgerank_type {
            JudgeRankType::BmsRank => {
                let new_rank = if (0..5).contains(&judgerank) {
                    rule.judge.windowrule.judgerank[judgerank as usize]
                } else {
                    rule.judge.windowrule.judgerank[2]
                };
                model.judgerank = new_rank;
            }
            JudgeRankType::BmsDefexrank => {
                let new_rank = if judgerank > 0 {
                    judgerank * rule.judge.windowrule.judgerank[2] / 100
                } else {
                    rule.judge.windowrule.judgerank[2]
                };
                model.judgerank = new_rank;
            }
            JudgeRankType::BmsonJudgerank => {
                let new_rank = if judgerank > 0 { judgerank } else { 100 };
                model.judgerank = new_rank;
            }
        }
        model.judgerank_type = JudgeRankType::BmsonJudgerank;

        let totalnotes = model.total_notes();
        match model.total_type {
            TotalType::Bms => {
                // TOTAL undefined case
                if model.total <= 0.0 {
                    model.total = calculate_default_total(&mode, totalnotes);
                }
            }
            TotalType::Bmson => {
                let total = calculate_default_total(&mode, totalnotes);
                let new_total = if model.total > 0.0 {
                    model.total / 100.0 * total
                } else {
                    total
                };
                model.total = new_total;
            }
        }
        model.total_type = TotalType::Bms;
    }
}

/// Java: BMSPlayerRule.calculateDefaultTotal
///
/// For most modes: max(260.0, 7.605 * n / (0.01 * n + 6.5))
/// For KEYBOARD modes: max(300.0, 7.605 * (n + 100) / (0.01 * n + 6.5))
fn calculate_default_total(mode: &Mode, totalnotes: i32) -> f64 {
    let n = totalnotes as f64;
    match mode {
        Mode::KEYBOARD_24K | Mode::KEYBOARD_24K_DOUBLE => {
            (300.0f64).max(7.605 * (n + 100.0) / (0.01 * n + 6.5))
        }
        _ => (260.0f64).max(7.605 * n / (0.01 * n + 6.5)),
    }
}

/// BMSPlayerRuleSet::LR2
///
/// Java BMSPlayerRule.java:19 uses `JudgeProperty.SEVENKEYS` for the LR2 ruleset.
/// Rubato intentionally uses `JudgePropertyType::Lr2` instead, which provides a more
/// accurate approximation of LR2's non-linear judge window scaling behavior
/// (see `lr2_judge_scaling` and `LR2_SCALING` in judge/property.rs).
fn bms_player_rule_set_lr2() -> Vec<BMSPlayerRule> {
    vec![BMSPlayerRule::new(
        GaugeProperty::Lr2,
        JudgePropertyType::Lr2,
        vec![],
    )]
}

/// BMSPlayerRuleSet::Beatoraja
#[cfg(test)]
fn bms_player_rule_set_beatoraja() -> Vec<BMSPlayerRule> {
    vec![
        BMSPlayerRule::new(
            GaugeProperty::FiveKeys,
            JudgePropertyType::FiveKeys,
            vec![Mode::BEAT_5K, Mode::BEAT_10K],
        ),
        BMSPlayerRule::new(
            GaugeProperty::SevenKeys,
            JudgePropertyType::SevenKeys,
            vec![Mode::BEAT_7K, Mode::BEAT_14K],
        ),
        BMSPlayerRule::new(
            GaugeProperty::Pms,
            JudgePropertyType::Pms,
            vec![Mode::POPN_5K, Mode::POPN_9K],
        ),
        BMSPlayerRule::new(
            GaugeProperty::Keyboard,
            JudgePropertyType::Keyboard,
            vec![Mode::KEYBOARD_24K, Mode::KEYBOARD_24K_DOUBLE],
        ),
        BMSPlayerRule::new(
            GaugeProperty::SevenKeys,
            JudgePropertyType::SevenKeys,
            vec![],
        ),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- get_bms_player_rule tests ---

    #[test]
    fn lr2_ruleset_returns_lr2_for_any_mode() {
        // LR2 ruleset has a single rule with empty mode list (matches all)
        let modes = [
            Mode::BEAT_5K,
            Mode::BEAT_7K,
            Mode::BEAT_10K,
            Mode::BEAT_14K,
            Mode::POPN_5K,
            Mode::POPN_9K,
            Mode::KEYBOARD_24K,
            Mode::KEYBOARD_24K_DOUBLE,
        ];
        for mode in &modes {
            let rule = BMSPlayerRule::for_mode(mode);
            assert_eq!(rule.gauge, GaugeProperty::Lr2);
        }
    }

    #[test]
    fn lr2_ruleset_returns_lr2_gauge() {
        let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
        assert_eq!(rule.gauge, GaugeProperty::Lr2);
    }

    #[test]
    fn lr2_ruleset_has_empty_mode_list() {
        let rule = BMSPlayerRule::for_mode(&Mode::BEAT_7K);
        assert!(rule.mode.is_empty());
    }

    // --- beatoraja ruleset tests ---

    #[test]
    fn rubato_ruleset_has_five_entries() {
        let ruleset = bms_player_rule_set_beatoraja();
        assert_eq!(ruleset.len(), 5);
    }

    #[test]
    fn rubato_ruleset_fivekeys_for_5k() {
        let ruleset = bms_player_rule_set_beatoraja();
        assert_eq!(ruleset[0].gauge, GaugeProperty::FiveKeys);
        assert!(ruleset[0].mode.contains(&Mode::BEAT_5K));
        assert!(ruleset[0].mode.contains(&Mode::BEAT_10K));
    }

    #[test]
    fn rubato_ruleset_sevenkeys_for_7k() {
        let ruleset = bms_player_rule_set_beatoraja();
        assert_eq!(ruleset[1].gauge, GaugeProperty::SevenKeys);
        assert!(ruleset[1].mode.contains(&Mode::BEAT_7K));
        assert!(ruleset[1].mode.contains(&Mode::BEAT_14K));
    }

    #[test]
    fn rubato_ruleset_pms_for_popn() {
        let ruleset = bms_player_rule_set_beatoraja();
        assert_eq!(ruleset[2].gauge, GaugeProperty::Pms);
        assert!(ruleset[2].mode.contains(&Mode::POPN_5K));
        assert!(ruleset[2].mode.contains(&Mode::POPN_9K));
    }

    #[test]
    fn rubato_ruleset_keyboard_for_24k() {
        let ruleset = bms_player_rule_set_beatoraja();
        assert_eq!(ruleset[3].gauge, GaugeProperty::Keyboard);
        assert!(ruleset[3].mode.contains(&Mode::KEYBOARD_24K));
        assert!(ruleset[3].mode.contains(&Mode::KEYBOARD_24K_DOUBLE));
    }

    #[test]
    fn rubato_ruleset_fallback_is_sevenkeys() {
        let ruleset = bms_player_rule_set_beatoraja();
        // Last entry has empty mode list (catches everything else)
        assert_eq!(ruleset[4].gauge, GaugeProperty::SevenKeys);
        assert!(ruleset[4].mode.is_empty());
    }

    // --- calculate_default_total tests ---

    // Java formula: max(260.0, 7.605 * n / (0.01 * n + 6.5))
    // For KEYBOARD: max(300.0, 7.605 * (n+100) / (0.01 * n + 6.5))

    #[test]
    fn default_total_with_zero_notes() {
        // 7.605 * 0 / (0.0 + 6.5) = 0 < 260 => 260
        let total = calculate_default_total(&Mode::BEAT_7K, 0);
        assert!((total - 260.0).abs() < 1e-9);
    }

    #[test]
    fn default_total_with_400_notes() {
        // 7.605 * 400 / (4.0 + 6.5) = 3042 / 10.5 = 289.71...
        let n = 400.0f64;
        let expected = 260.0f64.max(7.605 * n / (0.01 * n + 6.5));
        let total = calculate_default_total(&Mode::BEAT_7K, 400);
        assert!((total - expected).abs() < 1e-9);
    }

    #[test]
    fn default_total_with_500_notes() {
        // 7.605 * 500 / (5.0 + 6.5) = 3802.5 / 11.5 = 330.65...
        let n = 500.0f64;
        let expected = 260.0f64.max(7.605 * n / (0.01 * n + 6.5));
        let total = calculate_default_total(&Mode::BEAT_7K, 500);
        assert!((total - expected).abs() < 1e-9);
        // Sanity: must be higher than old formula result
        assert!(total > 260.0);
    }

    #[test]
    fn default_total_with_700_notes() {
        let n = 700.0f64;
        let expected = 260.0f64.max(7.605 * n / (0.01 * n + 6.5));
        let total = calculate_default_total(&Mode::BEAT_7K, 700);
        assert!((total - expected).abs() < 1e-9);
    }

    #[test]
    fn default_total_with_1000_notes() {
        let n = 1000.0f64;
        let expected = 260.0f64.max(7.605 * n / (0.01 * n + 6.5));
        let total = calculate_default_total(&Mode::BEAT_7K, 1000);
        assert!((total - expected).abs() < 1e-9);
    }

    #[test]
    fn default_total_with_200_notes() {
        // 7.605 * 200 / (2.0 + 6.5) = 1521 / 8.5 = 178.9... < 260 => 260
        let total = calculate_default_total(&Mode::BEAT_7K, 200);
        assert!((total - 260.0).abs() < 1e-9);
    }

    #[test]
    fn default_total_keyboard_mode_uses_higher_floor() {
        // KEYBOARD: max(300.0, 7.605 * (n+100) / (0.01 * n + 6.5))
        // With 0 notes: 7.605 * 100 / 6.5 = 116.9... < 300 => 300
        let total = calculate_default_total(&Mode::KEYBOARD_24K, 0);
        assert!((total - 300.0).abs() < 1e-9);
    }

    #[test]
    fn default_total_keyboard_uses_n_plus_100() {
        let n = 500.0f64;
        let expected = 300.0f64.max(7.605 * (n + 100.0) / (0.01 * n + 6.5));
        let total = calculate_default_total(&Mode::KEYBOARD_24K, 500);
        assert!((total - expected).abs() < 1e-9);
        // Must exceed SevenKeys with same note count
        let sk_total = calculate_default_total(&Mode::BEAT_7K, 500);
        assert!(total > sk_total);
    }

    // --- validate tests ---

    #[test]
    fn validate_converts_bms_rank_to_bmson() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        // BMS rank 2 is default (judgerank_type is BmsRank)
        BMSPlayerRule::validate(&mut model);
        // After validation, should be BmsonJudgerank
        assert_eq!(model.judgerank_type, JudgeRankType::BmsonJudgerank);
    }

    #[test]
    fn validate_bms_rank_index_maps_to_lr2_judgerank() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        // Default judgerank = 2 (BmsRank), LR2 judgerank table: [25, 50, 75, 100, 75]
        // Index 2 => 75
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.judgerank, 75);
    }

    #[test]
    fn validate_bms_rank_out_of_range_uses_default() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.judgerank = 10; // out of range 0..5
        BMSPlayerRule::validate(&mut model);
        // Should use judgerank[2] = 75 as fallback
        assert_eq!(model.judgerank, 75);
    }

    #[test]
    fn validate_sets_total_type_to_bms() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.total_type, TotalType::Bms);
    }

    #[test]
    fn validate_bmson_judgerank_preserves_positive_value() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.judgerank = 120;
        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        BMSPlayerRule::validate(&mut model);
        // Positive BmsonJudgerank preserved as-is
        assert_eq!(model.judgerank, 120);
    }

    #[test]
    fn validate_bmson_judgerank_zero_becomes_100() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.judgerank = 0;
        model.judgerank_type = JudgeRankType::BmsonJudgerank;
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.judgerank, 100);
    }
}
