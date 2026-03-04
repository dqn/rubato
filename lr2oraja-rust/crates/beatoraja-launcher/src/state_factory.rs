// LauncherStateFactory — concrete StateFactory implementation.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

use std::sync::{Arc, Mutex};

use beatoraja_core::config_pkg::key_configuration::KeyConfiguration;
use beatoraja_core::config_pkg::skin_configuration::SkinConfiguration;
use beatoraja_core::main_controller::{MainController, StateCreateResult, StateFactory};
use beatoraja_core::main_state::{MainState, MainStateData, MainStateType};
use beatoraja_core::timer_manager::TimerManager;
use beatoraja_play::bga::bga_processor::BGAProcessor;
use beatoraja_play::bms_player::BMSPlayer;
use beatoraja_state::decide::music_decide::MusicDecide;
use beatoraja_state::decide::stubs::MainControllerRef as DecideMainControllerRef;
use beatoraja_state::result::course_result::CourseResult;
use beatoraja_state::result::music_result::MusicResult;
use beatoraja_state::result::stubs::MainController as ResultMainController;
use beatoraja_state::result::stubs::PlayerResource as ResultPlayerResource;
use beatoraja_state::select::music_selector::MusicSelector;
use beatoraja_types::main_controller_access::{ConfigMainControllerAccess, MainControllerAccess};
use beatoraja_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};
use beatoraja_types::score_data::ScoreData;
use beatoraja_types::sound_type::SoundType;

/// Wrapper that delegates MainState methods to a shared `Arc<Mutex<MusicSelector>>`.
///
/// Java: StreamController and MusicSelect screen share the same MusicSelector instance.
/// In Rust, both hold an `Arc<Mutex<MusicSelector>>` so stream request bars appear in the
/// select screen's bar list.
///
/// The wrapper owns a local `MainStateData` for the `main_state_data()` / `main_state_data_mut()`
/// trait methods (which return references and cannot go through a Mutex). Lifecycle methods
/// (create, render, etc.) delegate through the Arc<Mutex<>> to the shared selector.
struct SharedMusicSelectorState {
    selector: Arc<Mutex<MusicSelector>>,
    /// Local state data for skin/score property access.
    /// Synced from the shared selector on create() and after render().
    state_data: MainStateData,
}

impl SharedMusicSelectorState {
    fn new(selector: Arc<Mutex<MusicSelector>>) -> Self {
        Self {
            selector,
            state_data: MainStateData::new(TimerManager::new()),
        }
    }

    // Note: MainStateData (score, timer, skin) is NOT synced from the shared selector
    // because ScoreDataProperty and TimerManager don't implement Clone. The local state_data
    // provides stable references for MainController but stays at defaults. The actual game
    // state (bars, songs, selections) lives inside the shared MusicSelector.
}

impl MainState for SharedMusicSelectorState {
    fn state_type(&self) -> Option<MainStateType> {
        Some(MainStateType::MusicSelect)
    }

    fn main_state_data(&self) -> &MainStateData {
        &self.state_data
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        &mut self.state_data
    }

    fn create(&mut self) {
        self.selector.lock().unwrap().create();
    }

    fn prepare(&mut self) {
        self.selector.lock().unwrap().prepare();
    }

    fn shutdown(&mut self) {
        self.selector.lock().unwrap().shutdown();
    }

    fn render(&mut self) {
        self.selector.lock().unwrap().render();
    }

    fn input(&mut self) {
        self.selector.lock().unwrap().input();
    }

    fn pause(&mut self) {
        self.selector.lock().unwrap().pause();
    }

    fn resume(&mut self) {
        self.selector.lock().unwrap().resume();
    }

    fn resize(&mut self, width: i32, height: i32) {
        self.selector.lock().unwrap().resize(width, height);
    }

    fn dispose(&mut self) {
        self.selector.lock().unwrap().dispose();
    }

    fn get_sound(&self, sound: SoundType) -> Option<String> {
        self.selector.lock().unwrap().get_sound(sound)
    }

    fn play_sound_loop(&mut self, sound: SoundType, loop_sound: bool) {
        self.selector
            .lock()
            .unwrap()
            .play_sound_loop(sound, loop_sound);
    }

    fn stop_sound(&mut self, sound: SoundType) {
        self.selector.lock().unwrap().stop_sound(sound);
    }

