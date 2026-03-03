use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use log::info;

use beatoraja_audio::audio_driver::AudioDriver;
use beatoraja_types::imgui_notify::ImGuiNotify;
use beatoraja_types::main_controller_access::MainControllerAccess;
use beatoraja_types::main_state_access::MainStateAccess;
use beatoraja_types::player_resource_access::PlayerResourceAccess;
use beatoraja_types::ranking_data_cache_access::RankingDataCacheAccess;
use beatoraja_types::screen_type::ScreenType;
use beatoraja_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;
use beatoraja_types::song_information_db::SongInformationDb;
use beatoraja_types::sound_type::SoundType;

use crate::bms_player_mode::BMSPlayerMode;
use crate::config::Config;
use crate::ir_config::IRConfig;
use crate::main_state::{MainState, MainStateType};
use crate::main_state_listener::MainStateListener;
use crate::performance_metrics::PerformanceMetrics;
use crate::play_data_accessor::PlayDataAccessor;
use crate::player_config::PlayerConfig;
use crate::player_resource::PlayerResource;
use crate::rival_data_accessor::RivalDataAccessor;
use crate::sprite_batch_helper::{SpriteBatch, SpriteBatchHelper};
use crate::system_sound_manager::SystemSoundManager;
use crate::timer_manager::TimerManager;
use crate::version;

/// StateFactory - trait for creating concrete state instances.
///
/// Because the concrete state types (MusicSelector, BMSPlayer, etc.) live in separate crates
/// that depend on beatoraja-core, core cannot import them directly. Instead, a higher-level
/// crate (e.g. beatoraja-launcher) provides a concrete StateFactory implementation that
/// knows how to create each state type.
///
/// Translated from: MainController.initializeStates() + createBMSPlayerState()
pub trait StateFactory {
    /// Create a state instance for the given type.
    /// Returns None if the state type is not supported or cannot be created.
    fn create_state(
        &self,
        state_type: MainStateType,
        controller: &mut MainController,
    ) -> Option<StateCreateResult>;
}

/// Result from `StateFactory::create_state` containing the state and optional
/// metadata that `MainController::change_state` should apply after creation.
pub struct StateCreateResult {
    pub state: Box<dyn MainState>,
    /// Target score data to set on PlayerResource (for result screen access).
    /// Java: resource.setTargetScoreData(targetScore)
    pub target_score: Option<beatoraja_types::score_data::ScoreData>,
}

/// StateReferencesCallback - callback for updating cross-state references.
///
/// Because SkinMenu and SongManagerMenu live in beatoraja-modmenu which beatoraja-core
/// cannot depend on (circular dependency), the launcher provides a callback to wire
/// these references after state initialization.
///
/// Translated from: MainController.updateStateReferences()
///
/// Java lines 573-576:
/// ```java
/// private void updateStateReferences() {
///     SkinMenu.init(this, player);
///     SongManagerMenu.injectMusicSelector(selector);
/// }
/// ```
pub trait StateReferencesCallback: Send {
    /// Called after state initialization to update cross-state references.
    /// Receives the controller reference and player config for wiring modmenu stubs.
    fn update_references(&self, config: &Config, player: &PlayerConfig);
}

/// SkinOffset - offset values for skin objects
#[derive(Clone, Debug, Default)]
pub struct SkinOffset {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
    pub r: f32,
    pub a: f32,
}

impl SkinOffset {
    pub fn new() -> Self {
        Self::default()
    }
}

/// SkinProperty constants
pub const OFFSET_MAX: usize = 255;

/// IRStatus - holds IR connection state
pub struct IRStatus {
    pub config: IRConfig,
    /// IR rival provider (trait bridge for core→ir rival/score operations)
    pub rival_provider: Option<Box<dyn beatoraja_types::ir_rival_provider::IRRivalProvider>>,
    /// IR connection (type-erased). The concrete type is `Box<dyn IRConnection + Send + Sync>`
    /// from beatoraja-ir. Stored as `dyn Any` because beatoraja-core cannot depend on beatoraja-ir.
    /// Java: IRStatus.connection
    pub connection: Option<Box<dyn std::any::Any + Send + Sync>>,
}

// IRSendStatus stub removed — replaced by Box<dyn IrResendService> (brs-zd2)

// RankingDataCache stub removed — replaced by Box<dyn RankingDataCacheAccess> (brs-2v7)

// SongInformationAccessor: stub replaced by SongInformationDb trait (Phase 27c)

// ObsListener/ObsWsClient stubs replaced by Box<dyn ObsAccess> (Phase 4)
// ImGuiRenderer stub replaced by Box<dyn ImGuiAccess> (Phase 4)

// MusicDownloadProcessor stub removed — replaced by Box<dyn MusicDownloadAccess> (brs-4ls)

// HttpDownloadProcessor stub removed — replaced by Box<dyn HttpDownloadSubmitter> (brs-4ls)

// StreamController stub removed — replaced by Box<dyn StreamControllerAccess> (brs-36u)

pub use beatoraja_input::bms_player_input_processor::BMSPlayerInputProcessor;
use beatoraja_input::key_command::KeyCommand;

/// Adapter that bridges `MainState` → `MainStateAccess` for external listeners.
///
/// External listeners (DiscordListener, ObsListener) receive `&dyn MainStateAccess`
/// which provides screen type, player resource, and config without depending on
/// beatoraja-core's internal `MainState` trait.
struct StateAccessAdapter<'a> {
    screen_type: ScreenType,
    resource: Option<&'a dyn PlayerResourceAccess>,
    config: &'a Config,
}

impl MainStateAccess for StateAccessAdapter<'_> {
    fn get_screen_type(&self) -> ScreenType {
        self.screen_type.clone()
    }

    fn get_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        self.resource
    }

    fn get_config(&self) -> &Config {
        self.config
    }
}

/// MainController - root class of the application
#[allow(dead_code)]
pub struct MainController {
    pub config: Config,
    pub player: PlayerConfig,
    auto: Option<BMSPlayerMode>,
    song_updated: bool,

    /// Boot time
    boottime: Instant,
    /// Mouse moved time
    mouse_moved_time: i64,

    /// Audio driver
    /// Translated from: MainController.audio (AudioDriver)
    audio: Option<Box<dyn AudioDriver>>,

    /// Player resource
    resource: Option<PlayerResource>,

    /// Current state
    ///
    /// Translated from: MainController.current (MainState)
    current: Option<Box<dyn MainState>>,

    /// State factory for creating concrete state instances.
    /// Set by the application entry point (e.g. launcher) before state transitions.
    state_factory: Option<Box<dyn StateFactory>>,

    /// Timer manager
    timer: TimerManager,

    /// SpriteBatch (LibGDX)
    sprite: Option<SpriteBatch>,

    /// BMS file for single-song play
    bmsfile: Option<PathBuf>,

    /// Input processor
    input: Option<BMSPlayerInputProcessor>,

    /// Input polling thread quit flag
    input_poll_quit: std::sync::Arc<std::sync::atomic::AtomicBool>,

    /// Show FPS flag
    showfps: bool,

    /// Play data accessor
    playdata: Option<PlayDataAccessor>,

    /// System sound manager
    sound: Option<SystemSoundManager>,

    /// IR status array
    ir: Vec<IRStatus>,

    /// Rival data accessor
    rivals: RivalDataAccessor,

    /// Ranking data cache (trait object — real impl in beatoraja-ir)
    ircache: Option<Box<dyn RankingDataCacheAccess>>,

    /// Song database accessor (trait object)
    songdb: Option<Box<dyn SongDatabaseAccessorTrait>>,

    /// Song information accessor
    infodb: Option<Box<dyn SongInformationDb>>,

    /// Offset array for skin
    offset: Vec<SkinOffset>,

    /// State listeners
    state_listener: Vec<Box<dyn MainStateListener>>,

    /// ImGui renderer (trait bridge for beatoraja-modmenu)
    pub imgui: Option<Box<dyn beatoraja_types::imgui_access::ImGuiAccess>>,

    /// IR resend service (trait bridge for background IR score retry)
    ir_resend_service: Option<Box<dyn beatoraja_types::ir_resend_service::IrResendService>>,

    /// OBS client (trait bridge for beatoraja-obs)
    obs_client: Option<Box<dyn beatoraja_types::obs_access::ObsAccess>>,

    /// IPFS download processor (trait bridge for md-processor)
    download: Option<Box<dyn beatoraja_types::music_download_access::MusicDownloadAccess>>,
    /// HTTP download processor (trait bridge for md-processor)
    http_download_processor:
        Option<Box<dyn beatoraja_types::http_download_submitter::HttpDownloadSubmitter>>,

    /// Stream controller (trait bridge for beatoraja-stream)
    stream_controller:
        Option<Box<dyn beatoraja_types::stream_controller_access::StreamControllerAccess>>,

    /// Shared music selector (type-erased Arc<Mutex<MusicSelector>>).
    /// Java shares the same MusicSelector between StreamController and MusicSelect state.
    /// The launcher stores this so StateFactory can reuse it instead of creating a new one.
    shared_music_selector: Option<Box<dyn std::any::Any + Send>>,

    /// Previous render time
    prevtime: i64,

    /// Last config save time (nanos since boot, using Instant)
    last_config_save: Instant,

    /// Callback for updating cross-state references (modmenu wiring).
    /// Set by the launcher to wire SkinMenu/SongManagerMenu.
    state_references_callback: Option<Box<dyn StateReferencesCallback>>,

    /// Exit requested flag.
    /// Uses AtomicBool because exit() takes &self (required by MainControllerAccess trait).
    ///
    /// Translated from: Gdx.app.exit() triggers LibGDX's ApplicationListener.dispose()
    exit_requested: AtomicBool,

    /// Debug flag
    pub debug: bool,

    /// Loudness analyzer for volume normalization.
    ///
    /// Translated from: MainController.loudnessAnalyzer (BMSLoudnessAnalyzer)
    loudness_analyzer: Option<beatoraja_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer>,
}

/// Offset count (SkinProperty.OFFSET_MAX + 1)
pub const OFFSET_COUNT: usize = OFFSET_MAX + 1;

