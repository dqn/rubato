// Default state creation logic for MainController.
//
// Moved from LauncherStateFactory::create_state() so that production code no
// longer needs to call set_state_factory(). Test code can still override via
// set_state_factory() which takes priority when present.

use std::sync::Arc;

use crate::core::config_pkg::key_configuration::KeyConfiguration;
use crate::core::config_pkg::skin_configuration::SkinConfiguration;
use crate::core::main_controller::{MainController, StateCreateResult};
use crate::core::main_state::{MainState, MainStateType};
use crate::core::timer_manager::TimerManager;
use crate::decide::music_decide::MusicDecide;
use crate::game_screen::GameScreen;
use crate::play::bms_player::BMSPlayer;
use crate::result::BMSPlayerMode;
use crate::result::BMSPlayerModeType;
use crate::result::MainController as ResultMainController;
use crate::result::PlayerResource as ResultPlayerResource;
use crate::result::course_result::CourseResult;
use crate::result::music_result::MusicResult;
use crate::select::music_selector::MusicSelector;
use crate::skin::score_data::ScoreData;
use crate::state_factory::shared_selector::SharedMusicSelectorState;
use crate::state_factory::wire_selector_dependencies;

/// Extract result-crate IR statuses from core MainController's IR statuses.
fn extract_ir_statuses(controller: &MainController) -> Vec<crate::result::ir_status::IRStatus> {
    controller
        .ir_status()
        .iter()
        .filter_map(|core_ir| {
            let connection = core_ir.connection.as_ref()?.clone();
            let player = core_ir.player_data.as_ref()?.clone();
            Some(crate::result::ir_status::IRStatus::new(
                core_ir.config.clone(),
                connection,
                player,
            ))
        })
        .collect()
}

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

