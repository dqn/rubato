// GdxInput/GdxGraphics replacements using SharedKeyState from winit_input_bridge.

use crate::winit_input_bridge::SharedKeyState;
use std::sync::Mutex;

/// Global shared key state. When set (via `set_shared_key_state()`),
/// GdxInput and GdxGraphics read from this instead of returning stub defaults.
///
/// Uses Mutex<Option<>> instead of OnceLock to allow replacement (needed for tests).
static SHARED_KEY_STATE: Mutex<Option<SharedKeyState>> = Mutex::new(None);

/// Set the global shared key state. Can be called multiple times (later calls replace earlier).
pub fn set_shared_key_state(state: SharedKeyState) {
    let mut guard = SHARED_KEY_STATE.lock().unwrap();
    *guard = Some(state);
}

/// Get the global shared key state, if set.
pub fn get_shared_key_state() -> Option<SharedKeyState> {
    let guard = SHARED_KEY_STATE.lock().unwrap();
    guard.clone()
}

/// Replacement for Gdx.input — reads from SharedKeyState when available.
pub struct GdxInput;

impl GdxInput {
    pub fn is_key_pressed(keycode: i32) -> bool {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.is_key_pressed(keycode)
        } else {
            false
        }
    }

    pub fn get_x() -> i32 {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.get_mouse_x()
        } else {
            0
        }
    }

    pub fn get_y() -> i32 {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.get_mouse_y()
        } else {
            0
        }
    }

    pub fn set_cursor_position(x: i32, y: i32) {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.set_cursor_position(x, y);
        }
    }
}

/// Replacement for Gdx.graphics — reads window size from SharedKeyState when available.
pub struct GdxGraphics;

impl GdxGraphics {
    pub fn get_width() -> i32 {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.get_window_width()
        } else {
            1920
        }
    }

    pub fn get_height() -> i32 {
        let guard = SHARED_KEY_STATE.lock().unwrap();
        if let Some(ref state) = *guard {
            state.get_window_height()
        } else {
            1080
        }
    }
}
