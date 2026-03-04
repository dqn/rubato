//! Type cast overflow audit tests for ScoreDataProperty.
//!
//! These tests document silent truncation vulnerabilities in `as i32` casts
//! within score calculation code. They are RED-ONLY: they assert correct
//! behavior that the current code does NOT satisfy.
//!
//! Source: crates/beatoraja-core/src/score_data_property.rs lines 138-141
//! Code:   `((1000000i64 * judges_total as i64 + ...) / totalnotes as i64) as i32`
//!
//! The `as i32` cast silently truncates when the i64 result exceeds i32::MAX.
//! In the default mode branch, the formula yields up to 1_000_000 per note for
//! PG judgements. When judge counts significantly exceed `totalnotes` (e.g.,
//! corrupt or hand-crafted ScoreData), the quotient can exceed i32::MAX and the
//! cast wraps to a negative value.

use bms_model::mode::Mode;
use rubato_core::score_data::ScoreData;
use rubato_core::score_data_property::ScoreDataProperty;

/// Helper: build a ScoreData with all notes in the PG (PGREAT) bucket.
fn make_all_pg_score(mode: Mode, notes: i32) -> ScoreData {
    let mut sd = ScoreData::new(mode);
    sd.notes = notes;
    // Split evenly between early-PG and late-PG
    sd.epg = notes / 2;
    sd.lpg = notes - sd.epg;
    // combo = notes for a perfect play
    sd.maxcombo = notes;
    sd
}

/// Baseline: 1000 notes, all PG, default mode.
/// Numerator = 1_000_000 * 1000 = 1_000_000_000 (fits i64).
/// Result    = 1_000_000_000 / 1000 = 1_000_000 (fits i32).
/// This should be fine.
#[test]
fn score_rate_1000_notes_no_overflow() {
    let sd = make_all_pg_score(Mode::KEYBOARD_24K, 1000);
    let mut prop = ScoreDataProperty::new();
    prop.update_score(Some(&sd));

    // For all-PG in the default branch: nowpoint = 1_000_000 * notes / notes = 1_000_000
    assert_eq!(
        prop.nowpoint, 1_000_000,
        "1000 notes all-PG should yield nowpoint=1_000_000"
    );
}

/// 3000 notes, all PG, default mode.
/// Numerator = 1_000_000 * 3000 = 3_000_000_000 (exceeds i32::MAX but fits i64).
/// Result    = 3_000_000_000 / 3000 = 1_000_000 (fits i32).
/// Consistent data is safe because division reduces the result.
#[test]
fn score_rate_3000_notes_consistent_no_overflow() {
    let sd = make_all_pg_score(Mode::KEYBOARD_24K, 3000);
    let mut prop = ScoreDataProperty::new();
    prop.update_score(Some(&sd));

    assert_eq!(
        prop.nowpoint, 1_000_000,
        "3000 notes all-PG should yield nowpoint=1_000_000"
    );
}

/// BUG: When judge counts exceed totalnotes (inconsistent/corrupt ScoreData),
/// the i64 quotient can exceed i32::MAX, and `as i32` silently truncates.
///
/// Scenario: notes=1 but epg=1500, lpg=1500 (judge_count_total(0) = 3000).
/// Numerator = 1_000_000 * 3000 = 3_000_000_000.
/// Denominator = 1.
/// Quotient = 3_000_000_000, which exceeds i32::MAX (2_147_483_647).
/// `3_000_000_000i64 as i32` wraps to -1_294_967_296 (silent truncation).
///
/// A correct implementation would either:
/// - Validate that judge counts don't exceed totalnotes, or
/// - Use i64 for nowpoint, or
/// - Use saturating/checked casts.
#[test]
#[ignore] // BUG: `as i32` truncates silently when quotient exceeds i32::MAX
fn score_rate_calculation_overflow() {
    let mut sd = ScoreData::new(Mode::KEYBOARD_24K);
    sd.notes = 1;
    // Intentionally inconsistent: 3000 PG judges but only 1 note
    sd.epg = 1500;
    sd.lpg = 1500;
    sd.maxcombo = 1;

    let mut prop = ScoreDataProperty::new();
    prop.update_score(Some(&sd));

    // The i64 quotient is 3_000_000_000, which exceeds i32::MAX.
    // After `as i32`, this wraps to a negative value.
    let expected_correct: i64 = 3_000_000_000;
    assert!(
        expected_correct > i32::MAX as i64,
        "sanity check: quotient exceeds i32::MAX"
    );

    // The correct value should be 3_000_000_000 (or clamped to i32::MAX).
    // Due to the bug, nowpoint is negative (silent truncation).
    assert!(
        prop.nowpoint > 0,
        "nowpoint should be positive for all-PG score, but got {} due to i32 truncation",
        prop.nowpoint
    );
}
