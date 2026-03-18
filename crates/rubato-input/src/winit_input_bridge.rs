//! WinitInputBridge - bridges winit window events to the input system
//!
//! Replaces the GdxInput stubs with real winit-based key/mouse state tracking.
//! winit KeyCode -> Java (libGDX) keycode mapping is provided here.
//!
//! Architecture:
//! - winit events are received on the main thread (event loop)
//! - Key state is stored in a shared array protected by a Mutex
//! - The keyboard processor's poll() reads from this shared state
//!   instead of the old GdxInput stubs

use std::sync::{Arc, Mutex};

use crate::keys::Keys;
use rubato_types::sync_utils::lock_or_recover;

/// Number of key slots (matches Java Gdx.input key array size)
const KEY_COUNT: usize = 256;

/// Shared key state that winit writes and the keyboard processor reads.
#[derive(Clone)]
pub struct SharedKeyState {
    inner: Arc<Mutex<KeyStateInner>>,
}

struct KeyStateInner {
    /// Each key pressed state, indexed by Java keycode
    keys: [bool; KEY_COUNT],
    /// Mouse position (logical pixels)
    mouse_x: i32,
    mouse_y: i32,
    /// Window size (logical pixels)
    window_width: i32,
    window_height: i32,
    /// Mouse button pressed state (indexed by libGDX button: 0=left, 1=right, 2=middle)
    mouse_buttons: [bool; 3],
    /// Accumulated scroll delta (drained on read)
    scroll_dx: f32,
    scroll_dy: f32,
}

impl SharedKeyState {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(KeyStateInner {
                keys: [false; KEY_COUNT],
                mouse_x: 0,
                mouse_y: 0,
                window_width: 1920,
                window_height: 1080,
                mouse_buttons: [false; 3],
                scroll_dx: 0.0,
                scroll_dy: 0.0,
            })),
        }
    }

    /// Query whether a key is pressed (by Java keycode).
    /// This replaces GdxInput::is_key_pressed().
    pub fn is_key_pressed(&self, keycode: i32) -> bool {
        if keycode < 0 || keycode as usize >= KEY_COUNT {
            return false;
        }
        let inner = lock_or_recover(&self.inner);
        inner.keys[keycode as usize]
    }

    /// Set key state (by Java keycode).
    pub fn set_key_pressed(&self, keycode: i32, pressed: bool) {
        if keycode >= 0 && (keycode as usize) < KEY_COUNT {
            let mut inner = lock_or_recover(&self.inner);
            inner.keys[keycode as usize] = pressed;
        }
    }

    /// Get mouse X position.
    pub fn mouse_x(&self) -> i32 {
        let inner = lock_or_recover(&self.inner);
        inner.mouse_x
    }

    /// Get mouse Y position.
    pub fn mouse_y(&self) -> i32 {
        let inner = lock_or_recover(&self.inner);
        inner.mouse_y
    }

    /// Set mouse position.
    pub fn set_mouse_position(&self, x: i32, y: i32) {
        let mut inner = lock_or_recover(&self.inner);
        inner.mouse_x = x;
        inner.mouse_y = y;
    }

    /// Get window width.
    pub fn window_width(&self) -> i32 {
        let inner = lock_or_recover(&self.inner);
        inner.window_width
    }

    /// Get window height.
    pub fn window_height(&self) -> i32 {
        let inner = lock_or_recover(&self.inner);
        inner.window_height
    }

    /// Set window size.
    pub fn set_window_size(&self, width: i32, height: i32) {
        let mut inner = lock_or_recover(&self.inner);
        inner.window_width = width;
        inner.window_height = height;
    }

    /// Set cursor position (used by MouseScratchInput).
    pub fn set_cursor_position(&self, _x: i32, _y: i32) {
        // In Java: Gdx.input.setCursorPosition(x, y)
        // In winit, cursor position setting requires the window handle,
        // which is not available here. This is a no-op for now;
        // the actual cursor warp would be done at the winit event loop level.
    }

    /// Set mouse button state (libGDX button index: 0=left, 1=right, 2=middle).
    pub fn set_mouse_button(&self, button: i32, pressed: bool) {
        if button >= 0 && (button as usize) < 3 {
            let mut inner = lock_or_recover(&self.inner);
            inner.mouse_buttons[button as usize] = pressed;
        }
    }

    /// Query mouse button state.
    pub fn is_mouse_button_pressed(&self, button: i32) -> bool {
        if button < 0 || button as usize >= 3 {
            return false;
        }
        let inner = lock_or_recover(&self.inner);
        inner.mouse_buttons[button as usize]
    }

    /// Accumulate scroll delta from winit MouseWheel events.
    pub fn add_scroll(&self, dx: f32, dy: f32) {
        let mut inner = lock_or_recover(&self.inner);
        inner.scroll_dx += dx;
        inner.scroll_dy += dy;
    }

    /// Drain accumulated scroll delta (returns and resets to zero).
    pub fn drain_scroll(&self) -> (f32, f32) {
        let mut inner = lock_or_recover(&self.inner);
        let dx = inner.scroll_dx;
        let dy = inner.scroll_dy;
        inner.scroll_dx = 0.0;
        inner.scroll_dy = 0.0;
        (dx, dy)
    }
}

