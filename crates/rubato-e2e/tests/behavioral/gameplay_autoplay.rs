//! Phase 5a: Autoplay gameplay E2E tests.
//!
//! Tests BMS loading, play state creation, and basic gameplay with
//! LauncherStateFactory and real BMSPlayer.

use std::path::PathBuf;

use rubato_e2e::{E2eHarness, MainStateType};
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;

fn test_bms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
}

fn harness_with_bms(bms_filename: &str) -> E2eHarness {
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();

    let bms_path = test_bms_dir().join(bms_filename);
    assert!(
        bms_path.exists(),
        "test BMS file must exist: {:?}",
        bms_path
    );

    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("controller should own a player resource")
        .set_bms_file(&bms_path, 2, 0); // mode_type=2 is AUTOPLAY
    assert!(loaded, "BMS file should load successfully");

    harness
}

#[test]
fn play_state_creates_with_bms_file() {
    let mut harness = harness_with_bms("minimal_7k.bms");

    harness.change_state(MainStateType::Play);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn play_state_renders_without_crash() {
    let mut harness = harness_with_bms("minimal_7k.bms");

    harness.change_state(MainStateType::Play);
    // Render 10 frames without panicking
    harness.render_frames(10);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn play_state_with_5key_bms() {
    let mut harness = harness_with_bms("5key.bms");

    harness.change_state(MainStateType::Play);
    harness.render_frames(5);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn play_state_with_longnotes() {
    let bms_path = test_bms_dir().join("longnote_basic.bms");
    if !bms_path.exists() {
        // Skip if test fixture doesn't exist
        return;
    }

    let mut harness = harness_with_bms("longnote_basic.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frames(5);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn play_state_with_bpm_changes() {
    let bms_path = test_bms_dir().join("bpm_change.bms");
    if !bms_path.exists() {
        return;
    }

    let mut harness = harness_with_bms("bpm_change.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frames(5);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn audio_events_recorded_during_play_state_creation() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.clear_audio_events();

    harness.change_state(MainStateType::Play);

    // The audio driver should have received at least a SetModel event
    // during state transition (keysound loading)
    let events = harness.audio_events();
    let has_set_model = events
        .iter()
        .any(|e| matches!(e, rubato_e2e::AudioEvent::SetModel));
    assert!(
        has_set_model,
        "audio driver should receive SetModel event during play state creation"
    );
}

#[test]
fn bms_model_accessible_after_load() {
    let harness = harness_with_bms("minimal_7k.bms");

    let model = harness
        .controller()
        .player_resource()
        .and_then(|r| r.bms_model());
    assert!(model.is_some(), "BMS model should be accessible after load");

    let model = model.unwrap();
    assert!(
        model.total_notes() > 0,
        "loaded BMS should have notes (got {})",
        model.total_notes()
    );
}