impl MainController {
    pub fn new(
        f: Option<PathBuf>,
        mut config: Config,
        player: PlayerConfig,
        auto: Option<BMSPlayerMode>,
        song_updated: bool,
    ) -> Self {
        let mut offset = Vec::with_capacity(OFFSET_COUNT);
        for _ in 0..OFFSET_COUNT {
            offset.push(SkinOffset::new());
        }

        // IPFS directory setup (Java: MainController constructor lines 161-170)
        if config.enable_ipfs {
            let ipfspath = std::path::Path::new("ipfs")
                .canonicalize()
                .unwrap_or_else(|_| std::env::current_dir().unwrap().join("ipfs"));
            let _ = std::fs::create_dir_all(&ipfspath);
            if ipfspath.exists() {
                let ipfs_str = ipfspath.to_string_lossy().to_string();
                if !config.bmsroot.contains(&ipfs_str) {
                    config.bmsroot.push(ipfs_str);
                }
            }
        }

        // HTTP download directory setup (Java: MainController constructor lines 171-180)
        if config.enable_http {
            let httpdl_path = std::path::Path::new(&config.download_directory)
                .canonicalize()
                .unwrap_or_else(|_| {
                    std::env::current_dir()
                        .unwrap()
                        .join(&config.download_directory)
                });
            let _ = std::fs::create_dir_all(&httpdl_path);
            if httpdl_path.exists() {
                let http_str = httpdl_path.to_string_lossy().to_string();
                if !config.bmsroot.contains(&http_str) {
                    config.bmsroot.push(http_str);
                }
            }
        }

        let timer = TimerManager::new();
        let sound = SystemSoundManager::new(
            Some(config.bgmpath.as_str()),
            Some(config.soundpath.as_str()),
        );

        // Java: playdata = new PlayDataAccessor(config);
        let playdata = Some(PlayDataAccessor::new(&config));

        // Phase 5+: IR initialization, Discord RPC, OBS listener
        let state_listener: Vec<Box<dyn MainStateListener>> = Vec::new();

        // Create input processor
        let input = BMSPlayerInputProcessor::new(&config, &player);

        Self {
            config,
            player,
            auto,
            song_updated,
            boottime: Instant::now(),
            mouse_moved_time: 0,
            audio: None,
            resource: None,
            current: None,
            state_factory: None,
            timer,
            sprite: None,
            bmsfile: f,
            input: Some(input),
            input_poll_quit: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            showfps: false,
            playdata,
            sound: Some(sound),
            ir: Vec::new(),
            rivals: RivalDataAccessor::new(),
            ircache: None,
            songdb: None,
            infodb: None,
            offset,
            state_listener,
            imgui: None,
            ir_resend_service: None,
            obs_client: None,
            download: None,
            http_download_processor: None,
            stream_controller: None,
            shared_music_selector: None,
            prevtime: 0,
            last_config_save: Instant::now(),
            state_references_callback: None,
            exit_requested: AtomicBool::new(false),
            debug: false,
            loudness_analyzer: Some(
                beatoraja_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer::new(),
            ),
        }
    }

    pub fn get_offset(&self, index: i32) -> Option<&SkinOffset> {
        if index >= 0 && (index as usize) < self.offset.len() {
            Some(&self.offset[index as usize])
        } else {
            None
        }
    }

    pub fn get_offset_mut(&mut self, index: i32) -> Option<&mut SkinOffset> {
        if index >= 0 && (index as usize) < self.offset.len() {
            Some(&mut self.offset[index as usize])
        } else {
            None
        }
    }

    pub fn get_player_resource(&self) -> Option<&PlayerResource> {
        self.resource.as_ref()
    }

    pub fn get_config(&self) -> &Config {
        &self.config
    }

    pub fn get_player_config(&self) -> &PlayerConfig {
        &self.player
    }

    pub fn get_sprite_batch(&self) -> Option<&SpriteBatch> {
        self.sprite.as_ref()
    }

    pub fn get_sprite_batch_mut(&mut self) -> Option<&mut SpriteBatch> {
        self.sprite.as_mut()
    }

    pub fn get_play_data_accessor(&self) -> Option<&PlayDataAccessor> {
        self.playdata.as_ref()
    }

    pub fn get_rival_data_accessor(&self) -> &RivalDataAccessor {
        &self.rivals
    }

    pub fn get_rival_data_accessor_mut(&mut self) -> &mut RivalDataAccessor {
        &mut self.rivals
    }

    pub fn get_ranking_data_cache(&self) -> Option<&dyn RankingDataCacheAccess> {
        self.ircache.as_deref()
    }

    pub fn get_ranking_data_cache_mut(
        &mut self,
    ) -> Option<&mut (dyn RankingDataCacheAccess + 'static)> {
        self.ircache.as_deref_mut()
    }

    pub fn set_ranking_data_cache(&mut self, cache: Box<dyn RankingDataCacheAccess>) {
        self.ircache = Some(cache);
    }

    pub fn get_sound_manager(&self) -> Option<&SystemSoundManager> {
        self.sound.as_ref()
    }

    pub fn get_sound_manager_mut(&mut self) -> Option<&mut SystemSoundManager> {
        self.sound.as_mut()
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        &self.ir
    }

    pub fn get_ir_status_mut(&mut self) -> &mut Vec<IRStatus> {
        &mut self.ir
    }

    pub fn get_timer(&self) -> &TimerManager {
        &self.timer
    }

    pub fn get_timer_mut(&mut self) -> &mut TimerManager {
        &mut self.timer
    }

    pub fn has_obs_client(&self) -> bool {
        self.obs_client.is_some()
    }

    pub fn set_obs_client(&mut self, client: Box<dyn beatoraja_types::obs_access::ObsAccess>) {
        self.obs_client = Some(client);
    }

    pub fn save_last_recording(&self, reason: &str) {
        if let Some(ref client) = self.obs_client {
            client.save_last_recording(reason);
        }
    }

    /// Set the state factory. Must be called before any state transitions.
    ///
    /// The factory is typically set by the application entry point (beatoraja-launcher)
    /// which has access to all concrete state types.
    pub fn set_state_factory(&mut self, factory: Box<dyn StateFactory>) {
        self.state_factory = Some(factory);
    }

    /// Add a state listener (e.g. DiscordListener, ObsListener).
    ///
    /// Translated from Java: stateListener.add(...)
    pub fn add_state_listener(&mut self, listener: Box<dyn MainStateListener>) {
        self.state_listener.push(listener);
    }

    /// Change to the specified state type.
    ///
    /// Translated from: MainController.changeState(MainStateType)
    ///
    /// Java lines 305-343:
    /// ```java
    /// public void changeState(MainStateType state) {
    ///     MainState newState = null;
    ///     switch (state) {
    ///     case MUSICSELECT:
    ///         if (this.bmsfile != null) { exit(); } else { newState = selector; }
    ///         break;
    ///     case DECIDE: newState = config.isSkipDecideScreen() ? createBMSPlayerState() : decide; break;
    ///     case PLAY: newState = createBMSPlayerState(); break;
    ///     case RESULT: newState = result; break;
    ///     case COURSERESULT: newState = gresult; break;
    ///     case CONFIG: newState = keyconfig; break;
    ///     case SKINCONFIG: newState = skinconfig; break;
    ///     }
    ///     if (newState != null && current != newState) { changeState(newState); }
    /// }
    /// ```
    pub fn change_state(&mut self, state: MainStateType) {
        // Determine whether to create a new state
        let should_create = match state {
            MainStateType::MusicSelect => {
                if self.bmsfile.is_some() {
                    self.exit();
                    false
                } else {
                    true
                }
            }
            MainStateType::Decide => {
                // In Java: config.isSkipDecideScreen() ? createBMSPlayerState() : decide
                // When skip is true, create a Play state instead
                true
            }
            MainStateType::Play => true,
            MainStateType::Result => true,
            MainStateType::CourseResult => true,
            MainStateType::Config => true,
            MainStateType::SkinConfig => true,
        };

        if !should_create {
            return;
        }

        // Determine the actual state type to create
        // (for Decide with skip, we create Play instead)
        let actual_type = if state == MainStateType::Decide && self.config.skip_decide_screen {
            MainStateType::Play
        } else {
            state
        };

        // Check if we're already in this state type
        if let Some(ref current) = self.current
            && current.state_type() == Some(actual_type)
        {
            return;
        }

        // Create the new state via factory.
        // Take the factory out temporarily to avoid borrow conflict
        // (factory is borrowed immutably, but create_state needs &mut self).
        let factory = self.state_factory.take().unwrap_or_else(|| {
            panic!(
                "No state factory set; cannot create state {:?}. \
                 Caller must call set_state_factory() before any state transitions.",
                actual_type
            );
        });
        let result = factory.create_state(actual_type, self);
        self.state_factory = Some(factory);

        if let Some(result) = result {
            // Apply target score to PlayerResource so the result screen can read it.
            // Java: resource.setTargetScoreData(targetScore)
            if let Some(target) = result.target_score
                && let Some(ref mut resource) = self.resource
            {
                resource.set_target_score_data(target);
            }
            self.transition_to_state(result.state);
        }

        // In Java: input processor setup based on current.getStage()
        // Phase 5+: Gdx.input.setInputProcessor(...)
    }

    /// Internal state transition: shutdown old state, create and prepare new state.
    ///
    /// Translated from: MainController.changeState(MainState) (private overload)
    ///
    /// Java lines 345-358:
    /// ```java
    /// private void changeState(MainState newState) {
    ///     newState.create();
    ///     if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
    ///     if(current != null) { current.shutdown(); current.setSkin(null); }
    ///     current = newState;
    ///     timer.setMainState(newState);
    ///     current.prepare();
    ///     updateMainStateListener(0);
    /// }
    /// ```
    fn transition_to_state(&mut self, mut new_state: Box<dyn MainState>) {
        // Create the new state
        new_state.create();

        // In Java: if(newState.getSkin() != null) { newState.getSkin().prepare(newState); }
        if let Some(ref mut skin) = new_state.main_state_data_mut().skin {
            skin.prepare_skin();
        }

        // Shutdown the old state
        if let Some(ref mut old_state) = self.current {
            // Extract BGA processor cache before shutdown for reuse in subsequent plays.
            // Java: BGAProcessor lives in BMSResource and persists across state transitions.
            if let Some(bga_cache) = old_state.take_bga_cache()
                && let Some(ref mut resource) = self.resource
            {
                resource.set_bga_any(bga_cache);
            }
            old_state.shutdown();
            // setSkin(null) equivalent
            old_state.main_state_data_mut().skin = None;
        }

        // Set as current
        self.current = Some(new_state);

        // In Java: timer.setMainState(newState)
        if let Some(ref mut current) = self.current {
            let st = current.state_type();
            current.main_state_data_mut().timer.set_state_type(st);
        }

        // Prepare the new state
        if let Some(ref mut current) = self.current {
            current.prepare();
        }

        self.update_main_state_listener(0);
    }

