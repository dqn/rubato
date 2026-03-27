//! Phase 8 Task 8.1: E2E tests for state transition paths.
//!
//! Covers the missing transition paths:
//! - Select -> Decide -> Play -> Result (normal flow)
//! - Select -> Decide -> Play -> Result -> Select (loop back)
//! - Select -> Course -> CourseResult
//! - Escape/Back from each state
//! - Rapid state transitions (quick enter/exit)
//! - Double-transition edge cases

use rubato_e2e::{E2eHarness, E2eScenario, MainStateType, StateEvent};
use rubato_game::state_factory::LauncherStateFactory;

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator())
}

fn harness_with_controller_create() -> E2eHarness {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();
    harness
}

// ============================================================
// Normal flow: Select -> Decide -> Play -> Result
// ============================================================

#[test]
fn select_to_decide_to_play_to_result_flow() {
    let mut harness = harness_with_controller_create();

    // Start in MusicSelect
    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
    harness.render_frames(3);

    // Move to Decide
    harness.clear_state_events();
    harness.change_state(MainStateType::Decide);
    harness.assert_state(MainStateType::Decide);
    harness.render_frames(3);

    // Move to Play
    harness.change_state(MainStateType::Play);
    harness.assert_state(MainStateType::Play);
    harness.render_frames(3);

    // Move to Result (requires a PlayerResource)
    harness.ensure_player_resource();
    harness.change_state(MainStateType::Result);
    harness.assert_state(MainStateType::Result);
    harness.render_frames(3);

    // Verify the full event chain includes all transitions
    let events = harness.state_events();
    let has_decide_created = events.iter().any(|e| {
        matches!(
            e,
            StateEvent::StateCreated {
                state: MainStateType::Decide
            }
        )
    });
    let has_play_created = events.iter().any(|e| {
        matches!(
            e,
            StateEvent::StateCreated {
                state: MainStateType::Play
            }
        )
    });
    let has_result_created = events.iter().any(|e| {
        matches!(
            e,
            StateEvent::StateCreated {
                state: MainStateType::Result
            }
        )
    });
    assert!(has_decide_created, "Decide should have been created");
    assert!(has_play_created, "Play should have been created");
    assert!(has_result_created, "Result should have been created");
}

// ============================================================
// Loop back: Select -> Decide -> Play -> Result -> Select
// ============================================================

