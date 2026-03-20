pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::atomic::{AtomicBool, Ordering};
pub(crate) use std::time::Instant;

pub(crate) use log::info;

pub(crate) use rubato_audio::audio_driver::AudioDriver;
pub(crate) use rubato_types::imgui_notify::ImGuiNotify;
pub(crate) use rubato_types::main_controller_access::MainControllerAccess;
pub(crate) use rubato_types::main_state_access::MainStateAccess;
pub(crate) use rubato_types::player_resource_access::PlayerResourceAccess;
pub(crate) use rubato_types::player_resource_access::{MediaAccess, ReplayAccess, SongAccess};
pub(crate) use rubato_types::ranking_data_cache_access::RankingDataCacheAccess;
pub(crate) use rubato_types::screen_type::ScreenType;
pub(crate) use rubato_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;
pub(crate) use rubato_types::song_information_db::SongInformationDb;
pub(crate) use rubato_types::sound_type::SoundType;

pub(crate) use crate::bms_player_mode::BMSPlayerMode;
pub(crate) use crate::config::Config;
pub(crate) use crate::ir_config::IRConfig;
pub(crate) use crate::main_state::{MainState, MainStateType};
pub(crate) use crate::main_state_listener::MainStateListener;
pub(crate) use crate::performance_metrics::PerformanceMetrics;
pub(crate) use crate::play_data_accessor::PlayDataAccessor;
pub(crate) use crate::player_config::PlayerConfig;
pub(crate) use crate::player_resource::PlayerResource;
pub(crate) use crate::rival_data_accessor::RivalDataAccessor;
pub(crate) use crate::sprite_batch_helper::{SpriteBatch, SpriteBatchHelper};
pub(crate) use crate::system_sound_manager::SystemSoundManager;
pub(crate) use crate::timer_manager::TimerManager;
pub(crate) use crate::version;

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
    pub target_score: Option<rubato_types::score_data::ScoreData>,
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

/// Re-export SkinOffset from rubato-types (single source of truth for the runtime type).
pub use rubato_types::skin_offset::SkinOffset;

/// SkinProperty constants
pub const OFFSET_MAX: usize = 255;

/// IRStatus - holds IR connection state
pub struct IRStatus {
    pub config: IRConfig,
    /// IR rival provider (trait bridge for core→ir rival/score operations)
    pub rival_provider: Option<Box<dyn rubato_types::ir_rival_provider::IRRivalProvider>>,
    /// IR connection (type-erased). The concrete type is `Arc<dyn IRConnection + Send + Sync>`
    /// from beatoraja-ir. Stored as `dyn Any` because beatoraja-core cannot depend on beatoraja-ir.
    /// Java: IRStatus.connection
    pub connection: Option<Box<dyn std::any::Any + Send + Sync>>,
    /// IR player data (type-erased). The concrete type is `IRPlayerData` from beatoraja-ir.
    /// Stored as `dyn Any` because beatoraja-core cannot depend on beatoraja-ir.
    /// Java: IRStatus.player
    pub player_data: Option<Box<dyn std::any::Any + Send + Sync>>,
}

// IRSendStatus stub removed — replaced by Box<dyn IrResendService> (brs-zd2)

// RankingDataCache stub removed — replaced by Box<dyn RankingDataCacheAccess> (brs-2v7)

// SongInformationAccessor: stub replaced by SongInformationDb trait (Phase 27c)

// ObsListener/ObsWsClient replaced by Box<dyn ObsAccess>
// ImGuiRenderer stub replaced by Box<dyn ImGuiAccess> (Phase 4)

// MusicDownloadProcessor stub removed — replaced by Box<dyn MusicDownloadAccess> (brs-4ls)

// HttpDownloadProcessor stub removed — replaced by Box<dyn HttpDownloadSubmitter> (brs-4ls)

// StreamController stub removed — replaced by Box<dyn StreamControllerAccess> (brs-36u)

pub(crate) use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
pub(crate) use rubato_input::key_command::KeyCommand;

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
    fn screen_type(&self) -> ScreenType {
        self.screen_type
    }

    fn resource(&self) -> Option<&dyn PlayerResourceAccess> {
        self.resource
    }

    fn config(&self) -> &Config {
        self.config
    }
}

/// Timing and lifecycle state for the main loop.
pub struct LifecycleState {
    pub boottime: Instant,
    pub prevtime: i64,
    pub last_config_save: Instant,
    pub mouse_moved_time: i64,
    /// Override for the input gate time. When Some, render() uses this instead
    /// of SystemTime::now(). Used by test harnesses to ensure deterministic
    /// input processing. Consumed (taken) on each render() call.
    pub override_input_gate_time: Option<i64>,
}

