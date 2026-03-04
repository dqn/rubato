// Replay round-trip E2E tests: validates record -> serialize -> deserialize -> replay.
//
// Tests that autoplay key logs can be recorded, serialized to JSON,
// loaded back, and replayed to produce identical scores.

use bms_model::judge_note::{JUDGE_MS, JUDGE_PG, JUDGE_PR};
use bms_model::mode::Mode;
use golden_master::e2e_helpers::*;
use rubato_types::groove_gauge::{EXHARD, HARD, NORMAL};
use rubato_types::replay_data::ReplayData;
use rubato_types::stubs::KeyInputLog as ReplayKeyInputLog;

/// Record autoplay key events from a simulation.
///
/// Generates a KeyInputLog from the BMS note data as if the player hit
/// every note perfectly (offset 0).
fn record_autoplay_keylog(
    model: &bms_model::bms_model::BMSModel,
) -> Vec<rubato_input::key_input_log::KeyInputLog> {
    let jn = model.build_judge_notes();
    let mode = model.get_mode().unwrap_or(&Mode::BEAT_7K);
    create_note_press_log(&jn, mode, 0)
}

// ============================================================================
// Record and replay match tests
// ============================================================================

/// Autoplay-generated keylog, when replayed manually, should produce the same score.
#[test]
fn record_and_replay_match() {
    let model = load_bms("minimal_7k.bms");
    let jn = model.build_judge_notes();
    let normal = count_normal_notes(&jn);

    // Record: generate key log as if perfect manual play
    let keylog = record_autoplay_keylog(&model);
    assert!(!keylog.is_empty(), "Should have key events");

    // Replay: use the keylog in manual simulation
    let manual_result = run_manual_simulation(&model, &keylog, NORMAL);

    // Both should have all PGREAT for normal notes
    assert_eq!(
        manual_result.score.get_judge_count_total(JUDGE_PG),
        normal as i32,
        "Replayed keylog should produce all PG (PG={}, total_judge={})",
        manual_result.score.get_judge_count_total(JUDGE_PG),
        (0..6)
            .map(|j| manual_result.score.get_judge_count_total(j))
            .sum::<i32>(),
    );

    // Gauge should be qualified
    assert!(
        manual_result.gauge_qualified,
        "Replayed should be qualified (gauge={})",
        manual_result.gauge_value
    );
}

// ============================================================================
// JSON serde round-trip tests
// ============================================================================

/// ReplayData serialization round-trip: serde_json produces identical keylog.
#[test]
fn replay_json_round_trip() {
    let model = load_bms("minimal_7k.bms");
    let keylog = record_autoplay_keylog(&model);

    // Convert rubato_input KeyInputLog to stub KeyInputLog for ReplayData
    let replay_keylog: Vec<ReplayKeyInputLog> = keylog
        .iter()
        .map(|k| ReplayKeyInputLog {
            time: k.get_time(),
            keycode: k.get_keycode(),
            pressed: k.is_pressed(),
        })
        .collect();

    let replay = ReplayData {
        player: Some("test".to_string()),
        sha256: Some("abc123".to_string()),
        mode: model.get_mode().map(|m| m.key()).unwrap_or(0),
        keylog: replay_keylog.clone(),
        gauge: NORMAL,
        ..Default::default()
    };

    // Serialize to JSON
    let json = serde_json::to_string(&replay).expect("Failed to serialize ReplayData");

    // Deserialize back
    let loaded: ReplayData = serde_json::from_str(&json).expect("Failed to deserialize ReplayData");

    // Verify key log matches
    assert_eq!(
        loaded.keylog.len(),
        replay_keylog.len(),
        "Loaded keylog length should match original"
    );
    for (i, (original, loaded_entry)) in replay_keylog.iter().zip(loaded.keylog.iter()).enumerate()
    {
        assert_eq!(original.time, loaded_entry.time, "Entry {i}: time mismatch");
        assert_eq!(
            original.keycode, loaded_entry.keycode,
            "Entry {i}: keycode mismatch"
        );
        assert_eq!(
            original.pressed, loaded_entry.pressed,
            "Entry {i}: pressed mismatch"
        );
    }

    // Verify metadata
    assert_eq!(loaded.player.as_deref(), Some("test"));
    assert_eq!(loaded.sha256.as_deref(), Some("abc123"));
    assert_eq!(loaded.mode, model.get_mode().map(|m| m.key()).unwrap_or(0));
}

