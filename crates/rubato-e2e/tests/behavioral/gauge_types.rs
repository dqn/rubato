//! Phase 5f: Gauge types E2E tests.
//!
//! Tests groove gauge creation and initial state for different gauge types.
//! Uses `create_groove_gauge` factory (rubato-play) which selects gauge properties
//! based on BMSPlayerRule for the model's mode. For BEAT_7K this uses LR2 gauge tables.

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use rubato_game::play::groove_gauge::{
    ASSISTEASY, EASY, EXHARD, GrooveGauge, HARD, HAZARD, NORMAL, create_groove_gauge,
};
use rubato_types::gauge_property::GaugeProperty;

/// Create a minimal BEAT_7K model (used by most tests).
fn make_7k_model() -> BMSModel {
    let mut model = BMSModel::new();
    model.set_mode(Mode::BEAT_7K);
    model
}

/// Helper: create a gauge via the factory with default (LR2) rule for 7K.
fn gauge_for_type(gauge_type: i32) -> GrooveGauge {
    let model = make_7k_model();
    create_groove_gauge(&model, gauge_type, 0, None).expect("gauge creation should succeed")
}

// ---------------------------------------------------------------------------
// Initial value tests
// ---------------------------------------------------------------------------

#[test]
fn normal_gauge_starts_at_20_percent() {
    let gg = gauge_for_type(NORMAL);
    assert!(
        (gg.value() - 20.0).abs() < f32::EPSILON,
        "LR2 NORMAL gauge should start at 20.0, got {}",
        gg.value()
    );
}

#[test]
fn hard_gauge_starts_at_100_percent() {
    let gg = gauge_for_type(HARD);
    assert!(
        (gg.value() - 100.0).abs() < f32::EPSILON,
        "LR2 HARD gauge should start at 100.0, got {}",
        gg.value()
    );
}

#[test]
fn easy_gauge_starts_at_20_percent() {
    let gg = gauge_for_type(EASY);
    assert!(
        (gg.value() - 20.0).abs() < f32::EPSILON,
        "LR2 EASY gauge should start at 20.0, got {}",
        gg.value()
    );
}

#[test]
fn assist_easy_gauge_starts_at_20_percent() {
    let gg = gauge_for_type(ASSISTEASY);
    assert!(
        (gg.value() - 20.0).abs() < f32::EPSILON,
        "LR2 ASSISTEASY gauge should start at 20.0, got {}",
        gg.value()
    );
}

#[test]
fn exhard_gauge_starts_at_100_percent() {
    let gg = gauge_for_type(EXHARD);
    assert!(
        (gg.value() - 100.0).abs() < f32::EPSILON,
        "LR2 EXHARD gauge should start at 100.0, got {}",
        gg.value()
    );
}

#[test]
fn hazard_gauge_starts_at_100_percent() {
    let gg = gauge_for_type(HAZARD);
    assert!(
        (gg.value() - 100.0).abs() < f32::EPSILON,
        "LR2 HAZARD gauge should start at 100.0, got {}",
        gg.value()
    );
}

// ---------------------------------------------------------------------------
// Border value tests
// ---------------------------------------------------------------------------

#[test]
fn normal_gauge_has_80_percent_border() {
    let gg = gauge_for_type(NORMAL);
    // LR2 NORMAL border = 80.0: gauge is not qualified at init (20.0)
    assert!(!gg.is_qualified(), "NORMAL at init should not be qualified");

    let mut gg = gg;
    gg.set_value(80.0);
    assert!(
        gg.is_qualified(),
        "NORMAL at 80.0 should be qualified (border=80.0)"
    );
}

#[test]
fn easy_gauge_has_80_percent_border() {
    let gg = gauge_for_type(EASY);
    assert!(
        !gg.is_qualified(),
        "EASY at init (20.0) should not be qualified"
    );

    let mut gg = gg;
    gg.set_value(79.9);
    assert!(
        !gg.is_qualified(),
        "EASY at 79.9 should not be qualified (border=80.0)"
    );

    gg.set_value(80.0);
    assert!(
        gg.is_qualified(),
        "EASY at 80.0 should be qualified (border=80.0)"
    );
}

