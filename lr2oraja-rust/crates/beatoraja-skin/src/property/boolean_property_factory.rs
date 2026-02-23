use super::boolean_property::BooleanProperty;
use crate::stubs::MainState;

const ID_LENGTH: usize = 65536;

/// Factory for creating BooleanProperty instances from option IDs.
pub struct BooleanPropertyFactory;

impl BooleanPropertyFactory {
    /// Returns a BooleanProperty for the given option ID.
    /// Negative IDs produce a negated property.
    pub fn get_boolean_property(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
        get_boolean_property(optionid)
    }
}

/// Returns a BooleanProperty for the given option ID.
/// Negative IDs produce a negated property.
pub fn get_boolean_property(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
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
    // Check BooleanType enum first
    if let Some(prop) = get_boolean_type_property(id) {
        return Some(prop);
    }

    // Course stage properties
    // OPTION_COURSE_STAGE1 .. OPTION_COURSE_STAGE4
    // These reference state.resource which is Phase 7+
    // Fallback to getBooleanProperty0
    if let Some(prop) = get_boolean_property0(id) {
        return Some(prop);
    }

    None
}

fn get_boolean_property0(optionid: i32) -> Option<Box<dyn BooleanProperty>> {
    // All of these reference MusicSelector, CourseData, PlayerResource etc.
    // Delegate to MainState::boolean_value() which is computed by the caller.
    Some(Box::new(DelegateBooleanProperty { id: optionid }))
}

fn get_boolean_type_property(id: i32) -> Option<Box<dyn BooleanProperty>> {
    // The Java BooleanType enum maps IDs to properties.
    // All implementations reference BMSPlayer, MusicSelector, AbstractResult etc.
    // which are Phase 7+ dependencies. Return stub implementations for known IDs.

    // IDs from the BooleanType enum
    let known_ids: &[i32] = &[
        40, 41, 42, 43, // bgaoff, bgaon, gauge_groove, gauge_hard
        44, 45, 46, 47, 48, 49, 50, // autoplay, replay, state options
        51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73,
        74, 75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 160, 161, 162, 163,
        164, // chart mode keys
        1160, 1161, 1046, // 24key, 48key, gauge_ex
    ];

    if known_ids.contains(&id) {
        return Some(Box::new(DelegateBooleanProperty { id }));
    }

    None
}

/// Delegate BooleanProperty that reads values from MainState::boolean_value().
/// The actual computation is performed by the caller (e.g., MusicSelector, BMSPlayer),
/// and the result is exposed via the MainState trait method.
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
    }

    impl BoolMockState {
        fn new(values: std::collections::HashMap<i32, bool>) -> Self {
            Self {
                timer: Timer::default(),
                main: MainController { debug: false },
                resource: PlayerResource,
                values,
            }
        }
    }

    impl MainState for BoolMockState {
        fn get_timer(&self) -> &Timer {
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
    }

    #[test]
    fn test_delegate_boolean_property_reads_from_state() {
        let mut values = std::collections::HashMap::new();
        values.insert(42, true);
        values.insert(50, false);
        let state = BoolMockState::new(values);

        // Known BooleanType IDs
        let prop42 = get_boolean_property(42).expect("id 42 should exist");
        assert!(prop42.get(&state));
        assert_eq!(prop42.get_id(), 42);

        let prop50 = get_boolean_property(50).expect("id 50 should exist");
        assert!(!prop50.get(&state));
    }

    #[test]
    fn test_negated_boolean_property() {
        let mut values = std::collections::HashMap::new();
        values.insert(42, true);
        let state = BoolMockState::new(values);

        // Negative ID → negated property
        let prop = get_boolean_property(-42).expect("negated id -42 should exist");
        // Original is true, negated should be false
        assert!(!prop.get(&state));
        assert_eq!(prop.get_id(), -42);
    }

    #[test]
    fn test_delegate_boolean_property_fallback_id() {
        let state = BoolMockState::new(std::collections::HashMap::new());

        // ID 999 is not in known_ids, falls through to get_boolean_property0
        let prop = get_boolean_property(999).expect("fallback id 999 should exist");
        assert!(!prop.get(&state));
        assert_eq!(prop.get_id(), 999);
    }

    #[test]
    fn test_boolean_property_out_of_range() {
        // ID >= ID_LENGTH should return None
        assert!(get_boolean_property(65536).is_none());
        assert!(get_boolean_property(-65536).is_none());
    }

    #[test]
    fn test_delegate_boolean_is_not_static() {
        let state = BoolMockState::new(std::collections::HashMap::new());
        let prop = get_boolean_property(42).unwrap();
        assert!(!prop.is_static(&state));
    }
}
