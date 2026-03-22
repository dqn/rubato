use crate::property::boolean_property::BooleanProperty;
use crate::skin_property::*;

use super::property_types::{
    DelegateBooleanProperty, StaticAllProperty, StaticOnResultProperty,
    StaticWithoutMusicSelectProperty,
};

/// Maps known BooleanType enum IDs to properties with correct staticness.
///
/// Java BooleanType categories:
/// - TYPE_NO_STATIC: autoplay, replay, state, gauge, judge, lanecover, etc.
/// - TYPE_STATIC_WITHOUT_MUSICSELECT: bgaoff/on, song data props, stagefile, banner, etc.
/// - TYPE_STATIC_ON_RESULT: rank conditions, judge exist conditions
/// - TYPE_STATIC_ALL: ir_offline, ir_online
pub(super) fn get_boolean_type_property(id: i32) -> Option<Box<dyn BooleanProperty>> {
    match id {
        // === TYPE_STATIC_WITHOUT_MUSICSELECT ===
        // BGA on/off
        OPTION_BGAOFF | OPTION_BGAON => Some(Box::new(StaticWithoutMusicSelectProperty { id })),
        // Song data boolean properties
        OPTION_NO_TEXT
        | OPTION_TEXT
        | OPTION_NO_LN
        | OPTION_LN
        | OPTION_NO_BGA
        | OPTION_BGA
        | OPTION_NO_RANDOMSEQUENCE
        | OPTION_RANDOMSEQUENCE
        | OPTION_NO_BPMCHANGE
        | OPTION_BPMCHANGE
        | OPTION_BPMSTOP => Some(Box::new(StaticWithoutMusicSelectProperty { id })),
        // Difficulty
        OPTION_DIFFICULTY0 | OPTION_DIFFICULTY1 | OPTION_DIFFICULTY2 | OPTION_DIFFICULTY3
        | OPTION_DIFFICULTY4 | OPTION_DIFFICULTY5 => {
            Some(Box::new(StaticWithoutMusicSelectProperty { id }))
        }
        // Judge difficulty
        OPTION_JUDGE_VERYHARD
        | OPTION_JUDGE_HARD
        | OPTION_JUDGE_NORMAL
        | OPTION_JUDGE_EASY
        | OPTION_JUDGE_VERYEASY => Some(Box::new(StaticWithoutMusicSelectProperty { id })),
        // Chart mode keys
        OPTION_7KEYSONG | OPTION_5KEYSONG | OPTION_14KEYSONG | OPTION_10KEYSONG
        | OPTION_9KEYSONG | OPTION_24KEYSONG | OPTION_24KEYDPSONG => {
            Some(Box::new(StaticWithoutMusicSelectProperty { id }))
        }
        // Stagefile, banner, backbmp
        OPTION_STAGEFILE | OPTION_NO_STAGEFILE | OPTION_BACKBMP | OPTION_NO_BACKBMP
        | OPTION_BANNER | OPTION_NO_BANNER => {
            Some(Box::new(StaticWithoutMusicSelectProperty { id }))
        }
        // Trophy/clear conditions
        OPTION_CLEAR_EASY
        | OPTION_CLEAR_GROOVE
        | OPTION_CLEAR_HARD
        | OPTION_CLEAR_EXHARD
        | OPTION_CLEAR_NORMAL
        | OPTION_CLEAR_MIRROR
        | OPTION_CLEAR_RANDOM
        | OPTION_CLEAR_RRANDOM
        | OPTION_CLEAR_SRANDOM
        | OPTION_CLEAR_SPIRAL
        | OPTION_CLEAR_HRANDOM
        | OPTION_CLEAR_ALLSCR
        | OPTION_CLEAR_EXRANDOM
        | OPTION_CLEAR_EXSRANDOM => Some(Box::new(StaticWithoutMusicSelectProperty { id })),

        // === TYPE_NO_STATIC ===
        // Gauge type
        OPTION_GAUGE_GROOVE | OPTION_GAUGE_HARD | OPTION_GAUGE_EX => {
            Some(Box::new(DelegateBooleanProperty { id }))
        }
        // Autoplay/replay/state
        OPTION_AUTOPLAYON
        | OPTION_AUTOPLAYOFF
        | OPTION_REPLAY_OFF
        | OPTION_REPLAY_PLAYING
        | OPTION_STATE_PRACTICE
        | OPTION_NOW_LOADING
        | OPTION_LOADED => Some(Box::new(DelegateBooleanProperty { id })),
        // Select bar clear conditions
        OPTION_SELECT_BAR_NOT_PLAYED
        | OPTION_SELECT_BAR_FAILED
        | OPTION_SELECT_BAR_ASSIST_EASY_CLEARED
        | OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED
        | OPTION_SELECT_BAR_EASY_CLEARED
        | OPTION_SELECT_BAR_NORMAL_CLEARED
        | OPTION_SELECT_BAR_HARD_CLEARED
        | OPTION_SELECT_BAR_EXHARD_CLEARED
        | OPTION_SELECT_BAR_FULL_COMBO_CLEARED
        | OPTION_SELECT_BAR_PERFECT_CLEARED
        | OPTION_SELECT_BAR_MAX_CLEARED => Some(Box::new(DelegateBooleanProperty { id })),
        // Replay data conditions
        OPTION_REPLAYDATA
        | OPTION_REPLAYDATA2
        | OPTION_REPLAYDATA3
        | OPTION_REPLAYDATA4
        | OPTION_NO_REPLAYDATA
        | OPTION_NO_REPLAYDATA2
        | OPTION_NO_REPLAYDATA3
        | OPTION_NO_REPLAYDATA4
        | OPTION_REPLAYDATA_SAVED
        | OPTION_REPLAYDATA2_SAVED
        | OPTION_REPLAYDATA3_SAVED
        | OPTION_REPLAYDATA4_SAVED
        | OPTION_SELECT_REPLAYDATA
        | OPTION_SELECT_REPLAYDATA2
        | OPTION_SELECT_REPLAYDATA3
        | OPTION_SELECT_REPLAYDATA4 => Some(Box::new(DelegateBooleanProperty { id })),
        // Select panel/bar type
        OPTION_PANEL1 | OPTION_PANEL2 | OPTION_PANEL3 | OPTION_SONGBAR | OPTION_FOLDERBAR
        | OPTION_GRADEBAR => Some(Box::new(DelegateBooleanProperty { id })),
        // Course constraints
        OPTION_GRADEBAR_CLASS
        | OPTION_GRADEBAR_MIRROR
        | OPTION_GRADEBAR_RANDOM
        | OPTION_GRADEBAR_NOSPEED
        | OPTION_GRADEBAR_NOGOOD
        | OPTION_GRADEBAR_NOGREAT
        | OPTION_GRADEBAR_GAUGE_LR2
        | OPTION_GRADEBAR_GAUGE_5KEYS
        | OPTION_GRADEBAR_GAUGE_7KEYS
        | OPTION_GRADEBAR_GAUGE_9KEYS
        | OPTION_GRADEBAR_GAUGE_24KEYS
        | OPTION_GRADEBAR_LN
        | OPTION_GRADEBAR_CN
        | OPTION_GRADEBAR_HCN => Some(Box::new(DelegateBooleanProperty { id })),
        // Judge timing conditions
        OPTION_1P_PERFECT | OPTION_1P_EARLY | OPTION_1P_LATE | OPTION_2P_PERFECT
        | OPTION_2P_EARLY | OPTION_2P_LATE | OPTION_3P_PERFECT | OPTION_3P_EARLY
        | OPTION_3P_LATE => Some(Box::new(DelegateBooleanProperty { id })),
        // Lanecover/lift/hidden
        OPTION_LANECOVER1_CHANGING
        | OPTION_LANECOVER1_ON
        | OPTION_LIFT1_ON
        | OPTION_HIDDEN1_ON
        | OPTION_1P_BORDER_OR_MORE => Some(Box::new(DelegateBooleanProperty { id })),
        // Gauge range (Java: TYPE_STATIC_ON_RESULT -- static on result screens)
        OPTION_1P_0_9 | OPTION_1P_10_19 | OPTION_1P_20_29 | OPTION_1P_30_39 | OPTION_1P_40_49
        | OPTION_1P_50_59 | OPTION_1P_60_69 | OPTION_1P_70_79 | OPTION_1P_80_89
        | OPTION_1P_90_99 | OPTION_1P_100 => Some(Box::new(StaticOnResultProperty { id })),
        // Result update conditions
        OPTION_UPDATE_SCORE
        | OPTION_DRAW_SCORE
        | OPTION_UPDATE_MAXCOMBO
        | OPTION_DRAW_MAXCOMBO
        | OPTION_UPDATE_MISSCOUNT
        | OPTION_DRAW_MISSCOUNT
        | OPTION_UPDATE_SCORERANK
        | OPTION_DRAW_SCORERANK
        | OPTION_UPDATE_TARGET
        | OPTION_DRAW_TARGET => Some(Box::new(DelegateBooleanProperty { id })),
        // Result clear/fail
        OPTION_RESULT_CLEAR | OPTION_RESULT_FAIL => Some(Box::new(DelegateBooleanProperty { id })),
        // Win/lose/draw
        OPTION_1PWIN | OPTION_2PWIN | OPTION_DRAW => Some(Box::new(DelegateBooleanProperty { id })),
        // IR conditions
        OPTION_IR_NOPLAYER | OPTION_IR_FAILED | OPTION_IR_BUSY | OPTION_IR_WAITING => {
            Some(Box::new(DelegateBooleanProperty { id }))
        }
        // Constant
        OPTION_CONSTANT => Some(Box::new(DelegateBooleanProperty { id })),

        // === TYPE_STATIC_ON_RESULT ===
        // Rank conditions (1P current rank, result rank, now rank, best rank)
        OPTION_1P_AAA | OPTION_1P_AA | OPTION_1P_A | OPTION_1P_B | OPTION_1P_C | OPTION_1P_D
        | OPTION_1P_E | OPTION_1P_F => Some(Box::new(StaticOnResultProperty { id })),
        OPTION_RESULT_AAA_1P | OPTION_RESULT_AA_1P | OPTION_RESULT_A_1P | OPTION_RESULT_B_1P
        | OPTION_RESULT_C_1P | OPTION_RESULT_D_1P | OPTION_RESULT_E_1P | OPTION_RESULT_F_1P => {
            Some(Box::new(StaticOnResultProperty { id }))
        }
        OPTION_NOW_AAA_1P | OPTION_NOW_AA_1P | OPTION_NOW_A_1P | OPTION_NOW_B_1P
        | OPTION_NOW_C_1P | OPTION_NOW_D_1P | OPTION_NOW_E_1P | OPTION_NOW_F_1P => {
            Some(Box::new(StaticOnResultProperty { id }))
        }
        OPTION_BEST_AAA_1P | OPTION_BEST_AA_1P | OPTION_BEST_A_1P | OPTION_BEST_B_1P
        | OPTION_BEST_C_1P | OPTION_BEST_D_1P | OPTION_BEST_E_1P | OPTION_BEST_F_1P => {
            Some(Box::new(StaticOnResultProperty { id }))
        }
        // Overall rank conditions
        OPTION_AAA | OPTION_AA | OPTION_A | OPTION_B | OPTION_C | OPTION_D | OPTION_E
        | OPTION_F => Some(Box::new(StaticOnResultProperty { id })),
        // Judge exist conditions
        OPTION_PERFECT_EXIST | OPTION_GREAT_EXIST | OPTION_GOOD_EXIST | OPTION_BAD_EXIST
        | OPTION_POOR_EXIST | OPTION_MISS_EXIST => Some(Box::new(StaticOnResultProperty { id })),

        // === TYPE_STATIC_ALL ===
        OPTION_OFFLINE | OPTION_ONLINE => Some(Box::new(StaticAllProperty { id })),

        _ => None,
    }
}

/// Fallback: properties from getBooleanProperty0 in Java.
/// These reference MusicSelector, CourseData, PlayerResource etc.
/// Delegate to MainState::boolean_value() which is computed by the caller.
pub(super) fn get_boolean_property0(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
    // Map specific IDs to their correct staticness categories
    match optionid {
        // TYPE_STATIC_WITHOUT_MUSICSELECT
        OPTION_TABLE_SONG | OPTION_MODE_COURSE => {
            Some(Box::new(StaticWithoutMusicSelectProperty { id: optionid }))
        }
        // TYPE_NO_STATIC (these reference MusicSelector-specific data)
        OPTION_RANDOMSELECTBAR
        | OPTION_RANDOMCOURSEBAR
        | OPTION_PLAYABLEBAR
        | OPTION_NOT_COMPARE_RIVAL
        | OPTION_COMPARE_RIVAL
        | OPTION_DISABLE_SAVE_SCORE
        | OPTION_ENABLE_SAVE_SCORE
        | OPTION_NO_SAVE_CLEAR => Some(Box::new(DelegateBooleanProperty { id: optionid })),
        _ => None,
    }
}
