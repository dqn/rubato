use std::path::PathBuf;

use rubato_types::skin_type::SkinType;

pub(super) const OPTION_RANDOM_VALUE: i32 = -1;

/// SkinProperty button constants (mirrors rubato_skin::skin_property).
/// Defined locally to avoid circular dependency on beatoraja-skin.
pub(super) const BUTTON_CHANGE_SKIN: i32 = 190;
pub(super) const BUTTON_SKIN_CUSTOMIZE1: i32 = 220;
pub(super) const BUTTON_SKIN_CUSTOMIZE10: i32 = 229;
pub(super) const BUTTON_SKINSELECT_7KEY: i32 = 170;
pub(super) const BUTTON_SKINSELECT_COURSE_RESULT: i32 = 185;
pub(super) const BUTTON_SKINSELECT_24KEY: i32 = 386;
pub(super) const BUTTON_SKINSELECT_24KEY_BATTLE: i32 = 388;

// Local SkinPropertyMapper helpers (mirrors rubato_skin::skin_property_mapper).
// Defined locally to avoid circular dependency on beatoraja-skin.

pub(super) fn is_skin_customize_button(id: i32) -> bool {
    (BUTTON_SKIN_CUSTOMIZE1..=BUTTON_SKIN_CUSTOMIZE10).contains(&id)
}

pub(super) fn skin_customize_index(id: i32) -> i32 {
    id - BUTTON_SKIN_CUSTOMIZE1
}

pub(super) fn is_skin_select_type_id(id: i32) -> bool {
    (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id)
        || (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id)
}

pub(super) fn skin_select_type(id: i32) -> Option<SkinType> {
    if (BUTTON_SKINSELECT_7KEY..=BUTTON_SKINSELECT_COURSE_RESULT).contains(&id) {
        SkinType::skin_type_by_id(id - BUTTON_SKINSELECT_7KEY)
    } else if (BUTTON_SKINSELECT_24KEY..=BUTTON_SKINSELECT_24KEY_BATTLE).contains(&id) {
        SkinType::skin_type_by_id(id - BUTTON_SKINSELECT_24KEY + 16)
    } else {
        None
    }
}

/// Lightweight skin header for use within beatoraja-core.
///
/// beatoraja-skin's `SkinHeader` cannot be imported here due to circular dependencies.
/// This struct contains the subset of fields needed by `SkinConfiguration`.
#[derive(Clone, Debug, Default)]
pub struct SkinHeaderInfo {
    pub path: Option<PathBuf>,
    pub skin_type: Option<SkinType>,
    pub skin_type_id: i32,
    pub name: Option<String>,
    pub custom_options: Vec<CustomOptionDef>,
    pub custom_files: Vec<CustomFileDef>,
    pub custom_offsets: Vec<CustomOffsetDef>,
}

/// Definition of a custom option from the skin header.
#[derive(Clone, Debug)]
pub struct CustomOptionDef {
    pub name: String,
    pub option: Vec<i32>,
    pub contents: Vec<String>,
    pub def: Option<String>,
}

/// Definition of a custom file from the skin header.
#[derive(Clone, Debug)]
pub struct CustomFileDef {
    pub name: String,
    pub path: String,
    pub def: Option<String>,
}

/// Definition of a custom offset from the skin header.
#[derive(Clone, Debug)]
pub struct CustomOffsetDef {
    pub name: String,
    pub caps: rubato_types::offset_capabilities::OffsetCapabilities,
}

/// UI item for skin configuration (replaces Java inner classes CustomItemBase hierarchy).
#[derive(Clone, Debug)]
pub enum CustomItem {
    Option {
        category_name: String,
        contents: Vec<String>,
        options: Vec<i32>,
        selection: usize,
        display_value: String,
    },
    File {
        category_name: String,
        display_values: Vec<String>,
        actual_values: Vec<String>,
        selection: usize,
        display_value: String,
    },
    Offset {
        category_name: String,
        offset_name: String,
        kind: usize,
        min: i32,
        max: i32,
        value: i32,
    },
}

impl CustomItem {
    pub fn category_name(&self) -> &str {
        match self {
            CustomItem::Option { category_name, .. } => category_name,
            CustomItem::File { category_name, .. } => category_name,
            CustomItem::Offset { category_name, .. } => category_name,
        }
    }

    pub fn display_value(&self) -> String {
        match self {
            CustomItem::Option { display_value, .. } => display_value.clone(),
            CustomItem::File { display_value, .. } => display_value.clone(),
            CustomItem::Offset { value, .. } => value.to_string(),
        }
    }

    pub fn value(&self) -> i32 {
        match self {
            CustomItem::Option { selection, .. } => *selection as i32,
            CustomItem::File { selection, .. } => *selection as i32,
            CustomItem::Offset { value, .. } => *value,
        }
    }

    pub fn min(&self) -> i32 {
        match self {
            CustomItem::Option { .. } => 0,
            CustomItem::File { .. } => 0,
            CustomItem::Offset { min, .. } => *min,
        }
    }

    pub fn max(&self) -> i32 {
        match self {
            CustomItem::Option { contents, .. } => contents.len() as i32 - 1,
            CustomItem::File { actual_values, .. } => actual_values.len() as i32 - 1,
            CustomItem::Offset { max, .. } => *max,
        }
    }
}

/// Helper for deferring persistence actions in set_custom_item_value to avoid borrow conflicts.
pub(super) enum PersistAction {
    Option {
        name: String,
        value: i32,
    },
    File {
        name: String,
        path: String,
    },
    Offset {
        name: String,
        kind: usize,
        value: i32,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn is_skin_customize_button_includes_slot_10() {
        assert!(is_skin_customize_button(BUTTON_SKIN_CUSTOMIZE1)); // 220
        assert!(is_skin_customize_button(BUTTON_SKIN_CUSTOMIZE10)); // 229
        for id in BUTTON_SKIN_CUSTOMIZE1..=BUTTON_SKIN_CUSTOMIZE10 {
            assert!(
                is_skin_customize_button(id),
                "slot ID {id} should be recognized"
            );
        }
        assert!(!is_skin_customize_button(BUTTON_SKIN_CUSTOMIZE1 - 1));
        assert!(!is_skin_customize_button(BUTTON_SKIN_CUSTOMIZE10 + 1));
    }
}
