//! E2E test harness providing a MainController with RecordingAudioDriver
//! and deterministic (frozen) timing.

use std::sync::{Arc, Mutex};

use rubato_audio::recording_audio_driver::{AudioEvent, RecordingAudioDriver};
use rubato_audio::shared_recording_audio_driver::SharedRecordingAudioDriver;
use rubato_core::config::Config;
use rubato_core::main_controller::{MainController, StateFactory};
use rubato_core::player_config::PlayerConfig;
use rubato_core::player_resource::PlayerResource;
use rubato_render::sprite_batch::CapturedDrawQuad;
use rubato_types::main_state_type::MainStateType;
use rubato_types::state_event::StateEvent;

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
    state_event_log: Arc<Mutex<Vec<StateEvent>>>,
    /// Monotonically increasing input gate time (milliseconds) used to override
    /// the wall-clock check in `MainController::render()`. Incremented on each
    /// `render_frame()` call so that the `time > prevtime` gate always passes,
    /// regardless of actual wall-clock advancement.
    input_gate_time_ms: i64,
}

impl E2eHarness {
    fn sync_current_state_timer_to_controller(&mut self) {
        let now = self.controller.timer().now_micro_time();
        if let Some(current) = self.controller.current_state_mut() {
            let timer = &mut current.main_state_data_mut().timer;
            timer.frozen = true;
            timer.set_now_micro_time(now);
        }
    }

    fn instrument_controller(mut controller: MainController) -> Self {
        // Inject shared recording audio driver
        let shared_driver = SharedRecordingAudioDriver::new();
        let audio_handle = shared_driver.inner();
        controller.set_audio_driver(Box::new(shared_driver));

        // Freeze timer so wall-clock time does not advance
        controller.timer_mut().frozen = true;
        controller.timer_mut().set_now_micro_time(0);

        // Wire up state event log for observability
        let state_event_log = Arc::new(Mutex::new(Vec::new()));
        controller.set_state_event_log(Arc::clone(&state_event_log));

        let mut harness = Self {
            controller,
            audio_handle,
            state_event_log,
            input_gate_time_ms: 0,
        };
        harness.sync_current_state_timer_to_controller();
        harness
    }

    /// Create a new harness with a RecordingAudioDriver and frozen timer.
    ///
    /// The MainController is constructed with default Config and PlayerConfig.
    /// The timer is frozen at time 0.
    pub fn new() -> Self {
        Self::new_with_config_player(Config::default(), PlayerConfig::default())
    }

    /// Create a new harness with a custom PlayerConfig.
    pub fn new_with_player_config(player: PlayerConfig) -> Self {
        Self::new_with_config_player(Config::default(), player)
    }

    /// Wrap an existing MainController with E2E instrumentation.
    pub fn from_controller(controller: MainController) -> Self {
        Self::instrument_controller(controller)
    }

    /// Create a new harness with custom Config and PlayerConfig.
    pub fn new_with_config_player(config: Config, player: PlayerConfig) -> Self {
        let controller = MainController::new(None, config, player, None, false);
        Self::instrument_controller(controller)
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
        self.sync_current_state_timer_to_controller();
    }

    /// Step the timer forward by `n` frames.
    pub fn step_frames(&mut self, n: usize) {
        let current = self.controller.timer().now_micro_time();
        self.controller
            .timer_mut()
            .set_now_micro_time(current + FRAME_DURATION_US * n as i64);
        self.sync_current_state_timer_to_controller();
    }

