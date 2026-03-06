use crate::bms_player_rule::BMSPlayerRule;
use crate::gauge_property::GaugeProperty;
use crate::groove_gauge::{GrooveGauge, create_groove_gauge};
use bms_model::bms_model::BMSModel;
use bms_model::bms_model_utils;
use bms_model::mode::Mode;
use rubato_core::pattern::lane_shuffle_modifier::PlayerFlipModifier;
use rubato_core::pattern::pattern_modifier::{PatternModifier, create_pattern_modifier};
use rubato_core::pattern::practice_modifier::PracticeModifier;
use rubato_types::player_config::PlayerConfig;
use serde::{Deserialize, Serialize};

static GAUGE: &[&str] = &[
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
static RANDOM: &[&str] = &[
    "NORMAL",
    "MIRROR",
    "RANDOM",
    "R-RANDOM",
    "S-RANDOM",
    "SPIRAL",
    "H-RANDOM",
    "ALL-SCR",
    "RANDOM-EX",
    "S-RANDOM-EX",
];
static DPRANDOM: &[&str] = &["NORMAL", "FLIP"];
static GRAPHTYPESTR: &[&str] = &["NOTETYPE", "JUDGE", "EARLYLATE"];

/// Colors used in practice configuration UI drawing.
#[derive(Clone, Debug, PartialEq)]
pub enum PracticeColor {
    Yellow,
    Cyan,
    Orange,
    White,
}

/// Draw commands emitted by PracticeConfiguration.draw().
/// The skin layer executes these using SkinObjectRenderer.
#[derive(Clone, Debug)]
pub enum PracticeDrawCommand {
    /// Draw text at position with color
    DrawText {
        text: String,
        x: f32,
        y: f32,
        color: PracticeColor,
    },
    /// Draw note distribution graph
    DrawGraph {
        graph_type: i32,
        region: (f32, f32, f32, f32),
        start_time: i32,
        end_time: i32,
        freq: f32,
    },
}

/// Practice mode settings
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PracticeProperty {
    /// Play start time
    pub starttime: i32,
    /// Play end time
    pub endtime: i32,
    /// Selected gauge category
    #[serde(skip)]
    pub gaugecategory: Option<GaugeProperty>,
    /// Selected gauge type
    pub gaugetype: i32,
    /// Start gauge value
    pub startgauge: i32,
    /// 1P option
    pub random: i32,
    /// 2P option
    pub random2: i32,
    /// DP option
    pub doubleop: i32,
    /// Judge window
    pub judgerank: i32,
    /// Playback speed ratio
    pub freq: i32,
    /// TOTAL value
    pub total: f64,
    /// Graph type
    pub graphtype: i32,
}

impl PracticeProperty {
    pub fn new() -> Self {
        PracticeProperty {
            starttime: 0,
            endtime: 10000,
            gaugecategory: None,
            gaugetype: 2,
            startgauge: 20,
            random: 0,
            random2: 0,
            doubleop: 0,
            judgerank: 100,
            freq: 100,
            total: 0.0,
            graphtype: 0,
        }
    }

    /// Clamp all index fields to valid array bounds.
    /// Called after deserialization to prevent out-of-bounds panics.
    fn sanitize(&mut self) {
        self.gaugetype = self.gaugetype.rem_euclid(GAUGE.len() as i32);
        self.random = self.random.rem_euclid(RANDOM.len() as i32);
        self.random2 = self.random2.rem_euclid(RANDOM.len() as i32);
        self.doubleop = self.doubleop.rem_euclid(DPRANDOM.len() as i32);
        self.graphtype = self.graphtype.rem_euclid(GRAPHTYPESTR.len() as i32);
    }
}

/// Result of applying practice configuration to a model.
/// Returned by `PracticeConfiguration::apply_to_model`.
pub struct PracticeApplyResult {
    /// Groove gauge initialized with practice start gauge
    pub gauge: Option<GrooveGauge>,
    /// Frequency ratio if freq != 100 (caller should set global audio pitch)
    pub freq_ratio: Option<f32>,
    /// Adjusted start time offset in milliseconds
    pub starttimeoffset: i64,
    /// Adjusted play time limit in milliseconds
    pub playtime: i64,
}

/// Cached model data for practice mode (extracted from BMSModel to avoid Clone)
struct PracticeModelData {
    mode: Mode,
    /// Time values from all timelines (used for start/end time bounds)
    timeline_times: Vec<i32>,
}

/// Practice mode configuration display/edit
pub struct PracticeConfiguration {
    cursorpos: usize,
    presscount: i64,
    model_data: Option<PracticeModelData>,
    property: PracticeProperty,
    /// SHA256 of the current model (for save/load path)
    sha256: String,
}

impl Default for PracticeConfiguration {
    fn default() -> Self {
        Self::new()
    }
}

impl PracticeConfiguration {
    pub fn new() -> Self {
        PracticeConfiguration {
            cursorpos: 0,
            presscount: 0,
            model_data: None,
            property: PracticeProperty::new(),
            sha256: String::new(),
        }
    }

    pub fn create(&mut self, model: &BMSModel) {
        self.sha256 = model.sha256().to_string();
        self.property.judgerank = model.judgerank();
        self.property.endtime = model.last_time() + 1000;

        // Load saved practice property from practice/<sha256>.json if exists
        if !self.sha256.is_empty() {
            let path = format!("practice/{}.json", self.sha256);
            if let Ok(data) = std::fs::read_to_string(&path)
                && let Ok(mut saved) = serde_json::from_str::<PracticeProperty>(&data)
            {
                saved.sanitize();
                self.property = saved;
                // Restore model-specific data
                let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
                let timeline_times: Vec<i32> =
                    model.all_time_lines().iter().map(|tl| tl.time()).collect();
                self.model_data = Some(PracticeModelData {
                    mode,
                    timeline_times,
                });
                return;
            }
        }

        if self.property.gaugecategory.is_none() {
            let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
            self.property.gaugecategory = Some(BMSPlayerRule::for_mode(&mode).gauge);
        }
        let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        let timeline_times: Vec<i32> = model.all_time_lines().iter().map(|tl| tl.time()).collect();
        self.model_data = Some(PracticeModelData {
            mode,
            timeline_times,
        });
        if self.property.total == 0.0 {
            self.property.total = model.total();
        }
    }

    /// Save practice property to practice/<sha256>.json.
    /// Translates: PracticeConfiguration.saveProperty()
    pub fn save_property(&self) {
        if self.sha256.is_empty() {
            return;
        }
        let path = format!("practice/{}.json", self.sha256);
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        match serde_json::to_string_pretty(&self.property) {
            Ok(json) => {
                if let Err(e) = std::fs::write(&path, json) {
                    log::warn!("Failed to save practice property: {}", e);
                }
            }
            Err(e) => {
                log::warn!("Failed to serialize practice property: {}", e);
            }
        }
    }

    pub fn practice_property(&self) -> &PracticeProperty {
        &self.property
    }

    pub fn practice_property_mut(&mut self) -> &mut PracticeProperty {
        &mut self.property
    }

    pub fn gauge(&self, model: &BMSModel) -> Option<GrooveGauge> {
        let gauge_category = self
            .property
            .gaugecategory
            .unwrap_or(GaugeProperty::SevenKeys);
        let mut gauge =
            create_groove_gauge(model, self.property.gaugetype, 0, Some(gauge_category))?;
        gauge.set_value(self.property.startgauge as f32);
        Some(gauge)
    }

    pub fn dispose(&mut self) {
        // cleanup rendering resources (stub - no GPU resources in Rust translation)
    }

    /// Apply practice settings to the model.
    ///
    /// Translates Java BMSPlayer lines 684-723 (practice mode initialization).
    /// Modifies the model in-place (frequency, total, time range, pattern, judgerank).
    /// Returns timing offsets and gauge for the caller.
    pub fn apply_to_model(
        &self,
        model: &mut BMSModel,
        config: &PlayerConfig,
    ) -> PracticeApplyResult {
        let property = &self.property;

        // Frequency change
        let freq_ratio = if property.freq != 100 {
            let ratio = property.freq as f32 / 100.0;
            bms_model_utils::change_frequency(model, ratio);
            Some(ratio)
        } else {
            None
        };

        // Set total
        model.set_total(property.total);

        // PracticeModifier: filter notes outside the time range (scaled by freq)
        let pm_start = (property.starttime as i64) * 100 / (property.freq as i64);
        let pm_end = (property.endtime as i64) * 100 / (property.freq as i64);
        let mut pm = PracticeModifier::new(pm_start, pm_end);
        pm.modify(model);

        // DP modifiers
        let mode = model.mode().cloned().unwrap_or(Mode::BEAT_7K);
        if mode.player() == 2 {
            if property.doubleop == 1 {
                PlayerFlipModifier::new().modify(model);
            }
            create_pattern_modifier(property.random2, 1, &mode, config).modify(model);
        }

        // 1P random modifier
        create_pattern_modifier(property.random, 0, &mode, config).modify(model);

        // Second PracticeModifier application (preserves Java behavior)
        pm.modify(model);

        // Gauge
        let gauge = self.gauge(model);

        // Judge rank
        model.set_judgerank(property.judgerank);

        // Timing calculations
        let starttimeoffset = if property.starttime > 1000 {
            (property.starttime as i64 - 1000) * 100 / property.freq as i64
        } else {
            0
        };
        let playtime = (property.endtime as i64 + 1000) * 100 / property.freq as i64;

        PracticeApplyResult {
            gauge,
            freq_ratio,
            starttimeoffset,
            playtime,
        }
    }

    /// Process input for practice mode elements
    pub fn process_input_action(&mut self, element_index: usize, inc: bool) {
        match element_index {
            0 => {
                // STARTTIME
                if let Some(ref md) = self.model_data {
                    let times = &md.timeline_times;
                    if inc {
                        if !times.is_empty()
                            && self.property.starttime + 2000 <= *times.last().unwrap()
                        {
                            self.property.starttime += 100;
                        }
                        if self.property.starttime + 900 >= self.property.endtime {
                            self.property.endtime += 100;
                        }
                    } else if self.property.starttime >= 100 {
                        self.property.starttime -= 100;
                    }
                }
            }
            1 => {
                // ENDTIME
                if let Some(ref md) = self.model_data {
                    let times = &md.timeline_times;
                    if inc {
                        if !times.is_empty()
                            && self.property.endtime <= *times.last().unwrap() + 1000
                        {
                            self.property.endtime += 100;
                        }
                    } else if self.property.endtime > self.property.starttime + 1000 {
                        self.property.endtime -= 100;
                    }
                }
            }
            2 => {
                // GAUGETYPE
                self.property.gaugetype = (self.property.gaugetype + if inc { 1 } else { 8 }) % 9;
                if let Some(ref md) = self.model_data {
                    let mode = &md.mode;
                    if (*mode == Mode::POPN_5K || *mode == Mode::POPN_9K)
                        && self.property.gaugetype >= 3
                        && self.property.startgauge > 100
                    {
                        self.property.startgauge = 100;
                    }
                }
            }
            3 => {
                // GAUGECATEGORY
                let categories = GaugeProperty::values();
                if let Some(current) = self.property.gaugecategory {
                    for i in 0..categories.len() {
                        if current == categories[i] {
                            let next = if inc {
                                (i + 1) % categories.len()
                            } else {
                                (i + categories.len() - 1) % categories.len()
                            };
                            self.property.gaugecategory = Some(categories[next]);
                            let values = categories[next].element_values();
                            self.property.startgauge =
                                values[self.property.gaugetype as usize].init as i32;
                            break;
                        }
                    }
                }
            }
            4 => {
                // GAUGEVALUE
                if let Some(category) = self.property.gaugecategory {
                    let values = category.element_values();
                    let max = values[self.property.gaugetype as usize].max as i32;
                    self.property.startgauge =
                        (self.property.startgauge + if inc { 1 } else { -1 }).clamp(1, max);
                }
            }
            5 => {
                // JUDGERANK
                self.property.judgerank =
                    (self.property.judgerank + if inc { 1 } else { -1 }).clamp(1, 400);
            }
            6 => {
                // TOTAL
                self.property.total =
                    (self.property.total + if inc { 10.0 } else { -10.0 }).clamp(20.0, 5000.0);
            }
            7 => {
                // FREQ
                self.property.freq = (self.property.freq + if inc { 5 } else { -5 }).clamp(50, 200);
            }
            8 => {
                // GRAPHTYPE
                self.property.graphtype = (self.property.graphtype + if inc { 1 } else { 2 }) % 3;
            }
            9 => {
                // OPTION1P
                let options = if let Some(ref md) = self.model_data {
                    let mode = &md.mode;
                    if *mode == Mode::POPN_5K || *mode == Mode::POPN_9K {
                        7
                    } else {
                        10
                    }
                } else {
                    10
                };
                self.property.random =
                    (self.property.random + if inc { 1 } else { options - 1 }) % options;
            }
            10 => {
                // OPTION2P
                self.property.random2 = (self.property.random2 + if inc { 1 } else { 9 }) % 10;
            }
            11 => {
                // OPTIONDP
                self.property.doubleop = (self.property.doubleop + 1) % 2;
            }
            _ => {}
        }
    }

    pub fn element_text(&self, index: usize) -> String {
        match index {
            0 => format!(
                "START TIME : {:2}:{:02}.{:1}",
                self.property.starttime / 60000,
                (self.property.starttime / 1000) % 60,
                (self.property.starttime / 100) % 10
            ),
            1 => format!(
                "END TIME : {:2}:{:02}.{:1}",
                self.property.endtime / 60000,
                (self.property.endtime / 1000) % 60,
                (self.property.endtime / 100) % 10
            ),
            2 => format!(
                "GAUGE TYPE : {}",
                GAUGE.get(self.property.gaugetype as usize).unwrap_or(&"?")
            ),
            3 => format!(
                "GAUGE CATEGORY : {}",
                self.property
                    .gaugecategory
                    .map_or("".to_string(), |g| g.name().to_string())
            ),
            4 => format!("GAUGE VALUE : {}", self.property.startgauge),
            5 => format!("JUDGERANK : {}", self.property.judgerank),
            6 => format!("TOTAL : {}", self.property.total as i32),
            7 => format!("FREQUENCY : {}", self.property.freq),
            8 => format!(
                "GRAPHTYPE : {}",
                GRAPHTYPESTR
                    .get(self.property.graphtype as usize)
                    .unwrap_or(&"?")
            ),
            9 => format!(
                "OPTION-1P : {}",
                RANDOM.get(self.property.random as usize).unwrap_or(&"?")
            ),
            10 => format!(
                "OPTION-2P : {}",
                RANDOM.get(self.property.random2 as usize).unwrap_or(&"?")
            ),
            11 => format!(
                "OPTION-DP : {}",
                DPRANDOM
                    .get(self.property.doubleop as usize)
                    .unwrap_or(&"?")
            ),
            _ => String::new(),
        }
    }

    /// Generate draw commands for practice configuration UI.
    ///
    /// Translated from: Java PracticeConfiguration.draw(Rectangle r, SkinObjectRenderer sprite, long time, MainState state)
    ///
    /// Returns a list of `PracticeDrawCommand` that the caller (skin layer)
    /// executes using SkinObjectRenderer.
    ///
    /// `region`: the BGA region rectangle (x, y, width, height)
    /// `judge_counts`: array of [total, fast, slow] for each of 6 judge types
    /// `media_loaded`: whether the audio/BGA has finished loading
    pub fn draw(
        &self,
        region: (f32, f32, f32, f32),
        judge_counts: &[(i32, i32, i32); 6],
        media_loaded: bool,
    ) -> Vec<PracticeDrawCommand> {
        let (rx, ry, rw, rh) = region;
        let x = rx + rw / 8.0;
        let y = ry + rh * 7.0 / 8.0;
        let mut commands = Vec::new();

        // Draw element labels
        for i in 0..Self::ELEMENT_COUNT {
            if self.is_element_visible(i) {
                let color = if self.cursorpos == i {
                    PracticeColor::Yellow
                } else {
                    PracticeColor::Cyan
                };
                commands.push(PracticeDrawCommand::DrawText {
                    text: self.element_text(i),
                    x,
                    y: y - 22.0 * i as f32,
                    color,
                });
            }
        }

        // "PRESS 1KEY TO PLAY" prompt
        if media_loaded {
            commands.push(PracticeDrawCommand::DrawText {
                text: "PRESS 1KEY TO PLAY".to_string(),
                x,
                y: y - 276.0,
                color: PracticeColor::Orange,
            });
        }

        // Judge count table
        let judge_labels = [
            "PGREAT :", "GREAT  :", "GOOD   :", "BAD    :", "POOR   :", "KPOOR  :",
        ];
        for i in 0..6 {
            let (total, fast, slow) = judge_counts[i];
            commands.push(PracticeDrawCommand::DrawText {
                text: format!("{} {} {} {}", judge_labels[i], total, fast, slow),
                x: x + 250.0,
                y: y - (i as f32 * 22.0),
                color: PracticeColor::White,
            });
        }

        // Graph drawing command (graph type, region, time range, frequency)
        commands.push(PracticeDrawCommand::DrawGraph {
            graph_type: self.property.graphtype,
            region: (rx, ry, rw, rh / 4.0),
            start_time: self.property.starttime,
            end_time: self.property.endtime,
            freq: self.property.freq as f32 / 100.0,
        });

        commands
    }

    /// Number of practice configuration elements (indices 0..ELEMENT_COUNT).
    const ELEMENT_COUNT: usize = 12;

    /// Process input for practice mode navigation.
    ///
    /// Translated from: Java PracticeConfiguration.processInput(BMSPlayerInputProcessor input)
    /// Navigates cursor with UP/DOWN, adjusts values with LEFT/RIGHT.
    ///
    /// `control_up_pressed`: control key UP was pressed this frame
    /// `control_down_pressed`: control key DOWN was pressed this frame
    /// `control_left_held`: control key LEFT is currently held
    /// `control_right_held`: control key RIGHT is currently held
    /// `now_millis`: current time in milliseconds (for repeat logic)
    pub fn process_input(
        &mut self,
        control_up_pressed: bool,
        control_down_pressed: bool,
        control_left_held: bool,
        control_right_held: bool,
        now_millis: i64,
    ) {
        let element_count = Self::ELEMENT_COUNT;

        // Move cursor up (skip invisible elements)
        if control_up_pressed {
            loop {
                self.cursorpos = (self.cursorpos + element_count - 1) % element_count;
                if self.is_element_visible(self.cursorpos) {
                    break;
                }
            }
        }
        // Move cursor down (skip invisible elements)
        if control_down_pressed {
            loop {
                self.cursorpos = (self.cursorpos + 1) % element_count;
                if self.is_element_visible(self.cursorpos) {
                    break;
                }
            }
        }

        // Left: decrement current element (with repeat)
        if control_left_held && (self.presscount == 0 || self.presscount + 10 < now_millis) {
            if self.presscount == 0 {
                self.presscount = now_millis + 500;
            } else {
                self.presscount = now_millis;
            }
            self.process_input_action(self.cursorpos, false);
        } else if control_right_held && (self.presscount == 0 || self.presscount + 10 < now_millis)
        {
            // Right: increment current element (with repeat)
            if self.presscount == 0 {
                self.presscount = now_millis + 500;
            } else {
                self.presscount = now_millis;
            }
            self.process_input_action(self.cursorpos, true);
        } else if !control_left_held && !control_right_held {
            self.presscount = 0;
        }
    }

    /// Get current cursor position.
    pub fn cursor_pos(&self) -> usize {
        self.cursorpos
    }

    pub fn is_element_visible(&self, index: usize) -> bool {
        match index {
            10 | 11 => {
                // OPTION2P, OPTIONDP only visible in DP mode
                if let Some(ref md) = self.model_data {
                    md.mode.player() == 2
                } else {
                    false
                }
            }
            _ => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bms_model::time_line::TimeLine;

    fn make_test_model(mode: &Mode, times: &[i32]) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(mode.clone());
        let mut timelines = Vec::new();
        for &t in times {
            let mut tl = TimeLine::new(t.into(), 0, mode.key());
            tl.set_bpm(120.0);
            timelines.push(tl);
        }
        model.set_all_time_line(timelines);
        model.set_total(300.0);
        model.set_judgerank(100);
        model
    }

    #[test]
    fn test_apply_to_model_default_freq() {
        let mut practice = PracticeConfiguration::new();
        practice.property.freq = 100;
        practice.property.total = 250.0;
        practice.property.judgerank = 80;
        practice.property.starttime = 0;
        practice.property.endtime = 10000;
        practice.property.gaugecategory = Some(GaugeProperty::SevenKeys);

        let mut model = make_test_model(&Mode::BEAT_7K, &[0, 1000, 5000, 9000]);
        let config = PlayerConfig::default();

        let result = practice.apply_to_model(&mut model, &config);

        // freq == 100 → no frequency change
        assert!(result.freq_ratio.is_none());
        // total overwritten
        assert!((model.total() - 250.0).abs() < f64::EPSILON);
        // judgerank overwritten
        assert_eq!(model.judgerank(), 80);
        // starttimeoffset: starttime(0) <= 1000 → 0
        assert_eq!(result.starttimeoffset, 0);
        // playtime: (10000 + 1000) * 100 / 100 = 11000
        assert_eq!(result.playtime, 11000);
    }

    #[test]
    fn test_apply_to_model_half_speed() {
        let mut practice = PracticeConfiguration::new();
        practice.property.freq = 50;
        practice.property.total = 200.0;
        practice.property.judgerank = 100;
        practice.property.starttime = 2000;
        practice.property.endtime = 8000;
        practice.property.gaugecategory = Some(GaugeProperty::SevenKeys);

        let mut model = make_test_model(&Mode::BEAT_7K, &[0, 1000, 5000, 9000]);
        let config = PlayerConfig::default();

        let result = practice.apply_to_model(&mut model, &config);

        // freq == 50 → ratio = 0.5
        assert_eq!(result.freq_ratio, Some(0.5));
        // starttimeoffset: (2000 - 1000) * 100 / 50 = 2000
        assert_eq!(result.starttimeoffset, 2000);
        // playtime: (8000 + 1000) * 100 / 50 = 18000
        assert_eq!(result.playtime, 18000);
    }

    #[test]
    fn test_apply_to_model_returns_gauge() {
        let mut practice = PracticeConfiguration::new();
        practice.property.gaugecategory = Some(GaugeProperty::SevenKeys);
        practice.property.gaugetype = 2; // NORMAL
        practice.property.startgauge = 50;

        let mut model = make_test_model(&Mode::BEAT_7K, &[0, 5000]);
        let config = PlayerConfig::default();

        let result = practice.apply_to_model(&mut model, &config);

        // Gauge should be created with startgauge value
        assert!(result.gauge.is_some());
        let gauge = result.gauge.unwrap();
        assert!((gauge.value() - 50.0).abs() < f64::EPSILON as f32);
    }

    #[test]
    fn test_apply_to_model_starttime_below_1000() {
        let mut practice = PracticeConfiguration::new();
        practice.property.starttime = 500;
        practice.property.endtime = 5000;
        practice.property.freq = 100;
        practice.property.gaugecategory = Some(GaugeProperty::SevenKeys);

        let mut model = make_test_model(&Mode::BEAT_7K, &[0, 2000]);
        let config = PlayerConfig::default();

        let result = practice.apply_to_model(&mut model, &config);

        // starttime(500) <= 1000 → offset = 0
        assert_eq!(result.starttimeoffset, 0);
    }

    // --- process_input tests ---

    /// Helper to make a model with real micro-times for practice tests.
    fn make_timed_model(mode: &Mode, time_millis: &[i32]) -> BMSModel {
        let mut model = BMSModel::new();
        model.set_mode(mode.clone());
        let mut timelines = Vec::new();
        for &t_ms in time_millis {
            let micro = t_ms as i64 * 1000;
            let tl = TimeLine::new(120.0, micro, mode.key());
            timelines.push(tl);
        }
        model.set_all_time_line(timelines);
        model.set_total(300.0);
        model.set_judgerank(100);
        model
    }

    #[test]
    fn process_input_down_advances_cursor() {
        let mut practice = PracticeConfiguration::new();
        let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
        practice.create(&model);
        assert_eq!(practice.cursor_pos(), 0);

        practice.process_input(false, true, false, false, 1000);
        assert_eq!(practice.cursor_pos(), 1);
    }

    #[test]
    fn process_input_up_wraps_cursor() {
        let mut practice = PracticeConfiguration::new();
        let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
        practice.create(&model);
        assert_eq!(practice.cursor_pos(), 0);

        // UP from 0 should go to element 9 (skipping invisible 10, 11 in SP)
        practice.process_input(true, false, false, false, 1000);
        assert_eq!(practice.cursor_pos(), 9);
    }

    #[test]
    fn process_input_right_increments_value() {
        let mut practice = PracticeConfiguration::new();
        // Need timeline times large enough so starttime + 2000 <= last_time
        let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
        practice.create(&model);

        let start_before = practice.practice_property().starttime;
        // Right held = increment. presscount starts at 0, so first press triggers immediately.
        practice.process_input(false, false, false, true, 1000);
        let start_after = practice.practice_property().starttime;

        // cursor at 0 = STARTTIME, right should increment by 100
        assert_eq!(start_after, start_before + 100);
    }

    #[test]
    fn process_input_left_decrements_value() {
        let mut practice = PracticeConfiguration::new();
        let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
        practice.create(&model);

        // First set starttime to something > 0 so we can decrement
        practice.practice_property_mut().starttime = 500;

        practice.process_input(false, false, true, false, 1000);
        assert_eq!(practice.practice_property().starttime, 400);
    }

    #[test]
    fn process_input_resets_presscount_when_no_lr() {
        let mut practice = PracticeConfiguration::new();
        let model = make_timed_model(&Mode::BEAT_7K, &[0, 60000]);
        practice.create(&model);

        // Trigger a press to set presscount
        practice.process_input(false, false, false, true, 1000);
        assert_ne!(practice.presscount, 0);

        // Release both → presscount resets
        practice.process_input(false, false, false, false, 1500);
        assert_eq!(practice.presscount, 0);
    }

    // --- draw() tests ---

    #[test]
    fn draw_emits_element_text_commands() {
        let practice = PracticeConfiguration::new();
        let region = (0.0, 0.0, 800.0, 600.0);
        let judge_counts = [(0, 0, 0); 6];

        let commands = practice.draw(region, &judge_counts, false);

        // Should have element text commands for visible elements (indices 0..9 in SP mode)
        let text_cmds: Vec<_> = commands
            .iter()
            .filter(|c| matches!(c, PracticeDrawCommand::DrawText { .. }))
            .collect();
        // 10 elements visible in SP (0..9) + 6 judge count lines = 16 text commands
        // (no "PRESS 1KEY" because media_loaded is false)
        assert_eq!(text_cmds.len(), 16);
    }

    #[test]
    fn draw_emits_press_1key_when_media_loaded() {
        let practice = PracticeConfiguration::new();
        let region = (0.0, 0.0, 800.0, 600.0);
        let judge_counts = [(0, 0, 0); 6];

        let commands = practice.draw(region, &judge_counts, true);

        let press_cmd = commands.iter().find(|c| match c {
            PracticeDrawCommand::DrawText { text, .. } => text.contains("PRESS 1KEY"),
            _ => false,
        });
        assert!(press_cmd.is_some());
    }

    #[test]
    fn draw_does_not_emit_press_1key_when_not_loaded() {
        let practice = PracticeConfiguration::new();
        let region = (0.0, 0.0, 800.0, 600.0);
        let judge_counts = [(0, 0, 0); 6];

        let commands = practice.draw(region, &judge_counts, false);

        let press_cmd = commands.iter().find(|c| match c {
            PracticeDrawCommand::DrawText { text, .. } => text.contains("PRESS 1KEY"),
            _ => false,
        });
        assert!(press_cmd.is_none());
    }

    #[test]
    fn draw_emits_graph_command() {
        let mut practice = PracticeConfiguration::new();
        practice.property.graphtype = 1;
        practice.property.starttime = 1000;
        practice.property.endtime = 5000;
        practice.property.freq = 100;
        let region = (10.0, 20.0, 400.0, 300.0);
        let judge_counts = [(0, 0, 0); 6];

        let commands = practice.draw(region, &judge_counts, false);

        let graph_cmd = commands
            .iter()
            .find(|c| matches!(c, PracticeDrawCommand::DrawGraph { .. }));
        assert!(graph_cmd.is_some());

        if let Some(PracticeDrawCommand::DrawGraph {
            graph_type,
            region: gr,
            start_time,
            end_time,
            freq,
        }) = graph_cmd
        {
            assert_eq!(*graph_type, 1);
            assert_eq!(*start_time, 1000);
            assert_eq!(*end_time, 5000);
            assert!((freq - 1.0).abs() < f32::EPSILON);
            // Region height should be rh / 4
            assert!((gr.3 - 75.0).abs() < f32::EPSILON);
        }
    }

    #[test]
    fn draw_cursor_position_colors_element_yellow() {
        let mut practice = PracticeConfiguration::new();
        // Move cursor to element 2
        practice.cursorpos = 2;
        let region = (0.0, 0.0, 800.0, 600.0);
        let judge_counts = [(0, 0, 0); 6];

        let commands = practice.draw(region, &judge_counts, false);

        // Element text commands: elements 0..9 visible
        // Element at index 2 (cursorpos) should be Yellow, others Cyan
        let element_texts: Vec<_> = commands
            .iter()
            .filter_map(|c| match c {
                PracticeDrawCommand::DrawText { color, text, .. }
                    if text.starts_with("START")
                        || text.starts_with("END")
                        || text.starts_with("GAUGE")
                        || text.starts_with("JUDGE")
                        || text.starts_with("TOTAL")
                        || text.starts_with("FREQ")
                        || text.starts_with("GRAPH")
                        || text.starts_with("OPTION") =>
                {
                    Some(color.clone())
                }
                _ => None,
            })
            .collect();

        // Element 2 should be Yellow (cursor position)
        assert_eq!(element_texts[2], PracticeColor::Yellow);
        // Element 0 should be Cyan (not cursor)
        assert_eq!(element_texts[0], PracticeColor::Cyan);
    }

    #[test]
    fn draw_judge_counts_are_white() {
        let practice = PracticeConfiguration::new();
        let region = (0.0, 0.0, 800.0, 600.0);
        let judge_counts = [
            (10, 3, 7),
            (5, 2, 3),
            (1, 0, 1),
            (0, 0, 0),
            (0, 0, 0),
            (0, 0, 0),
        ];

        let commands = practice.draw(region, &judge_counts, false);

        let white_texts: Vec<_> = commands
            .iter()
            .filter_map(|c| match c {
                PracticeDrawCommand::DrawText { text, color, .. }
                    if *color == PracticeColor::White =>
                {
                    Some(text.clone())
                }
                _ => None,
            })
            .collect();

        // Should have 6 judge count lines
        assert_eq!(white_texts.len(), 6);
        // First line should contain PGREAT and the counts
        assert!(white_texts[0].contains("PGREAT"));
        assert!(white_texts[0].contains("10"));
        assert!(white_texts[0].contains("3"));
        assert!(white_texts[0].contains("7"));
    }

    #[test]
    fn test_sanitize_clamps_out_of_bounds() {
        let mut prop = PracticeProperty::new();
        prop.gaugetype = 100;
        prop.random = -3;
        prop.random2 = 15;
        prop.doubleop = 10;
        prop.graphtype = -1;
        prop.sanitize();
        assert!((prop.gaugetype as usize) < GAUGE.len());
        assert!((prop.random as usize) < RANDOM.len());
        assert!((prop.random2 as usize) < RANDOM.len());
        assert!((prop.doubleop as usize) < DPRANDOM.len());
        assert!((prop.graphtype as usize) < GRAPHTYPESTR.len());
    }

    #[test]
    fn test_get_element_text_out_of_bounds_no_panic() {
        let mut pc = PracticeConfiguration::default();
        pc.property.gaugetype = 99;
        pc.property.random = -5;
        pc.property.random2 = 100;
        pc.property.doubleop = 50;
        pc.property.graphtype = -1;
        // Should not panic, should produce "?" for out-of-bounds
        let text = pc.element_text(2);
        assert!(text.contains("?"));
        let text = pc.element_text(9);
        assert!(text.contains("?"));
    }
}