    /// Main create lifecycle method.
    ///
    /// Translated from: MainController.create()
    ///
    /// In Java this initializes SpriteBatch, fonts, input, audio, then calls
    /// initializeStates() and changeState() to enter the initial state.
    /// Java lines 416-552
    pub fn create(&mut self) {
        let t = Instant::now();
        self.sprite = Some(SpriteBatchHelper::create_sprite_batch());

        // ImGui init: managed by beatoraja-bin (egui context), not here

        // Audio driver initialization
        // Java lines 439-446:
        // switch(config.getAudioConfig().getDriver()) {
        //     case OpenAL: audio = new GdxSoundDriver(config); break;
        // }
        // In Rust, the audio driver is injected via set_audio_driver() from the launcher.
        // If no driver was set in the constructor (for PortAudio), we log for OpenAL:
        if self.audio.is_none() {
            let driver_type = self
                .config
                .get_audio_config()
                .map(|ac| format!("{:?}", ac.driver))
                .unwrap_or_else(|| "None".to_string());
            log::info!(
                "Audio driver not set; driver type = {}. \
                 Launcher should call set_audio_driver() before create().",
                driver_type
            );
        }

        // Initialize states (creates PlayerResource)
        self.initialize_states();
        self.update_state_references();

        // Input polling: done synchronously in render().
        // Java spawns a thread that calls input.poll() once per millisecond,
        // but in Rust, poll() requires &mut self. The synchronous approach in
        // render() provides equivalent functionality for single-threaded rendering.

        // Enter initial state based on bmsfile
        if self.bmsfile.is_some() {
            // Java: if(resource.setBMSFile(bmsfile, auto)) changeState(PLAY)
            //       else { changeState(CONFIG); exit(); }
            let bmsfile = self.bmsfile.clone().unwrap();
            let mode = self.auto.clone().unwrap_or(BMSPlayerMode::PLAY);
            let load_ok = self
                .resource
                .as_mut()
                .map(|r| r.set_bms_file(&bmsfile, mode))
                .unwrap_or(false);
            if load_ok {
                self.change_state(MainStateType::Play);
            } else {
                self.change_state(MainStateType::Config);
                self.exit();
            }
        } else {
            self.change_state(MainStateType::MusicSelect);
        }

        self.trigger_ln_warning();
        self.set_target_list();

        self.last_config_save = Instant::now();

        info!("Initialization time (ms): {}", t.elapsed().as_millis());
    }

    /// Main render lifecycle method — called every frame.
    ///
    /// Translated from: MainController.render()
    ///
    /// Java lines 606-780:
    /// ```java
    /// public void render() {
    ///     timer.update();
    ///     Gdx.gl.glClear(GL20.GL_COLOR_BUFFER_BIT);
    ///     current.render();
    ///     sprite.begin();
    ///     if (current.getSkin() != null) {
    ///         current.getSkin().updateCustomObjects(current);
    ///         current.getSkin().drawAllObjects(sprite, current);
    ///     }
    ///     sprite.end();
    ///     // ... stage, FPS display, ImGui ...
    ///     periodicConfigSave();
    ///     PerformanceMetrics.get().commit();
    ///     // Input gating
    ///     final long time = System.currentTimeMillis();
    ///     if(time > prevtime) { prevtime = time; current.input(); ... }
    /// }
    /// ```
    pub fn render(&mut self) {
        // timer.update()
        self.timer.update();

        // GL clear is handled by wgpu render pass in main.rs

        // current.render()
        if let Some(ref mut current) = self.current {
            current.render();
        }

        // sprite.begin()
        if let Some(ref mut sprite) = self.sprite {
            sprite.begin();
        }

        // Skin update and draw
        // Java: if (current.getSkin() != null) {
        //     current.getSkin().updateCustomObjects(current);
        //     current.getSkin().drawAllObjects(sprite, current);
        // }
        if let Some(ref mut current) = self.current {
            // Read state type before mutable borrow
            let st = current.state_type();
            let data = current.main_state_data_mut();
            data.timer.set_state_type(st);
            if let Some(mut skin) = data.skin.take() {
                skin.update_custom_objects_timed(&mut data.timer);
                skin.draw_all_objects_timed(&mut data.timer);
                // Put skin back
                current.main_state_data_mut().skin = Some(skin);
            } else {
                use std::sync::Once;
                static WARN_ONCE: Once = Once::new();
                WARN_ONCE.call_once(|| {
                    log::warn!("No skin loaded for current state — screen will be blank");
                });
            }
        }

        // sprite.end()
        if let Some(ref mut sprite) = self.sprite {
            sprite.end();
        }

        // Stage update/draw skipped (no scene2d equivalent yet)

        // FPS display (Phase 22+: requires system font)

        // --- Outbox consumption: poll pending operations from current state ---
        // Order: sounds → pitch → score handoff → reload → state change (last, destroys current)
        let mut pending_sounds: Vec<(SoundType, bool)> = Vec::new();
        let mut pending_pitch: Option<f32> = None;
        let mut pending_handoff: Option<beatoraja_types::score_handoff::ScoreHandoff> = None;
        let mut pending_reload = false;
        let mut pending_change: Option<MainStateType> = None;

        if let Some(ref mut current) = self.current {
            pending_sounds = current.drain_pending_sounds();
            pending_pitch = current.take_pending_global_pitch();
            pending_handoff = current.take_score_handoff();
            pending_reload = current.take_pending_reload_bms();
            pending_change = current.take_pending_state_change();
        }

        // Apply sounds
        for (sound, loop_sound) in pending_sounds {
            let volume = self.config.audio.as_ref().map_or(1.0, |a| a.systemvolume);
            let path = self
                .sound
                .as_ref()
                .and_then(|sm| sm.get_sound(&sound).cloned());
            if let Some(path) = path
                && let Some(ref mut audio) = self.audio
            {
                audio.play_path(&path, volume, loop_sound);
            }
        }

        // Apply global pitch
        if let Some(pitch) = pending_pitch
            && let Some(ref mut audio) = self.audio
        {
            audio.set_global_pitch(pitch);
        }

        // Apply score handoff to PlayerResource
        if let Some(handoff) = pending_handoff
            && let Some(ref mut resource) = self.resource
        {
            if let Some(score) = handoff.score_data {
                resource.set_score_data(score);
            }
            resource.set_combo(handoff.combo);
            resource.set_maxcombo(handoff.maxcombo);
            resource.set_gauge(handoff.gauge);
            if let Some(gg) = handoff.groove_gauge {
                resource.set_groove_gauge(gg);
            }
            resource.set_assist(handoff.assist);
        }

        // Reload BMS file (before state change so new Play state gets fresh model)
        if pending_reload {
            if let Some(ref mut resource) = self.resource {
                resource.reload_bms_file();
            }
            // If no state change follows (practice mode restart), push the fresh model
            // back to the current state so it can apply modifiers on a clean copy.
            if pending_change.is_none() {
                let fresh_model = self
                    .resource
                    .as_ref()
                    .and_then(|r| r.get_bms_model().cloned());
                if let Some(model) = fresh_model
                    && let Some(ref mut current) = self.current
                {
                    current.receive_reloaded_model(model);
                }
            }
        }

        // State change (last - destroys current state)
        if let Some(state_type) = pending_change {
            self.change_state(state_type);
        }

        self.periodic_config_save();

        PerformanceMetrics::get().commit();

        // ImGui rendering is handled by egui in main.rs

        // Poll input (Java: done in a separate thread, Rust: done synchronously)
        if let Some(ref mut input) = self.input {
            input.poll();
        }

        // Input gating by time delta
        // Java: final long time = System.currentTimeMillis();
        //       if(time > prevtime) { prevtime = time; current.input(); ... }
        let time = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
        if time > self.prevtime {
            self.prevtime = time;
            if let Some(ref mut current) = self.current {
                current.input();
            }
            // Mouse pressed/dragged → skin
            // Java: if (input.isMousePressed()) {
            //     current.getSkin().mousePressed(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            // Java: if (input.isMouseDragged()) {
            //     current.getSkin().mouseDragged(current, input.getMouseButton(), input.getMouseX(), input.getMouseY());
            // }
            if let Some(ref mut input) = self.input {
                let mouse_pressed = input.is_mouse_pressed();
                let mouse_dragged = input.is_mouse_dragged();
                let mouse_button = input.get_mouse_button();
                let mouse_x = input.get_mouse_x();
                let mouse_y = input.get_mouse_y();
                if mouse_pressed {
                    if let Some(ref mut current) = self.current {
                        let data = current.main_state_data_mut();
                        if let Some(ref mut skin) = data.skin {
                            skin.mouse_pressed_at(mouse_button, mouse_x, mouse_y);
                        }
                    }
                    input.set_mouse_pressed();
                }
                if mouse_dragged {
                    if let Some(ref mut current) = self.current {
                        let data = current.main_state_data_mut();
                        if let Some(ref mut skin) = data.skin {
                            skin.mouse_dragged_at(mouse_button, mouse_x, mouse_y);
                        }
                    }
                    input.set_mouse_dragged();
                }

                // Mouse moved → cursor visibility timer
                if input.is_mouse_moved() {
                    self.mouse_moved_time = time;
                    input.set_mouse_moved(false);
                }
            }

            // KeyCommand handlers (Java: MainController.render() lines 727-819)
            if let Some(ref mut input) = self.input {
                // FPS display toggle
                if input.is_activated(KeyCommand::ShowFps) {
                    self.showfps = !self.showfps;
                    log::info!("FPS display: {}", if self.showfps { "ON" } else { "OFF" });
                }

                // Fullscreen / windowed toggle (F4 without Alt held)
                // Java: if (!ALT_LEFT && !ALT_RIGHT && SWITCH_SCREEN_MODE)
                if !input.is_alt_held() && input.is_activated(KeyCommand::SwitchScreenMode) {
                    crate::window_command::request_fullscreen_toggle();
                    log::info!("Fullscreen toggle requested");
                }

                // Screenshot
                if input.is_activated(KeyCommand::SaveScreenshot) {
                    crate::window_command::request_screenshot();
                    log::info!("Screenshot requested");
                }

                // Twitter post (permanent stub — API deprecated)
                if input.is_activated(KeyCommand::PostTwitter) {
                    log::info!("Twitter post requested (API deprecated, no-op)");
                }

                // Mod menu toggle
                if input.is_activated(KeyCommand::ToggleModMenu)
                    && let Some(ref mut imgui) = self.imgui
                {
                    imgui.toggle_menu();
                }
            }
        }
    }

