// Exhaustive E2E tests: 4 PlayModes x 6 GaugeTypes x 3 InputModes = 72 tests
//
// Validates invariants across the full combination matrix:
// - Autoplay: all PG, gauge qualified, max_combo >= total_notes
// - ManualPerfect: all PG, gauge qualified (normal notes only, 0ms offset)
// - ManualAllMiss: all PR/MS, max_combo=0, gauge not qualified

use beatoraja_types::groove_gauge::{ASSISTEASY, EASY, EXHARD, HARD, HAZARD, NORMAL};
use bms_model::judge_note::{JUDGE_MS, JUDGE_PG, JUDGE_PR};
use bms_model::mode::Mode;
use golden_master::e2e_helpers::*;

fn run_autoplay_test(bms_file: &str, gauge_type: i32, label: &str) {
    let model = load_bms(bms_file);
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "{label}: should have playable notes");

    let result = run_autoplay_simulation(&model, gauge_type);
    assert_all_pgreat(&result, total, label);
}

fn run_manual_perfect_test(bms_file: &str, gauge_type: i32, label: &str) {
    let model = load_bms(bms_file);
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);
    assert!(normal > 0, "{label}: should have normal notes");

    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    let log = create_note_press_log(&jn, mode, 0);
    let result = run_manual_simulation(&model, &log, gauge_type);

    let score = &result.score;
    assert_eq!(
        score.get_judge_count_total(JUDGE_PG),
        normal as i32,
        "{label}: all normal notes should be PG (PG={}, total_judge={})",
        score.get_judge_count_total(JUDGE_PG),
        (0..6).map(|j| score.get_judge_count_total(j)).sum::<i32>()
    );
    assert!(
        result.gauge_qualified,
        "{label}: gauge should be qualified (value={})",
        result.gauge_value
    );
    assert!(
        result.max_combo >= normal as i32,
        "{label}: max_combo {} < normal_notes {}",
        result.max_combo,
        normal
    );
}

fn run_manual_all_miss_test(bms_file: &str, gauge_type: i32, label: &str) {
    let model = load_bms(bms_file);
    let total = model.get_total_notes() as usize;
    assert!(total > 0, "{label}: should have playable notes");

    let result = run_manual_simulation(&model, &[], gauge_type);

    let score = &result.score;
    let miss_count = score.get_judge_count_total(JUDGE_PR) + score.get_judge_count_total(JUDGE_MS);
    assert_eq!(
        miss_count,
        total as i32,
        "{label}: all notes should be PR/MS (PR={}, MS={}, total={})",
        score.get_judge_count_total(JUDGE_PR),
        score.get_judge_count_total(JUDGE_MS),
        total
    );
    assert_eq!(result.max_combo, 0, "{label}: max_combo should be 0");
    assert!(
        !result.gauge_qualified,
        "{label}: gauge should NOT be qualified (value={})",
        result.gauge_value
    );
}

// ============================================================================
// Beat5K (5key.bms): 6 gauges x 3 inputs = 18 tests
// ============================================================================

#[test]
fn beat5k_assist_easy_autoplay() {
    run_autoplay_test("5key.bms", ASSISTEASY, "beat5k_assist_easy_autoplay");
}
#[test]
fn beat5k_easy_autoplay() {
    run_autoplay_test("5key.bms", EASY, "beat5k_easy_autoplay");
}
#[test]
fn beat5k_normal_autoplay() {
    run_autoplay_test("5key.bms", NORMAL, "beat5k_normal_autoplay");
}
#[test]
fn beat5k_hard_autoplay() {
    run_autoplay_test("5key.bms", HARD, "beat5k_hard_autoplay");
}
#[test]
fn beat5k_exhard_autoplay() {
    run_autoplay_test("5key.bms", EXHARD, "beat5k_exhard_autoplay");
}
#[test]
fn beat5k_hazard_autoplay() {
    run_autoplay_test("5key.bms", HAZARD, "beat5k_hazard_autoplay");
}

