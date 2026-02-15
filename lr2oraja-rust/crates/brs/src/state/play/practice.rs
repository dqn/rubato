// Practice mode configuration and UI logic.
//
// Ported from Java: PracticeConfiguration.java
// Manages the practice settings menu displayed before play starts.

use std::path::PathBuf;

use bms_config::PracticeProperty;
use bms_input::control_keys::ControlKeys;
use bms_model::{BmsModel, PlayMode};

use crate::input_mapper::InputState;

/// Practice menu cursor items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PracticeElement {
    StartTime,
    EndTime,
    GaugeType,
    GaugeValue,
    JudgeRank,
    Total,
    Freq,
    GraphType,
    Option1P,
    Option2P,
    OptionDP,
}

#[allow(dead_code)]
impl PracticeElement {
    /// All elements in display order for SP mode.
    const SP_ELEMENTS: [PracticeElement; 9] = [
        PracticeElement::StartTime,
        PracticeElement::EndTime,
        PracticeElement::GaugeType,
        PracticeElement::GaugeValue,
        PracticeElement::JudgeRank,
        PracticeElement::Total,
        PracticeElement::Freq,
        PracticeElement::GraphType,
        PracticeElement::Option1P,
    ];

    /// All elements in display order for DP mode.
    const DP_ELEMENTS: [PracticeElement; 11] = [
        PracticeElement::StartTime,
        PracticeElement::EndTime,
        PracticeElement::GaugeType,
        PracticeElement::GaugeValue,
        PracticeElement::JudgeRank,
        PracticeElement::Total,
        PracticeElement::Freq,
        PracticeElement::GraphType,
        PracticeElement::Option1P,
        PracticeElement::Option2P,
        PracticeElement::OptionDP,
    ];

    /// Display label for this element.
    pub fn label(self) -> &'static str {
        match self {
            PracticeElement::StartTime => "START TIME",
            PracticeElement::EndTime => "END TIME",
            PracticeElement::GaugeType => "GAUGE TYPE",
            PracticeElement::GaugeValue => "GAUGE VALUE",
            PracticeElement::JudgeRank => "JUDGE RANK",
            PracticeElement::Total => "TOTAL",
            PracticeElement::Freq => "FREQ",
            PracticeElement::GraphType => "GRAPH TYPE",
            PracticeElement::Option1P => "OPTION 1P",
            PracticeElement::Option2P => "OPTION 2P",
            PracticeElement::OptionDP => "OPTION DP",
        }
    }
}

/// Gauge type names matching Java PracticeConfiguration.
const GAUGE_NAMES: [&str; 9] = [
    "ASSIST EASY",
    "EASY",
    "NORMAL",
    "HARD",
    "EX-HARD",
    "HAZARD",
    "GRADE",
    "EX GRADE",
    "EXHARD GRADE",
];

/// Random option names.
const RANDOM_NAMES: [&str; 10] = [
    "NORMAL",
    "MIRROR",
    "RANDOM",
    "R-RANDOM",
    "S-RANDOM",
    "SPIRAL",
    "H-RANDOM",
    "ALL-SCR",
    "RANDOM+",
    "S-RANDOM+",
];

/// Double option names.
const DOUBLE_NAMES: [&str; 2] = ["NORMAL", "FLIP"];

/// Graph type names.
const GRAPH_NAMES: [&str; 3] = ["NOTETYPE", "JUDGE", "EARLYLATE"];

/// Key repeat threshold in milliseconds (initial delay).
#[allow(dead_code)]
const KEY_REPEAT_DELAY_MS: i64 = 500;
/// Key repeat interval in milliseconds.
#[allow(dead_code)]
const KEY_REPEAT_INTERVAL_MS: i64 = 50;

/// Practice configuration state — manages cursor, property, and key repeat.
pub struct PracticeConfiguration {
    /// Per-song practice settings.
    pub property: PracticeProperty,
    /// Current cursor position (index into element list).
    cursor_pos: usize,
    /// Key repeat counter (milliseconds held).
    press_count: i64,
    /// Last time of the model in milliseconds (for time range clamping).
    model_last_time_ms: i32,
    /// Whether the chart is DP.
    is_dp: bool,
    /// Whether the chart is PMS (PopN 5K/9K).
    is_pms: bool,
    /// SHA256 of the chart (for persistence).
    sha256: String,
    /// Config directory (for save/load).
    config_dir: PathBuf,
}

