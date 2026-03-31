mod controller_config;
mod keyboard_config;
mod midi_config;
mod mouse_scratch_config;

pub use controller_config::*;
pub use keyboard_config::*;
pub use midi_config::*;
pub use mouse_scratch_config::*;

use crate::skin::bm_keys::BMKeys;
use bms::model::mode::Mode;

use crate::skin::play_config::PlayConfig;

// libGDX Keys constants (from com.badlogic.gdx.Input.Keys)
pub(crate) mod gdx_keys {
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
        let keyboard = KeyboardConfig::new(mode, !is_midi);
        let player_count = mode.player() as usize;
        let controller: Vec<ControllerConfig> = (0..player_count)
            .map(|i| ControllerConfig::new_with_mode(mode, i as i32, false))
            .collect();
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
                for &key in &c.keys {
                    if index < newkeys.len() {
                        newkeys[index] = key;
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
                for key in &mut c.keys {
                    if *key >= BMKeys::BUTTON_17 && *key <= BMKeys::BUTTON_20 {
                        *key += BMKeys::AXIS1_PLUS - BMKeys::BUTTON_17;
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
        for c in &mut self.controller {
            validate_exclusive(&mut c.keys, &mut exclusive);
        }

        for (i, key) in self.midi.keys.iter_mut().enumerate() {
            if exclusive[i] {
                *key = None;
            }
        }
    }
}

fn validate_exclusive(keys: &mut [i32], exclusive: &mut [bool]) {
    for (key, excl) in keys.iter_mut().zip(exclusive.iter_mut()) {
        if *excl {
            *key = -1;
        } else if *key != -1 {
            *excl = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_beat_7k() {
        let config = PlayModeConfig::new(Mode::BEAT_7K);
        assert_eq!(config.keyboard.keys.len(), 9);
        assert_ne!(config.keyboard.keys[0], -1);
        assert_eq!(config.controller.len(), 1);
        assert!(config.midi.keys.iter().all(|k| k.is_none()));
        assert_eq!(config.version, 0);
    }

    #[test]
    fn test_new_keyboard_24k() {
        let config = PlayModeConfig::new(Mode::KEYBOARD_24K);
        assert!(config.keyboard.keys.iter().all(|k| *k == -1));
        assert_eq!(config.midi.keys.len(), 26);
        assert!(config.midi.keys[0].is_some());
        assert_eq!(config.controller.len(), 1);
    }

    #[test]
    fn test_new_beat_14k_two_controllers() {
        let config = PlayModeConfig::new(Mode::BEAT_14K);
        assert_eq!(config.controller.len(), 2);
        assert_eq!(config.keyboard.keys.len(), 18);
    }

    #[test]
    fn test_validate_resizes_keyboard_keys() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.keyboard.keys.clear();
        config.validate(9);
        assert_eq!(config.keyboard.keys.len(), 9);
        assert_eq!(config.keyboard.keys[0], gdx_keys::Z);
    }

    #[test]
    fn test_validate_resizes_controller_keys() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.keyboard.keys = vec![-1; 9];
        config.controller[0].keys = vec![BMKeys::BUTTON_4, BMKeys::BUTTON_7];
        config.validate(9);
        assert_eq!(config.controller[0].keys.len(), 9);
        assert_eq!(config.controller[0].keys[0], BMKeys::BUTTON_4);
        assert_eq!(config.controller[0].keys[1], BMKeys::BUTTON_7);
    }

    #[test]
    fn test_validate_resizes_midi_keys() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.midi.keys.clear();
        config.validate(9);
        assert_eq!(config.midi.keys.len(), 9);
    }

    #[test]
    fn test_validate_exclusive_keyboard_claims_slot() {
        let mut config = PlayModeConfig::new_with_configs(
            KeyboardConfig::new(Mode::BEAT_7K, true),
            vec![ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, true)],
            MidiConfig::new(Mode::BEAT_7K, false),
        );
        assert_ne!(config.keyboard.keys[0], -1);
        assert_ne!(config.controller[0].keys[0], -1);
        config.validate(9);
        assert_ne!(config.keyboard.keys[0], -1);
        assert_eq!(config.controller[0].keys[0], -1);
    }

    #[test]
    fn test_validate_exclusive_midi_cleared_when_kb_claims() {
        let mut config = PlayModeConfig::new_with_configs(
            KeyboardConfig::new(Mode::BEAT_7K, true),
            vec![ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, false)],
            MidiConfig::new(Mode::BEAT_7K, true),
        );
        assert_ne!(config.keyboard.keys[0], -1);
        assert!(config.midi.keys[0].is_some());
        config.validate(9);
        assert!(config.midi.keys[0].is_none());
    }

    #[test]
    fn test_validate_exclusive_unbound_slots_passthrough() {
        let mut config = PlayModeConfig::new_with_configs(
            KeyboardConfig::new(Mode::BEAT_7K, false),
            vec![ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, false)],
            MidiConfig::new(Mode::BEAT_7K, true),
        );
        config.validate(9);
        assert!(config.midi.keys[0].is_some());
    }

    #[test]
    fn test_validate_exclusive_fn_directly() {
        let mut keys = vec![5, -1, 3];
        let mut exclusive = vec![false, false, false];
        validate_exclusive(&mut keys, &mut exclusive);
        assert_eq!(keys, vec![5, -1, 3]);
        assert_eq!(exclusive, vec![true, false, true]);

        let mut keys2 = vec![5, 7, -1];
        validate_exclusive(&mut keys2, &mut exclusive);
        assert_eq!(keys2, vec![-1, 7, -1]);
        assert_eq!(exclusive, vec![true, true, true]);
    }

    #[test]
    fn test_validate_v0_migrates_button17_to_axis_range() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.version = 0;
        config.keyboard.keys = vec![-1; 9];
        config.controller[0].keys = vec![BMKeys::BUTTON_17; 9];
        config.validate(9);
        assert_eq!(config.controller[0].keys[0], BMKeys::AXIS1_PLUS);
        assert_eq!(config.version, 1);
    }

    #[test]
    fn test_validate_v1_no_migration() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.version = 1;
        config.keyboard.keys = vec![-1; 9];
        config.controller[0].keys = vec![BMKeys::BUTTON_17; 9];
        config.validate(9);
        assert_eq!(config.controller[0].keys[0], BMKeys::BUTTON_17);
    }

    #[test]
    fn test_validate_clamps_keyboard_duration() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.keyboard.duration = 200;
        config.validate(9);
        assert_eq!(config.keyboard.duration, 100);
        config.keyboard.duration = -5;
        config.validate(9);
        assert_eq!(config.keyboard.duration, 0);
    }

    #[test]
    fn test_validate_clamps_mouse_scratch_fields() {
        let mut config = PlayModeConfig::new(Mode::BEAT_7K);
        config.keyboard.mouse_scratch_config.mouse_scratch_distance = 0;
        config
            .keyboard
            .mouse_scratch_config
            .mouse_scratch_time_threshold = 0;
        config.validate(9);
        assert_eq!(
            config.keyboard.mouse_scratch_config.mouse_scratch_distance,
            1
        );
        assert_eq!(
            config
                .keyboard
                .mouse_scratch_config
                .mouse_scratch_time_threshold,
            1
        );
    }
}
