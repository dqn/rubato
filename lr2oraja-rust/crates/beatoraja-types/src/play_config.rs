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
        if JudgeAlgorithm::get_index(&self.judgetype) == -1 {
            self.judgetype = "Combo".to_string();
        }
    }

    pub fn is_enablelift(&self) -> bool {
        self.enablelift
    }

    pub fn set_enablelift(&mut self, v: bool) {
        self.enablelift = v;
    }

    pub fn get_lift(&self) -> f32 {
        self.lift
    }

    pub fn set_lift(&mut self, v: f32) {
        self.lift = v;
    }

    pub fn is_enablehidden(&self) -> bool {
        self.enablehidden
    }

    pub fn set_enablehidden(&mut self, v: bool) {
        self.enablehidden = v;
    }

    pub fn get_hidden(&self) -> f32 {
        self.hidden
    }

    pub fn set_hidden(&mut self, v: f32) {
        self.hidden = v;
    }

    pub fn is_enablelanecover(&self) -> bool {
        self.enablelanecover
    }

    pub fn set_enablelanecover(&mut self, v: bool) {
        self.enablelanecover = v;
    }

    pub fn get_lanecover(&self) -> f32 {
        self.lanecover
    }

    pub fn set_lanecover(&mut self, v: f32) {
        self.lanecover = v;
    }

    pub fn get_lanecovermarginlow(&self) -> f32 {
        self.lanecovermarginlow
    }

    pub fn set_lanecovermarginlow(&mut self, v: f32) {
        self.lanecovermarginlow = v;
    }

    pub fn get_lanecovermarginhigh(&self) -> f32 {
        self.lanecovermarginhigh
    }

    pub fn set_lanecovermarginhigh(&mut self, v: f32) {
        self.lanecovermarginhigh = v;
    }

    pub fn get_lanecoverswitchduration(&self) -> i32 {
        self.lanecoverswitchduration
    }

    pub fn set_lanecoverswitchduration(&mut self, v: i32) {
        self.lanecoverswitchduration = v;
    }

    pub fn is_enable_constant(&self) -> bool {
        self.enable_constant
    }

    pub fn set_enable_constant(&mut self, v: bool) {
        self.enable_constant = v;
    }

    pub fn get_constant_fadein_time(&self) -> i32 {
        self.constant_fadein_time
    }

    pub fn set_constant_fadein_time(&mut self, v: i32) {
        self.constant_fadein_time = v;
    }

    pub fn get_hispeed(&self) -> f32 {
        self.hispeed
    }

    pub fn set_hispeed(&mut self, v: f32) {
        self.hispeed = v;
    }

    pub fn get_duration(&self) -> i32 {
        self.duration
    }

    pub fn set_duration(&mut self, v: i32) {
        self.duration = v;
    }

    pub fn get_fixhispeed(&self) -> i32 {
        self.fixhispeed
    }

    pub fn set_fixhispeed(&mut self, v: i32) {
        self.fixhispeed = v;
    }

    pub fn get_hispeedmargin(&self) -> f32 {
        self.hispeedmargin
    }

    pub fn is_hispeedautoadjust(&self) -> bool {
        self.hispeedautoadjust
    }

    pub fn get_judgetype(&self) -> String {
        for alg in JudgeAlgorithm::values() {
            if alg.name() == self.judgetype {
                return self.judgetype.clone();
            }
        }
        "Combo".to_string()
    }
}