impl LifecycleState {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            boottime: now,
            prevtime: 0,
            last_config_save: now,
            mouse_moved_time: 0,
            override_input_gate_time: None,
        }
    }
}

impl Default for LifecycleState {
    fn default() -> Self {
        Self::new()
    }
}

/// Database and data accessor state.
#[derive(Default)]
pub struct DatabaseState {
    pub playdata: Option<PlayDataAccessor>,
    pub songdb: Option<std::sync::Arc<dyn SongDatabaseAccessorTrait>>,
    pub infodb: Option<Box<dyn SongInformationDb>>,
    pub rivals: RivalDataAccessor,
    pub ircache: Option<Box<dyn RankingDataCacheAccess>>,
    pub ir: Vec<IRStatus>,
}

/// External integration state (ImGui, OBS, IR, downloads, streaming).
#[derive(Default)]
pub struct IntegrationState {
    pub imgui: Option<Box<dyn rubato_types::imgui_access::ImGuiAccess>>,
    pub ir_resend_service: Option<Box<dyn rubato_types::ir_resend_service::IrResendService>>,
    pub obs_client: Option<Box<dyn rubato_types::obs_access::ObsAccess>>,
    pub download: Option<Box<dyn rubato_types::music_download_access::MusicDownloadAccess>>,
    pub http_download_processor:
        Option<std::sync::Arc<dyn rubato_types::http_download_submitter::HttpDownloadSubmitter>>,
    pub stream_controller:
        Option<Box<dyn rubato_types::stream_controller_access::StreamControllerAccess>>,
}

/// MainController - root class of the application
#[allow(dead_code)]
pub struct MainController {
    pub config: Config,
    pub player: PlayerConfig,
    auto: Option<BMSPlayerMode>,
    song_updated: bool,

    /// Timing and lifecycle state.
    lifecycle: LifecycleState,

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

    /// Database and data accessor state.
    db: DatabaseState,

    /// System sound manager
    sound: Option<SystemSoundManager>,

    /// Offset array for skin
    offset: Vec<SkinOffset>,

    /// State listeners
    state_listener: Vec<Box<dyn MainStateListener>>,

    /// Deferred controller commands from state-facing access proxies.
    command_queue: rubato_types::main_controller_access::MainControllerCommandQueue,

    /// External integration state (ImGui, OBS, IR, downloads, streaming).
    pub integration: IntegrationState,

    /// Shared music selector (type-erased Arc<Mutex<MusicSelector>>).
    /// Java shares the same MusicSelector between StreamController and MusicSelect state.
    /// The launcher stores this so StateFactory can reuse it instead of creating a new one.
    shared_music_selector: Option<Box<dyn std::any::Any + Send>>,

    /// Callback for updating cross-state references (modmenu wiring).
    /// Set by the launcher to wire SkinMenu/SongManagerMenu.
    state_references_callback: Option<Box<dyn StateReferencesCallback>>,

    /// JoinHandles for background threads (song update, table update, etc.).
    /// Joined on dispose() to ensure clean shutdown and release of DB handles.
    background_threads: Vec<std::thread::JoinHandle<()>>,

    /// Exit requested flag.
    /// Uses AtomicBool because exit() takes &self (required by MainControllerAccess trait).
    ///
    /// Translated from: Gdx.app.exit() triggers LibGDX's ApplicationListener.dispose()
    exit_requested: AtomicBool,

    /// Debug flag
    pub debug: bool,

    /// Optional event log for state machine observability (E2E testing).
    /// When set, state transition / lifecycle / handoff events are pushed here.
    state_event_log:
        Option<std::sync::Arc<std::sync::Mutex<Vec<rubato_types::state_event::StateEvent>>>>,

    /// Loudness analyzer for volume normalization.
    ///
    /// Translated from: MainController.loudnessAnalyzer (BMSLoudnessAnalyzer)
    loudness_analyzer: Option<rubato_audio::bms_loudness_analyzer::BMSLoudnessAnalyzer>,
}

/// Offset count (SkinProperty.OFFSET_MAX + 1)
pub const OFFSET_COUNT: usize = OFFSET_MAX + 1;

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

mod accessors;
mod lifecycle;
mod state_machine;
mod trait_impls;
mod utilities;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;