    fn load_skin(&mut self, skin_type: i32) {
        self.selector.lock().unwrap().load_skin(skin_type);
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.selector.lock().unwrap().take_pending_state_change()
    }
}

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
    /// Compute a target score using the full TargetProperty pipeline.
    ///
    /// Translated from: TargetProperty.getTargetProperty(id).getTarget(main)
    fn compute_target_score(
        targetid: &str,
        total_notes: i32,
        controller: &mut MainController,
    ) -> Option<ScoreData> {
        use beatoraja_play::target_property::TargetProperty;
        let mut target = TargetProperty::get_target_property(targetid)?;
        match target {
            TargetProperty::Static(ref p) => {
                // Static targets can be computed without MainController access.
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
                // Rival, IR, and NextRank targets use the full get_target() pipeline.
                let score = target.get_target(controller);
                Some(score)
            }
        }
    }
}

impl StateFactory for LauncherStateFactory {
    fn create_state(
        &self,
        state_type: MainStateType,
        controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        match state_type {
            MainStateType::MusicSelect => {
                // Java: selector = new MusicSelector(this, songUpdated);
                // If a shared selector exists (created for StreamController), use it
                // so stream request bars appear in the select screen.
                if let Some(shared) = controller.get_shared_music_selector()
                    && let Some(arc) = shared.downcast_ref::<Arc<Mutex<MusicSelector>>>()
                {
                    let wrapper = SharedMusicSelectorState::new(Arc::clone(arc));
                    return Some(StateCreateResult {
                        state: Box::new(wrapper),
                        target_score: None,
                    });
                }
                // Fallback: create a standalone selector (no stream controller)
                let selector = MusicSelector::with_config(controller.get_config().clone());
                Some(StateCreateResult {
                    state: Box::new(selector),
                    target_score: None,
                })
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
                Some(StateCreateResult {
                    state: Box::new(decide),
                    target_score: None,
                })
            }
            MainStateType::Play => {
                // Java: new BMSPlayer(this, resource)
                // Get model from PlayerResource, fall back to default
                let resource = controller.get_player_resource();
                let model = resource
                    .and_then(|r| r.get_bms_model())
                    .cloned()
                    .unwrap_or_default();
                let song_resource_gen = controller.get_config().song_resource_gen;
                let mut player = BMSPlayer::new_with_resource_gen(model.clone(), song_resource_gen);

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

                // Compute target score for both BMSPlayer and PlayerResource (result screen).
                // Java: TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)
                // Java: resource.setTargetScoreData(targetScore)
                let target_score = if rival_score.is_none() || is_course_mode {
                    let targetid = controller.get_player_config().targetid.clone();
                    let total_notes = model.get_total_notes();
                    Self::compute_target_score(&targetid, total_notes, controller)
                } else {
                    rival_score
                };
                player.set_target_score(target_score.clone());

                Some(StateCreateResult {
                    state: Box::new(player),
                    target_score,
                })
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
                Some(StateCreateResult {
                    state: Box::new(result),
                    target_score: None,
                })
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
                Some(StateCreateResult {
                    state: Box::new(course_result),
                    target_score: None,
                })
            }
            MainStateType::Config => {
                // Java: keyconfig = new KeyConfiguration(this);
                let keyconfig = KeyConfiguration::new(controller);
                Some(StateCreateResult {
                    state: Box::new(keyconfig),
                    target_score: None,
                })
            }
            MainStateType::SkinConfig => {
                // Java: skinconfig = new SkinConfiguration(this, player);
                let skinconfig = SkinConfiguration::new(controller, controller.get_player_config());
                Some(StateCreateResult {
                    state: Box::new(skinconfig),
                    target_score: None,
                })
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
        let mut controller = make_test_controller();

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
            let result = factory.create_state(*state_type, &mut controller);
            assert!(
                result.is_some(),
                "Failed to create state for {:?}",
                state_type
            );
            let result = result.unwrap();
            assert_eq!(
                result.state.state_type(),
                Some(*state_type),
                "State type mismatch for {:?}",
                state_type
            );
        }
    }

    #[test]
    fn test_music_select_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::MusicSelect, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::MusicSelect));
    }

    #[test]
    fn test_decide_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::Decide, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::Decide));
    }

    #[test]
    fn test_play_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::Play, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_result_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::Result, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_course_result_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::CourseResult, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::CourseResult));
    }

    #[test]
    fn test_config_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::Config, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::Config));
    }

    #[test]
    fn test_skin_config_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        let result = factory
            .create_state(MainStateType::SkinConfig, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::SkinConfig));
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
