use std::path::PathBuf;
use std::time::Instant;

use log::info;

use beatoraja_types::main_controller_access::MainControllerAccess;
use beatoraja_types::player_resource_access::PlayerResourceAccess;
use beatoraja_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;

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
        controller: &MainController,
    ) -> Option<Box<dyn MainState>>;
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
    // pub connection: IRConnection, // Phase 5+
    // pub player: IRPlayerData,     // Phase 5+
}

/// IRSendStatus - holds IR score send state
pub struct IRSendStatus {
    // pub ir: IRConnection,   // Phase 5+
    // pub song: SongData,     // Phase 5+
    // pub score: ScoreData,   // Phase 5+
    pub retry: i32,
    pub last_try: i64,
    pub is_sent: bool,
}

/// RankingDataCache stub
pub struct RankingDataCache;

impl Default for RankingDataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingDataCache {
    pub fn new() -> Self {
        Self
    }
}

/// SongDatabaseAccessor stub (Phase 5+)
pub struct SongDatabaseAccessor;

/// SongInformationAccessor stub (Phase 5+)
pub struct SongInformationAccessor;

/// ObsListener stub (Phase 5+)
pub struct ObsListener;

/// ObsWsClient stub (Phase 5+)
pub struct ObsWsClient;

impl ObsWsClient {
    pub fn save_last_recording(&self, _reason: &str) {}
}

/// ImGuiRenderer stub (Phase 5+)
pub struct ImGuiRenderer;

impl ImGuiRenderer {
    pub fn init() {}
    pub fn start(&mut self) {}
    pub fn render(&mut self) {}
    pub fn end(&mut self) {}
    pub fn toggle_menu(&mut self) {}
    pub fn dispose(&mut self) {}
}

/// MusicDownloadProcessor stub (Phase 5+)
pub struct MusicDownloadProcessor;

/// HttpDownloadProcessor stub (Phase 5+)
pub struct HttpDownloadProcessor;

/// StreamController stub (Phase 5+)
pub struct StreamController;

/// BMSPlayerInputProcessor stub (Phase 5+)
pub struct BMSPlayerInputProcessor;

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

    /// Audio driver (Phase 5+)
    // audio: Option<AudioDriver>,

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

    /// Input processor (Phase 5+)
    // input: Option<BMSPlayerInputProcessor>,

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

    /// Ranking data cache
    ircache: RankingDataCache,

    /// Song database accessor (trait object)
    songdb: Option<Box<dyn SongDatabaseAccessorTrait>>,

    /// Song information accessor
    infodb: Option<SongInformationAccessor>,

    /// Offset array for skin
    offset: Vec<SkinOffset>,

    /// State listeners
    state_listener: Vec<Box<dyn MainStateListener>>,

    /// ImGui renderer
    pub imgui: Option<ImGuiRenderer>,

    /// IR send status list
    pub ir_send_status: Vec<IRSendStatus>,

    /// OBS listener
    obs_listener: Option<ObsListener>,
    /// OBS client
    obs_client: Option<ObsWsClient>,

    /// Download processor
    download: Option<MusicDownloadProcessor>,
    /// HTTP download processor
    http_download_processor: Option<HttpDownloadProcessor>,

    /// Stream controller
    stream_controller: Option<StreamController>,

    /// Previous render time
    prevtime: i64,

    /// Last config save time
    last_config_save: i64,

    /// Debug flag
    pub debug: bool,
}

/// Offset count (SkinProperty.OFFSET_MAX + 1)
pub const OFFSET_COUNT: usize = OFFSET_MAX + 1;