#[allow(dead_code)]
impl PracticeConfiguration {
    /// Create a new practice configuration from a model.
    pub fn new(model: &BmsModel, config_dir: PathBuf) -> Self {
        let sha256 = model.sha256.clone();
        let mut property = PracticeProperty::load(&config_dir, &sha256);

        let last_time_ms = model.last_event_time_ms();
        let is_dp = model.mode.player_count() > 1;
        let is_pms = matches!(model.mode, PlayMode::PopN5K | PlayMode::PopN9K);

        // Initialize endtime from model if not previously set
        if property.endtime <= 0 {
            property.endtime = last_time_ms;
        }

        // Clamp to model range
        property.starttime = property.starttime.clamp(0, last_time_ms);
        property.endtime = property.endtime.clamp(property.starttime, last_time_ms);

        // Initialize judgerank and total from model if defaults
        if property.judgerank == 100 {
            property.judgerank = model.judge_rank;
        }
        if (property.total - 300.0).abs() < f64::EPSILON {
            property.total = model.total;
        }

        Self {
            property,
            cursor_pos: 0,
            press_count: 0,
            model_last_time_ms: last_time_ms,
            is_dp,
            is_pms,
            sha256,
            config_dir,
        }
    }

    /// Get the element list based on SP/DP mode.
    fn elements(&self) -> &[PracticeElement] {
        if self.is_dp {
            &PracticeElement::DP_ELEMENTS
        } else {
            &PracticeElement::SP_ELEMENTS
        }
    }

    /// Current cursor element.
    pub fn current_element(&self) -> PracticeElement {
        self.elements()[self.cursor_pos]
    }

    /// Current cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursor_pos
    }

    /// Number of menu items.
    pub fn element_count(&self) -> usize {
        self.elements().len()
    }

    /// Process input for the practice menu.
    ///
    /// Returns true if the user pressed a play key (1KEY) to start playing.
    pub fn process_input(&mut self, input_state: &InputState) -> bool {
        for key in &input_state.pressed_keys {
            match key {
                ControlKeys::Up => {
                    if self.cursor_pos > 0 {
                        self.cursor_pos -= 1;
                    }
                    self.press_count = 0;
                }
                ControlKeys::Down => {
                    let max = self.elements().len() - 1;
                    if self.cursor_pos < max {
                        self.cursor_pos += 1;
                    }
                    self.press_count = 0;
                }
                ControlKeys::Left => {
                    self.adjust_value(-1);
                    self.press_count = 0;
                }
                ControlKeys::Right => {
                    self.adjust_value(1);
                    self.press_count = 0;
                }
                ControlKeys::Num1 | ControlKeys::Enter => {
                    // Start playing
                    return true;
                }
                _ => {}
            }
        }
        false
    }

    /// Adjust the current menu item value by `delta` (-1 or +1).
    fn adjust_value(&mut self, delta: i32) {
        let element = self.current_element();
        let p = &mut self.property;

        match element {
            PracticeElement::StartTime => {
                p.starttime = (p.starttime + delta * 100).clamp(0, p.endtime);
            }
            PracticeElement::EndTime => {
                p.endtime = (p.endtime + delta * 100).clamp(p.starttime, self.model_last_time_ms);
            }
            PracticeElement::GaugeType => {
                p.gaugetype = (p.gaugetype + delta).rem_euclid(GAUGE_NAMES.len() as i32);
                // PMS modes: clamp gauge value to 100 when switching to hard+ gauge types
                if self.is_pms && p.gaugetype >= 3 && p.startgauge > 100 {
                    p.startgauge = 100;
                }
            }
            PracticeElement::GaugeValue => {
                // PMS ASSIST_EASY/EASY/NORMAL have max=120, all others max=100
                let max = if self.is_pms && p.gaugetype < 3 {
                    120
                } else {
                    100
                };
                p.startgauge = (p.startgauge + delta).clamp(1, max);
            }
            PracticeElement::JudgeRank => {
                p.judgerank = (p.judgerank + delta).clamp(1, 400);
            }
            PracticeElement::Total => {
                p.total = (p.total + delta as f64 * 10.0).clamp(20.0, 5000.0);
            }
            PracticeElement::Freq => {
                p.freq = (p.freq + delta * 5).clamp(50, 200);
            }
            PracticeElement::GraphType => {
                p.graphtype = (p.graphtype + delta).rem_euclid(GRAPH_NAMES.len() as i32);
            }
            PracticeElement::Option1P => {
                p.random = (p.random + delta).rem_euclid(RANDOM_NAMES.len() as i32);
            }
            PracticeElement::Option2P => {
                p.random2 = (p.random2 + delta).rem_euclid(RANDOM_NAMES.len() as i32);
            }
            PracticeElement::OptionDP => {
                p.doubleop = (p.doubleop + delta).rem_euclid(DOUBLE_NAMES.len() as i32);
            }
        }
    }

    /// Save current practice property to disk.
    pub fn save_property(&self) {
        if let Err(e) = self.property.save(&self.config_dir, &self.sha256) {
            tracing::warn!("Failed to save practice property: {e}");
        }
    }

    /// Format a value for display.
    pub fn value_text(&self, element: PracticeElement) -> String {
        let p = &self.property;
        match element {
            PracticeElement::StartTime => format_time_ms(p.starttime),
            PracticeElement::EndTime => format_time_ms(p.endtime),
            PracticeElement::GaugeType => GAUGE_NAMES
                .get(p.gaugetype as usize)
                .unwrap_or(&"?")
                .to_string(),
            PracticeElement::GaugeValue => format!("{}", p.startgauge),
            PracticeElement::JudgeRank => format!("{}", p.judgerank),
            PracticeElement::Total => format!("{:.0}", p.total),
            PracticeElement::Freq => format!("{}%", p.freq),
            PracticeElement::GraphType => GRAPH_NAMES
                .get(p.graphtype as usize)
                .unwrap_or(&"?")
                .to_string(),
            PracticeElement::Option1P => RANDOM_NAMES
                .get(p.random as usize)
                .unwrap_or(&"?")
                .to_string(),
            PracticeElement::Option2P => RANDOM_NAMES
                .get(p.random2 as usize)
                .unwrap_or(&"?")
                .to_string(),
            PracticeElement::OptionDP => DOUBLE_NAMES
                .get(p.doubleop as usize)
                .unwrap_or(&"?")
                .to_string(),
        }
    }
}

