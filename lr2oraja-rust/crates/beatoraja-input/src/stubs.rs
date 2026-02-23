// Config types re-exported from beatoraja-types
pub use beatoraja_types::config::Config;
pub use beatoraja_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, MidiInputType, MouseScratchConfig,
    PlayModeConfig,
};
pub use beatoraja_types::player_config::PlayerConfig;
pub use beatoraja_types::resolution::Resolution;

// GdxInput and GdxGraphics — delegate to gdx_compat which reads SharedKeyState.
pub use crate::gdx_compat::{GdxGraphics, GdxInput};

/// Stub for Controller (com.badlogic.gdx.controllers.Controller)
pub struct Controller {
    name: String,
    pub axis_state: Vec<f32>,
    pub button_state: Vec<bool>,
}

impl Controller {
    pub fn new(name: String) -> Self {
        Self {
            name,
            axis_state: Vec::new(),
            button_state: Vec::new(),
        }
    }

    /// Creates a controller with pre-initialized state arrays.
    pub fn with_state(name: String, num_buttons: usize, num_axes: usize) -> Self {
        Self {
            name,
            axis_state: vec![0.0; num_axes],
            button_state: vec![false; num_buttons],
        }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_button(&self, button: i32) -> bool {
        if button < 0 || button as usize >= self.button_state.len() {
            return false;
        }
        self.button_state[button as usize]
    }

    pub fn get_axis(&self, axis: i32) -> f32 {
        if axis < 0 || axis as usize >= self.axis_state.len() {
            return 0.0;
        }
        self.axis_state[axis as usize]
    }
}

/// Stub for libGDX Keys constants
pub struct Keys;

#[allow(non_upper_case_globals, dead_code)]
impl Keys {
    pub const Z: i32 = 54;
    pub const S: i32 = 47;
    pub const X: i32 = 52;
    pub const D: i32 = 32;
    pub const C: i32 = 31;
    pub const F: i32 = 34;
    pub const V: i32 = 50;
    pub const SHIFT_LEFT: i32 = 59;
    pub const CONTROL_LEFT: i32 = 129;
    pub const COMMA: i32 = 55;
    pub const L: i32 = 40;
    pub const PERIOD: i32 = 56;
    pub const SEMICOLON: i32 = 74;
    pub const SLASH: i32 = 76;
    pub const APOSTROPHE: i32 = 75;
    pub const BACKSLASH: i32 = 73;
    pub const SHIFT_RIGHT: i32 = 60;
    pub const CONTROL_RIGHT: i32 = 130;
    pub const Q: i32 = 45;
    pub const W: i32 = 51;
    pub const NUM_0: i32 = 7;
    pub const NUM_1: i32 = 8;
    pub const NUM_2: i32 = 9;
    pub const NUM_3: i32 = 10;
    pub const NUM_4: i32 = 11;
    pub const NUM_5: i32 = 12;
    pub const NUM_6: i32 = 13;
    pub const NUM_7: i32 = 14;
    pub const NUM_8: i32 = 15;
    pub const NUM_9: i32 = 16;
    pub const F1: i32 = 244;
    pub const F2: i32 = 245;
    pub const F3: i32 = 246;
    pub const F4: i32 = 247;
    pub const F5: i32 = 248;
    pub const F6: i32 = 249;
    pub const F7: i32 = 250;
    pub const F8: i32 = 251;
    pub const F9: i32 = 252;
    pub const F10: i32 = 253;
    pub const F11: i32 = 254;
    pub const F12: i32 = 255;
    pub const UP: i32 = 19;
    pub const DOWN: i32 = 20;
    pub const LEFT: i32 = 21;
    pub const RIGHT: i32 = 22;
    pub const ENTER: i32 = 66;
    pub const INSERT: i32 = 133;
    pub const FORWARD_DEL: i32 = 112;
    pub const ESCAPE: i32 = 111;
    pub const ALT_LEFT: i32 = 57;
    pub const ALT_RIGHT: i32 = 58;
}
