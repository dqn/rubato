//! KeyInputLog - key input log
//!
//! Translated from: bms.player.beatoraja.input.KeyInputLog

use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
use bms_model::note::Note;

/// Key input log
#[derive(Clone, Debug)]
pub struct KeyInputLog {
    /// Key press time (us)
    presstime: i64,
    /// Key code
    keycode: i32,
    /// Key pressed/released
    pressed: bool,
    /// Key press time (ms) - for backward compatibility with old data
    time: i64,
}

impl Default for KeyInputLog {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyInputLog {
    pub fn new() -> Self {
        KeyInputLog {
            presstime: 0,
            keycode: 0,
            pressed: false,
            time: 0,
        }
    }

    pub fn with_data(presstime: i64, keycode: i32, pressed: bool) -> Self {
        let mut log = Self::new();
        log.set_data(presstime, keycode, pressed);
        log
    }

    pub fn set_data(&mut self, presstime: i64, keycode: i32, pressed: bool) {
        self.presstime = presstime;
        self.keycode = keycode;
        self.pressed = pressed;
    }

    pub fn get_time(&self) -> i64 {
        if self.presstime != 0 {
            self.presstime
        } else {
            self.time * 1000
        }
    }

    pub fn get_keycode(&self) -> i32 {
        self.keycode
    }

    pub fn is_pressed(&self) -> bool {
        self.pressed
    }

    /// Create autoplay KeyInputLog
    ///
    /// Translated from: KeyInputLog.createAutoplayLog(BMSModel)
    pub fn create_autoplay_log(model: &BMSModel) -> Vec<KeyInputLog> {
        // Java: "TODO 地雷を確実に回避するアルゴリズム" — unimplemented in Java: mine avoidance algorithm
        let mut keylog: Vec<KeyInputLog> = Vec::new();
        let mode: &Mode = match model.get_mode() {
            Some(m) => m,
            None => return keylog,
        };
        let keys: i32 = mode.key();
        let sc: &[i32] = mode.scratch_key();
        let mut ln: Vec<Option<Note>> = vec![None; keys as usize];
        for tl in model.get_all_time_lines() {
            let i: i64 = tl.get_micro_time();
            for lane in 0..keys {
                let note = tl.get_note(lane);
                if let Some(note) = note {
                    let note: &Note = note;
                    if note.is_long() {
                        if note.is_end() {
                            keylog.push(KeyInputLog::with_data(i, lane, false));
                            if model.get_lntype() != 0 && sc.contains(&lane) {
                                // BSS handling
                                keylog.push(KeyInputLog::with_data(i, lane + 1, true));
                            }
                            ln[lane as usize] = None;
                        } else {
                            keylog.push(KeyInputLog::with_data(i, lane, true));
                            ln[lane as usize] = Some(note.clone());
                        }
                    } else if note.is_normal() {
                        keylog.push(KeyInputLog::with_data(i, lane, true));
                    }
                } else if ln[lane as usize].is_none() {
                    keylog.push(KeyInputLog::with_data(i, lane, false));
                    if sc.contains(&lane) {
                        keylog.push(KeyInputLog::with_data(i, lane + 1, false));
                    }
                }
            }
        }
        keylog
    }

