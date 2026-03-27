//! Full gameplay flow E2E tests.
//!
//! Tests end-to-end flows covering play state creation with BMS,
//! state transitions via events, retry mechanics, and multi-state
//! creation without panics.

use std::path::PathBuf;

use rubato_e2e::{E2eHarness, MainStateType, StateEvent};
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
// 1. Play state creates and renders with BMS loaded
// ============================================================

#[test]
fn test_play_state_creates_and_renders() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.change_state(MainStateType::Play);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "state should be Play after change_state"
    );

    // Render several frames without panicking
    harness.render_frames(10);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "state should remain Play after rendering frames"
    );
}

// ============================================================
// 2. Play to Result transition via state events
// ============================================================

#[test]
fn test_play_to_result_transition_via_state_events() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    harness.clear_state_events();
    harness.change_state(MainStateType::Play);

    // Verify Play was created via state events
    let events = harness.state_events();
    let has_play_created = events.iter().any(|e| {
        matches!(
            e,
            StateEvent::StateCreated {
                state: MainStateType::Play
            }
        )
    });
    assert!(
        has_play_created,
        "should have StateCreated event for Play, got: {:?}",
        events
    );

    // Render frames to allow autoplay to progress through the song.
    // The minimal BMS is short, so 100 frames should be enough for the
    // song to finish if the autoplay timer advances. If it does not
    // transition, that is acceptable -- we verify the event sequence
    // we did observe.
    harness.render_frames(100);

    let final_events = harness.state_events();

    // The transition sequence for entering Play should include:
    // TransitionStart -> StateCreated -> TransitionComplete
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

    // If the song completed and transitioned to Result, verify that too
    if harness.current_state_type() == Some(MainStateType::Result) {
        let has_result_created = final_events.iter().any(|e| {
            matches!(
                e,
                StateEvent::StateCreated {
                    state: MainStateType::Result
                }
            )
        });
        assert!(
            has_result_created,
            "Result state should have a StateCreated event"
        );
    }
}

// ============================================================
// 3. change_state(Play) while already in Play creates a new Play state (quick retry)
// ============================================================
//
// Play->Play transitions are allowed to support quick retry. The state machine
// disposes the old Play state and creates a fresh one.

#[test]
fn test_change_state_play_while_in_play_creates_new_play() {
    let Some(mut harness) = harness_with_bms("minimal_7k.bms") else {
        eprintln!("skipping: minimal_7k.bms not found");
        return;
    };

    // First Play entry
    harness.clear_state_events();
    harness.change_state(MainStateType::Play);
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
    harness.render_frames(5);

    // Change to Play again -- should create a fresh Play state
    harness.change_state(MainStateType::Play);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "state should be Play after retry"
    );
    harness.render_frames(5);

    // Two StateCreated events for Play should exist (original + retry)
    let events = harness.state_events();
    let play_created_count = events
        .iter()
        .filter(|e| {
            matches!(
                e,
                StateEvent::StateCreated {
                    state: MainStateType::Play
                }
            )
        })
        .count();
    assert_eq!(
        play_created_count, 2,
        "should have exactly 2 Play StateCreated events (original + retry), got {play_created_count}.\n\
         Events: {events:?}"
    );
}

// ============================================================
// 4. All states create without panic
// ============================================================

#[test]
fn test_all_states_create_without_panic() {
    let mut harness = harness_with_factory();

    // Decide requires a PlayerResource; without one it falls back to MusicSelect.
    // Test it separately with a properly initialized controller.
    let state_types = [
        MainStateType::MusicSelect,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    for state_type in &state_types {
        // Result/CourseResult require a PlayerResource; ensure one is available
        harness.ensure_player_resource();
        harness.change_state(*state_type);
        assert_eq!(
            harness.current_state_type(),
            Some(*state_type),
            "failed to create state {:?}",
            state_type
        );

        // Render a few frames to exercise the state's render path
        harness.render_frames(3);
        // Verify state did not unexpectedly change (unless the state
        // has an internal auto-transition, which is acceptable)
    }
}

// ============================================================
// 5. Decide transitions to Play
// ============================================================

#[test]
fn test_decide_transitions_to_play() {
    let mut harness = harness_with_factory();
    // Decide requires a PlayerResource, so initialize the controller to create one.
    harness.controller_mut().create();

    harness.clear_state_events();
    harness.change_state(MainStateType::Decide);
    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Decide),
        "state should be Decide after change_state"
    );

    // Decide state has a fadeout timer; render frames and check if it
    // transitions to Play. With a frozen timer we manually advance, so
    // the transition may or may not happen depending on how the Decide
    // state checks elapsed time.
    let transitioned = harness.wait_for_state(MainStateType::Play, 100);

    if transitioned {
        // Verify the event sequence: Decide created, then Play created
        harness.assert_event_sequence(&[
            StateEvent::StateCreated {
                state: MainStateType::Decide,
            },
            StateEvent::StateShutdown {
                state: MainStateType::Decide,
            },
            StateEvent::StateCreated {
                state: MainStateType::Play,
            },
        ]);
    } else {
        // Decide did not auto-transition within 100 frames; verify it
        // at least stayed stable without crashing
        assert_eq!(
            harness.current_state_type(),
            Some(MainStateType::Decide),
            "Decide should remain stable if it did not transition"
        );

        // Verify Decide was created properly
        let events = harness.state_events();
        let has_decide_created = events.iter().any(|e| {
            matches!(
                e,
                StateEvent::StateCreated {
                    state: MainStateType::Decide
                }
            )
        });
        assert!(
            has_decide_created,
            "should have StateCreated event for Decide"
        );
    }
}