#[test]
fn result_to_select_loop_back() {
    let mut harness = harness_with_controller_create();

    // Full lifecycle: Select -> Decide -> Play -> Result -> Select
    for state in &[
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::MusicSelect,
    ] {
        // Result/CourseResult require a PlayerResource
        harness.ensure_player_resource();
        harness.change_state(*state);
        harness.assert_state(*state);
        harness.render_frames(2);
    }

    // After the loop, we should be back in MusicSelect
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn select_play_result_select_double_loop() {
    let mut harness = harness_with_controller_create();

    // Two full cycles through the state machine
    for _cycle in 0..2 {
        harness.change_state(MainStateType::MusicSelect);
        harness.render_frames(2);
        harness.change_state(MainStateType::Play);
        harness.render_frames(2);
        harness.ensure_player_resource();
        harness.change_state(MainStateType::Result);
        harness.render_frames(2);
    }

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

// ============================================================
// Select -> CourseResult path
// ============================================================

#[test]
fn select_to_course_result() {
    let mut harness = harness_with_controller_create();

    harness.change_state(MainStateType::MusicSelect);
    harness.render_frames(2);
    harness.ensure_player_resource();
    harness.change_state(MainStateType::CourseResult);
    harness.assert_state(MainStateType::CourseResult);
    harness.render_frames(3);
}

#[test]
fn play_to_course_result_path() {
    let mut harness = harness_with_controller_create();

    harness.change_state(MainStateType::Play);
    harness.render_frames(2);
    harness.ensure_player_resource();
    harness.change_state(MainStateType::CourseResult);
    harness.assert_state(MainStateType::CourseResult);
    harness.render_frames(3);
}

// ============================================================
// Escape/Back from each state
// ============================================================

#[test]
fn back_from_decide_to_select() {
    let mut harness = harness_with_controller_create();

    harness.change_state(MainStateType::Decide);
    harness.assert_state(MainStateType::Decide);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn back_from_play_to_select() {
    let mut harness = harness_with_controller_create();

    harness.change_state(MainStateType::Play);
    harness.assert_state(MainStateType::Play);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn back_from_result_to_select() {
    let mut harness = harness_with_controller_create();

    harness.ensure_player_resource();
    harness.change_state(MainStateType::Result);
    harness.assert_state(MainStateType::Result);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn back_from_course_result_to_select() {
    let mut harness = harness_with_controller_create();

    harness.ensure_player_resource();
    harness.change_state(MainStateType::CourseResult);
    harness.assert_state(MainStateType::CourseResult);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn back_from_config_to_select() {
    let mut harness = harness_with_factory();

    harness.change_state(MainStateType::Config);
    harness.assert_state(MainStateType::Config);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

#[test]
fn back_from_skin_config_to_select() {
    let mut harness = harness_with_factory();

    harness.change_state(MainStateType::SkinConfig);
    harness.assert_state(MainStateType::SkinConfig);
    harness.render_frames(2);

    harness.change_state(MainStateType::MusicSelect);
    harness.assert_state(MainStateType::MusicSelect);
}

// ============================================================
// Rapid state transitions (quick enter/exit)
// ============================================================

#[test]
fn rapid_select_play_toggling() {
    let mut harness = harness_with_controller_create();

    // Rapidly toggle between Select and Play 10 times
    for _ in 0..10 {
        harness.change_state(MainStateType::MusicSelect);
        harness.render_frame();
        harness.change_state(MainStateType::Play);
        harness.render_frame();
    }
    // Should not crash, state should be Play
    harness.assert_state(MainStateType::Play);
}

#[test]
fn rapid_cycle_through_all_states() {
    let mut harness = harness_with_controller_create();

    let states = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    // Cycle through all states 3 times with only 1 frame between each
    for _ in 0..3 {
        for &state in &states {
            // Result/CourseResult require a PlayerResource
            harness.ensure_player_resource();
            harness.change_state(state);
            harness.render_frame();
        }
    }

    // Should end on SkinConfig (last in the list)
    harness.assert_state(MainStateType::SkinConfig);
}

#[test]
fn change_state_without_render_between() {
    let mut harness = harness_with_controller_create();

    // Change state 5 times without rendering
    harness.change_state(MainStateType::MusicSelect);
    harness.change_state(MainStateType::Play);
    harness.ensure_player_resource();
    harness.change_state(MainStateType::Result);
    harness.change_state(MainStateType::MusicSelect);
    harness.change_state(MainStateType::Play);

    // Then render
    harness.render_frames(3);
    harness.assert_state(MainStateType::Play);
}

// ============================================================
// State event tracking for transitions
// ============================================================

#[test]
fn transition_events_ordered_correctly() {
    let mut harness = harness_with_controller_create();

    // MusicSelect may already be the current state after create(), so
    // transition to Play instead to get a full event sequence.
    harness.clear_state_events();
    harness.change_state(MainStateType::Play);

    // Should have TransitionStart, StateCreated, TransitionComplete in order
    harness.assert_event_sequence(&[
        StateEvent::TransitionStart {
            from: Some(MainStateType::MusicSelect),
            to: MainStateType::Play,
        },
        StateEvent::StateCreated {
            state: MainStateType::Play,
        },
        StateEvent::TransitionComplete {
            state: MainStateType::Play,
        },
    ]);
}

#[test]
fn shutdown_event_emitted_on_state_change() {
    let mut harness = harness_with_controller_create();

    harness.change_state(MainStateType::MusicSelect);
    harness.clear_state_events();

    // Transition to Play should shut down MusicSelect
    harness.change_state(MainStateType::Play);

    let events = harness.state_events();
    let has_select_shutdown = events.iter().any(|e| {
        matches!(
            e,
            StateEvent::StateShutdown {
                state: MainStateType::MusicSelect
            }
        )
    });
    assert!(
        has_select_shutdown,
        "MusicSelect should be shut down when transitioning to Play.\nEvents: {:?}",
        events
    );
}

// ============================================================
// Scenario builder tests
// ============================================================

#[test]
fn scenario_select_play_result_loop() {
    E2eScenario::new()
        .start_state(MainStateType::MusicSelect)
        .assert_state(MainStateType::MusicSelect)
        .render_frames(3)
        .change_state(MainStateType::Play)
        .assert_state(MainStateType::Play)
        .render_frames(3)
        .change_state(MainStateType::Result)
        .assert_state(MainStateType::Result)
        .render_frames(3)
        .change_state(MainStateType::MusicSelect)
        .assert_state(MainStateType::MusicSelect)
        .run();
}

#[test]
fn scenario_config_round_trip() {
    E2eScenario::new()
        .start_state(MainStateType::MusicSelect)
        .render_frames(2)
        .change_state(MainStateType::Config)
        .assert_state(MainStateType::Config)
        .render_frames(2)
        .change_state(MainStateType::MusicSelect)
        .assert_state(MainStateType::MusicSelect)
        .run();
}
