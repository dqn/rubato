use beatoraja_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, MouseScratchConfig, PlayModeConfig,
};

use crate::main_controller::MainController;
use crate::main_state::{MainState, MainStateData, MainStateType};
use crate::timer_manager::TimerManager;

// Key configuration screen.
// Translated from Java: KeyConfiguration extends MainState
//
// This is heavily dependent on libGDX UI (SpriteBatch, BitmapFont, ShapeDrawer, etc.)
// and input processing (BMSPlayerInputProcessor, BMControllerInputProcessor, MidiInputProcessor).
// Most rendering and input methods are stubbed pending Phase 5+ graphics integration.

static MODE: &[&str] = &[
    "5 KEYS",
    "7 KEYS",
    "9 KEYS",
    "10 KEYS",
    "14 KEYS",
    "24 KEYS",
    "24 KEYS DOUBLE",
];

static KEYS: &[&[&str]] = &[
    &[
        "1 KEY", "2 KEY", "3 KEY", "4 KEY", "5 KEY", "F-SCR", "R-SCR", "START", "SELECT",
    ],
    &[
        "1 KEY", "2 KEY", "3 KEY", "4 KEY", "5 KEY", "6 KEY", "7 KEY", "F-SCR", "R-SCR", "START",
        "SELECT",
    ],
    &[
        "1 KEY", "2 KEY", "3 KEY", "4 KEY", "5 KEY", "6 KEY", "7 KEY", "8 KEY", "9 KEY", "START",
        "SELECT",
    ],
    &[
        "1P-1 KEY", "1P-2 KEY", "1P-3 KEY", "1P-4 KEY", "1P-5 KEY", "1P-F-SCR", "1P-R-SCR",
        "2P-1 KEY", "2P-2 KEY", "2P-3 KEY", "2P-4 KEY", "2P-5 KEY", "2P-F-SCR", "2P-R-SCR",
        "START", "SELECT",
    ],
    &[
        "1P-1 KEY", "1P-2 KEY", "1P-3 KEY", "1P-4 KEY", "1P-5 KEY", "1P-6 KEY", "1P-7 KEY",
        "1P-F-SCR", "1P-R-SCR", "2P-1 KEY", "2P-2 KEY", "2P-3 KEY", "2P-4 KEY", "2P-5 KEY",
        "2P-6 KEY", "2P-7 KEY", "2P-F-SCR", "2P-R-SCR", "START", "SELECT",
    ],
    &[
        "C1",
        "C#1",
        "D1",
        "D#1",
        "E1",
        "F1",
        "F#1",
        "G1",
        "G#1",
        "A1",
        "A#1",
        "B1",
        "C2",
        "C#2",
        "D2",
        "D#2",
        "E2",
        "F2",
        "F#2",
        "G2",
        "G#2",
        "A2",
        "A#2",
        "B2",
        "WHEEL-UP",
        "WHEEL-DOWN",
        "START",
        "SELECT",
    ],
    &[
        "1P-C1",
        "1P-C#1",
        "1P-D1",
        "1P-D#1",
        "1P-E1",
        "1P-F1",
        "1P-F#1",
        "1P-G1",
        "1P-G#1",
        "1P-A1",
        "1P-A#1",
        "1P-B1",
        "1P-C2",
        "1P-C#2",
        "1P-D2",
        "1P-D#2",
        "1P-E2",
        "1P-F2",
        "1P-F#2",
        "1P-G2",
        "1P-G#2",
        "1P-A2",
        "1P-A#2",
        "1P-B2",
        "1P-WHEEL-UP",
        "1P-WHEEL-DOWN",
        "2P-C1",
        "2P-C#1",
        "2P-D1",
        "2P-D#1",
        "2P-E1",
        "2P-F1",
        "2P-F#1",
        "2P-G1",
        "2P-G#1",
        "2P-A1",
        "2P-A#1",
        "2P-B1",
        "2P-C2",
        "2P-C#2",
        "2P-D2",
        "2P-D#2",
        "2P-E2",
        "2P-F2",
        "2P-F#2",
        "2P-G2",
        "2P-G#2",
        "2P-A2",
        "2P-A#2",
        "2P-B2",
        "2P-WHEEL-UP",
        "2P-WHEEL-DOWN",
        "START",
        "SELECT",
    ],
];

