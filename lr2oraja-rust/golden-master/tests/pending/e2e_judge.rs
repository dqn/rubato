// E2E integration tests: BMS parse -> JudgeManager -> GrooveGauge -> ScoreData
//
// Validates that the full pipeline works correctly with real BMS files.
// Uses invariant-based assertions rather than golden-master comparison.
//
// Notes:
// - JudgeManager.prev_time starts at 0, so notes at time_us=0 are skipped on
//   the first frame. We prime the JudgeManager with update(-1) to work around this.
// - LN notes are split into start+end pairs via build_judge_notes() for JudgeManager.

use beatoraja_types::groove_gauge::{ASSISTEASY, EASY, EXHARD, HARD, HAZARD, NORMAL};
use bms_model::judge_note::{JUDGE_BD, JUDGE_GD, JUDGE_GR, JUDGE_MS, JUDGE_PG, JUDGE_PR};
use bms_model::mode::Mode;
use golden_master::e2e_helpers::*;

// ============================================================================
// Group A: Autoplay tests -- perfect play invariants (normal notes only)
// ============================================================================

#[test]
fn autoplay_minimal_7k() {
    let model = load_bms("minimal_7k.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "minimal_7k should have playable notes");
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_minimal_7k");
}

#[test]
fn autoplay_5key() {
    let model = load_bms("5key.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_5key");
}

#[test]
fn autoplay_14key_dp() {
    let model = load_bms("14key_dp.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_14key_dp");
}

#[test]
fn autoplay_9key_pms() {
    let model = load_bms("9key_pms.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_9key_pms");
}

#[test]
fn autoplay_bpm_change() {
    let model = load_bms("bpm_change.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0);
    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_bpm_change");
}

#[test]
fn autoplay_mine_no_damage() {
    let model = load_bms("mine_notes.bms");
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "mine_notes should have playable notes");

    let result = run_autoplay_simulation(&model, NORMAL);
    assert_all_pgreat(&result, total, "autoplay_mine_no_damage");

    assert!(
        result.gauge_value > 0.0,
        "Gauge should be alive (no mine damage in autoplay)"
    );
}

// LN autoplay tests: build_judge_notes() splits LN into start+end pairs with
// pair_index, so autoplay correctly tracks LN start->end as all PGREAT.
// Pure LN end notes are not independently judged (1 judgment per LN pair),
// so we use ghost.len() as the expected total rather than raw playable count.

#[test]
fn autoplay_longnote() {
    let model = load_bms("longnote_types.bms");
    let long_notes = model
        .build_judge_notes()
        .iter()
        .filter(|n| n.is_long_start())
        .count();
    assert!(long_notes > 0, "longnote_types should have LN notes");

    let result = run_autoplay_simulation(&model, NORMAL);
    // ghost.len() reflects actually-judged notes (excludes pure LN end)
    let total = result.ghost.len();
    assert!(total > 0);
    assert_all_pgreat(&result, total, "autoplay_longnote");
}

#[test]
fn autoplay_scratch_bss() {
    let model = load_bms("scratch_bss.bms");

    let result = run_autoplay_simulation(&model, NORMAL);
    // ghost.len() reflects actually-judged notes
    let total = result.ghost.len();
    assert!(total > 0);
    assert_all_pgreat(&result, total, "autoplay_scratch_bss");
}

// ============================================================================
// Group B: Manual input tests -- timing offset affects judgment
// ============================================================================

#[test]
fn manual_perfect() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);
    // Create press events at exact note times (0 offset)
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 0);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG),
        normal as i32,
        "All normal notes should be PG with exact timing"
    );
}

#[test]
fn manual_great() {
    let model = load_bms("minimal_7k.bms");
    // minimal_7k.bms has #RANK 2 -> resolved judgerank=75 (LR2). Scaled windows:
    //   PG +/-18ms, GR +/-40ms, GD +/-100ms, BD +/-200ms
    // Offset by 25ms -- within GR window (+/-40ms) but outside PG window (+/-18ms)
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 25_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_BD),
        0,
        "No BAD expected at 25ms offset"
    );
    assert_eq!(
        score.get_judge_count_total(JUDGE_MS),
        0,
        "No MISS expected at 25ms offset"
    );
    assert!(
        score.get_judge_count_total(JUDGE_GR) > 0,
        "Expected some GR at 25ms offset (PG={}, GR={}, GD={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD)
    );
}

#[test]
fn manual_good() {
    let model = load_bms("minimal_7k.bms");
    // minimal_7k.bms has #RANK 2 -> resolved judgerank=75 (LR2). Scaled windows:
    //   PG +/-18ms, GR +/-40ms, GD +/-100ms, BD +/-200ms
    // Offset by 50ms -- within GD window (+/-100ms) but outside GR window (+/-40ms)
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 50_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_MS),
        0,
        "No MISS expected at 50ms offset"
    );
    assert!(
        score.get_judge_count_total(JUDGE_GD) > 0,
        "Expected some GD at 50ms offset (PG={}, GR={}, GD={}, BD={}, PR={}, MS={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
    );
}

