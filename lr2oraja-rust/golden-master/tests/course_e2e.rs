// Course mode E2E tests: validates multi-song course invariants.
//
// Tests gauge carryover, score accumulation, and failure stopping behavior
// by simulating multiple songs in sequence with gauge state carried over
// between songs (using run_course_simulation / run_course_simulation_manual).

use beatoraja_types::groove_gauge::{CLASS, EXCLASS, EXHARD, EXHARDCLASS, HARD, NORMAL};
use bms_model::judge_note::JUDGE_PG;
use golden_master::e2e_helpers::*;

// ============================================================================
// Course autoplay tests (with real gauge carryover)
// ============================================================================

/// 2-song course with autoplay: total PG across the course should equal the
/// sum of individually-simulated PG counts for each song.
#[test]
fn course_two_stage_autoplay() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("5key.bms");

    let course = run_course_simulation(&[&model1, &model2], NORMAL);
    assert!(course.completed, "Course should complete with autoplay");
    assert_eq!(course.stages.len(), 2, "Should have 2 stage results");

    // Each stage should have all PG
    let total1 = course.stages[0].ghost.len();
    assert!(total1 > 0);
    assert_all_pgreat(&course.stages[0], total1, "course_stage1");

    let total2 = course.stages[1].ghost.len();
    assert!(total2 > 0);
    assert_all_pgreat(&course.stages[1], total2, "course_stage2");

    // Combined PG should be sum of individual
    let total_pg = course.stages[0].score.get_judge_count_total(JUDGE_PG)
        + course.stages[1].score.get_judge_count_total(JUDGE_PG);
    let individual_sum = total1 as i32 + total2 as i32;
    assert_eq!(
        total_pg, individual_sum,
        "Total PG across course should match sum of individual songs"
    );
}

// ============================================================================
// Course gauge carryover tests
// ============================================================================

/// Hard gauge value carries over between songs in a course.
/// With autoplay, Hard gauge should stay at ~100% throughout both songs.
#[test]
fn course_gauge_carryover() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("bpm_change.bms");

    let course = run_course_simulation(&[&model1, &model2], HARD);
    assert!(course.completed, "Course should complete with autoplay");
    assert_eq!(course.stages.len(), 2);

    // Hard gauge starts at 100 and should stay near 100 with autoplay
    for (i, stage) in course.stages.iter().enumerate() {
        assert!(
            (stage.gauge_value - 100.0).abs() < 1e-3,
            "Stage {}: Hard gauge should stay at ~100% with autoplay, got {}",
            i + 1,
            stage.gauge_value
        );
        assert!(
            stage.gauge_qualified,
            "Stage {}: should be qualified",
            i + 1
        );
    }
}

/// Class gauge carries over between songs: verify the gauge value after
/// the second song reflects accumulated recovery from both songs.
#[test]
fn course_class_gauge_carryover_accumulates() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("5key.bms");

    // Run each song independently to get individual gauge values
    let independent1 = run_autoplay_simulation(&model1, CLASS);
    let independent2 = run_autoplay_simulation(&model2, CLASS);

    // Run as course (gauge carries over)
    let course = run_course_simulation(&[&model1, &model2], CLASS);
    assert!(course.completed);
    assert_eq!(course.stages.len(), 2);

    // Stage 1 should match independent run (same starting gauge)
    assert!(
        (course.stages[0].gauge_value - independent1.gauge_value).abs() < 1e-3,
        "Stage 1 gauge should match independent: course={}, independent={}",
        course.stages[0].gauge_value,
        independent1.gauge_value
    );

    // Stage 2 in course starts with stage 1's end value (should be ~100),
    // while independent starts at init (100). Both are autoplay all-PG,
    // so the end values should be similar (both clamped to max).
    assert!(
        course.stages[1].gauge_qualified,
        "Stage 2 should be qualified in course mode"
    );
    assert!(
        independent2.gauge_qualified,
        "Independent stage 2 should be qualified"
    );
}

// ============================================================================
// Course failure stops tests
// ============================================================================

