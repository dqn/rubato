// Phase 4 stubs - will be replaced when Phase 4 is translated

/// Stub for bms.player.beatoraja.Config
pub struct Config {
    pub analog_scroll: bool,
}

impl Config {
    pub fn is_analog_scroll(&self) -> bool {
        self.analog_scroll
    }

    pub fn get_resolution(&self) -> Resolution {
        Resolution {
            width: 1920,
            height: 1080,
        }
    }
}

/// Stub for bms.player.beatoraja.Resolution
#[derive(Clone, Copy)]
pub struct Resolution {
    pub width: i32,
    pub height: i32,
}

/// Stub for bms.player.beatoraja.PlayerConfig
pub struct PlayerConfig;

impl PlayerConfig {
    pub fn get_mode14(&self) -> PlayModeConfig {
        PlayModeConfig::new()
    }

    pub fn get_mode7(&self) -> PlayModeConfig {
        PlayModeConfig::new()
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig.KeyboardConfig
#[derive(Clone, Default)]
pub struct KeyboardConfig {
    pub key_assign: Vec<i32>,
    pub duration: i32,
    pub start: i32,
    pub select: i32,
    pub mouse_scratch_config: MouseScratchConfig,
}

impl KeyboardConfig {
    pub fn get_key_assign(&self) -> &[i32] {
        &self.key_assign
    }

    pub fn get_duration(&self) -> i32 {
        self.duration
    }

    pub fn get_start(&self) -> i32 {
        self.start
    }

    pub fn get_select(&self) -> i32 {
        self.select
    }

    pub fn get_mouse_scratch_config(&self) -> &MouseScratchConfig {
        &self.mouse_scratch_config
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig.ControllerConfig
#[derive(Clone)]
pub struct ControllerConfig {
    pub name: Option<String>,
    pub key_assign: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub duration: i32,
    pub jkoc: bool,
    pub analog_scratch: bool,
    pub analog_scratch_threshold: i32,
    pub analog_scratch_mode: i32,
}

impl ControllerConfig {
    pub const ANALOG_SCRATCH_VER_1: i32 = 0;
    pub const ANALOG_SCRATCH_VER_2: i32 = 1;
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            name: None,
            key_assign: vec![],
            start: 0,
            select: 0,
            duration: 16,
            jkoc: false,
            analog_scratch: false,
            analog_scratch_threshold: 0,
            analog_scratch_mode: 0,
        }
    }
}

impl ControllerConfig {
    pub fn get_name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    pub fn set_name(&mut self, name: String) {
        self.name = Some(name);
    }

    pub fn get_key_assign(&self) -> &[i32] {
        &self.key_assign
    }

    pub fn get_start(&self) -> i32 {
        self.start
    }

    pub fn get_select(&self) -> i32 {
        self.select
    }

    pub fn get_duration(&self) -> i32 {
        self.duration
    }

    #[allow(non_snake_case)]
    pub fn get_jkoc(&self) -> bool {
        self.jkoc
    }

    pub fn is_analog_scratch(&self) -> bool {
        self.analog_scratch
    }

    pub fn get_analog_scratch_threshold(&self) -> i32 {
        self.analog_scratch_threshold
    }

    pub fn get_analog_scratch_mode(&self) -> i32 {
        self.analog_scratch_mode
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig.MidiConfig
#[derive(Clone, Default)]
pub struct MidiConfig {
    pub keys: Vec<Option<MidiInput>>,
    pub start: Option<MidiInput>,
    pub select: Option<MidiInput>,
}

impl MidiConfig {
    pub fn get_keys(&self) -> &[Option<MidiInput>] {
        &self.keys
    }

    pub fn get_keys_mut(&mut self) -> &mut [Option<MidiInput>] {
        &mut self.keys
    }

    pub fn get_start(&self) -> Option<&MidiInput> {
        self.start.as_ref()
    }

    pub fn get_select(&self) -> Option<&MidiInput> {
        self.select.as_ref()
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig.MidiConfig.Input
#[derive(Clone)]
pub struct MidiInput {
    pub input_type: MidiInputType,
    pub value: i32,
}

impl Default for MidiInput {
    fn default() -> Self {
        Self {
            input_type: MidiInputType::Note,
            value: 0,
        }
    }
}

impl MidiInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn copy_from(&self) -> Self {
        Self {
            input_type: self.input_type.clone(),
            value: self.value,
        }
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig.MidiConfig.Input.Type
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MidiInputType {
    Note,
    PitchBend,
    ControlChange,
}

/// Stub for bms.player.beatoraja.PlayModeConfig.MouseScratchConfig
#[derive(Clone)]
pub struct MouseScratchConfig {
    pub key_assign: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub enabled: bool,
    pub time_threshold: i32,
    pub distance: i32,
    pub mode: i32,
}

impl MouseScratchConfig {
    pub const MOUSE_SCRATCH_VER_1: i32 = 0;
    pub const MOUSE_SCRATCH_VER_2: i32 = 1;
}

impl Default for MouseScratchConfig {
    fn default() -> Self {
        Self {
            key_assign: vec![],
            start: -1,
            select: -1,
            enabled: false,
            time_threshold: 150,
            distance: 150,
            mode: 0,
        }
    }
}

impl MouseScratchConfig {
    pub fn get_key_assign(&self) -> &[i32] {
        &self.key_assign
    }

    pub fn get_start(&self) -> i32 {
        self.start
    }

    pub fn get_select(&self) -> i32 {
        self.select
    }

    pub fn is_mouse_scratch_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_mouse_scratch_time_threshold(&self) -> i32 {
        self.time_threshold
    }

    pub fn get_mouse_scratch_distance(&self) -> i32 {
        self.distance
    }

    pub fn get_mouse_scratch_mode(&self) -> i32 {
        self.mode
    }
}

/// Stub for bms.player.beatoraja.PlayModeConfig
pub struct PlayModeConfig {
    pub keyboard_config: KeyboardConfig,
    pub controller: Vec<ControllerConfig>,
    pub midi_config: MidiConfig,
}

impl Default for PlayModeConfig {
    fn default() -> Self {
        Self::new()
    }
}

impl PlayModeConfig {
    pub fn new() -> Self {
        Self {
            keyboard_config: KeyboardConfig::default(),
            controller: vec![],
            midi_config: MidiConfig::default(),
        }
    }

    pub fn get_keyboard_config(&self) -> &KeyboardConfig {
        &self.keyboard_config
    }

    pub fn get_controller(&self) -> &[ControllerConfig] {
        &self.controller
    }

    pub fn get_controller_mut(&mut self) -> &mut [ControllerConfig] {
        &mut self.controller
    }

    pub fn get_midi_config(&self) -> &MidiConfig {
        &self.midi_config
    }

    pub fn get_midi_config_mut(&mut self) -> &mut MidiConfig {
        &mut self.midi_config
    }
}

/// Stub for Gdx.input / Gdx.graphics
pub struct GdxInput;

impl GdxInput {
    pub fn is_key_pressed(_keycode: i32) -> bool {
        false
    }

    pub fn get_x() -> i32 {
        0
    }

    pub fn get_y() -> i32 {
        0
    }

    pub fn set_cursor_position(_x: i32, _y: i32) {
        // stub
    }
}

pub struct GdxGraphics;

impl GdxGraphics {
    pub fn get_width() -> i32 {
        1920
    }

    pub fn get_height() -> i32 {
        1080
    }
}

/// Stub for SkinWidgetManager
pub struct SkinWidgetManager;

impl SkinWidgetManager {
    pub fn get_focus() -> bool {
        false
    }
}

/// Stub for Controller (com.badlogic.gdx.controllers.Controller)
pub struct Controller {
    name: String,
}

impl Controller {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_button(&self, _button: i32) -> bool {
        false
    }

    pub fn get_axis(&self, _axis: i32) -> f32 {
        0.0
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
