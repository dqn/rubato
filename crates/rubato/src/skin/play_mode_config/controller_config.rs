use bms::model::mode::Mode;

use crate::skin::bm_keys::BMKeys;

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

    pub fn name(&self) -> Option<&str> {
        if self.name.is_empty() {
            None
        } else {
            Some(&self.name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    #[test]
    fn test_beat_7k_player0_9_keys() {
        let config = ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, true);
        assert_eq!(config.keys, iidx_ps2_keys());
    }

    #[test]
    fn test_beat_14k_player0_18_keys_with_p2_padding() {
        let config = ControllerConfig::new_with_mode(Mode::BEAT_14K, 0, true);
        let preset = iidx_ps2_keys();
        assert_eq!(config.keys.len(), 18);
        assert_eq!(&config.keys[..9], &preset[..]);
        assert!(config.keys[9..].iter().all(|&k| k == -1));
    }

    #[test]
    fn test_beat_14k_player1_p2_keys() {
        let config = ControllerConfig::new_with_mode(Mode::BEAT_14K, 1, true);
        let preset = iidx_ps2_keys();
        assert_eq!(config.keys.len(), 18);
        assert!(config.keys[..9].iter().all(|&k| k == -1));
        assert_eq!(&config.keys[9..], &preset[..]);
    }

    #[test]
    fn test_beat_7k_player1_all_disabled() {
        let config = ControllerConfig::new_with_mode(Mode::BEAT_7K, 1, true);
        assert!(config.keys.iter().all(|&k| k == -1));
    }

    #[test]
    fn test_disabled_all_keys_negative_one() {
        let config = ControllerConfig::new_with_mode(Mode::BEAT_7K, 0, false);
        assert!(config.keys.iter().all(|&k| k == -1));
        assert_eq!(config.start, iidx_ps2_start());
        assert_eq!(config.select, iidx_ps2_select());
    }

    #[test]
    fn test_analog_scratch_threshold_clamp() {
        let mut config = ControllerConfig::default();
        config.set_analog_scratch_threshold(0);
        assert_eq!(config.analog_scratch_threshold, 1);
        config.set_analog_scratch_threshold(-5);
        assert_eq!(config.analog_scratch_threshold, 1);
        config.set_analog_scratch_threshold(1001);
        assert_eq!(config.analog_scratch_threshold, 1000);
        config.set_analog_scratch_threshold(500);
        assert_eq!(config.analog_scratch_threshold, 500);
    }

    #[test]
    fn test_default_analog_scratch_threshold_50() {
        assert_eq!(ControllerConfig::default().analog_scratch_threshold, 50);
    }

    #[test]
    fn test_name_empty_returns_none() {
        assert!(ControllerConfig::default().name().is_none());
    }

    #[test]
    fn test_name_set_returns_some() {
        let mut config = ControllerConfig::default();
        config.name = "IIDX Controller".to_string();
        assert_eq!(config.name(), Some("IIDX Controller"));
    }

    #[test]
    fn test_serde_roundtrip() {
        let mut config = ControllerConfig::default();
        config.jkoc_hack = true;
        config.analog_scratch = true;
        config.analog_scratch_mode = ANALOG_SCRATCH_VER_1;
        config.analog_scratch_threshold = 75;
        let json = serde_json::to_string(&config).unwrap();
        assert!(json.contains("jkocHack"));
        assert!(json.contains("analogScratchThreshold"));
        let d: ControllerConfig = serde_json::from_str(&json).unwrap();
        assert!(d.jkoc_hack);
        assert_eq!(d.analog_scratch_threshold, 75);
    }
}