/// Hazard gauge: all-miss in song 1 kills gauge, preventing song 2 from running.
#[test]
fn course_failure_stops() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("5key.bms");

    // No input for song 1 (all miss), autoplay for song 2 doesn't matter
    // because the course should stop after song 1.
    let empty_log: &[beatoraja_input::key_input_log::KeyInputLog] = &[];
    let course = run_course_simulation_manual(&[&model1, &model2], &[empty_log, empty_log], EXHARD);

    assert!(
        !course.completed,
        "Course should NOT complete when gauge dies in song 1"
    );
    assert_eq!(
        course.stages.len(),
        1,
        "Only song 1 should be simulated (song 2 skipped due to gauge death)"
    );
    assert!(
        course.stages[0].gauge_value < 1e-6,
        "Song 1 gauge should be dead (value={})",
        course.stages[0].gauge_value
    );
    assert!(
        !course.stages[0].gauge_qualified,
        "Song 1 should NOT be qualified"
    );
}

/// ExHardClass: all-miss kills the gauge, stopping the course.
#[test]
fn course_exhardclass_failure_stops() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("bpm_change.bms");

    let empty_log: &[beatoraja_input::key_input_log::KeyInputLog] = &[];
    let course =
        run_course_simulation_manual(&[&model1, &model2], &[empty_log, empty_log], EXHARDCLASS);

    assert!(
        !course.completed,
        "ExHardClass course should not complete with all-miss"
    );
    assert_eq!(course.stages.len(), 1, "Only 1 stage should be completed");
    assert!(
        course.stages[0].gauge_value < 1e-6,
        "ExHardClass should be dead on all-miss (value={})",
        course.stages[0].gauge_value
    );
}

// ============================================================================
// Course gauge type tests
// ============================================================================

/// Course-specific gauge types (Class, ExClass, ExHardClass) should work with autoplay.
#[test]
fn course_gauge_types_autoplay() {
    let model = load_bms("minimal_7k.bms");

    for gauge_type in [CLASS, EXCLASS, EXHARDCLASS] {
        let result = run_autoplay_simulation(&model, gauge_type);
        assert!(
            result.gauge_qualified,
            "{gauge_type} should be qualified on autoplay (value={})",
            result.gauge_value
        );
    }
}

/// All-miss on course gauges should reduce gauge value compared to autoplay.
#[test]
fn course_gauge_types_all_miss_reduces_gauge() {
    let model = load_bms("minimal_7k.bms");

    for gauge_type in [CLASS, EXCLASS, EXHARDCLASS] {
        let autoplay_result = run_autoplay_simulation(&model, gauge_type);
        let miss_result = run_manual_simulation(&model, &[], gauge_type);

        assert!(
            miss_result.gauge_value < autoplay_result.gauge_value,
            "{gauge_type}: all-miss gauge ({}) should be less than autoplay gauge ({})",
            miss_result.gauge_value,
            autoplay_result.gauge_value
        );
    }
}

// ============================================================================
// Multi-BMS course score consistency
// ============================================================================

/// Autoplay score on each BMS should be deterministic across multiple runs.
#[test]
fn course_deterministic_scores() {
    let files = ["minimal_7k.bms", "5key.bms", "bpm_change.bms"];

    for filename in files {
        let model = load_bms(filename);
        let r1 = run_autoplay_simulation(&model, NORMAL);
        let r2 = run_autoplay_simulation(&model, NORMAL);

        assert_eq!(
            r1.score.get_judge_count_total(JUDGE_PG),
            r2.score.get_judge_count_total(JUDGE_PG),
            "{filename}: PG count should be deterministic"
        );
        assert_eq!(
            r1.max_combo, r2.max_combo,
            "{filename}: max combo should be deterministic"
        );
        assert_eq!(
            r1.ghost.len(),
            r2.ghost.len(),
            "{filename}: ghost length should be deterministic"
        );
    }
}

/// 3-song autoplay course should complete and produce consistent results.
#[test]
fn course_three_stage_autoplay() {
    let model1 = load_bms("minimal_7k.bms");
    let model2 = load_bms("5key.bms");
    let model3 = load_bms("bpm_change.bms");

    let course = run_course_simulation(&[&model1, &model2, &model3], CLASS);
    assert!(
        course.completed,
        "3-song course should complete with autoplay"
    );
    assert_eq!(course.stages.len(), 3);

    for (i, stage) in course.stages.iter().enumerate() {
        assert!(
            stage.gauge_qualified,
            "Stage {}: Class gauge should be qualified on autoplay (value={})",
            i + 1,
            stage.gauge_value
        );
    }
}
