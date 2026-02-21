// GrooveGauge, Gauge, GaugeModifier moved to beatoraja-types (Phase 15b)
// Only the `create` factory function remains here (depends on BMSPlayerRule).
pub use beatoraja_types::groove_gauge::*;

use crate::bms_player_rule::BMSPlayerRule;
use crate::gauge_property::GaugeProperty;
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

/// Factory function for creating a GrooveGauge with automatic gauge property selection.
/// This depends on BMSPlayerRule which cannot be moved to beatoraja-types.
pub fn create_groove_gauge(
    model: &BMSModel,
    gauge_type: i32,
    grade: i32,
    gauge: Option<GaugeProperty>,
) -> Option<GrooveGauge> {
    let id = if grade > 0 {
        // Course gauge
        if gauge_type <= 2 {
            6
        } else if gauge_type == 3 {
            7
        } else {
            8
        }
    } else {
        gauge_type
    };
    if id >= 0 {
        let gauge = gauge.unwrap_or_else(|| {
            let mode = model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
            BMSPlayerRule::get_bms_player_rule(&mode).gauge
        });
        Some(GrooveGauge::new(model, id, &gauge))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_model() -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(Mode::BEAT_7K);
        model
    }

    // --- create_groove_gauge factory tests ---

    #[test]
    fn create_groove_gauge_normal_type() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None);
        assert!(gg.is_some());
        let gg = gg.unwrap();
        assert_eq!(gg.get_type(), NORMAL);
    }

    #[test]
    fn create_groove_gauge_hard_type() {
        let model = make_model();
        let gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
        assert_eq!(gg.get_type(), HARD);
    }

    #[test]
    fn create_groove_gauge_exhard_type() {
        let model = make_model();
        let gg = create_groove_gauge(&model, EXHARD, 0, None).unwrap();
        assert_eq!(gg.get_type(), EXHARD);
    }

    #[test]
    fn create_groove_gauge_hazard_type() {
        let model = make_model();
        let gg = create_groove_gauge(&model, HAZARD, 0, None).unwrap();
        assert_eq!(gg.get_type(), HAZARD);
    }

    #[test]
    fn create_groove_gauge_course_normal_maps_to_class() {
        let model = make_model();
        // grade > 0 and gauge_type <= 2 => id = 6 (CLASS)
        let gg = create_groove_gauge(&model, NORMAL, 1, None).unwrap();
        assert_eq!(gg.get_type(), CLASS);
    }

    #[test]
    fn create_groove_gauge_course_hard_maps_to_exclass() {
        let model = make_model();
        // grade > 0 and gauge_type == 3 => id = 7 (EXCLASS)
        let gg = create_groove_gauge(&model, HARD, 1, None).unwrap();
        assert_eq!(gg.get_type(), EXCLASS);
    }

    #[test]
    fn create_groove_gauge_course_exhard_maps_to_exhardclass() {
        let model = make_model();
        // grade > 0 and gauge_type > 3 => id = 8 (EXHARDCLASS)
        let gg = create_groove_gauge(&model, EXHARD, 1, None).unwrap();
        assert_eq!(gg.get_type(), EXHARDCLASS);
    }

    // --- Gauge initial value tests ---

    #[test]
    fn normal_gauge_initial_value() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // LR2 Normal gauge init = 20.0
        let value = gg.get_value();
        assert!((value - 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hard_gauge_initial_value() {
        let model = make_model();
        let gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
        // LR2 Hard gauge init = 100.0
        let value = gg.get_value();
        assert!((value - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn exhard_gauge_initial_value() {
        let model = make_model();
        let gg = create_groove_gauge(&model, EXHARD, 0, None).unwrap();
        // LR2 ExHard gauge init = 100.0
        let value = gg.get_value();
        assert!((value - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hazard_gauge_initial_value() {
        let model = make_model();
        let gg = create_groove_gauge(&model, HAZARD, 0, None).unwrap();
        // Hazard gauge init = 100.0
        let value = gg.get_value();
        assert!((value - 100.0).abs() < f32::EPSILON);
    }

    // --- Gauge update tests ---

    #[test]
    fn normal_gauge_increases_on_pgreat() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        let initial = gg.get_value();
        gg.update(0); // PGREAT
        assert!(gg.get_value() > initial);
    }

    #[test]
    fn normal_gauge_decreases_on_poor() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // Set to a higher value first so decrease is visible
        gg.set_value(80.0);
        let before = gg.get_value();
        gg.update(4); // POOR
        assert!(gg.get_value() < before);
    }

    #[test]
    fn hard_gauge_decreases_on_bad() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
        let initial = gg.get_value();
        gg.update(3); // BAD
        assert!(gg.get_value() < initial);
    }

    #[test]
    fn hard_gauge_decreases_on_poor() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
        let initial = gg.get_value();
        gg.update(4); // POOR
        assert!(gg.get_value() < initial);
    }

    #[test]
    fn hazard_gauge_drops_to_zero_on_bad() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, HAZARD, 0, None).unwrap();
        assert!((gg.get_value() - 100.0).abs() < f32::EPSILON);
        gg.update(3); // BAD
        assert!((gg.get_value() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hazard_gauge_drops_to_zero_on_poor() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, HAZARD, 0, None).unwrap();
        gg.update(4); // POOR
        assert!((gg.get_value() - 0.0).abs() < f32::EPSILON);
    }

    #[test]
    fn hazard_gauge_survives_miss() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, HAZARD, 0, None).unwrap();
        let before = gg.get_value();
        gg.update(5); // MISS: LR2 Hazard value[5] = -10.0, not -100.0
        // Hazard gauge decreases on MISS but does not instantly die
        assert!(gg.get_value() < before);
    }

    // --- Gauge clamping tests ---

    #[test]
    fn gauge_value_cannot_exceed_100() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        gg.set_value(200.0);
        assert!(gg.get_value() <= 100.0);
    }

    #[test]
    fn gauge_value_cannot_go_below_min() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // For LR2 Normal gauge, min is 2.0
        gg.set_value(50.0);
        // Repeatedly apply damage
        for _ in 0..100 {
            gg.update(4); // POOR
        }
        // Value should be clamped (either at min or 0 if below death)
        assert!(gg.get_value() >= 0.0);
    }

    // --- GrooveGauge type tests ---

    #[test]
    fn is_type_changed_initially_false() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        assert!(!gg.is_type_changed());
    }

    #[test]
    fn is_type_changed_after_set_type() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        gg.set_type(HARD);
        assert!(gg.is_type_changed());
        assert_eq!(gg.get_type(), HARD);
    }

    #[test]
    fn is_course_gauge() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 1, None).unwrap();
        assert!(gg.is_course_gauge());
    }

    #[test]
    fn is_not_course_gauge() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        assert!(!gg.is_course_gauge());
    }

    #[test]
    fn gauge_type_length() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // LR2 gauge property has 9 types (ASSISTEASY through EXHARDCLASS)
        assert_eq!(gg.get_gauge_type_length(), 9);
    }

    // --- Gauge qualified tests ---

    #[test]
    fn normal_gauge_not_qualified_at_init() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // LR2 Normal init = 20.0, border = 80.0
        assert!(!gg.is_qualified());
    }

    #[test]
    fn normal_gauge_qualified_when_above_border() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        gg.set_value(80.0);
        assert!(gg.is_qualified());
    }

    #[test]
    fn hard_gauge_qualified_when_alive() {
        let model = make_model();
        let gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
        // Hard gauge: border = 0.0, init = 100.0 => qualified as long as alive
        assert!(gg.is_qualified());
    }

    // --- add_value tests ---

    #[test]
    fn add_value_increases_gauge() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        let before = gg.get_value();
        gg.add_value(10.0);
        assert!((gg.get_value() - (before + 10.0)).abs() < f32::EPSILON);
    }

    // --- get/set value by type tests ---

    #[test]
    fn get_value_by_type() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        // NORMAL type = 2
        let normal_val = gg.get_value_by_type(NORMAL);
        assert!((normal_val - gg.get_value()).abs() < f32::EPSILON);
    }

    #[test]
    fn set_value_by_type() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        gg.set_value_by_type(HARD, 50.0);
        assert!((gg.get_value_by_type(HARD) - 50.0).abs() < f32::EPSILON);
    }

    // --- get_gauge tests ---

    #[test]
    fn get_gauge_returns_current_type_gauge() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        let gauge = gg.get_gauge();
        assert!((gauge.get_value() - gg.get_value()).abs() < f32::EPSILON);
    }

    #[test]
    fn get_gauge_by_type_returns_specific_gauge() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        let hard_gauge = gg.get_gauge_by_type(HARD);
        // Hard gauge init = 100.0
        assert!((hard_gauge.get_value() - 100.0).abs() < f32::EPSILON);
    }

    // --- Gauge is_max tests ---

    #[test]
    fn gauge_is_max_when_at_max() {
        let model = make_model();
        let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        gg.set_value(100.0);
        assert!(gg.get_gauge().is_max());
    }

    #[test]
    fn gauge_is_not_max_below_max() {
        let model = make_model();
        let gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
        assert!(!gg.get_gauge().is_max());
    }

    // --- update_with_rate tests ---

    #[test]
    fn update_with_rate_scales_gauge_change() {
        let model = make_model();
        // Use HARD gauge (index 3) which has non-zero PGREAT damage values
        // that don't depend on TOTAL modifier (uses LimitIncrement)
        let mut gg1 = create_groove_gauge(&model, HARD, 0, None).unwrap();
        let mut gg2 = create_groove_gauge(&model, HARD, 0, None).unwrap();
        // Apply BAD (judge=3) which always has negative values
        gg1.update(3); // BAD with rate 1.0
        gg2.update_with_rate(3, 0.5); // BAD with rate 0.5
        // Rate 0.5 should decrease less (higher value remaining)
        assert!(gg2.get_value() > gg1.get_value());
    }
}
