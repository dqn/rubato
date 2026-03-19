use crate::judge_algorithm::JudgeAlgorithm;

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
        if self.judgetype.parse::<JudgeAlgorithm>().is_err() {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn play_config_default_values() {
        let pc = PlayConfig::default();
        assert_eq!(pc.hispeed, 1.0);
        assert_eq!(pc.duration, 500);
        assert!(!pc.enable_constant);
        assert_eq!(pc.fixhispeed, FIX_HISPEED_MAINBPM);
        assert_eq!(pc.judgetype, "Combo");
    }

    #[test]
    fn play_config_serde_round_trip() {
        let mut pc = PlayConfig::default();
        pc.hispeed = 3.5;
        pc.duration = 800;
        pc.enable_constant = true;
        pc.lanecover = 0.5;
        pc.judgetype = "Score".to_string();

        let json = serde_json::to_string(&pc).unwrap();
        let deserialized: PlayConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.hispeed, 3.5);
        assert_eq!(deserialized.duration, 800);
        assert!(deserialized.enable_constant);
        assert_eq!(deserialized.lanecover, 0.5);
        assert_eq!(deserialized.judgetype, "Score");
    }

    #[test]
    fn play_config_deserialize_empty_object_uses_defaults() {
        let pc: PlayConfig = serde_json::from_str("{}").unwrap();
        let default = PlayConfig::default();
        assert_eq!(pc.hispeed, default.hispeed);
        assert_eq!(pc.duration, default.duration);
        assert_eq!(pc.judgetype, default.judgetype);
    }

    #[test]
    fn play_config_validate_clamps_hispeed() {
        let mut pc = PlayConfig::default();
        pc.hispeed = 0.0;
        pc.validate();
        assert_eq!(pc.hispeed, HISPEED_MIN);

        pc.hispeed = 100.0;
        pc.validate();
        assert_eq!(pc.hispeed, HISPEED_MAX);
    }

    #[test]
    fn play_config_validate_clamps_duration() {
        let mut pc = PlayConfig::default();
        pc.duration = 0;
        pc.validate();
        assert_eq!(pc.duration, DURATION_MIN);

        pc.duration = 99999;
        pc.validate();
        assert_eq!(pc.duration, DURATION_MAX);
    }

    #[test]
    fn play_config_validate_clamps_constant_fadein_time() {
        let mut pc = PlayConfig::default();
        pc.constant_fadein_time = -9999;
        pc.validate();
        assert_eq!(pc.constant_fadein_time, CONSTANT_FADEIN_MIN);

        pc.constant_fadein_time = 9999;
        pc.validate();
        assert_eq!(pc.constant_fadein_time, CONSTANT_FADEIN_MAX);
    }

    #[test]
    fn play_config_validate_clamps_lanecover_lift_hidden() {
        let mut pc = PlayConfig::default();
        pc.lanecover = -0.5;
        pc.lift = 1.5;
        pc.hidden = -1.0;
        pc.validate();
        assert_eq!(pc.lanecover, 0.0);
        assert_eq!(pc.lift, 1.0);
        assert_eq!(pc.hidden, 0.0);
    }

    #[test]
    fn play_config_validate_invalid_judgetype_resets_to_combo() {
        let mut pc = PlayConfig::default();
        pc.judgetype = "InvalidType".to_string();
        pc.validate();
        assert_eq!(pc.judgetype, "Combo");
    }

    #[test]
    fn play_config_validate_preserves_valid_judgetype() {
        let mut pc = PlayConfig::default();
        pc.judgetype = "Score".to_string();
        pc.validate();
        assert_eq!(pc.judgetype, "Score");
    }

    #[test]
    fn play_config_judgetype_returns_combo_for_unknown() {
        let mut pc = PlayConfig::default();
        pc.judgetype = "Unknown".to_string();
        assert_eq!(pc.judgetype(), "Combo");
    }

    #[test]
    fn play_config_judgetype_returns_name_for_valid() {
        let pc = PlayConfig::default();
        assert_eq!(pc.judgetype(), "Combo");
    }

    #[test]
    fn play_config_validate_clamps_fixhispeed() {
        let mut pc = PlayConfig::default();
        pc.fixhispeed = -1;
        pc.validate();
        assert_eq!(pc.fixhispeed, 0);

        pc.fixhispeed = 100;
        pc.validate();
        assert_eq!(pc.fixhispeed, FIX_HISPEED_MINBPM);
    }

    #[test]
    fn play_config_validate_clamps_hispeedmargin() {
        let mut pc = PlayConfig::default();
        pc.hispeedmargin = -1.0;
        pc.validate();
        assert_eq!(pc.hispeedmargin, HISPEEDMARGIN_MIN);

        pc.hispeedmargin = 100.0;
        pc.validate();
        assert_eq!(pc.hispeedmargin, HISPEEDMARGIN_MAX);
    }

    #[test]
    fn play_config_serializes_with_java_field_names() {
        let pc = PlayConfig::default();
        let json = serde_json::to_string(&pc).unwrap();
        assert!(
            json.contains("\"enableConstant\""),
            "missing enableConstant"
        );
        assert!(
            json.contains("\"constantFadeinTime\""),
            "missing constantFadeinTime"
        );
        assert!(!json.contains("\"enable_constant\""), "snake_case leak");
        assert!(
            !json.contains("\"constant_fadein_time\""),
            "snake_case leak"
        );
    }
}
