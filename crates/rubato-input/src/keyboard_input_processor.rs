//! KeyBoardInputProcesseor - keyboard input processing
//!
//! Translated from: bms.player.beatoraja.input.KeyBoardInputProcesseor
//! Note: The typo "Processeor" is preserved from the Java source.

use crate::bms_player_input_device::{BMSPlayerInputDevice, DeviceType};
use crate::mouse_scratch_input::MouseScratchInput;
use crate::stubs::{GdxGraphics, GdxInput, KeyboardConfig, Keys, Resolution};

pub const MASK_SHIFT: i32 = 1 << 0;
pub const MASK_CTRL: i32 = 1 << 1;
pub const MASK_ALT: i32 = 1 << 2;

/// ControlKeys enum
///
/// Translated from: KeyBoardInputProcesseor.ControlKeys
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKeys {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
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
    Up,
    Down,
    Left,
    Right,
    Enter,
    Insert,
    Del,
    Escape,
    KeyC,
}

impl ControlKeys {
    pub fn id(&self) -> i32 {
        match self {
            ControlKeys::Num0 => 0,
            ControlKeys::Num1 => 1,
            ControlKeys::Num2 => 2,
            ControlKeys::Num3 => 3,
            ControlKeys::Num4 => 4,
            ControlKeys::Num5 => 5,
            ControlKeys::Num6 => 6,
            ControlKeys::Num7 => 7,
            ControlKeys::Num8 => 8,
            ControlKeys::Num9 => 9,
            ControlKeys::F1 => 10,
            ControlKeys::F2 => 11,
            ControlKeys::F3 => 12,
            ControlKeys::F4 => 13,
            ControlKeys::F5 => 14,
            ControlKeys::F6 => 15,
            ControlKeys::F7 => 16,
            ControlKeys::F8 => 17,
            ControlKeys::F9 => 18,
            ControlKeys::F10 => 19,
            ControlKeys::F11 => 20,
            ControlKeys::F12 => 21,
            ControlKeys::Up => 22,
            ControlKeys::Down => 23,
            ControlKeys::Left => 24,
            ControlKeys::Right => 25,
            ControlKeys::Enter => 26,
            ControlKeys::Insert => 27,
            ControlKeys::Del => 28,
            ControlKeys::Escape => 29,
            ControlKeys::KeyC => 30,
        }
    }

    pub fn keycode(&self) -> i32 {
        match self {
            ControlKeys::Num0 => Keys::NUM_0,
            ControlKeys::Num1 => Keys::NUM_1,
            ControlKeys::Num2 => Keys::NUM_2,
            ControlKeys::Num3 => Keys::NUM_3,
            ControlKeys::Num4 => Keys::NUM_4,
            ControlKeys::Num5 => Keys::NUM_5,
            ControlKeys::Num6 => Keys::NUM_6,
            ControlKeys::Num7 => Keys::NUM_7,
            ControlKeys::Num8 => Keys::NUM_8,
            ControlKeys::Num9 => Keys::NUM_9,
            ControlKeys::F1 => Keys::F1,
            ControlKeys::F2 => Keys::F2,
            ControlKeys::F3 => Keys::F3,
            ControlKeys::F4 => Keys::F4,
            ControlKeys::F5 => Keys::F5,
            ControlKeys::F6 => Keys::F6,
            ControlKeys::F7 => Keys::F7,
            ControlKeys::F8 => Keys::F8,
            ControlKeys::F9 => Keys::F9,
            ControlKeys::F10 => Keys::F10,
            ControlKeys::F11 => Keys::F11,
            ControlKeys::F12 => Keys::F12,
            ControlKeys::Up => Keys::UP,
            ControlKeys::Down => Keys::DOWN,
            ControlKeys::Left => Keys::LEFT,
            ControlKeys::Right => Keys::RIGHT,
            ControlKeys::Enter => Keys::ENTER,
            ControlKeys::Insert => Keys::INSERT,
            ControlKeys::Del => Keys::FORWARD_DEL,
            ControlKeys::Escape => Keys::ESCAPE,
            ControlKeys::KeyC => Keys::C,
        }
    }

