//! Phase 5c: State lifecycle E2E tests.
//!
//! Tests full state transition flows using LauncherStateFactory.

use rubato_e2e::{E2eHarness, MainStateType};
use rubato_game::state_factory::LauncherStateFactory;

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator())
}

#[test]
fn music_select_to_play_to_result_lifecycle() {
    let mut harness = harness_with_factory();

    // MusicSelect
    harness.change_state(MainStateType::MusicSelect);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    // Render a few frames in MusicSelect
    harness.render_frames(3);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect)
    );

    // Play (without BMS file, creates BMSPlayer with default model)
    harness.change_state(MainStateType::Play);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));

    // Render a frame in Play
    harness.render_frame();
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));

    // Result (requires a PlayerResource)
    harness.ensure_player_resource();
    harness.change_state(MainStateType::Result);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Result));

    // Back to MusicSelect
    harness.change_state(MainStateType::MusicSelect);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect)
    );
}

#[test]
fn config_and_skin_config_transitions() {
    let mut harness = harness_with_factory();

    harness.change_state(MainStateType::Config);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Config));

    harness.change_state(MainStateType::SkinConfig);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::SkinConfig)
    );

    harness.change_state(MainStateType::MusicSelect);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::MusicSelect)
    );
}

#[test]
fn decide_state_creation() {
    let mut harness = harness_with_factory();
    // Decide requires a PlayerResource; initialize the controller to create one.
    harness.controller_mut().create();

    harness.change_state(MainStateType::Decide);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Decide));
}

#[test]
fn course_result_state_creation() {
    let mut harness = harness_with_factory();

    harness.ensure_player_resource();
    harness.change_state(MainStateType::CourseResult);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::CourseResult)
    );
}

#[test]
fn all_seven_state_types_can_be_created() {
    let mut harness = harness_with_factory();
    // Initialize controller so PlayerResource is available for Decide.
    harness.controller_mut().create();

    let types = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    for state_type in &types {
        // Result/CourseResult require a PlayerResource
        harness.ensure_player_resource();
        harness.change_state(*state_type);
        assert_eq!(
            harness.current_state_type(),
            Some(*state_type),
            "failed to create state {:?}",
            state_type
        );
    }
}

#[test]
fn render_does_not_crash_for_any_state() {
    let mut harness = harness_with_factory();
    // Initialize controller so PlayerResource is available for Decide.
    harness.controller_mut().create();

    let types = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::Config,
    ];

    for state_type in &types {
        // Result requires a PlayerResource
        harness.ensure_player_resource();
        harness.change_state(*state_type);
        harness.render_frame();
        // No panic = success
    }
}
