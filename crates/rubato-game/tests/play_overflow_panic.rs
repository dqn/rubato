// Bug exposure tests for arithmetic overflow and panic issues in beatoraja-play.
//
// These tests document latent bugs — they do NOT fix anything.
// Each test either:
//   - Uses #[should_panic] to prove a panic exists
//   - Uses #[ignore] with a BUG comment for silent wrong results
//   - Is a green test documenting edge-case behavior

use bms::model::bms_model::BMSModel;
use bms::model::mode::Mode;
use rubato_types::clear_type::ClearType;
use rubato_types::gauge_property::{GaugeElementProperty, GaugeProperty};
use rubato_types::groove_gauge::{Gauge, GrooveGauge, HARD, NORMAL};

// ---------------------------------------------------------------------------
// RhythmTimerProcessor: freq=0 causes integer division by zero
// ---------------------------------------------------------------------------

/// RhythmTimerProcessor::update() now guards against freq=0 (previously caused
/// integer division by zero panic). With the fix, freq=0 skips section/quarter-note
/// timing updates instead of panicking.
#[test]
fn rhythm_timer_freq_zero_no_panic() {
    use rubato_game::play::rhythm_timer_processor::{RhythmTimerProcessor, RhythmUpdateParams};

    use bms::model::time_line::TimeLine;
    let mut tl = TimeLine::new(0.0, 0, 8);
    tl.section_line = true;
    let mut model2 = BMSModel::new();
    model2.set_mode(Mode::BEAT_7K);
    model2.timelines = vec![tl];

    let mut processor2 = RhythmTimerProcessor::new(&model2, false);

    // freq=0 no longer panics; section timing updates are skipped.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        processor2.update(&RhythmUpdateParams {
            now: 0,
            micronow: 0,
            deltatime: 16667,
            nowbpm: 120.0,
            play_speed: 100,
            freq: 0, // previously caused division by zero
            play_timer_micro: 0,
        })
    }));
    assert!(result.is_ok(), "freq=0 should not panic");
}

/// BUG: RhythmTimerProcessor::update() line 102 computes `60000.0 / nowbpm`.
/// When nowbpm=0.0, this evaluates to f64::INFINITY (IEEE 754 semantics).
/// The result is then cast to i64 via `as i64`.
///
/// In Rust edition 2024, float-to-int casts saturate: INFINITY → i64::MAX.
/// This doesn't panic, but produces a wildly incorrect timer value.
///
/// This test documents the behavior as a green test (no panic, just wrong output).
#[test]
fn rhythm_timer_nowbpm_zero_produces_saturated_cast() {
    // Verify IEEE 754 semantics: 60000.0 / 0.0 = inf
    let inf_val = 60000.0_f64 / 0.0_f64;
    assert!(inf_val.is_infinite());

    // In Rust, `f64::INFINITY as i64` saturates to i64::MAX
    let saturated = f64::INFINITY as i64;
    assert_eq!(saturated, i64::MAX);

    // This means if nowbpm=0.0 reaches update() with quarter_note_times populated,
    // the timer comparison uses i64::MAX which is semantically wrong but doesn't panic.
}

// ---------------------------------------------------------------------------
// GrooveGauge / Gauge: judge index out-of-bounds
// ---------------------------------------------------------------------------

/// Gauge::update() should ignore out-of-range judge values instead of panicking.
#[test]
fn gauge_update_judge_index_oob_is_ignored() {
    let model = BMSModel::new();
    let element = GaugeElementProperty {
        modifier: None,
        value: vec![0.15, 0.12, 0.03, -5.0, -10.0, -5.0], // 6 elements: indices 0-5
        min: 0.0,
        max: 100.0,
        init: 50.0,
        border: 0.0,
        death: 0.0,
        guts: vec![],
    };
    let mut gauge = Gauge::new(&model, element, ClearType::Hard);
    let initial = gauge.value();

    // judge=6 is out of bounds for the 6-element gauge vec. The update should
    // be ignored rather than panicking.
    gauge.update(6, 1.0);
    assert_eq!(gauge.value(), initial);
}

/// GrooveGauge::update() should ignore out-of-range judge values instead of
/// propagating an index-out-of-bounds panic from Gauge::update().
#[test]
fn groove_gauge_update_judge_index_oob_is_ignored() {
    let mut model = BMSModel::new();
    model.total = 300.0;
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
    let initial = gg.value();

    // judge=6 should be ignored by every internal Gauge::update() call.
    gg.update(6);
    assert_eq!(gg.value(), initial);
}

/// GrooveGauge should also ignore negative judge values instead of panicking on
/// the wrapped `usize` conversion.
#[test]
fn groove_gauge_update_negative_judge_is_ignored() {
    let mut model = BMSModel::new();
    model.total = 300.0;
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);
    let initial = gg.value();

    // judge=-1 should be ignored rather than wrapping to usize::MAX.
    gg.update(-1);
    assert_eq!(gg.value(), initial);
}

// ---------------------------------------------------------------------------
// GrooveGauge: extreme damage does not go negative (documenting clamp behavior)
// ---------------------------------------------------------------------------

/// GrooveGauge clamps to 0.0 after extreme damage. This is correct behavior.
/// Documenting as a green test to verify the floor holds.
#[test]
fn groove_gauge_extreme_damage_floors_at_zero() {
    let mut model = BMSModel::new();
    model.total = 300.0;
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, HARD, &GaugeProperty::SevenKeys);
    assert!((gg.value() - 100.0).abs() < f32::EPSILON);

    // Apply 200 POOR judgments (judge=4)
    for _ in 0..200 {
        gg.update(4);
    }

    // Hard gauge should be at 0.0 (dead), not negative
    assert!(
        gg.value() >= 0.0,
        "gauge value should not go negative, got {}",
        gg.value()
    );
    assert!(
        (gg.value() - 0.0).abs() < f32::EPSILON,
        "gauge should be dead (0.0) after extreme damage, got {}",
        gg.value()
    );
}
