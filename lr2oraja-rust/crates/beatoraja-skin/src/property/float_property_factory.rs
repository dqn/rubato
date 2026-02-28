use super::float_property::FloatProperty;
use super::float_writer::FloatWriter;
use crate::stubs::MainState;

/// Returns a FloatProperty for the given RateType ID.
pub fn get_rate_property_by_id(optionid: i32) -> Option<Box<dyn FloatProperty>> {
    for rt in RATE_TYPES.iter() {
        if rt.id == optionid {
            return Some(Box::new(DelegateFloatProperty { id: rt.id }));
        }
    }
    None
}

/// Returns a FloatProperty for the given RateType name.
pub fn get_rate_property_by_name(name: &str) -> Option<Box<dyn FloatProperty>> {
    for rt in RATE_TYPES.iter() {
        if rt.name == name {
            return Some(Box::new(DelegateFloatProperty { id: rt.id }));
        }
    }
    None
}

/// Returns a FloatWriter for the given RateType ID.
pub fn get_rate_writer_by_id(id: i32) -> Option<Box<dyn FloatWriter>> {
    for rt in RATE_TYPES.iter() {
        if rt.id == id && rt.has_writer {
            return Some(Box::new(DelegateFloatWriter { id: rt.id }));
        }
    }
    None
}

/// Returns a FloatWriter for the given RateType name.
pub fn get_rate_writer_by_name(name: &str) -> Option<Box<dyn FloatWriter>> {
    for rt in RATE_TYPES.iter() {
        if rt.name == name && rt.has_writer {
            return Some(Box::new(DelegateFloatWriter { id: rt.id }));
        }
    }
    None
}

/// Returns a FloatProperty for the given FloatType or RateType ID.
pub fn get_float_property_by_id(optionid: i32) -> Option<Box<dyn FloatProperty>> {
    for ft in FLOAT_TYPES.iter() {
        if ft.id == optionid {
            return Some(Box::new(DelegateFloatProperty { id: ft.id }));
        }
    }
    for rt in RATE_TYPES.iter() {
        if rt.id == optionid {
            return Some(Box::new(DelegateFloatProperty { id: rt.id }));
        }
    }
    None
}

