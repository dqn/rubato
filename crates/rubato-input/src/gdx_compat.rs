// GdxInput/GdxGraphics replacements using SharedKeyState from winit_input_bridge.

use crate::winit_input_bridge::SharedKeyState;
use rubato_types::sync_utils::lock_or_recover;
use std::sync::Mutex;

/// Global shared key state. When set (via `set_shared_key_state()`),
/// deprecated GdxInput and GdxGraphics read from this.
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

// ============================================================
// Direct SharedKeyState query functions
// ============================================================

/// Query whether a key is pressed via the given SharedKeyState.
pub fn is_key_pressed(key_state: &SharedKeyState, keycode: i32) -> bool {
    key_state.is_key_pressed(keycode)
}

/// Get mouse X position from the given SharedKeyState.
pub fn get_x(key_state: &SharedKeyState) -> i32 {
    key_state.mouse_x()
}

/// Get mouse Y position from the given SharedKeyState.
pub fn get_y(key_state: &SharedKeyState) -> i32 {
    key_state.mouse_y()
}

/// Set cursor position via the given SharedKeyState.
pub fn set_cursor_position(key_state: &SharedKeyState, x: i32, y: i32) {
    key_state.set_cursor_position(x, y);
}

/// Query whether a mouse button is pressed via the given SharedKeyState.
pub fn is_button_pressed(key_state: &SharedKeyState, button: i32) -> bool {
    key_state.is_mouse_button_pressed(button)
}

/// Drain accumulated scroll delta from the given SharedKeyState.
pub fn drain_scroll(key_state: &SharedKeyState) -> (f32, f32) {
    key_state.drain_scroll()
}

/// Drain mouse dragged flag from the given SharedKeyState.
pub fn drain_mouse_dragged(key_state: &SharedKeyState) -> bool {
    key_state.drain_mouse_dragged()
}

/// Get window width from the given SharedKeyState.
pub fn get_width(key_state: &SharedKeyState) -> i32 {
    key_state.window_width()
}

/// Get window height from the given SharedKeyState.
pub fn get_height(key_state: &SharedKeyState) -> i32 {
    key_state.window_height()
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

    #[test]
    fn test_direct_key_state_functions() {
        let state = SharedKeyState::new();
        assert!(!is_key_pressed(&state, 54)); // Keys::Z
        state.set_key_pressed(54, true);
        assert!(is_key_pressed(&state, 54));

        assert_eq!(get_x(&state), 0);
        assert_eq!(get_y(&state), 0);
        state.set_mouse_position(100, 200);
        assert_eq!(get_x(&state), 100);
        assert_eq!(get_y(&state), 200);

        assert_eq!(get_width(&state), 1920);
        assert_eq!(get_height(&state), 1080);
        state.set_window_size(800, 600);
        assert_eq!(get_width(&state), 800);
        assert_eq!(get_height(&state), 600);
    }

    #[test]
    fn test_direct_mouse_button_functions() {
        let state = SharedKeyState::new();
        assert!(!is_button_pressed(&state, 0));
        state.set_mouse_button(0, true);
        assert!(is_button_pressed(&state, 0));
    }

    #[test]
    fn test_direct_scroll_functions() {
        let state = SharedKeyState::new();
        state.add_scroll(1.0, 2.0);
        let (dx, dy) = drain_scroll(&state);
        assert_eq!(dx, 1.0);
        assert_eq!(dy, 2.0);
        // Second drain should return zeros
        let (dx2, dy2) = drain_scroll(&state);
        assert_eq!(dx2, 0.0);
        assert_eq!(dy2, 0.0);
    }

    #[test]
    fn test_direct_mouse_dragged_functions() {
        let state = SharedKeyState::new();
        assert!(!drain_mouse_dragged(&state));
        state.set_mouse_dragged(true);
        assert!(drain_mouse_dragged(&state));
        // Second drain should return false
        assert!(!drain_mouse_dragged(&state));
    }
}
