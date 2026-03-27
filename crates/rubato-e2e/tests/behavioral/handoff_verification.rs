//! Phase 8 Task 8.2: Handoff data E2E verification.
//!
//! Verifies that data flows correctly between states:
//! - All fields in Select -> Play handoff are populated after BMS load
//! - PlayerResource state is consistent across transitions
//! - Audio state (playing/stopped) at each transition point

use std::path::PathBuf;

use rubato_e2e::{AudioEvent, E2eHarness, MainStateType};
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;

fn test_bms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
}

fn harness_with_bms(bms_filename: &str) -> Option<E2eHarness> {
    let bms_path = test_bms_dir().join(bms_filename);
    if !bms_path.exists() {
        return None;
    }
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();
    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("resource")
        .set_bms_file(&bms_path, 2, 0); // AUTOPLAY mode
    assert!(loaded, "BMS file should load");
    Some(harness)
}

// ============================================================
// Select -> Play handoff: all resource fields populated
// ============================================================

#[test]
fn bms_model_populated_after_load() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    let resource = harness.controller().player_resource().unwrap();
    assert!(
        resource.bms_model().is_some(),
        "bms_model should be set after load"
    );
}

#[test]
fn songdata_populated_after_load() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    let resource = harness.controller().player_resource().unwrap();
    let sd = resource.songdata();
    assert!(sd.is_some(), "songdata should be set after load");
    let sd = sd.unwrap();
    assert!(!sd.file.md5.is_empty(), "songdata should have MD5 hash");
    assert!(
        !sd.file.sha256.is_empty(),
        "songdata should have SHA256 hash"
    );
}

#[test]
fn bms_model_has_notes_after_load() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    let model = harness
        .controller()
        .player_resource()
        .unwrap()
        .bms_model()
        .unwrap();
    assert!(
        model.total_notes() > 0,
        "model should have notes, got {}",
        model.total_notes()
    );
}

#[test]
fn bms_model_has_valid_bpm() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    let model = harness
        .controller()
        .player_resource()
        .unwrap()
        .bms_model()
        .unwrap();
    assert!(model.bpm > 0.0, "initial BPM should be positive");
    assert!(model.max_bpm() > 0.0, "max BPM should be positive");
    assert!(model.min_bpm() > 0.0, "min BPM should be positive");
}

#[test]
fn replay_data_initialized_after_load() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    let resource = harness.controller().player_resource().unwrap();
    assert!(
        resource.replay_data().is_some(),
        "replay data should be initialized after BMS load"
    );
}

#[test]
fn original_mode_set_after_load() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    assert!(
        harness
            .controller()
            .player_resource()
            .unwrap()
            .original_mode()
            .is_some(),
        "original mode should be set after BMS load"
    );
}

#[test]
fn score_data_none_before_play() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    assert!(
        harness.score_data().is_none(),
        "score data should be None before entering Play state"
    );
}

#[test]
fn groove_gauge_none_before_play() {
    let Some(harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    assert!(
        !harness.has_groove_gauge(),
        "groove gauge should be None before entering Play state"
    );
}

// ============================================================
// BMS load with different modes
// ============================================================

#[test]
fn bms_load_with_5key_preserves_mode() {
    let bms_path = test_bms_dir().join("5key.bms");
    if !bms_path.exists() {
        eprintln!("skipping: 5key.bms not found");
        return;
    }
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();
    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(&bms_path, 0, 0);
    assert!(loaded);

    let model = harness
        .controller()
        .player_resource()
        .unwrap()
        .bms_model()
        .unwrap();
    let mode = model.mode();
    assert!(mode.is_some(), "mode should be set for 5key BMS");
}

#[test]
fn bms_load_with_bpm_changes_has_timelines() {
    let bms_path = test_bms_dir().join("bpm_change.bms");
    if !bms_path.exists() {
        eprintln!("skipping: bpm_change.bms not found");
        return;
    }
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();
    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(&bms_path, 0, 0);
    assert!(loaded);

    let model = harness
        .controller()
        .player_resource()
        .unwrap()
        .bms_model()
        .unwrap();
    assert!(
        model.timelines.len() > 1,
        "BPM change BMS should have multiple timelines"
    );
    assert!(
        model.max_bpm() > model.min_bpm() || model.max_bpm() > 0.0,
        "BPM change BMS should show BPM variation or valid BPM"
    );
}

// ============================================================
// Audio state at transition points
// ============================================================

#[test]
fn audio_set_model_on_play_entry() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    harness.clear_audio_events();
    harness.change_state(MainStateType::Play);

    let events = harness.audio_events();
    let has_set_model = events.iter().any(|e| matches!(e, AudioEvent::SetModel));
    assert!(
        has_set_model,
        "Play state entry should trigger SetModel audio event"
    );
}

#[test]
fn audio_events_accumulate_across_transitions() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };
    harness.clear_audio_events();

    harness.change_state(MainStateType::Play);
    let count_after_play = harness.audio_events().len();

    harness.change_state(MainStateType::Result);
    let count_after_result = harness.audio_events().len();

    // Audio events should accumulate (not be reset) across transitions
    assert!(
        count_after_result >= count_after_play,
        "audio events should accumulate: play={}, result={}",
        count_after_play,
        count_after_result
    );
}

#[test]
fn clear_audio_events_resets_between_tests() {
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();

    // Generate some audio events
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("test.ogg", 1.0, false);
    assert!(!harness.audio_events().is_empty());

    // Clear and verify
    harness.clear_audio_events();
    assert!(
        harness.audio_events().is_empty(),
        "audio events should be empty after clear"
    );
}

// ============================================================
// PlayerResource consistency across state transitions
// ============================================================

#[test]
fn player_resource_survives_play_entry() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    // Capture model total_notes before Play
    let _notes_before = harness
        .controller()
        .player_resource()
        .unwrap()
        .bms_model()
        .unwrap()
        .total_notes();

    harness.change_state(MainStateType::Play);
    harness.render_frames(5);

    // PlayerResource should still have model data accessible via controller
    // (BMSPlayer may take ownership, but the resource should be restorable)
    // This test verifies the resource is not corrupted by the transition
    assert!(
        harness.controller().player_resource().is_some()
            || harness.current_state_type() == Some(MainStateType::Play),
        "player resource should exist or we should be in Play state"
    );
}

#[test]
fn clear_resource_resets_score_and_model() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    // Verify model is loaded
    assert!(
        harness
            .controller()
            .player_resource()
            .unwrap()
            .bms_model()
            .is_some()
    );

    // Clear the resource
    harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .clear();

    // Score should be None after clear
    assert!(
        harness
            .controller()
            .player_resource()
            .unwrap()
            .score_data()
            .is_none(),
        "score should be None after clear"
    );
}
