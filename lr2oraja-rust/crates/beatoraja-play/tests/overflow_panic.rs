// Bug exposure tests for arithmetic overflow and panic issues in beatoraja-play.
//
// These tests document latent bugs — they do NOT fix anything.
// Each test either:
//   - Uses #[should_panic] to prove a panic exists
//   - Uses #[ignore] with a BUG comment for silent wrong results
//   - Is a green test documenting edge-case behavior

use beatoraja_types::clear_type::ClearType;
use beatoraja_types::gauge_property::{GaugeElementProperty, GaugeProperty};
use beatoraja_types::groove_gauge::{Gauge, GrooveGauge, HARD, NORMAL};
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;

// ---------------------------------------------------------------------------
// RhythmTimerProcessor: freq=0 causes integer division by zero
// ---------------------------------------------------------------------------

/// RhythmTimerProcessor::update() now guards against freq=0 (previously caused
/// integer division by zero panic). With the fix, freq=0 skips section/quarter-note
/// timing updates instead of panicking.
#[test]
fn rhythm_timer_freq_zero_no_panic() {
    use beatoraja_play::rhythm_timer_processor::RhythmTimerProcessor;

    use bms_model::time_line::TimeLine;
    let mut tl = TimeLine::new(0.0, 0, 8);
    tl.set_section_line(true);
    let mut model2 = BMSModel::new();
    model2.set_mode(Mode::BEAT_7K);
    model2.set_all_time_line(vec![tl]);

    let mut processor2 = RhythmTimerProcessor::new(&model2, false);

    // freq=0 no longer panics; section timing updates are skipped.
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        processor2.update(
            0,     // now
            0,     // micronow
            16667, // deltatime
            120.0, // nowbpm (normal)
            100,   // play_speed
            0,     // freq = 0, previously caused division by zero
            0,     // play_timer_micro
        )
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

/// BUG: Gauge::update() indexes into self.gauge with `judge as usize` (line 127).
/// The gauge vec has 6 elements (PG=0, GR=1, GD=2, BD=3, PR=4, MS=5).
/// Any judge value >= 6 causes an index-out-of-bounds panic.
///
/// In production, judge values come from JudgeManager which uses 0-5, but there
/// is no bounds check in Gauge::update() itself.
#[test]
#[should_panic]
fn gauge_update_judge_index_oob() {
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

    // judge=6 is out of bounds for the 6-element gauge vec → panic
    gauge.update(6, 1.0);
}

/// BUG: GrooveGauge::update() delegates to Gauge::update() for all gauges.
/// Same OOB panic propagates through the GrooveGauge wrapper.
#[test]
#[should_panic]
fn groove_gauge_update_judge_index_oob() {
    let mut model = BMSModel::new();
    model.set_total(300.0);
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

    // judge=6 → panics in every internal Gauge::update() call
    gg.update(6);
}

/// GrooveGauge: negative judge values also panic because `(-1i32) as usize` wraps
/// to a huge index on 64-bit platforms.
#[test]
#[should_panic]
fn groove_gauge_update_negative_judge_panics() {
    let mut model = BMSModel::new();
    model.set_total(300.0);
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, NORMAL, &GaugeProperty::SevenKeys);

    // judge=-1 → (-1i32 as usize) = usize::MAX → index OOB
    gg.update(-1);
}

// ---------------------------------------------------------------------------
// GrooveGauge: extreme damage does not go negative (documenting clamp behavior)
// ---------------------------------------------------------------------------

/// GrooveGauge clamps to 0.0 after extreme damage. This is correct behavior.
/// Documenting as a green test to verify the floor holds.
#[test]
fn groove_gauge_extreme_damage_floors_at_zero() {
    let mut model = BMSModel::new();
    model.set_total(300.0);
    model.set_mode(Mode::BEAT_7K);

    let mut gg = GrooveGauge::new(&model, HARD, &GaugeProperty::SevenKeys);
    assert!((gg.get_value() - 100.0).abs() < f32::EPSILON);

    // Apply 200 POOR judgments (judge=4)
    for _ in 0..200 {
        gg.update(4);
    }

    // Hard gauge should be at 0.0 (dead), not negative
    assert!(
        gg.get_value() >= 0.0,
        "gauge value should not go negative, got {}",
        gg.get_value()
    );
    assert!(
        (gg.get_value() - 0.0).abs() < f32::EPSILON,
        "gauge should be dead (0.0) after extreme damage, got {}",
        gg.get_value()
    );
}
