// GdxInput/GdxGraphics replacements using SharedKeyState from winit_input_bridge.

use crate::winit_input_bridge::SharedKeyState;
use rubato_types::sync_utils::lock_or_recover;
use std::sync::Mutex;

/// Global shared key state. When set (via `set_shared_key_state()`),
/// GdxInput and GdxGraphics read from this instead of returning stub defaults.
///
/// Uses Mutex<Option<>> instead of OnceLock to allow replacement (needed for tests).
static SHARED_KEY_STATE: Mutex<Option<SharedKeyState>> = Mutex::new(None);

/// Set the global shared key state. Can be called multiple times (later calls replace earlier).
pub fn set_shared_key_state(state: SharedKeyState) {
    let mut guard = lock_or_recover(&SHARED_KEY_STATE);
    *guard = Some(state);
}

/// Get the global shared key state, if set.
pub fn get_shared_key_state() -> Option<SharedKeyState> {
    let guard = lock_or_recover(&SHARED_KEY_STATE);
    guard.clone()
}

/// Clear the global shared key state, resetting it to None.
pub fn clear_shared_key_state() {
    let mut guard = lock_or_recover(&SHARED_KEY_STATE);
    *guard = None;
}

/// RAII guard that clears the shared key state on drop.
/// Use in tests to ensure cleanup even on panic.
#[cfg(test)]
pub struct SharedKeyStateGuard;

#[cfg(test)]
impl Drop for SharedKeyStateGuard {
    fn drop(&mut self) {
        clear_shared_key_state();
    }
}

/// Set shared key state and return a guard that clears it on drop.
#[cfg(test)]
pub fn set_shared_key_state_guarded(state: SharedKeyState) -> SharedKeyStateGuard {
    set_shared_key_state(state);
    SharedKeyStateGuard
}

/// Replacement for Gdx.input — reads from SharedKeyState when available.
pub struct GdxInput;

impl GdxInput {
    pub fn is_key_pressed(keycode: i32) -> bool {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.is_key_pressed(keycode)
        } else {
            false
        }
    }

    pub fn get_x() -> i32 {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.mouse_x()
        } else {
            0
        }
    }

    pub fn get_y() -> i32 {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.mouse_y()
        } else {
            0
        }
    }

    pub fn set_cursor_position(x: i32, y: i32) {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.set_cursor_position(x, y);
        }
    }

    pub fn is_button_pressed(button: i32) -> bool {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.is_mouse_button_pressed(button)
        } else {
            false
        }
    }

    pub fn drain_scroll() -> (f32, f32) {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.drain_scroll()
        } else {
            (0.0, 0.0)
        }
    }
}

/// Replacement for Gdx.graphics — reads window size from SharedKeyState when available.
pub struct GdxGraphics;

impl GdxGraphics {
    pub fn get_width() -> i32 {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.window_width()
        } else {
            1920
        }
    }

    pub fn get_height() -> i32 {
        let guard = SHARED_KEY_STATE
            .lock()
            .expect("SHARED_KEY_STATE lock poisoned");
        if let Some(ref state) = *guard {
            state.window_height()
        } else {
            1080
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_guard_clears_state_on_drop() {
        // State should be None initially (or from a previous test).
        // Set it via the guarded helper inside a scope.
        {
            let _guard = set_shared_key_state_guarded(SharedKeyState::new());
            assert!(
                get_shared_key_state().is_some(),
                "shared key state should be Some while guard is alive"
            );
        }
        // After the guard is dropped, state must be None.
        assert!(
            get_shared_key_state().is_none(),
            "shared key state should be None after guard is dropped"
        );
    }

    #[test]
    fn test_clear_shared_key_state() {
        set_shared_key_state(SharedKeyState::new());
        assert!(get_shared_key_state().is_some());
        clear_shared_key_state();
        assert!(get_shared_key_state().is_none());
    }
}
