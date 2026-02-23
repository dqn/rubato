// Stubs for types not yet available from other crates or external libraries
// Real platform implementations moved to platform.rs in Phase 25a.

use beatoraja_core::config::Config;
use beatoraja_core::main_loader::MainLoader as CoreMainLoader;
use beatoraja_core::player_config::PlayerConfig;
use beatoraja_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

// Re-export platform helpers so existing callers continue to work
pub use crate::platform::{
    DeviceInfo, EguiContext, MonitorInfo, copy_to_clipboard, get_cached_desktop_display_mode,
    get_cached_display_modes, get_monitors, get_port_audio_devices, open_folder_in_file_manager,
    open_url_in_browser, show_directory_chooser, show_file_chooser, update_monitors_from_winit,
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

    /// Get available display modes.
    ///
    /// Translated from: MainLoader.getAvailableDisplayMode()
    /// Java: Lwjgl3ApplicationConfiguration.getDisplayModes()
    ///
    /// Uses winit-cached display modes if available, falls back to common defaults.
    pub fn get_available_display_mode() -> Vec<DisplayMode> {
        let cached = get_cached_display_modes();
        if cached.is_empty() {
            // Fallback before event loop populates the cache
            vec![
                DisplayMode {
                    width: 1280,
                    height: 720,
                },
                DisplayMode {
                    width: 1920,
                    height: 1080,
                },
                DisplayMode {
                    width: 2560,
                    height: 1440,
                },
                DisplayMode {
                    width: 3840,
                    height: 2160,
                },
            ]
        } else {
            cached
                .into_iter()
                .map(|(w, h)| DisplayMode {
                    width: w as i32,
                    height: h as i32,
                })
                .collect()
        }
    }

    /// Get the desktop display mode.
    ///
    /// Translated from: MainLoader.getDesktopDisplayMode()
    /// Java: Lwjgl3ApplicationConfiguration.getDisplayMode()
    ///
    /// Uses winit-cached desktop mode if available, falls back to 1920x1080.
    pub fn get_desktop_display_mode() -> DisplayMode {
        let (w, h) = get_cached_desktop_display_mode();
        if w == 0 && h == 0 {
            // Fallback before event loop populates the cache
            DisplayMode {
                width: 1920,
                height: 1080,
            }
        } else {
            DisplayMode {
                width: w as i32,
                height: h as i32,
            }
        }
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

/// Version checker that queries GitHub API for the latest release.
///
/// Translated from: MainLoader.GithubVersionChecker
///
/// Lazily fetches version info from GitHub API on first access.
#[derive(Clone, Debug, Default)]
pub struct VersionChecker {
    message: Option<String>,
    download_url: Option<String>,
}

impl VersionChecker {
    pub fn get_message(&mut self) -> &str {
        if self.message.is_none() {
            self.get_information();
        }
        self.message.as_deref().unwrap_or("")
    }

    pub fn get_download_url(&mut self) -> Option<&str> {
        if self.message.is_none() {
            self.get_information();
        }
        self.download_url.as_deref()
    }

    fn get_information(&mut self) {
        let result = self.fetch_latest_release();
        match result {
            Ok((name, html_url)) => {
                let cmp = Version::compare_to_string(Some(&name));
                if cmp == 0 {
                    self.message = Some("Already on the latest version".to_string());
                } else if cmp == -1 {
                    self.message = Some(format!("Version [{}] is available to download", name));
                    self.download_url = Some(html_url);
                } else {
                    self.message = Some(format!(
                        "On Development Build for {}",
                        Version::get_version()
                    ));
                }
            }
            Err(e) => {
                log::warn!("Failed to fetch version info: {}", e);
                self.message = Some("Could not retrieve version information".to_string());
            }
        }
    }

    fn fetch_latest_release(&self) -> anyhow::Result<(String, String)> {
        let client = reqwest::blocking::Client::builder()
            .user_agent("beatoraja-rust")
            .build()?;
        let resp: serde_json::Value = client
            .get("https://api.github.com/repos/seraxis/lr2oraja-endlessdream/releases/latest")
            .send()?
            .json()?;
        let name = resp["name"].as_str().unwrap_or("").to_string();
        let html_url = resp["html_url"].as_str().unwrap_or("").to_string();
        Ok((name, html_url))
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

// === SongDatabaseUpdateListener ===

use std::sync::atomic::{AtomicI32, Ordering};

/// Listen to songdata.db update progress.
///
/// Translated from: bms.player.beatoraja.song.SongDatabaseUpdateListener
///
/// Java uses AtomicInteger for thread-safe counters. In Rust, we use AtomicI32.
pub struct SongDatabaseUpdateListener {
    bms_files: AtomicI32,
    processed_bms_files: AtomicI32,
    new_bms_files: AtomicI32,
}

impl Default for SongDatabaseUpdateListener {
    fn default() -> Self {
        SongDatabaseUpdateListener {
            bms_files: AtomicI32::new(0),
            processed_bms_files: AtomicI32::new(0),
            new_bms_files: AtomicI32::new(0),
        }
    }
}

impl SongDatabaseUpdateListener {
    pub fn add_bms_files_count(&self, count: i32) {
        self.bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_processed_bms_files_count(&self, count: i32) {
        self.processed_bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn add_new_bms_files_count(&self, count: i32) {
        self.new_bms_files.fetch_add(count, Ordering::Relaxed);
    }

    pub fn get_bms_files_count(&self) -> i32 {
        self.bms_files.load(Ordering::Relaxed)
    }

    pub fn get_processed_bms_files_count(&self) -> i32 {
        self.processed_bms_files.load(Ordering::Relaxed)
    }

    pub fn get_new_bms_files_count(&self) -> i32 {
        self.new_bms_files.load(Ordering::Relaxed)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn song_database_update_listener_default() {
        let listener = SongDatabaseUpdateListener::default();
        assert_eq!(listener.get_bms_files_count(), 0);
        assert_eq!(listener.get_processed_bms_files_count(), 0);
        assert_eq!(listener.get_new_bms_files_count(), 0);
    }

    #[test]
    fn song_database_update_listener_add_counts() {
        let listener = SongDatabaseUpdateListener::default();
        listener.add_bms_files_count(10);
        listener.add_bms_files_count(5);
        assert_eq!(listener.get_bms_files_count(), 15);

        listener.add_processed_bms_files_count(3);
        assert_eq!(listener.get_processed_bms_files_count(), 3);

        listener.add_new_bms_files_count(2);
        assert_eq!(listener.get_new_bms_files_count(), 2);
    }

    #[test]
    fn display_mode_default() {
        let dm = DisplayMode::default();
        assert_eq!(dm.width, 0);
        assert_eq!(dm.height, 0);
    }

    #[test]
    fn get_available_display_modes_not_empty() {
        let modes = MainLoader::get_available_display_mode();
        assert!(!modes.is_empty());
        assert!(modes.iter().any(|m| m.width == 1920 && m.height == 1080));
    }

    #[test]
    fn get_desktop_display_mode_returns_1080p() {
        let dm = MainLoader::get_desktop_display_mode();
        assert_eq!(dm.width, 1920);
        assert_eq!(dm.height, 1080);
    }
}
