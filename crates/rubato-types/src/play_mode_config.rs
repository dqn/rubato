use crate::bm_keys::BMKeys;
use bms_model::mode::Mode;

use crate::play_config::PlayConfig;

// libGDX Keys constants (from com.badlogic.gdx.Input.Keys)
mod gdx_keys {
    pub const UNKNOWN: i32 = 0;
    pub const SHIFT_LEFT: i32 = 59;
    pub const SHIFT_RIGHT: i32 = 60;
    pub const CONTROL_LEFT: i32 = 129;
    pub const CONTROL_RIGHT: i32 = 130;
    pub const Z: i32 = 54;
    pub const S: i32 = 47;
    pub const X: i32 = 52;
    pub const D: i32 = 32;
    pub const C: i32 = 31;
    pub const F: i32 = 34;
    pub const V: i32 = 50;
    pub const G: i32 = 35;
    pub const B: i32 = 30;
    pub const Q: i32 = 45;
    pub const W: i32 = 51;
    pub const COMMA: i32 = 55;
    pub const L: i32 = 40;
    pub const PERIOD: i32 = 56;
    pub const SEMICOLON: i32 = 74;
    pub const SLASH: i32 = 76;
    pub const APOSTROPHE: i32 = 75;
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlayModeConfig {
    pub playconfig: PlayConfig,
    pub keyboard: KeyboardConfig,
    pub controller: Vec<ControllerConfig>,
    pub midi: MidiConfig,
    pub version: i32,
}

impl Default for PlayModeConfig {
    fn default() -> Self {
        PlayModeConfig::new(Mode::BEAT_7K)
    }
}

impl PlayModeConfig {
    pub fn new(mode: Mode) -> Self {
        let is_midi = mode == Mode::KEYBOARD_24K || mode == Mode::KEYBOARD_24K_DOUBLE;
        let keyboard = KeyboardConfig::new(mode.clone(), !is_midi);
        let player_count = mode.player() as usize;
        let mut controller = Vec::with_capacity(player_count);
        for i in 0..player_count {
            controller.push(ControllerConfig::new_with_mode(
                mode.clone(),
                i as i32,
                false,
            ));
        }
        let midi = MidiConfig::new(mode, is_midi);
        PlayModeConfig {
            playconfig: PlayConfig::default(),
            keyboard,
            controller,
            midi,
            version: 0,
        }
    }

    pub fn new_with_configs(
        keyboard: KeyboardConfig,
        controllers: Vec<ControllerConfig>,
        midi: MidiConfig,
    ) -> Self {
        PlayModeConfig {
            playconfig: PlayConfig::default(),
            keyboard,
            controller: controllers,
            midi,
            version: 0,
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub fn validate(&mut self, keys: usize) {
        self.playconfig.validate();

        if self.keyboard.keys.is_empty() {
            self.keyboard.keys = vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::F,
                gdx_keys::V,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
            ];
        }
        if self.keyboard.keys.len() != keys {
            self.keyboard.keys.resize(keys, 0);
        }
        self.keyboard.duration = self.keyboard.duration.clamp(0, 100);

        let mousescratch = &mut self.keyboard.mouse_scratch_config;
        if mousescratch.keys.len() != keys {
            mousescratch.keys = vec![-1; keys];
        }
        mousescratch.mouse_scratch_distance = mousescratch.mouse_scratch_distance.clamp(1, 10000);
        mousescratch.mouse_scratch_time_threshold =
            mousescratch.mouse_scratch_time_threshold.clamp(1, 10000);

        let mut index = 0usize;
        for c in &mut self.controller {
            if c.keys.is_empty() {
                c.keys = vec![
                    BMKeys::BUTTON_4,
                    BMKeys::BUTTON_7,
                    BMKeys::BUTTON_3,
                    BMKeys::BUTTON_8,
                    BMKeys::BUTTON_2,
                    BMKeys::BUTTON_5,
                    BMKeys::AXIS2_PLUS,
                    BMKeys::AXIS1_PLUS,
                    BMKeys::AXIS1_MINUS,
                ];
            }
            if c.keys.len() != keys {
                let mut newkeys = vec![-1i32; keys];
                for i in 0..c.keys.len() {
                    if index < newkeys.len() {
                        newkeys[index] = c.keys[i];
                        index += 1;
                    }
                }
                c.keys = newkeys;
            }
            c.duration = c.duration.clamp(0, 100);
        }

        // Button count extension (16->32) conversion (0.8.1 -> 0.8.2)
        if self.version == 0 {
            for c in &mut self.controller {
                for i in 0..c.keys.len() {
                    if c.keys[i] >= BMKeys::BUTTON_17 && c.keys[i] <= BMKeys::BUTTON_20 {
                        c.keys[i] += BMKeys::AXIS1_PLUS - BMKeys::BUTTON_17;
                    }
                }
            }
            self.version = 1;
        }

        if self.midi.keys.is_empty() {
            self.midi = MidiConfig::new(Mode::BEAT_7K, true);
        }
        if self.midi.keys.len() != keys {
            self.midi.keys.resize(keys, None);
        }

        // Exclusive processing for KB, controller, Midi buttons
        let mut exclusive = vec![false; self.keyboard.keys.len()];
        validate_exclusive(&mut self.keyboard.keys, &mut exclusive);
        for i in 0..self.controller.len() {
            // Need to use index-based access to avoid borrow issues
            let controller_keys = &mut self.controller[i].keys;
            validate_exclusive(controller_keys, &mut exclusive);
        }

        for i in 0..self.midi.keys.len() {
            if exclusive[i] {
                self.midi.keys[i] = None;
            }
        }
    }
}

// Compatibility getters for stub API
impl PlayModeConfig {
    pub fn get_playconfig(&self) -> &PlayConfig {
        &self.playconfig
    }

