use rubato_types::timer_id::TimerId;
use rubato_types::value_id::ValueId;

use crate::skin_property::*;
use crate::types::skin_type::SkinType;

/// Returns the bomb animation timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 (scratch + keys) or 10-99 (extended).
/// Returns -1 if out of range.
pub fn bomb_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_BOMB_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_BOMB_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Returns the long-note hold timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hold_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_HOLD_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_HOLD_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Returns the HCN (hell charge note) active timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hcn_active_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_HCN_ACTIVE_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_HCN_ACTIVE_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Returns the HCN (hell charge note) damage timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn hcn_damage_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_HCN_DAMAGE_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_HCN_DAMAGE_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Returns the key-on (key press) timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn key_on_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYON_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYON_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Returns the key-off (key release) timer ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns -1 if out of range.
pub fn key_off_timer_id(player: i32, key: i32) -> TimerId {
    if player < 2 {
        if key < 10 {
            return TimerId::new(TIMER_KEYOFF_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return TimerId::new(TIMER_KEYOFF_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    TimerId::UNDEFINED
}

/// Extracts the player index (0 or 1) from a per-key judge value ID.
///
/// `value_id`: a value in the `VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY99` range.
pub fn key_judge_value_player(value_id: ValueId) -> i32 {
    let raw = value_id.as_i32();
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (raw - VALUE_JUDGE_1P_SCRATCH.as_i32()) / 10
    } else {
        (raw - VALUE_JUDGE_1P_KEY10.as_i32()) / 100
    }
}

/// Extracts the key offset from a per-key judge value ID.
///
/// `value_id`: a value in the `VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY99` range.
/// Returns 0-9 for standard keys, 10-99 for extended keys.
pub fn key_judge_value_offset(value_id: ValueId) -> i32 {
    let raw = value_id.as_i32();
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (raw - VALUE_JUDGE_1P_SCRATCH.as_i32()) % 10
    } else {
        (raw - VALUE_JUDGE_1P_KEY10.as_i32()) % 100 + 10
    }
}

/// Returns the per-key judge value ID for the given player and key.
///
/// `player`: 0 (1P) or 1 (2P). `key`: 0-9 or 10-99 (extended).
/// Returns ValueId::UNDEFINED if out of range.
pub fn key_judge_value_id(player: i32, key: i32) -> ValueId {
    if player < 2 {
        if key < 10 {
            return ValueId::new(VALUE_JUDGE_1P_SCRATCH.as_i32() + key + player * 10);
        } else if key < 100 {
            return ValueId::new(VALUE_JUDGE_1P_KEY10.as_i32() + key - 10 + player * 100);
        }
    }
    ValueId::UNDEFINED
}

/// Returns whether the button ID is a skin-select button (7KEY..COURSE_RESULT or 24KEY range).
pub fn is_skin_select_type_id(id: i32) -> bool {
    (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id)
        || (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id)
}

/// Converts a skin-select button ID to the corresponding `SkinType`.
///
/// Returns `None` if the ID is not in a valid skin-select range.
pub fn skin_select_type(id: i32) -> Option<SkinType> {
    if (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id) {
        SkinType::skin_type_by_id(id - BUTTON_SKINSELECT_7KEY)
    } else if (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id) {
        SkinType::skin_type_by_id(id - BUTTON_SKINSELECT_24KEY + 16)
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
pub fn skin_customize_index(id: i32) -> i32 {
    id - BUTTON_SKIN_CUSTOMIZE1
}

/// Returns whether the string ID is a skin-customize category label (slots 1-10).
pub fn is_skin_customize_category(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_CATEGORY1..=STRING_SKIN_CUSTOMIZE_CATEGORY10).contains(&id)
}

/// Returns the 0-based index from a skin-customize category string ID.
pub fn skin_customize_category_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_CATEGORY1
}

/// Returns whether the string ID is a skin-customize item label (slots 1-10).
pub fn is_skin_customize_item(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_ITEM1..=STRING_SKIN_CUSTOMIZE_ITEM10).contains(&id)
}

/// Returns the 0-based index from a skin-customize item string ID.
pub fn skin_customize_item_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_ITEM1
}

/// Returns whether the event ID is in the user-defined custom event range (1000-1999).
pub(crate) fn is_custom_event_id(id: i32) -> bool {
    (EVENT_CUSTOM_BEGIN..=EVENT_CUSTOM_END).contains(&id)
}

/// Returns whether the event ID can be triggered by a skin (currently always true).
pub(crate) fn is_event_runnable_by_skin(id: i32) -> bool {
    if is_custom_event_id(id) {
        return true;
    }
    true
}

/// Returns whether the timer ID is in the user-defined custom timer range (10000-19999).
pub(crate) fn is_custom_timer_id(id: TimerId) -> bool {
    (TIMER_CUSTOM_BEGIN..=TIMER_CUSTOM_END).contains(&id)
}

/// Returns whether the timer ID can be written by a skin (only custom timers).
pub(crate) fn is_timer_writable_by_skin(id: TimerId) -> bool {
    is_custom_timer_id(id)
}
