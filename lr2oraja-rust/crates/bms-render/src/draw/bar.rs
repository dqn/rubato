// Bar scroll state for the song selection screen.
//
// Provides runtime state injected from the game state (BarManager)
// into the skin renderer for bar list rendering.

/// Runtime state for bar scroll rendering.
#[derive(Debug, Clone, Default)]
pub struct BarScrollState {
    /// Which bar index is the center (from skin config BAR_CENTER).
    pub center_bar: usize,
    /// Currently selected bar index within the bar list.
    pub selected_index: usize,
    /// Total number of bars in the list.
    pub total_bars: usize,
    /// Interpolation factor for scroll animation (-1.0 to 1.0).
    pub angle_lerp: f32,
    /// Scroll direction (-1 = up, 0 = idle, 1 = down).
    pub angle: i32,
    /// Per-slot data for all 60 bar positions.
    pub slots: Vec<BarSlotData>,
}

/// Data for a single bar slot in the visible list.
#[derive(Debug, Clone, Default)]
pub struct BarSlotData {
    /// Bar type determines rendering style.
    pub bar_type: BarType,
    /// Clear lamp ID (0-10, maps to SkinBar.lamp indices).
    pub lamp_id: i32,
    /// Trophy ID (0=bronze, 1=silver, 2=gold) for grade bars.
    pub trophy_id: Option<usize>,
    /// Song level value for SkinNumber display.
    pub level: i32,
    /// Difficulty index (0-6) for selecting bar_level variant.
    pub difficulty: i32,
    /// Song title text.
    pub title: String,
    /// Subtitle text (used by Function bars).
    pub subtitle: Option<String>,
    /// Text type index (0-10) for selecting SkinText variant.
    pub text_type: usize,
    /// Feature flags: bit 0 = LN, bit 1 = Mine, bit 2 = Random,
    /// bit 3 = ChargeNote, bit 4 = HellChargeNote.
    pub features: u32,
}

/// Bar type classification matching Java BarRenderer.prepare() ba.value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BarType {
    /// Song bar. `exists` = true if the BMS file is present.
    Song { exists: bool },
    /// Folder bar.
    Folder,
    /// Grade/course bar. `all_songs` = true if all constituent songs are present.
    Grade { all_songs: bool },
    /// Table or hash bar.
    Table,
    /// Command or container bar.
    Command,
    /// Search result bar.
    Search,
    /// Function bar with custom display types.
    Function {
        display_bar_type: i32,
        display_text_type: usize,
    },
}

impl Default for BarType {
    fn default() -> Self {
        BarType::Song { exists: false }
    }
}

// Feature flag constants.
pub const FEATURE_LN: u32 = 1;
pub const FEATURE_MINE: u32 = 2;
pub const FEATURE_RANDOM: u32 = 4;
pub const FEATURE_CHARGENOTE: u32 = 8;
pub const FEATURE_HELL_CHARGENOTE: u32 = 16;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_scroll_state_default() {
        let state = BarScrollState::default();
        assert_eq!(state.center_bar, 0);
        assert_eq!(state.selected_index, 0);
        assert_eq!(state.total_bars, 0);
        assert_eq!(state.angle_lerp, 0.0);
        assert_eq!(state.angle, 0);
        assert!(state.slots.is_empty());
    }

    #[test]
    fn test_bar_slot_data_default() {
        let slot = BarSlotData::default();
        assert_eq!(slot.bar_type, BarType::Song { exists: false });
        assert_eq!(slot.lamp_id, 0);
        assert!(slot.trophy_id.is_none());
        assert_eq!(slot.level, 0);
        assert_eq!(slot.difficulty, 0);
        assert!(slot.title.is_empty());
        assert_eq!(slot.text_type, 0);
        assert_eq!(slot.features, 0);
    }

    #[test]
    fn test_bar_type_variants() {
        assert!(matches!(
            BarType::default(),
            BarType::Song { exists: false }
        ));
        let _folder = BarType::Folder;
        let _grade = BarType::Grade { all_songs: true };
        let _table = BarType::Table;
        let _command = BarType::Command;
        let _search = BarType::Search;
        let _func = BarType::Function {
            display_bar_type: 0,
            display_text_type: 2,
        };
    }

    #[test]
    fn test_feature_flags() {
        let mut slot = BarSlotData::default();
        slot.features = FEATURE_LN | FEATURE_MINE;
        assert_ne!(slot.features & FEATURE_LN, 0);
        assert_ne!(slot.features & FEATURE_MINE, 0);
        assert_eq!(slot.features & FEATURE_RANDOM, 0);
    }
}
