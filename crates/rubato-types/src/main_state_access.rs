use crate::abstract_result_access::AbstractResultAccess;
use crate::config::Config;
use crate::player_resource_access::PlayerResourceAccess;
use crate::screen_type::ScreenType;

/// Trait interface for MainState access by external listeners.
///
/// Downstream crates use `&dyn MainStateAccess` instead of concrete MainState stubs.
/// Provides the subset of MainState functionality needed by external modules
/// (DiscordListener, ScreenShotExporter, WebhookHandler, etc.).
///
/// Translated from Java: MainState (field access pattern for external observers)
pub trait MainStateAccess {
    /// Get the current screen type
    fn get_screen_type(&self) -> ScreenType;

    /// Get player resource (immutable)
    fn get_resource(&self) -> Option<&dyn PlayerResourceAccess>;

    /// Get config reference
    fn get_config(&self) -> &Config;

    /// Get abstract result access (for result screen states).
    /// Java: instanceof AbstractResult cast
    fn get_abstract_result(&self) -> Option<&dyn AbstractResultAccess> {
        None
    }
}

/// Trait for listeners that observe MainState changes.
///
/// Translated from Java: MainStateListener interface
pub trait MainStateListener {
    fn update(&mut self, state: &dyn MainStateAccess, status: i32);
}

#[cfg(test)]
mod tests {
    use super::*;
    struct TestState;
    impl MainStateAccess for TestState {
        fn get_screen_type(&self) -> ScreenType {
            ScreenType::Other
        }
        fn get_resource(&self) -> Option<&dyn PlayerResourceAccess> {
            None
        }
        fn get_config(&self) -> &Config {
            static CONFIG: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
            CONFIG.get_or_init(Config::default)
        }
    }

    struct TestListener {
        called: bool,
    }
    impl MainStateListener for TestListener {
        fn update(&mut self, _state: &dyn MainStateAccess, _status: i32) {
            self.called = true;
        }
    }

    #[test]
    fn test_main_state_access_trait() {
        let state = TestState;
        assert_eq!(state.get_screen_type(), ScreenType::Other);
        assert!(state.get_resource().is_none());
    }

    #[test]
    fn test_main_state_listener_trait() {
        let state = TestState;
        let mut listener = TestListener { called: false };
        listener.update(&state, 0);
        assert!(listener.called);
    }
}