    pub fn text(&self) -> bool {
        matches!(
            self,
            ControlKeys::Num0
                | ControlKeys::Num1
                | ControlKeys::Num2
                | ControlKeys::Num3
                | ControlKeys::Num4
                | ControlKeys::Num5
                | ControlKeys::Num6
                | ControlKeys::Num7
                | ControlKeys::Num8
                | ControlKeys::Num9
        )
    }

    pub fn values() -> &'static [ControlKeys] {
        &[
            ControlKeys::Num0,
            ControlKeys::Num1,
            ControlKeys::Num2,
            ControlKeys::Num3,
            ControlKeys::Num4,
            ControlKeys::Num5,
            ControlKeys::Num6,
            ControlKeys::Num7,
            ControlKeys::Num8,
            ControlKeys::Num9,
            ControlKeys::F1,
            ControlKeys::F2,
            ControlKeys::F3,
            ControlKeys::F4,
            ControlKeys::F5,
            ControlKeys::F6,
            ControlKeys::F7,
            ControlKeys::F8,
            ControlKeys::F9,
            ControlKeys::F10,
            ControlKeys::F11,
            ControlKeys::F12,
            ControlKeys::Up,
            ControlKeys::Down,
            ControlKeys::Left,
            ControlKeys::Right,
            ControlKeys::Enter,
            ControlKeys::Insert,
            ControlKeys::Del,
            ControlKeys::Escape,
            ControlKeys::KeyC,
        ]
    }
}

/// Callback interface for BMSPlayerInputProcessor methods called from keyboard processor
pub trait KeyboardCallback {
    fn key_changed_from_keyboard(&mut self, microtime: i64, key: usize, pressed: bool);
    fn start_changed(&mut self, pressed: bool);
    fn set_select_pressed(&mut self, pressed: bool);
    fn set_analog_state(&mut self, key: usize, is_analog: bool, value: f32);
    fn set_mouse_moved(&mut self, moved: bool);
    fn set_mouse_x(&mut self, x: i32);
    fn set_mouse_y(&mut self, y: i32);
    fn set_mouse_button(&mut self, button: i32);
    fn set_mouse_pressed(&mut self, pressed: bool);
    fn set_mouse_dragged(&mut self, dragged: bool);
    fn add_scroll_x(&mut self, amount: f32);
    fn add_scroll_y(&mut self, amount: f32);
}

/// Keyboard input processing
pub struct KeyBoardInputProcesseor {
    keys: Vec<i32>,
    control: Vec<i32>,

    mouse_scratch_input: MouseScratchInput,

    reserved: Vec<i32>,
    /// Last pressed key
    pub last_pressed_key: i32,

    pub textmode: bool,

    /// Screen resolution. Used for mouse input event processing
    resolution: Resolution,

    /// Each key on/off state
    keystate: [bool; 256],
    /// Each key state change time
    keytime: [i64; 256],
    /// Modifier keys held when each key was last pressed
    keymodifiers: [i32; 256],
    /// Minimum key input interval (ms)
    duration: i32,
}

impl KeyBoardInputProcesseor {
    pub fn new(config: &KeyboardConfig, resolution: Resolution) -> Self {
        let mut reserved = Vec::new();
        for key in ControlKeys::values() {
            reserved.push(key.keycode());
        }

        let mut keytime = [0i64; 256];
        keytime.fill(i64::MIN);

        let mouse_scratch_input = MouseScratchInput::new(config);

        let mut proc = Self {
            keys: vec![
                Keys::Z,
                Keys::S,
                Keys::X,
                Keys::D,
                Keys::C,
                Keys::F,
                Keys::V,
                Keys::SHIFT_LEFT,
                Keys::CONTROL_LEFT,
                Keys::COMMA,
                Keys::L,
                Keys::PERIOD,
                Keys::SEMICOLON,
                Keys::SLASH,
                Keys::APOSTROPHE,
                Keys::BACKSLASH,
                Keys::SHIFT_RIGHT,
                Keys::CONTROL_RIGHT,
            ],
            control: vec![Keys::Q, Keys::W],
            mouse_scratch_input,
            reserved,
            last_pressed_key: -1,
            textmode: false,
            resolution,
            keystate: [false; 256],
            keytime,
            keymodifiers: [0; 256],
            duration: 0,
        };
        proc.set_config(config);
        proc
    }

