// LauncherStateFactory — concrete StateFactory implementation.
// Creates all 6 screen state types for MainController state dispatch.
//
// Translated from: MainController.initializeStates() + createBMSPlayerState()
// Java creates states eagerly in initializeStates(); Rust creates them on-demand via factory.

use std::any::Any;
use std::collections::HashSet;
use std::sync::{Arc, Mutex};

use rubato_audio::audio_driver::AudioDriver;
use rubato_core::config_pkg::key_configuration::KeyConfiguration;
use rubato_core::config_pkg::skin_configuration::SkinConfiguration;
use rubato_core::main_controller::{MainController, StateCreateResult, StateFactory};
use rubato_core::main_state::{MainState, MainStateData, MainStateType};
use rubato_core::play_data_accessor::PlayDataAccessor;
use rubato_core::system_sound_manager::SystemSoundManager;
use rubato_core::timer_manager::TimerManager;
use rubato_ir::ir_chart_data::IRChartData;
use rubato_ir::ir_connection::IRConnection;
use rubato_ir::ir_course_data::IRCourseData;
use rubato_ir::ranking_data_cache::RankingDataCache;
use rubato_play::bga::bga_processor::BGAProcessor;
use rubato_play::bms_player::BMSPlayer;
use rubato_state::decide::music_decide::MusicDecide;
use rubato_state::decide::stubs::MainControllerRef as DecideMainControllerRef;
use rubato_state::result::course_result::CourseResult;
use rubato_state::result::music_result::MusicResult;
use rubato_state::result::stubs::BMSPlayerMode;
use rubato_state::result::stubs::BMSPlayerModeType;
use rubato_state::result::stubs::MainController as ResultMainController;
use rubato_state::result::stubs::PlayerResource as ResultPlayerResource;
use rubato_state::result::stubs::RankingData;
use rubato_state::select::music_selector::MusicSelector;
use rubato_types::main_controller_access::{
    MainControllerAccess, MainControllerCommand, MainControllerCommandQueue,
};
use rubato_types::player_information::PlayerInformation;
use rubato_types::player_resource_access::{NullPlayerResource, PlayerResourceAccess};
use rubato_types::score_data::ScoreData;
use rubato_types::sound_type::SoundType;

struct QueuedControllerAccess {
    config: rubato_core::config::Config,
    player_config: rubato_core::player_config::PlayerConfig,
    commands: MainControllerCommandQueue,
    sound: SystemSoundManager,
    play_data_accessor: PlayDataAccessor,
    ranking_data_cache: Box<dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess>,
    ir_connection: Option<Arc<dyn IRConnection + Send + Sync>>,
    rivals: Vec<PlayerInformation>,
    ipfs_download_alive: bool,
    http_downloader: Option<Arc<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>>,
    active_audio_paths: HashSet<String>,
}

fn ensure_controller_ranking_cache(controller: &mut MainController) {
    if controller.ranking_data_cache().is_none() {
        controller.set_ranking_data_cache(Box::new(RankingDataCache::new()));
    }
}

impl QueuedControllerAccess {
    fn from_controller(
        controller: &mut MainController,
        commands: MainControllerCommandQueue,
    ) -> Self {
        ensure_controller_ranking_cache(controller);
        let config = controller.config().clone();
        let player_config = controller.player_config().clone();
        let ir_connection = controller.ir_connection_any().and_then(|any| {
            any.downcast_ref::<Arc<dyn IRConnection + Send + Sync>>()
                .cloned()
        });
        let rivals = (0..controller.rival_count())
            .filter_map(|i| controller.rival_information(i))
            .collect();
        let ranking_data_cache = controller
            .ranking_data_cache()
            .map(|cache| cache.clone_box())
            .unwrap_or_else(|| Box::new(RankingDataCache::new()));

        Self {
            sound: SystemSoundManager::new(
                Some(config.bgmpath.as_str()),
                Some(config.soundpath.as_str()),
            ),
            play_data_accessor: PlayDataAccessor::new(&config),
            ranking_data_cache,
            ipfs_download_alive: controller.is_ipfs_download_alive(),
            http_downloader: controller.clone_http_download_processor(),
            config,
            player_config,
            commands,
            ir_connection,
            rivals,
            active_audio_paths: HashSet::new(),
        }
    }
}

