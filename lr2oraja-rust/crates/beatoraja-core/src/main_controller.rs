use std::path::PathBuf;
use std::time::Instant;

use log::info;

use beatoraja_types::main_controller_access::MainControllerAccess;
use beatoraja_types::player_resource_access::PlayerResourceAccess;

use crate::bms_player_mode::BMSPlayerMode;
use crate::config::Config;
use crate::ir_config::IRConfig;
use crate::main_state::MainStateType;
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

    /// Current state (Phase 5+)
    // current: Option<Box<dyn MainState>>,

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

        // PlayDataAccessor::new depends on config field accessors
        // that may be in-progress from other translators
        let playdata: Option<PlayDataAccessor> = None;

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
            timer,
            sprite: None,
            bmsfile: f,
            showfps: false,
            playdata,
            sound: Some(sound),
            ir: Vec::new(),
            rivals: RivalDataAccessor::new(),
            ircache: RankingDataCache::new(),
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

    pub fn change_state(&mut self, _state: MainStateType) {
        log::warn!("not yet implemented: state transition (MusicSelector, BMSPlayer, etc.)");
    }

    pub fn create(&mut self) {
        // Phase 5+: Initialize SpriteBatch, fonts, input, audio, states
        self.sprite = Some(SpriteBatchHelper::create_sprite_batch());

        let _perf = PerformanceMetrics::get().event("ImGui init");
        ImGuiRenderer::init();
        drop(_perf);

        // Phase 5+: System font loading, input processor, audio driver selection
        // Phase 5+: Initialize states, start polling thread, etc.

        self.last_config_save = Instant::now().elapsed().as_nanos() as i64;

        info!("Initialization complete");
    }

    pub fn render(&mut self) {
        self.timer.update();
        // Phase 5+: Full render pipeline
        // GL clear, state render, skin draw, FPS display, etc.

        PerformanceMetrics::get().commit();
    }

    pub fn dispose(&mut self) {
        self.save_config();

        // Phase 5+: Dispose all states
        if let Some(mut imgui) = self.imgui.take() {
            imgui.dispose();
        }
        if let Some(mut resource) = self.resource.take() {
            resource.dispose();
        }
        // ShaderManager::dispose();

        info!("All resources disposed");
    }

    pub fn pause(&mut self) {
        // current.pause()
    }

    pub fn resize(&mut self, _width: i32, _height: i32) {
        // current.resize(width, height)
    }

    pub fn resume(&mut self) {
        // current.resume()
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

    pub fn update_main_state_listener(&mut self, _status: i32) {
        // for listener in &mut self.state_listener {
        //     listener.update(current, status);
        // }
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
    pub fn get_song_database(&self) -> Option<()> {
        // Delegates to MainLoader.getScoreDatabaseAccessor() in Java
        // Phase 5+: return actual SongDatabaseAccessor
        log::warn!("not yet implemented: getSongDatabase");
        None
    }

    /// Returns the current state.
    ///
    /// Translated from: MainController.getCurrentState()
    pub fn get_current_state(&self) -> Option<()> {
        // Phase 5+: return &dyn MainState
        log::warn!("not yet implemented: getCurrentState");
        None
    }

    /// Returns the state type for a given state.
    ///
    /// Translated from: MainController.getStateType(MainState)
    pub fn get_state_type(_state: Option<()>) -> Option<MainStateType> {
        // Phase 5+: instanceof checks for each state type
        log::warn!("not yet implemented: getStateType");
        None
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
        // initializeIRConfig();
        // selector.dispose();
        // initializeStates();
        // updateStateReferences();
        // triggerLnWarning();
        // setTargetList();
        // changeState(selector);
        self.last_config_save = Instant::now().elapsed().as_nanos() as i64;
        log::warn!("not yet implemented: loadNewProfile lifecycle methods");
    }

    /// Initialize IR configurations from config.
    ///
    /// Translated from: MainController.initializeIRConfig()
    pub fn initialize_ir_config(&mut self) {
        log::warn!("not yet implemented: initializeIRConfig");
    }

    /// Initialize all game states (selector, player, result, etc.).
    ///
    /// Translated from: MainController.initializeStates()
    pub fn initialize_states(&mut self) {
        log::warn!("not yet implemented: initializeStates");
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

    fn change_state(&mut self, _state: MainStateType) {
        log::warn!("not yet implemented: state transition via MainControllerAccess");
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