    pub fn get_playconfig_mut(&mut self) -> &mut PlayConfig {
        &mut self.playconfig
    }

    pub fn get_keyboard_config(&self) -> &KeyboardConfig {
        &self.keyboard
    }

    pub fn get_controller(&self) -> &[ControllerConfig] {
        &self.controller
    }

    pub fn get_controller_mut(&mut self) -> &mut [ControllerConfig] {
        &mut self.controller
    }

    pub fn get_midi_config(&self) -> &MidiConfig {
        &self.midi
    }

    pub fn get_midi_config_mut(&mut self) -> &mut MidiConfig {
        &mut self.midi
    }
}

fn validate_exclusive(keys: &mut [i32], exclusive: &mut [bool]) {
    for i in 0..exclusive.len().min(keys.len()) {
        if exclusive[i] {
            keys[i] = -1;
        } else if keys[i] != -1 {
            exclusive[i] = true;
        }
    }
}

// -- KeyboardConfig --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {
    #[serde(rename = "mouseScratchConfig")]
    pub mouse_scratch_config: MouseScratchConfig,
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub duration: i32,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        KeyboardConfig::new(Mode::BEAT_14K, true)
    }
}

impl KeyboardConfig {
    pub fn new(mode: Mode, enable: bool) -> Self {
        let mut config = KeyboardConfig {
            mouse_scratch_config: MouseScratchConfig::new(mode.clone()),
            keys: Vec::new(),
            start: 0,
            select: 0,
            duration: 16,
        };
        config.set_key_assign(mode, enable);
        config
    }

    pub fn set_key_assign(&mut self, mode: Mode, enable: bool) {
        self.keys = match mode {
            Mode::BEAT_5K => vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
            ],
            Mode::BEAT_7K => vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::F,
                gdx_keys::V,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
            ],
            Mode::BEAT_10K => vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
                gdx_keys::COMMA,
                gdx_keys::L,
                gdx_keys::PERIOD,
                gdx_keys::SEMICOLON,
                gdx_keys::SLASH,
                gdx_keys::SHIFT_RIGHT,
                gdx_keys::CONTROL_RIGHT,
            ],
            Mode::POPN_5K | Mode::POPN_9K => vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::F,
                gdx_keys::V,
                gdx_keys::G,
                gdx_keys::B,
            ],
            Mode::KEYBOARD_24K => {
                let mut keys = vec![
                    gdx_keys::Z,
                    gdx_keys::S,
                    gdx_keys::X,
                    gdx_keys::D,
                    gdx_keys::C,
                    gdx_keys::F,
                    gdx_keys::V,
                    gdx_keys::SHIFT_LEFT,
                    gdx_keys::CONTROL_LEFT,
                    gdx_keys::COMMA,
                    gdx_keys::L,
                    gdx_keys::PERIOD,
                    gdx_keys::SEMICOLON,
                    gdx_keys::SLASH,
                    gdx_keys::APOSTROPHE,
                    gdx_keys::UNKNOWN,
                    gdx_keys::SHIFT_RIGHT,
                    gdx_keys::CONTROL_RIGHT,
                ];
                keys.resize(26, 0);
                keys
            }
            Mode::KEYBOARD_24K_DOUBLE => {
                let mut keys = vec![
                    gdx_keys::Z,
                    gdx_keys::S,
                    gdx_keys::X,
                    gdx_keys::D,
                    gdx_keys::C,
                    gdx_keys::F,
                    gdx_keys::V,
                    gdx_keys::SHIFT_LEFT,
                    gdx_keys::CONTROL_LEFT,
                    gdx_keys::COMMA,
                    gdx_keys::L,
                    gdx_keys::PERIOD,
                    gdx_keys::SEMICOLON,
                    gdx_keys::SLASH,
                    gdx_keys::APOSTROPHE,
                    gdx_keys::UNKNOWN,
                    gdx_keys::SHIFT_RIGHT,
                    gdx_keys::CONTROL_RIGHT,
                ];
                keys.resize(52, 0);
                keys
            }
            // BEAT_14K and default
            _ => vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::F,
                gdx_keys::V,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
                gdx_keys::COMMA,
                gdx_keys::L,
                gdx_keys::PERIOD,
                gdx_keys::SEMICOLON,
                gdx_keys::SLASH,
                gdx_keys::APOSTROPHE,
                gdx_keys::UNKNOWN,
                gdx_keys::SHIFT_RIGHT,
                gdx_keys::CONTROL_RIGHT,
            ],
        };
        if !enable {
            for k in &mut self.keys {
                *k = -1;
            }
        }
        self.start = gdx_keys::Q;
        self.select = gdx_keys::W;
    }
}

