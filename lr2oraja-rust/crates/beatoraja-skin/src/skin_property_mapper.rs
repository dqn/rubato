use crate::skin_property::*;
use crate::skin_type::SkinType;

/// SkinPropertyMapper utility functions
///
/// Translated from SkinPropertyMapper.java
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

pub fn get_key_judge_value_player(value_id: i32) -> i32 {
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (value_id - VALUE_JUDGE_1P_SCRATCH) / 10
    } else {
        (value_id - VALUE_JUDGE_1P_KEY10) / 100
    }
}

pub fn get_key_judge_value_offset(value_id: i32) -> i32 {
    if (VALUE_JUDGE_1P_SCRATCH..=VALUE_JUDGE_2P_KEY9).contains(&value_id) {
        (value_id - VALUE_JUDGE_1P_SCRATCH) % 10
    } else {
        (value_id - VALUE_JUDGE_1P_KEY10) % 100 + 10
    }
}

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

pub fn is_skin_select_type_id(id: i32) -> bool {
    (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id)
        || (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id)
}

pub fn get_skin_select_type(id: i32) -> Option<SkinType> {
    if (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_7KEY)
    } else if (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id) {
        SkinType::get_skin_type_by_id(id - BUTTON_SKINSELECT_24KEY + 16)
    } else {
        None
    }
}

pub fn skin_select_type_id(skin_type: &SkinType) -> i32 {
    if skin_type.id() <= 15 {
        BUTTON_SKINSELECT_7KEY + skin_type.id()
    } else {
        BUTTON_SKINSELECT_24KEY + skin_type.id() - 16
    }
}

pub fn is_skin_customize_button(id: i32) -> bool {
    (BUTTON_SKIN_CUSTOMIZE1..BUTTON_SKIN_CUSTOMIZE10).contains(&id)
}

pub fn get_skin_customize_index(id: i32) -> i32 {
    id - BUTTON_SKIN_CUSTOMIZE1
}

pub fn is_skin_customize_category(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_CATEGORY1..=STRING_SKIN_CUSTOMIZE_CATEGORY10).contains(&id)
}

pub fn get_skin_customize_category_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_CATEGORY1
}

pub fn is_skin_customize_item(id: i32) -> bool {
    (STRING_SKIN_CUSTOMIZE_ITEM1..=STRING_SKIN_CUSTOMIZE_ITEM10).contains(&id)
}

pub fn get_skin_customize_item_index(id: i32) -> i32 {
    id - STRING_SKIN_CUSTOMIZE_ITEM1
}

pub fn is_custom_event_id(id: i32) -> bool {
    (EVENT_CUSTOM_BEGIN..=EVENT_CUSTOM_END).contains(&id)
}

pub fn is_event_runnable_by_skin(id: i32) -> bool {
    if is_custom_event_id(id) {
        return true;
    }
    true
}

pub fn is_custom_timer_id(id: i32) -> bool {
    (TIMER_CUSTOM_BEGIN..=TIMER_CUSTOM_END).contains(&id)
}

pub fn is_timer_writable_by_skin(id: i32) -> bool {
    is_custom_timer_id(id)
}
