// LauncherStateFactory -- concrete StateCreator provider.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

pub(crate) mod shared_selector;

use std::sync::Arc;

use crate::core::config_pkg::key_configuration::KeyConfiguration;
use crate::core::config_pkg::skin_configuration::SkinConfiguration;
use crate::core::main_controller::{MainController, StateCreateResult, StateCreator};
use crate::core::main_state::{MainState, MainStateType};
use crate::core::timer_manager::TimerManager;
use crate::play::bms_player::BMSPlayer;
use crate::state::decide::music_decide::MusicDecide;
use crate::state::result::BMSPlayerMode;
use crate::state::result::BMSPlayerModeType;
use crate::state::result::MainController as ResultMainController;
use crate::state::result::PlayerResource as ResultPlayerResource;
use crate::state::result::course_result::CourseResult;
use crate::state::result::music_result::MusicResult;
use crate::state::select::music_selector::MusicSelector;
use rubato_types::score_data::ScoreData;

use shared_selector::SharedMusicSelectorState;

use crate::game_screen::GameScreen;

/// Extract result-crate IR statuses from core MainController's IR statuses.
fn extract_ir_statuses(
    controller: &MainController,
) -> Vec<crate::state::result::ir_status::IRStatus> {
    controller
        .ir_status()
        .iter()
        .filter_map(|core_ir| {
            let connection = core_ir.connection.as_ref()?.clone();
            let player = core_ir.player_data.as_ref()?.clone();
            Some(crate::state::result::ir_status::IRStatus::new(
                core_ir.config.clone(),
                connection,
                player,
            ))
        })
        .collect()
}

/// Wire individual dependencies from MainController into a MusicSelector.
/// Wires individual dependencies from MainController into MusicSelector.
pub fn wire_selector_dependencies(selector: &mut MusicSelector, controller: &mut MainController) {
    use crate::ir::ranking_data_cache::RankingDataCache;
    use crate::song::song_information_accessor::SongInformationAccessor;

    // Ensure ranking data cache exists on controller
    if controller.ranking_data_cache().is_none() {
        controller.set_ranking_data_cache(Box::new(RankingDataCache::new()));
    }

    let config = controller.config().clone();

    // Ranking data cache (clone box for independent mutation)
    selector.ranking_data_cache = controller
        .ranking_data_cache()
        .map(|cache| cache.clone_box());

    // IR connection
    selector.ir_connection = controller.ir_connection().cloned();

    // Play data accessor
    selector.play_data_accessor = Some(crate::core::play_data_accessor::PlayDataAccessor::new(
        &config,
    ));

    // Song information database
    selector.info_database = controller.info_database().and_then(|_| {
        SongInformationAccessor::new(&config.paths.songinfopath)
            .map(|db| Box::new(db) as Box<dyn rubato_types::song_information_db::SongInformationDb>)
            .map_err(|e| {
                log::warn!(
                    "Failed to open song information database for MusicSelector: {}",
                    e
                );
                e
            })
            .ok()
    });

    // Rivals
    selector.rivals = (0..controller.rival_count())
        .filter_map(|i| controller.rival_information(i))
        .collect();

    // Sound paths
    if let Some(sm) = controller.sound_manager() {
        selector.sound_paths = sm.sound_map_clone();
    }

    // HTTP downloader
    selector.http_downloader = controller.clone_http_download_processor();

    // IPFS download alive
    selector.ipfs_download_alive = controller.is_ipfs_download_alive();
}