    pub fn set_config(&mut self, config: &KeyboardConfig) {
        self.keys = config.keys.to_vec();
        self.duration = config.duration;
        self.control = vec![config.start, config.select];
        self.mouse_scratch_input.set_config(config);
    }

    pub fn key_down(&mut self, keycode: i32) -> bool {
        self.last_pressed_key = keycode;
        true
    }

    pub fn key_typed(&mut self, _keycode: char) -> bool {
        false
    }

    pub fn key_up(&mut self, _keycode: i32) -> bool {
        true
    }

    pub fn poll(&mut self, microtime: i64, callback: &mut dyn KeyboardCallback) {
        // NOTE: For further dev came here, it's better to wrap this variable instead of
        // accessing imgui menu's field directly
        let accept_input = !rubato_types::skin_widget_focus::focus();
        if accept_input && !self.textmode {
            for (i, &key) in self.keys.iter().enumerate() {
                if key < 0 {
                    continue;
                }
                let key_idx = key as usize;
                let pressed = GdxInput::is_key_pressed(key);
                if pressed != self.keystate[key_idx]
                    && microtime >= self.keytime[key_idx] + (self.duration as i64) * 1000
                {
                    self.keystate[key_idx] = pressed;
                    self.keytime[key_idx] = microtime;
                    callback.key_changed_from_keyboard(microtime, i, pressed);
                    callback.set_analog_state(i, false, 0.0);
                }
            }

            let startpressed = GdxInput::is_key_pressed(self.control[0]);
            let ctrl0 = self.control[0] as usize;
            if startpressed != self.keystate[ctrl0] {
                self.keystate[ctrl0] = startpressed;
                callback.start_changed(startpressed);
            }
            let selectpressed = GdxInput::is_key_pressed(self.control[1]);
            let ctrl1 = self.control[1] as usize;
            if selectpressed != self.keystate[ctrl1] {
                self.keystate[ctrl1] = selectpressed;
                callback.set_select_pressed(selectpressed);
            }
        }

        for key in ControlKeys::values() {
            let pressed = GdxInput::is_key_pressed(key.keycode());
            let kc = key.keycode() as usize;
            if !(self.textmode && key.text()) && pressed != self.keystate[kc] && accept_input {
                self.keystate[kc] = pressed;
                self.keytime[kc] = microtime;
                self.keymodifiers[kc] = if pressed {
                    self.currently_held_modifiers()
                } else {
                    0
                };
            }
        }

        self.mouse_scratch_input.poll(microtime, callback);
    }

    fn currently_held_modifiers(&self) -> i32 {
        let shift = GdxInput::is_key_pressed(Keys::SHIFT_LEFT)
            || GdxInput::is_key_pressed(Keys::SHIFT_RIGHT);
        let ctrl = GdxInput::is_key_pressed(Keys::CONTROL_LEFT)
            || GdxInput::is_key_pressed(Keys::CONTROL_RIGHT);
        let alt =
            GdxInput::is_key_pressed(Keys::ALT_LEFT) || GdxInput::is_key_pressed(Keys::ALT_RIGHT);
        (if shift { MASK_SHIFT } else { 0 })
            | (if ctrl { MASK_CTRL } else { 0 })
            | (if alt { MASK_ALT } else { 0 })
    }

    pub fn key_state(&self, keycode: i32) -> bool {
        self.keystate[keycode as usize]
    }

    pub fn set_key_state(&mut self, keycode: i32, pressed: bool) {
        self.keystate[keycode as usize] = pressed;
    }

    pub fn is_key_pressed(&mut self, keycode: i32) -> bool {
        let kc = keycode as usize;
        if self.keystate[kc] && self.keytime[kc] != i64::MIN {
            self.keytime[kc] = i64::MIN;
            return true;
        }
        false
    }