impl Default for SharedKeyState {
    fn default() -> Self {
        Self::new()
    }
}

/// Map a winit physical KeyCode to the Java (libGDX) keycode.
///
/// Java libGDX uses Android-derived keycodes (com.badlogic.gdx.Input.Keys).
/// winit uses platform-independent physical key codes.
///
/// Returns -1 if the key has no mapping.
pub fn winit_keycode_to_java(key: WinitKeyCode) -> i32 {
    match key {
        // Letters
        WinitKeyCode::KeyA => 29,
        WinitKeyCode::KeyB => 30,
        WinitKeyCode::KeyC => Keys::C, // 31
        WinitKeyCode::KeyD => Keys::D, // 32
        WinitKeyCode::KeyE => 33,
        WinitKeyCode::KeyF => Keys::F, // 34
        WinitKeyCode::KeyG => 35,
        WinitKeyCode::KeyH => 36,
        WinitKeyCode::KeyI => 37,
        WinitKeyCode::KeyJ => 38,
        WinitKeyCode::KeyK => 39,
        WinitKeyCode::KeyL => Keys::L, // 40
        WinitKeyCode::KeyM => 41,
        WinitKeyCode::KeyN => 42,
        WinitKeyCode::KeyO => 43,
        WinitKeyCode::KeyP => 44,
        WinitKeyCode::KeyQ => Keys::Q, // 45
        WinitKeyCode::KeyR => 46,
        WinitKeyCode::KeyS => Keys::S, // 47
        WinitKeyCode::KeyT => 48,
        WinitKeyCode::KeyU => 49,
        WinitKeyCode::KeyV => Keys::V, // 50
        WinitKeyCode::KeyW => Keys::W, // 51
        WinitKeyCode::KeyX => Keys::X, // 52
        WinitKeyCode::KeyY => 53,
        WinitKeyCode::KeyZ => Keys::Z, // 54

        // Number row
        WinitKeyCode::Digit0 => Keys::NUM_0, // 7
        WinitKeyCode::Digit1 => Keys::NUM_1, // 8
        WinitKeyCode::Digit2 => Keys::NUM_2, // 9
        WinitKeyCode::Digit3 => Keys::NUM_3, // 10
        WinitKeyCode::Digit4 => Keys::NUM_4, // 11
        WinitKeyCode::Digit5 => Keys::NUM_5, // 12
        WinitKeyCode::Digit6 => Keys::NUM_6, // 13
        WinitKeyCode::Digit7 => Keys::NUM_7, // 14
        WinitKeyCode::Digit8 => Keys::NUM_8, // 15
        WinitKeyCode::Digit9 => Keys::NUM_9, // 16

        // Navigation
        WinitKeyCode::ArrowUp => Keys::UP,       // 19
        WinitKeyCode::ArrowDown => Keys::DOWN,   // 20
        WinitKeyCode::ArrowLeft => Keys::LEFT,   // 21
        WinitKeyCode::ArrowRight => Keys::RIGHT, // 22

        // Special keys
        WinitKeyCode::Enter => Keys::ENTER,        // 66
        WinitKeyCode::Escape => Keys::ESCAPE,      // 111
        WinitKeyCode::Delete => Keys::FORWARD_DEL, // 112
        WinitKeyCode::Insert => Keys::INSERT,      // 133

        // Modifiers
        WinitKeyCode::ShiftLeft => Keys::SHIFT_LEFT, // 59
        WinitKeyCode::ShiftRight => Keys::SHIFT_RIGHT, // 60
        WinitKeyCode::ControlLeft => Keys::CONTROL_LEFT, // 129
        WinitKeyCode::ControlRight => Keys::CONTROL_RIGHT, // 130
        WinitKeyCode::AltLeft => Keys::ALT_LEFT,     // 57
        WinitKeyCode::AltRight => Keys::ALT_RIGHT,   // 58

        // Punctuation (matching libGDX constants)
        WinitKeyCode::Comma => Keys::COMMA,         // 55
        WinitKeyCode::Period => Keys::PERIOD,       // 56
        WinitKeyCode::Semicolon => Keys::SEMICOLON, // 74
        WinitKeyCode::Quote => Keys::APOSTROPHE,    // 75
        WinitKeyCode::Slash => Keys::SLASH,         // 76
        WinitKeyCode::Backslash => Keys::BACKSLASH, // 73

        // Function keys
        WinitKeyCode::F1 => Keys::F1,   // 244
        WinitKeyCode::F2 => Keys::F2,   // 245
        WinitKeyCode::F3 => Keys::F3,   // 246
        WinitKeyCode::F4 => Keys::F4,   // 247
        WinitKeyCode::F5 => Keys::F5,   // 248
        WinitKeyCode::F6 => Keys::F6,   // 249
        WinitKeyCode::F7 => Keys::F7,   // 250
        WinitKeyCode::F8 => Keys::F8,   // 251
        WinitKeyCode::F9 => Keys::F9,   // 252
        WinitKeyCode::F10 => Keys::F10, // 253
        WinitKeyCode::F11 => Keys::F11, // 254
        WinitKeyCode::F12 => Keys::F12, // 255

        // Additional keys
        WinitKeyCode::Space => 62,
        WinitKeyCode::Backspace => 67,
        WinitKeyCode::Tab => 61,
        WinitKeyCode::Minus => 69,
        WinitKeyCode::Equal => 70,
        WinitKeyCode::BracketLeft => 71,
        WinitKeyCode::BracketRight => 72,
        WinitKeyCode::Backquote => 68,
        WinitKeyCode::Home => 3,
        WinitKeyCode::End => 123,
        WinitKeyCode::PageUp => 92,
        WinitKeyCode::PageDown => 93,

        _ => -1,
    }
}

