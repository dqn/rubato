use crate::stubs::{
    BooleanPropertyFactory, IntegerPropertyFactory, MainState, NUMBER_CLEAR, OPTION_RESULT_A_1P,
    OPTION_RESULT_AA_1P, OPTION_RESULT_AAA_1P, OPTION_RESULT_B_1P, OPTION_RESULT_C_1P,
    OPTION_RESULT_D_1P, OPTION_RESULT_E_1P, OPTION_RESULT_F_1P,
};

/// ScreenShotExporter interface.
/// Translated from Java: ScreenShotExporter (interface)
pub trait ScreenShotExporter {
    fn send(&self, state: &MainState, pixels: &[u8]) -> bool;
}

/// Returns the clear type name string for the current state.
/// Translated from Java: ScreenShotExporter.getClearTypeName (static default method)
pub(crate) fn clear_type_name(current_state: &MainState) -> String {
    let clear_type_name: [&str; 11] = [
        "NO PLAY",
        "FAILED",
        "ASSIST EASY CLEAR",
        "LIGHT ASSIST EASY CLEAR",
        "EASY CLEAR",
        "CLEAR",
        "HARD CLEAR",
        "EXHARD CLEAR",
        "FULL COMBO",
        "PERFECT",
        "MAX",
    ];

    let clear = IntegerPropertyFactory::integer_property(NUMBER_CLEAR).get(current_state);
    if clear >= 0 && (clear as usize) < clear_type_name.len() {
        return clear_type_name[clear as usize].to_string();
    }

    String::new()
}

/// Returns the clear type colour as an integer for the current state.
/// Translated from Java: ScreenShotExporter.getClearTypeColour (static default method)
pub(crate) fn clear_type_colour(current_state: &MainState) -> i32 {
    let clear_type_rgb: [&str; 11] = [
        "7F7F7F", "8A0000", "9F39CF", "C467D5", "00D70F", "229AFF", "FDFDFD", "FFDB00", "78FFF7",
        "A7F583", "F0F0FF",
    ];

    let clear = IntegerPropertyFactory::integer_property(NUMBER_CLEAR).get(current_state);
    if clear >= 0 && (clear as usize) < clear_type_rgb.len() {
        return i32::from_str_radix(clear_type_rgb[clear as usize], 16).unwrap_or(0);
    }

    0
}

/// Returns the rank type name string for the current state.
/// Translated from Java: ScreenShotExporter.getRankTypeName (static default method)
pub(crate) fn rank_type_name(current_state: &MainState) -> String {
    let mut rank_type_name = String::new();
    if BooleanPropertyFactory::boolean_property(OPTION_RESULT_AAA_1P).get(current_state) {
        rank_type_name += "AAA";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_AA_1P).get(current_state) {
        rank_type_name += "AA";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_A_1P).get(current_state) {
        rank_type_name += "A";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_B_1P).get(current_state) {
        rank_type_name += "B";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_C_1P).get(current_state) {
        rank_type_name += "C";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_D_1P).get(current_state) {
        rank_type_name += "D";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_E_1P).get(current_state) {
        rank_type_name += "E";
    } else if BooleanPropertyFactory::boolean_property(OPTION_RESULT_F_1P).get(current_state) {
        rank_type_name += "F";
    }
    rank_type_name
}
