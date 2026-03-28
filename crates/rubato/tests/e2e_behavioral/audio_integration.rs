//! Phase 5d: Audio integration E2E tests.
//!
//! Tests that audio events are correctly captured through the full pipeline.

use crate::e2e_support::{AudioEvent, E2eHarness};
use rubato_audio::audio_driver::AudioDriver;

#[test]
fn play_path_events_captured_through_controller() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("bgm/select.ogg", 0.8, true);

    let events = harness.audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(
        events[0],
        AudioEvent::PlayPath {
            path: "bgm/select.ogg".to_string(),
            volume: 0.8,
            loop_play: true,
        }
    );
}

#[test]
fn stop_path_events_captured() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("bgm.ogg", 1.0, false);
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .stop_path("bgm.ogg");

    let events = harness.audio_events();
    assert_eq!(events.len(), 2);
    assert!(matches!(events[1], AudioEvent::StopPath { .. }));
}

#[test]
fn global_pitch_events_captured() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .set_global_pitch(1.5);

    let events = harness.audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], AudioEvent::SetGlobalPitch { pitch: 1.5 });
}

#[test]
fn clear_audio_events_resets_log() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("a.wav", 1.0, false);
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("b.wav", 1.0, false);

    assert_eq!(harness.audio_events().len(), 2);
    harness.clear_audio_events();
    assert!(harness.audio_events().is_empty());

    // New events still captured after clear
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("c.wav", 1.0, false);
    assert_eq!(harness.audio_events().len(), 1);
}

#[test]
fn with_recording_driver_queries_state() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("bgm.ogg", 1.0, false);

    let is_playing = harness.with_recording_driver(|d| d.is_playing_path("bgm.ogg"));
    assert!(is_playing);

    let not_playing = harness.with_recording_driver(|d| d.is_playing_path("other.ogg"));
    assert!(!not_playing);
}

#[test]
fn dispose_events_captured() {
    let mut harness = E2eHarness::new();

    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .dispose();

    let events = harness.audio_events();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0], AudioEvent::Dispose);
}
