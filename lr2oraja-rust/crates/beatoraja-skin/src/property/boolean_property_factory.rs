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
    // which are Phase 7+ dependencies. Return stub implementations.
    Some(Box::new(StubBooleanProperty))
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
        return Some(Box::new(StubBooleanProperty));
    }

    None
}

/// A BooleanProperty that always returns false / non-static.
/// Used as a placeholder for Phase 7+ dependencies.
struct StubBooleanProperty;

impl BooleanProperty for StubBooleanProperty {
    fn is_static(&self, _state: &dyn MainState) -> bool {
        false
    }

    fn get(&self, _state: &dyn MainState) -> bool {
        log::warn!("not yet implemented: BooleanPropertyFactory requires MainState subtypes");
        false
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
}