#[test]
fn beat5k_assist_easy_manual_perfect() {
    run_manual_perfect_test("5key.bms", ASSISTEASY, "beat5k_assist_easy_manual_perfect");
}
#[test]
fn beat5k_easy_manual_perfect() {
    run_manual_perfect_test("5key.bms", EASY, "beat5k_easy_manual_perfect");
}
#[test]
fn beat5k_normal_manual_perfect() {
    run_manual_perfect_test("5key.bms", NORMAL, "beat5k_normal_manual_perfect");
}
#[test]
fn beat5k_hard_manual_perfect() {
    run_manual_perfect_test("5key.bms", HARD, "beat5k_hard_manual_perfect");
}
#[test]
fn beat5k_exhard_manual_perfect() {
    run_manual_perfect_test("5key.bms", EXHARD, "beat5k_exhard_manual_perfect");
}
#[test]
fn beat5k_hazard_manual_perfect() {
    run_manual_perfect_test("5key.bms", HAZARD, "beat5k_hazard_manual_perfect");
}

#[test]
fn beat5k_assist_easy_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", ASSISTEASY, "beat5k_assist_easy_manual_all_miss");
}
#[test]
fn beat5k_easy_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", EASY, "beat5k_easy_manual_all_miss");
}
#[test]
fn beat5k_normal_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", NORMAL, "beat5k_normal_manual_all_miss");
}
#[test]
fn beat5k_hard_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", HARD, "beat5k_hard_manual_all_miss");
}
#[test]
fn beat5k_exhard_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", EXHARD, "beat5k_exhard_manual_all_miss");
}
#[test]
fn beat5k_hazard_manual_all_miss() {
    run_manual_all_miss_test("5key.bms", HAZARD, "beat5k_hazard_manual_all_miss");
}

// ============================================================================
// Beat7K (minimal_7k.bms): 6 gauges x 3 inputs = 18 tests
// ============================================================================

#[test]
fn beat7k_assist_easy_autoplay() {
    run_autoplay_test("minimal_7k.bms", ASSISTEASY, "beat7k_assist_easy_autoplay");
}
#[test]
fn beat7k_easy_autoplay() {
    run_autoplay_test("minimal_7k.bms", EASY, "beat7k_easy_autoplay");
}
#[test]
fn beat7k_normal_autoplay() {
    run_autoplay_test("minimal_7k.bms", NORMAL, "beat7k_normal_autoplay");
}
#[test]
fn beat7k_hard_autoplay() {
    run_autoplay_test("minimal_7k.bms", HARD, "beat7k_hard_autoplay");
}
#[test]
fn beat7k_exhard_autoplay() {
    run_autoplay_test("minimal_7k.bms", EXHARD, "beat7k_exhard_autoplay");
}
#[test]
fn beat7k_hazard_autoplay() {
    run_autoplay_test("minimal_7k.bms", HAZARD, "beat7k_hazard_autoplay");
}

#[test]
fn beat7k_assist_easy_manual_perfect() {
    run_manual_perfect_test(
        "minimal_7k.bms",
        ASSISTEASY,
        "beat7k_assist_easy_manual_perfect",
    );
}
#[test]
fn beat7k_easy_manual_perfect() {
    run_manual_perfect_test("minimal_7k.bms", EASY, "beat7k_easy_manual_perfect");
}
#[test]
fn beat7k_normal_manual_perfect() {
    run_manual_perfect_test("minimal_7k.bms", NORMAL, "beat7k_normal_manual_perfect");
}
#[test]
fn beat7k_hard_manual_perfect() {
    run_manual_perfect_test("minimal_7k.bms", HARD, "beat7k_hard_manual_perfect");
}
#[test]
fn beat7k_exhard_manual_perfect() {
    run_manual_perfect_test("minimal_7k.bms", EXHARD, "beat7k_exhard_manual_perfect");
}
#[test]
fn beat7k_hazard_manual_perfect() {
    run_manual_perfect_test("minimal_7k.bms", HAZARD, "beat7k_hazard_manual_perfect");
}

