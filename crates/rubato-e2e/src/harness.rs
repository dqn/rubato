//! E2E test harness providing a MainController with RecordingAudioDriver
//! and deterministic (frozen) timing.

use std::sync::{Arc, Mutex};

use rubato_audio::recording_audio_driver::{AudioEvent, RecordingAudioDriver};
use rubato_audio::shared_recording_audio_driver::SharedRecordingAudioDriver;
use rubato_core::config::Config;
use rubato_core::main_controller::{MainController, StateFactory};
use rubato_core::player_config::PlayerConfig;
use rubato_types::main_state_type::MainStateType;

/// One frame at 60 fps in microseconds (1_000_000 / 60 = 16_667, truncated).
pub const FRAME_DURATION_US: i64 = 16_667;

/// E2E test harness wrapping a MainController with deterministic timing.
///
/// The internal TimerManager is frozen so `update()` never advances time
/// from the wall clock. Use `step_frame()`, `step_frames()`, or
/// `set_time()` to control the current time explicitly.
pub struct E2eHarness {
    controller: MainController,
    audio_handle: Arc<Mutex<RecordingAudioDriver>>,
}

impl E2eHarness {
    /// Create a new harness with a RecordingAudioDriver and frozen timer.
    ///
    /// The MainController is constructed with default Config and PlayerConfig.
    /// The timer is frozen at time 0.
    pub fn new() -> Self {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut controller = MainController::new(None, config, player, None, false);

        // Inject shared recording audio driver
        let shared_driver = SharedRecordingAudioDriver::new();
        let audio_handle = shared_driver.inner();
        controller.set_audio_driver(Box::new(shared_driver));

        // Freeze timer so wall-clock time does not advance
        controller.timer_mut().frozen = true;
        controller.timer_mut().set_now_micro_time(0);

        Self {
            controller,
            audio_handle,
        }
    }

    // ============================================================
    // Controller access
    // ============================================================

    /// Access the MainController immutably.
    pub fn controller(&self) -> &MainController {
        &self.controller
    }

    /// Access the MainController mutably.
    pub fn controller_mut(&mut self) -> &mut MainController {
        &mut self.controller
    }

    // ============================================================
    // Audio
    // ============================================================

    /// Return a snapshot of all recorded audio events.
    pub fn audio_events(&self) -> Vec<AudioEvent> {
        self.audio_handle.lock().unwrap().events().to_vec()
    }

    /// Clear the recorded audio events.
    pub fn clear_audio_events(&self) {
        self.audio_handle.lock().unwrap().clear_events();
    }

    /// Execute a closure with a mutable reference to the RecordingAudioDriver.
    pub fn with_recording_driver<F, R>(&self, f: F) -> R
    where
        F: FnOnce(&mut RecordingAudioDriver) -> R,
    {
        let mut guard = self.audio_handle.lock().unwrap();
        f(&mut guard)
    }

    // ============================================================
    // Time control
    // ============================================================

    /// Step the timer forward by one frame (16,667 microseconds at 60 fps).
    pub fn step_frame(&mut self) {
        let current = self.controller.timer().now_micro_time();
        self.controller
            .timer_mut()
            .set_now_micro_time(current + FRAME_DURATION_US);
    }

    /// Step the timer forward by `n` frames.
    pub fn step_frames(&mut self, n: usize) {
        let current = self.controller.timer().now_micro_time();
        self.controller
            .timer_mut()
            .set_now_micro_time(current + FRAME_DURATION_US * n as i64);
    }

    /// Set the current time directly (microseconds from the state start).
    pub fn set_time(&mut self, time_us: i64) {
        self.controller.timer_mut().set_now_micro_time(time_us);
    }

    /// Return the current frozen time in microseconds.
    pub fn current_time_us(&self) -> i64 {
        self.controller.timer().now_micro_time()
    }

    // ============================================================
    // State factory & transitions (Phase 4a/4b)
    // ============================================================

    /// Set a custom state factory for the harness.
    pub fn with_state_factory(mut self, factory: Box<dyn StateFactory>) -> Self {
        self.controller.set_state_factory(factory);
        self
    }