#[test]
fn hard_gauge_has_zero_border() {
    let gg = gauge_for_type(HARD);
    // HARD border = 0.0: qualified as long as alive (value > 0)
    assert!(
        gg.is_qualified(),
        "HARD at 100.0 should be qualified (border=0.0)"
    );
}

#[test]
fn assist_easy_gauge_has_60_percent_border() {
    let gg = gauge_for_type(ASSISTEASY);
    assert!(
        !gg.is_qualified(),
        "ASSISTEASY at init (20.0) should not be qualified"
    );

    let mut gg = gg;
    gg.set_value(60.0);
    assert!(
        gg.is_qualified(),
        "ASSISTEASY at 60.0 should be qualified (border=60.0)"
    );
}

// ---------------------------------------------------------------------------
// Clamping tests
// ---------------------------------------------------------------------------

#[test]
fn gauge_value_clamps_to_max() {
    let mut gg = gauge_for_type(NORMAL);
    gg.set_value(200.0);
    assert!(
        gg.value() <= 100.0,
        "gauge should clamp to max (100.0), got {}",
        gg.value()
    );
}

#[test]
fn gauge_value_clamps_to_min() {
    let mut gg = gauge_for_type(NORMAL);
    // LR2 NORMAL min = 2.0, so setting below 2.0 clamps to 2.0
    gg.set_value(1.0);
    assert!(
        (gg.value() - 2.0).abs() < f32::EPSILON,
        "LR2 NORMAL gauge should clamp to min (2.0), got {}",
        gg.value()
    );
}

// ---------------------------------------------------------------------------
// Gauge type metadata tests
// ---------------------------------------------------------------------------

#[test]
fn all_gauge_types_produce_9_sub_gauges() {
    // GrooveGauge always carries all 9 gauge variants internally
    let gg = gauge_for_type(NORMAL);
    assert_eq!(
        gg.gauge_type_length(),
        9,
        "LR2 gauge should have 9 sub-gauge types"
    );
}

#[test]
fn each_gauge_type_reports_correct_type() {
    for &(gt, name) in &[
        (ASSISTEASY, "ASSISTEASY"),
        (EASY, "EASY"),
        (NORMAL, "NORMAL"),
        (HARD, "HARD"),
        (EXHARD, "EXHARD"),
        (HAZARD, "HAZARD"),
    ] {
        let gg = gauge_for_type(gt);
        assert_eq!(gg.gauge_type(), gt, "{} gauge_type mismatch", name);
    }
}

// ---------------------------------------------------------------------------
// Cross-gauge-property tests (SevenKeys vs LR2 differences)
// ---------------------------------------------------------------------------

#[test]
fn sevenkeys_normal_border_matches_lr2() {
    let model = make_7k_model();
    let lr2 = GrooveGauge::new(&model, NORMAL, &GaugeProperty::Lr2);
    let sk = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

    // Both LR2 and SevenKeys NORMAL have border = 80.0
    assert!(
        !lr2.is_qualified(),
        "LR2 NORMAL at init should not be qualified"
    );
    assert!(
        !sk.is_qualified(),
        "SevenKeys NORMAL at init should not be qualified"
    );
}

#[test]
fn sevenkeys_hard_has_no_death_border() {
    let model = make_7k_model();
    let sk = GrooveGauge::new(&model, HARD, &GaugeProperty::SevenKeys);

    // SevenKeys HARD has death=0.0 (no death), unlike LR2 HARD which has death=2.0
    let gauge = sk.gauge();
    assert!(
        (gauge.property().death - 0.0).abs() < f32::EPSILON,
        "SevenKeys HARD should have death=0.0"
    );
}

#[test]
fn lr2_hard_has_death_border_at_2() {
    let model = make_7k_model();
    let lr2 = GrooveGauge::new(&model, HARD, &GaugeProperty::Lr2);

    let gauge = lr2.gauge();
    assert!(
        (gauge.property().death - 2.0).abs() < f32::EPSILON,
        "LR2 HARD should have death=2.0, got {}",
        gauge.property().death
    );
}
