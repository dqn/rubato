// LauncherStateFactory -- concrete StateFactory implementation.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

mod queued_access;
mod shared_selector;

use std::sync::{Arc, Mutex};

use rubato_core::config_pkg::key_configuration::KeyConfiguration;
use rubato_core::config_pkg::skin_configuration::SkinConfiguration;
use rubato_core::main_controller::{MainController, StateCreateResult, StateFactory};
use rubato_core::main_state::{MainState, MainStateType};
use rubato_core::timer_manager::TimerManager;
use rubato_play::bga::bga_processor::BGAProcessor;
use rubato_play::bms_player::BMSPlayer;
use rubato_state::decide::main_controller_ref::MainControllerRef as DecideMainControllerRef;
use rubato_state::decide::music_decide::MusicDecide;
use rubato_state::result::BMSPlayerMode;
use rubato_state::result::BMSPlayerModeType;
use rubato_state::result::MainController as ResultMainController;
use rubato_state::result::PlayerResource as ResultPlayerResource;
use rubato_state::result::RankingData;
use rubato_state::result::course_result::CourseResult;
use rubato_state::result::music_result::MusicResult;
use rubato_state::select::music_selector::MusicSelector;
use rubato_types::main_controller_access::MainControllerAccess as _;
use rubato_types::player_resource_access::MediaAccess as _;
use rubato_types::score_data::ScoreData;

pub use queued_access::new_state_main_controller_access;
use queued_access::{QueuedAudioDriver, QueuedControllerAccess};
use shared_selector::SharedMusicSelectorState;

/// Extract result-crate IR statuses from core MainController's type-erased IR statuses.
///
/// Core IRStatus stores connection as `Box<dyn Any>` and player_data as `Box<dyn Any>`.
/// This downcasts them back to their concrete types to build result-crate IRStatus instances.
fn extract_ir_statuses(
    controller: &MainController,
) -> Vec<rubato_state::result::ir_status::IRStatus> {
    controller
        .ir_status()
        .iter()
        .filter_map(|core_ir| {
            let connection = core_ir
                .connection
                .as_ref()?
                .downcast_ref::<Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync>>()?
                .clone();
            let player = core_ir
                .player_data
                .as_ref()?
                .downcast_ref::<rubato_ir::ir_player_data::IRPlayerData>()?
                .clone();
            Some(rubato_state::result::ir_status::IRStatus::new(
                core_ir.config.clone(),
                connection,
                player,
            ))
        })
        .collect()
}

