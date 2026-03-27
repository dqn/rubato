// PlayConfigurationView.java -> play_configuration_view.rs
// Mechanical line-by-line translation.

pub(crate) use std::sync::Arc;
pub(crate) use std::thread::JoinHandle;

pub(crate) use egui;
pub(crate) use log::{info, warn};

pub(crate) use crate::core::config::Config;
pub(crate) use crate::core::player_config::PlayerConfig;
pub(crate) use crate::song::md_processor::http_download_processor::DOWNLOAD_SOURCES;
pub(crate) use crate::song::song_database_update_listener::SongDatabaseUpdateListener as SongListener;
pub(crate) use crate::song::song_information_accessor::SongInformationAccessor;
pub(crate) use crate::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;
pub(crate) use bms::model::mode::Mode;

pub(crate) use crate::main_loader::{BMSPlayerMode, MainLoader};
pub(crate) use crate::version_checker::Version;
pub(crate) use crate::views::config::audio_configuration_view::AudioConfigurationView;
pub(crate) use crate::views::config::discord_configuration_view::DiscordConfigurationView;
pub(crate) use crate::views::config::input_configuration_view::InputConfigurationView;
pub(crate) use crate::views::config::ir_configuration_view::IRConfigurationView;
pub(crate) use crate::views::config::music_select_configuration_view::MusicSelectConfigurationView;
pub(crate) use crate::views::config::obs_configuration_view::ObsConfigurationView;
pub(crate) use crate::views::config::stream_editor_view::StreamEditorView;
pub(crate) use crate::views::config::trainer_view::TrainerView;
pub(crate) use crate::views::config::video_configuration_view::VideoConfigurationView;
pub(crate) use crate::views::editors::table_editor_view::TableEditorView;
pub(crate) use crate::views::resource_configuration_view::ResourceConfigurationView;
pub(crate) use crate::views::skin_configuration_view::SkinConfigurationView;

/// State of async BMS database loading.
///
/// Translated from: PlayConfigurationView.loadBMS() thread lifecycle.
/// Java uses two threads (progress UI + DB update). In Rust, egui polls
/// this state each frame to display progress.
#[derive(Debug)]
pub enum BmsLoadingState {
    /// No loading in progress.
    Idle,
    /// Background thread is running. Counters come from the shared listener.
    Loading {
        bms_files: i32,
        processed_files: i32,
        new_files: i32,
    },
    /// Loading finished successfully.
    Completed,
    /// Loading failed with an error message.
    Failed(String),
}

/// Handle to a background BMS loading thread.
///
/// Holds the shared `SongDatabaseUpdateListener` (atomic counters) and the
/// `JoinHandle` so the UI can poll progress and detect completion.
struct BmsLoadingHandle {
    listener: Arc<SongListener>,
    join_handle: JoinHandle<anyhow::Result<()>>,
}

/// PlayMode enum
/// Translated from PlayConfigurationView.PlayMode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(non_camel_case_types)]
pub enum PlayMode {
    BEAT_5K,
    BEAT_7K,
    BEAT_10K,
    BEAT_14K,
    POPN_9K,
    KEYBOARD_24K,
    KEYBOARD_24K_DOUBLE,
}

impl PlayMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            PlayMode::BEAT_5K => "5KEYS",
            PlayMode::BEAT_7K => "7KEYS",
            PlayMode::BEAT_10K => "10KEYS",
            PlayMode::BEAT_14K => "14KEYS",
            PlayMode::POPN_9K => "9KEYS",
            PlayMode::KEYBOARD_24K => "24KEYS",
            PlayMode::KEYBOARD_24K_DOUBLE => "24KEYS DOUBLE",
        }
    }

    pub fn to_mode(&self) -> Mode {
        match self {
            PlayMode::BEAT_5K => Mode::BEAT_5K,
            PlayMode::BEAT_7K => Mode::BEAT_7K,
            PlayMode::BEAT_10K => Mode::BEAT_10K,
            PlayMode::BEAT_14K => Mode::BEAT_14K,
            PlayMode::POPN_9K => Mode::POPN_9K,
            PlayMode::KEYBOARD_24K => Mode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE => Mode::KEYBOARD_24K_DOUBLE,
        }
    }

    pub fn values() -> Vec<PlayMode> {
        vec![
            PlayMode::BEAT_5K,
            PlayMode::BEAT_7K,
            PlayMode::BEAT_10K,
            PlayMode::BEAT_14K,
            PlayMode::POPN_9K,
            PlayMode::KEYBOARD_24K,
            PlayMode::KEYBOARD_24K_DOUBLE,
        ]
    }
}