impl MainControllerAccess for QueuedControllerAccess {
    fn config(&self) -> &rubato_types::config::Config {
        &self.config
    }

    fn player_config(&self) -> &rubato_types::player_config::PlayerConfig {
        &self.player_config
    }

    fn change_state(&mut self, state: MainStateType) {
        self.commands
            .push(MainControllerCommand::ChangeState(state));
    }

    fn save_config(&self) {
        self.commands.push(MainControllerCommand::SaveConfig);
    }

    fn exit(&self) {
        self.commands.push(MainControllerCommand::Exit);
    }

    fn save_last_recording(&self, reason: &str) {
        self.commands
            .push(MainControllerCommand::SaveLastRecording(reason.to_string()));
    }

    fn update_song(&mut self, path: Option<&str>) {
        self.commands
            .push(MainControllerCommand::UpdateSong(path.map(str::to_string)));
    }

    fn player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }

    fn player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }

    fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        self.commands
            .push(MainControllerCommand::PlaySound(sound.clone(), loop_sound));
    }

    fn stop_sound(&mut self, sound: &SoundType) {
        self.commands
            .push(MainControllerCommand::StopSound(sound.clone()));
    }

    fn sound_path(&self, sound: &SoundType) -> Option<String> {
        self.sound.sound(sound).cloned()
    }

    fn play_audio_path(&mut self, path: &str, volume: f32, loop_play: bool) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.insert(path.to_string());
        self.commands.push(MainControllerCommand::PlayAudioPath(
            path.to_string(),
            volume,
            loop_play,
        ));
    }

    fn set_audio_path_volume(&mut self, path: &str, volume: f32) {
        if path.is_empty() {
            return;
        }
        self.commands
            .push(MainControllerCommand::SetAudioPathVolume(
                path.to_string(),
                volume,
            ));
    }

    fn is_audio_path_playing(&self, path: &str) -> bool {
        self.active_audio_paths.contains(path)
    }

    fn stop_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.remove(path);
        self.commands
            .push(MainControllerCommand::StopAudioPath(path.to_string()));
    }

    fn dispose_audio_path(&mut self, path: &str) {
        if path.is_empty() {
            return;
        }
        self.active_audio_paths.remove(path);
        self.commands
            .push(MainControllerCommand::DisposeAudioPath(path.to_string()));
    }

    fn shuffle_sounds(&mut self) {
        self.sound.shuffle();
        self.commands.push(MainControllerCommand::ShuffleSounds);
    }

    fn read_replay_data(
        &self,
        sha256: &str,
        has_ln: bool,
        lnmode: i32,
        index: i32,
    ) -> Option<rubato_types::replay_data::ReplayData> {
        self.play_data_accessor
            .read_replay_data(sha256, has_ln, lnmode, index)
    }

    fn ir_song_url(&self, song_data: &rubato_types::song_data::SongData) -> Option<String> {
        self.ir_connection
            .as_ref()
            .and_then(|conn| conn.get_song_url(&IRChartData::new(song_data)))
    }

    fn ir_course_url(&self, course_data: &rubato_types::course_data::CourseData) -> Option<String> {
        self.ir_connection.as_ref().and_then(|conn| {
            conn.get_course_url(&IRCourseData::new_with_lntype(
                course_data,
                self.player_config.lnmode,
            ))
        })
    }

    fn update_table(
        &mut self,
        source: Box<dyn rubato_types::table_update_source::TableUpdateSource>,
    ) {
        self.commands
            .push(MainControllerCommand::UpdateTable(source));
    }

    fn http_downloader(
        &self,
    ) -> Option<&dyn rubato_types::http_download_submitter::HttpDownloadSubmitter> {
        self.http_downloader
            .as_ref()
            .map(|downloader| downloader.as_ref())
    }

    fn is_ipfs_download_alive(&self) -> bool {
        self.ipfs_download_alive
    }

    fn start_ipfs_download(&mut self, song: &rubato_types::song_data::SongData) -> bool {
        if !self.ipfs_download_alive {
            return false;
        }
        self.commands
            .push(MainControllerCommand::StartIpfsDownload(Box::new(
                song.clone(),
            )));
        true
    }

    fn ranking_data_cache(
        &self,
    ) -> Option<&dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess> {
        Some(&*self.ranking_data_cache)
    }

    fn ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn rubato_types::ranking_data_cache_access::RankingDataCacheAccess + 'static)>
    {
        Some(&mut *self.ranking_data_cache)
    }

    fn rival_count(&self) -> usize {
        self.rivals.len()
    }

    fn rival_information(&self, index: usize) -> Option<PlayerInformation> {
        self.rivals.get(index).cloned()
    }

    fn read_score_data_by_hash(&self, hash: &str, ln: bool, lnmode: i32) -> Option<ScoreData> {
        self.play_data_accessor
            .read_score_data_by_hash(hash, ln, lnmode)
    }

    fn read_player_data(&self) -> Option<rubato_types::player_data::PlayerData> {
        self.play_data_accessor.read_player_data()
    }

    fn ir_connection_any(&self) -> Option<&dyn Any> {
        self.ir_connection.as_ref().map(|conn| conn as &dyn Any)
    }
}

