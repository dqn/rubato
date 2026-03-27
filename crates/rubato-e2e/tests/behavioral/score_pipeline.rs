//! Phase 5e: Score pipeline E2E tests.
//!
//! Tests that ScoreData, PlayerResource, and replay data flow correctly.

use std::path::PathBuf;

use rubato_e2e::E2eHarness;
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;

fn test_bms_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("test-bms")
}

fn harness_with_resource() -> E2eHarness {
    let mut harness =
        E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator());
    harness.controller_mut().create();
    harness
}

#[test]
fn player_resource_loads_bms_model() {
    let mut harness = harness_with_resource();

    let bms_path = test_bms_dir().join("minimal_7k.bms");
    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("resource")
        .set_bms_file(&bms_path, 0, 0);

    assert!(loaded, "BMS file should load");
    assert!(
        harness
            .controller()
            .player_resource()
            .unwrap()
            .bms_model()
            .is_some(),
        "model should be populated"
    );
}

#[test]
fn player_resource_populates_songdata_after_load() {
    let mut harness = harness_with_resource();

    let bms_path = test_bms_dir().join("minimal_7k.bms");
    harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(&bms_path, 0, 0);

    let songdata = harness.controller().player_resource().unwrap().songdata();
    assert!(
        songdata.is_some(),
        "songdata should be populated after load"
    );
    assert!(
        !songdata.unwrap().file.md5.is_empty(),
        "songdata should have MD5 hash"
    );
}

#[test]
fn player_resource_returns_false_for_nonexistent_file() {
    let mut harness = harness_with_resource();

    let result = harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(std::path::Path::new("/nonexistent/file.bms"), 0, 0);

    assert!(!result, "should return false for nonexistent file");
}

#[test]
fn player_resource_score_data_initially_none() {
    let mut harness = harness_with_resource();

    let score = harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .score_data();
    assert!(score.is_none(), "score should be None initially");
}

#[test]
fn player_resource_groove_gauge_initially_none() {
    let mut harness = harness_with_resource();

    let gauge = harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .groove_gauge();
    assert!(gauge.is_none(), "groove gauge should be None initially");
}

#[test]
fn player_resource_clear_via_trait() {
    let mut harness = harness_with_resource();

    let bms_path = test_bms_dir().join("minimal_7k.bms");
    harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(&bms_path, 0, 0);

    // Verify songdata is populated
    assert!(
        harness
            .controller_mut()
            .player_resource_mut()
            .unwrap()
            .songdata()
            .is_some()
    );

    // Clear resets score-related fields
    harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .clear();

    // Score should be None after clear
    assert!(
        harness
            .controller_mut()
            .player_resource_mut()
            .unwrap()
            .score_data()
            .is_none(),
        "score should be None after clear"
    );
}

#[test]
fn player_resource_original_mode_set_after_bms_load() {
    let mut harness = harness_with_resource();

    let bms_path = test_bms_dir().join("minimal_7k.bms");
    harness
        .controller_mut()
        .player_resource_mut()
        .unwrap()
        .set_bms_file(&bms_path, 0, 0);

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