/// LauncherStateFactory -- creates concrete state instances for all screen types.
///
/// This struct provides the `StateCreator` closure (via `into_creator()`) that lives
/// in beatoraja-launcher, which has access to all screen state crates. Core cannot
/// import these directly due to the dependency direction (screen crates depend on
/// core, not vice versa).
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
        use crate::play::target_property::TargetProperty;
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

    /// Convert this factory into a `StateCreator` closure for use with `MainController`.
    pub fn into_creator(self) -> StateCreator {
        Box::new(move |state_type, controller| self.create_state(state_type, controller))
    }

    pub fn create_state(
        &self,
        state_type: MainStateType,
        controller: &mut MainController,
    ) -> Option<StateCreateResult> {
        match state_type {
            MainStateType::MusicSelect => {
                // Java: selector = new MusicSelector(this, songUpdated);
                // If a shared selector exists (created for StreamController), use it
                // so stream request bars appear in the select screen.
                if let Some(arc) = controller.shared_music_selector() {
                    let wrapper = SharedMusicSelectorState::new(Arc::clone(arc));
                    return Some(StateCreateResult {
                        state: Box::new(GameScreen::SharedSelect(Box::new(wrapper))),
                        target_score: None,
                    });
                }
                // Fallback: create a standalone selector (no stream controller).
                // Open a separate SQLite connection for the selector (same pattern
                // as download processors in main.rs).
                let config = controller.config().clone();
                let mut selector = match crate::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
                    &config.paths.songpath,
                    &config.paths.bmsroot,
                ) {
                    Ok(db) => MusicSelector::with_song_database(Box::new(db)),
                    Err(e) => {
                        log::warn!("Failed to open song database for MusicSelector: {}", e);
                        MusicSelector::with_config(config.clone())
                    }
                };
                // Wire individual dependencies directly.
                wire_selector_dependencies(&mut selector, controller);
                selector.config = controller.player_config().clone();
                selector.app_config = config;
                Some(StateCreateResult {
                    state: Box::new(GameScreen::Select(Box::new(selector))),
                    target_score: None,
                })
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                match controller.take_player_resource() {
                    Some(resource) => {
                        let decide = MusicDecide::new(
                            controller.config().clone(),
                            resource,
                            TimerManager::new(),
                        );
                        Some(StateCreateResult {
                            state: Box::new(GameScreen::Decide(Box::new(decide))),
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
                if let Some(bga_arc) = resource.and_then(|r| r.bga()) {
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

                // Wire player data for skin property IDs 17-19 (playtime) and 30-37, 333 (statistics).
                // Java: IntegerPropertyFactory reads state.main.getPlayerResource().getPlayerData()
                if let Some(res) = resource {
                    player.set_cumulative_playtime(res.player_data().playtime);
                    player.set_player_data(*res.player_data());
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
                    let offset_count = crate::core::main_controller::OFFSET_COUNT;
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

                // Wire replay key state for replay mode entry.
                // Java: BMSPlayer constructor reads key states from input processor.
                // main.getInputProcessor().getKeyState(N) -> keystate[N]
                if let Some(input) = controller.input_processor() {
                    player.set_replay_key_state(crate::play::bms_player::ReplayKeyState {
                        pattern_key: input.key_state(1),
                        option_key: input.key_state(2),
                        hs_key: input.key_state(4),
                        gauge_shift_key3: input.key_state(3),
                        gauge_shift_key5: input.key_state(5),
                    });
                }

                // --- Pattern modification pipeline ---
                // Java: BMSPlayer constructor lines 94-348
                // Initializes playinfo from config, restores replay data, handles RANDOM
                // syntax, calculates non-modifier assist, applies pattern modifiers
                // (scroll, LN, mine, extra, battle, random options, 7to9), and applies
                // HS replay config from replay mode.
                player.prepare_pattern_pipeline();

                // Apply frequency trainer if enabled (Java lines 246-267)
                // FreqTrainerMenu is a global static; read it here and pass to the player.
                // BMSPlayer stores freq_on/force_no_ir_send; these flow to PlayerResource
                // via ScoreHandoff when the play session ends.
                {
                    let freq =
                        crate::state::modmenu::freq_trainer_menu::FreqTrainerMenu::get_freq();
                    let is_play_mode =
                        player.play_mode().mode == crate::core::bms_player_mode::Mode::Play;
                    let freq_option = controller
                        .config()
                        .audio_config()
                        .map(|a| a.freq_option)
                        .unwrap_or(rubato_types::audio_config::FrequencyType::UNPROCESSED);
                    player.apply_freq_trainer(freq, is_play_mode, is_course_mode, &freq_option);
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
                    state: Box::new(GameScreen::Play(Box::new(player))),
                    target_score,
                })
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
                let core_res = match controller.take_player_resource() {
                    Some(r) => r,
                    None => {
                        log::error!(
                            "Cannot enter Result without PlayerResource; falling back to MusicSelect"
                        );
                        return self.create_state(MainStateType::MusicSelect, controller);
                    }
                };
                let ir_statuses = extract_ir_statuses(controller);
                let config = controller.config().clone();
                let ranking_cache = controller
                    .ranking_data_cache()
                    .map(|cache| cache.clone_box())
                    .unwrap_or_else(|| {
                        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
                    });
                let sound_paths = controller
                    .sound_manager()
                    .map(|sm| sm.sound_map_clone())
                    .unwrap_or_default();
                let pm = core_res.play_mode().cloned().unwrap_or_else(|| {
                    log::warn!("PlayerResource missing play_mode for Result state");
                    BMSPlayerMode::new(BMSPlayerModeType::Play)
                });
                let bm = core_res.bms_model().cloned().unwrap_or_default();
                let cm = core_res.course_bms_models().cloned();
                let ranking = core_res.ranking_data().cloned();
                let mut rr = ResultPlayerResource::new(core_res, pm);
                rr.bms_model = bm;
                rr.course_bms_models = cm;
                rr.ranking_data = ranking;
                let mut result_main =
                    ResultMainController::with_ir_statuses(config, ranking_cache, ir_statuses);
                result_main.set_sound_paths(sound_paths);
                let result = MusicResult::new(result_main, rr, TimerManager::new());
                Some(StateCreateResult {
                    state: Box::new(GameScreen::Result(Box::new(result))),
                    target_score: None,
                })
            }
            MainStateType::CourseResult => {
                // Java: gresult = new CourseResult(this);
                let core_res = match controller.take_player_resource() {
                    Some(r) => r,
                    None => {
                        log::error!(
                            "Cannot enter CourseResult without PlayerResource; falling back to MusicSelect"
                        );
                        return self.create_state(MainStateType::MusicSelect, controller);
                    }
                };
                let ir_statuses = extract_ir_statuses(controller);
                let config = controller.config().clone();
                let ranking_cache = controller
                    .ranking_data_cache()
                    .map(|cache| cache.clone_box())
                    .unwrap_or_else(|| {
                        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
                    });
                let sound_paths = controller
                    .sound_manager()
                    .map(|sm| sm.sound_map_clone())
                    .unwrap_or_default();
                let pm = core_res.play_mode().cloned().unwrap_or_else(|| {
                    log::warn!("PlayerResource missing play_mode for CourseResult state");
                    BMSPlayerMode::new(BMSPlayerModeType::Play)
                });
                let bm = core_res.bms_model().cloned().unwrap_or_default();
                let cm = core_res.course_bms_models().cloned();
                let ranking = core_res.ranking_data().cloned();
                let mut rr = ResultPlayerResource::new(core_res, pm);
                rr.bms_model = bm;
                rr.course_bms_models = cm;
                rr.ranking_data = ranking;
                let mut course_main =
                    ResultMainController::with_ir_statuses(config, ranking_cache, ir_statuses);
                course_main.set_sound_paths(sound_paths);
                let course_result = CourseResult::new(course_main, rr, TimerManager::new());
                Some(StateCreateResult {
                    state: Box::new(GameScreen::CourseResult(Box::new(course_result))),
                    target_score: None,
                })
            }
            MainStateType::Config => {
                // Java: keyconfig = new KeyConfiguration(this);
                let keyconfig = KeyConfiguration::new(controller);
                Some(StateCreateResult {
                    state: Box::new(GameScreen::Config(Box::new(keyconfig))),
                    target_score: None,
                })
            }
            MainStateType::SkinConfig => {
                // Java: skinconfig = new SkinConfiguration(this, player);
                let skinconfig = SkinConfiguration::new(controller, controller.player_config());
                Some(StateCreateResult {
                    state: Box::new(GameScreen::SkinConfig(Box::new(skinconfig))),
                    target_score: None,
                })
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::main_state::StateTransition;
    use crate::core::score_database_accessor::ScoreDatabaseAccessor;
    use crate::core::sprite_batch_helper::SpriteBatchHelper;
    use crate::song::song_information_accessor::SongInformationAccessor;
    use crate::state::select::preview_music_processor::PreviewMusicProcessor;
    use rubato_audio::audio_system::AudioSystem;
    use rubato_types::skin_config::SkinConfig;
    use rubato_types::skin_render_context::SkinRenderContext;
    use rubato_types::skin_type::SkinType;
    use rubato_types::song_data::SongData;
    use rubato_types::song_information::SongInformation;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    struct MockSkin;

    impl crate::core::main_state::SkinDrawable for MockSkin {
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
        fn prepare_skin(
            &mut self,
            _state_type: Option<rubato_types::main_state_type::MainStateType>,
        ) {
        }
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
            _batch: &mut crate::core::sprite_batch_helper::SpriteBatch,
        ) {
        }
    }

    use rubato_audio::recording_audio_driver::RecordingAudioDriver;

    fn make_empty_game_context() -> crate::core::app_context::GameContext {
        crate::core::app_context::GameContext {
            config: Config::default(),
            player: PlayerConfig::default(),
            audio: None,
            sound: None,
            loudness_analyzer: None,
            timer: crate::core::timer_manager::TimerManager::new(),
            input: None,
            input_poll_quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            db: Default::default(),
            offset: Vec::new(),
            showfps: false,
            debug: false,
            integration: Default::default(),
            lifecycle: Default::default(),
            exit_requested: std::sync::atomic::AtomicBool::new(false),
            resource: None,
            modmenu_outbox: std::sync::Arc::new(crate::state::modmenu::ModmenuOutbox::new()),
            transition: None,
        }
    }

    struct ChangeStateSkin;

    impl crate::core::main_state::SkinDrawable for ChangeStateSkin {
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

        fn prepare_skin(
            &mut self,
            _state_type: Option<rubato_types::main_state_type::MainStateType>,
        ) {
        }
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
            _batch: &mut crate::core::sprite_batch_helper::SpriteBatch,
        ) {
        }
    }
    use crate::core::config::Config;
    use crate::core::player_config::PlayerConfig;
    use crate::core::player_resource::PlayerResource;

    fn make_test_controller() -> MainController {
        let config = Config::default();
        let player = PlayerConfig::default();
        MainController::new(None, config, player, None, false)
    }

    /// Create a test controller with a PlayerResource installed so that
    /// Result / CourseResult state creation does not fall back to MusicSelect.
    fn make_test_controller_with_resource() -> MainController {
        let mut mc = make_test_controller();
        mc.restore_player_resource(PlayerResource::new(
            Config::default(),
            PlayerConfig::default(),
        ));
        mc
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

        // Decide/Result/CourseResult require a PlayerResource; without one they fall back to MusicSelect.
        // Use a controller with resource so all types create successfully.
        let mut controller = make_test_controller_with_resource();

        let types_without_decide = [
            MainStateType::MusicSelect,
            MainStateType::Play,
            MainStateType::Result,
            MainStateType::CourseResult,
            MainStateType::Config,
            MainStateType::SkinConfig,
        ];

        for state_type in &types_without_decide {
            // Result/CourseResult consume the resource via take_player_resource(),
            // so restore it before each iteration to ensure it is available.
            controller.restore_player_resource(PlayerResource::new(
                Config::default(),
                PlayerConfig::default(),
            ));
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
        let mut controller = make_test_controller_with_resource();

        let result = factory
            .create_state(MainStateType::Result, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_result_state_falls_back_without_resource() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        // Without PlayerResource, Result falls back to MusicSelect
        let result = factory
            .create_state(MainStateType::Result, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::MusicSelect));
    }

    #[test]
    fn test_course_result_state() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller_with_resource();

        let result = factory
            .create_state(MainStateType::CourseResult, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::CourseResult));
    }

    #[test]
    fn test_course_result_state_falls_back_without_resource() {
        let factory = LauncherStateFactory::new();
        let mut controller = make_test_controller();

        // Without PlayerResource, CourseResult falls back to MusicSelect
        let result = factory
            .create_state(MainStateType::CourseResult, &mut controller)
            .unwrap();
        assert_eq!(result.state.state_type(), Some(MainStateType::MusicSelect));
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
        mc.set_state_factory(LauncherStateFactory::new().into_creator());

        // Test full state dispatch via MainController
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        // Decide without PlayerResource falls back to MusicSelect
        mc.change_state(MainStateType::Decide);
        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        mc.change_state(MainStateType::Play);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Play));

        // Result/CourseResult require a PlayerResource; install one before each transition
        mc.restore_player_resource(PlayerResource::new(
            Config::default(),
            PlayerConfig::default(),
        ));
        mc.change_state(MainStateType::Result);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Result));

        mc.restore_player_resource(PlayerResource::new(
            Config::default(),
            PlayerConfig::default(),
        ));
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
        mc.set_state_factory(LauncherStateFactory::new().into_creator());

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

        let song_db = crate::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
            &song_db_path.to_string_lossy(),
            &[],
        )
        .expect("song db should open");
        rubato_types::song_database_accessor::SongDatabaseAccessor::set_song_datas(
            &song_db,
            &[song.clone()],
        )
        .expect("song db should store the test song");

        let mut score = crate::core::score_data::ScoreData {
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
        wire_selector_dependencies(&mut selector, &mut controller);
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
        let mut audio = AudioSystem::Recording(RecordingAudioDriver::new());

        shared.sync_audio(&mut audio);

        if let AudioSystem::Recording(ref inner) = audio {
            assert_eq!(inner.play_path_count(), 1);
        } else {
            panic!("expected Recording variant");
        }
    }

    #[test]
    fn shared_music_selector_state_delegates_skin_mouse_pressed() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));
        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));

        <SharedMusicSelectorState as MainState>::handle_skin_mouse_pressed(&mut shared, 0, 32, 48);

        // Mouse press sets pending_state_change; render_with_game_context drains it
        let mut ctx = make_empty_game_context();
        let result = shared.render_with_game_context(&mut ctx);
        assert_eq!(result, StateTransition::ChangeTo(MainStateType::Config));
    }

    #[test]
    fn shared_music_selector_state_delegates_skin_mouse_dragged() {
        let mut selector = MusicSelector::new();
        selector.main_state_data.skin = Some(Box::new(ChangeStateSkin));
        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));

        <SharedMusicSelectorState as MainState>::handle_skin_mouse_dragged(&mut shared, 0, 32, 48);

        // Mouse drag sets pending_state_change; render_with_game_context drains it
        let mut ctx = make_empty_game_context();
        let result = shared.render_with_game_context(&mut ctx);
        assert_eq!(result, StateTransition::ChangeTo(MainStateType::SkinConfig));
    }

    #[test]
    fn decide_state_uses_live_controller_input() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(LauncherStateFactory::new().into_creator());
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
        mc.set_state_factory(LauncherStateFactory::new().into_creator());
        mc.create();
        assert!(
            mc.player_resource_mut()
                .expect("controller should own a player resource")
                .set_bms_file(&bms_path, BMSPlayerMode::PLAY),
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
    impl crate::ir::ir_connection::IRConnection for MockIRConnection {
        fn get_rivals(
            &self,
        ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_player_data::IRPlayerData>>
        {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_table_datas(
            &self,
        ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_table_data::IRTableData>>
        {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_play_data(
            &self,
            _player: Option<&crate::ir::ir_player_data::IRPlayerData>,
            _chart: Option<&crate::ir::ir_chart_data::IRChartData>,
        ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
        {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_course_play_data(
            &self,
            _player: Option<&crate::ir::ir_player_data::IRPlayerData>,
            _course: &crate::ir::ir_course_data::IRCourseData,
        ) -> crate::ir::ir_response::IRResponse<Vec<crate::ir::ir_score_data::IRScoreData>>
        {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn send_play_data(
            &self,
            _model: &crate::ir::ir_chart_data::IRChartData,
            _score: &crate::ir::ir_score_data::IRScoreData,
        ) -> crate::ir::ir_response::IRResponse<()> {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn send_course_play_data(
            &self,
            _course: &crate::ir::ir_course_data::IRCourseData,
            _score: &crate::ir::ir_score_data::IRScoreData,
        ) -> crate::ir::ir_response::IRResponse<()> {
            crate::ir::ir_response::IRResponse::failure("mock".to_string())
        }
        fn get_song_url(&self, _chart: &crate::ir::ir_chart_data::IRChartData) -> Option<String> {
            None
        }
        fn get_course_url(
            &self,
            _course: &crate::ir::ir_course_data::IRCourseData,
        ) -> Option<String> {
            None
        }
        fn get_player_url(
            &self,
            _player: &crate::ir::ir_player_data::IRPlayerData,
        ) -> Option<String> {
            None
        }
        fn name(&self) -> &str {
            "MockIR"
        }
    }

    #[test]
    fn test_extract_ir_statuses_from_core_controller() {
        use crate::ir::ir_player_data::IRPlayerData;

        let mut mc = make_test_controller();
        let conn: Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection);
        let player = IRPlayerData::new("test-id".into(), "TestPlayer".into(), "1st".into());
        mc.ir_status_mut()
            .push(crate::core::main_controller::IRStatus {
                config: crate::core::ir_config::IRConfig::default(),
                rival_provider: None,
                connection: Some(conn.clone()),
                player_data: Some(player.clone()),
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
            .push(crate::core::main_controller::IRStatus {
                config: crate::core::ir_config::IRConfig::default(),
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
        use crate::ir::ir_player_data::IRPlayerData;

        let mut mc = make_test_controller_with_resource();
        mc.set_state_factory(LauncherStateFactory::new().into_creator());
        let conn: Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync> =
            Arc::new(MockIRConnection);
        let player = IRPlayerData::new("ir-test".into(), "IRPlayer".into(), "2nd".into());
        mc.ir_status_mut()
            .push(crate::core::main_controller::IRStatus {
                config: crate::core::ir_config::IRConfig::default(),
                rival_provider: None,
                connection: Some(conn),
                player_data: Some(player),
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