    /// Validate and migrate old data format
    pub fn validate(&mut self) -> bool {
        if self.time > 0 {
            self.presstime = self.time * 1000;
            self.time = 0;
        }
        self.presstime >= 0 && self.keycode >= 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::mode::Mode;
    use bms_model::note::Note;
    use bms_model::time_line::TimeLine;

    // --- KeyInputLog::new ---

    #[test]
    fn test_new_defaults() {
        let log = KeyInputLog::new();
        assert_eq!(log.get_time(), 0);
        assert_eq!(log.get_keycode(), 0);
        assert!(!log.is_pressed());
    }

    #[test]
    fn test_default_equals_new() {
        let a = KeyInputLog::new();
        let b = KeyInputLog::default();
        assert_eq!(a.get_time(), b.get_time());
        assert_eq!(a.get_keycode(), b.get_keycode());
        assert_eq!(a.is_pressed(), b.is_pressed());
    }

    // --- KeyInputLog::with_data ---

    #[test]
    fn test_with_data() {
        let log = KeyInputLog::with_data(5000, 3, true);
        assert_eq!(log.get_time(), 5000);
        assert_eq!(log.get_keycode(), 3);
        assert!(log.is_pressed());
    }

    #[test]
    fn test_with_data_released() {
        let log = KeyInputLog::with_data(1000, 7, false);
        assert_eq!(log.get_time(), 1000);
        assert_eq!(log.get_keycode(), 7);
        assert!(!log.is_pressed());
    }

    // --- KeyInputLog::set_data ---

    #[test]
    fn test_set_data_overwrites() {
        let mut log = KeyInputLog::new();
        log.set_data(9999, 5, true);
        assert_eq!(log.get_time(), 9999);
        assert_eq!(log.get_keycode(), 5);
        assert!(log.is_pressed());
    }

    // --- KeyInputLog::get_time (presstime vs time fallback) ---

    #[test]
    fn test_get_time_uses_presstime_when_set() {
        let log = KeyInputLog::with_data(12345, 0, true);
        assert_eq!(log.get_time(), 12345);
    }

    #[test]
    fn test_get_time_falls_back_to_time_ms_converted() {
        // Simulate old format: presstime=0, time=500 (ms)
        let log = KeyInputLog {
            presstime: 0,
            keycode: 0,
            pressed: false,
            time: 500,
        };
        // Should return time * 1000 = 500000 us
        assert_eq!(log.get_time(), 500_000);
    }

    #[test]
    fn test_get_time_prefers_presstime_over_time() {
        let log = KeyInputLog {
            presstime: 42,
            keycode: 0,
            pressed: false,
            time: 999,
        };
        // presstime is non-zero, so it should be returned
        assert_eq!(log.get_time(), 42);
    }

    // --- KeyInputLog::validate ---

    #[test]
    fn test_validate_migrates_old_time_format() {
        let mut log = KeyInputLog {
            presstime: 0,
            keycode: 2,
            pressed: true,
            time: 300,
        };
        let valid = log.validate();
        assert!(valid);
        // presstime should now be time*1000
        assert_eq!(log.get_time(), 300_000);
    }

    #[test]
    fn test_validate_clears_time_after_migration() {
        let mut log = KeyInputLog {
            presstime: 0,
            keycode: 0,
            pressed: false,
            time: 100,
        };
        log.validate();
        // After migration, the old `time` field is zeroed
        assert_eq!(log.time, 0);
    }

    #[test]
    fn test_validate_no_migration_when_time_is_zero() {
        let mut log = KeyInputLog::with_data(5000, 1, true);
        let valid = log.validate();
        assert!(valid);
        assert_eq!(log.get_time(), 5000);
    }

    #[test]
    fn test_validate_returns_false_for_negative_presstime() {
        let mut log = KeyInputLog::with_data(-100, 0, false);
        assert!(!log.validate());
    }

    #[test]
    fn test_validate_returns_false_for_negative_keycode() {
        let mut log = KeyInputLog::with_data(0, -1, false);
        assert!(!log.validate());
    }

    #[test]
    fn test_validate_returns_true_for_zero_values() {
        let mut log = KeyInputLog::new();
        assert!(log.validate());
    }

    // --- create_autoplay_log ---

    /// Helper: build a minimal BMSModel with the given mode and timelines.
    fn make_model(mode: Mode, timelines: Vec<TimeLine>) -> BMSModel {
        let mut model = BMSModel::new();
        // Must set timelines before set_mode, because set_mode resizes lane counts
        model.set_all_time_line(timelines);
        model.set_mode(mode);
        model
    }

    #[test]
    fn test_autoplay_log_empty_model() {
        let model = BMSModel::new(); // no mode set
        let log = KeyInputLog::create_autoplay_log(&model);
        assert!(log.is_empty());
    }

    #[test]
    fn test_autoplay_log_no_notes() {
        // Model with mode but no timelines
        let model = make_model(Mode::BEAT_7K, Vec::new());
        let log = KeyInputLog::create_autoplay_log(&model);
        assert!(log.is_empty());
    }

    #[test]
    fn test_autoplay_log_normal_note_generates_press() {
        // BEAT_7K has 8 keys (lanes 0-7), scratch at lane 7
        let mut tl = TimeLine::new(0.0, 1_000_000, 8); // 1 second in microseconds
        tl.set_note(0, Some(Note::new_normal(1)));

        let model = make_model(Mode::BEAT_7K, vec![tl]);
        let log = KeyInputLog::create_autoplay_log(&model);

        // Should have at least one press event for lane 0
        let presses: Vec<_> = log
            .iter()
            .filter(|l| l.get_keycode() == 0 && l.is_pressed())
            .collect();
        assert!(!presses.is_empty(), "should have press event for lane 0");
        assert_eq!(presses[0].get_time(), 1_000_000);
    }

    #[test]
    fn test_autoplay_log_long_note_generates_press_and_release() {
        // BEAT_7K: 8 keys
        let mut tl_start = TimeLine::new(0.0, 1_000_000, 8);
        let ln_start = Note::new_long(1);
        // Mark start (end=false is default)
        tl_start.set_note(2, Some(ln_start));

        let mut tl_end = TimeLine::new(1.0, 2_000_000, 8);
        let mut ln_end = Note::new_long(1);
        ln_end.set_end(true);
        tl_end.set_note(2, Some(ln_end));

        let model = make_model(Mode::BEAT_7K, vec![tl_start, tl_end]);
        let log = KeyInputLog::create_autoplay_log(&model);

        // Find press at lane 2
        let press = log.iter().find(|l| l.get_keycode() == 2 && l.is_pressed());
        assert!(press.is_some(), "should have press for LN start at lane 2");
        assert_eq!(press.unwrap().get_time(), 1_000_000);

        // Find release at lane 2
        let release = log.iter().find(|l| l.get_keycode() == 2 && !l.is_pressed());
        assert!(
            release.is_some(),
            "should have release for LN end at lane 2"
        );
        assert_eq!(release.unwrap().get_time(), 2_000_000);
    }

    #[test]
    fn test_autoplay_log_empty_lanes_generate_release() {
        // When a lane has no note and no active LN, the algorithm generates a release event.
        // POPN_5K: 5 keys, no scratch
        let tl = TimeLine::new(0.0, 500_000, 5);
        // All lanes empty -- no notes set

        let model = make_model(Mode::POPN_5K, vec![tl]);
        let log = KeyInputLog::create_autoplay_log(&model);

        // All generated events should be releases (pressed=false)
        for entry in &log {
            assert!(
                !entry.is_pressed(),
                "empty lanes should generate release events"
            );
        }
        // Should have exactly 5 release events (one per lane)
        assert_eq!(log.len(), 5);
    }

    #[test]
    fn test_autoplay_log_scratch_lane_release_generates_extra() {
        // When scratch lane has no note and no active LN, it generates
        // release for both lane and lane+1
        // BEAT_7K: scratch at lane 7
        let tl = TimeLine::new(0.0, 100_000, 8);
        // All lanes empty

        let model = make_model(Mode::BEAT_7K, vec![tl]);
        let log = KeyInputLog::create_autoplay_log(&model);

        // Lane 7 (scratch) should generate two release events: lane 7 and lane 8
        let scratch_releases: Vec<_> = log
            .iter()
            .filter(|l| (l.get_keycode() == 7 || l.get_keycode() == 8) && !l.is_pressed())
            .collect();
        assert_eq!(scratch_releases.len(), 2);
    }
}