/// Winit physical key codes - a Rust-only enum that mirrors winit::keyboard::KeyCode
/// for use without pulling in the winit dependency directly into beatoraja-input.
///
/// The caller (e.g., beatoraja-bin or beatoraja-launcher) maps from the actual
/// winit::keyboard::KeyCode to this enum before calling into the input bridge.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum WinitKeyCode {
    // Letters
    KeyA,
    KeyB,
    KeyC,
    KeyD,
    KeyE,
    KeyF,
    KeyG,
    KeyH,
    KeyI,
    KeyJ,
    KeyK,
    KeyL,
    KeyM,
    KeyN,
    KeyO,
    KeyP,
    KeyQ,
    KeyR,
    KeyS,
    KeyT,
    KeyU,
    KeyV,
    KeyW,
    KeyX,
    KeyY,
    KeyZ,

    // Number row
    Digit0,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,

    // Navigation
    ArrowUp,
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    Home,
    End,
    PageUp,
    PageDown,

    // Special keys
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Delete,
    Insert,

    // Modifiers
    ShiftLeft,
    ShiftRight,
    ControlLeft,
    ControlRight,
    AltLeft,
    AltRight,

    // Punctuation
    Comma,
    Period,
    Semicolon,
    Quote,
    Slash,
    Backslash,
    Minus,
    Equal,
    BracketLeft,
    BracketRight,
    Backquote,

    // Function keys
    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,

    // Unknown / unmapped
    Unknown,
}