/// JSON round-trip preserves playback: loaded keylog produces same simulation result.
#[test]
fn replay_json_playback_matches() {
    let model = load_bms("minimal_7k.bms");
    let keylog = record_autoplay_keylog(&model);

    // Original simulation
    let original_result = run_manual_simulation(&model, &keylog, NORMAL);

    // Convert to stub KeyInputLog for ReplayData
    let replay_keylog: Vec<ReplayKeyInputLog> = keylog
        .iter()
        .map(|k| ReplayKeyInputLog {
            time: k.get_time(),
            keycode: k.get_keycode(),
            pressed: k.is_pressed(),
        })
        .collect();

    let replay = ReplayData {
        mode: model.get_mode().map(|m| m.key()).unwrap_or(0),
        keylog: replay_keylog,
        ..Default::default()
    };

    // Round-trip through JSON
    let json = serde_json::to_string(&replay).unwrap();
    let loaded: ReplayData = serde_json::from_str(&json).unwrap();

    // Convert loaded stub KeyInputLog back to rubato_input KeyInputLog
    let loaded_keylog: Vec<rubato_input::key_input_log::KeyInputLog> = loaded
        .keylog
        .iter()
        .map(|entry| {
            rubato_input::key_input_log::KeyInputLog::with_data(
                entry.time,
                entry.keycode,
                entry.pressed,
            )
        })
        .collect();

    // Replay with loaded keylog
    let loaded_result = run_manual_simulation(&model, &loaded_keylog, NORMAL);

    // Scores should match exactly
    assert_eq!(
        original_result.score.get_judge_count_total(JUDGE_PG),
        loaded_result.score.get_judge_count_total(JUDGE_PG),
        "PG count should match after JSON round-trip"
    );
    assert_eq!(
        original_result.max_combo, loaded_result.max_combo,
        "Max combo should match after JSON round-trip"
    );
    assert_eq!(
        original_result.ghost, loaded_result.ghost,
        "Ghost data should match after JSON round-trip"
    );
}

// ============================================================================
// Same input, different gauge tests
// ============================================================================

/// Same keylog with different gauge types should produce identical judgements
/// but potentially different gauge values and qualification results.
#[test]
fn replay_different_gauge_same_input() {
    let model = load_bms("minimal_7k.bms");
    let keylog = record_autoplay_keylog(&model);

    let normal_result = run_manual_simulation(&model, &keylog, NORMAL);
    let hard_result = run_manual_simulation(&model, &keylog, HARD);
    let exhard_result = run_manual_simulation(&model, &keylog, EXHARD);

    // Judgements should be identical across all gauge types
    assert_eq!(
        normal_result.score.get_judge_count_total(JUDGE_PG),
        hard_result.score.get_judge_count_total(JUDGE_PG),
        "PG count should be same for Normal vs Hard"
    );
    assert_eq!(
        normal_result.score.get_judge_count_total(JUDGE_PG),
        exhard_result.score.get_judge_count_total(JUDGE_PG),
        "PG count should be same for Normal vs ExHard"
    );

    // Max combo should be the same
    assert_eq!(
        normal_result.max_combo, hard_result.max_combo,
        "Max combo should be same across gauge types"
    );

    // Ghost data should be identical
    assert_eq!(
        normal_result.ghost, hard_result.ghost,
        "Ghost should be same for Normal vs Hard"
    );
    assert_eq!(
        normal_result.ghost, exhard_result.ghost,
        "Ghost should be same for Normal vs ExHard"
    );

    // All should be qualified (perfect input)
    assert!(normal_result.gauge_qualified, "Normal should be qualified");
    assert!(hard_result.gauge_qualified, "Hard should be qualified");
    assert!(exhard_result.gauge_qualified, "ExHard should be qualified");
}

/// Same keylog (all-miss) with different gauge types: scores same, gauge results differ.
#[test]
fn replay_different_gauge_all_miss() {
    let model = load_bms("minimal_7k.bms");
    let total = model.get_total_notes() as usize;

    let normal_result = run_manual_simulation(&model, &[], NORMAL);
    let hard_result = run_manual_simulation(&model, &[], HARD);
    let exhard_result = run_manual_simulation(&model, &[], EXHARD);

    // All should have same miss count
    for (label, result) in [
        ("Normal", &normal_result),
        ("Hard", &hard_result),
        ("ExHard", &exhard_result),
    ] {
        let miss = result.score.get_judge_count_total(JUDGE_PR)
            + result.score.get_judge_count_total(JUDGE_MS);
        assert_eq!(miss, total as i32, "{label}: all notes should be MISS/PR");
    }

    // None should be qualified
    assert!(!normal_result.gauge_qualified);
    assert!(!hard_result.gauge_qualified);
    assert!(!exhard_result.gauge_qualified);

    // Hard/ExHard should have gauge = 0 (dead)
    assert!(
        hard_result.gauge_value < 1e-6,
        "Hard gauge should be 0 on all-miss"
    );
    assert!(
        exhard_result.gauge_value < 1e-6,
        "ExHard gauge should be 0 on all-miss"
    );
}
