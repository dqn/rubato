// Integration tests for the input -> judge -> gauge pipeline.
//
// These tests cover the "middle layer" between raw input events and final
// results, validating that:
// 1. GrooveGauge responds correctly to different judge results
// 2. JudgeManager initializes and tracks state through from_config
// 3. Autoplay log generation produces correct key events for a model

use bms_model::bms_model::BMSModel;
use bms_model::judge_note::{JUDGE_PG, JudgeNoteKind};
use bms_model::mode::Mode;
use bms_model::note::Note;
use bms_model::time_line::TimeLine;
use rubato_input::key_input_log::KeyInputLog;
use rubato_play::bms_player_rule::BMSPlayerRule;
use rubato_play::groove_gauge::create_groove_gauge;
use rubato_play::judge_algorithm::JudgeAlgorithm;
use rubato_play::judge_manager::{JudgeConfig, JudgeManager};
use rubato_types::groove_gauge::{HARD, NORMAL};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Build a minimal BMSModel with the given mode and timelines.
/// Sets timelines before mode so that set_mode can resize lane counts.
fn make_model(mode: Mode, timelines: Vec<TimeLine>) -> BMSModel {
    let mut model = BMSModel::new();
    model.timelines = timelines;
    model.set_mode(mode);
    model
}

/// Build a BEAT_7K model with `count` normal notes on lane 0, spaced 1 second
/// apart starting at 1 second.
fn make_model_with_normal_notes(count: usize) -> BMSModel {
    let key_count = Mode::BEAT_7K.key();
    let mut timelines = Vec::with_capacity(count);
    for i in 0..count {
        let time_us = ((i + 1) as i64) * 1_000_000; // 1s, 2s, 3s, ...
        let mut tl = TimeLine::new(i as f64, time_us, key_count);
        tl.set_note(0, Some(Note::new_normal(1)));
        timelines.push(tl);
    }
    make_model(Mode::BEAT_7K, timelines)
}

// ===========================================================================
// Test 1: Groove gauge changes after judge events
// ===========================================================================

#[test]
fn gauge_increases_on_pgreat() {
    let model = make_model_with_normal_notes(1);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();

    let initial = gg.value();
    gg.update(0); // PGREAT
    assert!(
        gg.value() > initial,
        "gauge should increase on PGREAT: before={}, after={}",
        initial,
        gg.value()
    );
}

#[test]
fn gauge_decreases_on_poor() {
    let model = make_model_with_normal_notes(1);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();

    // Raise gauge first so the decrease is observable
    gg.set_value(80.0);
    let before = gg.value();
    gg.update(4); // POOR
    assert!(
        gg.value() < before,
        "gauge should decrease on POOR: before={}, after={}",
        before,
        gg.value()
    );
}

#[test]
fn gauge_sequence_pgreat_then_poor() {
    let model = make_model_with_normal_notes(4);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();

    let initial = gg.value();

    // Two PGREATs should increase the gauge
    gg.update(0);
    gg.update(0);
    let after_pgreats = gg.value();
    assert!(
        after_pgreats > initial,
        "gauge should rise after 2 PGREATs: {} -> {}",
        initial,
        after_pgreats
    );

    // One POOR should decrease
    gg.update(4);
    let after_poor = gg.value();
    assert!(
        after_poor < after_pgreats,
        "gauge should drop after POOR: {} -> {}",
        after_pgreats,
        after_poor
    );
}

#[test]
fn hard_gauge_dies_after_many_poors() {
    let model = make_model_with_normal_notes(1);
    let mut gg = create_groove_gauge(&model, HARD, 0, None).unwrap();
    assert!((gg.value() - 100.0).abs() < f32::EPSILON);

    // Repeatedly apply POOR until gauge reaches 0
    for _ in 0..50 {
        gg.update(4); // POOR
    }
    assert!(
        (gg.value() - 0.0).abs() < f32::EPSILON,
        "hard gauge should be dead after many POORs, got {}",
        gg.value()
    );
    assert!(
        !gg.is_qualified(),
        "dead hard gauge should not be qualified"
    );
}

// ===========================================================================
// Test 2: JudgeManager initialization and basic judge assignment
// ===========================================================================

#[test]
fn judge_manager_default_state() {
    let jm = JudgeManager::new();
    assert_eq!(jm.combo(), 0);
    assert_eq!(jm.course_combo(), 0);
    assert_eq!(jm.course_maxcombo(), 0);
    // All judge counts should be zero
    for judge in 0..6 {
        assert_eq!(
            jm.judge_count(judge),
            0,
            "judge count for {} should be 0",
            judge
        );
    }
}