/// Returns a FloatProperty for the given FloatType or RateType name.
pub fn get_float_property_by_name(name: &str) -> Option<Box<dyn FloatProperty>> {
    for ft in FLOAT_TYPES.iter() {
        if ft.name == name {
            return Some(Box::new(DelegateFloatProperty { id: ft.id }));
        }
    }
    for rt in RATE_TYPES.iter() {
        if rt.name == name {
            return Some(Box::new(DelegateFloatProperty { id: rt.id }));
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

/// Delegate FloatProperty that reads values from MainState::float_value().
/// This enables both StaticStateProvider (golden-master) and real game states
/// to provide float values through the same interface.
struct DelegateFloatProperty {
    id: i32,
}

impl FloatProperty for DelegateFloatProperty {
    fn get(&self, state: &dyn MainState) -> f32 {
        state.float_value(self.id)
    }

    fn get_id(&self) -> i32 {
        self.id
    }
}

/// Delegate FloatWriter that writes values via MainState::set_float_value().
struct DelegateFloatWriter {
    id: i32,
}

impl FloatWriter for DelegateFloatWriter {
    fn set(&self, state: &mut dyn MainState, value: f32) {
        state.set_float_value(self.id, value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};

    /// MockMainState that returns configurable float values.
    struct FloatMockState {
        timer: Timer,
        main: MainController,
        resource: PlayerResource,
        /// Maps property ID to float value.
        values: std::collections::HashMap<i32, f32>,
        /// Records set_float_value calls: (id, value).
        set_calls: std::cell::RefCell<Vec<(i32, f32)>>,
    }

    impl FloatMockState {
        fn new(values: std::collections::HashMap<i32, f32>) -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
                values,
                set_calls: std::cell::RefCell::new(Vec::new()),
            }
        }
    }

    impl MainState for FloatMockState {
        fn get_timer(&self) -> &dyn beatoraja_types::timer_access::TimerAccess {
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
        fn float_value(&self, id: i32) -> f32 {
            self.values.get(&id).copied().unwrap_or(0.0)
        }
        fn set_float_value(&mut self, id: i32, value: f32) {
            self.set_calls.borrow_mut().push((id, value));
        }
    }

    #[test]
    fn test_delegate_float_property_reads_from_state() {
        let mut values = std::collections::HashMap::new();
        // lanecover (rate type id=4)
        values.insert(4, 0.75);
        // music_progress (rate type id=6)
        values.insert(6, 0.42);
        let state = FloatMockState::new(values);

        let prop = get_rate_property_by_id(4).expect("lanecover should exist");
        assert!(
            (prop.get(&state) - 0.75).abs() < f32::EPSILON,
            "lanecover should read 0.75 from state"
        );
        assert_eq!(prop.get_id(), 4);

        let prop2 = get_rate_property_by_id(6).expect("music_progress should exist");
        assert!(
            (prop2.get(&state) - 0.42).abs() < f32::EPSILON,
            "music_progress should read 0.42 from state"
        );
    }

    #[test]
    fn test_delegate_float_property_by_name() {
        let mut values = std::collections::HashMap::new();
        values.insert(4, 0.33);
        let state = FloatMockState::new(values);

        let prop = get_rate_property_by_name("lanecover").expect("lanecover by name should exist");
        assert!(
            (prop.get(&state) - 0.33).abs() < f32::EPSILON,
            "lanecover by name should read 0.33"
        );
        assert_eq!(prop.get_id(), 4);
    }

    #[test]
    fn test_delegate_float_property_default_zero() {
        // State returns default 0.0 for unknown IDs
        let state = FloatMockState::new(std::collections::HashMap::new());

        let prop = get_rate_property_by_id(4).expect("lanecover should exist");
        assert!(
            (prop.get(&state)).abs() < f32::EPSILON,
            "default should be 0.0"
        );
    }

    #[test]
    fn test_get_float_property_by_id_covers_float_types() {
        let mut values = std::collections::HashMap::new();
        // score_rate (float type id=1102)
        values.insert(1102, 0.95);
        let state = FloatMockState::new(values);

        let prop = get_float_property_by_id(1102).expect("score_rate should exist");
        assert!(
            (prop.get(&state) - 0.95).abs() < f32::EPSILON,
            "score_rate should read 0.95"
        );
        assert_eq!(prop.get_id(), 1102);
    }

    #[test]
    fn test_get_float_property_by_name_covers_float_types() {
        let mut values = std::collections::HashMap::new();
        values.insert(1102, 0.88);
        let state = FloatMockState::new(values);

        let prop =
            get_float_property_by_name("score_rate").expect("score_rate by name should exist");
        assert!(
            (prop.get(&state) - 0.88).abs() < f32::EPSILON,
            "score_rate by name should read 0.88"
        );
    }

    #[test]
    fn test_get_float_property_by_id_falls_through_to_rate_types() {
        let mut values = std::collections::HashMap::new();
        // lanecover is a rate type (id=4), not a float type
        values.insert(4, 0.5);
        let state = FloatMockState::new(values);

        let prop =
            get_float_property_by_id(4).expect("lanecover should be found via rate types fallback");
        assert!((prop.get(&state) - 0.5).abs() < f32::EPSILON);
    }

    #[test]
    fn test_delegate_float_writer_calls_set_float_value() {
        let mut state = FloatMockState::new(std::collections::HashMap::new());

        // musicselect_position (id=1) has has_writer=true
        let writer = get_rate_writer_by_id(1).expect("musicselect_position writer should exist");
        writer.set(&mut state, 0.65);

        let calls = state.set_calls.borrow();
        assert_eq!(
            calls.len(),
            1,
            "set_float_value should have been called once"
        );
        assert_eq!(calls[0].0, 1, "set_float_value should receive id=1");
        assert!(
            (calls[0].1 - 0.65).abs() < f32::EPSILON,
            "set_float_value should receive value 0.65"
        );
    }

    #[test]
    fn test_delegate_float_writer_by_name() {
        let mut state = FloatMockState::new(std::collections::HashMap::new());

        let writer =
            get_rate_writer_by_name("mastervolume").expect("mastervolume writer should exist");
        writer.set(&mut state, 0.8);

        let calls = state.set_calls.borrow();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].0, 17, "mastervolume id should be 17");
        assert!((calls[0].1 - 0.8).abs() < f32::EPSILON);
    }

    #[test]
    fn test_rate_writer_not_available_for_readonly() {
        // lanecover (id=4) has has_writer=false
        assert!(
            get_rate_writer_by_id(4).is_none(),
            "lanecover should not have a writer"
        );
        assert!(
            get_rate_writer_by_name("lanecover").is_none(),
            "lanecover by name should not have a writer"
        );
    }

    #[test]
    fn test_nonexistent_rate_property() {
        assert!(get_rate_property_by_id(9999).is_none());
        assert!(get_rate_property_by_name("nonexistent").is_none());
    }

    #[test]
    fn test_nonexistent_float_property() {
        assert!(get_float_property_by_id(9999).is_none());
        assert!(get_float_property_by_name("nonexistent").is_none());
    }
}
