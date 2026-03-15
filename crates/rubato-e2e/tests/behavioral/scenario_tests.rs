//! Phase 6d: Scenario builder E2E tests.
//!
//! Demonstrates the fluent `E2eScenario` builder for concise test authoring.

use rubato_e2e::{E2eScenario, MainStateType};

#[test]
fn scenario_play_renders_without_panic() {
    E2eScenario::new()
        .start_state(MainStateType::Play)
        .render_frames(30)
        .assert_state(MainStateType::Play)
        .run();
}

#[test]
fn scenario_state_transition_chain() {
    E2eScenario::new()
        .start_state(MainStateType::MusicSelect)
        .assert_state(MainStateType::MusicSelect)
        .render_frames(5)
        .change_state(MainStateType::Play)
        .assert_state(MainStateType::Play)
        .render_frames(3)
        .change_state(MainStateType::Result)
        .assert_state(MainStateType::Result)
        .run();
}

#[test]
fn scenario_custom_step_inspects_harness() {
    E2eScenario::new()
        .start_state(MainStateType::Config)
        .render_frames(10)
        .then(|h| {
            let frame = h.dump_frame_state();
            assert_eq!(frame.state_type, Some(MainStateType::Config));
            assert!(frame.time_us > 0, "time should have advanced");
        })
        .run();
}