#[test]
fn beat7k_assist_easy_manual_all_miss() {
    run_manual_all_miss_test(
        "minimal_7k.bms",
        ASSISTEASY,
        "beat7k_assist_easy_manual_all_miss",
    );
}
#[test]
fn beat7k_easy_manual_all_miss() {
    run_manual_all_miss_test("minimal_7k.bms", EASY, "beat7k_easy_manual_all_miss");
}
#[test]
fn beat7k_normal_manual_all_miss() {
    run_manual_all_miss_test("minimal_7k.bms", NORMAL, "beat7k_normal_manual_all_miss");
}
#[test]
fn beat7k_hard_manual_all_miss() {
    run_manual_all_miss_test("minimal_7k.bms", HARD, "beat7k_hard_manual_all_miss");
}
#[test]
fn beat7k_exhard_manual_all_miss() {
    run_manual_all_miss_test("minimal_7k.bms", EXHARD, "beat7k_exhard_manual_all_miss");
}
#[test]
fn beat7k_hazard_manual_all_miss() {
    run_manual_all_miss_test("minimal_7k.bms", HAZARD, "beat7k_hazard_manual_all_miss");
}

// ============================================================================
// Beat14K (14key_dp.bms): 6 gauges x 3 inputs = 18 tests
// ============================================================================

#[test]
fn beat14k_assist_easy_autoplay() {
    run_autoplay_test("14key_dp.bms", ASSISTEASY, "beat14k_assist_easy_autoplay");
}
#[test]
fn beat14k_easy_autoplay() {
    run_autoplay_test("14key_dp.bms", EASY, "beat14k_easy_autoplay");
}
#[test]
fn beat14k_normal_autoplay() {
    run_autoplay_test("14key_dp.bms", NORMAL, "beat14k_normal_autoplay");
}
#[test]
fn beat14k_hard_autoplay() {
    run_autoplay_test("14key_dp.bms", HARD, "beat14k_hard_autoplay");
}
#[test]
fn beat14k_exhard_autoplay() {
    run_autoplay_test("14key_dp.bms", EXHARD, "beat14k_exhard_autoplay");
}
#[test]
fn beat14k_hazard_autoplay() {
    run_autoplay_test("14key_dp.bms", HAZARD, "beat14k_hazard_autoplay");
}

#[test]
fn beat14k_assist_easy_manual_perfect() {
    run_manual_perfect_test(
        "14key_dp.bms",
        ASSISTEASY,
        "beat14k_assist_easy_manual_perfect",
    );
}
#[test]
fn beat14k_easy_manual_perfect() {
    run_manual_perfect_test("14key_dp.bms", EASY, "beat14k_easy_manual_perfect");
}
#[test]
fn beat14k_normal_manual_perfect() {
    run_manual_perfect_test("14key_dp.bms", NORMAL, "beat14k_normal_manual_perfect");
}
#[test]
fn beat14k_hard_manual_perfect() {
    run_manual_perfect_test("14key_dp.bms", HARD, "beat14k_hard_manual_perfect");
}
#[test]
fn beat14k_exhard_manual_perfect() {
    run_manual_perfect_test("14key_dp.bms", EXHARD, "beat14k_exhard_manual_perfect");
}
#[test]
fn beat14k_hazard_manual_perfect() {
    run_manual_perfect_test("14key_dp.bms", HAZARD, "beat14k_hazard_manual_perfect");
}

