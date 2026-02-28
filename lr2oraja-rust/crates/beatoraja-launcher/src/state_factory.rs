// LauncherStateFactory — concrete StateFactory implementation.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

use beatoraja_core::config_pkg::key_configuration::KeyConfiguration;
use beatoraja_core::config_pkg::skin_configuration::SkinConfiguration;
use beatoraja_core::main_controller::{MainController, StateFactory};
use beatoraja_core::main_state::{MainState, MainStateType};
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_decide::music_decide::MusicDecide;
use beatoraja_decide::stubs::{
    MainControllerRef as DecideMainControllerRef, NullMainController as DecideNullMainController,
};
use beatoraja_play::bms_player::BMSPlayer;
use beatoraja_result::course_result::CourseResult;
use beatoraja_result::music_result::MusicResult;
use beatoraja_result::stubs::PlayerResource as ResultPlayerResource;
use beatoraja_result::stubs::{
    MainController as ResultMainController, NullMainController as ResultNullMainController,
};
use beatoraja_select::music_selector::MusicSelector;
use beatoraja_types::player_resource_access::NullPlayerResource;

/// LauncherStateFactory — creates concrete state instances for all screen types.
///
/// This is the concrete implementation of StateFactory that lives in beatoraja-launcher,
/// which has access to all screen state crates. Core cannot import these directly due
/// to the dependency direction (screen crates depend on core, not vice versa).
///
/// Translated from: MainController.initializeStates() (Java lines 554-571)
/// ```java
/// private void initializeStates() {
///     resource = new PlayerResource(audio, config, player, loudnessAnalyzer);
///     selector = new MusicSelector(this, songUpdated);
///     decide = new MusicDecide(this);
///     result = new MusicResult(this);
///     gresult = new CourseResult(this);
///     keyconfig = new KeyConfiguration(this);
///     skinconfig = new SkinConfiguration(this, player);
/// }
/// ```
pub struct LauncherStateFactory;

impl LauncherStateFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LauncherStateFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl StateFactory for LauncherStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        controller: &MainController,
    ) -> Option<Box<dyn MainState>> {
        match state_type {
            MainStateType::MusicSelect => {
                // Java: selector = new MusicSelector(this, songUpdated);
                let selector = MusicSelector::new();
                Some(Box::new(selector))
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                let decide = MusicDecide::new(
                    DecideMainControllerRef::new(Box::new(DecideNullMainController)),
                    Box::new(NullPlayerResource::new()),
                    TimerManager::new(),
                );
                Some(Box::new(decide))
            }
            MainStateType::Play => {
                // Java: new BMSPlayer(this, resource)
                // BMSPlayer requires a BMSModel; use default for now
                let player = BMSPlayer::new(bms_model::bms_model::BMSModel::default());
                Some(Box::new(player))
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
                let result = MusicResult::new(
                    ResultMainController::new(Box::new(ResultNullMainController)),
                    ResultPlayerResource::default(),
                    TimerManager::new(),
                );
                Some(Box::new(result))
            }
            MainStateType::CourseResult => {
                // Java: gresult = new CourseResult(this);
                let course_result = CourseResult::new(
                    ResultMainController::new(Box::new(ResultNullMainController)),
                    ResultPlayerResource::default(),
                    TimerManager::new(),
                );
                Some(Box::new(course_result))
            }
            MainStateType::Config => {
                // Java: keyconfig = new KeyConfiguration(this);
                let keyconfig = KeyConfiguration::new(controller);
                Some(Box::new(keyconfig))
            }
            MainStateType::SkinConfig => {
                // Java: skinconfig = new SkinConfiguration(this, player);
                let skinconfig = SkinConfiguration::new(controller, controller.get_player_config());
                Some(Box::new(skinconfig))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_core::config::Config;
    use beatoraja_core::player_config::PlayerConfig;

    fn make_test_controller() -> MainController {
        let config = Config::default();
        let player = PlayerConfig::default();
        MainController::new(None, config, player, None, false)
    }

    #[test]
    fn test_create_all_state_types() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let types = [
            MainStateType::MusicSelect,
            MainStateType::Decide,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];

        for state_type in &types {
            let state = factory.create_state(*state_type, &controller);
            assert!(
                state.is_some(),
                "Failed to create state for {:?}",
                state_type
            );
            let state = state.unwrap();
            assert_eq!(
                state.state_type(),
                Some(*state_type),
                "State type mismatch for {:?}",
                state_type
            );
        }
    }

    #[test]
    fn test_music_select_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::MusicSelect, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::MusicSelect));
    }

    #[test]
    fn test_decide_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::Decide, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::Decide));
    }

    #[test]
    fn test_play_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::Play, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_result_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::Result, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_course_result_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::CourseResult, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::CourseResult));
    }

    #[test]
    fn test_config_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::Config, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::Config));
    }

    #[test]
    fn test_skin_config_state() {
        let factory = LauncherStateFactory::new();
        let controller = make_test_controller();

        let state = factory
            .create_state(MainStateType::SkinConfig, &controller)
            .unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::SkinConfig));
    }

    #[test]
    fn test_factory_with_main_controller_dispatch() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(LauncherStateFactory::new()));

        // Test full state dispatch via MainController
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );

        mc.change_state(MainStateType::Decide);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Decide));

        mc.change_state(MainStateType::Play);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));

        mc.change_state(MainStateType::Result);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));

        mc.change_state(MainStateType::CourseResult);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::CourseResult)
        );

        mc.change_state(MainStateType::Config);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));

        mc.change_state(MainStateType::SkinConfig);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::SkinConfig));
    }

    #[test]
    fn test_state_lifecycle_with_factory() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(LauncherStateFactory::new()));

        // Create state, then transition through lifecycle
        mc.change_state(MainStateType::MusicSelect);
        mc.render();
        mc.pause();
        mc.resume();
        mc.resize(1920, 1080);

        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );

        // Transition to different state (old state should be shut down)
        mc.change_state(MainStateType::Config);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));

        // Dispose
        mc.dispose();
        assert!(mc.get_current_state().is_none());
    }
}
