use super::boolean_property::BooleanProperty;
use crate::skin_property::*;
use crate::stubs::MainState;

const ID_LENGTH: usize = 65536;

/// Factory for creating BooleanProperty instances from option IDs.
pub struct BooleanPropertyFactory;

impl BooleanPropertyFactory {
    /// Returns a BooleanProperty for the given option ID.
    /// Negative IDs produce a negated property.
    pub fn boolean_property(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
        boolean_property(optionid)
    }
}

/// Returns a BooleanProperty for the given option ID.
/// Negative IDs produce a negated property.
pub fn boolean_property(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
    let id = optionid.unsigned_abs() as usize;
    if id >= ID_LENGTH {
        return None;
    }

    // Due to the complexity of caching with trait objects in Rust,
    // we create properties on each call. The Java version uses static caches,
    // but the property creation is cheap enough.
    let result = get_boolean_property_by_id(id as i32);

    match result {
        Some(prop) => {
            if optionid < 0 {
                // Negate the property
                Some(Box::new(NegatedBooleanProperty { inner: prop }))
            } else {
                Some(prop)
            }
        }
        None => None,
    }
}

fn get_boolean_property_by_id(id: i32) -> Option<Box<dyn BooleanProperty>> {
    // Check BooleanType enum first (known IDs with proper staticness)
    if let Some(prop) = get_boolean_type_property(id) {
        return Some(prop);
    }

    // Course stage properties (OPTION_COURSE_STAGE1 .. OPTION_COURSE_STAGE4, OPTION_COURSE_STAGE_FINAL)
    if (OPTION_COURSE_STAGE1..=OPTION_COURSE_STAGE4).contains(&id) {
        return Some(Box::new(StaticWithoutMusicSelectProperty { id }));
    }
    if id == OPTION_COURSE_STAGE_FINAL {
        return Some(Box::new(StaticWithoutMusicSelectProperty { id }));
    }

    // Fallback to getBooleanProperty0
    if let Some(prop) = get_boolean_property0(id) {
        return Some(prop);
    }

    None
}

/// Fallback: properties from getBooleanProperty0 in Java.
/// These reference MusicSelector, CourseData, PlayerResource etc.
/// Delegate to MainState::boolean_value() which is computed by the caller.
fn get_boolean_property0(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
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
        | OPTION_SELECT_BAR_NOT_PLAYED
        | OPTION_DISABLE_SAVE_SCORE
        | OPTION_ENABLE_SAVE_SCORE
        | OPTION_NO_SAVE_CLEAR => Some(Box::new(DelegateBooleanProperty { id: optionid })),
        // Default: delegate with no-static behavior
        _ => Some(Box::new(DelegateBooleanProperty { id: optionid })),
    }
}