    /// Set the current time directly (microseconds from the state start).
    pub fn set_time(&mut self, time_us: i64) {
        self.controller.timer_mut().set_now_micro_time(time_us);
        self.sync_current_state_timer_to_controller();
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

    /// Install a default PlayerResource on the controller so that
    /// Result / CourseResult state creation does not fall back to MusicSelect.
    pub fn ensure_player_resource(&mut self) {
        if self.controller.player_resource().is_none() {
            self.controller.restore_player_resource(PlayerResource::new(
                Config::default(),
                PlayerConfig::default(),
            ));
        }
    }

    /// Trigger a state transition.
    pub fn change_state(&mut self, state: MainStateType) {
        self.controller.change_state(state);
        self.sync_current_state_timer_to_controller();
    }

    /// Return the current state type (None if no state is active).
    pub fn current_state_type(&self) -> Option<MainStateType> {
        self.controller.current_state_type()
    }

    /// Render one frame: advance timer by 1 frame, then call controller.render().
    ///
    /// Sets an input gate time override before each render() call so the
    /// `time > prevtime` check always passes, regardless of wall-clock speed.
    pub fn render_frame(&mut self) {
        self.step_frame();
        self.input_gate_time_ms += 1;
        self.controller
            .set_input_gate_time_override(self.input_gate_time_ms);
        self.controller.render();
        self.sync_current_state_timer_to_controller();
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

    // ============================================================
    // Input injection (Phase 4c)
    // ============================================================

    /// Inject a key-down event for the given key index.
    pub fn inject_key_down(&mut self, key: i32) {
        let time = self.current_time_us();
        if let Some(input) = self.controller.input_processor_mut() {
            input.set_key_state(key, true, time);
        }
    }

    /// Inject a key-up event for the given key index.
    pub fn inject_key_up(&mut self, key: i32) {
        let time = self.current_time_us();
        if let Some(input) = self.controller.input_processor_mut() {
            input.set_key_state(key, false, time);
        }
    }

    /// Inject a key press: key-down, render n frames, then key-up.
    pub fn inject_key_press(&mut self, key: i32, duration_frames: usize) {
        self.inject_key_down(key);
        self.render_frames(duration_frames);
        self.inject_key_up(key);
    }

    // ============================================================
    // Gameplay state inspection (Phase 4d)
    // ============================================================

    /// Returns the current score data from PlayerResource (if available).
    pub fn score_data(&self) -> Option<&rubato_types::score_data::ScoreData> {
        self.controller.player_resource()?.score_data()
    }

    /// Returns the current groove gauge value for the active gauge type,
    /// or 0.0 if PlayerResource or GrooveGauge is unavailable.
    pub fn gauge_value(&self) -> f32 {
        self.controller
            .player_resource()
            .and_then(|r| r.groove_gauge())
            .map(|g| g.value())
            .unwrap_or(0.0)
    }

    /// Returns whether the player resource has a groove gauge set.
    pub fn has_groove_gauge(&self) -> bool {
        self.controller
            .player_resource()
            .and_then(|r| r.groove_gauge())
            .is_some()
    }

    // ============================================================
    // State event observability (Phase 3)
    // ============================================================

    /// Return a snapshot of all recorded state events.
    pub fn state_events(&self) -> Vec<StateEvent> {
        self.state_event_log.lock().unwrap().clone()
    }

    /// Clear the recorded state events.
    pub fn clear_state_events(&self) {
        self.state_event_log.lock().unwrap().clear();
    }

    /// Assert that the recorded state events contain the given subsequence
    /// in order. Panics with a diff if the expected sequence is not found.
    pub fn assert_event_sequence(&self, expected: &[StateEvent]) {
        let events = self.state_events();
        if expected.is_empty() {
            return;
        }
        let mut expected_idx = 0;
        for event in &events {
            if event == &expected[expected_idx] {
                expected_idx += 1;
                if expected_idx == expected.len() {
                    return;
                }
            }
        }
        panic!(
            "Expected event sequence not found.\n\
             Expected ({} events): {:#?}\n\
             Matched {}/{} events.\n\
             Actual ({} events): {:#?}",
            expected.len(),
            expected,
            expected_idx,
            expected.len(),
            events.len(),
            events,
        );
    }

    // ============================================================
    // Rendering observability (Phase 4)
    // ============================================================

    /// Enable draw capture on the SpriteBatch, recording all draw quads
    /// for GPU-free verification. No-op if no SpriteBatch exists yet.
    pub fn enable_render_capture(&mut self) {
        if let Some(sb) = self.controller.sprite_batch_mut() {
            sb.enable_capture();
        }
    }

    /// Disable draw capture and drop the capture buffer.
    pub fn disable_render_capture(&mut self) {
        if let Some(sb) = self.controller.sprite_batch_mut() {
            sb.disable_capture();
        }
    }

    /// Return a copy of all captured draw quads from the SpriteBatch.
    /// Returns an empty Vec if capture is disabled or no SpriteBatch exists.
    pub fn captured_draw_quads(&self) -> Vec<CapturedDrawQuad> {
        self.controller
            .sprite_batch()
            .map(|sb| sb.captured_quads().to_vec())
            .unwrap_or_default()
    }

    /// Clear the capture buffer without disabling capture.
    pub fn clear_captured_quads(&mut self) {
        if let Some(sb) = self.controller.sprite_batch_mut() {
            sb.clear_captured();
        }
    }

    /// Check if any captured quad has the given texture key.
    pub fn assert_texture_drawn(&self, texture_key: &str) -> bool {
        self.controller
            .sprite_batch()
            .map(|sb| {
                sb.captured_quads()
                    .iter()
                    .any(|q| q.texture_key.as_deref() == Some(texture_key))
            })
            .unwrap_or(false)
    }

    /// Check if any captured quad is at the given position within tolerance.
    pub fn assert_draw_at(&self, x: f32, y: f32, tolerance: f32) -> bool {
        self.controller
            .sprite_batch()
            .map(|sb| {
                sb.captured_quads()
                    .iter()
                    .any(|q| (q.x - x).abs() <= tolerance && (q.y - y).abs() <= tolerance)
            })
            .unwrap_or(false)
    }

    /// Render frames until the given state type is reached, up to `max_frames`.
    /// Returns true if the target state was reached.
    pub fn wait_for_state(&mut self, target: MainStateType, max_frames: usize) -> bool {
        for _ in 0..max_frames {
            if self.current_state_type() == Some(target) {
                return true;
            }
            self.render_frame();
        }
        self.current_state_type() == Some(target)
    }

    // ============================================================
    // Frame state dumper (Phase 6a)
    // ============================================================

    /// Capture a snapshot of the current harness state for debugging.
    pub fn dump_frame_state(&self) -> FrameState {
        FrameState {
            state_type: self.current_state_type(),
            time_us: self.current_time_us(),
            gauge_value: self.gauge_value(),
            audio_event_count: self.audio_events().len(),
            draw_quad_count: self.captured_draw_quads().len(),
            state_event_count: self.state_events().len(),
        }
    }

    // ============================================================
    // Fluent assertion helpers (Phase 6b)
    // ============================================================

    /// Assert that the current state matches the expected type.
    pub fn assert_state(&self, expected: MainStateType) {
        assert_eq!(
            self.current_state_type(),
            Some(expected),
            "expected state {:?}, got {:?}",
            expected,
            self.current_state_type()
        );
    }

    /// Assert that the gauge value is within `[min, max]`.
    pub fn assert_gauge_between(&self, min: f32, max: f32) {
        let g = self.gauge_value();
        assert!(
            g >= min && g <= max,
            "gauge {} not in [{}, {}]",
            g,
            min,
            max
        );
    }

    /// Assert that the current exscore is at least `min_exscore`.
    pub fn assert_score_at_least(&self, min_exscore: i32) {
        if let Some(sd) = self.score_data() {
            assert!(
                sd.exscore() >= min_exscore,
                "exscore {} < minimum {}",
                sd.exscore(),
                min_exscore
            );
        } else {
            panic!(
                "no score data available, expected exscore >= {}",
                min_exscore
            );
        }
    }

    /// Assert that at least `min` audio events have been recorded.
    pub fn assert_audio_event_count_at_least(&self, min: usize) {
        let count = self.audio_events().len();
        assert!(
            count >= min,
            "audio event count {} < minimum {}",
            count,
            min
        );
    }

    /// Render `n` frames and assert no panics occur.
    pub fn assert_no_panics_after_frames(&mut self, n: usize) {
        self.render_frames(n);
    }
}

/// Snapshot of harness state at a point in time, for debugging.
#[derive(Debug)]
pub struct FrameState {
    pub state_type: Option<MainStateType>,
    pub time_us: i64,
    pub gauge_value: f32,
    pub audio_event_count: usize,
    pub draw_quad_count: usize,
    pub state_event_count: usize,
}

impl Default for E2eHarness {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_core::main_controller::StateCreateResult;
    use rubato_core::main_state::MainStateData;
    use rubato_core::timer_manager::TimerManager;

    struct TimerSyncState {
        data: MainStateData,
        state_type: MainStateType,
    }

    impl TimerSyncState {
        fn new(state_type: MainStateType) -> Self {
            Self {
                data: MainStateData::new(TimerManager::new()),
                state_type,
            }
        }
    }

    impl rubato_core::main_state::MainState for TimerSyncState {
        fn state_type(&self) -> Option<MainStateType> {
            Some(self.state_type)
        }

        fn main_state_data(&self) -> &MainStateData {
            &self.data
        }

        fn main_state_data_mut(&mut self) -> &mut MainStateData {
            &mut self.data
        }

        fn create(&mut self) {}

        fn render(&mut self) {}
    }

    struct TimerSyncFactory;

    impl StateFactory for TimerSyncFactory {
        fn create_state(
            &self,
            state_type: MainStateType,
            _controller: &mut MainController,
        ) -> Option<StateCreateResult> {
            Some(StateCreateResult {
                state: Box::new(TimerSyncState::new(state_type)),
                target_score: None,
            })
        }
    }

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
    fn new_with_player_config_uses_custom_player() {
        let mut player = PlayerConfig::default();
        player.name = "ECFN Tester".to_string();

        let harness = E2eHarness::new_with_player_config(player);

        assert_eq!(harness.controller().player_config().name, "ECFN Tester");
    }

    #[test]
    fn from_controller_preserves_existing_player() {
        let mut player = PlayerConfig::default();
        player.name = "MainLoader Player".to_string();

        let controller = MainController::new(None, Config::default(), player, None, false);
        let harness = E2eHarness::from_controller(controller);

        assert_eq!(
            harness.controller().player_config().name,
            "MainLoader Player"
        );
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

    #[test]
    fn step_frame_keeps_current_state_timer_in_sync_when_frozen() {
        let mut harness = E2eHarness::new().with_state_factory(Box::new(TimerSyncFactory));
        harness.controller_mut().create();
        harness.change_state(MainStateType::MusicSelect);

        harness.step_frame();

        let state_time = harness
            .controller()
            .current_state()
            .expect("current state should exist")
            .main_state_data()
            .timer
            .now_micro_time();
        assert_eq!(
            state_time,
            harness.current_time_us(),
            "frozen harness must advance current state's timer together with controller timer"
        );
    }

    #[test]
    fn inject_key_down_sets_key_state() {
        let mut harness = E2eHarness::new();
        harness.inject_key_down(0);
        let pressed = harness
            .controller()
            .input_processor()
            .map(|ip| ip.key_state(0))
            .unwrap_or(false);
        assert!(pressed, "key 0 should be pressed after inject_key_down");
    }

    #[test]
    fn inject_key_up_clears_key_state() {
        let mut harness = E2eHarness::new();
        harness.inject_key_down(0);
        harness.inject_key_up(0);
        let pressed = harness
            .controller()
            .input_processor()
            .map(|ip| ip.key_state(0))
            .unwrap_or(true);
        assert!(!pressed, "key 0 should not be pressed after inject_key_up");
    }

    #[test]
    fn inject_key_press_presses_and_releases() {
        let mut harness = E2eHarness::new();
        harness.inject_key_press(5, 3);
        // After inject_key_press, the key should be released
        let pressed = harness
            .controller()
            .input_processor()
            .map(|ip| ip.key_state(5))
            .unwrap_or(true);
        assert!(
            !pressed,
            "key 5 should be released after inject_key_press completes"
        );
        // Time should have advanced by 3 frames
        assert_eq!(harness.current_time_us(), FRAME_DURATION_US * 3);
    }

    #[test]
    fn captured_draw_quads_empty_without_sprite_batch() {
        let harness = E2eHarness::new();
        // No SpriteBatch exists before create(), so captured quads is empty
        assert!(harness.captured_draw_quads().is_empty());
    }

    #[test]
    fn enable_render_capture_noop_without_sprite_batch() {
        let mut harness = E2eHarness::new();
        // Should not panic when no SpriteBatch exists
        harness.enable_render_capture();
        assert!(harness.captured_draw_quads().is_empty());
    }

    #[test]
    fn assert_texture_drawn_false_without_sprite_batch() {
        let harness = E2eHarness::new();
        assert!(!harness.assert_texture_drawn("anything"));
    }

    #[test]
    fn assert_draw_at_false_without_sprite_batch() {
        let harness = E2eHarness::new();
        assert!(!harness.assert_draw_at(0.0, 0.0, 1.0));
    }

    #[test]
    fn clear_captured_quads_noop_without_sprite_batch() {
        let mut harness = E2eHarness::new();
        // Should not panic when no SpriteBatch exists
        harness.clear_captured_quads();
    }

    #[test]
    fn disable_render_capture_noop_without_sprite_batch() {
        let mut harness = E2eHarness::new();
        // Should not panic when no SpriteBatch exists
        harness.disable_render_capture();
    }
}
