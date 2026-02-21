use super::float_property::FloatProperty;
use super::float_writer::FloatWriter;
use crate::stubs::MainState;

/// Returns a FloatProperty for the given RateType ID.
pub fn get_rate_property_by_id(optionid: i32) -> Option<Box<dyn FloatProperty>> {
    for rt in RATE_TYPES.iter() {
        if rt.id == optionid {
            return Some(Box::new(StubFloatProperty));
        }
    }
    None
}

/// Returns a FloatProperty for the given RateType name.
pub fn get_rate_property_by_name(name: &str) -> Option<Box<dyn FloatProperty>> {
    for rt in RATE_TYPES.iter() {
        if rt.name == name {
            return Some(Box::new(StubFloatProperty));
        }
    }
    None
}

/// Returns a FloatWriter for the given RateType ID.
pub fn get_rate_writer_by_id(id: i32) -> Option<Box<dyn FloatWriter>> {
    for rt in RATE_TYPES.iter() {
        if rt.id == id && rt.has_writer {
            return Some(Box::new(StubFloatWriter));
        }
    }
    None
}

/// Returns a FloatWriter for the given RateType name.
pub fn get_rate_writer_by_name(name: &str) -> Option<Box<dyn FloatWriter>> {
    for rt in RATE_TYPES.iter() {
        if rt.name == name && rt.has_writer {
            return Some(Box::new(StubFloatWriter));
        }
    }
    None
}

/// Returns a FloatProperty for the given FloatType or RateType ID.
pub fn get_float_property_by_id(optionid: i32) -> Option<Box<dyn FloatProperty>> {
    for ft in FLOAT_TYPES.iter() {
        if ft.id == optionid {
            return Some(Box::new(StubFloatProperty));
        }
    }
    for rt in RATE_TYPES.iter() {
        if rt.id == optionid {
            return Some(Box::new(StubFloatProperty));
        }
    }
    None
}

/// Returns a FloatProperty for the given FloatType or RateType name.
pub fn get_float_property_by_name(name: &str) -> Option<Box<dyn FloatProperty>> {
    for ft in FLOAT_TYPES.iter() {
        if ft.name == name {
            return Some(Box::new(StubFloatProperty));
        }
    }
    for rt in RATE_TYPES.iter() {
        if rt.name == name {
            return Some(Box::new(StubFloatProperty));
        }
    }
    None
}

// RateType enum data
struct RateTypeEntry {
    id: i32,
    name: &'static str,
    has_writer: bool,
}

static RATE_TYPES: &[RateTypeEntry] = &[
    RateTypeEntry {
        id: 1,
        name: "musicselect_position",
        has_writer: true,
    },
    RateTypeEntry {
        id: 4,
        name: "lanecover",
        has_writer: false,
    },
    RateTypeEntry {
        id: 5,
        name: "lanecover2",
        has_writer: false,
    },
    RateTypeEntry {
        id: 6,
        name: "music_progress",
        has_writer: false,
    },
    RateTypeEntry {
        id: 7,
        name: "skinselect_position",
        has_writer: true,
    },
    RateTypeEntry {
        id: 8,
        name: "ranking_position",
        has_writer: true,
    },
    RateTypeEntry {
        id: 17,
        name: "mastervolume",
        has_writer: true,
    },
    RateTypeEntry {
        id: 18,
        name: "keyvolume",
        has_writer: true,
    },
    RateTypeEntry {
        id: 19,
        name: "bgmvolume",
        has_writer: true,
    },
    RateTypeEntry {
        id: 101,
        name: "music_progress_bar",
        has_writer: false,
    },
    RateTypeEntry {
        id: 102,
        name: "load_progress",
        has_writer: false,
    },
    RateTypeEntry {
        id: 103,
        name: "level",
        has_writer: false,
    },
    RateTypeEntry {
        id: 105,
        name: "level_beginner",
        has_writer: false,
    },
    RateTypeEntry {
        id: 106,
        name: "level_normal",
        has_writer: false,
    },
    RateTypeEntry {
        id: 107,
        name: "level_hyper",
        has_writer: false,
    },
    RateTypeEntry {
        id: 108,
        name: "level_another",
        has_writer: false,
    },
    RateTypeEntry {
        id: 109,
        name: "level_insane",
        has_writer: false,
    },
    RateTypeEntry {
        id: 110,
        name: "scorerate",
        has_writer: false,
    },
    RateTypeEntry {
        id: 111,
        name: "scorerate_final",
        has_writer: false,
    },
    RateTypeEntry {
        id: 112,
        name: "bestscorerate_now",
        has_writer: false,
    },
    RateTypeEntry {
        id: 113,
        name: "bestscorerate",
        has_writer: false,
    },
    RateTypeEntry {
        id: 114,
        name: "targetscorerate_now",
        has_writer: false,
    },
    RateTypeEntry {
        id: 115,
        name: "targetscorerate",
        has_writer: false,
    },
    RateTypeEntry {
        id: 140,
        name: "rate_pgreat",
        has_writer: false,
    },
    RateTypeEntry {
        id: 141,
        name: "rate_great",
        has_writer: false,
    },
    RateTypeEntry {
        id: 142,
        name: "rate_good",
        has_writer: false,
    },
    RateTypeEntry {
        id: 143,
        name: "rate_bad",
        has_writer: false,
    },
    RateTypeEntry {
        id: 144,
        name: "rate_poor",
        has_writer: false,
    },
    RateTypeEntry {
        id: 145,
        name: "rate_maxcombo",
        has_writer: false,
    },
    RateTypeEntry {
        id: 147,
        name: "rate_exscore",
        has_writer: false,
    },
];

