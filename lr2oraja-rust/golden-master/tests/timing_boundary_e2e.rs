// Timing boundary E2E tests: validates judge window precision.
//
// Tests that judge windows produce correct results at exact boundary offsets.
// Uses minimal_7k.bms (#RANK 2 -> judgerank=75, LR2 scaling):
//   PG: ±18ms, GR: ±40ms, GD: ±100ms, BD: ±200ms
//
// All offsets are in microseconds (1ms = 1000us).

use beatoraja_types::groove_gauge::NORMAL;
use bms_model::judge_note::{JUDGE_BD, JUDGE_GD, JUDGE_GR, JUDGE_MS, JUDGE_PG, JUDGE_PR};
use bms_model::mode::Mode;
use golden_master::e2e_helpers::*;

// ============================================================================
// PG/GR boundary tests (±18ms)
// ============================================================================

/// At exactly 17ms offset, all notes should be PGREAT.
#[test]
fn boundary_17ms_all_pgreat() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 17_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG),
        normal as i32,
        "17ms should be within PG window (PG={}, GR={}, GD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
    );
}

/// At exactly 19ms offset, no notes should be PGREAT (outside ±18ms PG window).
#[test]
fn boundary_19ms_no_pgreat() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 19_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG),
        0,
        "19ms should be outside PG window (PG={}, GR={}, GD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
    );
    assert!(
        score.get_judge_count_total(JUDGE_GR) > 0,
        "19ms should be within GR window"
    );
}

// ============================================================================
// GR/GD boundary tests (±40ms)
// ============================================================================

/// At exactly 39ms offset, all notes should be GREAT or better.
#[test]
fn boundary_39ms_all_great() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 39_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    let pg_gr = score.get_judge_count_total(JUDGE_PG) + score.get_judge_count_total(JUDGE_GR);
    assert_eq!(
        pg_gr,
        normal as i32,
        "39ms should be within GR window (PG={}, GR={}, GD={}, BD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
    );
}

/// At exactly 41ms offset, no notes should be GR or better.
#[test]
fn boundary_41ms_no_great() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 41_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG) + score.get_judge_count_total(JUDGE_GR),
        0,
        "41ms should be outside GR window (PG={}, GR={}, GD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
    );
    assert!(
        score.get_judge_count_total(JUDGE_GD) > 0,
        "41ms should be within GD window"
    );
}

// ============================================================================
// GD/BD boundary tests (±100ms)
// ============================================================================

/// At exactly 99ms offset, all notes should be GOOD or better.
#[test]
fn boundary_99ms_all_good() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 99_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    let pg_gr_gd = score.get_judge_count_total(JUDGE_PG)
        + score.get_judge_count_total(JUDGE_GR)
        + score.get_judge_count_total(JUDGE_GD);
    assert_eq!(
        pg_gr_gd,
        normal as i32,
        "99ms should be within GD window (PG={}, GR={}, GD={}, BD={}, PR={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
    );
}

/// At exactly 101ms offset, no notes should be GD or better.
#[test]
fn boundary_101ms_no_good() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 101_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG)
            + score.get_judge_count_total(JUDGE_GR)
            + score.get_judge_count_total(JUDGE_GD),
        0,
        "101ms should be outside GD window (PG={}, GR={}, GD={}, BD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
    );
    assert!(
        score.get_judge_count_total(JUDGE_BD) > 0 || score.get_judge_count_total(JUDGE_PR) > 0,
        "101ms should be in BD/PR window"
    );
}

// ============================================================================
// BD/MISS boundary tests (±200ms)
// ============================================================================

/// At exactly 199ms offset, notes should not be MISS.
#[test]
fn boundary_199ms_not_miss() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 199_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_MS),
        0,
        "199ms should not be MISS (BD={}, PR={}, MS={})",
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
    );
}

/// At exactly 201ms offset, all notes should be MISS (outside BD window).
#[test]
fn boundary_201ms_all_miss() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 201_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    let total: i32 = (0..6).map(|j| score.get_judge_count_total(j)).sum();
    let miss_count = score.get_judge_count_total(JUDGE_PR) + score.get_judge_count_total(JUDGE_MS);
    // At 201ms, key presses are beyond BD window, so notes pass without being hit
    assert_eq!(
        miss_count,
        total,
        "201ms should result in all MISS/PR (PG={}, GR={}, GD={}, BD={}, PR={}, MS={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
    );
}

// ============================================================================
// Early timing symmetry tests (negative offsets)
// ============================================================================

/// Early and late offsets of the same magnitude should produce identical judgements.
#[test]
fn early_late_symmetry_25ms() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);

    let late_log = create_note_press_log(&jn, mode, 25_000);
    let early_log = create_note_press_log(&jn, mode, -25_000);

    let late_result = run_manual_simulation(&model, &late_log, NORMAL);
    let early_result = run_manual_simulation(&model, &early_log, NORMAL);

    let late_score = &late_result.score;
    let early_score = &early_result.score;

    // Both should have same PG/GR distribution (within GR window)
    assert_eq!(
        early_score.get_judge_count_total(JUDGE_PG),
        late_score.get_judge_count_total(JUDGE_PG),
        "PG count should be symmetric: early={}, late={}",
        early_score.get_judge_count_total(JUDGE_PG),
        late_score.get_judge_count_total(JUDGE_PG),
    );
    assert_eq!(
        early_score.get_judge_count_total(JUDGE_GR),
        late_score.get_judge_count_total(JUDGE_GR),
        "GR count should be symmetric: early={}, late={}",
        early_score.get_judge_count_total(JUDGE_GR),
        late_score.get_judge_count_total(JUDGE_GR),
    );
}

/// Early and late offsets of the same magnitude should produce identical judgements.
#[test]
fn early_late_symmetry_50ms() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);

    let late_log = create_note_press_log(&jn, mode, 50_000);
    let early_log = create_note_press_log(&jn, mode, -50_000);

    let late_result = run_manual_simulation(&model, &late_log, NORMAL);
    let early_result = run_manual_simulation(&model, &early_log, NORMAL);

    let late_score = &late_result.score;
    let early_score = &early_result.score;

    // Both should have same GD distribution (within GD window)
    assert_eq!(
        early_score.get_judge_count_total(JUDGE_GD),
        late_score.get_judge_count_total(JUDGE_GD),
        "GD count should be symmetric: early={}, late={}",
        early_score.get_judge_count_total(JUDGE_GD),
        late_score.get_judge_count_total(JUDGE_GD),
    );
}