// Compatibility getters for stub API
impl KeyboardConfig {
    pub fn get_key_assign(&self) -> &[i32] {
        &self.keys
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

// -- MouseScratchConfig --

pub const MOUSE_SCRATCH_VER_2: i32 = 0;
pub const MOUSE_SCRATCH_VER_1: i32 = 1;

const MOUSESCRATCH_STRING: [&str; 4] = ["MOUSE RIGHT", "MOUSE LEFT", "MOUSE DOWN", "MOUSE UP"];

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MouseScratchConfig {
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    #[serde(rename = "mouseScratchEnabled")]
    pub mouse_scratch_enabled: bool,
    #[serde(rename = "mouseScratchTimeThreshold")]
    pub mouse_scratch_time_threshold: i32,
    #[serde(rename = "mouseScratchDistance")]
    pub mouse_scratch_distance: i32,
    #[serde(rename = "mouseScratchMode")]
    pub mouse_scratch_mode: i32,
}

impl Default for MouseScratchConfig {
    fn default() -> Self {
        MouseScratchConfig::new(Mode::BEAT_7K)
    }
}

impl MouseScratchConfig {
    pub fn new(mode: Mode) -> Self {
        let mut config = MouseScratchConfig {
            keys: Vec::new(),
            start: -1,
            select: -1,
            mouse_scratch_enabled: false,
            mouse_scratch_time_threshold: 150,
            mouse_scratch_distance: 12,
            mouse_scratch_mode: 0,
        };
        config.set_key_assign(mode);
        config
    }

    pub fn set_key_assign(&mut self, mode: Mode) {
        let len = match mode {
            Mode::BEAT_5K => 7,
            Mode::BEAT_7K => 9,
            Mode::BEAT_10K => 14,
            Mode::POPN_5K | Mode::POPN_9K => 9,
            Mode::KEYBOARD_24K => 26,
            Mode::KEYBOARD_24K_DOUBLE => 52,
            // BEAT_14K and default
            _ => 18,
        };
        self.keys = vec![-1; len];
        self.start = -1;
        self.select = -1;
    }

    pub fn get_key_string(&self, index: usize) -> Option<&'static str> {
        if self.keys[index] == -1 {
            return None;
        }
        Some(MOUSESCRATCH_STRING[self.keys[index] as usize])
    }

    pub fn get_start_string(&self) -> Option<&'static str> {
        if self.start == -1 {
            return None;
        }
        Some(MOUSESCRATCH_STRING[self.start as usize])
    }

    pub fn get_select_string(&self) -> Option<&'static str> {
        if self.select == -1 {
            return None;
        }
        Some(MOUSESCRATCH_STRING[self.select as usize])
    }

    pub fn set_mouse_scratch_time_threshold(&mut self, value: i32) {
        self.mouse_scratch_time_threshold = if value > 0 { value } else { 1 };
    }

    pub fn set_mouse_scratch_distance(&mut self, value: i32) {
        self.mouse_scratch_distance = if value > 0 { value } else { 1 };
    }
}

