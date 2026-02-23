use super::integer_property::IntegerProperty;
use crate::stubs::MainState;

const ID_LENGTH: usize = 65536;

/// Returns an IntegerProperty for the given option ID.
pub fn get_integer_property_by_id(optionid: i32) -> Option<Box<dyn IntegerProperty>> {
    if optionid < 0 || optionid as usize >= ID_LENGTH {
        return None;
    }

    // Check ValueType enum
    for vt in VALUE_TYPES.iter() {
        if vt.id == optionid {
            return Some(Box::new(DelegateIntegerProperty { id: vt.id }));
        }
    }

    // Check various range-based properties and switch-based properties
    // All reference BMSPlayer, MusicSelector, AbstractResult etc.
    Some(Box::new(DelegateIntegerProperty { id: optionid }))
}

/// Returns an IntegerProperty for the given ValueType name.
pub fn get_integer_property_by_name(name: &str) -> Option<Box<dyn IntegerProperty>> {
    for vt in VALUE_TYPES.iter() {
        if vt.name == name {
            return Some(Box::new(DelegateIntegerProperty { id: vt.id }));
        }
    }
    None
}

/// Returns an IntegerProperty for image index usage.
pub fn get_image_index_property_by_id(optionid: i32) -> Option<Box<dyn IntegerProperty>> {
    if optionid < 0 || optionid as usize >= ID_LENGTH {
        return None;
    }

    // Check IndexType enum
    for it in INDEX_TYPES.iter() {
        if it.id == optionid {
            return Some(Box::new(DelegateIntegerProperty { id: it.id }));
        }
    }

    // Judge properties (VALUE_JUDGE_1P_SCRATCH to VALUE_JUDGE_2P_KEY99)
    // SkinSelectType properties
    // All require Phase 7+ dependencies

    Some(Box::new(DelegateIntegerProperty { id: optionid }))
}

/// Returns an IntegerProperty for the given IndexType name.
pub fn get_image_index_property_by_name(name: &str) -> Option<Box<dyn IntegerProperty>> {
    for it in INDEX_TYPES.iter() {
        if it.name == name {
            return Some(Box::new(DelegateIntegerProperty { id: it.id }));
        }
    }
    None
}

// ValueType enum data
struct ValueTypeEntry {
    id: i32,
    name: &'static str,
}

