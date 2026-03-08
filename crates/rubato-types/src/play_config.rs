use crate::stubs::JudgeAlgorithm;

pub const HISPEED_MAX: f32 = 20.0;
pub const HISPEED_MIN: f32 = 0.01;

pub const DURATION_MAX: i32 = 10000;
pub const DURATION_MIN: i32 = 1;

pub const CONSTANT_FADEIN_MAX: i32 = 1000;
pub const CONSTANT_FADEIN_MIN: i32 = -1000;

pub const FIX_HISPEED_OFF: i32 = 0;
pub const FIX_HISPEED_STARTBPM: i32 = 1;
pub const FIX_HISPEED_MAXBPM: i32 = 2;
pub const FIX_HISPEED_MAINBPM: i32 = 3;
pub const FIX_HISPEED_MINBPM: i32 = 4;

pub const HISPEEDMARGIN_MAX: f32 = 10.0;
pub const HISPEEDMARGIN_MIN: f32 = 0.0;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct PlayConfig {
    pub hispeed: f32,
    pub duration: i32,
    #[serde(rename = "enableConstant")]
    pub enable_constant: bool,
    #[serde(rename = "constantFadeinTime")]
    pub constant_fadein_time: i32,
    pub fixhispeed: i32,
    pub hispeedmargin: f32,
    pub lanecover: f32,
    pub enablelanecover: bool,
    pub lift: f32,
    pub enablelift: bool,
    pub hidden: f32,
    pub enablehidden: bool,
    pub lanecovermarginlow: f32,
    pub lanecovermarginhigh: f32,
    pub lanecoverswitchduration: i32,
    pub hispeedautoadjust: bool,
    pub judgetype: String,
}

impl Default for PlayConfig {
    fn default() -> Self {
        PlayConfig {
            hispeed: 1.0,
            duration: 500,
            enable_constant: false,
            constant_fadein_time: 100,
            fixhispeed: FIX_HISPEED_MAINBPM,
            hispeedmargin: 0.25,
            lanecover: 0.2,
            enablelanecover: true,
            lift: 0.1,
            enablelift: false,
            hidden: 0.1,
            enablehidden: false,
            lanecovermarginlow: 0.001,
            lanecovermarginhigh: 0.01,
            lanecoverswitchduration: 500,
            hispeedautoadjust: false,
            judgetype: "Combo".to_string(),
        }
    }
}

impl PlayConfig {
    pub fn validate(&mut self) {
        self.hispeed = self.hispeed.clamp(HISPEED_MIN, HISPEED_MAX);
        self.duration = self.duration.clamp(DURATION_MIN, DURATION_MAX);
        self.constant_fadein_time = self
            .constant_fadein_time
            .clamp(CONSTANT_FADEIN_MIN, CONSTANT_FADEIN_MAX);
        self.hispeedmargin = self
            .hispeedmargin
            .clamp(HISPEEDMARGIN_MIN, HISPEEDMARGIN_MAX);
        self.fixhispeed = self.fixhispeed.clamp(0, FIX_HISPEED_MINBPM);
        self.lanecover = self.lanecover.clamp(0.0, 1.0);
        self.lift = self.lift.clamp(0.0, 1.0);
        self.hidden = self.hidden.clamp(0.0, 1.0);
        self.lanecovermarginlow = self.lanecovermarginlow.clamp(0.0, 1.0);
        self.lanecovermarginhigh = self.lanecovermarginhigh.clamp(0.0, 1.0);
        self.lanecoverswitchduration = self.lanecoverswitchduration.clamp(0, 1000000);
        if JudgeAlgorithm::index(&self.judgetype) == -1 {
            self.judgetype = "Combo".to_string();
        }
    }

    pub fn judgetype(&self) -> &str {
        for alg in JudgeAlgorithm::values() {
            if alg.name() == self.judgetype {
                return &self.judgetype;
            }
        }
        "Combo"
    }
}
