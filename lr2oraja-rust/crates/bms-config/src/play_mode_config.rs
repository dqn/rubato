use serde::{Deserialize, Serialize};

use crate::play_config::PlayConfig;

// LibGDX Keys constants
#[allow(dead_code)] // Parsed for completeness (LibGDX key code mapping)
mod libgdx_keys {
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
    pub const SHIFT_LEFT: i32 = 59;
    pub const CONTROL_LEFT: i32 = 129;
    pub const SHIFT_RIGHT: i32 = 60;
    pub const CONTROL_RIGHT: i32 = 130;
    pub const UNKNOWN: i32 = 0;
}

// BMControllerInputProcessor.BMKeys constants
#[allow(dead_code)] // Parsed for completeness (BM controller key constants)
mod bm_keys {
    pub const BUTTON_1: i32 = 0;
    pub const BUTTON_2: i32 = 1;
    pub const BUTTON_3: i32 = 2;
    pub const BUTTON_4: i32 = 3;
    pub const BUTTON_5: i32 = 4;
    pub const BUTTON_6: i32 = 5;
    pub const BUTTON_7: i32 = 6;
    pub const BUTTON_8: i32 = 7;
    pub const BUTTON_9: i32 = 8;
    pub const BUTTON_10: i32 = 9;
    pub const BUTTON_17: i32 = 16;
    pub const BUTTON_20: i32 = 19;
    pub const AXIS1_PLUS: i32 = 32;
    pub const AXIS1_MINUS: i32 = 33;
    pub const AXIS2_PLUS: i32 = 34;
    pub const AXIS3_PLUS: i32 = 36;
    pub const AXIS3_MINUS: i32 = 37;
    pub const AXIS4_MINUS: i32 = 39;
}

// IIDX PS2 controller preset keys
const IIDX_PS2_KEYS: [i32; 9] = [
    bm_keys::BUTTON_4,
    bm_keys::BUTTON_7,
    bm_keys::BUTTON_3,
    bm_keys::BUTTON_8,
    bm_keys::BUTTON_2,
    bm_keys::BUTTON_5,
    bm_keys::AXIS4_MINUS,
    bm_keys::AXIS3_MINUS,
    bm_keys::AXIS3_PLUS,
];
const IIDX_PS2_START: i32 = bm_keys::BUTTON_9;
const IIDX_PS2_SELECT: i32 = bm_keys::BUTTON_10;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
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
        Self {
            playconfig: PlayConfig::default(),
            keyboard: KeyboardConfig::default(),
            controller: vec![ControllerConfig::default()],
            midi: MidiConfig::default(),
            version: 0,
        }
    }
}

impl PlayModeConfig {
    pub fn validate(&mut self, keys: usize) {
        self.playconfig.validate();

        // Normalize keyboard keys
        if self.keyboard.keys.is_empty() {
            self.keyboard.keys = default_keyboard_keys_7k();
        }
        self.keyboard.keys.resize(keys, 0);
        self.keyboard.duration = self.keyboard.duration.clamp(0, 100);

        // Normalize mouse scratch keys
        let ms = &mut self.keyboard.mouse_scratch_config;
        if ms.keys.len() != keys {
            ms.keys = vec![-1; keys];
        }
        ms.mouse_scratch_distance = ms.mouse_scratch_distance.clamp(1, 10000);
        ms.mouse_scratch_time_threshold = ms.mouse_scratch_time_threshold.clamp(1, 10000);

        // Normalize controller keys
        let mut index = 0usize;
        for c in &mut self.controller {
            if c.keys.is_empty() {
                c.keys = default_controller_keys_7k();
            }
            if c.keys.len() != keys {
                let mut new_keys = vec![-1i32; keys];
                for i in 0..c.keys.len() {
                    if index < new_keys.len() {
                        new_keys[index] = c.keys[i];
                        index += 1;
                    }
                }
                c.keys = new_keys;
            }
            c.duration = c.duration.clamp(0, 100);
        }

        // Version 0->1 migration: remap button 17-20 to axis values
        if self.version == 0 {
            for c in &mut self.controller {
                for key in &mut c.keys {
                    if *key >= bm_keys::BUTTON_17 && *key <= bm_keys::BUTTON_20 {
                        *key += bm_keys::AXIS1_PLUS - bm_keys::BUTTON_17;
                    }
                }
            }
            self.version = 1;
        }

        // Normalize MIDI keys
        if self.midi.keys.len() != keys {
            self.midi.keys.resize(keys, None);
        }

        // Exclusive key processing: keyboard takes priority, then controllers, midi last
        let mut exclusive = vec![false; keys];
        validate_exclusive(&mut self.keyboard.keys, &mut exclusive);
        for c in &mut self.controller {
            validate_exclusive(&mut c.keys, &mut exclusive);
        }
        for (i, &is_exclusive) in exclusive.iter().enumerate().take(self.midi.keys.len()) {
            if is_exclusive {
                self.midi.keys[i] = None;
            }
        }
    }
}

