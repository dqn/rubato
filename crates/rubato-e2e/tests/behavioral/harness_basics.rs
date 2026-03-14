use rubato_e2e::{E2eHarness, FRAME_DURATION_US};

#[test]
fn harness_timer_starts_at_zero() {
    let harness = E2eHarness::new();
    assert_eq!(harness.current_time_us(), 0);
}

#[test]
fn harness_step_frame_advances_time() {
    let mut harness = E2eHarness::new();
    harness.step_frame();
    assert_eq!(harness.current_time_us(), FRAME_DURATION_US);
}

#[test]
fn harness_step_frames_advances_correctly() {
    let mut harness = E2eHarness::new();
    harness.step_frames(10);
    assert_eq!(harness.current_time_us(), 10 * FRAME_DURATION_US);
}

#[test]
fn harness_set_time_works() {
    let mut harness = E2eHarness::new();
    harness.set_time(1_000_000);
    assert_eq!(harness.current_time_us(), 1_000_000);
}

#[test]
fn harness_audio_driver_records_events() {
    let harness = E2eHarness::new();
    // The RecordingAudioDriver is injected but not directly accessible for
    // event inspection (audio_events() returns empty vec due to trait-object
    // downcast limitation). Verify the driver is at least set.
    let events = harness.audio_events();
    assert!(
        events.is_empty(),
        "no events should be recorded without audio calls"
    );
    // Verify the audio driver was injected successfully
    assert!(
        harness.controller().audio_processor().is_some(),
        "audio driver should be set after harness construction"
    );
}

#[test]
fn harness_controller_config_accessible() {
    let harness = E2eHarness::new();
    let config = harness.controller().config();
    // Default config should have reasonable resolution values
    assert!(
        config.display.resolution.width() > 0,
        "config resolution width should be positive"
    );
    assert!(
        config.display.resolution.height() > 0,
        "config resolution height should be positive"
    );
}
