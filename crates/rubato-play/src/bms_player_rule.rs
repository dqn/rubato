use crate::gauge_property::GaugeProperty;
use crate::judge_property::{JudgeProperty, JudgePropertyType};
use bms_model::bms_model::{BMSModel, JudgeRankType, TotalType};
use bms_model::mode::Mode;

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
        let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        let rule = Self::for_mode(&mode);
        let judgerank = model.judgerank();
        match model.judgerank_type() {
            JudgeRankType::BmsRank => {
                let new_rank = if (0..5).contains(&judgerank) {
                    rule.judge.windowrule.judgerank[judgerank as usize]
                } else {
                    rule.judge.windowrule.judgerank[2]
                };
                model.set_judgerank(new_rank);
            }
            JudgeRankType::BmsDefexrank => {
                let new_rank = if judgerank > 0 {
                    judgerank * rule.judge.windowrule.judgerank[2] / 100
                } else {
                    rule.judge.windowrule.judgerank[2]
                };
                model.set_judgerank(new_rank);
            }
            JudgeRankType::BmsonJudgerank => {
                let new_rank = if judgerank > 0 { judgerank } else { 100 };
                model.set_judgerank(new_rank);
            }
        }
        model.set_judgerank_type(JudgeRankType::BmsonJudgerank);

        let totalnotes = model.total_notes();
        match model.total_type() {
            TotalType::Bms => {
                // TOTAL undefined case
                if model.total() <= 0.0 {
                    model.set_total(calculate_default_total(&mode, totalnotes));
                }
            }
            TotalType::Bmson => {
                let total = calculate_default_total(&mode, totalnotes);
                let new_total = if model.total() > 0.0 {
                    model.total() / 100.0 * total
                } else {
                    total
                };
                model.set_total(new_total);
            }
        }
        model.set_total_type(TotalType::Bms);
    }
}

fn calculate_default_total(_mode: &Mode, totalnotes: i32) -> f64 {
    160.0 + (totalnotes as f64 + (totalnotes as f64 - 400.0).clamp(0.0, 200.0)) * 0.16
}

/// BMSPlayerRuleSet::LR2
fn bms_player_rule_set_lr2() -> Vec<BMSPlayerRule> {
    vec![BMSPlayerRule::new(
        GaugeProperty::Lr2,
        JudgePropertyType::Lr2,
        vec![],
    )]
}

/// BMSPlayerRuleSet::Beatoraja
#[allow(dead_code)]
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

    #[test]
    fn default_total_with_zero_notes() {
        // 160.0 + (0 + max(0 - 400, 0).min(200)) * 0.16
        // = 160.0 + (0 + 0) * 0.16 = 160.0
        let total = calculate_default_total(&Mode::BEAT_7K, 0);
        assert!((total - 160.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_total_with_400_notes() {
        // 160.0 + (400 + max(400 - 400, 0).min(200)) * 0.16
        // = 160.0 + (400 + 0) * 0.16 = 160.0 + 64.0 = 224.0
        let total = calculate_default_total(&Mode::BEAT_7K, 400);
        assert!((total - 224.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_total_with_500_notes() {
        // 160.0 + (500 + max(500 - 400, 0).min(200)) * 0.16
        // = 160.0 + (500 + 100) * 0.16 = 160.0 + 96.0 = 256.0
        let total = calculate_default_total(&Mode::BEAT_7K, 500);
        assert!((total - 256.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_total_with_700_notes() {
        // 160.0 + (700 + max(700 - 400, 0).min(200)) * 0.16
        // = 160.0 + (700 + min(300, 200)) * 0.16
        // = 160.0 + (700 + 200) * 0.16 = 160.0 + 144.0 = 304.0
        let total = calculate_default_total(&Mode::BEAT_7K, 700);
        assert!((total - 304.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_total_with_1000_notes() {
        // (1000 - 400).max(0).min(200) = 200
        // 160.0 + (1000 + 200) * 0.16 = 160.0 + 192.0 = 352.0
        let total = calculate_default_total(&Mode::BEAT_7K, 1000);
        assert!((total - 352.0).abs() < f64::EPSILON);
    }

    #[test]
    fn default_total_with_200_notes() {
        // (200 - 400).max(0).min(200) = 0
        // 160.0 + (200 + 0) * 0.16 = 160.0 + 32.0 = 192.0
        let total = calculate_default_total(&Mode::BEAT_7K, 200);
        assert!((total - 192.0).abs() < f64::EPSILON);
    }

    // --- validate tests ---

    #[test]
    fn validate_converts_bms_rank_to_bmson() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        // BMS rank 2 is default (judgerank_type is BmsRank)
        BMSPlayerRule::validate(&mut model);
        // After validation, should be BmsonJudgerank
        assert_eq!(model.judgerank_type(), &JudgeRankType::BmsonJudgerank);
    }

    #[test]
    fn validate_bms_rank_index_maps_to_lr2_judgerank() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        // Default judgerank = 2 (BmsRank), LR2 judgerank table: [25, 50, 75, 100, 75]
        // Index 2 => 75
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.judgerank(), 75);
    }

    #[test]
    fn validate_bms_rank_out_of_range_uses_default() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(10); // out of range 0..5
        BMSPlayerRule::validate(&mut model);
        // Should use judgerank[2] = 75 as fallback
        assert_eq!(model.judgerank(), 75);
    }

    #[test]
    fn validate_sets_total_type_to_bms() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.total_type(), &TotalType::Bms);
    }

    #[test]
    fn validate_bmson_judgerank_preserves_positive_value() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(120);
        model.set_judgerank_type(JudgeRankType::BmsonJudgerank);
        BMSPlayerRule::validate(&mut model);
        // Positive BmsonJudgerank preserved as-is
        assert_eq!(model.judgerank(), 120);
    }

    #[test]
    fn validate_bmson_judgerank_zero_becomes_100() {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model.set_judgerank(0);
        model.set_judgerank_type(JudgeRankType::BmsonJudgerank);
        BMSPlayerRule::validate(&mut model);
        assert_eq!(model.judgerank(), 100);
    }
}