static KEYSA: &[&[i32]] = &[
    &[0, 1, 2, 3, 4, 5, 6, -1, -2],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, -1, -2],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, -1, -2],
    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, -1, -2],
    &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, -1, -2,
    ],
    &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, -1, -2,
    ],
    &[
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47,
        48, 49, 50, 51, -1, -2,
    ],
];

#[allow(dead_code)]
static PLAYER_OFFSET: i32 = 100;

#[allow(dead_code)]
static SELECTKEY: &[&str] = &["7 KEYS", "9 KEYS", "14 KEYS"];

#[allow(dead_code)]
pub struct KeyConfiguration {
    state_data: MainStateData,
    cursorpos: usize,
    scrollpos: usize,
    keyinput: bool,
    mode: usize,
    deletepressed: bool,
    // References to input processors and config are Phase 5+ types
    // Stubbed for now
}

impl KeyConfiguration {
    pub fn new(_main: &MainController) -> Self {
        Self {
            state_data: MainStateData::new(TimerManager::new()),
            cursorpos: 0,
            scrollpos: 0,
            keyinput: false,
            mode: 0,
            deletepressed: false,
        }
    }

    pub fn create(&mut self) {
        // TODO: loadSkin, font generation, input processor setup
        // Requires Phase 5+ types (SkinType, SkinHeader, FreeTypeFontGenerator, BMSPlayerInputProcessor)
        log::warn!("not yet implemented: KeyConfiguration::create requires Phase 5+ UI types");
    }

    pub fn render(&mut self) {
        // TODO: Full rendering with SpriteBatch, ShapeDrawer
        // Requires Phase 5+ graphics types
        log::warn!("not yet implemented: KeyConfiguration::render requires Phase 5+ UI types");
    }

    pub fn set_key_assign_mode(&mut self, index: usize) {
        self.cursorpos = index;
        self.keyinput = true;
    }

    pub fn get_key_assign(&self, index: usize) -> &str {
        if index >= KEYSA[self.mode].len() {
            return "!!!";
        }
        // TODO: requires input processor state
        "---"
    }

    pub fn get_mode(&self) -> usize {
        self.mode
    }

    pub fn get_mode_name(&self) -> &str {
        MODE[self.mode]
    }

    pub fn get_keys(&self) -> &[&str] {
        KEYS[self.mode]
    }

    pub fn get_keysa(&self) -> &[i32] {
        KEYSA[self.mode]
    }

    pub fn dispose_resources(&mut self) {
        // TODO: dispose font resources
    }

    // -- Getters --

    /// Returns the keyboard key assigned at the given index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: 0.
    ///
    /// Java: KeyConfiguration.getKeyboardKeyAssign(int index)
    pub fn get_keyboard_key_assign(kb: &KeyboardConfig, index: i32) -> i32 {
        if index >= 0 {
            kb.keys.get(index as usize).copied().unwrap_or(0)
        } else if index == -1 {
            kb.start
        } else if index == -2 {
            kb.select
        } else {
            0
        }
    }

    /// Returns the controller key assigned at the given device and index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: 0.
    ///
    /// Java: KeyConfiguration.getControllerKeyAssign(int device, int index)
    pub fn get_controller_key_assign(
        controllers: &[ControllerConfig],
        device: usize,
        index: i32,
    ) -> i32 {
        let cc = match controllers.get(device) {
            Some(c) => c,
            None => return 0,
        };
        if index >= 0 {
            cc.keys.get(index as usize).copied().unwrap_or(0)
        } else if index == -1 {
            cc.start
        } else if index == -2 {
            cc.select
        } else {
            0
        }
    }

