pub(crate) use std::path::PathBuf;
pub(crate) use std::sync::atomic::{AtomicBool, Ordering};
pub(crate) use std::time::Instant;

pub(crate) use log::info;

pub(crate) use crate::audio::audio_system::AudioSystem;
pub(crate) use crate::imgui_notify::ImGuiNotify;
pub(crate) use crate::main_state_access::MainStateAccess;
pub(crate) use crate::ranking_data_cache_access::RankingDataCacheAccess;
pub(crate) use crate::skin::screen_type::ScreenType;
pub(crate) use crate::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;
pub(crate) use crate::song_information_db::SongInformationDb;

pub(crate) use crate::core::app_context::GameContext;
pub(crate) use crate::core::bms_player_mode::BMSPlayerMode;
pub(crate) use crate::core::config::Config;
pub(crate) use crate::core::ir_config::IRConfig;
pub(crate) use crate::core::main_state::{MainStateType, StateTransition};
#[allow(deprecated)]
pub(crate) use crate::core::main_state_listener::MainStateListener;
pub(crate) use crate::core::performance_metrics::PerformanceMetrics;
pub(crate) use crate::core::play_data_accessor::PlayDataAccessor;
pub(crate) use crate::core::player_config::PlayerConfig;
pub(crate) use crate::core::player_resource::PlayerResource;
pub(crate) use crate::core::rival_data_accessor::RivalDataAccessor;
pub(crate) use crate::core::sprite_batch_helper::{SpriteBatch, SpriteBatchHelper};
pub(crate) use crate::core::system_sound_manager::SystemSoundManager;
pub(crate) use crate::core::timer_manager::TimerManager;
pub(crate) use crate::core::version;
pub(crate) use crate::game_screen::GameScreen;

/// Function pointer type for creating concrete state instances.
///
/// Because the concrete state types (MusicSelector, BMSPlayer, etc.) live in separate crates
/// that depend on beatoraja-core, core cannot import them directly. Instead, a higher-level
/// crate (e.g. beatoraja-launcher) provides a concrete StateCreator closure that
/// knows how to create each state type.
///
/// Translated from: MainController.initializeStates() + createBMSPlayerState()
pub type StateCreator =
    Box<dyn Fn(MainStateType, &mut MainController) -> Option<StateCreateResult> + Send>;

/// Result from `StateCreator` containing the state and optional
/// metadata that `MainController::change_state` should apply after creation.
pub struct StateCreateResult {
    pub state: crate::game_screen::GameScreen,
    /// Target score data to set on PlayerResource (for result screen access).
    /// Java: resource.setTargetScoreData(targetScore)
    pub target_score: Option<crate::skin::score_data::ScoreData>,
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
    /// Receives the controller reference, player config, and command queue
    /// for wiring modmenu stubs.
    fn update_references(
        &self,
        config: &Config,
        player: &PlayerConfig,
        commands: &std::sync::Arc<std::sync::Mutex<Vec<crate::core::command::Command>>>,
    );
}

/// Re-export SkinOffset from rubato-types (single source of truth for the runtime type).
pub use crate::skin::skin_offset::SkinOffset;

/// SkinProperty constants
pub const OFFSET_MAX: usize = 255;

/// IRStatus - holds IR connection state
pub struct IRStatus {
    pub config: IRConfig,
    /// IR rival provider (trait bridge for core→ir rival/score operations)
    pub rival_provider: Option<Box<dyn crate::ir_rival_provider::IRRivalProvider>>,
    /// IR connection. Java: IRStatus.connection
    pub connection:
        Option<std::sync::Arc<dyn crate::ir::ir_connection::IRConnection + Send + Sync>>,
    /// IR player data. Java: IRStatus.player
    pub player_data: Option<crate::ir::ir_player_data::IRPlayerData>,
}

// IRSendStatus stub removed — replaced by Box<dyn IrResendService> (brs-zd2)

// RankingDataCache stub removed — replaced by Box<dyn RankingDataCacheAccess> (brs-2v7)

// SongInformationAccessor: stub replaced by SongInformationDb trait (Phase 27c)

// ObsListener/ObsWsClient replaced by Box<dyn ObsAccess>
// ImGuiRenderer stub replaced by Box<dyn ImGuiAccess> (Phase 4)

// MusicDownloadProcessor stub removed — replaced by Box<dyn MusicDownloadAccess> (brs-4ls)

// HttpDownloadProcessor stub removed — replaced by Box<dyn HttpDownloadSubmitter> (brs-4ls)

// StreamController stub removed — replaced by Box<dyn StreamControllerAccess> (brs-36u)