static VALUE_TYPES: &[ValueTypeEntry] = &[
    ValueTypeEntry {
        id: 12,
        name: "notesdisplaytiming",
    },
    ValueTypeEntry {
        id: 17,
        name: "playtime_total_hour",
    },
    ValueTypeEntry {
        id: 18,
        name: "playtime_total_minute",
    },
    ValueTypeEntry {
        id: 19,
        name: "playtime_totla_saecond",
    },
    ValueTypeEntry {
        id: 20,
        name: "current_fps",
    },
    ValueTypeEntry {
        id: 21,
        name: "currenttime_year",
    },
    ValueTypeEntry {
        id: 22,
        name: "currenttime_month",
    },
    ValueTypeEntry {
        id: 23,
        name: "currenttime_day",
    },
    ValueTypeEntry {
        id: 24,
        name: "currenttime_hour",
    },
    ValueTypeEntry {
        id: 25,
        name: "currenttime_minute",
    },
    ValueTypeEntry {
        id: 26,
        name: "currenttime_saecond",
    },
    ValueTypeEntry {
        id: 27,
        name: "boottime_hour",
    },
    ValueTypeEntry {
        id: 28,
        name: "boottime_minute",
    },
    ValueTypeEntry {
        id: 29,
        name: "boottime_second",
    },
    ValueTypeEntry {
        id: 30,
        name: "player_playcount",
    },
    ValueTypeEntry {
        id: 31,
        name: "player_clearcount",
    },
    ValueTypeEntry {
        id: 32,
        name: "player_failcount",
    },
    ValueTypeEntry {
        id: 33,
        name: "player_perfect",
    },
    ValueTypeEntry {
        id: 34,
        name: "player_great",
    },
    ValueTypeEntry {
        id: 35,
        name: "player_good",
    },
    ValueTypeEntry {
        id: 36,
        name: "player_bad",
    },
    ValueTypeEntry {
        id: 37,
        name: "player_poor",
    },
    ValueTypeEntry {
        id: 333,
        name: "player_notes",
    },
    ValueTypeEntry {
        id: 57,
        name: "volume_system",
    },
    ValueTypeEntry {
        id: 58,
        name: "volume_key",
    },
    ValueTypeEntry {
        id: 59,
        name: "volume_background",
    },
    ValueTypeEntry {
        id: 77,
        name: "playcount",
    },
    ValueTypeEntry {
        id: 78,
        name: "clearcount",
    },
    ValueTypeEntry {
        id: 79,
        name: "failcount",
    },
    ValueTypeEntry {
        id: 90,
        name: "maxbpm",
    },
    ValueTypeEntry {
        id: 91,
        name: "minbpm",
    },
    ValueTypeEntry {
        id: 92,
        name: "mainbpm",
    },
    ValueTypeEntry {
        id: 160,
        name: "nowbpm",
    },
    ValueTypeEntry {
        id: 161,
        name: "playtime_minute",
    },
    ValueTypeEntry {
        id: 162,
        name: "playtime_second",
    },
    ValueTypeEntry {
        id: 163,
        name: "timeleft_minute",
    },
    ValueTypeEntry {
        id: 164,
        name: "timeleft_second",
    },
    ValueTypeEntry {
        id: 165,
        name: "loading_progress",
    },
    ValueTypeEntry {
        id: 179,
        name: "ir_rank",
    },
    ValueTypeEntry {
        id: 182,
        name: "ir_prevrank",
    },
    ValueTypeEntry {
        id: 202,
        name: "ir_player_noplay",
    },
    ValueTypeEntry {
        id: 210,
        name: "ir_player_failed",
    },
    ValueTypeEntry {
        id: 204,
        name: "ir_player_assist",
    },
    ValueTypeEntry {
        id: 206,
        name: "ir_player_lightassist",
    },
    ValueTypeEntry {
        id: 212,
        name: "ir_player_easy",
    },
    ValueTypeEntry {
        id: 214,
        name: "ir_player_normal",
    },
    ValueTypeEntry {
        id: 216,
        name: "ir_player_hard",
    },
    ValueTypeEntry {
        id: 208,
        name: "ir_player_exhard",
    },
    ValueTypeEntry {
        id: 218,
        name: "ir_player_fullcombo",
    },
    ValueTypeEntry {
        id: 222,
        name: "ir_player_perfect",
    },
    ValueTypeEntry {
        id: 224,
        name: "ir_player_max",
    },
    ValueTypeEntry {
        id: 220,
        name: "ir_update_waiting",
    },
    ValueTypeEntry {
        id: 226,
        name: "ir_totalclear",
    },
    ValueTypeEntry {
        id: 227,
        name: "ir_totalclearrate",
    },
    ValueTypeEntry {
        id: 241,
        name: "ir_totalclearrate_afterdot",
    },
    ValueTypeEntry {
        id: 228,
        name: "ir_totalfullcombo",
    },
    ValueTypeEntry {
        id: 229,
        name: "ir_totalfullcomborate",
    },
    ValueTypeEntry {
        id: 242,
        name: "ir_totalfullcomborate_afterdot",
    },
    ValueTypeEntry {
        id: 203,
        name: "ir_player_noplay_rate",
    },
    ValueTypeEntry {
        id: 230,
        name: "ir_player_noplay_rate_afterdot",
    },
    ValueTypeEntry {
        id: 211,
        name: "ir_player_failed_rate",
    },
    ValueTypeEntry {
        id: 234,
        name: "ir_player_failed_rate_afterdot",
    },
    ValueTypeEntry {
        id: 205,
        name: "ir_player_assist_rate",
    },
    ValueTypeEntry {
        id: 231,
        name: "ir_player_assist_rate_afterdot",
    },
    ValueTypeEntry {
        id: 207,
        name: "ir_player_lightassist_rate",
    },
    ValueTypeEntry {
        id: 232,
        name: "ir_player_lightassist_rate_afterdot",
    },
    ValueTypeEntry {
        id: 213,
        name: "ir_player_easy_rate",
    },
    ValueTypeEntry {
        id: 235,
        name: "ir_player_easy_rate_afterdot",
    },
    ValueTypeEntry {
        id: 215,
        name: "ir_player_normal_rate",
    },
    ValueTypeEntry {
        id: 236,
        name: "ir_player_normal_rate_afterdot",
    },
    ValueTypeEntry {
        id: 217,
        name: "ir_player_hard_rate",
    },
    ValueTypeEntry {
        id: 237,
        name: "ir_player_hard_rate_afterdot",
    },
    ValueTypeEntry {
        id: 209,
        name: "ir_player_exhard_rate",
    },
    ValueTypeEntry {
        id: 233,
        name: "ir_player_exhard_rate_afterdot",
    },
    ValueTypeEntry {
        id: 219,
        name: "ir_player_fullcombo_rate",
    },
    ValueTypeEntry {
        id: 238,
        name: "ir_player_fullcombo_rate_afterdot",
    },
    ValueTypeEntry {
        id: 223,
        name: "ir_player_perfect_rate",
    },
    ValueTypeEntry {
        id: 239,
        name: "ir_player_perfect_rate_afterdot",
    },
    ValueTypeEntry {
        id: 225,
        name: "ir_player_max_rate",
    },
    ValueTypeEntry {
        id: 240,
        name: "ir_player_max_rate_afterdot",
    },
    ValueTypeEntry {
        id: 312,
        name: "duration",
    },
    ValueTypeEntry {
        id: 313,
        name: "duration_green",
    },
    ValueTypeEntry {
        id: 320,
        name: "folder_noplay",
    },
    ValueTypeEntry {
        id: 321,
        name: "folder_failed",
    },
    ValueTypeEntry {
        id: 322,
        name: "folder_assist",
    },
    ValueTypeEntry {
        id: 323,
        name: "folder_lightassist",
    },
    ValueTypeEntry {
        id: 324,
        name: "folder_easy",
    },
    ValueTypeEntry {
        id: 325,
        name: "folder_normal",
    },
    ValueTypeEntry {
        id: 326,
        name: "folder_hard",
    },
    ValueTypeEntry {
        id: 327,
        name: "folder_exhard",
    },
    ValueTypeEntry {
        id: 328,
        name: "folder_fullcombo",
    },
    ValueTypeEntry {
        id: 329,
        name: "folder_prefect",
    },
    ValueTypeEntry {
        id: 330,
        name: "folder_max",
    },
    ValueTypeEntry {
        id: 350,
        name: "chart_totalnote_n",
    },
    ValueTypeEntry {
        id: 351,
        name: "chart_totalnote_ln",
    },
    ValueTypeEntry {
        id: 352,
        name: "chart_totalnote_s",
    },
    ValueTypeEntry {
        id: 353,
        name: "chart_totalnote_ls",
    },
    ValueTypeEntry {
        id: 364,
        name: "chart_averagedensity",
    },
    ValueTypeEntry {
        id: 365,
        name: "chart_averagedensity_afterdot",
    },
    ValueTypeEntry {
        id: 362,
        name: "chart_enddensity",
    },
    ValueTypeEntry {
        id: 363,
        name: "chart_enddensity_peak",
    },
    ValueTypeEntry {
        id: 360,
        name: "chart_peakdensity",
    },
    ValueTypeEntry {
        id: 361,
        name: "chart_peakdensity_afterdot",
    },
    ValueTypeEntry {
        id: 368,
        name: "chart_totalgauge",
    },
    ValueTypeEntry {
        id: 372,
        name: "duration_average",
    },
    ValueTypeEntry {
        id: 373,
        name: "duration_average_afterdot",
    },
    ValueTypeEntry {
        id: 374,
        name: "timing_average",
    },
    ValueTypeEntry {
        id: 375,
        name: "timing_average_afterdot",
    },
    ValueTypeEntry {
        id: 376,
        name: "timing_stddev",
    },
    ValueTypeEntry {
        id: 377,
        name: "timing_atddev_afterdot",
    },
    ValueTypeEntry {
        id: 380,
        name: "ranking_exscore1",
    },
    ValueTypeEntry {
        id: 381,
        name: "ranking_exscore2",
    },
    ValueTypeEntry {
        id: 382,
        name: "ranking_exscore3",
    },
    ValueTypeEntry {
        id: 383,
        name: "ranking_exscore4",
    },
    ValueTypeEntry {
        id: 384,
        name: "ranking_exscore5",
    },
    ValueTypeEntry {
        id: 385,
        name: "ranking_exscore6",
    },
    ValueTypeEntry {
        id: 386,
        name: "ranking_exscore7",
    },
    ValueTypeEntry {
        id: 387,
        name: "ranking_exscore8",
    },
    ValueTypeEntry {
        id: 388,
        name: "ranking_exscore9",
    },
    ValueTypeEntry {
        id: 389,
        name: "ranking_exscore10",
    },
    ValueTypeEntry {
        id: 390,
        name: "ranking_index1",
    },
    ValueTypeEntry {
        id: 391,
        name: "ranking_index2",
    },
    ValueTypeEntry {
        id: 392,
        name: "ranking_index3",
    },
    ValueTypeEntry {
        id: 393,
        name: "ranking_index4",
    },
    ValueTypeEntry {
        id: 394,
        name: "ranking_index5",
    },
    ValueTypeEntry {
        id: 395,
        name: "ranking_index6",
    },
    ValueTypeEntry {
        id: 396,
        name: "ranking_index7",
    },
    ValueTypeEntry {
        id: 397,
        name: "ranking_index8",
    },
    ValueTypeEntry {
        id: 398,
        name: "ranking_index9",
    },
    ValueTypeEntry {
        id: 399,
        name: "ranking_index10",
    },
    ValueTypeEntry {
        id: 400,
        name: "judgerank",
    },
    ValueTypeEntry {
        id: 525,
        name: "judge_duration1",
    },
    ValueTypeEntry {
        id: 526,
        name: "judge_duration2",
    },
    ValueTypeEntry {
        id: 527,
        name: "judge_duration3",
    },
    ValueTypeEntry {
        id: 1163,
        name: "chartlength_minute",
    },
    ValueTypeEntry {
        id: 1164,
        name: "chartlength_second",
    },
];