#[test]
fn manual_bad() {
    let model = load_bms("minimal_7k.bms");
    // minimal_7k.bms has #RANK 2 -> resolved judgerank=75 (LR2). Scaled windows:
    //   PG +/-18ms, GR +/-40ms, GD +/-100ms, BD +/-200ms
    // Offset by 150ms -- within BD window (+/-200ms) but outside GD window (+/-100ms)
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 150_000);
    let result = run_manual_simulation(&model, &log, NORMAL);

    let score = &result.score;
    assert!(
        score.get_judge_count_total(JUDGE_BD) > 0 || score.get_judge_count_total(JUDGE_PR) > 0,
        "Expected some BD/PR at 150ms offset (PG={}, GR={}, GD={}, BD={}, PR={}, MS={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
    );
}

#[test]
fn manual_all_miss() {
    let model = load_bms("minimal_7k.bms");
    let total = model.get_total_notes() as usize;
    // No input at all -- all notes should be MISS
    let result = run_manual_simulation(&model, &[], NORMAL);

    let score = &result.score;
    let miss_count = score.get_judge_count_total(JUDGE_PR) + score.get_judge_count_total(JUDGE_MS);
    assert_eq!(
        miss_count,
        total as i32,
        "All notes should be MISS/PR with no input (PG={}, GR={}, GD={}, BD={}, PR={}, MS={})",
        score.get_judge_count_total(JUDGE_PG),
        score.get_judge_count_total(JUDGE_GR),
        score.get_judge_count_total(JUDGE_GD),
        score.get_judge_count_total(JUDGE_BD),
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
    );
    assert_eq!(result.max_combo, 0, "Max combo should be 0 with no input");
}

// ============================================================================
// Group C: Gauge integration tests
// ============================================================================

#[test]
fn gauge_normal_autoplay() {
    let model = load_bms("minimal_7k.bms");
    let result = run_autoplay_simulation(&model, NORMAL);
    assert!(
        result.gauge_qualified,
        "Normal gauge should be qualified on autoplay (value={})",
        result.gauge_value
    );
}

#[test]
fn gauge_hard_autoplay() {
    let model = load_bms("minimal_7k.bms");
    let result = run_autoplay_simulation(&model, HARD);
    // Hard gauge starts at 100 and should not decrease with all PG
    assert!(
        (result.gauge_value - 100.0).abs() < 1e-3,
        "Hard gauge should stay at 100.0 on autoplay, got {}",
        result.gauge_value
    );
    assert!(result.gauge_qualified);
}

#[test]
fn gauge_exhard_all_miss() {
    let model = load_bms("minimal_7k.bms");
    let result = run_manual_simulation(&model, &[], EXHARD);
    assert!(
        result.gauge_value < 1e-6,
        "ExHard gauge should be dead (0.0) with all misses, got {}",
        result.gauge_value
    );
    assert!(!result.gauge_qualified);
}

#[test]
fn gauge_all_types_autoplay() {
    let model = load_bms("minimal_7k.bms");
    for gauge_type in [ASSISTEASY, EASY, NORMAL, HARD, EXHARD, HAZARD] {
        let result = run_autoplay_simulation(&model, gauge_type);
        assert!(
            result.gauge_qualified,
            "{gauge_type} gauge should be qualified on autoplay (value={})",
            result.gauge_value
        );
    }
}

// ============================================================================
// Group D: LN special tests
// ============================================================================

#[test]
fn ln_autoplay_judge_count() {
    let model = load_bms("longnote_types.bms");

    let result = run_autoplay_simulation(&model, NORMAL);

    // Pure LN: 1 judgment per LN pair (end not independently judged).
    // ghost.len() matches the number of actually-judged notes.
    let expected = result.ghost.len() as i32;
    let total_judge: i32 = (0..6).map(|j| result.score.get_judge_count_total(j)).sum();
    assert_eq!(
        total_judge, expected,
        "Judge count should match ghost length (expected={expected}, got={})",
        total_judge
    );
}

#[test]
fn scratch_autoplay_judge_count() {
    let model = load_bms("scratch_bss.bms");

    let result = run_autoplay_simulation(&model, NORMAL);

    // BSS (CN type): start + end independently judged.
    // ghost.len() matches the number of actually-judged notes.
    let expected = result.ghost.len() as i32;
    let total_judge: i32 = (0..6).map(|j| result.score.get_judge_count_total(j)).sum();
    assert_eq!(
        total_judge, expected,
        "Judge count should match ghost length (expected={expected}, got={})",
        total_judge
    );
}

// ============================================================================
// Group E: Cross-mode invariants
// ============================================================================

#[test]
fn cross_mode_invariants() {
    // Test all BMS files that contain only normal notes (no LN) across different modes.
    // Note: 9key_pms.bms is parsed as Beat7K because PMS detection requires .pms extension.
    let test_files = ["5key.bms", "minimal_7k.bms", "14key_dp.bms", "9key_pms.bms"];

    for filename in test_files {
        let model = load_bms(filename);
        let total = model.get_total_notes() as usize;
        assert!(total > 0, "{filename} should have playable notes");

        let result = run_autoplay_simulation(&model, NORMAL);
        assert_all_pgreat(&result, total, &format!("cross_mode_{filename}"));
    }
}