/// Format milliseconds as "MM:SS.d".
#[allow(dead_code)]
fn format_time_ms(ms: i32) -> String {
    let total_seconds = ms / 1000;
    let minutes = total_seconds / 60;
    let seconds = total_seconds % 60;
    let tenths = (ms % 1000) / 100;
    format!("{minutes:02}:{seconds:02}.{tenths}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::{BmsModel, Note, PlayMode};

    fn make_practice_model() -> BmsModel {
        BmsModel {
            mode: PlayMode::Beat7K,
            total: 400.0,
            judge_rank: 100,
            sha256: "test_sha256".to_string(),
            notes: vec![
                Note::normal(0, 1_000_000, 1),
                Note::normal(1, 5_000_000, 2),
                Note::normal(2, 10_000_000, 3),
            ],
            ..Default::default()
        }
    }

    #[test]
    fn new_initializes_from_model() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        assert_eq!(pc.property.endtime, 10000); // last_event_time_ms = 10000
        assert!((pc.property.total - 400.0).abs() < f64::EPSILON);
        assert!(!pc.is_dp);
        assert_eq!(pc.element_count(), 9); // SP elements
    }

    #[test]
    fn cursor_movement() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        assert_eq!(pc.cursor_pos(), 0);
        assert_eq!(pc.current_element(), PracticeElement::StartTime);

        // Move down
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Down],
        };
        pc.process_input(&input);
        assert_eq!(pc.cursor_pos(), 1);
        assert_eq!(pc.current_element(), PracticeElement::EndTime);

        // Move up
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Up],
        };
        pc.process_input(&input);
        assert_eq!(pc.cursor_pos(), 0);

        // Move up at top stays at 0
        pc.process_input(&input);
        assert_eq!(pc.cursor_pos(), 0);
    }

    #[test]
    fn adjust_freq() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        // Navigate to Freq (index 6)
        pc.cursor_pos = 6;
        assert_eq!(pc.current_element(), PracticeElement::Freq);
        assert_eq!(pc.property.freq, 100);

        // Right -> increase by 5
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Right],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.freq, 105);

        // Left -> decrease by 5
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Left],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.freq, 100);
    }

    #[test]
    fn freq_clamped() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        pc.cursor_pos = 6;

        pc.property.freq = 50;
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Left],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.freq, 50); // Clamped at minimum

        pc.property.freq = 200;
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Right],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.freq, 200); // Clamped at maximum
    }

    #[test]
    fn gauge_type_wraps() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        pc.cursor_pos = 2; // GaugeType

        pc.property.gaugetype = 8;
        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Right],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.gaugetype, 0); // Wraps around

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Left],
        };
        pc.process_input(&input);
        assert_eq!(pc.property.gaugetype, 8); // Wraps back
    }

    #[test]
    fn enter_returns_true() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Enter],
        };
        assert!(pc.process_input(&input));
    }

    #[test]
    fn num1_returns_true() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        let input = InputState {
            commands: vec![],
            pressed_keys: vec![ControlKeys::Num1],
        };
        assert!(pc.process_input(&input));
    }

    #[test]
    fn save_and_reload() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        pc.property.freq = 75;
        pc.property.starttime = 2000;
        pc.save_property();

        let pc2 = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        assert_eq!(pc2.property.freq, 75);
        assert_eq!(pc2.property.starttime, 2000);
    }

    #[test]
    fn format_time_ms_basic() {
        assert_eq!(format_time_ms(0), "00:00.0");
        assert_eq!(format_time_ms(1500), "00:01.5");
        assert_eq!(format_time_ms(65300), "01:05.3");
        assert_eq!(format_time_ms(125000), "02:05.0");
    }

    #[test]
    fn dp_mode_has_extra_elements() {
        let mut model = make_practice_model();
        model.mode = PlayMode::Beat14K;
        let dir = tempfile::tempdir().unwrap();
        let pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        assert_eq!(pc.element_count(), 11);
        assert!(pc.is_dp);
    }

    #[test]
    fn pms_gauge_max_120_for_normal_types() {
        let mut model = make_practice_model();
        model.mode = PlayMode::PopN9K;
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        assert!(pc.is_pms);

        // ASSIST_EASY (gaugetype=0): max=120
        pc.cursor_pos = 3; // GaugeValue
        pc.property.gaugetype = 0;
        pc.property.startgauge = 119;
        pc.adjust_value(1);
        assert_eq!(pc.property.startgauge, 120);
        pc.adjust_value(1);
        assert_eq!(pc.property.startgauge, 120); // clamped at 120
    }

    #[test]
    fn pms_gauge_max_100_for_hard_types() {
        let mut model = make_practice_model();
        model.mode = PlayMode::PopN9K;
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        // HARD (gaugetype=3): max=100
        pc.cursor_pos = 3; // GaugeValue
        pc.property.gaugetype = 3;
        pc.property.startgauge = 100;
        pc.adjust_value(1);
        assert_eq!(pc.property.startgauge, 100); // clamped at 100
    }

    #[test]
    fn pms_gauge_type_change_clamps_value() {
        let mut model = make_practice_model();
        model.mode = PlayMode::PopN9K;
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        // Set gauge value to 120 on NORMAL (gaugetype=2)
        pc.property.gaugetype = 2;
        pc.property.startgauge = 120;

        // Switch to HARD (gaugetype=3) -> should clamp to 100
        pc.cursor_pos = 2; // GaugeType
        pc.adjust_value(1);
        assert_eq!(pc.property.gaugetype, 3);
        assert_eq!(pc.property.startgauge, 100);
    }

    #[test]
    fn non_pms_gauge_max_always_100() {
        let model = make_practice_model(); // Beat7K
        let dir = tempfile::tempdir().unwrap();
        let mut pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());
        assert!(!pc.is_pms);

        pc.cursor_pos = 3; // GaugeValue
        pc.property.gaugetype = 0; // ASSIST_EASY
        pc.property.startgauge = 100;
        pc.adjust_value(1);
        assert_eq!(pc.property.startgauge, 100); // max=100 even for non-PMS ASSIST_EASY
    }

    #[test]
    fn value_text_formatting() {
        let model = make_practice_model();
        let dir = tempfile::tempdir().unwrap();
        let pc = PracticeConfiguration::new(&model, dir.path().to_path_buf());

        assert_eq!(pc.value_text(PracticeElement::Freq), "100%");
        assert_eq!(pc.value_text(PracticeElement::GaugeType), "NORMAL");
        assert_eq!(pc.value_text(PracticeElement::Option1P), "NORMAL");
    }
}