pub(crate) use crate::input::bms_player_input_processor::BMSPlayerInputProcessor;
pub(crate) use crate::input::key_command::KeyCommand;

/// Adapter that bridges `MainState` → `MainStateAccess` for external listeners.
///
/// External listeners (DiscordListener, ObsListener) receive `&dyn MainStateAccess`
/// which provides screen type, player resource, and config without depending on
/// beatoraja-core's internal `MainState` trait.
struct StateAccessAdapter<'a> {
    screen_type: ScreenType,
    resource: Option<&'a PlayerResource>,
    config: &'a Config,
}

impl MainStateAccess for StateAccessAdapter<'_> {
    fn screen_type(&self) -> ScreenType {
        self.screen_type
    }

    fn config(&self) -> &Config {
        self.config
    }

    fn songdata(&self) -> Option<&crate::skin::song_data::SongData> {
        self.resource.and_then(|r| r.songdata())
    }

    fn replay_data(&self) -> Option<&crate::skin::replay_data::ReplayData> {
        self.resource.and_then(|r| r.replay_data())
    }

    fn course_data(&self) -> Option<&crate::skin::course_data::CourseData> {
        self.resource.and_then(|r| r.course_data())
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
    pub imgui: Option<Box<dyn crate::imgui_access::ImGuiAccess>>,
    pub ir_resend_service: Option<Box<dyn crate::ir_resend_service::IrResendService>>,
    pub obs_client: Option<Box<dyn crate::obs_access::ObsAccess>>,
    pub download: Option<Box<dyn crate::music_download_access::MusicDownloadAccess>>,
    pub http_download_processor:
        Option<std::sync::Arc<dyn crate::http_download_submitter::HttpDownloadSubmitter>>,
    pub stream_controller: Option<Box<dyn crate::stream_controller_access::StreamControllerAccess>>,
}

/// MainController - root class of the application
#[allow(dead_code)]
pub struct MainController {
    /// Shared application context (config, audio, input, timer, database, etc.).
    pub(crate) ctx: GameContext,

    auto: Option<BMSPlayerMode>,
    song_updated: bool,

    /// Player resource
    resource: Option<PlayerResource>,

    /// Current state
    ///
    /// Translated from: MainController.current (MainState)
    current: Option<crate::game_screen::GameScreen>,

    /// State creator for creating concrete state instances.
    /// Set by the application entry point (e.g. launcher) before state transitions.
    state_factory: Option<StateCreator>,

    /// SpriteBatch (LibGDX)
    sprite: Option<SpriteBatch>,

    /// BMS file for single-song play
    bmsfile: Option<PathBuf>,

    /// State listeners (legacy trait-based, kept for backward compatibility during migration)
    #[allow(deprecated)]
    state_listener: Vec<Box<dyn MainStateListener>>,

    /// Channel-based event senders for external listeners and test harnesses.
    event_senders: Vec<std::sync::mpsc::SyncSender<crate::skin::app_event::AppEvent>>,

    /// Shared music selector.
    /// Java shares the same MusicSelector between StreamController and MusicSelect state.
    /// The launcher stores this so the StateCreator can reuse it instead of creating a new one.
    shared_music_selector:
        Option<std::sync::Arc<std::sync::Mutex<crate::select::music_selector::MusicSelector>>>,

    /// Callback for updating cross-state references (modmenu wiring).
    /// Set by the launcher to wire SkinMenu/SongManagerMenu.
    state_references_callback: Option<Box<dyn StateReferencesCallback>>,

    /// JoinHandles for background threads (song update, table update, etc.).
    /// Joined on dispose() to ensure clean shutdown and release of DB handles.
    background_threads: Vec<std::thread::JoinHandle<()>>,

    /// Optional event log for state machine observability (E2E testing).
    /// When set, state transition / lifecycle / handoff events are pushed here.
    state_event_log:
        Option<std::sync::Arc<std::sync::Mutex<Vec<crate::skin::state_event::StateEvent>>>>,

    /// Cached decide skin to avoid reloading on every Select -> Decide transition.
    /// The decide skin is large (3+ seconds to load) but doesn't change between songs,
    /// so we cache it after the first load and reuse on subsequent transitions.
    decide_skin_cache: Option<Box<dyn crate::core::main_state::SkinDrawable>>,

    /// Background thread pre-loading the play skin during the decide screen.
    /// Tuple: (skin_type_id, join_handle). Consumed when creating the Play state.
    preloaded_play_skin: Option<(
        i32,
        std::thread::JoinHandle<Option<crate::skin::types::skin::Skin>>,
    )>,
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
mod state_creation;
mod state_machine;
mod trait_impls;
mod utilities;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests;
