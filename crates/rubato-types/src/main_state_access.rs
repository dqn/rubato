use crate::abstract_result_access::AbstractResultAccess;
use crate::config::Config;
use crate::course_data::CourseData;
use crate::replay_data::ReplayData;
use crate::screen_type::ScreenType;
use crate::song_data::SongData;

/// Trait interface for MainState access by external listeners.
///
/// Downstream crates use `&dyn MainStateAccess` instead of concrete MainState stubs.
/// Provides the subset of MainState functionality needed by external modules
/// (DiscordListener, ScreenShotExporter, WebhookHandler, etc.).
///
/// Translated from Java: MainState (field access pattern for external observers)
pub trait MainStateAccess {
    /// Get the current screen type
    fn screen_type(&self) -> ScreenType;
    /// Get config reference
    fn config(&self) -> &Config;
    /// Get song data (immutable)
    fn songdata(&self) -> Option<&SongData> {
        None
    }
    /// Get replay data (immutable)
    fn replay_data(&self) -> Option<&ReplayData> {
        None
    }
    /// Get course data (immutable)
    fn course_data(&self) -> Option<&CourseData> {
        None
    }
    /// Get abstract result access (for result screen states).
    /// Java: instanceof AbstractResult cast
    fn abstract_result(&self) -> Option<&dyn AbstractResultAccess> {
        None
    }
}

/// Trait for listeners that observe MainState changes.
///
/// Translated from Java: MainStateListener interface
///
/// **Deprecated**: Use `AppEvent` channel via `MainController::add_event_sender()` instead.
/// This trait is kept for backward compatibility during migration.
#[deprecated(note = "Use AppEvent channel via MainController::add_event_sender() instead")]
pub trait MainStateListener {
    fn update(&mut self, state: &dyn MainStateAccess, status: i32);
}

#[cfg(test)]
#[allow(deprecated)]
mod tests {
    use super::*;
    struct TestState;
    impl MainStateAccess for TestState {
        fn screen_type(&self) -> ScreenType {
            ScreenType::Other
        }
        fn config(&self) -> &Config {
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
        assert_eq!(state.screen_type(), ScreenType::Other);
        assert!(state.songdata().is_none());
        assert!(state.replay_data().is_none());
        assert!(state.course_data().is_none());
    }

    #[test]
    fn test_main_state_listener_trait() {
        let state = TestState;
        let mut listener = TestListener { called: false };
        listener.update(&state, 0);
        assert!(listener.called);
    }
}