    /// Returns the MIDI input assigned at the given index.
    /// Positive index: keys[index]. -1: start. -2: select. Other: default.
    ///
    /// Java: KeyConfiguration.getMidiKeyAssign(int index)
    pub fn get_midi_key_assign(midi: &MidiConfig, index: i32) -> MidiInput {
        if index >= 0 {
            midi.keys
                .get(index as usize)
                .and_then(|m| m.clone())
                .unwrap_or_default()
        } else if index == -1 {
            midi.start.clone().unwrap_or_default()
        } else if index == -2 {
            midi.select.clone().unwrap_or_default()
        } else {
            MidiInput::default()
        }
    }

    /// Returns the mouse scratch key string at the given index, or `default` if none.
    ///
    /// Java: KeyConfiguration.getMouseScratchKeyString(int index, String defaultKeyString)
    pub fn get_mouse_scratch_key_string(
        msc: &MouseScratchConfig,
        index: i32,
        default: Option<&str>,
    ) -> Option<String> {
        let result = if index >= 0 {
            msc.get_key_string(index as usize)
        } else if index == -1 {
            msc.get_start_string()
        } else if index == -2 {
            msc.get_select_string()
        } else {
            None
        };
        result
            .map(|s| s.to_string())
            .or_else(|| default.map(|s| s.to_string()))
    }

    // -- Mutation helpers --

