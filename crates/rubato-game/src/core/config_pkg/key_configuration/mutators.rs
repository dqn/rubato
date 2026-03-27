use rubato_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, PlayModeConfig,
};

use super::KeyConfiguration;
use super::constants::{KEYSA, PLAYER_OFFSET};

impl KeyConfiguration {
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
        // Java parity: keyboard.start/select are intentionally NOT cleared here
        // (only mouse_scratch, controller, and MIDI are cleared for START/SELECT deletion)
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
                cc.keys.resize(needed_keys, 0);
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
