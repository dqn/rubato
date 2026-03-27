// MainLoader: launcher-side wrapper around crate::core::main_loader::MainLoader.
// Handles display mode discovery, song DB initialization, version checking, and game launch.

use crate::core::config::Config;
use crate::core::main_loader::MainLoader as CoreMainLoader;
use crate::core::player_config::PlayerConfig;
use crate::song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

use crate::display_mode::DisplayMode;
use crate::platform::{
    cached_desktop_display_mode, cached_display_modes, cached_full_display_modes,
};
use crate::version_checker::VersionChecker;

pub use crate::core::bms_player_mode::BMSPlayerMode;

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
        match SQLiteSongDatabaseAccessor::new(&config.paths.songpath, &config.paths.bmsroot) {
            Ok(accessor) => {
                CoreMainLoader::set_score_database_accessor(Box::new(accessor));
                log::info!(
                    "Song database accessor initialized: {}",
                    &config.paths.songpath
                );
            }
            Err(e) => {
                log::error!("Failed to create song database accessor: {}", e);
            }
        }
    }

    pub fn version_checker() -> VersionChecker {
        VersionChecker::default()
    }

    /// Get available display modes.
    ///
    /// Translated from: MainLoader.getAvailableDisplayMode()
    /// Java: Lwjgl3ApplicationConfiguration.getDisplayModes()
    ///
    /// Returns all available video modes from the primary monitor with full
    /// refresh rate and bit depth information. Uses winit-cached display modes
    /// if available, falls back to common defaults.
    pub fn available_display_mode() -> Vec<DisplayMode> {
        let full_modes = cached_full_display_modes();
        if !full_modes.is_empty() {
            // Use full video mode info (includes refresh rate and bit depth)
            full_modes
                .into_iter()
                .map(|m| DisplayMode {
                    width: m.width as i32,
                    height: m.height as i32,
                    refresh_rate_millihertz: m.refresh_rate_millihertz,
                    bit_depth: m.bit_depth,
                })
                .collect()
        } else {
            let cached = cached_display_modes();
            if cached.is_empty() {
                // Fallback before event loop populates the cache
                vec![
                    DisplayMode {
                        width: 1280,
                        height: 720,
                        ..Default::default()
                    },
                    DisplayMode {
                        width: 1920,
                        height: 1080,
                        ..Default::default()
                    },
                    DisplayMode {
                        width: 2560,
                        height: 1440,
                        ..Default::default()
                    },
                    DisplayMode {
                        width: 3840,
                        height: 2160,
                        ..Default::default()
                    },
                ]
            } else {
                cached
                    .into_iter()
                    .map(|(w, h)| DisplayMode {
                        width: w as i32,
                        height: h as i32,
                        ..Default::default()
                    })
                    .collect()
            }
        }
    }

    /// Get the desktop display mode (primary monitor's native mode).
    ///
    /// Translated from: MainLoader.getDesktopDisplayMode()
    /// Java: Lwjgl3ApplicationConfiguration.getDisplayMode()
    ///
    /// Returns the native display mode of the primary monitor with full refresh
    /// rate and bit depth information. Falls back to 1920x1080 if cache is not
    /// yet populated.
    pub fn desktop_display_mode() -> DisplayMode {
        let (w, h) = cached_desktop_display_mode();
        if w == 0 && h == 0 {
            // Fallback before event loop populates the cache
            DisplayMode {
                width: 1920,
                height: 1080,
                ..Default::default()
            }
        } else {
            // Find the best mode at the desktop resolution (highest refresh rate and bit depth)
            // Java: selects mode with highest refreshRate and bitsPerPixel
            let full_modes = cached_full_display_modes();
            let best = full_modes
                .iter()
                .filter(|m| m.width == w && m.height == h)
                .max_by_key(|m| (m.refresh_rate_millihertz, m.bit_depth));

            if let Some(mode) = best {
                DisplayMode {
                    width: w as i32,
                    height: h as i32,
                    refresh_rate_millihertz: mode.refresh_rate_millihertz,
                    bit_depth: mode.bit_depth,
                }
            } else {
                DisplayMode {
                    width: w as i32,
                    height: h as i32,
                    ..Default::default()
                }
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
        let _ = CoreMainLoader::play(
            path.map(std::path::PathBuf::from),
            Some(mode),
            launcher,
            Some(config.clone()),
            Some(player.clone()),
            song_updated,
        );
    }
}