// Compatibility getters and constants for stub API
impl MouseScratchConfig {
    pub const MOUSE_SCRATCH_VER_1: i32 = MOUSE_SCRATCH_VER_1;
    pub const MOUSE_SCRATCH_VER_2: i32 = MOUSE_SCRATCH_VER_2;

    pub fn get_key_assign(&self) -> &[i32] {
        &self.keys
    }

    pub fn get_start(&self) -> i32 {
        self.start
    }

    pub fn get_select(&self) -> i32 {
        self.select
    }

    pub fn is_mouse_scratch_enabled(&self) -> bool {
        self.mouse_scratch_enabled
    }

    pub fn get_mouse_scratch_time_threshold(&self) -> i32 {
        self.mouse_scratch_time_threshold
    }

    pub fn get_mouse_scratch_distance(&self) -> i32 {
        self.mouse_scratch_distance
    }

    pub fn get_mouse_scratch_mode(&self) -> i32 {
        self.mouse_scratch_mode
    }
}

// -- ControllerConfig --

pub const ANALOG_SCRATCH_VER_2: i32 = 0;
pub const ANALOG_SCRATCH_VER_1: i32 = 1;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct ControllerConfig {
    pub name: String,
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub duration: i32,
    #[serde(rename = "jkocHack")]
    pub jkoc_hack: bool,
    #[serde(rename = "analogScratch")]
    pub analog_scratch: bool,
    #[serde(rename = "analogScratchMode")]
    pub analog_scratch_mode: i32,
    #[serde(rename = "analogScratchThreshold")]
    pub analog_scratch_threshold: i32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, true)
    }
}

// Static controller presets
fn iidx_ps2_keys() -> Vec<i32> {
    vec![
        BMKeys::BUTTON_4,
        BMKeys::BUTTON_7,
        BMKeys::BUTTON_3,
        BMKeys::BUTTON_8,
        BMKeys::BUTTON_2,
        BMKeys::BUTTON_5,
        BMKeys::AXIS4_MINUS,
        BMKeys::AXIS3_MINUS,
        BMKeys::AXIS3_PLUS,
    ]
}

fn iidx_ps2_start() -> i32 {
    BMKeys::BUTTON_9
}

fn iidx_ps2_select() -> i32 {
    BMKeys::BUTTON_10
}

impl ControllerConfig {
    pub fn new_with_keys(keys: Vec<i32>, start: i32, select: i32) -> Self {
        ControllerConfig {
            name: String::new(),
            keys,
            start,
            select,
            duration: 16,
            jkoc_hack: false,
            analog_scratch: false,
            analog_scratch_mode: 0,
            analog_scratch_threshold: 50,
        }
    }

    pub fn new_with_mode(mode: Mode, player: i32, enable: bool) -> Self {
        let mut config = ControllerConfig {
            name: String::new(),
            keys: Vec::new(),
            start: 0,
            select: 0,
            duration: 16,
            jkoc_hack: false,
            analog_scratch: false,
            analog_scratch_mode: 0,
            analog_scratch_threshold: 50,
        };
        config.set_key_assign(mode, player, enable);
        config
    }