/// Mouse button indices (matching libGDX conventions)
pub const MOUSE_BUTTON_LEFT: i32 = 0;
pub const MOUSE_BUTTON_RIGHT: i32 = 1;
pub const MOUSE_BUTTON_MIDDLE: i32 = 2;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shared_key_state_default_all_released() {
        let state = SharedKeyState::new();
        for i in 0..256 {
            assert!(!state.is_key_pressed(i));
        }
    }

    #[test]
    fn test_shared_key_state_set_and_get() {
        let state = SharedKeyState::new();
        state.set_key_pressed(Keys::Z, true);
        assert!(state.is_key_pressed(Keys::Z));
        assert!(!state.is_key_pressed(Keys::X));

        state.set_key_pressed(Keys::Z, false);
        assert!(!state.is_key_pressed(Keys::Z));
    }

    #[test]
    fn test_shared_key_state_out_of_range() {
        let state = SharedKeyState::new();
        assert!(!state.is_key_pressed(-1));
        assert!(!state.is_key_pressed(256));
        assert!(!state.is_key_pressed(1000));
        // Setting out of range should not panic
        state.set_key_pressed(-1, true);
        state.set_key_pressed(256, true);
    }

    #[test]
    fn test_shared_key_state_mouse_position() {
        let state = SharedKeyState::new();
        assert_eq!(state.mouse_x(), 0);
        assert_eq!(state.mouse_y(), 0);

        state.set_mouse_position(100, 200);
        assert_eq!(state.mouse_x(), 100);
        assert_eq!(state.mouse_y(), 200);
    }

    #[test]
    fn test_shared_key_state_window_size() {
        let state = SharedKeyState::new();
        assert_eq!(state.window_width(), 1920);
        assert_eq!(state.window_height(), 1080);

        state.set_window_size(1280, 720);
        assert_eq!(state.window_width(), 1280);
        assert_eq!(state.window_height(), 720);
    }

    #[test]
    fn test_shared_key_state_clone_shares_state() {
        let state1 = SharedKeyState::new();
        let state2 = state1.clone();

        state1.set_key_pressed(Keys::Z, true);
        assert!(state2.is_key_pressed(Keys::Z));
    }

    #[test]
    fn test_winit_keycode_to_java_letters() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyZ), Keys::Z);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyS), Keys::S);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyX), Keys::X);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyD), Keys::D);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyC), Keys::C);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyF), Keys::F);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyV), Keys::V);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyQ), Keys::Q);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyW), Keys::W);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::KeyL), Keys::L);
    }

    #[test]
    fn test_winit_keycode_to_java_numbers() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Digit0), Keys::NUM_0);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Digit1), Keys::NUM_1);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Digit9), Keys::NUM_9);
    }

    #[test]
    fn test_winit_keycode_to_java_modifiers() {
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::ShiftLeft),
            Keys::SHIFT_LEFT
        );
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::ShiftRight),
            Keys::SHIFT_RIGHT
        );
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::ControlLeft),
            Keys::CONTROL_LEFT
        );
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::ControlRight),
            Keys::CONTROL_RIGHT
        );
        assert_eq!(winit_keycode_to_java(WinitKeyCode::AltLeft), Keys::ALT_LEFT);
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::AltRight),
            Keys::ALT_RIGHT
        );
    }

    #[test]
    fn test_winit_keycode_to_java_function_keys() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::F1), Keys::F1);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::F12), Keys::F12);
    }

    #[test]
    fn test_winit_keycode_to_java_navigation() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::ArrowUp), Keys::UP);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::ArrowDown), Keys::DOWN);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::ArrowLeft), Keys::LEFT);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::ArrowRight), Keys::RIGHT);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Enter), Keys::ENTER);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Escape), Keys::ESCAPE);
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::Delete),
            Keys::FORWARD_DEL
        );
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Insert), Keys::INSERT);
    }

    #[test]
    fn test_winit_keycode_to_java_punctuation() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Comma), Keys::COMMA);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Period), Keys::PERIOD);
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::Semicolon),
            Keys::SEMICOLON
        );
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Quote), Keys::APOSTROPHE);
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Slash), Keys::SLASH);
        assert_eq!(
            winit_keycode_to_java(WinitKeyCode::Backslash),
            Keys::BACKSLASH
        );
    }

    #[test]
    fn test_winit_keycode_to_java_unknown() {
        assert_eq!(winit_keycode_to_java(WinitKeyCode::Unknown), -1);
    }

    #[test]
    fn test_key_state_press_and_release_cycle() {
        let state = SharedKeyState::new();

        // Simulate pressing Z, S, X, D, C keys
        let keys_to_press = [Keys::Z, Keys::S, Keys::X, Keys::D, Keys::C];
        for &k in &keys_to_press {
            state.set_key_pressed(k, true);
        }

        for &k in &keys_to_press {
            assert!(state.is_key_pressed(k));
        }

        // Release all
        for &k in &keys_to_press {
            state.set_key_pressed(k, false);
        }

        for &k in &keys_to_press {
            assert!(!state.is_key_pressed(k));
        }
    }

    #[test]
    fn test_shared_key_state_thread_safety() {
        let state = SharedKeyState::new();
        let state_clone = state.clone();

        let handle = std::thread::spawn(move || {
            for i in 0..10 {
                state_clone.set_key_pressed(i, true);
            }
        });

        handle.join().unwrap();

        for i in 0..10 {
            assert!(state.is_key_pressed(i));
        }
    }

    // -----------------------------------------------------------------------
    // Scroll tests
    // -----------------------------------------------------------------------

    #[test]
    fn test_scroll_accumulation() {
        let state = SharedKeyState::new();
        state.add_scroll(1.0, 2.0);
        state.add_scroll(3.0, 4.0);
        let (dx, dy) = state.drain_scroll();
        assert_eq!(dx, 4.0);
        assert_eq!(dy, 6.0);
    }

    #[test]
    fn test_scroll_drain_resets() {
        let state = SharedKeyState::new();
        state.add_scroll(5.0, 10.0);
        let (dx, dy) = state.drain_scroll();
        assert_eq!(dx, 5.0);
        assert_eq!(dy, 10.0);

        // Second drain should return zeros
        let (dx2, dy2) = state.drain_scroll();
        assert_eq!(dx2, 0.0);
        assert_eq!(dy2, 0.0);
    }

    #[test]
    fn test_scroll_drain_empty() {
        let state = SharedKeyState::new();
        // Drain without any scroll events
        let (dx, dy) = state.drain_scroll();
        assert_eq!(dx, 0.0);
        assert_eq!(dy, 0.0);
    }

    // -----------------------------------------------------------------------
    // Press-drag-release sequence
    // -----------------------------------------------------------------------

    #[test]
    fn test_press_drag_release_sequence() {
        let state = SharedKeyState::new();

        // Step 1: Press left mouse button
        state.set_mouse_button(MOUSE_BUTTON_LEFT, true);
        assert!(state.is_mouse_button_pressed(MOUSE_BUTTON_LEFT));
        assert_eq!(state.mouse_x(), 0);
        assert_eq!(state.mouse_y(), 0);

        // Step 2: Drag (move mouse while button is still pressed)
        state.set_mouse_position(100, 200);
        assert!(state.is_mouse_button_pressed(MOUSE_BUTTON_LEFT));
        assert_eq!(state.mouse_x(), 100);
        assert_eq!(state.mouse_y(), 200);

        // Step 3: Release mouse button
        state.set_mouse_button(MOUSE_BUTTON_LEFT, false);
        assert!(!state.is_mouse_button_pressed(MOUSE_BUTTON_LEFT));
        // Position should remain after release
        assert_eq!(state.mouse_x(), 100);
        assert_eq!(state.mouse_y(), 200);
    }

    // -----------------------------------------------------------------------
    // Modifier composite
    // -----------------------------------------------------------------------

    #[test]
    fn test_modifier_keys_composite() {
        let state = SharedKeyState::new();

        // Press multiple modifier keys simultaneously
        state.set_key_pressed(Keys::SHIFT_LEFT, true);
        state.set_key_pressed(Keys::CONTROL_LEFT, true);
        state.set_key_pressed(Keys::ALT_LEFT, true);

        // Verify each independently
        assert!(state.is_key_pressed(Keys::SHIFT_LEFT));
        assert!(state.is_key_pressed(Keys::CONTROL_LEFT));
        assert!(state.is_key_pressed(Keys::ALT_LEFT));

        // Release one modifier
        state.set_key_pressed(Keys::CONTROL_LEFT, false);

        // Other modifiers should still be pressed
        assert!(state.is_key_pressed(Keys::SHIFT_LEFT));
        assert!(!state.is_key_pressed(Keys::CONTROL_LEFT));
        assert!(state.is_key_pressed(Keys::ALT_LEFT));
    }

    // -----------------------------------------------------------------------
    // Key isolation
    // -----------------------------------------------------------------------

    #[test]
    fn test_key_state_isolation() {
        let state = SharedKeyState::new();

        // Press a single key
        state.set_key_pressed(Keys::Z, true);

        // Verify it does not affect neighboring or unrelated keys
        assert!(state.is_key_pressed(Keys::Z));
        assert!(!state.is_key_pressed(Keys::X));
        assert!(!state.is_key_pressed(Keys::S));
        assert!(!state.is_key_pressed(Keys::D));
        assert!(!state.is_key_pressed(Keys::C));
        assert!(!state.is_key_pressed(Keys::F));
        assert!(!state.is_key_pressed(Keys::SHIFT_LEFT));
        assert!(!state.is_key_pressed(Keys::CONTROL_LEFT));
        assert!(!state.is_key_pressed(Keys::ALT_LEFT));
    }
}