pub fn new_state_main_controller_access(
    controller: &mut MainController,
) -> Box<dyn MainControllerAccess + Send> {
    Box::new(QueuedControllerAccess::from_controller(
        controller,
        controller.controller_command_queue(),
    ))
}

struct QueuedAudioDriver {
    commands: MainControllerCommandQueue,
    global_pitch: f32,
}

impl QueuedAudioDriver {
    fn new(commands: MainControllerCommandQueue) -> Self {
        Self {
            commands,
            global_pitch: 1.0,
        }
    }
}

impl AudioDriver for QueuedAudioDriver {
    fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {}
    fn set_volume_path(&mut self, _path: &str, _volume: f32) {}
    fn is_playing_path(&self, _path: &str) -> bool {
        false
    }
    fn stop_path(&mut self, _path: &str) {}
    fn dispose_path(&mut self, _path: &str) {}
    fn set_model(&mut self, _model: &bms_model::bms_model::BMSModel) {}
    fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}
    fn abort(&mut self) {}
    fn get_progress(&self) -> f32 {
        1.0
    }
    fn play_note(&mut self, _n: &bms_model::note::Note, _volume: f32, _pitch: i32) {}
    fn play_judge(&mut self, _judge: i32, _fast: bool) {}
    fn stop_note(&mut self, _n: Option<&bms_model::note::Note>) {
        self.commands.push(MainControllerCommand::StopAllNotes);
    }
    fn set_volume_note(&mut self, _n: &bms_model::note::Note, _volume: f32) {}
    fn set_global_pitch(&mut self, pitch: f32) {
        self.global_pitch = pitch;
        self.commands
            .push(MainControllerCommand::SetGlobalPitch(pitch));
    }
    fn get_global_pitch(&self) -> f32 {
        self.global_pitch
    }
    fn dispose_old(&mut self) {}
    fn dispose(&mut self) {}
}

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
        let state_data = {
            let mut selector_guard = selector.lock().unwrap();
            std::mem::replace(
                &mut selector_guard.main_state_data,
                MainStateData::new(TimerManager::new()),
            )
        };
        Self {
            selector,
            state_data,
        }
    }

    fn with_selector<R>(&mut self, f: impl FnOnce(&mut MusicSelector) -> R) -> R {
        let mut selector = self.selector.lock().unwrap();
        std::mem::swap(&mut self.state_data, &mut selector.main_state_data);
        let result = f(&mut selector);
        std::mem::swap(&mut self.state_data, &mut selector.main_state_data);
        result
    }
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
        self.with_selector(|selector| selector.create());
    }

    fn prepare(&mut self) {
        self.with_selector(|selector| selector.prepare());
    }

    fn shutdown(&mut self) {
        self.with_selector(|selector| selector.shutdown());
    }

    fn render(&mut self) {
        self.with_selector(|selector| selector.render());
    }

    fn input(&mut self) {
        self.with_selector(|selector| selector.input());
    }

    fn sync_audio(&mut self, audio: &mut dyn rubato_audio::audio_driver::AudioDriver) {
        self.with_selector(|selector| selector.sync_audio(audio));
    }

    fn pause(&mut self) {
        self.with_selector(|selector| selector.pause());
    }

    fn resume(&mut self) {
        self.with_selector(|selector| selector.resume());
    }

    fn resize(&mut self, width: i32, height: i32) {
        self.with_selector(|selector| selector.resize(width, height));
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        self.with_selector(|selector| selector.handle_skin_mouse_pressed(button, x, y));
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        self.with_selector(|selector| selector.handle_skin_mouse_dragged(button, x, y));
    }

    fn dispose(&mut self) {
        self.with_selector(|selector| selector.dispose());
    }

    fn sound(&self, sound: SoundType) -> Option<String> {
        self.selector.lock().unwrap().sound(sound)
    }

    fn play_sound_loop(&mut self, sound: SoundType, loop_sound: bool) {
        self.with_selector(|selector| selector.play_sound_loop(sound, loop_sound));
    }

    fn stop_sound(&mut self, sound: SoundType) {
        self.with_selector(|selector| selector.stop_sound(sound));
    }

    fn load_skin(&mut self, skin_type: i32) {
        self.with_selector(|selector| selector.load_skin(skin_type));
    }

    fn render_skin(&mut self, sprite: &mut rubato_core::sprite_batch_helper::SpriteBatch) {
        self.with_selector(|selector| selector.render_skin(sprite));
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        self.with_selector(|selector| selector.take_pending_state_change())
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
        use rubato_play::target_property::TargetProperty;
        let mut target = TargetProperty::from_id(targetid)?;
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
                let config = controller.config();
                let mut selector = match rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor::new(
                    &config.songpath,
                    &config.bmsroot,
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
                selector.set_player_config(controller.player_config().clone());
                Some(StateCreateResult {
                    state: Box::new(selector),
                    target_score: None,
                })
            }
            MainStateType::Decide => {
                // Java: decide = new MusicDecide(this);
                let command_queue = controller.controller_command_queue();
                let mc_access =
                    QueuedControllerAccess::from_controller(controller, command_queue.clone());
                let resource: Box<dyn PlayerResourceAccess> =
                    if let Some(r) = controller.take_player_resource() {
                        Box::new(r)
                    } else {
                        Box::new(NullPlayerResource::new())
                    };
                let decide = MusicDecide::new(
                    DecideMainControllerRef::with_audio(
                        Box::new(mc_access),
                        Box::new(QueuedAudioDriver::new(command_queue)),
                    ),
                    resource,
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
                let resource = controller.player_resource();
                let model = resource
                    .and_then(|r| r.bms_model())
                    .cloned()
                    .unwrap_or_default();
                let song_resource_gen = controller.config().song_resource_gen;
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

                // Wire course mode flag
                let is_course_mode = resource.and_then(|r| r.course_data()).is_some();
                player.set_course_mode(is_course_mode);

                // Wire play mode from PlayerResource
                if let Some(mode) = resource.and_then(|r| r.play_mode()).cloned() {
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

                // Wire course constraints
                if let Some(res) = resource {
                    player.set_constraints(res.constraint());
                }

                // Wire guide SE from player config
                player.set_guide_se(controller.player_config().is_guide_se);

                // Wire audio config
                if let Some(audio_config) = controller.config().audio_config() {
                    player.set_fast_forward_freq_option(audio_config.fast_forward.clone());
                    player.set_bg_volume(audio_config.bgvolume);
                }

                // Wire replay data for REPLAY mode
                if let Some(replay) = resource.and_then(|r| r.replay_data()).cloned() {
                    player.set_active_replay(Some(replay));
                }

                // --- Target/rival score DB load ---
                // Java: main.getPlayDataAccessor().readScoreData(model, config.getLnmode())
                let lnmode = controller.player_config().lnmode;
                let sha256 = model.sha256();
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
                    let targetid = controller.player_config().targetid.clone();
                    let total_notes = model.total_notes();
                    Self::compute_target_score(&targetid, total_notes, controller)
                } else {
                    rival_score
                };
                player.set_target_score(target_score.clone());

                if let Some(skin_type) = player.skin_type()
                    && let Some(skin) = rubato_skin::skin_loader::load_skin_from_config(
                        controller.config(),
                        controller.player_config(),
                        skin_type.id(),
                    )
                {
                    player.set_skin_name(skin.header.name().map(str::to_string));
                    player.main_state_data_mut().skin = Some(Box::new(skin));
                }

                Some(StateCreateResult {
                    state: Box::new(player),
                    target_score,
                })
            }
            MainStateType::Result => {
                // Java: result = new MusicResult(this);
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
                    rr.set_bms_model(bm);
                    rr.set_course_bms_models(cm);
                    rr.set_ranking_data(ranking);
                    rr
                } else {
                    ResultPlayerResource::default()
                };
                let result = MusicResult::new(
                    ResultMainController::with_audio(
                        Box::new(mc_access),
                        Box::new(QueuedAudioDriver::new(command_queue)),
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
                    rr.set_bms_model(bm);
                    rr.set_course_bms_models(cm);
                    rr.set_ranking_data(ranking);
                    rr
                } else {
                    ResultPlayerResource::default()
                };
                let course_result = CourseResult::new(
                    ResultMainController::with_audio(
                        Box::new(mc_access),
                        Box::new(QueuedAudioDriver::new(command_queue)),
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
    use ::bms_model::bms_model::BMSModel;
    use ::bms_model::note::Note;
    use rubato_audio::audio_driver::AudioDriver;
    use rubato_core::sprite_batch_helper::SpriteBatchHelper;
    use rubato_ir::ranking_data::RankingData;
    use rubato_state::select::preview_music_processor::PreviewMusicProcessor;
    use rubato_types::skin_render_context::SkinRenderContext;
    use rubato_types::song_data::SongData;
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

    struct MockAudioDriver {
        play_count: usize,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self { play_count: 0 }
        }
    }

    impl AudioDriver for MockAudioDriver {
        fn play_path(&mut self, _path: &str, _volume: f32, _loop_play: bool) {
            self.play_count += 1;
        }

        fn set_volume_path(&mut self, _path: &str, _volume: f32) {}

        fn is_playing_path(&self, _path: &str) -> bool {
            false
        }

        fn stop_path(&mut self, _path: &str) {}

        fn dispose_path(&mut self, _path: &str) {}

        fn set_model(&mut self, _model: &BMSModel) {}

        fn set_additional_key_sound(&mut self, _judge: i32, _fast: bool, _path: Option<&str>) {}

        fn abort(&mut self) {}

        fn get_progress(&self) -> f32 {
            1.0
        }

        fn play_note(&mut self, _n: &Note, _volume: f32, _pitch: i32) {}

        fn play_judge(&mut self, _judge: i32, _fast: bool) {}

        fn stop_note(&mut self, _n: Option<&Note>) {}

        fn set_volume_note(&mut self, _n: &Note, _volume: f32) {}

        fn set_global_pitch(&mut self, _pitch: f32) {}

        fn get_global_pitch(&self) -> f32 {
            1.0
        }

        fn dispose_old(&mut self) {}

        fn dispose(&mut self) {}
    }

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
        assert_eq!(mc.current_state_type(), Some(MainStateType::MusicSelect));

        mc.change_state(MainStateType::Decide);
        assert_eq!(mc.current_state_type(), Some(MainStateType::Decide));

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
        selector.preview = Some(preview);

        let mut shared = SharedMusicSelectorState::new(Arc::new(Mutex::new(selector)));
        let mut audio = MockAudioDriver::new();

        shared.sync_audio(&mut audio);

        assert_eq!(audio.play_count, 1);
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
        controller.set_ranking_data_cache(Box::new(RankingDataCache::new()));
        let queue = controller.controller_command_queue();
        let mut access = QueuedControllerAccess::from_controller(&mut controller, queue);
        let song = SongData::default();

        access
            .ranking_data_cache_mut()
            .expect("queued access should expose ranking cache")
            .put_song_any(&song, 0, Box::new(RankingData::new()));

        let cached = controller
            .ranking_data_cache()
            .expect("controller should expose ranking cache")
            .song_any(&song, 0)
            .and_then(|any| any.downcast::<RankingData>().ok())
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
}