    pub fn is_key_pressed_with_modifiers(
        &mut self,
        keycode: i32,
        held_modifiers: i32,
        not_held_modifiers: &[i32],
    ) -> bool {
        let kc = keycode as usize;
        if self.keystate[kc] && self.keytime[kc] != i64::MIN {
            let modifiers = self.keymodifiers[kc];
            if (modifiers & held_modifiers) != held_modifiers {
                return false;
            }
            for &modifier in not_held_modifiers {
                if (modifiers & modifier) == modifier {
                    return false;
                }
            }
            self.keytime[kc] = i64::MIN;
            return true;
        }
        false
    }

    pub fn mouse_moved(&self, x: i32, y: i32, callback: &mut dyn KeyboardCallback) -> bool {
        callback.set_mouse_moved(true);
        callback.set_mouse_x(x * self.resolution.width() / GdxGraphics::get_width());
        callback.set_mouse_y(
            self.resolution.height() - y * self.resolution.height() / GdxGraphics::get_height(),
        );
        false
    }

    /// Legacy InputProcessor method - to be removed on libGDX update
    pub fn scrolled_int(&self, amount: i32, callback: &mut dyn KeyboardCallback) -> bool {
        self.scrolled(0.0, amount as f32, callback)
    }

    pub fn scrolled(
        &self,
        amount_x: f32,
        amount_y: f32,
        callback: &mut dyn KeyboardCallback,
    ) -> bool {
        callback.add_scroll_x(amount_x);
        callback.add_scroll_y(amount_y);
        false
    }

    pub fn touch_down(
        &self,
        x: i32,
        y: i32,
        _point: i32,
        button: i32,
        callback: &mut dyn KeyboardCallback,
    ) -> bool {
        callback.set_mouse_button(button);
        callback.set_mouse_x(x * self.resolution.width() / GdxGraphics::get_width());
        callback.set_mouse_y(
            self.resolution.height() - y * self.resolution.height() / GdxGraphics::get_height(),
        );
        callback.set_mouse_pressed(true);
        false
    }

    pub fn touch_dragged(
        &self,
        x: i32,
        y: i32,
        _point: i32,
        callback: &mut dyn KeyboardCallback,
    ) -> bool {
        callback.set_mouse_x(x * self.resolution.width() / GdxGraphics::get_width());
        callback.set_mouse_y(
            self.resolution.height() - y * self.resolution.height() / GdxGraphics::get_height(),
        );
        callback.set_mouse_dragged(true);
        false
    }

    pub fn touch_up(&self, _arg0: i32, _arg1: i32, _arg2: i32, _arg3: i32) -> bool {
        false
    }

    pub fn touch_cancelled(
        &self,
        _screen_x: i32,
        _screen_y: i32,
        _pointer: i32,
        _button: i32,
    ) -> bool {
        false
    }

    pub fn last_pressed_key(&self) -> i32 {
        self.last_pressed_key
    }
    pub fn get_mouse_scratch_input(&self) -> &MouseScratchInput {
        &self.mouse_scratch_input
    }

    pub fn mouse_scratch_input_mut(&mut self) -> &mut MouseScratchInput {
        &mut self.mouse_scratch_input
    }
    pub fn is_reserved_key(&self, key: i32) -> bool {
        self.reserved.contains(&key)
    }

    pub fn resolution(&self) -> &Resolution {
        &self.resolution
    }

    pub fn sync_runtime_state_from(&mut self, source: &Self) {
        self.keystate = source.keystate;
        self.keytime = source.keytime;
        self.keymodifiers = source.keymodifiers;
        self.last_pressed_key = source.last_pressed_key;
    }
}

impl BMSPlayerInputDevice for KeyBoardInputProcesseor {
    fn device_type(&self) -> DeviceType {
        DeviceType::Keyboard
    }

    fn clear(&mut self) {
        // Arrays.fill(keystate, false);
        self.keytime.fill(i64::MIN);
        self.last_pressed_key = -1;
        self.mouse_scratch_input.clear();
    }
}