#[test]
fn judge_manager_from_config_initializes_correctly() {
    let model = make_model_with_normal_notes(3);
    let judge_notes = model.build_judge_notes();
    let mode = Mode::BEAT_7K;
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let jm = JudgeManager::from_config(&config);

    // Initial state: no notes judged yet
    assert_eq!(jm.combo(), 0);
    for judge in 0..6 {
        assert_eq!(
            jm.judge_count(judge),
            0,
            "initial judge count for {} should be 0",
            judge
        );
    }
    // Ghost should be initialized with POOR (4) for each playable note
    let ghost = jm.ghost();
    assert_eq!(
        ghost.len(),
        3,
        "ghost should have one entry per playable note"
    );
}

#[test]
fn judge_manager_autoplay_judges_all_pgreat() {
    let model = make_model_with_normal_notes(4);
    let judge_notes = model.build_judge_notes();
    let mode = Mode::BEAT_7K;
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: true,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let mut jm = JudgeManager::from_config(&config);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();

    let key_count = mode.key() as usize;
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    // Prime
    jm.update(-1, &judge_notes, &key_states, &key_times, &mut gg);

    // Simulate past all notes (last note at 4s, run to 6s)
    let mut time = 0i64;
    while time <= 6_000_000 {
        jm.update(time, &judge_notes, &key_states, &key_times, &mut gg);
        time += 1_000; // 1ms steps
    }

    // Autoplay should judge all 4 notes as PGREAT
    assert_eq!(
        jm.judge_count(JUDGE_PG),
        4,
        "autoplay should produce 4 PGREATs, got PG={} GR={} GD={} BD={} PR={} MS={}",
        jm.judge_count(0),
        jm.judge_count(1),
        jm.judge_count(2),
        jm.judge_count(3),
        jm.judge_count(4),
        jm.judge_count(5),
    );
    assert_eq!(jm.combo(), 4, "combo should be 4 after 4 PGREATs");
}

// ===========================================================================
// Test 3: Autoplay log validation
// ===========================================================================

#[test]
fn autoplay_log_correct_count_for_normal_notes() {
    let model = make_model_with_normal_notes(3);
    let log = KeyInputLog::create_autoplay_log(&model);

    // For 3 normal notes on lane 0, the autoplay algorithm generates:
    // - For each timeline: a press event for lane 0 (note present)
    // - For each timeline: release events for lanes 1-6 (no note, no active LN)
    // - For each timeline: release events for lane 7 (scratch, generates two: lane 7 + lane 8)
    // Total per timeline: 1 press + 6 releases + 2 scratch releases = 9
    // But we primarily care about the press events for lane 0.
    let presses: Vec<_> = log
        .iter()
        .filter(|l| l.keycode() == 0 && l.is_pressed())
        .collect();
    assert_eq!(
        presses.len(),
        3,
        "should have 3 press events for lane 0, got {}",
        presses.len()
    );
}

#[test]
fn autoplay_log_correct_timing() {
    let model = make_model_with_normal_notes(2);
    let log = KeyInputLog::create_autoplay_log(&model);

    let presses: Vec<_> = log
        .iter()
        .filter(|l| l.keycode() == 0 && l.is_pressed())
        .collect();
    assert_eq!(presses.len(), 2);
    assert_eq!(presses[0].time(), 1_000_000, "first note at 1s");
    assert_eq!(presses[1].time(), 2_000_000, "second note at 2s");
}

#[test]
fn autoplay_log_correct_keycodes_for_multiple_lanes() {
    let key_count = Mode::BEAT_7K.key();
    // Place notes on lanes 0, 2, and 4 at different times
    let mut tl1 = TimeLine::new(0.0, 1_000_000, key_count);
    tl1.set_note(0, Some(Note::new_normal(1)));

    let mut tl2 = TimeLine::new(1.0, 2_000_000, key_count);
    tl2.set_note(2, Some(Note::new_normal(1)));

    let mut tl3 = TimeLine::new(2.0, 3_000_000, key_count);
    tl3.set_note(4, Some(Note::new_normal(1)));

    let model = make_model(Mode::BEAT_7K, vec![tl1, tl2, tl3]);
    let log = KeyInputLog::create_autoplay_log(&model);

    // Verify press events for each lane
    let lane0_presses: Vec<_> = log
        .iter()
        .filter(|l| l.keycode() == 0 && l.is_pressed())
        .collect();
    let lane2_presses: Vec<_> = log
        .iter()
        .filter(|l| l.keycode() == 2 && l.is_pressed())
        .collect();
    let lane4_presses: Vec<_> = log
        .iter()
        .filter(|l| l.keycode() == 4 && l.is_pressed())
        .collect();

    assert_eq!(lane0_presses.len(), 1, "lane 0 should have 1 press");
    assert_eq!(lane2_presses.len(), 1, "lane 2 should have 1 press");
    assert_eq!(lane4_presses.len(), 1, "lane 4 should have 1 press");

    assert_eq!(lane0_presses[0].time(), 1_000_000);
    assert_eq!(lane2_presses[0].time(), 2_000_000);
    assert_eq!(lane4_presses[0].time(), 3_000_000);
}