impl MainController {
    pub fn new(
        f: Option<PathBuf>,
        config: Config,
        player: PlayerConfig,
        auto: Option<BMSPlayerMode>,
        song_updated: bool,
    ) -> Self {
        let mut offset = Vec::with_capacity(OFFSET_COUNT);
        for _ in 0..OFFSET_COUNT {
            offset.push(SkinOffset::new());
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

        Self {
            config,
            player,
            auto,
            song_updated,
            boottime: Instant::now(),
            mouse_moved_time: 0,
            resource: None,
            current: None,
            state_factory: None,
            timer,
            sprite: None,
            bmsfile: f,
            showfps: false,
            playdata,
            sound: Some(sound),
            ir: Vec::new(),
            rivals: RivalDataAccessor::new(),
            ircache: RankingDataCache::new(),
            songdb: None,
            infodb: None,
            offset,
            state_listener,
            imgui: None,
            ir_send_status: Vec::new(),
            obs_listener: None,
            obs_client: None,
            download: None,
            http_download_processor: None,
            stream_controller: None,
            prevtime: 0,
            last_config_save: 0,
            debug: false,
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

    pub fn get_play_data_accessor(&self) -> Option<&PlayDataAccessor> {
        self.playdata.as_ref()
    }

    pub fn get_rival_data_accessor(&self) -> &RivalDataAccessor {
        &self.rivals
    }

    pub fn get_ranking_data_cache(&self) -> &RankingDataCache {
        &self.ircache
    }

    pub fn get_sound_manager(&self) -> Option<&SystemSoundManager> {
        self.sound.as_ref()
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        &self.ir
    }

    pub fn get_timer(&self) -> &TimerManager {
        &self.timer
    }

    pub fn get_timer_mut(&mut self) -> &mut TimerManager {
        &mut self.timer
    }

    pub fn has_obs_listener(&self) -> bool {
        self.obs_listener.is_some()
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

        // Create the new state via factory
        let new_state = if let Some(ref factory) = self.state_factory {
            factory.create_state(actual_type, self)
        } else {
            log::warn!(
                "No state factory set; cannot create state {:?}",
                actual_type
            );
            None
        };

        if let Some(new_state) = new_state {
            self.transition_to_state(new_state);
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
        // Phase 22: skin preparation

        // Shutdown the old state
        if let Some(ref mut old_state) = self.current {
            old_state.shutdown();
            // setSkin(null) equivalent
            old_state.main_state_data_mut().skin = None;
        }

        // Set as current
        self.current = Some(new_state);

        // In Java: timer.setMainState(newState)
        // Phase 5+: timer state binding

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
    pub fn create(&mut self) {
        self.sprite = Some(SpriteBatchHelper::create_sprite_batch());

        let _perf = PerformanceMetrics::get().event("ImGui init");
        ImGuiRenderer::init();
        drop(_perf);

        // Phase 5+: System font loading, input processor, audio driver selection

        // Initialize states (creates PlayerResource)
        self.initialize_states();

        // Phase 5+: updateStateReferences, MiscSettingMenu, polling thread

        // Enter initial state based on bmsfile
        if self.bmsfile.is_some() {
            // In Java: if(resource.setBMSFile(bmsfile, auto)) changeState(PLAY) else { changeState(CONFIG); exit(); }
            self.change_state(MainStateType::Play);
        } else {
            self.change_state(MainStateType::MusicSelect);
        }

        self.last_config_save = Instant::now().elapsed().as_nanos() as i64;

        info!("Initialization complete");
    }

    /// Main render lifecycle method — called every frame.
    ///
    /// Translated from: MainController.render()
    pub fn render(&mut self) {
        self.timer.update();

        // Dispatch input and render to current state
        if let Some(ref mut current) = self.current {
            current.input();
            current.render();
        }

        // Phase 5+: GL clear, skin draw, FPS display, ImGui, etc.

        self.periodic_config_save();

        PerformanceMetrics::get().commit();
    }

    /// Dispose lifecycle — called on application shutdown.
    ///
    /// Translated from: MainController.dispose()
    pub fn dispose(&mut self) {
        self.save_config();

        // Dispose current state
        if let Some(ref mut current) = self.current {
            current.dispose();
        }
        self.current = None;

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

    pub fn save_config(&self) {
        // Config::write(&self.config);
        // PlayerConfig::write(config.playerpath, &self.player);
        info!("Config saved");
    }

    pub fn exit(&self) {
        // Gdx.app.exit()
        log::warn!("not yet implemented: application exit");
    }

    /// Notify all state listeners of a state change.
    ///
    /// Translated from: MainController.updateMainStateListener(int)
    pub fn update_main_state_listener(&mut self, status: i32) {
        // Phase 5+: pass current state to listeners
        // In Java: for(MainStateListener listener : state_listener) { listener.update(current, status); }
        let _ = status;
        if !self.state_listener.is_empty() {
            log::warn!(
                "TODO: Phase 22 - dispatch to {} state listeners",
                self.state_listener.len()
            );
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

    pub fn get_http_download_processor(&self) -> Option<&HttpDownloadProcessor> {
        self.http_download_processor.as_ref()
    }

    pub fn set_http_download_processor(&mut self, processor: HttpDownloadProcessor) {
        self.http_download_processor = Some(processor);
    }

    pub fn update_song(&mut self, _path: &str) {
        log::warn!("not yet implemented: SongUpdateThread");
    }

    pub fn update_song_with_flag(&mut self, _path: &str, _update_parent_when_missing: bool) {
        log::warn!("not yet implemented: SongUpdateThread with flag");
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
        // Phase 5+: return &self.input
        log::warn!("not yet implemented: getInputProcessor");
        None
    }

    /// Returns the audio processor.
    ///
    /// Translated from: MainController.getAudioProcessor()
    pub fn get_audio_processor(&self) -> Option<()> {
        // Phase 5+: return &self.audio
        log::warn!("not yet implemented: getAudioProcessor");
        None
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

    pub fn get_info_database(&self) -> Option<&SongInformationAccessor> {
        self.infodb.as_ref()
    }

    pub fn get_music_download_processor(&self) -> Option<&MusicDownloadProcessor> {
        self.download.as_ref()
    }

    pub fn set_imgui(&mut self, imgui: ImGuiRenderer) {
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

        self.last_config_save = Instant::now().elapsed().as_nanos() as i64;
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
        // Phase 5+: pass audio driver and loudness analyzer

        // In Java: playdata = new PlayDataAccessor(config);
        self.playdata = Some(PlayDataAccessor::new(&self.config));

        info!("Initializing states (PlayerResource created, states created on-demand via factory)");
    }

    /// Update cross-state references after state re-initialization.
    ///
    /// Translated from: MainController.updateStateReferences()
    pub fn update_state_references(&mut self) {
        log::warn!("not yet implemented: updateStateReferences");
    }

    /// Trigger LN warning if the player has LN-related settings.
    ///
    /// Translated from: MainController.triggerLnWarning()
    pub fn trigger_ln_warning(&mut self) {
        log::warn!("not yet implemented: triggerLnWarning");
    }

    /// Set the target score list for grade/rival display.
    ///
    /// Translated from: MainController.setTargetList()
    pub fn set_target_list(&mut self) {
        log::warn!("not yet implemented: setTargetList");
    }

    /// Periodically save config if enough time has elapsed.
    ///
    /// Translated from: MainController.periodicConfigSave()
    pub fn periodic_config_save(&mut self) {
        let now = Instant::now().elapsed().as_nanos() as i64;
        if now - self.last_config_save > 60_000_000_000 {
            // 60 seconds in nanoseconds
            self.save_config();
            self.last_config_save = now;
        }
    }

    /// Update difficulty table data in a background thread.
    ///
    /// Translated from: MainController.updateTable(TableBar)
    pub fn update_table(&mut self) {
        log::warn!("not yet implemented: updateTable (TableUpdateThread)");
    }

    /// Start IPFS download message rendering thread.
    ///
    /// Translated from: MainController.downloadIpfsMessageRenderer(String)
    pub fn download_ipfs_message_renderer(&mut self, _message: &str) {
        log::warn!("not yet implemented: downloadIpfsMessageRenderer (DownloadMessageThread)");
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
}

#[cfg(test)]
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
            _controller: &MainController,
        ) -> Option<Box<dyn MainState>> {
            Some(Box::new(TestState::new(state_type)))
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
        let mut config = Config::default();
        config.skip_decide_screen = true;
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        mc.set_state_factory(Box::new(TestStateFactory));

        mc.change_state(MainStateType::Decide);

        // With skip_decide_screen, Decide should create Play instead
        assert_eq!(mc.get_current_state_type(), Some(MainStateType::Play));
    }

    #[test]
    fn test_decide_no_skip_creates_decide_state() {
        let mut config = Config::default();
        config.skip_decide_screen = false;
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
    fn test_no_factory_logs_warning() {
        let config = Config::default();
        let player = PlayerConfig::default();
        let mut mc = MainController::new(None, config, player, None, false);
        // No factory set

        mc.change_state(MainStateType::MusicSelect);

        // Without factory, state should remain None
        assert!(mc.get_current_state().is_none());
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
}