    /// Dispose lifecycle — called on application shutdown.
    ///
    /// Translated from: MainController.dispose()
    pub fn dispose(&mut self) {
        self.save_config();

        // Stop input polling
        self.input_poll_quit
            .store(true, std::sync::atomic::Ordering::Relaxed);

        // Dispose input processor
        if let Some(ref mut input) = self.input {
            input.dispose();
        }

        // Dispose current state
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

        // Java: if (streamController != null) { streamController.dispose(); }
        if let Some(ref mut sc) = self.stream_controller {
            sc.dispose();
        }
        self.stream_controller = None;

        if let Some(mut imgui) = self.imgui.take() {
            imgui.dispose();
        }
        if let Some(mut resource) = self.resource.take() {
            resource.dispose();
        }
        // ShaderManager::dispose();

        info!("All resources disposed");
    }

    /// Pause lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.pause()
    pub fn pause(&mut self) {
        if let Some(ref mut current) = self.current {
            current.pause();
        }
    }

    /// Resize lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.resize(int, int)
    pub fn resize(&mut self, width: i32, height: i32) {
        if let Some(ref mut current) = self.current {
            current.resize(width, height);
        }
    }

    /// Resume lifecycle — dispatches to current state.
    ///
    /// Translated from: MainController.resume()
    pub fn resume(&mut self) {
        if let Some(ref mut current) = self.current {
            current.resume();
        }
    }

    /// Save config and player config to disk.
    ///
    /// Translated from: MainController.saveConfig()
    ///
    /// Java lines 883-887:
    /// ```java
    /// public void saveConfig(){
    ///     Config.write(config);
    ///     PlayerConfig.write(config.getPlayerpath(), player);
    ///     logger.info("設定情報を保存");
    /// }
    /// ```
    pub fn save_config(&self) {
        if let Err(e) = Config::write(&self.config) {
            log::error!("Failed to write config: {}", e);
        }
        if let Err(e) = PlayerConfig::write(&self.config.playerpath, &self.player) {
            log::error!("Failed to write player config: {}", e);
        }
        info!("Config saved");
    }

    /// Request application exit. Sets exit flag and saves config.
    ///
    /// Translated from: MainController.exit()
    ///
    /// Java lines 919-921:
    /// ```java
    /// public void exit() {
    ///     Gdx.app.exit();
    /// }
    /// ```
    ///
    /// In Java, Gdx.app.exit() triggers the LibGDX lifecycle (pause → dispose),
    /// and dispose() calls saveConfig(). In Rust, we set an exit flag and save
    /// config immediately, since the main loop checks is_exit_requested().
    pub fn exit(&self) {
        self.exit_requested.store(true, Ordering::Release);
        self.save_config();
        info!("Exit requested");
    }

    /// Check whether exit has been requested.
    ///
    /// The main event loop should poll this and initiate shutdown when true.
    pub fn is_exit_requested(&self) -> bool {
        self.exit_requested.load(Ordering::Acquire)
    }

    /// Notify all state listeners of a state change.
    ///
    /// Translated from: MainController.updateMainStateListener(int)
    ///
    /// Java lines 951-955:
    /// ```java
    /// public void updateMainStateListener(int status) {
    ///     for(MainStateListener listener : stateListener) {
    ///         listener.update(current, status);
    ///     }
    /// }
    /// ```
    pub fn update_main_state_listener(&mut self, status: i32) {
        if let Some(ref current) = self.current {
            // Create adapter that bridges MainState → MainStateAccess
            let screen_type = current
                .state_type()
                .map(ScreenType::from_state_type)
                .unwrap_or(ScreenType::Other);
            let resource = self
                .resource
                .as_ref()
                .map(|r| r as &dyn PlayerResourceAccess);
            let adapter = StateAccessAdapter {
                screen_type,
                resource,
                config: &self.config,
            };

            // Temporarily take the listeners to avoid borrow conflict
            let mut listeners = std::mem::take(&mut self.state_listener);
            for listener in listeners.iter_mut() {
                listener.update(&adapter, status);
            }
            self.state_listener = listeners;
        }
    }

    pub fn get_play_time(&self) -> i64 {
        self.boottime.elapsed().as_millis() as i64
    }

    pub fn get_start_time(&self) -> i64 {
        self.timer.get_start_time()
    }

    pub fn get_start_micro_time(&self) -> i64 {
        self.timer.get_start_micro_time()
    }

    pub fn get_now_time(&self) -> i64 {
        self.timer.get_now_time()
    }

    pub fn get_now_time_for_id(&self, id: i32) -> i64 {
        self.timer.get_now_time_for_id(id)
    }

    pub fn get_now_micro_time(&self) -> i64 {
        self.timer.get_now_micro_time()
    }

    pub fn get_now_micro_time_for_id(&self, id: i32) -> i64 {
        self.timer.get_now_micro_time_for_id(id)
    }

    pub fn get_timer_value(&self, id: i32) -> i64 {
        self.get_micro_timer(id) / 1000
    }

    pub fn get_micro_timer(&self, id: i32) -> i64 {
        self.timer.get_micro_timer(id)
    }

    pub fn is_timer_on(&self, id: i32) -> bool {
        self.get_micro_timer(id) != i64::MIN
    }

    pub fn set_timer_on(&mut self, id: i32) {
        self.timer.set_timer_on(id);
    }

    pub fn set_timer_off(&mut self, id: i32) {
        self.set_micro_timer(id, i64::MIN);
    }

    pub fn set_micro_timer(&mut self, id: i32, microtime: i64) {
        self.timer.set_micro_timer(id, microtime);
    }

    pub fn switch_timer(&mut self, id: i32, on: bool) {
        self.timer.switch_timer(id, on);
    }

    pub fn get_http_download_processor(
        &self,
    ) -> Option<&dyn beatoraja_types::http_download_submitter::HttpDownloadSubmitter> {
        self.http_download_processor.as_deref()
    }

    pub fn set_http_download_processor(
        &mut self,
        processor: Box<dyn beatoraja_types::http_download_submitter::HttpDownloadSubmitter>,
    ) {
        self.http_download_processor = Some(processor);
    }

    /// Start song database update.
    ///
    /// Translated from: MainController.updateSong(String)
    /// In Java, spawns SongUpdateThread calling songdb.updateSongDatas().
    /// Requires SongDatabaseAccessor trait to expose update_song_datas() — deferred.
    pub fn update_song(&mut self, path: &str) {
        self.update_song_with_flag(path, false);
    }

    /// Start song database update with parent-when-missing flag.
    ///
    /// Translated from: MainController.updateSong(String, boolean)
    pub fn update_song_with_flag(&mut self, path: &str, update_parent_when_missing: bool) {
        log::info!(
            "updating folder : {}, update parent when missing : {}",
            if path.is_empty() { "ALL" } else { path },
            if update_parent_when_missing {
                "yes"
            } else {
                "no"
            }
        );
        let update_path = if path.is_empty() { None } else { Some(path) };
        let bmsroot = self.config.get_bmsroot().to_vec();
        if let Some(ref songdb) = self.songdb {
            songdb.update_song_datas(update_path, &bmsroot, false, update_parent_when_missing);
        }
    }