#[cfg(test)]
mod prop_tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn scroll_accumulation_invariant(
            deltas in prop::collection::vec(
                (-100.0f32..100.0, -100.0f32..100.0),
                0..50
            )
        ) {
            let state = SharedKeyState::new();
            let mut expected_x = 0.0f32;
            let mut expected_y = 0.0f32;
            for (dx, dy) in &deltas {
                state.add_scroll(*dx, *dy);
                expected_x += dx;
                expected_y += dy;
            }
            let (actual_x, actual_y) = state.drain_scroll();
            // Use approximate comparison for floating point
            prop_assert!((actual_x - expected_x).abs() < 0.01,
                "x: expected {}, got {}", expected_x, actual_x);
            prop_assert!((actual_y - expected_y).abs() < 0.01,
                "y: expected {}, got {}", expected_y, actual_y);
        }
    }

    #[test]
    fn concurrent_scroll_no_loss() {
        use std::sync::Arc;
        use std::thread;

        let state = Arc::new(SharedKeyState::new());
        let n_threads = 4;
        let n_per_thread = 100;

        let handles: Vec<_> = (0..n_threads)
            .map(|_| {
                let s = Arc::clone(&state);
                thread::spawn(move || {
                    for _ in 0..n_per_thread {
                        s.add_scroll(1.0, 1.0);
                    }
                })
            })
            .collect();

        for h in handles {
            h.join().unwrap();
        }

        let (x, y) = state.drain_scroll();
        let expected = (n_threads * n_per_thread) as f32;
        assert!(
            (x - expected).abs() < 0.01,
            "x: expected {}, got {}",
            expected,
            x
        );
        assert!(
            (y - expected).abs() < 0.01,
            "y: expected {}, got {}",
            expected,
            y
        );
    }
}