/// Maps known BooleanType enum IDs to properties with correct staticness.
///
/// Java BooleanType categories:
/// - TYPE_NO_STATIC: autoplay, replay, state, gauge, judge, lanecover, etc.
/// - TYPE_STATIC_WITHOUT_MUSICSELECT: bgaoff/on, song data props, stagefile, banner, etc.
/// - TYPE_STATIC_ON_RESULT: rank conditions, judge exist conditions
/// - TYPE_STATIC_ALL: ir_offline, ir_online
fn get_boolean_type_property(id: i32) -> Option<Box<dyn BooleanProperty>> {
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
        // Gauge range
        OPTION_1P_0_9 | OPTION_1P_10_19 | OPTION_1P_20_29 | OPTION_1P_30_39 | OPTION_1P_40_49
        | OPTION_1P_50_59 | OPTION_1P_60_69 | OPTION_1P_70_79 | OPTION_1P_80_89
        | OPTION_1P_90_99 | OPTION_1P_100 => Some(Box::new(DelegateBooleanProperty { id })),
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

// ============================================================
// Property types with different staticness categories
// ============================================================

/// TYPE_NO_STATIC: never static, always re-evaluated.
/// Delegates to MainState::boolean_value().
struct DelegateBooleanProperty {
    id: i32,
}

impl BooleanProperty for DelegateBooleanProperty {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        false
    }

    fn get(&self, state: &dyn MainState) -> bool {
        state.boolean_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

/// TYPE_STATIC_WITHOUT_MUSICSELECT: static when the state is NOT a MusicSelector.
/// These properties depend on resource data that doesn't change once loaded
/// (e.g., BGA status, song metadata, chart mode).
struct StaticWithoutMusicSelectProperty {
    id: i32,
}

impl BooleanProperty for StaticWithoutMusicSelectProperty {
    fn is_static(&self, state: &dyn MainState) -> bool {
        !state.is_music_selector()
    }

    fn get(&self, state: &dyn MainState) -> bool {
        state.boolean_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

/// TYPE_STATIC_ON_RESULT: static when on a result screen (MusicResult or CourseResult).
/// Rank/judge conditions are fixed once the result is shown.
struct StaticOnResultProperty {
    id: i32,
}

impl BooleanProperty for StaticOnResultProperty {
    fn is_static(&self, state: &dyn MainState) -> bool {
        state.is_result_state()
    }

    fn get(&self, state: &dyn MainState) -> bool {
        state.boolean_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

/// TYPE_STATIC_ALL: always static (value never changes after initial evaluation).
struct StaticAllProperty {
    id: i32,
}

impl BooleanProperty for StaticAllProperty {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        true
    }

    fn get(&self, state: &dyn MainState) -> bool {
        state.boolean_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

/// A BooleanProperty that negates another property.
struct NegatedBooleanProperty {
    inner: Box<dyn BooleanProperty>,
}

impl BooleanProperty for NegatedBooleanProperty {
    fn is_static(&self, state: &dyn MainState) -> bool {
        self.inner.is_static(state)
    }

    fn get(&self, state: &dyn MainState) -> bool {
        !self.inner.get(state)
    }

    fn get_id(&self) -> i32 {
        let inner_id = self.inner.get_id();
        if inner_id == i32::MIN {
            i32::MIN
        } else {
            -inner_id
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};

    /// MockMainState that returns configurable boolean values.
    struct BoolMockState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
        /// Maps property ID to boolean value.
        values: std::collections::HashMap<i32, bool>,
        is_music_selector: bool,
        is_result_state: bool,
    }

    impl BoolMockState {
        fn new(values: std::collections::HashMap<i32, bool>) -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
                values,
                is_music_selector: false,
                is_result_state: false,
            }
        }

        fn with_music_selector(mut self) -> Self {
            self.is_music_selector = true;
            self
        }

        fn with_result_state(mut self) -> Self {
            self.is_result_state = true;
            self
        }
    }

    impl MainState for BoolMockState {
        fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
            &self.timer
        }
        fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
            None
        }
        fn get_main(&self) -> &MainController {
            &self.main
        }
        fn get_image(&self, _id: i32) -> Option<TextureRegion> {
            None
        }
        fn get_resource(&self) -> &PlayerResource {
            &self.resource
        }
        fn boolean_value(&self, id: i32) -> bool {
            self.values.get(&id).copied().unwrap_or(false)
        }
        fn is_music_selector(&self) -> bool {
            self.is_music_selector
        }
        fn is_result_state(&self) -> bool {
            self.is_result_state
        }
    }

    #[test]
    fn test_delegate_boolean_property_reads_from_state() {
        let mut values = std::collections::HashMap::new();
        values.insert(OPTION_GAUGE_GROOVE, true);
        values.insert(OPTION_OFFLINE, false);
        let state = BoolMockState::new(values);

        let prop42 = boolean_property(OPTION_GAUGE_GROOVE).expect("gauge_groove should exist");
        assert!(prop42.get(&state));
        assert_eq!(prop42.get_id(), OPTION_GAUGE_GROOVE);

        let prop50 = boolean_property(OPTION_OFFLINE).expect("offline should exist");
        assert!(!prop50.get(&state));
    }

    #[test]
    fn test_negated_boolean_property() {
        let mut values = std::collections::HashMap::new();
        values.insert(OPTION_GAUGE_GROOVE, true);
        let state = BoolMockState::new(values);

        // Negative ID -> negated property
        let prop =
            boolean_property(-OPTION_GAUGE_GROOVE).expect("negated gauge_groove should exist");
        // Original is true, negated should be false
        assert!(!prop.get(&state));
        assert_eq!(prop.get_id(), -OPTION_GAUGE_GROOVE);
    }

    #[test]
    fn test_delegate_boolean_property_fallback_id() {
        let state = BoolMockState::new(std::collections::HashMap::new());

        // ID 999 is not in known_ids, falls through to get_boolean_property0
        let prop = boolean_property(999).expect("fallback id 999 should exist");
        assert!(!prop.get(&state));
        assert_eq!(prop.get_id(), 999);
    }

    #[test]
    fn test_boolean_property_out_of_range() {
        // ID >= ID_LENGTH should return None
        assert!(boolean_property(65536).is_none());
        assert!(boolean_property(-65536).is_none());
    }

    #[test]
    fn test_no_static_gauge_groove() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        // OPTION_GAUGE_GROOVE is TYPE_NO_STATIC
        let prop = boolean_property(OPTION_GAUGE_GROOVE).unwrap();
        assert!(!prop.is_static(&state));
    }

    // === Staticness category tests ===

    #[test]
    fn test_static_without_music_select_in_play_state() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        // Not a MusicSelector, so should be static
        let prop = boolean_property(OPTION_BGAOFF).unwrap();
        assert!(
            prop.is_static(&state),
            "BGAOFF should be static outside MusicSelector"
        );
    }

    #[test]
    fn test_static_without_music_select_in_music_selector() {
        let state = BoolMockState::new(std::collections::HashMap::new()).with_music_selector();
        // IS a MusicSelector, so should NOT be static
        let prop = boolean_property(OPTION_BGAOFF).unwrap();
        assert!(
            !prop.is_static(&state),
            "BGAOFF should not be static in MusicSelector"
        );
    }

    #[test]
    fn test_static_on_result_in_result_state() {
        let state = BoolMockState::new(std::collections::HashMap::new()).with_result_state();
        let prop = boolean_property(OPTION_1P_AAA).unwrap();
        assert!(
            prop.is_static(&state),
            "1P_AAA should be static on result screen"
        );
    }

    #[test]
    fn test_static_on_result_in_play_state() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        let prop = boolean_property(OPTION_1P_AAA).unwrap();
        assert!(
            !prop.is_static(&state),
            "1P_AAA should not be static outside result screen"
        );
    }

    #[test]
    fn test_static_all_always_static() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        let prop = boolean_property(OPTION_OFFLINE).unwrap();
        assert!(prop.is_static(&state), "OFFLINE should always be static");

        let selector_state =
            BoolMockState::new(std::collections::HashMap::new()).with_music_selector();
        assert!(
            prop.is_static(&selector_state),
            "OFFLINE should be static even in MusicSelector"
        );
    }

    #[test]
    fn test_song_data_property_static_without_music_select() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        let prop = boolean_property(OPTION_LN).unwrap();
        assert!(
            prop.is_static(&state),
            "LN should be static outside MusicSelector"
        );
        assert_eq!(prop.get_id(), OPTION_LN);
    }

    #[test]
    fn test_course_stage_property_static_without_music_select() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        let prop = boolean_property(OPTION_COURSE_STAGE1).unwrap();
        assert!(
            prop.is_static(&state),
            "COURSE_STAGE1 should be static outside MusicSelector"
        );

        let selector = BoolMockState::new(std::collections::HashMap::new()).with_music_selector();
        assert!(
            !prop.is_static(&selector),
            "COURSE_STAGE1 should not be static in MusicSelector"
        );
    }

    #[test]
    fn test_negated_preserves_staticness() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        // OPTION_BGAOFF is StaticWithoutMusicSelect
        let prop = boolean_property(-OPTION_BGAOFF).unwrap();
        assert!(
            prop.is_static(&state),
            "Negated BGAOFF should preserve staticness"
        );
        assert_eq!(prop.get_id(), -OPTION_BGAOFF);
    }

    #[test]
    fn test_judge_exist_static_on_result() {
        let state = BoolMockState::new(std::collections::HashMap::new()).with_result_state();
        let prop = boolean_property(OPTION_PERFECT_EXIST).unwrap();
        assert!(
            prop.is_static(&state),
            "PERFECT_EXIST should be static on result"
        );
    }

    #[test]
    fn test_all_boolean_type_ids_return_some() {
        // All IDs from the Java BooleanType enum should return Some
        let boolean_type_ids = [
            OPTION_BGAOFF,
            OPTION_BGAON,
            OPTION_GAUGE_GROOVE,
            OPTION_GAUGE_HARD,
            OPTION_AUTOPLAYON,
            OPTION_AUTOPLAYOFF,
            OPTION_REPLAY_OFF,
            OPTION_REPLAY_PLAYING,
            OPTION_STATE_PRACTICE,
            OPTION_NOW_LOADING,
            OPTION_LOADED,
            OPTION_NO_TEXT,
            OPTION_TEXT,
            OPTION_NO_LN,
            OPTION_LN,
            OPTION_NO_BGA,
            OPTION_BGA,
            OPTION_NO_RANDOMSEQUENCE,
            OPTION_RANDOMSEQUENCE,
            OPTION_NO_BPMCHANGE,
            OPTION_BPMCHANGE,
            OPTION_BPMSTOP,
            OPTION_DIFFICULTY0,
            OPTION_DIFFICULTY1,
            OPTION_DIFFICULTY2,
            OPTION_DIFFICULTY3,
            OPTION_DIFFICULTY4,
            OPTION_DIFFICULTY5,
            OPTION_JUDGE_VERYHARD,
            OPTION_JUDGE_HARD,
            OPTION_JUDGE_NORMAL,
            OPTION_JUDGE_EASY,
            OPTION_JUDGE_VERYEASY,
            OPTION_7KEYSONG,
            OPTION_5KEYSONG,
            OPTION_14KEYSONG,
            OPTION_10KEYSONG,
            OPTION_9KEYSONG,
            OPTION_SELECT_BAR_FAILED,
            OPTION_SELECT_BAR_ASSIST_EASY_CLEARED,
            OPTION_SELECT_BAR_LIGHT_ASSIST_EASY_CLEARED,
            OPTION_SELECT_BAR_EASY_CLEARED,
            OPTION_SELECT_BAR_NORMAL_CLEARED,
            OPTION_SELECT_BAR_HARD_CLEARED,
            OPTION_SELECT_BAR_EXHARD_CLEARED,
            OPTION_SELECT_BAR_FULL_COMBO_CLEARED,
            OPTION_SELECT_BAR_PERFECT_CLEARED,
            OPTION_SELECT_BAR_MAX_CLEARED,
            OPTION_REPLAYDATA,
            OPTION_REPLAYDATA2,
            OPTION_REPLAYDATA3,
            OPTION_REPLAYDATA4,
            OPTION_NO_REPLAYDATA,
            OPTION_NO_REPLAYDATA2,
            OPTION_NO_REPLAYDATA3,
            OPTION_NO_REPLAYDATA4,
            OPTION_REPLAYDATA_SAVED,
            OPTION_REPLAYDATA2_SAVED,
            OPTION_REPLAYDATA3_SAVED,
            OPTION_REPLAYDATA4_SAVED,
            OPTION_SELECT_REPLAYDATA,
            OPTION_SELECT_REPLAYDATA2,
            OPTION_SELECT_REPLAYDATA3,
            OPTION_SELECT_REPLAYDATA4,
            OPTION_PANEL1,
            OPTION_PANEL2,
            OPTION_PANEL3,
            OPTION_SONGBAR,
            OPTION_FOLDERBAR,
            OPTION_GRADEBAR,
            OPTION_GRADEBAR_CLASS,
            OPTION_GRADEBAR_MIRROR,
            OPTION_GRADEBAR_RANDOM,
            OPTION_GRADEBAR_NOSPEED,
            OPTION_GRADEBAR_NOGOOD,
            OPTION_GRADEBAR_NOGREAT,
            OPTION_GRADEBAR_GAUGE_LR2,
            OPTION_GRADEBAR_GAUGE_5KEYS,
            OPTION_GRADEBAR_GAUGE_7KEYS,
            OPTION_GRADEBAR_GAUGE_9KEYS,
            OPTION_GRADEBAR_GAUGE_24KEYS,
            OPTION_GRADEBAR_LN,
            OPTION_GRADEBAR_CN,
            OPTION_GRADEBAR_HCN,
            OPTION_STAGEFILE,
            OPTION_NO_STAGEFILE,
            OPTION_BACKBMP,
            OPTION_NO_BACKBMP,
            OPTION_BANNER,
            OPTION_NO_BANNER,
            OPTION_1P_PERFECT,
            OPTION_1P_EARLY,
            OPTION_1P_LATE,
            OPTION_2P_PERFECT,
            OPTION_2P_EARLY,
            OPTION_2P_LATE,
            OPTION_3P_PERFECT,
            OPTION_3P_EARLY,
            OPTION_3P_LATE,
            OPTION_PERFECT_EXIST,
            OPTION_GREAT_EXIST,
            OPTION_GOOD_EXIST,
            OPTION_BAD_EXIST,
            OPTION_POOR_EXIST,
            OPTION_MISS_EXIST,
            OPTION_LANECOVER1_CHANGING,
            OPTION_LANECOVER1_ON,
            OPTION_LIFT1_ON,
            OPTION_HIDDEN1_ON,
            OPTION_1P_BORDER_OR_MORE,
            OPTION_1P_0_9,
            OPTION_1P_10_19,
            OPTION_1P_20_29,
            OPTION_1P_30_39,
            OPTION_1P_40_49,
            OPTION_1P_50_59,
            OPTION_1P_60_69,
            OPTION_1P_70_79,
            OPTION_1P_80_89,
            OPTION_1P_90_99,
            OPTION_1P_100,
            OPTION_1P_AAA,
            OPTION_1P_AA,
            OPTION_1P_A,
            OPTION_1P_B,
            OPTION_1P_C,
            OPTION_1P_D,
            OPTION_1P_E,
            OPTION_1P_F,
            OPTION_RESULT_AAA_1P,
            OPTION_RESULT_AA_1P,
            OPTION_RESULT_A_1P,
            OPTION_RESULT_B_1P,
            OPTION_RESULT_C_1P,
            OPTION_RESULT_D_1P,
            OPTION_RESULT_E_1P,
            OPTION_RESULT_F_1P,
            OPTION_NOW_AAA_1P,
            OPTION_NOW_AA_1P,
            OPTION_NOW_A_1P,
            OPTION_NOW_B_1P,
            OPTION_NOW_C_1P,
            OPTION_NOW_D_1P,
            OPTION_NOW_E_1P,
            OPTION_NOW_F_1P,
            OPTION_BEST_AAA_1P,
            OPTION_BEST_AA_1P,
            OPTION_BEST_A_1P,
            OPTION_BEST_B_1P,
            OPTION_BEST_C_1P,
            OPTION_BEST_D_1P,
            OPTION_BEST_E_1P,
            OPTION_BEST_F_1P,
            OPTION_AAA,
            OPTION_AA,
            OPTION_A,
            OPTION_B,
            OPTION_C,
            OPTION_D,
            OPTION_E,
            OPTION_F,
            OPTION_UPDATE_SCORE,
            OPTION_DRAW_SCORE,
            OPTION_UPDATE_MAXCOMBO,
            OPTION_DRAW_MAXCOMBO,
            OPTION_UPDATE_MISSCOUNT,
            OPTION_DRAW_MISSCOUNT,
            OPTION_UPDATE_SCORERANK,
            OPTION_DRAW_SCORERANK,
            OPTION_UPDATE_TARGET,
            OPTION_DRAW_TARGET,
            OPTION_RESULT_CLEAR,
            OPTION_RESULT_FAIL,
            OPTION_1PWIN,
            OPTION_2PWIN,
            OPTION_DRAW,
            OPTION_OFFLINE,
            OPTION_ONLINE,
            OPTION_IR_NOPLAYER,
            OPTION_IR_FAILED,
            OPTION_IR_BUSY,
            OPTION_IR_WAITING,
            OPTION_24KEYSONG,
            OPTION_24KEYDPSONG,
            OPTION_GAUGE_EX,
            OPTION_CLEAR_EASY,
            OPTION_CLEAR_GROOVE,
            OPTION_CLEAR_HARD,
            OPTION_CLEAR_EXHARD,
            OPTION_CLEAR_NORMAL,
            OPTION_CLEAR_MIRROR,
            OPTION_CLEAR_RANDOM,
            OPTION_CLEAR_RRANDOM,
            OPTION_CLEAR_SRANDOM,
            OPTION_CLEAR_SPIRAL,
            OPTION_CLEAR_HRANDOM,
            OPTION_CLEAR_ALLSCR,
            OPTION_CLEAR_EXRANDOM,
            OPTION_CLEAR_EXSRANDOM,
            OPTION_CONSTANT,
        ];
        for &id in &boolean_type_ids {
            assert!(
                boolean_property(id).is_some(),
                "BooleanType id {} should return Some",
                id
            );
        }
    }
}
