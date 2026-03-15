use crate::property::boolean_property::BooleanProperty;
use crate::reexports::MainState;

// ============================================================
// Property types with different staticness categories
// ============================================================

/// TYPE_NO_STATIC: never static, always re-evaluated.
/// Delegates to MainState::boolean_value().
pub(super) struct DelegateBooleanProperty {
    pub(super) id: i32,
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
pub(super) struct StaticWithoutMusicSelectProperty {
    pub(super) id: i32,
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
pub(super) struct StaticOnResultProperty {
    pub(super) id: i32,
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
pub(super) struct StaticAllProperty {
    pub(super) id: i32,
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
pub(super) struct NegatedBooleanProperty {
    pub(super) inner: Box<dyn BooleanProperty>,
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