    #[allow(unreachable_patterns)]
    pub fn set_key_assign(&mut self, mode: Mode, player: i32, enable: bool) {
        let con_keys = iidx_ps2_keys();
        if player == 0 {
            self.keys = match mode {
                Mode::BEAT_5K => vec![
                    con_keys[0],
                    con_keys[1],
                    con_keys[2],
                    con_keys[3],
                    con_keys[4],
                    con_keys[7],
                    con_keys[8],
                ],
                Mode::BEAT_7K | Mode::POPN_5K | Mode::POPN_9K => vec![
                    con_keys[0],
                    con_keys[1],
                    con_keys[2],
                    con_keys[3],
                    con_keys[4],
                    con_keys[5],
                    con_keys[6],
                    con_keys[7],
                    con_keys[8],
                ],
                Mode::BEAT_10K => vec![
                    con_keys[0],
                    con_keys[1],
                    con_keys[2],
                    con_keys[3],
                    con_keys[4],
                    con_keys[7],
                    con_keys[8],
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                ],
                Mode::BEAT_14K => vec![
                    con_keys[0],
                    con_keys[1],
                    con_keys[2],
                    con_keys[3],
                    con_keys[4],
                    con_keys[5],
                    con_keys[6],
                    con_keys[7],
                    con_keys[8],
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                    -1,
                ],
                Mode::KEYBOARD_24K => {
                    let mut keys = con_keys.clone();
                    keys.resize(26, 0);
                    keys
                }
                Mode::KEYBOARD_24K_DOUBLE => {
                    let mut keys = con_keys.clone();
                    keys.resize(52, 0);
                    keys
                }
                _ => vec![
                    con_keys[0],
                    con_keys[1],
                    con_keys[2],
                    con_keys[3],
                    con_keys[4],
                    con_keys[5],
                    con_keys[6],
                    con_keys[7],
                    con_keys[8],
                ],
            };
        } else {
            self.keys = match mode {
                Mode::BEAT_5K | Mode::BEAT_7K | Mode::POPN_5K | Mode::POPN_9K => {
                    vec![-1; 9]
                }
                Mode::BEAT_10K => {
                    vec![
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        con_keys[0],
                        con_keys[1],
                        con_keys[2],
                        con_keys[3],
                        con_keys[4],
                        con_keys[7],
                        con_keys[8],
                    ]
                }
                Mode::BEAT_14K => {
                    vec![
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        con_keys[0],
                        con_keys[1],
                        con_keys[2],
                        con_keys[3],
                        con_keys[4],
                        con_keys[5],
                        con_keys[6],
                        con_keys[7],
                        con_keys[8],
                    ]
                }
                Mode::KEYBOARD_24K => {
                    let mut keys = vec![
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        con_keys[0],
                        con_keys[1],
                        con_keys[2],
                        con_keys[3],
                        con_keys[4],
                        con_keys[5],
                        con_keys[6],
                        con_keys[7],
                        con_keys[8],
                    ];
                    keys.resize(26, 0);
                    keys
                }
                Mode::KEYBOARD_24K_DOUBLE => {
                    let mut keys = vec![
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        -1,
                        con_keys[0],
                        con_keys[1],
                        con_keys[2],
                        con_keys[3],
                        con_keys[4],
                        con_keys[5],
                        con_keys[6],
                        con_keys[7],
                        con_keys[8],
                    ];
                    keys.resize(52, 0);
                    keys
                }
                _ => {
                    vec![-1; 9]
                }
            };
        }
        if !enable {
            for k in &mut self.keys {
                *k = -1;
            }
        }
        self.start = iidx_ps2_start();
        self.select = iidx_ps2_select();
    }

    pub fn set_analog_scratch_threshold(&mut self, value: i32) {
        self.analog_scratch_threshold = if value > 0 {
            if value <= 1000 { value } else { 1000 }
        } else {
            1
        };
    }
}

// Compatibility getters and constants for stub API
impl ControllerConfig {
    pub const ANALOG_SCRATCH_VER_1: i32 = ANALOG_SCRATCH_VER_1;
    pub const ANALOG_SCRATCH_VER_2: i32 = ANALOG_SCRATCH_VER_2;

    pub fn get_key_assign(&self) -> &[i32] {
        &self.keys
    }

    pub fn get_name(&self) -> Option<&str> {
        if self.name.is_empty() {
            None
        } else {
            Some(&self.name)
        }
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
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
        self.jkoc_hack
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

// -- MidiConfig --

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MidiConfig {
    pub keys: Vec<Option<MidiInput>>,
    pub start: Option<MidiInput>,
    pub select: Option<MidiInput>,
}

impl Default for MidiConfig {
    fn default() -> Self {
        MidiConfig::new(Mode::BEAT_7K, true)
    }
}

#[allow(non_camel_case_types)]
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum MidiInputType {
    #[default]
    NOTE,
    PITCH_BEND,
    CONTROL_CHANGE,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct MidiInput {
    #[serde(rename = "type")]
    pub input_type: MidiInputType,
    pub value: i32,
}

impl Default for MidiInput {
    fn default() -> Self {
        MidiInput {
            input_type: MidiInputType::NOTE,
            value: 0,
        }
    }
}

impl MidiInput {
    pub fn new(input_type: MidiInputType, value: i32) -> Self {
        MidiInput { input_type, value }
    }

    pub fn copy_from(&self) -> Self {
        self.clone()
    }
}

impl std::fmt::Display for MidiInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.input_type {
            MidiInputType::NOTE => write!(f, "NOTE {}", self.value),
            MidiInputType::PITCH_BEND => {
                if self.value > 0 {
                    write!(f, "PITCH +")
                } else {
                    write!(f, "PITCH -")
                }
            }
            MidiInputType::CONTROL_CHANGE => write!(f, "CC {}", self.value),
        }
    }
}

impl MidiConfig {
    pub fn new(mode: Mode, enable: bool) -> Self {
        let mut config = MidiConfig {
            keys: Vec::new(),
            start: None,
            select: None,
        };
        config.set_key_assign(mode, enable);
        config
    }