#[test]
fn autoplay_log_generates_nothing_for_no_mode() {
    let model = BMSModel::new(); // no mode set
    let log = KeyInputLog::create_autoplay_log(&model);
    assert!(
        log.is_empty(),
        "model without mode should produce empty log"
    );
}

// ===========================================================================
// Test 4: End-to-end manual input -> judge -> gauge pipeline
// ===========================================================================

#[test]
fn manual_input_at_note_time_produces_pgreat() {
    // Create a model with a single note
    let model = make_model_with_normal_notes(1);
    let judge_notes = model.build_judge_notes();
    assert_eq!(judge_notes.len(), 1);
    assert_eq!(judge_notes[0].kind, JudgeNoteKind::Normal);

    let mode = Mode::BEAT_7K;
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let mut jm = JudgeManager::from_config(&config);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();
    let gauge_before = gg.value();

    let key_count = mode.key() as usize;

    // Prime
    let empty_states = vec![false; key_count];
    let empty_times = vec![i64::MIN; key_count];
    jm.update(-1, &judge_notes, &empty_states, &empty_times, &mut gg);

    // Press key 0 at exactly note time (1_000_000 us)
    let note_time = 1_000_000i64;
    let mut key_states = vec![false; key_count];
    key_states[0] = true;
    let mut key_times = vec![i64::MIN; key_count];
    key_times[0] = note_time;

    jm.update(note_time, &judge_notes, &key_states, &key_times, &mut gg);

    // The note should be judged as PGREAT (exact timing)
    assert_eq!(
        jm.judge_count(JUDGE_PG),
        1,
        "exact timing should produce PGREAT"
    );
    assert_eq!(jm.combo(), 1);

    // Gauge should have increased
    assert!(
        gg.value() > gauge_before,
        "gauge should increase after PGREAT: before={}, after={}",
        gauge_before,
        gg.value()
    );
}

#[test]
fn no_input_produces_all_miss() {
    // Create a model with 2 notes
    let model = make_model_with_normal_notes(2);
    let judge_notes = model.build_judge_notes();
    let mode = Mode::BEAT_7K;
    let rule = BMSPlayerRule::for_mode(&mode);

    let config = JudgeConfig {
        notes: &judge_notes,
        mode: &mode,
        ln_type: model.lntype(),
        judge_rank: model.judgerank,
        judge_window_rate: [100, 100, 100],
        scratch_judge_window_rate: [100, 100, 100],
        algorithm: JudgeAlgorithm::Combo,
        autoplay: false,
        judge_property: &rule.judge,
        lane_property: None,
        auto_adjust_enabled: false,
        is_play_or_practice: false,
    };

    let mut jm = JudgeManager::from_config(&config);
    let mut gg = create_groove_gauge(&model, NORMAL, 0, None).unwrap();

    let key_count = mode.key() as usize;
    let key_states = vec![false; key_count];
    let key_times = vec![i64::MIN; key_count];

    // Prime
    jm.update(-1, &judge_notes, &key_states, &key_times, &mut gg);

    // Run past all notes without pressing any keys (notes at 1s and 2s, run to 4s)
    let mut time = 0i64;
    while time <= 4_000_000 {
        jm.update(time, &judge_notes, &key_states, &key_times, &mut gg);
        time += 1_000;
    }

    // All notes should be POOR or MISS
    let miss_count = jm.judge_count(4) + jm.judge_count(5); // PR + MS
    assert_eq!(
        miss_count,
        2,
        "2 notes with no input should all be PR/MS, got PR={} MS={}",
        jm.judge_count(4),
        jm.judge_count(5)
    );
    assert_eq!(jm.combo(), 0, "combo should remain 0 with no hits");
}
