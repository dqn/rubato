use bms::model::mode::Mode;

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

    pub fn key_string(&self, index: usize) -> Option<&'static str> {
        let key = *self.keys.get(index)?;
        if key < 0 || (key as usize) >= MOUSESCRATCH_STRING.len() {
            return None;
        }
        Some(MOUSESCRATCH_STRING[key as usize])
    }

    pub fn start_string(&self) -> Option<&'static str> {
        if self.start < 0 || (self.start as usize) >= MOUSESCRATCH_STRING.len() {
            return None;
        }
        Some(MOUSESCRATCH_STRING[self.start as usize])
    }

    pub fn select_string(&self) -> Option<&'static str> {
        if self.select < 0 || (self.select as usize) >= MOUSESCRATCH_STRING.len() {
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

#[cfg(test)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    #[test]
    fn test_key_lengths_per_mode() {
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
            let config = MouseScratchConfig::new(mode);
            assert_eq!(config.keys.len(), expected_len, "mode: {:?}", mode);
        }
    }

    #[test]
    fn test_all_keys_initialized_to_negative_one() {
        let config = MouseScratchConfig::new(Mode::BEAT_7K);
        assert!(config.keys.iter().all(|&k| k == -1));
        assert_eq!(config.start, -1);
        assert_eq!(config.select, -1);
    }

    #[test]
    fn test_key_string_valid() {
        let mut config = MouseScratchConfig::new(Mode::BEAT_7K);
        config.keys[0] = 0;
        assert_eq!(config.key_string(0), Some("MOUSE RIGHT"));
        config.keys[0] = 1;
        assert_eq!(config.key_string(0), Some("MOUSE LEFT"));
        config.keys[0] = 2;
        assert_eq!(config.key_string(0), Some("MOUSE DOWN"));
        config.keys[0] = 3;
        assert_eq!(config.key_string(0), Some("MOUSE UP"));
    }

    #[test]
    fn test_key_string_out_of_range() {
        let mut config = MouseScratchConfig::new(Mode::BEAT_7K);
        config.keys[0] = 4;
        assert_eq!(config.key_string(0), None);
        config.keys[0] = -1;
        assert_eq!(config.key_string(0), None);
    }

    #[test]
    fn test_start_select_string() {
        let mut config = MouseScratchConfig::new(Mode::BEAT_7K);
        config.start = 0;
        assert_eq!(config.start_string(), Some("MOUSE RIGHT"));
        assert_eq!(config.select_string(), None);
    }

    #[test]
    fn test_time_threshold_clamp() {
        let mut config = MouseScratchConfig::new(Mode::BEAT_7K);
        config.set_mouse_scratch_time_threshold(0);
        assert_eq!(config.mouse_scratch_time_threshold, 1);
        config.set_mouse_scratch_time_threshold(-5);
        assert_eq!(config.mouse_scratch_time_threshold, 1);
        config.set_mouse_scratch_time_threshold(500);
        assert_eq!(config.mouse_scratch_time_threshold, 500);
    }

    #[test]
    fn test_distance_clamp() {
        let mut config = MouseScratchConfig::new(Mode::BEAT_7K);
        config.set_mouse_scratch_distance(0);
        assert_eq!(config.mouse_scratch_distance, 1);
        config.set_mouse_scratch_distance(-5);
        assert_eq!(config.mouse_scratch_distance, 1);
        config.set_mouse_scratch_distance(500);
        assert_eq!(config.mouse_scratch_distance, 500);
    }

    #[test]
    fn test_defaults() {
        let config = MouseScratchConfig::new(Mode::BEAT_7K);
        assert!(!config.mouse_scratch_enabled);
        assert_eq!(config.mouse_scratch_time_threshold, 150);
        assert_eq!(config.mouse_scratch_distance, 12);
        assert_eq!(config.mouse_scratch_mode, 0);
    }
}