impl std::fmt::Display for PlayMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.display_name())
    }
}

/// OptionListCell — translates the JavaFX ListCell<Integer>
/// In egui, we just store the label mapping.
#[derive(Clone, Debug)]
pub struct OptionListCell {
    pub strings: Vec<String>,
}

impl OptionListCell {
    pub fn new(strings: Vec<String>) -> Self {
        OptionListCell { strings }
    }

    pub fn text(&self, index: Option<i32>) -> String {
        if let Some(idx) = index
            && idx >= 0
            && (idx as usize) < self.strings.len()
        {
            return self.strings[idx as usize].clone();
        }
        String::new()
    }
}

/// Shared handle for a background version check: `(message, download_url)`.
type VersionCheckHandle = Arc<std::sync::Mutex<Option<(String, Option<String>)>>>;

/// Beatoraja configuration dialog
///
/// Translated from PlayConfigurationView.java
pub struct PlayConfigurationView {
    // UI fields (JavaFX widgets → egui state)
    pub newversion_text: String,
    pub newversion_url: Option<String>,
    /// Background version check result (message, download_url).
    pub pending_version_check: Option<VersionCheckHandle>,

    // Player selector
    pub players: Vec<String>,
    pub players_selected: Option<String>,
    pub playername: String,

    // Play config selector
    pub playconfig: Option<PlayMode>,

    // Hi-speed
    pub hispeed: f64,

    // Layout spacing (grid)
    pub lr2configuration_hgap: f64,
    pub lr2configuration_vgap: f64,
    pub lr2configurationassist_hgap: f64,
    pub lr2configurationassist_vgap: f64,

    // Fix hispeed
    pub fixhispeed: Option<i32>,
    pub gvalue: i32,
    pub enable_constant: bool,
    pub const_fadein_time: i32,
    pub hispeedmargin: f64,
    pub hispeedautoadjust: bool,

    // Score options
    pub scoreop: Option<i32>,
    pub scoreop2: Option<i32>,
    pub doubleop: Option<i32>,
    pub gaugeop: Option<i32>,
    pub lntype: Option<i32>,

    // Lane cover
    pub enable_lanecover: bool,
    pub lanecover: i32,
    pub lanecovermarginlow: i32,
    pub lanecovermarginhigh: i32,
    pub lanecoverswitchduration: i32,
    pub enable_lift: bool,
    pub lift: i32,
    pub enable_hidden: bool,
    pub hidden: i32,

    // Paths
    pub bgmpath: String,
    pub soundpath: String,

    // Timing
    pub notesdisplaytiming: i32,
    pub notesdisplaytimingautoadjust: bool,
    pub bpmguide: bool,
    pub gaugeautoshift: Option<i32>,
    pub bottomshiftablegauge: Option<i32>,

    // Custom judge
    pub customjudge: bool,
    pub njudgepg: i32,
    pub njudgegr: i32,
    pub njudgegd: i32,
    pub sjudgepg: i32,
    pub sjudgegr: i32,
    pub sjudgegd: i32,