/// Set conflicting keys to -1 (keyboard/controller) or None (midi).
/// Keyboard takes priority, then controllers in order, then midi.
fn validate_exclusive(keys: &mut [i32], exclusive: &mut [bool]) {
    for i in 0..exclusive.len() {
        if exclusive[i] {
            keys[i] = -1;
        } else if keys[i] != -1 {
            exclusive[i] = true;
        }
    }
}

fn default_keyboard_keys_7k() -> Vec<i32> {
    vec![
        libgdx_keys::Z,
        libgdx_keys::S,
        libgdx_keys::X,
        libgdx_keys::D,
        libgdx_keys::C,
        libgdx_keys::F,
        libgdx_keys::V,
        libgdx_keys::SHIFT_LEFT,
        libgdx_keys::CONTROL_LEFT,
    ]
}

fn default_controller_keys_7k() -> Vec<i32> {
    vec![
        bm_keys::BUTTON_4,
        bm_keys::BUTTON_7,
        bm_keys::BUTTON_3,
        bm_keys::BUTTON_8,
        bm_keys::BUTTON_2,
        bm_keys::BUTTON_5,
        bm_keys::AXIS4_MINUS,
        bm_keys::AXIS3_MINUS,
        bm_keys::AXIS3_PLUS,
    ]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct KeyboardConfig {
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub duration: i32,
    pub mouse_scratch_config: MouseScratchConfig,
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            keys: default_keyboard_keys_7k(),
            start: libgdx_keys::Q,
            select: libgdx_keys::W,
            duration: 16,
            mouse_scratch_config: MouseScratchConfig::default(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct MouseScratchConfig {
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub mouse_scratch_enabled: bool,
    pub mouse_scratch_time_threshold: i32,
    pub mouse_scratch_distance: i32,
    pub mouse_scratch_mode: i32,
}

impl Default for MouseScratchConfig {
    fn default() -> Self {
        Self {
            keys: vec![-1; 9],
            start: -1,
            select: -1,
            mouse_scratch_enabled: false,
            mouse_scratch_time_threshold: 150,
            mouse_scratch_distance: 12,
            mouse_scratch_mode: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct ControllerConfig {
    pub name: String,
    pub keys: Vec<i32>,
    pub start: i32,
    pub select: i32,
    pub duration: i32,
    pub jkoc_hack: bool,
    pub analog_scratch: bool,
    pub analog_scratch_mode: i32,
    pub analog_scratch_threshold: i32,
}

impl Default for ControllerConfig {
    fn default() -> Self {
        Self {
            name: String::new(),
            keys: IIDX_PS2_KEYS.to_vec(),
            start: IIDX_PS2_START,
            select: IIDX_PS2_SELECT,
            duration: 16,
            jkoc_hack: false,
            analog_scratch: false,
            analog_scratch_mode: 0,
            analog_scratch_threshold: 50,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[serde(default)]
pub struct MidiConfig {
    pub keys: Vec<Option<MidiInput>>,
    pub start: Option<MidiInput>,
    pub select: Option<MidiInput>,
}

impl Default for MidiConfig {
    fn default() -> Self {
        // Default: 7-key MIDI assignment
        let mut keys: Vec<Option<MidiInput>> = Vec::with_capacity(9);
        for i in 0..7 {
            keys.push(Some(MidiInput {
                type_: MidiInputType::Note,
                value: 53 + i,
            }));
        }
        // Turntable keys
        keys.push(Some(MidiInput {
            type_: MidiInputType::Note,
            value: 49,
        }));
        keys.push(Some(MidiInput {
            type_: MidiInputType::Note,
            value: 51,
        }));

        Self {
            keys,
            start: Some(MidiInput {
                type_: MidiInputType::Note,
                value: 47,
            }),
            select: Some(MidiInput {
                type_: MidiInputType::Note,
                value: 48,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MidiInput {
    #[serde(rename = "type")]
    pub type_: MidiInputType,
    pub value: i32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum MidiInputType {
    Note,
    PitchBend,
    ControlChange,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_play_mode_config_default() {
        let config = PlayModeConfig::default();
        assert_eq!(config.keyboard.keys.len(), 9);
        assert_eq!(config.controller.len(), 1);
        assert_eq!(config.midi.keys.len(), 9);
        assert_eq!(config.version, 0);
    }

    #[test]
    fn test_keyboard_config_default() {
        let kb = KeyboardConfig::default();
        assert_eq!(
            kb.keys,
            vec![
                libgdx_keys::Z,
                libgdx_keys::S,
                libgdx_keys::X,
                libgdx_keys::D,
                libgdx_keys::C,
                libgdx_keys::F,
                libgdx_keys::V,
                libgdx_keys::SHIFT_LEFT,
                libgdx_keys::CONTROL_LEFT,
            ]
        );
        assert_eq!(kb.start, libgdx_keys::Q);
        assert_eq!(kb.select, libgdx_keys::W);
        assert_eq!(kb.duration, 16);
    }

    #[test]
    fn test_controller_config_default() {
        let cc = ControllerConfig::default();
        assert_eq!(cc.keys, IIDX_PS2_KEYS.to_vec());
        assert_eq!(cc.start, bm_keys::BUTTON_9);
        assert_eq!(cc.select, bm_keys::BUTTON_10);
        assert_eq!(cc.duration, 16);
        assert!(!cc.jkoc_hack);
        assert!(!cc.analog_scratch);
        assert_eq!(cc.analog_scratch_mode, 0);
        assert_eq!(cc.analog_scratch_threshold, 50);
    }

    #[test]
    fn test_midi_config_default() {
        let midi = MidiConfig::default();
        assert_eq!(midi.keys.len(), 9);
        // Keys 0..6 = NOTE 53..59
        for i in 0..7 {
            let input = midi.keys[i].as_ref().unwrap();
            assert_eq!(input.type_, MidiInputType::Note);
            assert_eq!(input.value, 53 + i as i32);
        }
        // Turntables
        assert_eq!(midi.keys[7].as_ref().unwrap().value, 49);
        assert_eq!(midi.keys[8].as_ref().unwrap().value, 51);
        // Start/Select
        assert_eq!(midi.start.as_ref().unwrap().value, 47);
        assert_eq!(midi.select.as_ref().unwrap().value, 48);
    }

    #[test]
    fn test_mouse_scratch_config_default() {
        let ms = MouseScratchConfig::default();
        assert_eq!(ms.keys.len(), 9);
        assert!(ms.keys.iter().all(|&k| k == -1));
        assert_eq!(ms.start, -1);
        assert_eq!(ms.select, -1);
        assert!(!ms.mouse_scratch_enabled);
        assert_eq!(ms.mouse_scratch_time_threshold, 150);
        assert_eq!(ms.mouse_scratch_distance, 12);
        assert_eq!(ms.mouse_scratch_mode, 0);
    }

    #[test]
    fn test_validate_normalizes_key_lengths() {
        let mut config = PlayModeConfig::default();
        config.keyboard.keys = vec![1, 2, 3]; // Too short
        config.validate(9);
        assert_eq!(config.keyboard.keys.len(), 9);
        assert_eq!(config.keyboard.mouse_scratch_config.keys.len(), 9);
    }

    #[test]
    fn test_validate_clamps_duration() {
        let mut config = PlayModeConfig::default();
        config.keyboard.duration = 200;
        config.controller[0].duration = -5;
        config.validate(9);
        assert_eq!(config.keyboard.duration, 100);
        assert_eq!(config.controller[0].duration, 0);
    }

    #[test]
    fn test_validate_clamps_mouse_scratch() {
        let mut config = PlayModeConfig::default();
        config.keyboard.mouse_scratch_config.mouse_scratch_distance = 0;
        config
            .keyboard
            .mouse_scratch_config
            .mouse_scratch_time_threshold = -10;
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

    #[test]
    fn test_validate_version_migration() {
        let mut config = PlayModeConfig::default();
        config.version = 0;
        // Clear keyboard keys so exclusive processing doesn't interfere
        config.keyboard.keys = vec![-1; 9];
        // Set a key in button 17-20 range
        config.controller[0].keys =
            vec![bm_keys::BUTTON_17, bm_keys::BUTTON_20, 0, 0, 0, 0, 0, 0, 0];
        config.validate(9);

        // BUTTON_17 (16) -> AXIS1_PLUS (32), BUTTON_20 (19) -> 35
        assert_eq!(
            config.controller[0].keys[0],
            bm_keys::BUTTON_17 + (bm_keys::AXIS1_PLUS - bm_keys::BUTTON_17)
        );
        assert_eq!(
            config.controller[0].keys[1],
            bm_keys::BUTTON_20 + (bm_keys::AXIS1_PLUS - bm_keys::BUTTON_17)
        );
        assert_eq!(config.version, 1);
    }

    #[test]
    fn test_validate_exclusive_keys() {
        let mut config = PlayModeConfig::default();
        // Set same keys for keyboard and controller
        config.keyboard.keys = vec![10, 20, 30, 40, 50, 60, 70, 80, 90];
        config.controller[0].keys = vec![10, 20, 30, 40, 50, 60, 70, 80, 90];
        config.version = 1; // Skip migration
        config.validate(9);

        // Keyboard keeps all keys
        assert_eq!(
            config.keyboard.keys,
            vec![10, 20, 30, 40, 50, 60, 70, 80, 90]
        );
        // Controller keys should be set to -1 where keyboard already claimed
        assert!(config.controller[0].keys.iter().all(|&k| k == -1));
        // MIDI keys should also be None where exclusive
        assert!(config.midi.keys.iter().all(|k| k.is_none()));
    }

    #[test]
    fn test_serde_round_trip() {
        let config = PlayModeConfig::default();
        let json = serde_json::to_string(&config).unwrap();
        let deserialized: PlayModeConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(config.keyboard.keys, deserialized.keyboard.keys);
        assert_eq!(config.controller.len(), deserialized.controller.len());
        assert_eq!(config.midi.keys.len(), deserialized.midi.keys.len());
        assert_eq!(config.version, deserialized.version);
    }

    #[test]
    fn test_serde_camel_case() {
        let config = PlayModeConfig::default();
        let json = serde_json::to_string(&config).unwrap();

        assert!(json.contains("\"playconfig\""));
        assert!(json.contains("\"mouseScratchConfig\""));
        assert!(json.contains("\"analogScratch\""));
        assert!(json.contains("\"jkocHack\""));
    }

    #[test]
    fn test_serde_default_fills_missing() {
        let json = r#"{"version": 1}"#;
        let config: PlayModeConfig = serde_json::from_str(json).unwrap();

        assert_eq!(config.version, 1);
        assert_eq!(config.keyboard.keys.len(), 9);
        assert_eq!(config.controller.len(), 1);
    }

    #[test]
    fn test_midi_input_type_serde() {
        let input = MidiInput {
            type_: MidiInputType::Note,
            value: 53,
        };
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"NOTE\""));

        let input2 = MidiInput {
            type_: MidiInputType::PitchBend,
            value: 1,
        };
        let json2 = serde_json::to_string(&input2).unwrap();
        assert!(json2.contains("\"PITCH_BEND\""));

        let input3 = MidiInput {
            type_: MidiInputType::ControlChange,
            value: 64,
        };
        let json3 = serde_json::to_string(&input3).unwrap();
        assert!(json3.contains("\"CONTROL_CHANGE\""));
    }

    #[test]
    fn test_validate_empty_keyboard_fills_defaults() {
        let mut config = PlayModeConfig::default();
        config.keyboard.keys = vec![];
        config.validate(9);
        assert_eq!(config.keyboard.keys.len(), 9);
        // Should have filled with default 7k keys
        assert_eq!(config.keyboard.keys[0], libgdx_keys::Z);
    }

    #[test]
    fn test_validate_empty_controller_fills_defaults() {
        let mut config = PlayModeConfig::default();
        // Clear keyboard keys so exclusive processing doesn't interfere
        config.keyboard.keys = vec![-1; 9];
        config.controller[0].keys = vec![];
        config.version = 1;
        config.validate(9);
        assert_eq!(config.controller[0].keys.len(), 9);
        assert_eq!(config.controller[0].keys[0], bm_keys::BUTTON_4);
    }
}
