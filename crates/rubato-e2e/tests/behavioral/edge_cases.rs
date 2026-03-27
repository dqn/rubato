//! Phase 5e: Edge case robustness E2E tests.
//!
//! Tests that the system handles unusual or extreme scenarios without
//! panicking, including empty BMS files, rapid state transitions,
//! double renders, and high frame counts.

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

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator())
}

fn harness_with_bms(bms_filename: &str) -> Option<E2eHarness> {
    let bms_path = test_bms_dir().join(bms_filename);
    if !bms_path.exists() {
        return None;
    }

    let mut harness = harness_with_factory();
    harness.controller_mut().create();

    let loaded = harness
        .controller_mut()
        .player_resource_mut()
        .expect("controller should own a player resource")
        .set_bms_file(&bms_path, 2, 0); // mode_type=2 is AUTOPLAY
    assert!(loaded, "BMS file should load successfully");

    Some(harness)
}

// ============================================================
// 1. Empty BMS file (no notes) does not panic
// ============================================================

#[test]
fn test_empty_bms_no_panic() {
    let Some(mut harness) = harness_with_bms("empty_measures.bms") else {
        eprintln!("skipping: empty_measures.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "Play state should be created even with an empty BMS"
    );

    // Render several frames - should not panic
    harness.render_frames(20);
}

// ============================================================
// 2. Rapid state transitions do not panic
// ============================================================

#[test]
fn test_rapid_state_transitions() {
    let mut harness = harness_with_factory();

    // Rapidly cycle through multiple state transitions
    let transitions = [
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::Play,
        MainStateType::MusicSelect,
        MainStateType::Play,
    ];

    for state_type in &transitions {
        // Result/CourseResult require a PlayerResource
        harness.ensure_player_resource();
        harness.change_state(*state_type);
        assert_eq!(
            harness.current_state_type(),
            Some(*state_type),
            "state should be {:?} after rapid transition",
            state_type
        );
    }

    // Render a frame after all transitions to ensure the final state is stable
    harness.render_frame();
}

// ============================================================
// 3. Double render at the same time value does not panic
// ============================================================

#[test]
fn test_double_render_same_frame() {
    let mut harness = harness_with_factory();

    harness.change_state(MainStateType::Play);

    // Manually set time and call render twice at the same time value
    harness.set_time(100_000);
    harness.controller_mut().render();
    harness.controller_mut().render();

    // No panic = success
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

// ============================================================
// 4. State transition during render sequence does not panic
// ============================================================

#[test]
fn test_state_transition_during_render() {
    let mut harness = harness_with_factory();

    // Start rendering in Play state
    harness.change_state(MainStateType::Play);
    harness.render_frames(3);

    // Transition to a different state mid-sequence
    harness.change_state(MainStateType::MusicSelect);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    // Continue rendering in the new state
    harness.render_frames(3);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect),
        "MusicSelect should remain stable after transition during render"
    );
}

// ============================================================
// 5. Large frame count without BMS does not panic or overflow
// ============================================================

#[test]
fn test_large_frame_count() {
    let mut harness = harness_with_factory();

    harness.change_state(MainStateType::Play);

    // Render 500 frames without BMS loaded - should not panic or overflow
    harness.render_frames(500);

    // Verify time advanced correctly (500 frames * 16667us per frame)
    // render_frame() calls step_frame() then render(), so time = (500 + 0) * 16667
    // Note: change_state does not step time, only render_frames does.
    let expected_time = 500 * rubato_e2e::FRAME_DURATION_US;
    assert_eq!(
        harness.current_time_us(),
        expected_time,
        "time should be 500 * FRAME_DURATION_US after 500 render_frames"
    );
}
