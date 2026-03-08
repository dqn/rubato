use crate::lane_renderer::LaneRenderer;
use rubato_core::bms_player_mode::Mode as AutoplayMode;

/// Bundles the external input state needed by ControlInputProcessor,
/// avoiding the need for the processor to hold references to the parent player.
/// Modeled after KeyInputProccessor's InputContext pattern.
pub struct ControlInputContext<'a> {
    /// Mutable reference to the lane renderer for lane cover / hispeed / duration changes.
    pub lanerender: &'a mut LaneRenderer,
    /// Whether the START button is currently pressed (from BMSPlayerInputProcessor).
    pub start_pressed: bool,
    /// Whether the SELECT button is currently pressed (from BMSPlayerInputProcessor).
    pub select_pressed: bool,
    /// Control key states: UP, DOWN, ESCAPE, NUM1-4
    pub control_key_up: bool,
    pub control_key_down: bool,
    pub control_key_escape_pressed: bool,
    pub control_key_num1: bool,
    pub control_key_num2: bool,
    pub control_key_num3: bool,
    pub control_key_num4: bool,
    /// Key states array (indexed by key ID) from BMSPlayerInputProcessor.
    pub key_states: &'a [bool],
    /// Mouse scroll value (from BMSPlayerInputProcessor.getScroll()).
    pub scroll: i32,
    /// Analog input queries — closures that read from BMSPlayerInputProcessor.
    /// Returns true if key `i` is analog input.
    pub is_analog: &'a [bool],
    /// Analog diff and reset function.
    /// Takes (key_index, ms_tolerance) -> diff_ticks.
    pub analog_diff_and_reset: &'a mut dyn FnMut(usize, i32) -> i32,
    /// Whether TIMER_PLAY is on (from timer manager).
    pub is_timer_play_on: bool,
    /// Whether all notes have been passed (from BMSPlayer.isNoteEnd()).
    pub is_note_end: bool,
    /// Whether windowHold is enabled (from PlayerConfig).
    pub window_hold: bool,
    /// The autoplay mode (Play, Practice, Autoplay, Replay).
    pub autoplay_mode: AutoplayMode,
    /// Current time in milliseconds (System.currentTimeMillis() equivalent).
    pub now_millis: i64,
}

/// Context for scratch-input-driven value changes (cover/duration).
pub struct ScratchInputContext<'a> {
    pub key: usize,
    pub up: bool,
    pub key_states: &'a [bool],
    pub is_analog: &'a [bool],
    pub now_millis: i64,
}

/// Actions produced by ControlInputProcessor.input() that need to be
/// applied by the caller (BMSPlayer).
#[derive(Debug, Default)]
pub struct ControlInputResult {
    /// Whether play should be stopped (START+SELECT held or ESC pressed or note end + start/select).
    pub stop_play: bool,
    /// Play speed to set (only for autoplay/replay modes). None means no change.
    pub play_speed: Option<i32>,
    /// Whether to clear start_pressed on the input processor.
    pub clear_start: bool,
    /// Whether to clear select_pressed on the input processor.
    pub clear_select: bool,
    /// Whether to reset scroll on the input processor.
    pub reset_scroll: bool,
}
