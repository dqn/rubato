//! Read-only snapshot of input state for the current frame.
//!
//! Captures all input state from BMSPlayerInputProcessor at a point in time,
//! allowing states to read input without holding a reference to the processor.

use std::collections::HashMap;

use crate::key_command::KeyCommand;
use crate::keyboard_input_processor::ControlKeys;

/// Read-only snapshot of input state for the current frame.
///
/// Built by `BMSPlayerInputProcessor::build_snapshot()` once per frame,
/// then passed to `MainState::sync_input_snapshot()` so states can read
/// input without depending on the processor type.
#[derive(Clone)]
pub struct InputSnapshot {
    /// Per-key pressed state (256 keys).
    pub key_state: [bool; 256],
    /// Per-key last change timestamp in microseconds.
    pub key_changed_time: [i64; 256],
    /// START button pressed.
    pub start_pressed: bool,
    /// SELECT button pressed.
    pub select_pressed: bool,
    /// Mouse position (resolution-scaled).
    pub mouse_x: i32,
    pub mouse_y: i32,
    /// Mouse button ID (e.g. 0 = left).
    pub mouse_button: i32,
    /// Mouse button was pressed this frame.
    pub mouse_pressed: bool,
    /// Mouse was dragged this frame.
    pub mouse_dragged: bool,
    /// Accumulated scroll amounts since last reset.
    pub scroll_x: f32,
    pub scroll_y: f32,
    /// Per-key analog input flags.
    pub is_analog: [bool; 256],
    /// Per-key analog diff values (ticks since last reset).
    pub analog_diff: [f32; 256],
    /// KeyCommand activations detected this frame.
    ///
    /// Building the snapshot consumes key presses via `is_activated()`,
    /// so each command appears at most once per frame.
    pub activated_commands: Vec<KeyCommand>,
    /// Control key states (non-consuming read of current pressed state).
    pub control_key_states: HashMap<ControlKeys, bool>,
}

impl Default for InputSnapshot {
    fn default() -> Self {
        Self {
            key_state: [false; 256],
            key_changed_time: [i64::MIN; 256],
            start_pressed: false,
            select_pressed: false,
            mouse_x: 0,
            mouse_y: 0,
            mouse_button: 0,
            mouse_pressed: false,
            mouse_dragged: false,
            scroll_x: 0.0,
            scroll_y: 0.0,
            is_analog: [false; 256],
            analog_diff: [0.0; 256],
            activated_commands: Vec::new(),
            control_key_states: HashMap::new(),
        }
    }
}