// IndexType enum data
struct IndexTypeEntry {
    id: i32,
    name: &'static str,
}

static INDEX_TYPES: &[IndexTypeEntry] = &[
    IndexTypeEntry {
        id: 303,
        name: "showjudgearea",
    },
    IndexTypeEntry {
        id: 305,
        name: "markprocessednote",
    },
    IndexTypeEntry {
        id: 306,
        name: "bpmguide",
    },
    IndexTypeEntry {
        id: 301,
        name: "customjudge",
    },
    IndexTypeEntry {
        id: 308,
        name: "lnmode",
    },
    IndexTypeEntry {
        id: 75,
        name: "notesdisplaytimingautoadjust",
    },
    IndexTypeEntry {
        id: 78,
        name: "gaugeautoshift",
    },
    IndexTypeEntry {
        id: 341,
        name: "bottomshiftablegauge",
    },
    IndexTypeEntry {
        id: 72,
        name: "bga",
    },
    IndexTypeEntry {
        id: 11,
        name: "mode",
    },
    IndexTypeEntry {
        id: 12,
        name: "sort",
    },
    IndexTypeEntry {
        id: 40,
        name: "gaugetype_1p",
    },
    IndexTypeEntry {
        id: 42,
        name: "option_1p",
    },
    IndexTypeEntry {
        id: 43,
        name: "option_2p",
    },
    IndexTypeEntry {
        id: 54,
        name: "option_dp",
    },
    IndexTypeEntry {
        id: 55,
        name: "hsfix",
    },
    IndexTypeEntry {
        id: 61,
        name: "option_target1_1p",
    },
    IndexTypeEntry {
        id: 62,
        name: "option_target1_2p",
    },
    IndexTypeEntry {
        id: 63,
        name: "option_target1_dp",
    },
    IndexTypeEntry {
        id: 342,
        name: "hispeedautoadjust",
    },
    IndexTypeEntry {
        id: 89,
        name: "favorite_song",
    },
    IndexTypeEntry {
        id: 90,
        name: "favorite_chart",
    },
    IndexTypeEntry {
        id: 321,
        name: "autosave_replay1",
    },
    IndexTypeEntry {
        id: 322,
        name: "autosave_replay2",
    },
    IndexTypeEntry {
        id: 323,
        name: "autosave_replay3",
    },
    IndexTypeEntry {
        id: 324,
        name: "autosave_replay4",
    },
    IndexTypeEntry {
        id: 330,
        name: "lanecover",
    },
    IndexTypeEntry {
        id: 331,
        name: "lift",
    },
    IndexTypeEntry {
        id: 332,
        name: "hidden",
    },
    IndexTypeEntry {
        id: 340,
        name: "judgealgorithm",
    },
    IndexTypeEntry {
        id: 343,
        name: "guidese",
    },
    IndexTypeEntry {
        id: 350,
        name: "extranotedepth",
    },
    IndexTypeEntry {
        id: 351,
        name: "minemode",
    },
    IndexTypeEntry {
        id: 352,
        name: "scrollmode",
    },
    IndexTypeEntry {
        id: 353,
        name: "longnotemode",
    },
    IndexTypeEntry {
        id: 360,
        name: "seventonine_pattern",
    },
    IndexTypeEntry {
        id: 361,
        name: "seventonine_type",
    },
    IndexTypeEntry {
        id: 370,
        name: "cleartype",
    },
    IndexTypeEntry {
        id: 371,
        name: "cleartype_target",
    },
    IndexTypeEntry {
        id: 390,
        name: "cleartype_ranking1",
    },
    IndexTypeEntry {
        id: 391,
        name: "cleartype_ranking2",
    },
    IndexTypeEntry {
        id: 392,
        name: "cleartype_ranking3",
    },
    IndexTypeEntry {
        id: 393,
        name: "cleartype_ranking4",
    },
    IndexTypeEntry {
        id: 394,
        name: "cleartype_ranking5",
    },
    IndexTypeEntry {
        id: 395,
        name: "cleartype_ranking6",
    },
    IndexTypeEntry {
        id: 396,
        name: "cleartype_ranking7",
    },
    IndexTypeEntry {
        id: 397,
        name: "cleartype_ranking8",
    },
    IndexTypeEntry {
        id: 398,
        name: "cleartype_ranking9",
    },
    IndexTypeEntry {
        id: 399,
        name: "cleartype_ranking10",
    },
    IndexTypeEntry {
        id: 400,
        name: "constant",
    },
    IndexTypeEntry {
        id: 450,
        name: "pattern_1p_1",
    },
    IndexTypeEntry {
        id: 451,
        name: "pattern_1p_2",
    },
    IndexTypeEntry {
        id: 452,
        name: "pattern_1p_3",
    },
    IndexTypeEntry {
        id: 453,
        name: "pattern_1p_4",
    },
    IndexTypeEntry {
        id: 454,
        name: "pattern_1p_5",
    },
    IndexTypeEntry {
        id: 455,
        name: "pattern_1p_6",
    },
    IndexTypeEntry {
        id: 456,
        name: "pattern_1p_7",
    },
    IndexTypeEntry {
        id: 457,
        name: "pattern_1p_8",
    },
    IndexTypeEntry {
        id: 458,
        name: "pattern_1p_9",
    },
    IndexTypeEntry {
        id: 459,
        name: "pattern_1p_SCR",
    },
    IndexTypeEntry {
        id: 460,
        name: "pattern_2p_1",
    },
    IndexTypeEntry {
        id: 461,
        name: "pattern_2p_2",
    },
    IndexTypeEntry {
        id: 462,
        name: "pattern_2p_3",
    },
    IndexTypeEntry {
        id: 463,
        name: "pattern_2p_4",
    },
    IndexTypeEntry {
        id: 464,
        name: "pattern_2p_5",
    },
    IndexTypeEntry {
        id: 465,
        name: "pattern_2p_6",
    },
    IndexTypeEntry {
        id: 466,
        name: "pattern_2p_7",
    },
    IndexTypeEntry {
        id: 469,
        name: "pattern_2p_SCR",
    },
    // Old spec assist options
    IndexTypeEntry {
        id: 1046,
        name: "assist_constant",
    },
    IndexTypeEntry {
        id: 1047,
        name: "assist_legacy",
    },
    IndexTypeEntry {
        id: 1048,
        name: "assist_nomine",
    },
];

/// Delegate IntegerProperty that reads values from MainState::integer_value().
/// This enables both StaticStateProvider (golden-master) and real game states
/// to provide integer values through the same interface.
struct DelegateIntegerProperty {
    id: i32,
}

impl IntegerProperty for DelegateIntegerProperty {
    fn get(&self, state: &dyn MainState) -> i32 {
        state.integer_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}