    #[allow(unreachable_patterns)]
    pub fn set_key_assign(&mut self, mode: Mode, enable: bool) {
        match mode {
            Mode::BEAT_5K => {
                self.keys = Vec::with_capacity(7);
                for i in 0..5 {
                    self.keys
                        .push(Some(MidiInput::new(MidiInputType::NOTE, 53 + i)));
                }
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 49)));
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 51)));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::BEAT_7K => {
                self.keys = Vec::with_capacity(9);
                for i in 0..7 {
                    self.keys
                        .push(Some(MidiInput::new(MidiInputType::NOTE, 53 + i)));
                }
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 49)));
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 51)));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::BEAT_10K => {
                self.keys = vec![None; 14];
                for i in 0..5 {
                    // 1P keys
                    self.keys[i] = Some(MidiInput::new(MidiInputType::NOTE, 53 + i as i32));
                    // 2P keys
                    self.keys[7 + i] = Some(MidiInput::new(MidiInputType::NOTE, 65 + i as i32));
                }
                // 1P turntables
                self.keys[5] = Some(MidiInput::new(MidiInputType::NOTE, 49));
                self.keys[6] = Some(MidiInput::new(MidiInputType::NOTE, 51));
                // 2P turntables
                self.keys[12] = Some(MidiInput::new(MidiInputType::NOTE, 73));
                self.keys[13] = Some(MidiInput::new(MidiInputType::NOTE, 75));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::BEAT_14K => {
                self.keys = vec![None; 18];
                for i in 0..7 {
                    // 1P keys
                    self.keys[i] = Some(MidiInput::new(MidiInputType::NOTE, 53 + i as i32));
                    // 2P keys
                    self.keys[9 + i] = Some(MidiInput::new(MidiInputType::NOTE, 65 + i as i32));
                }
                // 1P turntables
                self.keys[7] = Some(MidiInput::new(MidiInputType::NOTE, 49));
                self.keys[8] = Some(MidiInput::new(MidiInputType::NOTE, 51));
                // 2P turntables
                self.keys[16] = Some(MidiInput::new(MidiInputType::NOTE, 73));
                self.keys[17] = Some(MidiInput::new(MidiInputType::NOTE, 75));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::POPN_5K | Mode::POPN_9K => {
                self.keys = Vec::with_capacity(9);
                for i in 0..9 {
                    self.keys
                        .push(Some(MidiInput::new(MidiInputType::NOTE, 52 + i)));
                }
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::KEYBOARD_24K => {
                self.keys = vec![None; 26];
                for i in 0..24 {
                    self.keys[i] = Some(MidiInput::new(MidiInputType::NOTE, 48 + i as i32));
                }
                self.keys[24] = Some(MidiInput::new(MidiInputType::PITCH_BEND, 1));
                self.keys[25] = Some(MidiInput::new(MidiInputType::PITCH_BEND, -1));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 44));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 46));
            }
            Mode::KEYBOARD_24K_DOUBLE => {
                self.keys = vec![None; 52];
                for i in 0..24 {
                    self.keys[i] = Some(MidiInput::new(MidiInputType::NOTE, 48 + i as i32));
                    self.keys[i + 26] = Some(MidiInput::new(MidiInputType::NOTE, 72 + i as i32));
                }
                self.keys[24] = Some(MidiInput::new(MidiInputType::PITCH_BEND, 1));
                self.keys[25] = Some(MidiInput::new(MidiInputType::PITCH_BEND, -1));
                self.keys[50] = Some(MidiInput::new(MidiInputType::NOTE, 99));
                self.keys[51] = Some(MidiInput::new(MidiInputType::NOTE, 97));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 44));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 46));
            }
            _ => {
                // Default same as BEAT_7K
                self.keys = Vec::with_capacity(9);
                for i in 0..7 {
                    self.keys
                        .push(Some(MidiInput::new(MidiInputType::NOTE, 53 + i)));
                }
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 49)));
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 51)));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
        }
        if !enable {
            for k in &mut self.keys {
                *k = None;
            }
        }
    }
}

// Compatibility getters for stub API
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
