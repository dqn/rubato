use crate::bms_player_rule::BMSPlayerRule;
use crate::gauge_property::GaugeProperty;
use crate::groove_gauge::{GrooveGauge, create_groove_gauge};
use bms_model::bms_model::BMSModel;
use bms_model::mode::Mode;
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
        }
    }

    pub fn create(&mut self, model: &BMSModel) {
        self.property.judgerank = model.get_judgerank();
        self.property.endtime = model.get_last_time() + 1000;

        // TODO: load from practice/<sha256>.json if exists

        if self.property.gaugecategory.is_none() {
            let mode = model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
            self.property.gaugecategory = Some(BMSPlayerRule::get_bms_player_rule(&mode).gauge);
        }
        let mode = model.get_mode().cloned().unwrap_or(Mode::BEAT_7K);
        let timeline_times: Vec<i32> = model
            .get_all_time_lines()
            .iter()
            .map(|tl| tl.get_time())
            .collect();
        self.model_data = Some(PracticeModelData {
            mode,
            timeline_times,
        });
        if self.property.total == 0.0 {
            self.property.total = model.get_total();
        }
    }

    pub fn save_property(&self) {
        // TODO: save to practice/<sha256>.json
    }

    pub fn get_practice_property(&self) -> &PracticeProperty {
        &self.property
    }

    pub fn get_practice_property_mut(&mut self) -> &mut PracticeProperty {
        &mut self.property
    }

    pub fn get_gauge(&self, model: &BMSModel) -> Option<GrooveGauge> {
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
                            let values = categories[next].get_values();
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
                    let values = category.get_values();
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

    pub fn get_element_text(&self, index: usize) -> String {
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
            2 => format!("GAUGE TYPE : {}", GAUGE[self.property.gaugetype as usize]),
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
                GRAPHTYPESTR[self.property.graphtype as usize]
            ),
            9 => format!("OPTION-1P : {}", RANDOM[self.property.random as usize]),
            10 => format!("OPTION-2P : {}", RANDOM[self.property.random2 as usize]),
            11 => format!("OPTION-DP : {}", DPRANDOM[self.property.doubleop as usize]),
            _ => String::new(),
        }
    }

    /// Draw practice configuration UI.
    /// Corresponds to Java draw(Rectangle r, SkinObjectRenderer sprite, long time, MainState state).
    pub fn draw(&self, _time: i64) {
        // TODO: Phase 7+ dependency - requires Rectangle, SkinObjectRenderer, BitmapFont, MainState
        // In Java, this method:
        // 1. Iterates elements, draws text with yellow (cursor) or cyan color
        // 2. If media loaded, draws "PRESS 1KEY TO PLAY" in orange
        // 3. Draws judge count table (PGREAT/GREAT/GOOD/BAD/POOR/KPOOR)
        // 4. Draws practice graph at bottom quarter
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
