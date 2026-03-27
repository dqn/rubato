//! Phase 5b: Manual input gameplay E2E tests.
//!
//! Tests input injection and key state tracking through the harness.
//! Since full judge timing tests require precise note alignment, these tests
//! focus on verifying that:
//! 1. Input injection during play state does not crash
//! 2. Key state changes are visible through the input processor
//! 3. Multiple keys can be pressed simultaneously

use std::path::PathBuf;
use std::time::Duration;

use rubato_audio::recording_audio_driver::AudioEvent;
use rubato_e2e::{E2eHarness, MainStateType};
use rubato_game::state_factory::LauncherStateFactory;
use rubato_types::main_controller_access::MainControllerAccess;
use rubato_types::timer_id::TimerId;

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
        .set_bms_file(&bms_path, 0, 0); // mode_type=0 is PLAY (manual)
    assert!(loaded, "BMS file should load successfully");

    harness
}

#[test]
fn input_processor_exists_on_harness() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);

    let input = harness.controller().input_processor();
    assert!(
        input.is_some(),
        "input processor should be available after entering play state"
    );
}

#[test]
fn key_injection_during_play_does_not_crash() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frames(3);

    // Inject key down/up for several keys without crashing
    for key in 0..7 {
        harness.inject_key_down(key);
    }
    harness.render_frame();

    for key in 0..7 {
        harness.inject_key_up(key);
    }
    harness.render_frame();

    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "play state should remain active after key injection"
    );
}

#[test]
fn multiple_keys_can_be_pressed_simultaneously() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frame();

    let keys = [0, 2, 4, 6];
    for &key in &keys {
        harness.inject_key_down(key);
    }

    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist");
    for &key in &keys {
        assert!(
            input.key_state(key),
            "key {} should be pressed after inject_key_down",
            key
        );
    }

    // Keys not pressed should remain unpressed
    for &key in &[1, 3, 5] {
        assert!(!input.key_state(key), "key {} should NOT be pressed", key);
    }
}

#[test]
fn key_state_reflects_injection() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frame();

    // Key 3 should start unpressed
    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist");
    assert!(!input.key_state(3), "key 3 should start unpressed");

    // Press key 3
    harness.inject_key_down(3);
    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist");
    assert!(
        input.key_state(3),
        "key 3 should be pressed after inject_key_down"
    );

    // Release key 3
    harness.inject_key_up(3);
    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist");
    assert!(
        !input.key_state(3),
        "key 3 should be released after inject_key_up"
    );
}

#[test]
fn stale_key_state_is_cleared_when_entering_manual_play() {
    let mut harness = harness_with_bms("minimal_7k.bms");

    harness.inject_key_down(0);
    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist before play transition");
    assert!(
        input.key_state(0),
        "precondition: injected stale key should be set"
    );

    harness.change_state(MainStateType::Play);
    harness.render_frame();

    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist after play transition");
    assert!(
        !input.key_state(0),
        "play transition must clear stale manual key state"
    );
    assert!(
        !harness.controller().timer().is_timer_on(TimerId::new(101)),
        "stale key state must not leave the first lane beam timer on"
    );
}

#[test]
fn gameplay_key_held_during_ready_is_cleared_at_play_start() {
    let timer_play = TimerId::new(41);

    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);

    harness.inject_key_down(0);
    harness.clear_audio_events();

    let frames_to_timer_play = harness.render_until(
        |h| {
            h.controller()
                .current_state()
                .is_some_and(|state| state.main_state_data().timer.is_timer_on(timer_play))
        },
        240,
    );
    assert!(
        frames_to_timer_play < 240,
        "manual play should start TIMER_PLAY within warmup"
    );

    harness.render_frames(240);

    let play_note_events: Vec<AudioEvent> = harness
        .audio_events()
        .into_iter()
        .filter(|event| matches!(event, AudioEvent::PlayNote { .. }))
        .collect();
    assert!(
        play_note_events.is_empty(),
        "held Ready input must not auto-play note sounds after play start: {:?}",
        play_note_events
    );
}

#[test]
fn manual_play_release_switches_keyon_to_keyoff() {
    let timer_play = TimerId::new(41);
    let timer_keyon_first_lane = TimerId::new(101);
    let timer_keyoff_first_lane = TimerId::new(121);

    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);

    let frames = harness.render_until(
        |h| {
            h.controller()
                .current_state()
                .is_some_and(|state| state.main_state_data().timer.is_timer_on(timer_play))
        },
        240,
    );
    assert!(
        frames < 240,
        "play state should start TIMER_PLAY within warmup"
    );

    let play_start_us = harness
        .controller()
        .current_state()
        .expect("play state should exist")
        .main_state_data()
        .timer
        .micro_timer(timer_play);
    assert_ne!(
        play_start_us,
        i64::MIN,
        "manual play should have a TIMER_PLAY start time"
    );

    harness
        .controller_mut()
        .current_state_mut()
        .expect("play state should still exist")
        .main_state_data_mut()
        .timer
        .set_timer_on(timer_keyon_first_lane);
    std::thread::sleep(Duration::from_millis(2));
    harness.render_frame();

    let timer = &harness
        .controller()
        .current_state()
        .expect("play state should still exist")
        .main_state_data()
        .timer;
    assert!(
        !timer.is_timer_on(timer_keyon_first_lane),
        "manual play release should clear KEYON during the next play frame"
    );
    assert!(
        timer.is_timer_on(timer_keyoff_first_lane),
        "manual play release should enable KEYOFF during the next play frame"
    );
}

#[test]
fn inject_key_press_sends_down_then_up() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frame();

    // inject_key_press does key_down, renders N frames, then key_up
    harness.inject_key_press(2, 3);

    // After inject_key_press, the key should be released
    let input = harness
        .controller()
        .input_processor()
        .expect("input processor should exist");
    assert!(
        !input.key_state(2),
        "key should be released after inject_key_press completes"
    );

    // State should still be Play
    assert_eq!(harness.current_state_type(), Some(MainStateType::Play));
}

#[test]
fn key_injection_with_5key_bms() {
    let mut harness = harness_with_bms("5key.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frames(2);

    // Inject keys for 5-key mode (keys 0-4)
    for key in 0..5 {
        harness.inject_key_down(key);
    }
    harness.render_frame();

    for key in 0..5 {
        harness.inject_key_up(key);
    }
    harness.render_frame();

    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "play state should remain active after 5-key injection"
    );
}

#[test]
fn rapid_key_toggling_does_not_crash() {
    let mut harness = harness_with_bms("minimal_7k.bms");
    harness.change_state(MainStateType::Play);
    harness.render_frame();

    // Rapidly toggle key 0 on/off across multiple frames
    for _ in 0..20 {
        harness.inject_key_down(0);
        harness.render_frame();
        harness.inject_key_up(0);
        harness.render_frame();
    }

    assert_eq!(
        harness.current_state_type(),
        Some(MainStateType::Play),
        "play state should survive rapid key toggling"
    );
}
