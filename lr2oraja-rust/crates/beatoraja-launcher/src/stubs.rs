// Stubs for types not yet available from other crates or external libraries
// Real platform implementations moved to platform.rs in Phase 25a.

use beatoraja_core::config::Config;
use beatoraja_core::main_loader::MainLoader as CoreMainLoader;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

// Re-export platform helpers so existing callers continue to work
pub use crate::platform::{
    DeviceInfo, EguiContext, MonitorInfo, copy_to_clipboard, get_monitors, get_port_audio_devices,
    open_folder_in_file_manager, open_url_in_browser, show_directory_chooser, show_file_chooser,
    update_monitors_from_winit,
};

// === MainLoader stubs ===

#[derive(Clone, Debug, Default)]
pub struct MainLoader;

impl MainLoader {
    /// Create and register a SQLiteSongDatabaseAccessor with the core MainLoader.
    ///
    /// Translated from: MainLoader.getScoreDatabaseAccessor()
    /// Java: if(songdb == null) { songdb = new SQLiteSongDatabaseAccessor(config.getSongpath(), config.getBmsroot()); }
    ///
    /// In Java this is lazily created on first access. In Rust, we eagerly create it
    /// and set it on the core MainLoader's global slot, which then passes it to MainController.
    pub fn init_score_database_accessor(config: &Config) {
        match SQLiteSongDatabaseAccessor::new(config.get_songpath(), config.get_bmsroot()) {
            Ok(accessor) => {
                CoreMainLoader::set_score_database_accessor(Box::new(accessor));
                log::info!(
                    "Song database accessor initialized: {}",
                    config.get_songpath()
                );
            }
            Err(e) => {
                log::error!("Failed to create song database accessor: {}", e);
            }
        }
    }

    pub fn get_version_checker() -> VersionChecker {
        VersionChecker::default()
    }

    pub fn get_available_display_mode() -> Vec<DisplayMode> {
        log::warn!("not yet implemented: MainLoader.get_available_display_mode");
        Vec::new()
    }

    pub fn get_desktop_display_mode() -> DisplayMode {
        log::warn!("not yet implemented: MainLoader.get_desktop_display_mode");
        DisplayMode::default()
    }

    /// Launch the game.
    ///
    /// Translated from: MainLoader.play() (launcher side)
    ///
    /// This creates the song database accessor and delegates to the core MainLoader.play().
    pub fn play(
        path: Option<&str>,
        mode: BMSPlayerMode,
        launcher: bool,
        config: &Config,
        player: &PlayerConfig,
        song_updated: bool,
    ) {
        // Initialize song database accessor before play
        Self::init_score_database_accessor(config);

        // Delegate to core MainLoader
        CoreMainLoader::play(
            path.map(std::path::PathBuf::from),
            Some(mode),
            launcher,
            Some(config.clone()),
            Some(player.clone()),
            song_updated,
        );
    }
}

#[derive(Clone, Debug, Default)]
pub struct VersionChecker {
    pub message: String,
    pub download_url: Option<String>,
}

impl VersionChecker {
    pub fn get_message(&self) -> &str {
        &self.message
    }

    pub fn get_download_url(&self) -> Option<&str> {
        self.download_url.as_deref()
    }
}

#[derive(Clone, Debug, Default)]
pub struct DisplayMode {
    pub width: i32,
    pub height: i32,
}

pub use beatoraja_core::bms_player_mode::BMSPlayerMode;

// === Version (re-exported from beatoraja-core) ===

pub use beatoraja_core::version::Version;

// === SongDatabaseUpdateListener stub ===

#[derive(Clone, Debug, Default)]
pub struct SongDatabaseUpdateListener {
    bms_files_count: i32,
    processed_bms_files_count: i32,
    new_bms_files_count: i32,
}

impl SongDatabaseUpdateListener {
    pub fn get_bms_files_count(&self) -> i32 {
        self.bms_files_count
    }

    pub fn get_processed_bms_files_count(&self) -> i32 {
        self.processed_bms_files_count
    }

    pub fn get_new_bms_files_count(&self) -> i32 {
        self.new_bms_files_count
    }
}

// === Twitter stubs — Twitter API not supported in Rust port ===

pub struct TwitterAuth;

impl TwitterAuth {
    pub fn start_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
    ) -> anyhow::Result<(String, String)> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }

    pub fn complete_pin_auth(
        _consumer_key: &str,
        _consumer_secret: &str,
        _request_token: &str,
        _request_secret: &str,
        _pin: &str,
    ) -> anyhow::Result<(String, String)> {
        anyhow::bail!(
            "Twitter API is not supported in Rust port (twitter4j has no Rust equivalent)"
        )
    }
}