#[test]
fn beat14k_assist_easy_manual_all_miss() {
    run_manual_all_miss_test(
        "14key_dp.bms",
        ASSISTEASY,
        "beat14k_assist_easy_manual_all_miss",
    );
}
#[test]
fn beat14k_easy_manual_all_miss() {
    run_manual_all_miss_test("14key_dp.bms", EASY, "beat14k_easy_manual_all_miss");
}
#[test]
fn beat14k_normal_manual_all_miss() {
    run_manual_all_miss_test("14key_dp.bms", NORMAL, "beat14k_normal_manual_all_miss");
}
#[test]
fn beat14k_hard_manual_all_miss() {
    run_manual_all_miss_test("14key_dp.bms", HARD, "beat14k_hard_manual_all_miss");
}
#[test]
fn beat14k_exhard_manual_all_miss() {
    run_manual_all_miss_test("14key_dp.bms", EXHARD, "beat14k_exhard_manual_all_miss");
}
#[test]
fn beat14k_hazard_manual_all_miss() {
    run_manual_all_miss_test("14key_dp.bms", HAZARD, "beat14k_hazard_manual_all_miss");
}

// ============================================================================
// PopN9K (9key_pms.pms): 6 gauges x 3 inputs = 18 tests
// ============================================================================

#[test]
fn popn9k_assist_easy_autoplay() {
    run_autoplay_test("9key_pms.pms", ASSISTEASY, "popn9k_assist_easy_autoplay");
}
#[test]
fn popn9k_easy_autoplay() {
    run_autoplay_test("9key_pms.pms", EASY, "popn9k_easy_autoplay");
}
#[test]
fn popn9k_normal_autoplay() {
    run_autoplay_test("9key_pms.pms", NORMAL, "popn9k_normal_autoplay");
}
#[test]
fn popn9k_hard_autoplay() {
    run_autoplay_test("9key_pms.pms", HARD, "popn9k_hard_autoplay");
}
#[test]
fn popn9k_exhard_autoplay() {
    run_autoplay_test("9key_pms.pms", EXHARD, "popn9k_exhard_autoplay");
}
#[test]
fn popn9k_hazard_autoplay() {
    run_autoplay_test("9key_pms.pms", HAZARD, "popn9k_hazard_autoplay");
}

#[test]
fn popn9k_assist_easy_manual_perfect() {
    run_manual_perfect_test(
        "9key_pms.pms",
        ASSISTEASY,
        "popn9k_assist_easy_manual_perfect",
    );
}
#[test]
fn popn9k_easy_manual_perfect() {
    run_manual_perfect_test("9key_pms.pms", EASY, "popn9k_easy_manual_perfect");
}
#[test]
fn popn9k_normal_manual_perfect() {
    run_manual_perfect_test("9key_pms.pms", NORMAL, "popn9k_normal_manual_perfect");
}
#[test]
fn popn9k_hard_manual_perfect() {
    run_manual_perfect_test("9key_pms.pms", HARD, "popn9k_hard_manual_perfect");
}
#[test]
fn popn9k_exhard_manual_perfect() {
    run_manual_perfect_test("9key_pms.pms", EXHARD, "popn9k_exhard_manual_perfect");
}
#[test]
fn popn9k_hazard_manual_perfect() {
    run_manual_perfect_test("9key_pms.pms", HAZARD, "popn9k_hazard_manual_perfect");
}

#[test]
fn popn9k_assist_easy_manual_all_miss() {
    run_manual_all_miss_test(
        "9key_pms.pms",
        ASSISTEASY,
        "popn9k_assist_easy_manual_all_miss",
    );
}
#[test]
fn popn9k_easy_manual_all_miss() {
    run_manual_all_miss_test("9key_pms.pms", EASY, "popn9k_easy_manual_all_miss");
}
#[test]
fn popn9k_normal_manual_all_miss() {
    run_manual_all_miss_test("9key_pms.pms", NORMAL, "popn9k_normal_manual_all_miss");
}
#[test]
fn popn9k_hard_manual_all_miss() {
    run_manual_all_miss_test("9key_pms.pms", HARD, "popn9k_hard_manual_all_miss");
}
#[test]
fn popn9k_exhard_manual_all_miss() {
    run_manual_all_miss_test("9key_pms.pms", EXHARD, "popn9k_exhard_manual_all_miss");
}
#[test]
fn popn9k_hazard_manual_all_miss() {
    run_manual_all_miss_test("9key_pms.pms", HAZARD, "popn9k_hazard_manual_all_miss");
}