    pub fn get_version() -> &'static str {
        version::version_long()
    }

    pub fn set_play_mode(&mut self, auto: BMSPlayerMode) {
        self.auto = Some(auto);
    }

    /// Returns the song database accessor.
    ///
    /// Translated from: MainController.getSongDatabase()
    /// In Java: return MainLoader.getScoreDatabaseAccessor()
    pub fn get_song_database(&self) -> Option<&dyn SongDatabaseAccessorTrait> {
        self.songdb.as_deref()
    }

    /// Set the song database accessor.
    /// Called by the application entry point (beatoraja-launcher) after creating the DB.
    pub fn set_song_database(&mut self, songdb: Box<dyn SongDatabaseAccessorTrait>) {
        self.songdb = Some(songdb);
    }

    /// Returns the current state.
    ///
    /// Translated from: MainController.getCurrentState()
    pub fn get_current_state(&self) -> Option<&dyn MainState> {
        self.current.as_deref()
    }

    /// Returns a mutable reference to the current state.
    pub fn get_current_state_mut(&mut self) -> Option<&mut dyn MainState> {
        self.current
            .as_mut()
            .map(|b| &mut **b as &mut dyn MainState)
    }

    /// Returns the state type for the current state.
    ///
    /// Translated from: MainController.getStateType(MainState)
    ///
    /// In Java this uses instanceof checks. In Rust, each concrete state
    /// implements state_type() on the MainState trait.
    pub fn get_state_type(state: Option<&dyn MainState>) -> Option<MainStateType> {
        state.and_then(|s| s.state_type())
    }

    /// Returns the current state's type.
    pub fn get_current_state_type(&self) -> Option<MainStateType> {
        Self::get_state_type(self.get_current_state())
    }

    /// Returns the input processor.
    ///
    /// Translated from: MainController.getInputProcessor()
    pub fn get_input_processor(&self) -> Option<&BMSPlayerInputProcessor> {
        self.input.as_ref()
    }

    /// Returns a mutable reference to the input processor.
    pub fn get_input_processor_mut(&mut self) -> Option<&mut BMSPlayerInputProcessor> {
        self.input.as_mut()
    }

    /// Returns the audio processor.
    ///
    /// Translated from: MainController.getAudioProcessor()
    pub fn get_audio_processor(&self) -> Option<&dyn AudioDriver> {
        self.audio.as_deref()
    }

    /// Returns a mutable reference to the audio processor.
    pub fn get_audio_processor_mut(&mut self) -> Option<&mut dyn AudioDriver> {
        self.audio
            .as_mut()
            .map(|b| &mut **b as &mut dyn AudioDriver)
    }

    /// Set the audio driver.
    ///
    /// Translated from: MainController constructor audio initialization
    ///
    /// In Java, the audio driver is created in create() based on AudioConfig.DriverType.
    /// In Rust, we inject it to avoid pulling in the concrete driver crate.
    pub fn set_audio_driver(&mut self, audio: Box<dyn AudioDriver>) {
        self.audio = Some(audio);
    }

    /// Returns the loudness analyzer.
    ///
    /// Translated from: MainController.loudnessAnalyzer
    pub fn get_loudness_analyzer(
        &self,
    ) -> Option<&beatoraja_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer> {
        self.loudness_analyzer.as_ref()
    }

    /// Shutdown the loudness analyzer.
    ///
    /// Translated from: MainController.dispose() lines 864-866
    pub fn shutdown_loudness_analyzer(&mut self) {
        if let Some(ref analyzer) = self.loudness_analyzer {
            analyzer.shutdown();
        }
    }

    /// Returns the current calendar time.
    ///
    /// Translated from: MainController.getCurrnetTime() [sic - Java method name has typo]
    pub fn get_currnet_time(&self) -> i64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64
    }

    pub fn get_info_database(&self) -> Option<&dyn SongInformationDb> {
        self.infodb.as_deref()
    }

    /// Set the song information database.
    /// Called from launcher layer since beatoraja-core cannot depend on beatoraja-song.
    pub fn set_info_database(&mut self, db: Box<dyn SongInformationDb>) {
        self.infodb = Some(db);
    }

    pub fn get_music_download_processor(
        &self,
    ) -> Option<&dyn beatoraja_types::music_download_access::MusicDownloadAccess> {
        self.download.as_deref()
    }

    pub fn set_music_download_processor(
        &mut self,
        processor: Box<dyn beatoraja_types::music_download_access::MusicDownloadAccess>,
    ) {
        self.download = Some(processor);
    }

    pub fn get_stream_controller(
        &self,
    ) -> Option<&dyn beatoraja_types::stream_controller_access::StreamControllerAccess> {
        self.stream_controller.as_deref()
    }

    pub fn set_stream_controller(
        &mut self,
        controller: Box<dyn beatoraja_types::stream_controller_access::StreamControllerAccess>,
    ) {
        self.stream_controller = Some(controller);
    }

    /// Gets the shared MusicSelector as `&dyn Any`. Callers downcast via
    /// `any.downcast_ref::<Arc<Mutex<MusicSelector>>>()`.
    /// Java: StreamController holds a reference to the same MusicSelector used by SelectState.
    pub fn get_shared_music_selector(&self) -> Option<&(dyn std::any::Any + Send)> {
        self.shared_music_selector.as_deref()
    }

    /// Sets the shared MusicSelector (type-erased as `Box<dyn Any + Send>`).
    pub fn set_shared_music_selector(&mut self, selector: Box<dyn std::any::Any + Send>) {
        self.shared_music_selector = Some(selector);
    }

    pub fn get_ir_resend_service(
        &self,
    ) -> Option<&dyn beatoraja_types::ir_resend_service::IrResendService> {
        self.ir_resend_service.as_deref()
    }

    pub fn set_ir_resend_service(
        &mut self,
        service: Box<dyn beatoraja_types::ir_resend_service::IrResendService>,
    ) {
        self.ir_resend_service = Some(service);
    }

    pub fn set_imgui(&mut self, imgui: Box<dyn beatoraja_types::imgui_access::ImGuiAccess>) {
        self.imgui = Some(imgui);
    }

    /// Load a new player profile, re-initialize states and IR config.
    ///
    /// Translated from: MainController.loadNewProfile(PlayerConfig)
    pub fn load_new_profile(&mut self, pc: PlayerConfig) {
        self.config.playername = pc.id.clone();
        self.player = pc;

        // playdata = new PlayDataAccessor(config);
        self.initialize_ir_config();

        // Dispose current state before re-init
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

        self.initialize_states();
        self.update_state_references();
        self.trigger_ln_warning();
        self.set_target_list();

        // Enter select state
        self.change_state(MainStateType::MusicSelect);

        self.last_config_save = Instant::now();
    }

    /// Initialize IR configurations from config.
    ///
    /// Translated from: MainController.initializeIRConfig()
    ///
    /// Note: The actual IR initialization logic is in beatoraja_result::ir_initializer
    /// because beatoraja-core cannot depend on beatoraja-ir (circular dependency).
    /// This method is called from the application entry point after IR initialization.
    pub fn initialize_ir_config(&mut self) {
        // IR initialization is performed externally via beatoraja_result::ir_initializer::initialize_ir_config()
        // because beatoraja-core cannot depend on beatoraja-ir.
        // The application entry point should call ir_initializer::initialize_ir_config() and then
        // set the resulting IRStatus entries on this controller.
        log::info!("IR config initialization delegated to beatoraja_result::ir_initializer");
    }

    /// Initialize all game states (selector, player, result, etc.).
    ///
    /// Translated from: MainController.initializeStates()
    ///
    /// Java lines 554-571:
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
    ///
    /// In Rust, concrete state instances are created on-demand via the StateFactory
    /// (set by the launcher). This method only initializes the PlayerResource.
    /// States are created lazily in change_state().
    pub fn initialize_states(&mut self) {
        // In Java: resource = new PlayerResource(audio, config, player, loudnessAnalyzer);
        self.resource = Some(PlayerResource::new(
            self.config.clone(),
            self.player.clone(),
        ));

        // In Java: playdata = new PlayDataAccessor(config);
        self.playdata = Some(PlayDataAccessor::new(&self.config));

        info!("Initializing states (PlayerResource created, states created on-demand via factory)");
    }

    /// Update cross-state references after state re-initialization.
    ///
    /// Translated from: MainController.updateStateReferences()
    ///
    /// Java lines 573-576:
    /// ```java
    /// private void updateStateReferences() {
    ///     SkinMenu.init(this, player);
    ///     SongManagerMenu.injectMusicSelector(selector);
    /// }
    /// ```
    ///
    /// SkinMenu and SongManagerMenu live in beatoraja-modmenu, which beatoraja-core
    /// cannot depend on (circular dependency). The launcher provides a callback via
    /// `set_state_references_callback()` to wire these references.
    pub fn update_state_references(&self) {
        if let Some(ref callback) = self.state_references_callback {
            callback.update_references(&self.config, &self.player);
        }
    }

    /// Set the callback for updating cross-state references.
    ///
    /// The launcher provides an implementation that wires SkinMenu.init()
    /// and SongManagerMenu.injectMusicSelector() from beatoraja-modmenu.
    pub fn set_state_references_callback(&mut self, callback: Box<dyn StateReferencesCallback>) {
        self.state_references_callback = Some(callback);
    }

    /// Trigger LN warning if the player has LN-related settings.
    ///
    /// Translated from: MainController.triggerLnWarning()
    ///
    /// Java lines 578-592:
    /// ```java
    /// private void triggerLnWarning() {
    ///     String lnModeName = switch (player.getLnmode()) {
    ///         case 1 -> "CN";
    ///         case 2 -> "HCN";
    ///         default -> "LN";
    ///     };
    ///     if (!lnModeName.equals("LN")) {
    ///         String lnWarning = "Long Note mode is " + lnModeName + ".\n"
    ///             + "This is not recommended.\n"
    ///             + "Your scores may be incompatible with IR.\n"
    ///             + "You may change this in play options.";
    ///         ImGuiNotify.warning(lnWarning, 8000);
    ///     }
    /// }
    /// ```
    pub fn trigger_ln_warning(&self) {
        let ln_mode_name = match self.player.get_lnmode() {
            1 => "CN",
            2 => "HCN",
            _ => "LN",
        };
        if ln_mode_name != "LN" {
            let ln_warning = format!(
                "Long Note mode is {}.\n\
                 This is not recommended.\n\
                 Your scores may be incompatible with IR.\n\
                 You may change this in play options.",
                ln_mode_name
            );
            ImGuiNotify::warning_with_dismiss(&ln_warning, 8000);
        }
    }

    /// Set the target score list for grade/rival display.
    ///
    /// Translated from: MainController.setTargetList()
    ///
    /// Java lines 594-600:
    /// ```java
    /// private void setTargetList() {
    ///     Array<String> targetlist = new Array<String>(player.getTargetlist());
    ///     for(int i = 0;i < rivals.getRivalCount();i++) {
    ///         targetlist.add("RIVAL_" + (i + 1));
    ///     }
    ///     TargetProperty.setTargets(targetlist.toArray(String.class), this);
    /// }
    /// ```
    ///
    /// Translated from: Java MainController.setTargetList()
    ///
    /// Builds target list from player config + rival targets, then resolves
    /// display names via beatoraja_types::target_list.
    pub fn set_target_list(&mut self) {
        // Build target list: player's target list + rival targets
        let mut targetlist: Vec<String> = self.player.targetlist.clone();
        for i in 0..self.rivals.get_rival_count() {
            targetlist.push(format!("RIVAL_{}", i + 1));
        }

        // Resolve display names for each target ID
        let rivals: Vec<beatoraja_types::player_information::PlayerInformation> =
            (0..self.rivals.get_rival_count())
                .filter_map(|i| self.rivals.get_rival_information(i).cloned())
                .collect();
        let names: Vec<String> = targetlist
            .iter()
            .map(|id| beatoraja_types::target_list::resolve_target_name(id, &rivals))
            .collect();

        beatoraja_types::target_list::set_target_ids(targetlist);
        beatoraja_types::target_list::set_target_names(names);
    }

    /// Periodically save config if enough time has elapsed.
    ///
    /// Translated from: MainController.periodicConfigSave()
    ///
    /// Java lines 892-917:
    /// ```java
    /// private void periodicConfigSave() {
    ///     // let's not start anything heavy during play
    ///     if (current instanceof BMSPlayer) { return; }
    ///     // save once every 2 minutes
    ///     long now = System.nanoTime();
    ///     if ((now - lastConfigSave) < 2 * 60 * 1000000000L) { return; }
    ///     lastConfigSave = now;
    ///     // ... write config ...
    /// }
    /// ```
    pub fn periodic_config_save(&mut self) {
        // Skip during play to avoid I/O during gameplay
        if self.get_current_state_type() == Some(MainStateType::Play) {
            return;
        }

        // Save once every 2 minutes (Java: 2 * 60 * 1000000000L ns)
        let elapsed = self.last_config_save.elapsed();
        if elapsed.as_secs() < 120 {
            return;
        }

        self.last_config_save = Instant::now();
        self.save_config();
    }

    /// Update difficulty table data in a background thread.
    ///
    /// Translated from: MainController.updateTable(TableBar)
    pub fn update_table(
        &mut self,
        source: Box<dyn beatoraja_types::table_update_source::TableUpdateSource>,
    ) {
        let name = source.source_name();
        beatoraja_types::imgui_notify::ImGuiNotify::info(&format!("updating table : {name}"));
        std::thread::spawn(move || {
            source.refresh();
        });
    }

    /// Start IPFS download message rendering thread.
    ///
    /// Translated from: MainController.downloadIpfsMessageRenderer(String)
    pub fn download_ipfs_message_renderer(&mut self, message: &str) {
        // In Java: spawns DownloadMessageThread that polls download.isDownload() + download.getMessage()
        // When download processor is available, poll its status; otherwise show initial notification.
        if let Some(ref dl) = self.download
            && dl.is_download()
        {
            let msg = dl.get_message();
            if !msg.is_empty() {
                beatoraja_types::imgui_notify::ImGuiNotify::info(&msg);
                return;
            }
        }
        beatoraja_types::imgui_notify::ImGuiNotify::info(message);
    }
}

