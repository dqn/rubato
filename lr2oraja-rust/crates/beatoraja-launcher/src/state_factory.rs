// LauncherStateFactory — concrete StateFactory implementation.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

use std::sync::{Arc, Mutex};

use beatoraja_core::config_pkg::key_configuration::KeyConfiguration;
use beatoraja_core::config_pkg::skin_configuration::SkinConfiguration;
use beatoraja_core::main_controller::{MainController, StateFactory};
use beatoraja_core::main_state::{MainState, MainStateType};
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_decide::music_decide::MusicDecide;
use beatoraja_decide::stubs::MainControllerRef as DecideMainControllerRef;
use beatoraja_play::bga::bga_processor::BGAProcessor;
use beatoraja_play::bms_player::BMSPlayer;
use beatoraja_result::course_result::CourseResult;
use beatoraja_result::music_result::MusicResult;
use beatoraja_result::stubs::MainController as ResultMainController;
use beatoraja_result::stubs::PlayerResource as ResultPlayerResource;
use beatoraja_select::music_selector::MusicSelector;
use beatoraja_types::main_controller_access::{ConfigMainControllerAccess, MainControllerAccess};
use beatoraja_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};
use beatoraja_types::score_data::ScoreData;

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

impl LauncherStateFactory {
    /// Compute a target score from read-only data using StaticTargetProperty logic.
    ///
    /// This handles the common case where the target is a static rate (e.g., MAX, AAA, A).
    /// For rival/IR targets that need mutable MainController access, returns None
    /// (BMSPlayer::create() will use a zero-score fallback).
    ///
    /// Translated from: TargetProperty.getTargetProperty(id).getTarget(main)
    /// (StaticTargetProperty path only)
    fn compute_static_target_score(targetid: &str, total_notes: i32) -> Option<ScoreData> {
        use beatoraja_play::target_property::TargetProperty;
        // Try to resolve the target property. If it's a static type, compute inline.
        // For non-static types (Rival, IR, NextRank), we cannot compute without &mut MainController.
        let target = TargetProperty::get_target_property(targetid)?;
        match target {
            TargetProperty::Static(p) => {
                let rivalscore = (total_notes as f64 * 2.0 * p.rate as f64 / 100.0).ceil() as i32;
                let score = ScoreData {
                    player: p.name.clone(),
                    epg: rivalscore / 2,
                    egr: rivalscore % 2,
                    ..Default::default()
                };
                Some(score)
            }
            _ => {
                // Rival, IR, and NextRank targets need mutable MainController access.
                // The target score will be zero in ScoreDataProperty, which is acceptable
                // as a fallback. A future enhancement could compute these via a different
                // mechanism (e.g., passing &mut MainController to the factory).
                log::warn!(
                    "Target '{}' requires mutable MainController access; using zero target score",
                    targetid
                );
                None
            }
        }
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
                let selector = MusicSelector::with_config(controller.get_config().clone());
                Some(Box::new(selector))
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                let mc_access = ConfigMainControllerAccess::new(
                    controller.get_config().clone(),
                    controller.get_player_config().clone(),
                );
                let decide = MusicDecide::new(
                    DecideMainControllerRef::new(Box::new(mc_access)),
                    Box::new(NullPlayerResource::new()),
                    TimerManager::new(),
                );
                Some(Box::new(decide))
            }
            MainStateType::Play => {
                // Java: new BMSPlayer(this, resource)
                // Get model from PlayerResource, fall back to default
                let resource = controller.get_player_resource();
                let model = resource
                    .and_then(|r| r.get_bms_model())
                    .cloned()
                    .unwrap_or_default();
                let mut player = BMSPlayer::new(model.clone());

                // Reuse BGAProcessor from PlayerResource to preserve texture cache between plays.
                // Java: bga = resource.getBGAManager() (BMSPlayer.java line 545)
                if let Some(bga_any) = resource.and_then(|r| r.get_bga_any())
                    && let Some(bga_arc) = bga_any.downcast_ref::<Arc<Mutex<BGAProcessor>>>()
                {
                    player.set_bga_processor(Arc::clone(bga_arc));
                }

                // Wire player config
                player.set_player_config(controller.get_player_config().clone());

                // Wire course mode flag
                let is_course_mode = resource.and_then(|r| r.get_course_data()).is_some();
                player.set_course_mode(is_course_mode);

                // --- Target/rival score DB load ---
                // Java: main.getPlayDataAccessor().readScoreData(model, config.getLnmode())
                let lnmode = controller.get_player_config().get_lnmode();
                let sha256 = model.get_sha256();
                let has_ln = model.contains_undefined_long_note();
                let db_score = controller.read_score_data_by_hash(sha256, has_ln, lnmode);
                player.set_db_score(db_score);

                // Java: resource.getRivalScoreData()
                let rival_score = resource.and_then(|r| r.get_rival_score_data()).cloned();
                player.set_rival_score(rival_score.clone());

                // Java: TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)
                // TargetProperty::get_target() requires &mut MainController which we don't have,
                // so we compute a static target from read-only data when rival score is absent
                // or in course mode.
                if rival_score.is_none() || is_course_mode {
                    let targetid = &controller.get_player_config().targetid;
                    let total_notes = model.get_total_notes();
                    let target_score = Self::compute_static_target_score(targetid, total_notes);
                    player.set_target_score(target_score);
                }
                // When rival_score is present and not in course mode, create() will use
                // rival_score as the target (matching Java behavior).
                //
                // TODO: Java also calls resource.setTargetScoreData(targetScore) so the
                // result screen can read it. This requires &mut access to PlayerResource,
                // which the factory doesn't have (only &MainController). The target score
                // should be set on PlayerResource in MainController::change_state() after
                // the factory returns, or the factory trait should take &mut MainController.

                Some(Box::new(player))
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
                let mc_access = ConfigMainControllerAccess::new(
                    controller.get_config().clone(),
                    controller.get_player_config().clone(),
                );
                let result = MusicResult::new(
                    ResultMainController::new(Box::new(mc_access)),
                    ResultPlayerResource::default(),
                    TimerManager::new(),
                );
                Some(Box::new(result))
            }
            MainStateType::CourseResult => {
                // Java: gresult = new CourseResult(this);
                let mc_access = ConfigMainControllerAccess::new(
                    controller.get_config().clone(),
                    controller.get_player_config().clone(),
                );
                let course_result = CourseResult::new(
                    ResultMainController::new(Box::new(mc_access)),
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
