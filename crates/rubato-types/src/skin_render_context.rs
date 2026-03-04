use crate::main_state_type::MainStateType;
use crate::timer_access::TimerAccess;

/// Extended context for skin rendering that provides timer access plus
/// additional capabilities (event execution, state changes, audio, timers).
///
/// Replaces the 5 no-op methods that were on skin's MainState trait, enabling
/// proper delegation when MainController context is available during rendering.
///
/// All methods have default no-op implementations for adapters that only carry
/// timer data (e.g., TimerOnlyMainState).
pub trait SkinRenderContext: TimerAccess {
    /// Execute a custom skin event by ID with arguments.
    fn execute_event(&mut self, _id: i32, _arg1: i32, _arg2: i32) {
        // default no-op
    }

    /// Change the application state (e.g., to CONFIG, SKINCONFIG).
    fn change_state(&mut self, _state: MainStateType) {
        // default no-op
    }

    /// Set a timer value by ID (micro-seconds).
    fn set_timer_micro(&mut self, _timer_id: i32, _micro_time: i64) {
        // default no-op
    }

    /// Play an audio file at the given path with volume and loop flag.
    fn audio_play(&mut self, _path: &str, _volume: f32, _is_loop: bool) {
        // default no-op
    }

    /// Stop an audio file at the given path.
    fn audio_stop(&mut self, _path: &str) {
        // default no-op
    }

    /// Returns the current main state type (e.g., Play, MusicSelect, Result).
    /// Used by skin adapters to answer state-specific queries like `is_bms_player()`.
    fn current_state_type(&self) -> Option<MainStateType> {
        None
    }

    /// Returns the recent judge timing offsets (milliseconds).
    /// 100-element circular buffer. Used by SkinTimingVisualizer and SkinHitErrorVisualizer.
    fn get_recent_judges(&self) -> &[i64] {
        &[]
    }

    /// Returns the current write index into the recent judges circular buffer.
    fn get_recent_judges_index(&self) -> usize {
        0
    }
}