// FloatType enum data
struct FloatTypeEntry {
    id: i32,
    name: &'static str,
}

static FLOAT_TYPES: &[FloatTypeEntry] = &[
    FloatTypeEntry {
        id: 1102,
        name: "score_rate",
    },
    FloatTypeEntry {
        id: 1115,
        name: "total_rate",
    },
    FloatTypeEntry {
        id: 155,
        name: "score_rate2",
    },
    FloatTypeEntry {
        id: 372,
        name: "duration_average",
    },
    FloatTypeEntry {
        id: 374,
        name: "timing_average",
    },
    FloatTypeEntry {
        id: 376,
        name: "timign_stddev",
    },
    FloatTypeEntry {
        id: 85,
        name: "perfect_rate",
    },
    FloatTypeEntry {
        id: 86,
        name: "great_rate",
    },
    FloatTypeEntry {
        id: 87,
        name: "good_rate",
    },
    FloatTypeEntry {
        id: 88,
        name: "bad_rate",
    },
    FloatTypeEntry {
        id: 89,
        name: "poor_rate",
    },
    FloatTypeEntry {
        id: 285,
        name: "rival_perfect_rate",
    },
    FloatTypeEntry {
        id: 286,
        name: "rival_great_rate",
    },
    FloatTypeEntry {
        id: 287,
        name: "rival_good_rate",
    },
    FloatTypeEntry {
        id: 288,
        name: "rival_bad_rate",
    },
    FloatTypeEntry {
        id: 289,
        name: "rival_poor_rate",
    },
    FloatTypeEntry {
        id: 183,
        name: "best_rate",
    },
    FloatTypeEntry {
        id: 122,
        name: "rival_rate",
    },
    FloatTypeEntry {
        id: 135,
        name: "target_rate",
    },
    FloatTypeEntry {
        id: 157,
        name: "target_rate2",
    },
    FloatTypeEntry {
        id: 310,
        name: "hispeed",
    },
    FloatTypeEntry {
        id: 1107,
        name: "groovegauge_1p",
    },
    FloatTypeEntry {
        id: 367,
        name: "chart_averagedensity",
    },
    FloatTypeEntry {
        id: 362,
        name: "chart_enddensity",
    },
    FloatTypeEntry {
        id: 360,
        name: "chart_peakdensity",
    },
    FloatTypeEntry {
        id: 368,
        name: "chart_totalgauge",
    },
    FloatTypeEntry {
        id: 165,
        name: "loading_progress",
    },
    FloatTypeEntry {
        id: 227,
        name: "ir_totalclearrate",
    },
    FloatTypeEntry {
        id: 229,
        name: "ir_totalfullcomborate",
    },
    FloatTypeEntry {
        id: 203,
        name: "ir_player_noplay_rate",
    },
    FloatTypeEntry {
        id: 211,
        name: "ir_player_failed_rate",
    },
    FloatTypeEntry {
        id: 205,
        name: "ir_player_assist_rate",
    },
    FloatTypeEntry {
        id: 207,
        name: "ir_player_lightassist_rate",
    },
    FloatTypeEntry {
        id: 213,
        name: "ir_player_easy_rate",
    },
    FloatTypeEntry {
        id: 215,
        name: "ir_player_normal_rate",
    },
    FloatTypeEntry {
        id: 217,
        name: "ir_player_hard_rate",
    },
    FloatTypeEntry {
        id: 209,
        name: "ir_player_exhard_rate",
    },
    FloatTypeEntry {
        id: 219,
        name: "ir_player_fullcombo_rate",
    },
    FloatTypeEntry {
        id: 223,
        name: "ir_player_perfect_rate",
    },
    FloatTypeEntry {
        id: 225,
        name: "ir_player_max_rate",
    },
];

/// Stub FloatProperty that will be replaced when Phase 7+ is available.
struct StubFloatProperty;

impl FloatProperty for StubFloatProperty {
    fn get(&self, _state: &dyn MainState) -> f32 {
        todo!("Phase 7+ dependency: FloatPropertyFactory requires MainState subtypes")
    }
}

/// Stub FloatWriter that will be replaced when Phase 7+ is available.
struct StubFloatWriter;

impl FloatWriter for StubFloatWriter {
    fn set(&self, _state: &mut dyn MainState, _value: f32) {
        todo!("Phase 7+ dependency: FloatPropertyFactory requires MainState subtypes")
    }
}