impl MainController {
    /// Create a concrete state for the given type.
    ///
    /// This is the default state creation path, used when no custom state factory
    /// has been set via `set_state_factory()`. The logic was moved from
    /// `LauncherStateFactory::create_state()` to eliminate the factory indirection
    /// for production code.
    pub(crate) fn create_state_for_type(
        &mut self,
        state_type: MainStateType,
    ) -> Option<StateCreateResult> {
        match state_type {
            MainStateType::MusicSelect => {
                // Java: selector = new MusicSelector(this, songUpdated);
                // If a shared selector exists (created for StreamController), use it
                // so stream request bars appear in the select screen.
                if let Some(arc) = self.shared_music_selector() {
                    let wrapper = SharedMusicSelectorState::new(Arc::clone(arc));
                    return Some(StateCreateResult {
                        state: GameScreen::SharedSelect(Box::new(wrapper)),
                        target_score: None,
                    });
                }
                // Fallback: create a standalone selector (no stream controller).
                // Open a separate SQLite connection for the selector (same pattern
                // as download processors in main.rs).
                let config = self.config().clone();
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
                wire_selector_dependencies(&mut selector, self);
                selector.config = self.player_config().clone();
                selector.app_config = config;
                Some(StateCreateResult {
                    state: GameScreen::Select(Box::new(selector)),
                    target_score: None,
                })
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                match self.take_player_resource() {
                    Some(resource) => {
                        // Pre-load the play skin on a background thread while the decide
                        // screen is displayed. By the time the user finishes viewing decide
                        // (typically 3+ seconds), the play skin is already loaded.
                        let model_mode = resource
                            .bms_model()
                            .and_then(|m| m.mode().copied())
                            .unwrap_or(bms::model::mode::Mode::BEAT_7K);
                        if let Some(skin_type) = crate::skin::skin_type::SkinType::values()
                            .into_iter()
                            .find(|&st| st.mode() == Some(model_mode))
                        {
                            let config = self.config().clone();
                            let player_config = self.player_config().clone();
                            let skin_type_id = skin_type.id();
                            self.preloaded_play_skin = Some((
                                skin_type_id,
                                std::thread::spawn(move || {
                                    crate::skin::skin_loader::load_skin_from_config(
                                        &config,
                                        &player_config,
                                        skin_type_id,
                                    )
                                }),
                            ));
                        }

                        let decide =
                            MusicDecide::new(self.config().clone(), resource, TimerManager::new());
                        Some(StateCreateResult {
                            state: GameScreen::Decide(Box::new(decide)),
                            target_score: None,
                        })
                    }
                    None => {
                        log::error!(
                            "Cannot enter Decide without PlayerResource; falling back to MusicSelect"
                        );
                        self.create_state_for_type(MainStateType::MusicSelect)
                    }
                }
            }
            MainStateType::Play => {
                // Java: new BMSPlayer(this, resource)
                // Get model from PlayerResource, fall back to default
                let resource = self.player_resource();
                let model = resource
                    .and_then(|r| r.bms_model())
                    .cloned()
                    .unwrap_or_default();
                let song_resource_gen = self.config().render.song_resource_gen;
                let mut player = BMSPlayer::new_with_resource_gen(model.clone(), song_resource_gen);

                // Reuse BGAProcessor from PlayerResource to preserve texture cache between plays.
                // Java: bga = resource.getBGAManager() (BMSPlayer.java line 545)
                if let Some(bga_arc) = resource.and_then(|r| r.bga()) {
                    player.set_bga_processor(Arc::clone(bga_arc));
                }

                // Wire player config
                player.set_player_config(self.player_config().clone());

                // Wire global config for skin property queries (BGA mode, etc.)
                player.set_config(self.config().clone());

                // Wire course mode flag and course info (index, song count)
                let is_course_mode = resource.and_then(|r| r.course_data()).is_some();
                player.set_course_mode(is_course_mode);
                if let Some(res) = resource {
                    let course_index = res.course_index();
                    let course_song_count =
                        res.course_bms_models().map_or(0, |models| models.len());
                    player.set_course_info(course_index, course_song_count);
                }

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
                        crate::skin::skin_render_context::compute_lnmode_from_chart(
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
                player.set_guide_se(self.player_config().display_settings.is_guide_se);

                // Wire skin offset snapshot from MainController.
                // Java: MainState inherits MainController.offset[] which skin objects
                // read via getOffset(id) during prepare().
                {
                    let offset_count = crate::core::main_controller::OFFSET_COUNT;
                    let mut offsets = Vec::with_capacity(offset_count);
                    for i in 0..offset_count {
                        offsets.push(self.offset(i as i32).copied().unwrap_or_default());
                    }
                    player.set_offset_snapshot(offsets);
                }

                // Wire audio config
                if let Some(audio_config) = self.config().audio_config() {
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
                if let Some(input) = self.input_processor() {
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
                    let freq = crate::modmenu::freq_trainer_menu::FreqTrainerMenu::get_freq();
                    let is_play_mode =
                        player.play_mode().mode == crate::core::bms_player_mode::Mode::Play;
                    let freq_option = self
                        .config()
                        .audio_config()
                        .map(|a| a.freq_option)
                        .unwrap_or(crate::skin::audio_config::FrequencyType::UNPROCESSED);
                    player.apply_freq_trainer(freq, is_play_mode, is_course_mode, &freq_option);
                }

                // --- Target/rival score DB load ---
                // Java: main.getPlayDataAccessor().readScoreData(model, config.getLnmode())
                let lnmode = self.player_config().play_settings.lnmode;
                let sha256 = &model.sha256;
                let has_ln = model.contains_undefined_long_note();
                let db_score = self.read_score_data_by_hash(sha256, has_ln, lnmode);
                player.set_db_score(db_score);

                // Java: resource.getRivalScoreData()
                let rival_score = resource.and_then(|r| r.rival_score_data()).cloned();
                player.set_rival_score(rival_score.clone());

                // Compute target score for both BMSPlayer and PlayerResource (result screen).
                // Java: TargetProperty.getTargetProperty(config.getTargetid()).getTarget(main)
                // Java: resource.setTargetScoreData(targetScore)
                let target_score = if rival_score.is_none() || is_course_mode {
                    let targetid = self.player_config().select_settings.targetid.clone();
                    let total_notes = model.total_notes();
                    compute_target_score(&targetid, total_notes, self)
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
                    // Try to use the play skin pre-loaded during the decide screen.
                    let preloaded = self.preloaded_play_skin.take().and_then(
                        |(preloaded_type_id, handle)| {
                            if preloaded_type_id == skin_type.id() {
                                handle.join().ok().flatten()
                            } else {
                                log::info!(
                                    "Preloaded skin type {} != requested {}; loading synchronously",
                                    preloaded_type_id,
                                    skin_type.id()
                                );
                                None
                            }
                        },
                    );
                    let skin = preloaded.or_else(|| {
                        crate::skin::skin_loader::load_skin_from_config(
                            self.config(),
                            self.player_config(),
                            skin_type.id(),
                        )
                    });
                    if let Some(skin) = skin {
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
                    state: GameScreen::Play(Box::new(player)),
                    target_score,
                })
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
                let core_res = match self.take_player_resource() {
                    Some(r) => r,
                    None => {
                        log::error!(
                            "Cannot enter Result without PlayerResource; falling back to MusicSelect"
                        );
                        return self.create_state_for_type(MainStateType::MusicSelect);
                    }
                };
                let ir_statuses = extract_ir_statuses(self);
                let config = self.config().clone();
                let ranking_cache = self
                    .ranking_data_cache()
                    .map(|cache| cache.clone_box())
                    .unwrap_or_else(|| {
                        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
                    });
                let sound_paths = self
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
                    state: GameScreen::Result(Box::new(result)),
                    target_score: None,
                })
            }
            MainStateType::CourseResult => {
                // Java: gresult = new CourseResult(this);
                let core_res = match self.take_player_resource() {
                    Some(r) => r,
                    None => {
                        log::error!(
                            "Cannot enter CourseResult without PlayerResource; falling back to MusicSelect"
                        );
                        return self.create_state_for_type(MainStateType::MusicSelect);
                    }
                };
                let ir_statuses = extract_ir_statuses(self);
                let config = self.config().clone();
                let ranking_cache = self
                    .ranking_data_cache()
                    .map(|cache| cache.clone_box())
                    .unwrap_or_else(|| {
                        Box::new(crate::ir::ranking_data_cache::RankingDataCache::new())
                    });
                let sound_paths = self
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
                    state: GameScreen::CourseResult(Box::new(course_result)),
                    target_score: None,
                })
            }
            MainStateType::Config => {
                // Java: keyconfig = new KeyConfiguration(this);
                let keyconfig = KeyConfiguration::new(self);
                Some(StateCreateResult {
                    state: GameScreen::Config(Box::new(keyconfig)),
                    target_score: None,
                })
            }
            MainStateType::SkinConfig => {
                // Java: skinconfig = new SkinConfiguration(this, player);
                let skinconfig = SkinConfiguration::new(self, self.player_config());
                Some(StateCreateResult {
                    state: GameScreen::SkinConfig(Box::new(skinconfig)),
                    target_score: None,
                })
            }
        }
    }
}
