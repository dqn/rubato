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
            })),
        }
    }

    /// Query whether a key is pressed (by Java keycode).
    /// This replaces GdxInput::is_key_pressed().
    pub fn is_key_pressed(&self, keycode: i32) -> bool {
        if keycode < 0 || keycode as usize >= KEY_COUNT {
            return false;
        }
        let inner = self.inner.lock().unwrap();
        inner.keys[keycode as usize]
    }

    /// Set key state (by Java keycode).
    pub fn set_key_pressed(&self, keycode: i32, pressed: bool) {
        if keycode >= 0 && (keycode as usize) < KEY_COUNT {
            let mut inner = self.inner.lock().unwrap();
            inner.keys[keycode as usize] = pressed;
        }
    }

    /// Get mouse X position.
    pub fn get_mouse_x(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.mouse_x
    }

    /// Get mouse Y position.
    pub fn get_mouse_y(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.mouse_y
    }

    /// Set mouse position.
    pub fn set_mouse_position(&self, x: i32, y: i32) {
        let mut inner = self.inner.lock().unwrap();
        inner.mouse_x = x;
        inner.mouse_y = y;
    }

    /// Get window width.
    pub fn get_window_width(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.window_width
    }

    /// Get window height.
    pub fn get_window_height(&self) -> i32 {
        let inner = self.inner.lock().unwrap();
        inner.window_height
    }

    /// Set window size.
    pub fn set_window_size(&self, width: i32, height: i32) {
        let mut inner = self.inner.lock().unwrap();
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
#[allow(clippy::match_same_arms)]
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
#[allow(dead_code)]
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
        assert_eq!(state.get_mouse_x(), 0);
        assert_eq!(state.get_mouse_y(), 0);

        state.set_mouse_position(100, 200);
        assert_eq!(state.get_mouse_x(), 100);
        assert_eq!(state.get_mouse_y(), 200);
    }

    #[test]
    fn test_shared_key_state_window_size() {
        let state = SharedKeyState::new();
        assert_eq!(state.get_window_width(), 1920);
        assert_eq!(state.get_window_height(), 1080);

        state.set_window_size(1280, 720);
        assert_eq!(state.get_window_width(), 1280);
        assert_eq!(state.get_window_height(), 720);
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
}