    /// Clears key assignment at `index` across ALL input devices (keyboard, controller,
    /// mouse scratch, MIDI). Only handles positive indices (not start/select).
    ///
    /// Java: KeyConfiguration.resetKeyAssign(int index)
    pub fn reset_key_assign(pmc: &mut PlayModeConfig, index: i32) {
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.keyboard.keys.len() {
                pmc.keyboard.keys[idx] = -1;
            }
            for cc in pmc.controller.iter_mut() {
                if idx < cc.keys.len() {
                    cc.keys[idx] = -1;
                }
            }
            if idx < pmc.keyboard.mouse_scratch_config.keys.len() {
                pmc.keyboard.mouse_scratch_config.keys[idx] = -1;
            }
            if idx < pmc.midi.keys.len() {
                pmc.midi.keys[idx] = None;
            }
        }
    }

    /// Deletes key assignment at `index` across ALL input devices.
    /// Handles positive indices, start(-1), and select(-2).
    ///
    /// Java: KeyConfiguration.deleteKeyAssign(int index)
    pub fn delete_key_assign(pmc: &mut PlayModeConfig, index: i32) {
        const NO_ASSIGN: i32 = -1;
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.keyboard.keys.len() {
                pmc.keyboard.keys[idx] = NO_ASSIGN;
            }
            if idx < pmc.keyboard.mouse_scratch_config.keys.len() {
                pmc.keyboard.mouse_scratch_config.keys[idx] = NO_ASSIGN;
            }
            for cc in pmc.controller.iter_mut() {
                if idx < cc.keys.len() {
                    cc.keys[idx] = NO_ASSIGN;
                }
            }
            if idx < pmc.midi.keys.len() {
                pmc.midi.keys[idx] = None;
            }
        } else if index == -1 {
            pmc.keyboard.mouse_scratch_config.start = NO_ASSIGN;
            for cc in pmc.controller.iter_mut() {
                cc.start = NO_ASSIGN;
            }
            pmc.midi.start = None;
        } else if index == -2 {
            pmc.keyboard.mouse_scratch_config.select = NO_ASSIGN;
            for cc in pmc.controller.iter_mut() {
                cc.select = NO_ASSIGN;
            }
            pmc.midi.select = None;
        }
    }

    // -- Setters --

    /// Assigns a keyboard key at the given index.
    /// Takes the last pressed key directly (caller provides from keyboard input processor).
    ///
    /// Java: KeyConfiguration.setKeyboardKeyAssign(int index)
    pub fn set_keyboard_key_assign(
        pmc: &mut PlayModeConfig,
        index: i32,
        last_pressed_key: i32,
        is_reserved: bool,
    ) {
        if is_reserved {
            return;
        }
        Self::reset_key_assign(pmc, index);
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.keyboard.keys.len() {
                pmc.keyboard.keys[idx] = last_pressed_key;
            }
        } else if index == -1 {
            pmc.keyboard.start = last_pressed_key;
        } else if index == -2 {
            pmc.keyboard.select = last_pressed_key;
        }
    }

    /// Assigns a controller key at the given index.
    /// Finds the controller by name and sets the key.
    ///
    /// Java: KeyConfiguration.setControllerKeyAssign(int index, BMControllerInputProcessor bmc)
    pub fn set_controller_key_assign(
        pmc: &mut PlayModeConfig,
        index: i32,
        controller_name: &str,
        last_pressed_button: i32,
    ) {
        let cindex = match pmc
            .controller
            .iter()
            .position(|c| c.name == controller_name)
        {
            Some(i) => i,
            None => return,
        };
        Self::reset_key_assign(pmc, index);
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.controller[cindex].keys.len() {
                pmc.controller[cindex].keys[idx] = last_pressed_button;
            }
        } else if index == -1 {
            pmc.controller[cindex].start = last_pressed_button;
        } else if index == -2 {
            pmc.controller[cindex].select = last_pressed_button;
        }
    }

    /// Assigns a MIDI key at the given index.
    ///
    /// Java: KeyConfiguration.setMidiKeyAssign(int index)
    pub fn set_midi_key_assign(
        pmc: &mut PlayModeConfig,
        index: i32,
        last_pressed: Option<MidiInput>,
    ) {
        Self::reset_key_assign(pmc, index);
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.midi.keys.len() {
                pmc.midi.keys[idx] = last_pressed;
            }
        } else if index == -1 {
            pmc.midi.start = last_pressed;
        } else if index == -2 {
            pmc.midi.select = last_pressed;
        }
    }

    /// Assigns a mouse scratch key at the given index.
    ///
    /// Java: KeyConfiguration.setMouseScratchKeyAssign(int index, KeyBoardInputProcesseor kbp)
    pub fn set_mouse_scratch_key_assign(
        pmc: &mut PlayModeConfig,
        index: i32,
        last_mouse_scratch: i32,
    ) {
        Self::reset_key_assign(pmc, index);
        if index >= 0 {
            let idx = index as usize;
            if idx < pmc.keyboard.mouse_scratch_config.keys.len() {
                pmc.keyboard.mouse_scratch_config.keys[idx] = last_mouse_scratch;
            }
        } else if index == -1 {
            pmc.keyboard.mouse_scratch_config.start = last_mouse_scratch;
        } else if index == -2 {
            pmc.keyboard.mouse_scratch_config.select = last_mouse_scratch;
        }
    }

    // -- Validators --

    /// Ensures keyboard keys array is long enough for the current mode.
    ///
    /// Java: KeyConfiguration.validateKeyboardLength()
    pub fn validate_keyboard_length(&self, kb: &mut KeyboardConfig) {
        let max_key = KEYSA[self.mode]
            .iter()
            .copied()
            .filter(|&k| k >= 0)
            .max()
            .unwrap_or(0);
        let needed = (max_key + 1) as usize;
        if kb.keys.len() < needed {
            kb.keys.resize(needed, 0);
        }
    }

    /// Ensures controller array has enough players and each controller has enough keys.
    ///
    /// Java: KeyConfiguration.validateControllerLength()
    pub fn validate_controller_length(&self, pmc: &mut PlayModeConfig) {
        let mut max_player: i32 = 0;
        let mut max_key: i32 = 0;
        for &key in KEYSA[self.mode] {
            if key >= 0 {
                if key / PLAYER_OFFSET > max_player {
                    max_player = key / PLAYER_OFFSET;
                }
                if key % PLAYER_OFFSET > max_key {
                    max_key = key % PLAYER_OFFSET;
                }
            }
        }
        let needed_players = (max_player + 1) as usize;
        while pmc.controller.len() < needed_players {
            pmc.controller.push(ControllerConfig::default());
        }
        let needed_keys = (max_key + 1) as usize;
        for cc in pmc.controller.iter_mut() {
            if cc.keys.len() < needed_keys {
                cc.keys.resize(needed_keys, -1);
            }
        }
    }

    /// Ensures MIDI keys array is long enough for the current mode.
    ///
    /// Java: KeyConfiguration.validateMidiLength()
    pub fn validate_midi_length(&self, midi: &mut MidiConfig) {
        let max_key = KEYSA[self.mode]
            .iter()
            .copied()
            .filter(|&k| k >= 0)
            .max()
            .unwrap_or(0);
        let needed = (max_key + 1) as usize;
        if midi.keys.len() < needed {
            midi.keys.resize(needed, None);
        }
    }
}