    /// Trigger a state transition.
    pub fn change_state(&mut self, state: MainStateType) {
        self.controller.change_state(state);
    }

    /// Return the current state type (None if no state is active).
    pub fn current_state_type(&self) -> Option<MainStateType> {
        self.controller.current_state_type()
    }

    /// Render one frame: advance timer by 1 frame, then call controller.render().
    pub fn render_frame(&mut self) {
        self.step_frame();
        self.controller.render();
    }

    /// Render `n` frames.
    pub fn render_frames(&mut self, n: usize) {
        for _ in 0..n {
            self.render_frame();
        }
    }

    /// Render frames until predicate returns true, up to `max_frames`.
    /// Returns the number of frames rendered.
    pub fn render_until<F>(&mut self, predicate: F, max_frames: usize) -> usize
    where
        F: Fn(&E2eHarness) -> bool,
    {
        for i in 0..max_frames {
            if predicate(self) {
                return i;
            }
            self.render_frame();
        }
        max_frames
    }
}

impl Default for E2eHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn harness_starts_at_time_zero() {
        let harness = E2eHarness::new();
        assert_eq!(harness.current_time_us(), 0);
    }

    #[test]
    fn step_frame_advances_by_one_frame() {
        let mut harness = E2eHarness::new();
        harness.step_frame();
        assert_eq!(harness.current_time_us(), FRAME_DURATION_US);
    }

    #[test]
    fn step_frames_advances_by_n_frames() {
        let mut harness = E2eHarness::new();
        harness.step_frames(3);
        assert_eq!(harness.current_time_us(), FRAME_DURATION_US * 3);
    }

    #[test]
    fn set_time_overrides_current_time() {
        let mut harness = E2eHarness::new();
        harness.set_time(500_000);
        assert_eq!(harness.current_time_us(), 500_000);
    }

    #[test]
    fn frozen_timer_does_not_advance_on_update() {
        let mut harness = E2eHarness::new();
        harness.set_time(1_000);
        harness.controller_mut().timer_mut().update();
        assert_eq!(harness.current_time_us(), 1_000);
    }

    #[test]
    fn controller_has_audio_driver() {
        let harness = E2eHarness::new();
        assert!(harness.controller().audio_processor().is_some());
    }

    #[test]
    fn audio_events_captures_play_path() {
        let mut harness = E2eHarness::new();

        harness
            .controller_mut()
            .audio_processor_mut()
            .unwrap()
            .play_path("test.ogg", 1.0, false);

        let events = harness.audio_events();
        assert_eq!(events.len(), 1);
        assert_eq!(
            events[0],
            AudioEvent::PlayPath {
                path: "test.ogg".to_string(),
                volume: 1.0,
                loop_play: false,
            }
        );
    }

    #[test]
    fn clear_audio_events_works() {
        let mut harness = E2eHarness::new();

        harness
            .controller_mut()
            .audio_processor_mut()
            .unwrap()
            .play_path("a.wav", 1.0, false);

        assert_eq!(harness.audio_events().len(), 1);
        harness.clear_audio_events();
        assert!(harness.audio_events().is_empty());
    }

    #[test]
    fn with_recording_driver_provides_access() {
        let mut harness = E2eHarness::new();

        harness
            .controller_mut()
            .audio_processor_mut()
            .unwrap()
            .play_path("bgm.ogg", 0.8, false);

        let count = harness.with_recording_driver(|driver| driver.play_path_count());
        assert_eq!(count, 1);
    }

    #[test]
    fn current_state_type_none_without_factory() {
        let harness = E2eHarness::new();
        assert_eq!(harness.current_state_type(), None);
    }

    #[test]
    fn render_until_stops_on_predicate() {
        let mut harness = E2eHarness::new();
        let target_time = FRAME_DURATION_US * 5;
        let frames = harness.render_until(|h| h.current_time_us() >= target_time, 100);
        assert!(frames <= 5);
        assert!(harness.current_time_us() >= target_time);
    }
}