    // Mine/scroll/LN modes
    pub minemode: Option<i32>,
    pub scrollmode: Option<i32>,
    pub longnotemode: Option<i32>,
    pub forcedcnendings: bool,
    pub longnoterate: f64,
    pub hranthresholdbpm: i32,
    pub seventoninepattern: Option<i32>,
    pub seventoninetype: Option<i32>,
    pub exitpressduration: i32,
    pub chartpreview: bool,
    pub guidese: bool,
    pub windowhold: bool,
    pub extranotedepth: i32,

    // Visual options
    pub judgeregion: bool,
    pub markprocessednote: bool,
    pub showhiddennote: bool,
    pub showpastnote: bool,
    pub target: Vec<String>,
    pub target_selected: Option<String>,

    // Judge algorithm
    pub judgealgorithm: Option<i32>,

    // Auto save replay
    pub autosavereplay1: Option<i32>,
    pub autosavereplay2: Option<i32>,
    pub autosavereplay3: Option<i32>,
    pub autosavereplay4: Option<i32>,

    // CIM
    pub usecim: bool,

    // IPFS
    pub enable_ipfs: bool,
    pub ipfsurl: String,

    // HTTP download
    pub enable_http: bool,
    pub http_download_source: Vec<String>,
    pub http_download_source_selected: Option<String>,
    pub default_download_url: String,
    pub override_download_url: String,

    // Clipboard screenshot
    pub clipboard_screenshot: bool,

    // ComboBox option labels
    pub score_options_labels: Vec<String>,
    pub double_options_labels: Vec<String>,
    pub seven_to_nine_pattern_labels: Vec<String>,
    pub seven_to_nine_type_labels: Vec<String>,
    pub gauge_options_labels: Vec<String>,
    pub fixhispeed_labels: Vec<String>,
    pub lntype_labels: Vec<String>,
    pub gaugeautoshift_labels: Vec<String>,
    pub bottomshiftablegauge_labels: Vec<String>,
    pub minemode_labels: Vec<String>,
    pub scrollmode_labels: Vec<String>,
    pub longnotemode_labels: Vec<String>,
    pub judgealgorithm_labels: Vec<String>,
    pub autosave_labels: Vec<String>,

    // Sub-controllers
    pub video_controller: VideoConfigurationView,
    pub audio_controller: AudioConfigurationView,
    pub input_controller: InputConfigurationView,
    pub resource_controller: ResourceConfigurationView,
    pub music_select_controller: MusicSelectConfigurationView,
    pub skin_controller: SkinConfigurationView,
    pub ir_controller: IRConfigurationView,
    pub table_controller: TableEditorView,
    pub stream_controller: StreamEditorView,
    pub discord_controller: DiscordConfigurationView,
    pub obs_controller: ObsConfigurationView,
    pub trainer_controller: TrainerView,

    // Internal state
    config: Option<Config>,
    player: Option<PlayerConfig>,
    loader: Option<MainLoader>,
    pub(crate) song_updated: bool,
    pc: Option<PlayMode>,
    /// Handle to the background BMS loading thread, if any.
    bms_loading_handle: Option<BmsLoadingHandle>,
    /// Cached terminal state after loading completes or fails.
    bms_loading_result: Option<Result<(), String>>,
    /// Handle to the background LR2 score import thread, if any.
    lr2_import_handle: Option<std::thread::JoinHandle<()>>,

    // Exit flag (replaces process::exit(0))
    pub exit_requested: bool,

    // Tab/panel disabled state
    pub player_panel_disabled: bool,
    pub video_tab_disabled: bool,
    pub audio_tab_disabled: bool,
    pub input_tab_disabled: bool,
    pub resource_tab_disabled: bool,
    pub option_tab_disabled: bool,
    pub other_tab_disabled: bool,
    pub ir_tab_disabled: bool,
    pub stream_tab_disabled: bool,
    pub discord_tab_disabled: bool,
    pub obs_tab_disabled: bool,
    pub control_panel_disabled: bool,
}

impl Default for PlayConfigurationView {
    fn default() -> Self {
        Self::new()
    }
}

mod bms_loading;
mod config_ops;
mod initialization;
mod render;

#[cfg(test)]
mod tests;
