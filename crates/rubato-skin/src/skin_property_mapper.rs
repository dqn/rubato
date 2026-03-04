use crate::skin_property::*;
use crate::skin_type::SkinType;

/// Returns the bomb animation timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 (scratch + keys) or 10-99 (extended).
/// Returns -1 if out of range.
pub fn bomb_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_BOMB_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_BOMB_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns the long-note hold timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hold_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_HOLD_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_HOLD_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns the HCN (hell charge note) active timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hcn_active_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_HCN_ACTIVE_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_HCN_ACTIVE_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns the HCN (hell charge note) damage timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hcn_damage_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_HCN_DAMAGE_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_HCN_DAMAGE_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns the key-on (key press) timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn key_on_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_KEYON_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_KEYON_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns the key-off (key release) timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn key_off_timer_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return TIMER_KEYOFF_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return TIMER_KEYOFF_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Extracts the player index (0 or 1) from a per-key judge value ID.
///
/// `value_id`: a value in the `VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY99` range.
pub fn get_key_judge_value_player(value_id: i32) -> i32 {
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (value_id - VALUE_JUDGE_1P_SCRATCH) / 10
    } else {
        (value_id - VALUE_JUDGE_1P_KEY10) / 100
    }
}

/// Extracts the key offset from a per-key judge value ID.
///
/// `value_id`: a value in the `VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY99` range.
/// Returns 0-9 for standard keys, 10-99 for extended keys.
pub fn get_key_judge_value_offset(value_id: i32) -> i32 {
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (value_id - VALUE_JUDGE_1P_SCRATCH) % 10
    } else {
        (value_id - VALUE_JUDGE_1P_KEY10) % 100 + 10
    }
}

/// Returns the per-key judge value ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn key_judge_value_id(player: i32, key: i32) -> i32 {
    if player < 2 {
        if key < 10 {
            return VALUE_JUDGE_1P_SCRATCH + key + player * 10;
        } else if key < 100 {
            return VALUE_JUDGE_1P_KEY10 + key - 10 + player * 100;
        }
    }
    -1
}

/// Returns whether the button ID is a skin-select button (7KEY..COURSE_RESULT or 24KEY range).
pub fn is_skin_select_type_id(id: i32) -> bool {
    (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id)
        || (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id)
}

/// Converts a skin-select button ID to the corresponding `SkinType`.
///
/// Returns `None` if the ID is not in a valid skin-select range.
pub fn get_skin_select_type(id: i32) -> Option<SkinType> {
    if (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_7KEY)
    } else if (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_24KEY + 16)
    } else {
        None
    }
}

/// Returns the skin-select button ID for the given `SkinType`.
///
/// IDs 0-15 map to the 7KEY..COURSE_RESULT range; 16+ map to the 24KEY range.
pub fn skin_select_type_id(skin_type: &SkinType) -> i32 {
    if skin_type.id() <= 15 {
        BUTTON_SKINSELECT_7KEY + skin_type.id()
    } else {
        BUTTON_SKINSELECT_24KEY + skin_type.id() - 16
    }
}

/// Returns whether the button ID is a skin-customize button (slots 1-9).
pub fn is_skin_customize_button(id: i32) -> bool {
    (BUTTON_SKIN_CUSTOMIZE1..BUTTON_SKIN_CUSTOMIZE10).contains(&id)
}

/// Returns the 0-based customize slot index from a skin-customize button ID.
pub fn get_skin_customize_index(id: i32) -> i32 {
    id - BUTTON_SKIN_CUSTOMIZE1
}

/// Returns whether the string ID is a skin-customize category label (slots 1-10).
pub fn is_skin_customize_category(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_CATEGORY1..=STRING_SKIN_CUSTOMIZE_CATEGORY10).contains(&id)
}

/// Returns the 0-based index from a skin-customize category string ID.
pub fn get_skin_customize_category_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_CATEGORY1
}

/// Returns whether the string ID is a skin-customize item label (slots 1-10).
pub fn is_skin_customize_item(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_ITEM1..=STRING_SKIN_CUSTOMIZE_ITEM10).contains(&id)
}

/// Returns the 0-based index from a skin-customize item string ID.
pub fn get_skin_customize_item_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_ITEM1
}

/// Returns whether the event ID is in the user-defined custom event range (1000-1999).
pub fn is_custom_event_id(id: i32) -> bool {
    (EVENT_CUSTOM_BEGIN..=EVENT_CUSTOM_END).contains(&id)
}

/// Returns whether the event ID can be triggered by a skin (currently always true).
pub fn is_event_runnable_by_skin(id: i32) -> bool {
    if is_custom_event_id(id) {
        return true;
    }
    true
}

/// Returns whether the timer ID is in the user-defined custom timer range (10000-19999).
pub fn is_custom_timer_id(id: i32) -> bool {
    (TIMER_CUSTOM_BEGIN..=TIMER_CUSTOM_END).contains(&id)
}

/// Returns whether the timer ID can be written by a skin (only custom timers).
pub fn is_timer_writable_by_skin(id: i32) -> bool {
    is_custom_timer_id(id)
}
