// MainState concrete struct bridging external code to skin's property system.

use crate::core::config::Config;
use rubato_types::screen_type::ScreenType;

use rubato_types::abstract_result_access::AbstractResultAccess;

use crate::external::player_resource_adapter::PlayerResource;

/// Legacy MainState wrapper for external code that accesses `state.resource`.
/// Implements MainStateAccess and provides direct field access for compatibility.
pub struct MainState {
    pub resource: PlayerResource,
    pub screen_type: ScreenType,
    /// Abstract result data for result screens (MusicResult / CourseResult).
    /// Populated when the current screen is a result screen; None otherwise.
    pub abstract_result: Option<Box<dyn AbstractResultAccess + Send + Sync>>,
}

impl rubato_types::main_state_access::MainStateAccess for MainState {
    fn screen_type(&self) -> ScreenType {
        self.screen_type
    }

    fn resource(&self) -> Option<&dyn rubato_types::player_resource_access::PlayerResourceAccess> {
        Some(&*self.resource.inner)
    }

    fn config(&self) -> &Config {
        self.resource.config()
    }

    fn abstract_result(&self) -> Option<&dyn AbstractResultAccess> {
        self.abstract_result
            .as_deref()
            .map(|r| r as &dyn AbstractResultAccess)
    }
}

impl Default for MainState {
    fn default() -> Self {
        Self {
            resource: PlayerResource::default(),
            screen_type: ScreenType::Other,
            abstract_result: None,
        }
    }
}

// skin::MainState trait impl — bridges external's concrete MainState
// to skin's property system (resolves type mismatch, not a circular dep)

impl rubato_types::timer_access::TimerAccess for MainState {
    fn now_time(&self) -> i64 {
        0
    }
    fn now_micro_time(&self) -> i64 {
        0
    }
    fn micro_timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
        i64::MIN
    }
    fn timer(&self, _: rubato_types::timer_id::TimerId) -> i64 {
        i64::MIN
    }
    fn now_time_for(&self, _: rubato_types::timer_id::TimerId) -> i64 {
        0
    }
    fn is_timer_on(&self, _: rubato_types::timer_id::TimerId) -> bool {
        false
    }
}

// Known limitation: screenshot/webhook MainState adapter returns default values for skin
// properties. Song metadata and clear type will be missing until full SkinRenderContext
// delegation is wired.
impl rubato_types::skin_render_context::SkinRenderContext for MainState {}

impl rubato_skin::reexports::MainState for MainState {}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::main_state_access::MainStateAccess;

    #[test]
    fn main_state_default_screen_type_is_other() {
        let state = MainState::default();
        assert_eq!(state.screen_type(), ScreenType::Other);
    }

    #[test]
    fn main_state_with_screen_type_returns_correct_type() {
        let state = MainState {
            resource: PlayerResource::default(),
            screen_type: ScreenType::MusicSelector,
            abstract_result: None,
        };
        assert_eq!(state.screen_type(), ScreenType::MusicSelector);
    }

    #[test]
    fn main_state_with_each_screen_type_variant() {
        let variants = vec![
            ScreenType::MusicSelector,
            ScreenType::MusicDecide,
            ScreenType::BMSPlayer,
            ScreenType::MusicResult,
            ScreenType::CourseResult,
            ScreenType::KeyConfiguration,
            ScreenType::Other,
        ];
        for variant in variants {
            let state = MainState {
                resource: PlayerResource::default(),
                screen_type: variant,
                abstract_result: None,
            };
            assert_eq!(state.screen_type(), variant);
        }
    }
}
