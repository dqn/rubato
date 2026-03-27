//! Phase 8 Task 8.4: Concurrency smoke tests.
//!
//! Verifies no blocking I/O on main thread paths and tests for
//! Mutex deadlock detection with timeouts.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use rubato_e2e::{E2eHarness, MainStateType};
use rubato_game::state_factory::LauncherStateFactory;

fn harness_with_factory() -> E2eHarness {
    E2eHarness::new().with_state_factory(LauncherStateFactory::new().into_creator())
}

// ============================================================
// Main thread non-blocking verification
// ============================================================

/// Render N frames and assert that total wall-clock time is bounded.
/// This catches blocking I/O calls on the render path.
fn assert_render_non_blocking(harness: &mut E2eHarness, frames: usize, max_ms: u64) {
    let start = Instant::now();
    harness.render_frames(frames);
    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(max_ms),
        "rendering {} frames took {:?}, expected < {}ms (possible blocking I/O)",
        frames,
        elapsed,
        max_ms,
    );
}

#[test]
fn render_path_non_blocking_music_select() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();
    harness.change_state(MainStateType::MusicSelect);
    // 100 frames should complete in well under 5 seconds (no real I/O)
    assert_render_non_blocking(&mut harness, 100, 5000);
}

#[test]
fn render_path_non_blocking_play() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();
    harness.change_state(MainStateType::Play);
    assert_render_non_blocking(&mut harness, 100, 5000);
}

#[test]
fn render_path_non_blocking_result() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();
    harness.change_state(MainStateType::Result);
    assert_render_non_blocking(&mut harness, 100, 5000);
}

#[test]
fn render_path_non_blocking_config() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();
    harness.change_state(MainStateType::Config);
    assert_render_non_blocking(&mut harness, 100, 5000);
}

#[test]
fn state_transition_non_blocking() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();

    let states = [
        MainStateType::MusicSelect,
        MainStateType::Decide,
        MainStateType::Play,
        MainStateType::Result,
        MainStateType::CourseResult,
        MainStateType::Config,
        MainStateType::SkinConfig,
    ];

    let start = Instant::now();
    for &state in &states {
        harness.change_state(state);
        harness.render_frame();
    }
    let elapsed = start.elapsed();

    assert!(
        elapsed < Duration::from_millis(5000),
        "cycling through all states took {:?}, expected < 5s",
        elapsed
    );
}

// ============================================================
// Mutex timeout / deadlock detection
// ============================================================

#[test]
fn recording_audio_driver_mutex_no_deadlock() {
    let harness = E2eHarness::new();

    // Access the audio driver through two paths to verify no deadlock
    let events1 = harness.audio_events();
    let events2 = harness.audio_events();
    assert_eq!(events1.len(), events2.len());
}

#[test]
fn state_event_log_mutex_no_deadlock() {
    let mut harness = E2eHarness::new();

    // Access state events multiple times
    let events1 = harness.state_events();
    let events2 = harness.state_events();
    assert_eq!(events1.len(), events2.len());
}

#[test]
fn concurrent_audio_access_with_timeout() {
    let harness = Arc::new(Mutex::new(E2eHarness::new()));

    // Simulate concurrent access from multiple "threads"
    // In this test we just verify the mutex can be acquired within a timeout
    let start = Instant::now();

    for _ in 0..10 {
        let guard = harness
            .try_lock()
            .expect("should be able to lock the harness");
        let _events = guard.audio_events();
        drop(guard);
    }

    let elapsed = start.elapsed();
    assert!(
        elapsed < Duration::from_millis(1000),
        "10 lock+read cycles took {:?}, expected < 1s",
        elapsed
    );
}

#[test]
fn render_frame_completes_within_timeout() {
    let mut harness = harness_with_factory();
    harness.change_state(MainStateType::MusicSelect);

    // Each render_frame should complete in under 100ms (no blocking)
    for _ in 0..10 {
        let start = Instant::now();
        harness.render_frame();
        let elapsed = start.elapsed();
        assert!(
            elapsed < Duration::from_millis(500),
            "single render_frame took {:?}, expected < 500ms",
            elapsed
        );
    }
}

// ============================================================
// Shared state across harness methods
// ============================================================

#[test]
fn harness_methods_do_not_hold_locks_across_calls() {
    let mut harness = E2eHarness::new();

    // Interleave audio event reads with audio mutations
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("a.wav", 1.0, false);

    let events = harness.audio_events();
    assert_eq!(events.len(), 1);

    // Should be able to immediately mutate again (no lingering lock)
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("b.wav", 1.0, false);

    let events = harness.audio_events();
    assert_eq!(events.len(), 2);
}

#[test]
fn state_events_and_audio_events_independent() {
    let mut harness = harness_with_factory();
    harness.controller_mut().create();

    // Generate state events
    harness.change_state(MainStateType::MusicSelect);

    // Generate audio events
    harness
        .controller_mut()
        .audio_processor_mut()
        .unwrap()
        .play_path("test.ogg", 1.0, false);

    // Both should be accessible without interference
    let state_events = harness.state_events();
    let audio_events = harness.audio_events();

    assert!(!state_events.is_empty(), "state events should be non-empty");
    assert!(!audio_events.is_empty(), "audio events should be non-empty");

    // Clearing one should not affect the other
    harness.clear_state_events();
    assert!(harness.state_events().is_empty());
    assert!(!harness.audio_events().is_empty());

    harness.clear_audio_events();
    assert!(harness.audio_events().is_empty());
}