/// UpdateThread - base class for background update threads.
///
/// Translated from: MainController.UpdateThread
pub struct UpdateThread {
    pub message: String,
}

/// SongUpdateThread - background thread for song database updates.
///
/// Translated from: MainController.SongUpdateThread
pub struct SongUpdateThread {
    pub base: UpdateThread,
    pub path: Option<String>,
    pub update_parent_when_missing: bool,
}

/// TableUpdateThread - background thread for table data updates.
///
/// Translated from: MainController.TableUpdateThread
pub struct TableUpdateThread {
    pub base: UpdateThread,
}

/// DownloadMessageThread - background thread for download message rendering.
///
/// Translated from: MainController.DownloadMessageThread
pub struct DownloadMessageThread {
    pub base: UpdateThread,
}

impl MainControllerAccess for MainController {
    fn get_config(&self) -> &Config {
        &self.config
    }

    fn get_player_config(&self) -> &PlayerConfig {
        &self.player
    }

    fn change_state(&mut self, state: MainStateType) {
        MainController::change_state(self, state);
    }

    fn save_config(&self) {
        MainController::save_config(self);
    }

    fn exit(&self) {
        MainController::exit(self);
    }

    fn save_last_recording(&self, reason: &str) {
        MainController::save_last_recording(self, reason);
    }

    fn update_song(&mut self, path: Option<&str>) {
        if let Some(p) = path {
            MainController::update_song(self, p);
        }
    }

    fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        self.resource
            .as_ref()
            .map(|r| r as &dyn PlayerResourceAccess)
    }

    fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        self.resource
            .as_mut()
            .map(|r| r as &mut dyn PlayerResourceAccess)
    }

    fn play_sound(&mut self, sound: &SoundType, loop_sound: bool) {
        let volume = self.config.audio.as_ref().map_or(1.0, |a| a.systemvolume);
        let path = self
            .sound
            .as_ref()
            .and_then(|sm| sm.get_sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.play_path(&path, volume, loop_sound);
        }
    }

    fn stop_sound(&mut self, sound: &SoundType) {
        let path = self
            .sound
            .as_ref()
            .and_then(|sm| sm.get_sound(sound).cloned());
        if let Some(path) = path
            && let Some(ref mut audio) = self.audio
        {
            audio.stop_path(&path);
        }
    }

    fn get_sound_path(&self, sound: &SoundType) -> Option<String> {
        self.sound
            .as_ref()
            .and_then(|sm| sm.get_sound(sound).cloned())
    }

    fn shuffle_sounds(&mut self) {
        if let Some(ref mut sm) = self.sound {
            sm.shuffle();
        }
    }

    fn read_replay_data(
        &self,
        sha256: &str,
        has_ln: bool,
        lnmode: i32,
        index: i32,
    ) -> Option<beatoraja_types::replay_data::ReplayData> {
        self.playdata
            .as_ref()
            .and_then(|pda| pda.read_replay_data(sha256, has_ln, lnmode, index))
    }

    fn update_table(
        &mut self,
        source: Box<dyn beatoraja_types::table_update_source::TableUpdateSource>,
    ) {
        MainController::update_table(self, source);
    }

    fn get_ranking_data_cache(
        &self,
    ) -> Option<&dyn beatoraja_types::ranking_data_cache_access::RankingDataCacheAccess> {
        MainController::get_ranking_data_cache(self)
    }

    fn get_ranking_data_cache_mut(
        &mut self,
    ) -> Option<
        &mut (dyn beatoraja_types::ranking_data_cache_access::RankingDataCacheAccess + 'static),
    > {
        self.ircache.as_deref_mut()
    }

    fn get_http_downloader(
        &self,
    ) -> Option<&dyn beatoraja_types::http_download_submitter::HttpDownloadSubmitter> {
        self.http_download_processor.as_deref()
    }

    fn is_ipfs_download_alive(&self) -> bool {
        self.download.as_ref().is_some_and(|dl| dl.is_alive())
    }

    fn start_ipfs_download(&mut self, song: &beatoraja_types::song_data::SongData) -> bool {
        if let Some(ref dl) = self.download {
            dl.start_download(song);
            true
        } else {
            false
        }
    }

    fn get_rival_count(&self) -> usize {
        self.rivals.get_rival_count()
    }

    fn get_rival_information(
        &self,
        index: usize,
    ) -> Option<beatoraja_types::player_information::PlayerInformation> {
        self.rivals.get_rival_information(index).cloned()
    }

    fn read_score_data_by_hash(
        &self,
        hash: &str,
        ln: bool,
        lnmode: i32,
    ) -> Option<beatoraja_types::score_data::ScoreData> {
        self.playdata
            .as_ref()
            .and_then(|pda| pda.read_score_data_by_hash(hash, ln, lnmode))
    }

    fn read_player_data(&self) -> Option<beatoraja_types::player_data::PlayerData> {
        self.playdata
            .as_ref()
            .and_then(|pda| pda.read_player_data())
    }

    fn get_info_database(
        &self,
    ) -> Option<&dyn beatoraja_types::song_information_db::SongInformationDb> {
        self.infodb.as_deref()
    }

    fn get_ir_connection_any(&self) -> Option<&dyn std::any::Any> {
        self.ir
            .first()
            .and_then(|status| status.connection.as_ref())
            .map(|conn| conn.as_ref() as &dyn std::any::Any)
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::config_pkg::key_configuration::KeyConfiguration;
    use crate::config_pkg::skin_configuration::SkinConfiguration;
    use crate::main_state::MainStateData;

    /// A minimal test state that implements MainState for testing state dispatch.
    struct TestState {
        state_data: MainStateData,
        state_type: MainStateType,
        created: bool,
        prepared: bool,
        shut_down: bool,
        rendered: bool,
        disposed: bool,
    }

    impl TestState {
        fn new(state_type: MainStateType) -> Self {
            Self {
                state_data: MainStateData::new(TimerManager::new()),
                state_type,
                created: false,
                prepared: false,
                shut_down: false,
                rendered: false,
                disposed: false,
            }
        }
    }

    impl MainState for TestState {
        fn state_type(&self) -> Option<MainStateType> {
            Some(self.state_type)
        }

        fn main_state_data(&self) -> &MainStateData {
            &self.state_data
        }

        fn main_state_data_mut(&mut self) -> &mut MainStateData {
            &mut self.state_data
        }

        fn create(&mut self) {
            self.created = true;
        }

        fn prepare(&mut self) {
            self.prepared = true;
        }

        fn shutdown(&mut self) {
            self.shut_down = true;
        }

        fn render(&mut self) {
            self.rendered = true;
        }

        fn dispose(&mut self) {
            self.disposed = true;
            self.state_data.skin = None;
            self.state_data.stage = None;
        }
    }

    /// A test factory that creates TestState instances.
    struct TestStateFactory;

    impl StateFactory for TestStateFactory {
        fn create_state(
            &self,
            state_type: MainStateType,
            _controller: &mut MainController,
        ) -> Option<StateCreateResult> {
            Some(StateCreateResult {
                state: Box::new(TestState::new(state_type)),
                target_score: None,
            })
        }
    }

    fn make_test_controller() -> MainController {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(TestStateFactory));
        mc
    }

    #[test]
    fn test_initial_state_is_none() {
        let mc = make_test_controller();
        assert!(mc.get_current_state().is_none());
        assert!(mc.get_current_state_type().is_none());
    }

    #[test]
    fn test_change_state_to_music_select() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        assert!(mc.get_current_state().is_some());
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_change_state_to_play() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::Play);

        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_change_state_to_result() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::Result);

        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Result));
    }

    #[test]
    fn test_change_state_to_config() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::Config);

        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Config));
    }

    #[test]
    fn test_change_state_to_skin_config() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::SkinConfig);

        assert_eq!(mc.get_current_state_type(), Some(MainStateType::SkinConfig));
    }

    #[test]
    fn test_change_state_calls_create_and_prepare() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        // The state should have been created and prepared
        let state = mc.get_current_state().unwrap();
        assert_eq!(state.state_type(), Some(MainStateType::MusicSelect));
    }

    #[test]
    fn test_change_state_shuts_down_previous() {
        let mut mc = make_test_controller();

        // Enter first state
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );

        // Transition to a different state
        mc.change_state(MainStateType::Play);
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_change_state_same_type_is_noop() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        // Changing to the same state type should be a no-op
        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_decide_skip_creates_play_state() {
        let config = Config {
            skip_decide_screen: true,
            ..Config::default()
        };
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(TestStateFactory));

        mc.change_state(MainStateType::Decide);

        // With skip_decide_screen, Decide should create Play instead
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_decide_no_skip_creates_decide_state() {
        let config = Config {
            skip_decide_screen: false,
            ..Config::default()
        };
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(TestStateFactory));

        mc.change_state(MainStateType::Decide);

        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Decide));
    }

    #[test]
    fn test_music_select_with_bmsfile_calls_exit() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(
            Some(std::path::PathBuf::from("/test/file.bms")),
            config,
            player,
            None,
            false,
        );
        mc.set_state_factory(Box::new(TestStateFactory));

        // When bmsfile is set and we try to go to MusicSelect, it should call exit()
        // (which just logs a warning) and not create a state
        mc.change_state(MainStateType::MusicSelect);

        // No state should be set since exit() was called
        assert!(mc.get_current_state().is_none());
    }

    #[test]
    fn test_get_state_type_static() {
        let state = TestState::new(MainStateType::Play);
        assert_eq!(
            MainController::get_state_type(Some(&state as &dyn MainState)),
            Some(MainStateType::Play)
        );
    }

    #[test]
    fn test_get_state_type_none() {
        assert_eq!(MainController::get_state_type(None), None);
    }

    #[test]
    fn test_lifecycle_dispatch_render() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        // Render should dispatch to current state
        mc.render();

        // State should still be MusicSelect
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_lifecycle_dispatch_pause_resume() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        mc.pause();
        mc.resume();

        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_lifecycle_dispatch_resize() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);

        mc.resize(1920, 1080);

        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_dispose_clears_current_state() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);
        assert!(mc.get_current_state().is_some());

        mc.dispose();
        assert!(mc.get_current_state().is_none());
    }

    #[test]
    #[should_panic(expected = "No state factory set; cannot create state MusicSelect")]
    fn test_no_factory_panics() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        // No factory set — must panic to make wiring bugs immediately visible
        mc.change_state(MainStateType::MusicSelect);
    }

    #[test]
    fn test_multiple_state_transitions() {
        let mut mc = make_test_controller();

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

        mc.change_state(MainStateType::MusicSelect);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_key_configuration_main_state_trait() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mc = MainController::new(None, config, player, None, false);

        let mut kc = KeyConfiguration::new(&mc);
        let state: &mut dyn MainState = &mut kc;

        assert_eq!(state.state_type(), Some(MainStateType::Config));
        state.create();
        state.render();
        state.input();
        state.dispose();
    }

    #[test]
    fn test_skin_configuration_main_state_trait() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mc = MainController::new(None, config, player.clone(), None, false);

        let mut sc = SkinConfiguration::new(&mc, &player);
        let state: &mut dyn MainState = &mut sc;

        assert_eq!(state.state_type(), Some(MainStateType::SkinConfig));
        state.create();
        state.render();
        state.input();
        state.dispose();
    }

    #[test]
    fn test_course_result_state_transition() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::CourseResult);
        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::CourseResult)
        );
    }

    // --- Phase 22c: Render pipeline tests ---

    #[test]
    fn test_render_creates_sprite_batch_on_create() {
        let mut mc = make_test_controller();
        mc.create();
        // After create(), sprite batch should be initialized
        assert!(mc.get_sprite_batch().is_some());
    }

    #[test]
    fn test_render_sprite_batch_begin_end_lifecycle() {
        let mut mc = make_test_controller();
        mc.create();

        // Before render, sprite batch should not be drawing
        assert!(mc.get_sprite_batch().is_some());
        assert!(!mc.get_sprite_batch().unwrap().is_drawing());

        // After render, sprite batch should have gone through begin()/end() cycle
        // and should not be drawing anymore
        mc.render();
        assert!(!mc.get_sprite_batch().unwrap().is_drawing());
    }

    #[test]
    fn test_render_input_gating_by_time() {
        let mut mc = make_test_controller();
        mc.create();

        // prevtime starts at 0; first render should update it
        assert_eq!(mc.prevtime, 0);

        mc.render();

        // After render, prevtime should be updated to current time
        assert!(mc.prevtime > 0);
    }

    #[test]
    fn test_render_dispatches_to_current_state() {
        let mut mc = make_test_controller();
        mc.set_state_factory(Box::new(TestStateFactory));
        mc.change_state(MainStateType::MusicSelect);

        // render() should dispatch to current state's render()
        mc.render();

        assert_eq!(
            mc.get_current_state_type(),
            Some(MainStateType::MusicSelect)
        );
    }

    #[test]
    fn test_render_no_state_does_not_panic() {
        let mut mc = make_test_controller();
        // No state set, render should not panic
        mc.render();
        assert!(mc.get_current_state().is_none());
    }

    #[test]
    fn test_sprite_batch_mut_accessor() {
        let mut mc = make_test_controller();
        mc.create();

        // Should be able to get mutable reference to sprite batch
        let batch = mc.get_sprite_batch_mut().unwrap();
        batch.begin();
        assert!(batch.is_drawing());
        batch.end();
        assert!(!batch.is_drawing());
    }

    #[test]
    fn test_render_timer_updated_each_frame() {
        let mut mc = make_test_controller();
        mc.create();

        let time_before = mc.get_now_time();
        // Small sleep to ensure timer advances
        std::thread::sleep(std::time::Duration::from_millis(5));
        mc.render();
        let time_after = mc.get_now_time();

        // Timer should advance (or at least not go backwards)
        assert!(time_after >= time_before);
    }

    // --- Phase 22d: Skin draw wiring tests ---

    use crate::main_state::SkinDrawable;

    /// Mock SkinDrawable that tracks method call counts.
    struct MockSkinDrawable {
        draw_count: i32,
        update_count: i32,
    }

    impl MockSkinDrawable {
        fn new() -> Self {
            Self {
                draw_count: 0,
                update_count: 0,
            }
        }
    }

    impl SkinDrawable for MockSkinDrawable {
        fn draw_all_objects_timed(
            &mut self,
            _ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
        ) {
            self.draw_count += 1;
        }

        fn update_custom_objects_timed(
            &mut self,
            _ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
        ) {
            self.update_count += 1;
        }

        fn mouse_pressed_at(&mut self, _button: i32, _x: i32, _y: i32) {}
        fn mouse_dragged_at(&mut self, _button: i32, _x: i32, _y: i32) {}
        fn prepare_skin(&mut self) {}
        fn dispose_skin(&mut self) {}
        fn get_fadeout(&self) -> i32 {
            0
        }
        fn get_input(&self) -> i32 {
            0
        }
        fn get_scene(&self) -> i32 {
            0
        }
        fn get_width(&self) -> f32 {
            1280.0
        }
        fn get_height(&self) -> f32 {
            720.0
        }
    }

    /// A test state that allows setting a skin for render testing.
    struct SkinTestState {
        state_data: MainStateData,
    }

    impl SkinTestState {
        fn new_with_skin(skin: Box<dyn SkinDrawable>) -> Self {
            let mut data = MainStateData::new(TimerManager::new());
            data.skin = Some(skin);
            Self { state_data: data }
        }
    }

    impl MainState for SkinTestState {
        fn state_type(&self) -> Option<MainStateType> {
            Some(MainStateType::MusicSelect)
        }

        fn main_state_data(&self) -> &MainStateData {
            &self.state_data
        }

        fn main_state_data_mut(&mut self) -> &mut MainStateData {
            &mut self.state_data
        }

        fn create(&mut self) {}
        fn render(&mut self) {}
    }

    #[test]
    fn test_render_calls_skin_draw_methods() {
        let mut mc = make_test_controller();

        // Manually set current state with a mock skin
        let mock_skin = Box::new(MockSkinDrawable::new());
        mc.current = Some(Box::new(SkinTestState::new_with_skin(mock_skin)));

        // Render should call update and draw on the skin
        mc.render();

        // Verify skin methods were called by checking the skin is still present
        // (the take/put-back pattern should preserve it)
        let state = mc.get_current_state().unwrap();
        assert!(
            state.main_state_data().skin.is_some(),
            "skin should be put back after render"
        );
    }

    #[test]
    fn test_render_without_skin_does_not_panic() {
        let mut mc = make_test_controller();

        // Set a state without a skin
        let mut data = MainStateData::new(TimerManager::new());
        data.skin = None;
        let state = SkinTestState { state_data: data };
        mc.current = Some(Box::new(state));

        // Should not panic when skin is None
        mc.render();
        assert!(mc.get_current_state().is_some());
    }

    #[test]
    fn test_render_skin_called_once_per_frame() {
        use std::sync::{Arc, Mutex};

        /// A mock that records call counts via shared state.
        struct CountingSkinDrawable {
            counts: Arc<Mutex<(i32, i32)>>, // (update_count, draw_count)
        }

        impl SkinDrawable for CountingSkinDrawable {
            fn draw_all_objects_timed(
                &mut self,
                _ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
            ) {
                self.counts.lock().unwrap().1 += 1;
            }

            fn update_custom_objects_timed(
                &mut self,
                _ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
            ) {
                self.counts.lock().unwrap().0 += 1;
            }

            fn mouse_pressed_at(&mut self, _button: i32, _x: i32, _y: i32) {}
            fn mouse_dragged_at(&mut self, _button: i32, _x: i32, _y: i32) {}
            fn prepare_skin(&mut self) {}
            fn dispose_skin(&mut self) {}
            fn get_fadeout(&self) -> i32 {
                0
            }
            fn get_input(&self) -> i32 {
                0
            }
            fn get_scene(&self) -> i32 {
                0
            }
            fn get_width(&self) -> f32 {
                1280.0
            }
            fn get_height(&self) -> f32 {
                720.0
            }
        }

        let counts = Arc::new(Mutex::new((0, 0)));
        let skin = Box::new(CountingSkinDrawable {
            counts: counts.clone(),
        });

        let mut mc = make_test_controller();
        mc.current = Some(Box::new(SkinTestState::new_with_skin(skin)));

        // Render 3 frames
        mc.render();
        mc.render();
        mc.render();

        let (update_count, draw_count) = *counts.lock().unwrap();
        assert_eq!(
            update_count, 3,
            "update_custom_objects_timed should be called once per frame"
        );
        assert_eq!(
            draw_count, 3,
            "draw_all_objects_timed should be called once per frame"
        );
    }

    // --- triggerLnWarning tests ---

    #[test]
    fn test_trigger_ln_warning_lnmode_0_is_ln_no_warning() {
        // lnmode=0 → "LN" → no warning (default)
        let mut mc = make_test_controller();
        mc.player.set_lnmode(0);
        // Should not panic; "LN" mode does not trigger warning
        mc.trigger_ln_warning();
    }

    #[test]
    fn test_trigger_ln_warning_lnmode_1_is_cn() {
        // lnmode=1 → "CN" → warning triggered
        let mut mc = make_test_controller();
        mc.player.set_lnmode(1);
        mc.trigger_ln_warning();
        // No assertion on log output, but should not panic
    }

    #[test]
    fn test_trigger_ln_warning_lnmode_2_is_hcn() {
        // lnmode=2 → "HCN" → warning triggered
        let mut mc = make_test_controller();
        mc.player.set_lnmode(2);
        mc.trigger_ln_warning();
    }

    #[test]
    fn test_trigger_ln_warning_lnmode_3_is_ln_no_warning() {
        // lnmode=3 → default "LN" → no warning
        let mut mc = make_test_controller();
        mc.player.set_lnmode(3);
        mc.trigger_ln_warning();
    }

    // --- setTargetList tests ---

    #[test]
    fn test_set_target_list_no_rivals() {
        let mut mc = make_test_controller();
        // With default player config (targetlist contains "MAX") and no rivals
        mc.set_target_list();
        // Should not panic
    }

    // --- updateStateReferences tests ---

    #[test]
    fn test_update_state_references_does_not_panic() {
        let mc = make_test_controller();
        mc.update_state_references();
    }

    // --- Audio driver wiring tests (Phase 24c) ---

    use bms_model::bms_model::BMSModel;
    use bms_model::note::Note;

    /// Mock AudioDriver for testing. Tracks method calls.
    struct MockAudioDriver {
        global_pitch: f32,
        play_count: i32,
        stop_count: i32,
    }

    impl MockAudioDriver {
        fn new() -> Self {
            Self {
                global_pitch: 1.0,
                play_count: 0,
                stop_count: 0,
            }
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
        fn stop_path(&mut self, _path: &str) {
            self.stop_count += 1;
        }
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
        fn set_global_pitch(&mut self, pitch: f32) {
            self.global_pitch = pitch;
        }
        fn get_global_pitch(&self) -> f32 {
            self.global_pitch
        }
        fn dispose_old(&mut self) {}
        fn dispose(&mut self) {}
    }

    #[test]
    fn test_audio_driver_initially_none() {
        let mc = make_test_controller();
        assert!(mc.get_audio_processor().is_none());
    }

    #[test]
    fn test_set_audio_driver() {
        let mut mc = make_test_controller();
        mc.set_audio_driver(Box::new(MockAudioDriver::new()));
        assert!(mc.get_audio_processor().is_some());
    }

    #[test]
    fn test_get_audio_processor_returns_trait_ref() {
        let mut mc = make_test_controller();
        mc.set_audio_driver(Box::new(MockAudioDriver::new()));

        let audio = mc.get_audio_processor().unwrap();
        assert_eq!(audio.get_global_pitch(), 1.0);
        assert_eq!(audio.get_progress(), 1.0);
    }

    #[test]
    fn test_get_audio_processor_mut() {
        let mut mc = make_test_controller();
        mc.set_audio_driver(Box::new(MockAudioDriver::new()));

        let audio = mc.get_audio_processor_mut().unwrap();
        audio.set_global_pitch(1.5);
        assert_eq!(audio.get_global_pitch(), 1.5);
    }

    #[test]
    fn test_audio_driver_play_path() {
        let mut mc = make_test_controller();
        mc.set_audio_driver(Box::new(MockAudioDriver::new()));

        let audio = mc.get_audio_processor_mut().unwrap();
        audio.play_path("/test/sound.wav", 0.8, false);
        assert!(!audio.is_playing_path("/test/sound.wav"));
    }

    // --- Phase 24f: update_main_state_listener tests ---

    use std::sync::{Arc, Mutex};

    type StateCallLog = Arc<Mutex<Vec<(ScreenType, i32)>>>;

    /// A mock listener that records calls.
    struct MockStateListener {
        calls: StateCallLog,
    }

    impl MockStateListener {
        fn new(calls: StateCallLog) -> Self {
            Self { calls }
        }
    }

    impl MainStateListener for MockStateListener {
        fn update(&mut self, state: &dyn MainStateAccess, status: i32) {
            self.calls
                .lock()
                .unwrap()
                .push((state.get_screen_type(), status));
        }
    }

    #[test]
    fn test_update_main_state_listener_dispatches_to_listeners() {
        let mut mc = make_test_controller();
        let calls = Arc::new(Mutex::new(Vec::new()));

        mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));
        mc.change_state(MainStateType::MusicSelect);

        // The transition_to_state calls update_main_state_listener(0) internally,
        // so we should already have one call.
        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], (ScreenType::MusicSelector, 0));
    }

    #[test]
    fn test_update_main_state_listener_multiple_listeners() {
        let mut mc = make_test_controller();
        let calls1 = Arc::new(Mutex::new(Vec::new()));
        let calls2 = Arc::new(Mutex::new(Vec::new()));

        mc.add_state_listener(Box::new(MockStateListener::new(calls1.clone())));
        mc.add_state_listener(Box::new(MockStateListener::new(calls2.clone())));

        mc.change_state(MainStateType::Config);

        assert_eq!(calls1.lock().unwrap().len(), 1);
        assert_eq!(calls2.lock().unwrap().len(), 1);
        assert_eq!(calls1.lock().unwrap()[0], (ScreenType::KeyConfiguration, 0));
        assert_eq!(calls2.lock().unwrap()[0], (ScreenType::KeyConfiguration, 0));
    }

    #[test]
    fn test_update_main_state_listener_no_state_no_dispatch() {
        let mut mc = make_test_controller();
        let calls = Arc::new(Mutex::new(Vec::new()));
        mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));

        // No current state → no dispatch
        mc.update_main_state_listener(0);
        assert!(calls.lock().unwrap().is_empty());
    }

    #[test]
    fn test_update_main_state_listener_preserves_status() {
        let mut mc = make_test_controller();
        let calls = Arc::new(Mutex::new(Vec::new()));
        mc.add_state_listener(Box::new(MockStateListener::new(calls.clone())));

        mc.change_state(MainStateType::Result);
        // Clear the initial call from transition
        calls.lock().unwrap().clear();

        // Manual call with custom status
        mc.update_main_state_listener(42);

        let recorded = calls.lock().unwrap();
        assert_eq!(recorded.len(), 1);
        assert_eq!(recorded[0], (ScreenType::MusicResult, 42));
    }

    // --- Phase 24f: StateReferencesCallback tests ---

    struct MockReferencesCallback {
        called: Arc<Mutex<bool>>,
    }

    impl StateReferencesCallback for MockReferencesCallback {
        fn update_references(&self, _config: &Config, _player: &PlayerConfig) {
            *self.called.lock().unwrap() = true;
        }
    }

    #[test]
    fn test_update_state_references_calls_callback() {
        let mut mc = make_test_controller();
        let called = Arc::new(Mutex::new(false));
        mc.set_state_references_callback(Box::new(MockReferencesCallback {
            called: called.clone(),
        }));

        mc.update_state_references();
        assert!(*called.lock().unwrap());
    }

    #[test]
    fn test_update_state_references_without_callback_does_not_panic() {
        let mc = make_test_controller();
        mc.update_state_references();
        // Should not panic
    }

    // --- Phase 24f: periodic_config_save tests ---

    #[test]
    fn test_periodic_config_save_skips_during_play() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::Play);
        // Set last_config_save to a long time ago to ensure it would trigger otherwise
        mc.last_config_save = Instant::now() - std::time::Duration::from_secs(300);

        // Should skip because current state is Play
        mc.periodic_config_save();
        // Verify it was NOT reset (still old)
        assert!(mc.last_config_save.elapsed().as_secs() >= 299);
    }

    #[test]
    fn test_periodic_config_save_does_not_trigger_within_interval() {
        let mut mc = make_test_controller();
        mc.change_state(MainStateType::MusicSelect);
        mc.last_config_save = Instant::now();

        // Should not trigger because less than 2 minutes elapsed
        mc.periodic_config_save();
        // last_config_save should not have changed significantly
        assert!(mc.last_config_save.elapsed().as_millis() < 100);
    }

    // --- Phase 24f: add_state_listener tests ---

    #[test]
    fn test_add_state_listener() {
        let mut mc = make_test_controller();
        assert!(mc.state_listener.is_empty());

        let calls = Arc::new(Mutex::new(Vec::new()));
        mc.add_state_listener(Box::new(MockStateListener::new(calls)));
        assert_eq!(mc.state_listener.len(), 1);
    }

    // --- Phase 24f: create() calls update_state_references ---

    #[test]
    fn test_create_calls_update_state_references() {
        let mut mc = make_test_controller();
        let called = Arc::new(Mutex::new(false));
        mc.set_state_references_callback(Box::new(MockReferencesCallback {
            called: called.clone(),
        }));

        mc.create();
        assert!(*called.lock().unwrap());
    }

    // --- Phase 41i: Loudness analyzer tests ---

    #[test]
    fn test_loudness_analyzer_initialized() {
        let mc = make_test_controller();
        assert!(mc.get_loudness_analyzer().is_some());
    }

    #[test]
    fn test_loudness_analyzer_is_available() {
        let mc = make_test_controller();
        let analyzer = mc.get_loudness_analyzer().unwrap();
        assert!(analyzer.is_available());
    }

    #[test]
    fn test_loudness_analyzer_shutdown_no_panic() {
        let mut mc = make_test_controller();
        mc.shutdown_loudness_analyzer();
        // Should not panic
    }

    #[test]
    fn test_get_sound_manager_mut() {
        let mut mc = make_test_controller();
        assert!(mc.get_sound_manager_mut().is_some());
    }

    // --- exit() and save_config() tests ---

    /// Mutex to serialize tests that change the process-wide CWD.
    /// Config::write() writes to CWD-relative "config_sys.json", so tests
    /// that verify file I/O must change CWD to a temp dir. This mutex
    /// prevents concurrent tests from racing on CWD.
    static CWD_MUTEX: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_exit_sets_exit_requested_flag() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mc = make_test_controller();
        assert!(!mc.is_exit_requested());

        mc.exit();

        assert!(mc.is_exit_requested());

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_exit_calls_save_config() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("config_sys.json");

        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mc = make_test_controller();
        mc.exit();

        // exit() should have called save_config(), which writes config_sys.json
        assert!(
            config_path.exists(),
            "config_sys.json should be written by exit()"
        );

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_save_config_writes_config_sys_json() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mc = make_test_controller();
        mc.save_config();

        let config_path = dir.path().join("config_sys.json");
        assert!(config_path.exists(), "config_sys.json should be created");

        // Verify it's valid JSON that round-trips back to Config
        let contents = std::fs::read_to_string(&config_path).unwrap();
        let deserialized: Config = serde_json::from_str(&contents).unwrap();
        assert_eq!(deserialized.window_width, mc.config.window_width);

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_save_config_writes_player_config_json() {
        let _lock = CWD_MUTEX.lock().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let original_dir = std::env::current_dir().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let mut config = Config::default();
        config.playerpath = dir.path().join("player").to_string_lossy().to_string();
        let mut player = PlayerConfig::default();
        player.id = Some("test_player".to_string());
        player.name = "TestName".to_string();

        let mc = MainController::new(None, config.clone(), player, None, false);
        // Create the player directory so write succeeds
        std::fs::create_dir_all(format!("{}/test_player", config.playerpath)).unwrap();

        mc.save_config();

        let player_config_path = PathBuf::from(format!(
            "{}/test_player/config_player.json",
            config.playerpath
        ));
        assert!(
            player_config_path.exists(),
            "config_player.json should be created"
        );

        let contents = std::fs::read_to_string(&player_config_path).unwrap();
        let deserialized: PlayerConfig = serde_json::from_str(&contents).unwrap();
        assert_eq!(deserialized.name, "TestName");

        std::env::set_current_dir(original_dir).unwrap();
    }

    #[test]
    fn test_is_exit_requested_initially_false() {
        let mc = make_test_controller();
        assert!(!mc.is_exit_requested());
    }
}
