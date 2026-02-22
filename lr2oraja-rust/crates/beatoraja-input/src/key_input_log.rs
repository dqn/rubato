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
        // Java: "TODO 地雷を確実に回避するアルゴリズム" — not implemented in Java either
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