impl MainState for KeyConfiguration {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::Config)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        log::warn!(
            "TODO: Phase 22 - KeyConfiguration::create (skin loading, font generation, input processor setup)"
        );
    }

    fn render(&mut self) {
        log::warn!(
            "TODO: Phase 22 - KeyConfiguration::render (SpriteBatch, ShapeDrawer rendering)"
        );
    }

    fn input(&mut self) {
        log::warn!("TODO: Phase 22 - KeyConfiguration::input (key assignment input handling)");
    }

    fn dispose(&mut self) {
        self.dispose_resources();
        // Call default trait dispose for skin/stage cleanup
        let data = self.main_state_data_mut();
        data.skin = None;
        data.stage = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::play_mode_config::{MidiInput, MidiInputType, PlayModeConfig};
    use bms_model::mode::Mode;

    /// Creates a PlayModeConfig for 7K mode (mode index 1 in KEYSA).
    fn make_pmc() -> PlayModeConfig {
        PlayModeConfig::new(Mode::BEAT_7K)
    }

    /// Creates a KeyConfiguration with the given mode index, without requiring MainController.
    fn make_kc(mode: usize) -> KeyConfiguration {
        KeyConfiguration {
            state_data: MainStateData::new(TimerManager::new()),
            cursorpos: 0,
            scrollpos: 0,
            keyinput: false,
            mode,
            deletepressed: false,
        }
    }

    // -- Getter tests --

    #[test]
    fn test_get_keyboard_key_assign_positive_index() {
        let pmc = make_pmc();
        let kb = &pmc.keyboard;
        // Index 0 should return the first assigned key value
        let val = KeyConfiguration::get_keyboard_key_assign(kb, 0);
        assert_eq!(val, kb.keys[0]);
    }

    #[test]
    fn test_get_keyboard_key_assign_start() {
        let pmc = make_pmc();
        let kb = &pmc.keyboard;
        assert_eq!(KeyConfiguration::get_keyboard_key_assign(kb, -1), kb.start);
    }

    #[test]
    fn test_get_keyboard_key_assign_select() {
        let pmc = make_pmc();
        let kb = &pmc.keyboard;
        assert_eq!(KeyConfiguration::get_keyboard_key_assign(kb, -2), kb.select);
    }

    #[test]
    fn test_get_keyboard_key_assign_other_negative() {
        let pmc = make_pmc();
        assert_eq!(
            KeyConfiguration::get_keyboard_key_assign(&pmc.keyboard, -3),
            0
        );
    }

    #[test]
    fn test_get_keyboard_key_assign_out_of_bounds() {
        let pmc = make_pmc();
        assert_eq!(
            KeyConfiguration::get_keyboard_key_assign(&pmc.keyboard, 9999),
            0
        );
    }

    #[test]
    fn test_get_controller_key_assign_positive_index() {
        let pmc = make_pmc();
        let val = KeyConfiguration::get_controller_key_assign(&pmc.controller, 0, 0);
        assert_eq!(val, pmc.controller[0].keys[0]);
    }

    #[test]
    fn test_get_controller_key_assign_start_select() {
        let pmc = make_pmc();
        assert_eq!(
            KeyConfiguration::get_controller_key_assign(&pmc.controller, 0, -1),
            pmc.controller[0].start
        );
        assert_eq!(
            KeyConfiguration::get_controller_key_assign(&pmc.controller, 0, -2),
            pmc.controller[0].select
        );
    }

    #[test]
    fn test_get_controller_key_assign_no_device() {
        let pmc = make_pmc();
        assert_eq!(
            KeyConfiguration::get_controller_key_assign(&pmc.controller, 99, 0),
            0
        );
    }

    #[test]
    fn test_get_midi_key_assign_positive_index() {
        // PlayModeConfig::new(BEAT_7K) creates MIDI with enable=false (is_midi=false),
        // so all keys are None. Create an enabled MIDI config directly.
        let midi = MidiConfig::new(Mode::BEAT_7K, true);
        let mi = KeyConfiguration::get_midi_key_assign(&midi, 0);
        // BEAT_7K MIDI enabled: keys[0] = Some(MidiInput { NOTE, 53 })
        assert_eq!(mi.input_type, MidiInputType::NOTE);
        assert_eq!(mi.value, 53);
    }

    #[test]
    fn test_get_midi_key_assign_start() {
        let pmc = make_pmc();
        let mi = KeyConfiguration::get_midi_key_assign(&pmc.midi, -1);
        assert_eq!(mi.input_type, MidiInputType::NOTE);
        assert_eq!(mi.value, 47);
    }

    #[test]
    fn test_get_midi_key_assign_select() {
        let pmc = make_pmc();
        let mi = KeyConfiguration::get_midi_key_assign(&pmc.midi, -2);
        assert_eq!(mi.input_type, MidiInputType::NOTE);
        assert_eq!(mi.value, 48);
    }

    #[test]
    fn test_get_midi_key_assign_other_negative() {
        let mi = KeyConfiguration::get_midi_key_assign(&make_pmc().midi, -5);
        assert_eq!(mi.input_type, MidiInputType::NOTE);
        assert_eq!(mi.value, 0);
    }

    #[test]
    fn test_get_mouse_scratch_key_string_no_assignment() {
        let pmc = make_pmc();
        let msc = &pmc.keyboard.mouse_scratch_config;
        // Default mouse scratch keys are all -1, so should return default
        let result = KeyConfiguration::get_mouse_scratch_key_string(msc, 0, Some("fallback"));
        assert_eq!(result, Some("fallback".to_string()));
    }

    #[test]
    fn test_get_mouse_scratch_key_string_none_default() {
        let pmc = make_pmc();
        let msc = &pmc.keyboard.mouse_scratch_config;
        let result = KeyConfiguration::get_mouse_scratch_key_string(msc, 0, None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_get_mouse_scratch_key_string_with_assignment() {
        let mut pmc = make_pmc();
        // Assign mouse scratch key at index 0: 0 = "MOUSE RIGHT"
        pmc.keyboard.mouse_scratch_config.keys[0] = 0;
        let result = KeyConfiguration::get_mouse_scratch_key_string(
            &pmc.keyboard.mouse_scratch_config,
            0,
            Some("fallback"),
        );
        assert_eq!(result, Some("MOUSE RIGHT".to_string()));
    }

    // -- Mutation tests --

    #[test]
    fn test_reset_key_assign() {
        let mut pmc = make_pmc();
        // Set some values
        pmc.keyboard.keys[0] = 42;
        pmc.controller[0].keys[0] = 99;
        pmc.keyboard.mouse_scratch_config.keys[0] = 2;
        pmc.midi.keys[0] = Some(MidiInput::new(MidiInputType::NOTE, 60));

        KeyConfiguration::reset_key_assign(&mut pmc, 0);

        assert_eq!(pmc.keyboard.keys[0], -1);
        assert_eq!(pmc.controller[0].keys[0], -1);
        assert_eq!(pmc.keyboard.mouse_scratch_config.keys[0], -1);
        assert!(pmc.midi.keys[0].is_none());
    }

    #[test]
    fn test_reset_key_assign_negative_is_noop() {
        let mut pmc = make_pmc();
        let start_before = pmc.keyboard.start;
        KeyConfiguration::reset_key_assign(&mut pmc, -1);
        // Start should not be affected
        assert_eq!(pmc.keyboard.start, start_before);
    }

    #[test]
    fn test_delete_key_assign_positive() {
        let mut pmc = make_pmc();
        pmc.keyboard.keys[2] = 42;
        pmc.keyboard.mouse_scratch_config.keys[2] = 1;
        pmc.controller[0].keys[2] = 88;
        pmc.midi.keys[2] = Some(MidiInput::new(MidiInputType::NOTE, 60));

        KeyConfiguration::delete_key_assign(&mut pmc, 2);

        assert_eq!(pmc.keyboard.keys[2], -1);
        assert_eq!(pmc.keyboard.mouse_scratch_config.keys[2], -1);
        assert_eq!(pmc.controller[0].keys[2], -1);
        assert!(pmc.midi.keys[2].is_none());
    }

    #[test]
    fn test_delete_key_assign_start() {
        let mut pmc = make_pmc();
        pmc.keyboard.mouse_scratch_config.start = 2;
        pmc.controller[0].start = 7;
        pmc.midi.start = Some(MidiInput::new(MidiInputType::NOTE, 47));

        KeyConfiguration::delete_key_assign(&mut pmc, -1);

        assert_eq!(pmc.keyboard.mouse_scratch_config.start, -1);
        assert_eq!(pmc.controller[0].start, -1);
        assert!(pmc.midi.start.is_none());
    }

    #[test]
    fn test_delete_key_assign_select() {
        let mut pmc = make_pmc();
        pmc.keyboard.mouse_scratch_config.select = 3;
        pmc.controller[0].select = 8;
        pmc.midi.select = Some(MidiInput::new(MidiInputType::NOTE, 48));

        KeyConfiguration::delete_key_assign(&mut pmc, -2);

        assert_eq!(pmc.keyboard.mouse_scratch_config.select, -1);
        assert_eq!(pmc.controller[0].select, -1);
        assert!(pmc.midi.select.is_none());
    }

    // -- Setter tests --

    #[test]
    fn test_set_keyboard_key_assign_positive() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_keyboard_key_assign(&mut pmc, 0, 77, false);
        assert_eq!(pmc.keyboard.keys[0], 77);
        // Other devices should be reset at index 0
        assert_eq!(pmc.controller[0].keys[0], -1);
    }

    #[test]
    fn test_set_keyboard_key_assign_reserved() {
        let mut pmc = make_pmc();
        let original = pmc.keyboard.keys[0];
        KeyConfiguration::set_keyboard_key_assign(&mut pmc, 0, 77, true);
        // Should be unchanged because is_reserved is true
        assert_eq!(pmc.keyboard.keys[0], original);
    }

    #[test]
    fn test_set_keyboard_key_assign_start() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_keyboard_key_assign(&mut pmc, -1, 99, false);
        assert_eq!(pmc.keyboard.start, 99);
    }

    #[test]
    fn test_set_keyboard_key_assign_select() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_keyboard_key_assign(&mut pmc, -2, 88, false);
        assert_eq!(pmc.keyboard.select, 88);
    }

    #[test]
    fn test_set_controller_key_assign() {
        let mut pmc = make_pmc();
        let name = pmc.controller[0].name.clone();
        KeyConfiguration::set_controller_key_assign(&mut pmc, 0, &name, 55);
        assert_eq!(pmc.controller[0].keys[0], 55);
    }

    #[test]
    fn test_set_controller_key_assign_unknown_name() {
        let mut pmc = make_pmc();
        let original = pmc.controller[0].keys[0];
        KeyConfiguration::set_controller_key_assign(&mut pmc, 0, "nonexistent", 55);
        // Should be unchanged — name not found
        assert_eq!(pmc.controller[0].keys[0], original);
    }

    #[test]
    fn test_set_midi_key_assign_positive() {
        let mut pmc = make_pmc();
        let mi = Some(MidiInput::new(MidiInputType::CONTROL_CHANGE, 64));
        KeyConfiguration::set_midi_key_assign(&mut pmc, 0, mi);
        let assigned = pmc.midi.keys[0].as_ref().unwrap();
        assert_eq!(assigned.input_type, MidiInputType::CONTROL_CHANGE);
        assert_eq!(assigned.value, 64);
    }

    #[test]
    fn test_set_midi_key_assign_start() {
        let mut pmc = make_pmc();
        let mi = Some(MidiInput::new(MidiInputType::PITCH_BEND, 1));
        KeyConfiguration::set_midi_key_assign(&mut pmc, -1, mi);
        let assigned = pmc.midi.start.as_ref().unwrap();
        assert_eq!(assigned.input_type, MidiInputType::PITCH_BEND);
        assert_eq!(assigned.value, 1);
    }

    #[test]
    fn test_set_mouse_scratch_key_assign_positive() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, 0, 2);
        assert_eq!(pmc.keyboard.mouse_scratch_config.keys[0], 2);
    }

    #[test]
    fn test_set_mouse_scratch_key_assign_start() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, -1, 3);
        assert_eq!(pmc.keyboard.mouse_scratch_config.start, 3);
    }

    #[test]
    fn test_set_mouse_scratch_key_assign_select() {
        let mut pmc = make_pmc();
        KeyConfiguration::set_mouse_scratch_key_assign(&mut pmc, -2, 1);
        assert_eq!(pmc.keyboard.mouse_scratch_config.select, 1);
    }

    // -- Validator tests --

    #[test]
    fn test_validate_keyboard_length_expands() {
        let kc = make_kc(1); // mode 1 = 7K, KEYSA[1] max = 8
        let mut kb = KeyboardConfig::new(Mode::BEAT_7K, true);
        kb.keys.clear(); // Empty
        kc.validate_keyboard_length(&mut kb);
        // KEYSA[1] = [0,1,2,3,4,5,6,7,8,-1,-2], max positive = 8, needed = 9
        assert!(kb.keys.len() >= 9);
    }

    #[test]
    fn test_validate_keyboard_length_already_sufficient() {
        let kc = make_kc(1);
        let mut kb = KeyboardConfig::new(Mode::BEAT_7K, true);
        let original_len = kb.keys.len();
        kc.validate_keyboard_length(&mut kb);
        // Should not shrink
        assert_eq!(kb.keys.len(), original_len);
    }

    #[test]
    fn test_validate_controller_length_adds_players() {
        let kc = make_kc(4); // mode 4 = 14K, has keys > 100 (2P keys)
        let mut pmc = PlayModeConfig::new(Mode::BEAT_14K);
        pmc.controller.clear();
        kc.validate_controller_length(&mut pmc);
        // Should have at least 1 controller (single-player keys are < 100)
        assert!(!pmc.controller.is_empty());
    }

    #[test]
    fn test_validate_controller_length_expands_keys() {
        let kc = make_kc(1); // mode 1 = 7K
        let mut pmc = PlayModeConfig::new(Mode::BEAT_7K);
        for cc in pmc.controller.iter_mut() {
            cc.keys.clear();
        }
        kc.validate_controller_length(&mut pmc);
        // KEYSA[1] max key%100 = 8, so each controller needs at least 9 keys
        for cc in &pmc.controller {
            assert!(cc.keys.len() >= 9);
        }
    }

    #[test]
    fn test_validate_midi_length_expands() {
        let kc = make_kc(1);
        let mut midi = MidiConfig::new(Mode::BEAT_7K, true);
        midi.keys.clear();
        kc.validate_midi_length(&mut midi);
        assert!(midi.keys.len() >= 9);
    }

    #[test]
    fn test_validate_midi_length_already_sufficient() {
        let kc = make_kc(1);
        let mut midi = MidiConfig::new(Mode::BEAT_7K, true);
        let original_len = midi.keys.len();
        kc.validate_midi_length(&mut midi);
        assert_eq!(midi.keys.len(), original_len);
    }
}