/// LauncherStateFactory -- creates concrete state instances for all screen types.
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
        use rubato_play::target_property::TargetProperty;
        let mut target = TargetProperty::from_id(targetid)?;
        match target {
            TargetProperty::Static(ref p) => {
                // Static targets can be computed without MainController access.
                let rivalscore = (total_notes as f64 * 2.0 * p.rate as f64 / 100.0).ceil() as i32;
                let mut score = ScoreData {
                    player: p.name.clone(),
                    ..Default::default()
                };
                score.judge_counts.epg = rivalscore / 2;
                score.judge_counts.egr = rivalscore % 2;
                Some(score)
            }
            _ => {
                // Rival, IR, and NextRank targets use the full target() pipeline.
                let score = target.target(controller);
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
                if let Some(shared) = controller.shared_music_selector()
                    && let Some(arc) = shared.downcast_ref::<Arc<Mutex<MusicSelector>>>()
                {
                    let wrapper = SharedMusicSelectorState::new(Arc::clone(arc));
                    return Some(StateCreateResult {
                        state: Box::new(wrapper),
                        target_score: None,
                    });
                }
                // Fallback: create a standalone selector (no stream controller).
                // Open a separate SQLite connection for the selector (same pattern
                // as download processors in main.rs).
                let config = controller.config().clone();
                let mut selector = match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
                    &config.paths.songpath,
                    &config.paths.bmsroot,
                ) {
                    Ok(db) => MusicSelector::with_song_database(Box::new(db)),
                    Err(e) => {
                        log::warn!("Failed to open song database for MusicSelector: {}", e);
                        MusicSelector::with_config(config.clone())
                    }
                };
                // Wire dependencies (same pattern as Decide/Result)
                let command_queue = controller.controller_command_queue();
                let mc_access = QueuedControllerAccess::from_controller(controller, command_queue);
                selector.set_main_controller(Box::new(mc_access));
                selector.config = controller.player_config().clone();
                selector.app_config = config;
                Some(StateCreateResult {
                    state: Box::new(selector),
                    target_score: None,
                })
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                match controller.take_player_resource() {
                    Some(resource) => {
                        let command_queue = controller.controller_command_queue();
                        let mc_access = QueuedControllerAccess::from_controller(
                            controller,
                            command_queue.clone(),
                        );
                        let decide = MusicDecide::new(
                            DecideMainControllerRef::with_audio(
                                Box::new(mc_access),
                                Box::new(QueuedAudioDriver::new(command_queue)),
                            ),
                            Box::new(resource),
                            TimerManager::new(),
                        );
                        Some(StateCreateResult {
                            state: Box::new(decide),
                            target_score: None,
                        })
                    }
                    None => {
                        log::error!(
                            "Cannot enter Decide without PlayerResource; falling back to MusicSelect"
                        );
                        self.create_state(MainStateType::MusicSelect, controller)
                    }
                }
            }
            MainStateType::Play => {
                // Java: new BMSPlayer(this, resource)
                // Get model from PlayerResource, fall back to default
                let resource = controller.player_resource();
                let model = resource
                    .and_then(|r| r.bms_model())
                    .cloned()
                    .unwrap_or_default();
                let song_resource_gen = controller.config().render.song_resource_gen;
                let mut player = BMSPlayer::new_with_resource_gen(model.clone(), song_resource_gen);

                // Reuse BGAProcessor from PlayerResource to preserve texture cache between plays.
                // Java: bga = resource.getBGAManager() (BMSPlayer.java line 545)
                if let Some(bga_any) = resource.and_then(|r| r.bga_any())
                    && let Some(bga_arc) = bga_any.downcast_ref::<Arc<Mutex<BGAProcessor>>>()
                {
                    player.set_bga_processor(Arc::clone(bga_arc));
                }

                // Wire player config
                player.set_player_config(controller.player_config().clone());

                // Wire global config for skin property queries (BGA mode, etc.)
                player.set_config(controller.config().clone());

                // Wire course mode flag
                let is_course_mode = resource.and_then(|r| r.course_data()).is_some();
                player.set_course_mode(is_course_mode);

                // Wire play mode from PlayerResource
                if let Some(mode) = resource.and_then(|r| r.play_mode()).copied() {
                    player.set_play_mode(mode);
                }

                // Wire chart option (chart replication / rival replay)
                if let Some(chart_opt) = resource.and_then(|r| r.chart_option()).cloned() {
                    player.set_chart_option(Some(chart_opt));
                }

                // Wire margin time
                if let Some(res) = resource {
                    player.set_margin_time(res.margin_time());
                }

                // Wire original mode for SkinGauge mode-change border alignment.
                // Java: SkinGauge.prepare() checks resource.getOriginalMode() != model.getMode()
                if let Some(res) = resource {
                    player.set_orgmode(res.original_mode());
                }

                // Wire lnmode override from chart data for image_index_value ID 308.
                // Java: IntegerPropertyFactory ID 308 checks SongData LN types on BMSPlayer.
                if let Some(songdata) = resource.and_then(|r| r.songdata()) {
                    player.set_lnmode_override(
                        rubato_types::skin_render_context::compute_lnmode_from_chart(
                            &songdata.chart,
                        ),
                    );
                    // Wire song metadata for skin string property queries (title, artist, genre).
                    // Java: StringPropertyFactory reads resource.getSongdata().getTitle() etc.
                    player.set_song_metadata(songdata.metadata.clone());
                    // Wire song data for boolean skin property queries (chart mode, LN, BGA, etc.).
                    // Java: SongDataBooleanProperty accesses state.resource.getSongdata().
                    player.set_song_data(songdata.clone());
                }

                // Wire course constraints
                if let Some(res) = resource {
                    player.set_constraints(res.constraint());
                }

                // Wire initial course combo and previous gauge values from PlayerResource
                // for course mode.
                // Java: judge.init() checks resource.getGauge() != null, then sets
                // coursecombo/coursemaxcombo from resource. The gauge is non-null on
                // subsequent course stages (after the first play stores gauge data).
                // Also restore per-gauge-type values from the previous stage's gauge log.
                if let Some(res) = resource
                    && let Some(gauge_log) = res.gauge()
                {
                    player.set_initial_course_combo(res.combo, res.maxcombo);
                    player.set_previous_gauge_values(gauge_log.clone());
                }

                // Wire guide SE from player config
                player.set_guide_se(controller.player_config().display_settings.is_guide_se);

                // Wire skin offset snapshot from MainController.
                // Java: MainState inherits MainController.offset[] which skin objects
                // read via getOffset(id) during prepare().
                {
                    let offset_count = rubato_core::main_controller::OFFSET_COUNT;
                    let mut offsets = Vec::with_capacity(offset_count);
                    for i in 0..offset_count {
                        offsets.push(controller.offset(i as i32).copied().unwrap_or_default());
                    }
                    player.set_offset_snapshot(offsets);
                }

                // Wire audio config
                if let Some(audio_config) = controller.config().audio_config() {
                    player.set_fast_forward_freq_option(audio_config.fast_forward);
                    player.set_bg_volume(audio_config.bgvolume);
                    player.set_system_volume(audio_config.systemvolume);
                    player.set_key_volume(audio_config.keyvolume);
                }

                // Wire replay data for REPLAY mode
                if let Some(replay) = resource.and_then(|r| r.replay_data()).cloned() {
                    player.set_active_replay(Some(replay));
                }

                // --- Target/rival score DB load ---
                // Java: main.getPlayDataAccessor().readScoreData(model, config.getLnmode())
                let lnmode = controller.player_config().play_settings.lnmode;
                let sha256 = &model.sha256;
                let has_ln = model.contains_undefined_long_note();
                let db_score = controller.read_score_data_by_hash(sha256, has_ln, lnmode);
                player.set_db_score(db_score);

                // Java: resource.getRivalScoreData()
                let rival_score = resource.and_then(|r| r.rival_score_data()).cloned();
                player.set_rival_score(rival_score.clone());

                // Compute target score for both BMSPlayer and PlayerResource (result screen).
                // Java: TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)
                // Java: resource.setTargetScoreData(targetScore)
                let target_score = if rival_score.is_none() || is_course_mode {
                    let targetid = controller.player_config().select_settings.targetid.clone();
                    let total_notes = model.total_notes();
                    Self::compute_target_score(&targetid, total_notes, controller)
                } else {
                    rival_score
                };
                player.set_target_score(target_score.clone());

                if let Some(skin_type) = player.skin_type() {
                    log::info!(
                        "Play skin loading: type={:?} id={}",
                        skin_type,
                        skin_type.id()
                    );
                    if let Some(skin) = rubato_skin::skin_loader::load_skin_from_config(
                        controller.config(),
                        controller.player_config(),
                        skin_type.id(),
                    ) {
                        log::info!("Play skin loaded: {} objects", skin.objects().len());
                        player.set_skin_name(skin.header.name().map(str::to_string));
                        player.main_state_data_mut().skin = Some(Box::new(skin));
                    } else {
                        log::warn!(
                            "Play skin failed to load for type {:?} (id={})",
                            skin_type,
                            skin_type.id()
                        );
                    }
                } else {
                    log::warn!("Play skin_type() returned None");
                }

                Some(StateCreateResult {
                    state: Box::new(player),
                    target_score,
                })
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
                let ir_statuses = extract_ir_statuses(controller);
                let command_queue = controller.controller_command_queue();
                let mc_access =
                    QueuedControllerAccess::from_controller(controller, command_queue.clone());
                let result_resource = if let Some(core_res) = controller.take_player_resource() {
                    let pm = core_res
                        .play_mode()
                        .cloned()
                        .unwrap_or_else(|| BMSPlayerMode::new(BMSPlayerModeType::Play));
                    let bm = core_res.bms_model().cloned().unwrap_or_default();
                    let cm = core_res.course_bms_models().cloned();
                    let ranking = core_res
                        .ranking_data_any()
                        .and_then(|a| a.downcast_ref::<RankingData>())
                        .cloned();
                    let mut rr = ResultPlayerResource::new(Box::new(core_res), pm);
                    rr.bms_model = bm;
                    rr.course_bms_models = cm;
                    rr.ranking_data = ranking;
                    rr
                } else {
                    ResultPlayerResource::default()
                };
                let result = MusicResult::new(
                    ResultMainController::with_audio_and_ir(
                        Box::new(mc_access),
                        Box::new(QueuedAudioDriver::new(command_queue)),
                        ir_statuses,
                    ),
                    result_resource,
                    TimerManager::new(),
                );
                Some(StateCreateResult {
                    state: Box::new(result),
                    target_score: None,
                })
            }
            MainStateType::CourseResult => {
                // Java: gresult = new CourseResult(this);
                let ir_statuses = extract_ir_statuses(controller);
                let command_queue = controller.controller_command_queue();
                let mc_access =
                    QueuedControllerAccess::from_controller(controller, command_queue.clone());
                let result_resource = if let Some(core_res) = controller.take_player_resource() {
                    let pm = core_res
                        .play_mode()
                        .cloned()
                        .unwrap_or_else(|| BMSPlayerMode::new(BMSPlayerModeType::Play));
                    let bm = core_res.bms_model().cloned().unwrap_or_default();
                    let cm = core_res.course_bms_models().cloned();
                    let ranking = core_res
                        .ranking_data_any()
                        .and_then(|a| a.downcast_ref::<RankingData>())
                        .cloned();
                    let mut rr = ResultPlayerResource::new(Box::new(core_res), pm);
                    rr.bms_model = bm;
                    rr.course_bms_models = cm;
                    rr.ranking_data = ranking;
                    rr
                } else {
                    ResultPlayerResource::default()
                };
                let course_result = CourseResult::new(
                    ResultMainController::with_audio_and_ir(
                        Box::new(mc_access),
                        Box::new(QueuedAudioDriver::new(command_queue)),
                        ir_statuses,
                    ),
                    result_resource,
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
                let skinconfig = SkinConfiguration::new(controller, controller.player_config());
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
    use rubato_core::score_database_accessor::ScoreDatabaseAccessor;
    use rubato_core::sprite_batch_helper::SpriteBatchHelper;
    use rubato_song::song_information_accessor::SongInformationAccessor;
    use rubato_state::select::preview_music_processor::PreviewMusicProcessor;
    use rubato_types::main_controller_access::MainControllerAccess;
    use rubato_types::skin_config::SkinConfig;
    use rubato_types::skin_render_context::SkinRenderContext;
    use rubato_types::skin_type::SkinType;
    use rubato_types::song_data::SongData;
    use rubato_types::song_information::SongInformation;
    use rubato_types::sound_type::SoundType;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct MockSkin;

    impl rubato_core::main_state::SkinDrawable for MockSkin {
        fn draw_all_objects_timed(&mut self, _ctx: &mut dyn SkinRenderContext) {}
        fn update_custom_objects_timed(&mut self, _ctx: &mut dyn SkinRenderContext) {}
        fn mouse_pressed_at(
            &mut self,
            _ctx: &mut dyn SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn mouse_dragged_at(
            &mut self,
            _ctx: &mut dyn SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
        }
        fn prepare_skin(&mut self) {}
        fn dispose_skin(&mut self) {}
        fn fadeout(&self) -> i32 {
            0
        }
        fn input(&self) -> i32 {
            0
        }
        fn scene(&self) -> i32 {
            0
        }
        fn get_width(&self) -> f32 {
            0.0
        }
        fn get_height(&self) -> f32 {
            0.0
        }
        fn swap_sprite_batch(
            &mut self,
            _batch: &mut rubato_core::sprite_batch_helper::SpriteBatch,
        ) {
        }
    }

    use rubato_audio::recording_audio_driver::RecordingAudioDriver;

    struct ChangeStateSkin;

    impl rubato_core::main_state::SkinDrawable for ChangeStateSkin {
        fn draw_all_objects_timed(&mut self, _ctx: &mut dyn SkinRenderContext) {}

        fn update_custom_objects_timed(&mut self, _ctx: &mut dyn SkinRenderContext) {}

        fn mouse_pressed_at(
            &mut self,
            ctx: &mut dyn SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            ctx.change_state(MainStateType::Config);
        }

        fn mouse_dragged_at(
            &mut self,
            ctx: &mut dyn SkinRenderContext,
            _button: i32,
            _x: i32,
            _y: i32,
        ) {
            ctx.change_state(MainStateType::SkinConfig);
        }

        fn prepare_skin(&mut self) {}
        fn dispose_skin(&mut self) {}
        fn fadeout(&self) -> i32 {
            0
        }
        fn input(&self) -> i32 {
            0
        }
        fn scene(&self) -> i32 {
            0
        }
        fn get_width(&self) -> f32 {
            0.0
        }
        fn get_height(&self) -> f32 {
            0.0
        }
        fn swap_sprite_batch(
            &mut self,
            _batch: &mut rubato_core::sprite_batch_helper::SpriteBatch,
        ) {
        }
    }
    use rubato_core::config::Config;
    use rubato_core::player_config::PlayerConfig;

    fn make_test_controller() -> MainController {
        let config = Config::default();
        let player = PlayerConfig::default();
        MainController::new(None, config, player, None, false)
    }

    fn write_song_info_row(path: &std::path::Path, info: &SongInformation) {
        let conn = rusqlite::Connection::open(path).expect("song info db should open");
        conn.execute(
            "INSERT INTO information (
                sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm,
                distribution, speedchange, lanenotes
            ) VALUES (?1, 0, 0, 0, 0, 0.0, 0.0, 0.0, 0.0, ?2, '', '', '')",
            rusqlite::params![info.sha256, info.mainbpm],
        )
        .expect("song info row should insert");
    }

    #[test]
    fn test_create_all_state_types() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        // Decide requires a PlayerResource; without one it falls back to MusicSelect
        let types_without_decide = [
            MainStateType::MusicSelect,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];

        for state_type in &types_without_decide {
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
    fn test_decide_state_falls_back_without_resource() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        // Without PlayerResource, Decide should fall back to MusicSelect
        let result = factory
            .create_state(MainStateType::Decide, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::MusicSelect));
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
        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        // Decide without PlayerResource falls back to MusicSelect
        mc.change_state(MainStateType::Decide);
        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        mc.change_state(MainStateType::Play);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

        mc.change_state(MainStateType::Result);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Result));

        mc.change_state(MainStateType::CourseResult);
        assert_eq!(mc.current_state_type(), Some(MainStateType::CourseResult));

        mc.change_state(MainStateType::Config);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Config));

        mc.change_state(MainStateType::SkinConfig);
        assert_eq!(mc.current_state_type(), Some(MainStateType::SkinConfig));
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

        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        // Transition to different state (old state should be shut down)
        mc.change_state(MainStateType::Config);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Config));

        // Dispose
        mc.dispose();
        assert!(mc.current_state().is_none());
    }

    #[test]
    fn queued_controller_access_enqueues_side_effect_commands() {
        let mut controller = make_test_controller();
        let queue = controller.controller_command_queue();
        let mut access = QueuedControllerAccess::from_controller(&mut controller, queue.clone());

        access.change_state(MainStateType::Play);
        access.play_sound(&SoundType::Decide, false);
        access.stop_sound(&SoundType::ResultClose);

        let commands = queue.drain();
        assert!(matches!(
            commands.first(),
            Some(
                rubato_types::main_controller_access::MainControllerCommand::ChangeState(
                    MainStateType::Play
                )
            )
        ));
        assert!(matches!(
            commands.get(1),
            Some(
                rubato_types::main_controller_access::MainControllerCommand::PlaySound(
                    SoundType::Decide,
                    false
                )
            )
        ));
        assert!(matches!(
            commands.get(2),
            Some(
                rubato_types::main_controller_access::MainControllerCommand::StopSound(
                    SoundType::ResultClose
                )
            )
        ));
    }

    #[test]
    fn queued_controller_access_exposes_song_info_database() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let info_db_path = tempdir.path().join("songinfo.db");
        let info = SongInformation {
            sha256: "q".repeat(64),
            mainbpm: 150.0,
            ..Default::default()
        };

        let mut config = Config::default();
        config.paths.songinfopath = info_db_path.to_string_lossy().to_string();
        let player = PlayerConfig::default();
        let mut controller = MainController::new(None, config, player, None, false);
        controller.set_info_database(Box::new(
            SongInformationAccessor::new(
                info_db_path
                    .to_str()
                    .expect("song info db path should be valid UTF-8"),
            )
            .expect("song info db should open"),
        ));
        write_song_info_row(&info_db_path, &info);

        let queue = controller.controller_command_queue();
        let access = QueuedControllerAccess::from_controller(&mut controller, queue);

        assert_eq!(
            access
                .info_database()
                .and_then(|db| db.information(&info.sha256))
                .map(|row| row.mainbpm as i32),
            Some(150),
            "queued access should preserve the song information database for select loading"
        );
    }

    #[test]
    fn standalone_music_select_create_loads_runtime_score_and_info() {
        let tempdir = tempfile::tempdir().expect("tempdir should be created");
        let song_db_path = tempdir.path().join("songdata.db");
        let info_db_path = tempdir.path().join("songinfo.db");
        let player_root = tempdir.path().join("player");
        let player_dir = player_root.join("player1");
        std::fs::create_dir_all(&player_dir).expect("player directory should be created");

        let mut song = SongData::default();
        song.metadata.title = "standalone-select".to_string();
        song.chart.mode = 7;
        song.chart.maxbpm = 180;
        song.chart.minbpm = 90;
        song.chart.level = 12;
        song.file.sha256 = "s".repeat(64);
        song.file.set_path(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("..")
                .join("..")
                .join("test-bms")
                .join("minimal_7k.bms")
                .to_string_lossy()
                .to_string(),
        );
        song.parent = "e2977170".to_string();

        let song_db = rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &song_db_path.to_string_lossy(),
            &[],
        )
        .expect("song db should open");
        rubato_types::song_database_accessor::SongDatabaseAccessor::set_song_datas(
            &song_db,
            &[song.clone()],
        )
        .expect("song db should store the test song");

        let mut score = rubato_core::score_data::ScoreData {
            sha256: song.file.sha256.clone(),
            ..Default::default()
        };
        score.judge_counts.epg = 100;
        score.judge_counts.lpg = 20;
        score.judge_counts.egr = 15;
        score.judge_counts.lgr = 5;
        score.notes = 400;
        score.maxcombo = 321;
        score.minbp = 7;
        score.playcount = 10;
        score.clearcount = 6;
        let score_db = ScoreDatabaseAccessor::new(
            player_dir
                .join("score.db")
                .to_str()
                .expect("score db path should be valid UTF-8"),
        )
        .expect("score db should open");
        score_db
            .create_table()
            .expect("score db schema should exist");
        score_db.set_score_data(&score);

        let info = SongInformation {
            sha256: song.file.sha256.clone(),
            mainbpm: 150.0,
            ..Default::default()
        };
        let info_db = SongInformationAccessor::new(
            info_db_path
                .to_str()
                .expect("song info db path should be valid UTF-8"),
        )
        .expect("song info db should open");
        write_song_info_row(&info_db_path, &info);

        let mut config = Config::default();
        config.playername = Some("player1".to_string());
        config.paths.playerpath = player_root.to_string_lossy().to_string();
        config.paths.songpath = song_db_path.to_string_lossy().to_string();
        config.paths.songinfopath = info_db_path.to_string_lossy().to_string();

        let mut player = PlayerConfig::default();
        player.skin[SkinType::MusicSelect.id() as usize] =
            Some(SkinConfig::new_with_path("skin/default/select.json"));
        player.validate();

        let mut controller = MainController::new(None, config, player.clone(), None, false);
        controller.set_info_database(Box::new(info_db));

        let mut selector = MusicSelector::with_song_database(Box::new(song_db));
        let queue = controller.controller_command_queue();
        selector.set_main_controller(Box::new(QueuedControllerAccess::from_controller(
            &mut controller,
            queue,
        )));
        selector.config = player;
        selector.create();

        let selected = selector
            .manager
            .selected()
            .and_then(|bar| bar.as_song_bar())
            .expect("selected song bar should exist");
        assert_eq!(
            selected
                .selectable
                .bar_data
                .score()
                .map(|row| row.exscore()),
            Some(score.exscore()),
            "standalone MusicSelect create() should load score data from the runtime score DB"
        );
        assert_eq!(
            selected
                .song_data()
                .info
                .as_ref()
                .map(|row| row.mainbpm as i32),
            Some(150),
            "standalone MusicSelect create() should load song information from the runtime song info DB"
        );
    }

    #[test]
    fn shared_music_selector_state_syncs_selector_main_state_data() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(MockSkin));

        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));
        shared.render();

        assert!(shared.main_state_data().skin.is_some());

        let mut sprite = SpriteBatchHelper::create_sprite_batch();
        shared.render_skin(&mut sprite);
        assert!(shared.main_state_data().skin.is_some());
    }

    #[test]
    fn shared_music_selector_state_delegates_sync_audio() {
        let config = Config::default();
        let mut selector = MusicSelector::with_config(config.clone());
        let mut preview = PreviewMusicProcessor::new(&config);
        preview.set_default("/bgm/default.ogg");
        preview.start(None);
        selector.preview_state.preview = Some(preview);

        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));
        let mut audio = RecordingAudioDriver::new();

        shared.sync_audio(&mut audio);

        assert_eq!(audio.play_path_count(), 1);
    }

    #[test]
    fn shared_music_selector_state_delegates_skin_mouse_pressed() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));
        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));

        <SharedMusicSelectorState as MainState>::handle_skin_mouse_pressed(&mut shared, 0, 32, 48);

        assert_eq!(
            shared.take_pending_state_change(),
            Some(MainStateType::Config)
        );
    }

    #[test]
    fn shared_music_selector_state_delegates_skin_mouse_dragged() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));
        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));

        <SharedMusicSelectorState as MainState>::handle_skin_mouse_dragged(&mut shared, 0, 32, 48);

        assert_eq!(
            shared.take_pending_state_change(),
            Some(MainStateType::SkinConfig)
        );
    }

    struct MockHttpDownloadSubmitter {
        submitted: Arc<Mutex<Vec<(String, String)>>>,
    }

    impl rubato_types::http_download_submitter::HttpDownloadSubmitter for MockHttpDownloadSubmitter {
        fn submit_md5_task(&self, md5: &str, task_name: &str) {
            self.submitted
                .lock()
                .unwrap()
                .push((md5.to_string(), task_name.to_string()));
        }
    }

    #[test]
    fn queued_controller_access_exposes_http_downloader() {
        let mut controller = make_test_controller();
        let submitted = Arc::new(Mutex::new(Vec::new()));
        controller.set_http_download_processor(Box::new(MockHttpDownloadSubmitter {
            submitted: Arc::clone(&submitted),
        }));
        let queue = controller.controller_command_queue();
        let access = QueuedControllerAccess::from_controller(&mut controller, queue);

        let downloader = access
            .http_downloader()
            .expect("queued access should keep the HTTP downloader connected");
        downloader.submit_md5_task("deadbeef", "Song");

        assert_eq!(
            &*submitted.lock().unwrap(),
            &[("deadbeef".to_string(), "Song".to_string())]
        );
    }

    #[test]
    fn queued_controller_access_shares_ranking_cache_with_controller() {
        let mut controller = make_test_controller();
        controller.set_ranking_data_cache(Box::new(
            rubato_ir::ranking_data_cache::RankingDataCache::new(),
        ));
        let queue = controller.controller_command_queue();
        let mut access = QueuedControllerAccess::from_controller(&mut controller, queue);
        let song = SongData::default();

        access
            .ranking_data_cache_mut()
            .expect("queued access should expose ranking cache")
            .put_song_any(
                &song,
                0,
                Box::new(rubato_ir::ranking_data::RankingData::new()),
            );

        let cached = controller
            .ranking_data_cache()
            .expect("controller should expose ranking cache")
            .song_any(&song, 0)
            .and_then(|any| any.downcast::<rubato_ir::ranking_data::RankingData>().ok())
            .map(|ranking| *ranking);
        assert!(
            cached.is_some(),
            "queued access should write into the controller-backed ranking cache"
        );
    }

    #[test]
    fn decide_state_uses_live_controller_input() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(LauncherStateFactory::new()));
        // create() initializes PlayerResource, which Decide requires
        mc.create();
        mc.change_state(MainStateType::Decide);

        {
            let state = mc
                .current_state_mut()
                .expect("decide state should be current");
            state
                .main_state_data_mut()
                .timer
                .set_timer_on(rubato_skin::skin_property::TIMER_STARTINPUT);
        }
        mc.input_processor_mut()
            .expect("controller should own an input processor")
            .set_key_state(0, true, 1);

        mc.render();

        assert!(
            mc.current_state()
                .expect("decide state should still be current for fadeout")
                .main_state_data()
                .timer
                .is_timer_on(rubato_skin::skin_property::TIMER_FADEOUT),
            "decide state should see the live controller input and enter fadeout"
        );
    }

    #[test]
    fn play_state_loads_skin_when_created_by_launcher() {
        let bms_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("test-bms")
            .join("minimal_7k.bms");
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(LauncherStateFactory::new()));
        mc.create();
        assert!(
            mc.player_resource_mut()
                .expect("controller should own a player resource")
                .set_bms_file(&bms_path, 0, 0),
            "test fixture should load into PlayerResource"
        );
        mc.change_state(MainStateType::Play);

        assert!(
            mc.current_state()
                .expect("play state should be current")
                .main_state_data()
                .skin
                .is_some(),
            "launcher-created play state should carry a loaded skin"
        );
    }

    /// Minimal mock IRConnection for state_factory tests.
    struct MockIRConnection;
    impl rubato_ir::ir_connection::IRConnection for MockIRConnection {
        fn get_rivals(
            &self,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_player_data::IRPlayerData>>
        {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(
            &self,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_table_data::IRTableData>>
        {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&rubato_ir::ir_player_data::IRPlayerData>,
            _chart: &rubato_ir::ir_chart_data::IRChartData,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
        {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&rubato_ir::ir_player_data::IRPlayerData>,
            _course: &rubato_ir::ir_course_data::IRCourseData,
        ) -> rubato_ir::ir_response::IRResponse<Vec<rubato_ir::ir_score_data::IRScoreData>>
        {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn send_play_data(
            &self,
            _model: &rubato_ir::ir_chart_data::IRChartData,
            _score: &rubato_ir::ir_score_data::IRScoreData,
        ) -> rubato_ir::ir_response::IRResponse<()> {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn send_course_play_data(
            &self,
            _course: &rubato_ir::ir_course_data::IRCourseData,
            _score: &rubato_ir::ir_score_data::IRScoreData,
        ) -> rubato_ir::ir_response::IRResponse<()> {
            rubato_ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_song_url(&self, _chart: &rubato_ir::ir_chart_data::IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(
            &self,
            _course: &rubato_ir::ir_course_data::IRCourseData,
        ) -> Option<String> {
            None
        }
        fn get_player_url(
            &self,
            _player: &rubato_ir::ir_player_data::IRPlayerData,
        ) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIR"
        }
    }

    #[test]
    fn test_extract_ir_statuses_from_core_controller() {
        use rubato_ir::ir_player_data::IRPlayerData;

        let mut mc = make_test_controller();
        let conn: Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection);
        let player = IRPlayerData::new("test-id".into(), "TestPlayer".into(), "1st".into());
        mc.ir_status_mut()
            .push(rubato_core::main_controller::IRStatus {
                config: rubato_core::ir_config::IRConfig::default(),
                rival_provider: None,
                connection: Some(Box::new(conn.clone())),
                player_data: Some(Box::new(player.clone())),
            });

        let extracted = extract_ir_statuses(&mc);

        assert_eq!(
            extracted.len(),
            1,
            "extract_ir_statuses should recover 1 IR status from core controller"
        );
        assert_eq!(extracted[0].player.id, "test-id");
        assert_eq!(extracted[0].player.name, "TestPlayer");
    }

    #[test]
    fn test_extract_ir_statuses_skips_entries_without_player_data() {
        let mut mc = make_test_controller();
        mc.ir_status_mut()
            .push(rubato_core::main_controller::IRStatus {
                config: rubato_core::ir_config::IRConfig::default(),
                rival_provider: None,
                connection: None,
                player_data: None,
            });

        let extracted = extract_ir_statuses(&mc);
        assert!(
            extracted.is_empty(),
            "extract_ir_statuses should skip entries without connection or player_data"
        );
    }

    #[test]
    fn test_result_state_receives_ir_statuses_from_core_controller() {
        use rubato_ir::ir_player_data::IRPlayerData;

        let mut mc = make_test_controller();
        mc.set_state_factory(Box::new(LauncherStateFactory::new()));
        let conn: Arc<dyn rubato_ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection);
        let player = IRPlayerData::new("ir-test".into(), "IRPlayer".into(), "2nd".into());
        mc.ir_status_mut()
            .push(rubato_core::main_controller::IRStatus {
                config: rubato_core::ir_config::IRConfig::default(),
                rival_provider: None,
                connection: Some(Box::new(conn)),
                player_data: Some(Box::new(player)),
            });

        // Create result state -- IR statuses should be wired through
        mc.change_state(MainStateType::Result);

        assert!(
            mc.current_state().is_some(),
            "result state should be created"
        );
        assert_eq!(
            mc.current_state_type(),
            Some(MainStateType::Result),
            "current state should be Result"
        );
    }
}
