use bms::model::mode::Mode;

use super::MouseScratchConfig;
use super::gdx_keys;

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
            mouse_scratch_config: MouseScratchConfig::new(mode),
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

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    #[test]
    fn test_beat_5k_7_keys() {
        let config = KeyboardConfig::new(Mode::BEAT_5K, true);
        assert_eq!(config.keys.len(), 7);
        assert_eq!(
            config.keys,
            vec![
                gdx_keys::Z,
                gdx_keys::S,
                gdx_keys::X,
                gdx_keys::D,
                gdx_keys::C,
                gdx_keys::SHIFT_LEFT,
                gdx_keys::CONTROL_LEFT,
            ]
        );
    }

    #[test]
    fn test_beat_7k_9_keys() {
        let config = KeyboardConfig::new(Mode::BEAT_7K, true);
        assert_eq!(config.keys.len(), 9);
        assert_eq!(config.keys[5], gdx_keys::F);
        assert_eq!(config.keys[6], gdx_keys::V);
    }

    #[test]
    fn test_beat_14k_18_keys() {
        let config = KeyboardConfig::new(Mode::BEAT_14K, true);
        assert_eq!(config.keys.len(), 18);
        assert_eq!(config.keys[9], gdx_keys::COMMA);
    }

    #[test]
    fn test_keyboard_24k_double_52_keys() {
        let config = KeyboardConfig::new(Mode::KEYBOARD_24K_DOUBLE, true);
        assert_eq!(config.keys.len(), 52);
        assert_eq!(config.keys[0], gdx_keys::Z);
        assert_eq!(config.keys[51], 0);
    }

    #[test]
    fn test_popn_modes_share_9_keys() {
        let popn5 = KeyboardConfig::new(Mode::POPN_5K, true);
        let popn9 = KeyboardConfig::new(Mode::POPN_9K, true);
        assert_eq!(popn5.keys, popn9.keys);
        assert_eq!(popn5.keys.len(), 9);
    }

    #[test]
    fn test_disabled_all_keys_negative_one() {
        let config = KeyboardConfig::new(Mode::BEAT_7K, false);
        assert!(config.keys.iter().all(|&k| k == -1));
        assert_eq!(config.start, gdx_keys::Q);
        assert_eq!(config.select, gdx_keys::W);
    }

    #[test]
    fn test_key_count_per_mode_table() {
        let cases = [
            (Mode::BEAT_5K, 7),
            (Mode::BEAT_7K, 9),
            (Mode::BEAT_10K, 14),
            (Mode::BEAT_14K, 18),
            (Mode::POPN_5K, 9),
            (Mode::POPN_9K, 9),
            (Mode::KEYBOARD_24K, 26),
            (Mode::KEYBOARD_24K_DOUBLE, 52),
        ];
        for (mode, expected_len) in cases {
            let config = KeyboardConfig::new(mode, true);
            assert_eq!(config.keys.len(), expected_len, "mode: {:?}", mode);
        }
    }

    #[test]
    fn test_serde_roundtrip() {
        let config = KeyboardConfig::new(Mode::BEAT_7K, true);
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("mouseScratchConfig"));
        let deserialized: KeyboardConfig = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.keys, config.keys);
        assert_eq!(deserialized.start, config.start);
    }
}
