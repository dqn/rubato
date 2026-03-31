use bms::model::mode::Mode;

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
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize, Default)]
pub enum MidiInputType {
    #[default]
    NOTE,
    PITCH_BEND,
    CONTROL_CHANGE,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
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
                self.keys = (0..5)
                    .map(|i| Some(MidiInput::new(MidiInputType::NOTE, 53 + i)))
                    .collect();
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 49)));
                self.keys
                    .push(Some(MidiInput::new(MidiInputType::NOTE, 51)));
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::BEAT_7K => {
                self.keys = (0..7)
                    .map(|i| Some(MidiInput::new(MidiInputType::NOTE, 53 + i)))
                    .collect();
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
                self.keys = (0..9)
                    .map(|i| Some(MidiInput::new(MidiInputType::NOTE, 52 + i)))
                    .collect();
                self.start = Some(MidiInput::new(MidiInputType::NOTE, 47));
                self.select = Some(MidiInput::new(MidiInputType::NOTE, 48));
            }
            Mode::KEYBOARD_24K => {
                self.keys = vec![None; 26];
                for (i, key) in self.keys.iter_mut().enumerate().take(24) {
                    *key = Some(MidiInput::new(MidiInputType::NOTE, 48 + i as i32));
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
                self.keys = (0..7)
                    .map(|i| Some(MidiInput::new(MidiInputType::NOTE, 53 + i)))
                    .collect();
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

impl MidiConfig {
    pub fn start(&self) -> Option<&MidiInput> {
        self.start.as_ref()
    }

    pub fn select(&self) -> Option<&MidiInput> {
        self.select.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    fn assert_note(input: &Option<MidiInput>, expected_value: i32) {
        let input = input.as_ref().unwrap();
        assert_eq!(input.input_type, MidiInputType::NOTE);
        assert_eq!(input.value, expected_value);
    }

    #[test]
    fn test_beat_5k_7_keys() {
        let config = MidiConfig::new(Mode::BEAT_5K, true);
        assert_eq!(config.keys.len(), 7);
        for i in 0..5 {
            assert_note(&config.keys[i], 53 + i as i32);
        }
        assert_note(&config.keys[5], 49);
        assert_note(&config.keys[6], 51);
        assert_note(&config.start, 47);
        assert_note(&config.select, 48);
    }

    #[test]
    fn test_beat_7k_9_keys() {
        let config = MidiConfig::new(Mode::BEAT_7K, true);
        assert_eq!(config.keys.len(), 9);
        for i in 0..7 {
            assert_note(&config.keys[i], 53 + i as i32);
        }
        assert_note(&config.keys[7], 49);
        assert_note(&config.keys[8], 51);
    }

    #[test]
    fn test_beat_14k_18_keys() {
        let config = MidiConfig::new(Mode::BEAT_14K, true);
        assert_eq!(config.keys.len(), 18);
        for i in 0..7 {
            assert_note(&config.keys[i], 53 + i as i32);
        }
        assert_note(&config.keys[7], 49);
        assert_note(&config.keys[8], 51);
        for i in 0..7 {
            assert_note(&config.keys[9 + i], 65 + i as i32);
        }
        assert_note(&config.keys[16], 73);
        assert_note(&config.keys[17], 75);
    }

    #[test]
    fn test_keyboard_24k_pitch_bend() {
        let config = MidiConfig::new(Mode::KEYBOARD_24K, true);
        assert_eq!(config.keys.len(), 26);
        for i in 0..24 {
            assert_note(&config.keys[i], 48 + i as i32);
        }
        let up = config.keys[24].as_ref().unwrap();
        assert_eq!(up.input_type, MidiInputType::PITCH_BEND);
        assert_eq!(up.value, 1);
        let down = config.keys[25].as_ref().unwrap();
        assert_eq!(down.input_type, MidiInputType::PITCH_BEND);
        assert_eq!(down.value, -1);
    }

    #[test]
    fn test_popn_9k_9_keys() {
        let config = MidiConfig::new(Mode::POPN_9K, true);
        for i in 0..9 {
            assert_note(&config.keys[i], 52 + i as i32);
        }
    }

    #[test]
    fn test_disabled_all_keys_none() {
        let config = MidiConfig::new(Mode::BEAT_7K, false);
        assert!(config.keys.iter().all(|k| k.is_none()));
        assert!(config.start.is_some());
        assert!(config.select.is_some());
    }

    #[test]
    fn test_midi_input_display_note() {
        assert_eq!(
            MidiInput::new(MidiInputType::NOTE, 53).to_string(),
            "NOTE 53"
        );
    }

    #[test]
    fn test_midi_input_display_pitch_bend() {
        assert_eq!(
            MidiInput::new(MidiInputType::PITCH_BEND, 1).to_string(),
            "PITCH +"
        );
        assert_eq!(
            MidiInput::new(MidiInputType::PITCH_BEND, -1).to_string(),
            "PITCH -"
        );
        assert_eq!(
            MidiInput::new(MidiInputType::PITCH_BEND, 0).to_string(),
            "PITCH -"
        );
    }

    #[test]
    fn test_midi_input_display_cc() {
        assert_eq!(
            MidiInput::new(MidiInputType::CONTROL_CHANGE, 7).to_string(),
            "CC 7"
        );
    }

    #[test]
    fn test_midi_input_default() {
        let input = MidiInput::default();
        assert_eq!(input.input_type, MidiInputType::NOTE);
        assert_eq!(input.value, 0);
    }

    #[test]
    fn test_serde_roundtrip() {
        let input = MidiInput::new(MidiInputType::PITCH_BEND, 1);
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("\"type\""));
        let d: MidiInput = serde_json::from_str(&json).unwrap();
        assert_eq!(d.input_type, MidiInputType::PITCH_BEND);
        assert_eq!(d.value, 1);
    }
}
