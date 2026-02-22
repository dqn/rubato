use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use log::{error, info};

use crate::bms_player_mode::BMSPlayerMode;
use crate::config::Config;
use crate::main_controller::MainController;
use crate::player_config::PlayerConfig;
use crate::version;

/// SongDatabaseAccessor stub (Phase 5+ dependency)
pub struct SongDatabaseAccessorStub;

impl SongDatabaseAccessorStub {
    pub fn new(_songpath: &str, _bmsroot: &[String]) -> Self {
        Self
    }
}

/// VersionChecker trait
pub trait VersionChecker: Send + Sync {
    fn get_message(&self) -> String;
    fn get_download_url(&self) -> Option<String>;
}

/// GithubVersionChecker - checks for updates via GitHub API
struct GithubVersionChecker {
    message: Mutex<Option<String>>,
    dlurl: Mutex<Option<String>>,
}

impl GithubVersionChecker {
    fn new() -> Self {
        Self {
            message: Mutex::new(None),
            dlurl: Mutex::new(None),
        }
    }

    fn get_information(&self) {
        // Phase 5+: HTTP request to GitHub API
        // https://api.github.com/repos/seraxis/lr2oraja-endlessdream/releases/latest
        let mut msg = self.message.lock().unwrap();
        if msg.is_none() {
            *msg = Some("Version information unavailable".to_string());
        }
    }
}

impl VersionChecker for GithubVersionChecker {
    fn get_message(&self) -> String {
        {
            let msg = self.message.lock().unwrap();
            if msg.is_some() {
                return msg.clone().unwrap();
            }
        }
        self.get_information();
        self.message.lock().unwrap().clone().unwrap_or_default()
    }

    fn get_download_url(&self) -> Option<String> {
        {
            let msg = self.message.lock().unwrap();
            if msg.is_none() {
                drop(msg);
                self.get_information();
            }
        }
        self.dlurl.lock().unwrap().clone()
    }
}

#[allow(dead_code)]
static SONGDB: OnceLock<Mutex<Option<SongDatabaseAccessorStub>>> = OnceLock::new();
static ILLEGAL_SONGS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static BMS_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static VERSION_CHECKER: OnceLock<Mutex<Option<Box<dyn VersionChecker>>>> = OnceLock::new();

/// MainLoader - application entry point and launcher
pub struct MainLoader;

impl MainLoader {
    #[allow(dead_code)]
    const ALLOWS_32BIT_JAVA: bool = false;

    fn illegal_songs() -> &'static Mutex<HashSet<String>> {
        ILLEGAL_SONGS.get_or_init(|| Mutex::new(HashSet::new()))
    }

    /// Main entry point
    pub fn main(args: &[String]) {
        let mut auto: Option<BMSPlayerMode> = None;
        let mut bms_path: Option<PathBuf> = None;

        for s in args {
            if s.starts_with('-') {
                match s.as_str() {
                    "-a" => auto = Some(BMSPlayerMode::AUTOPLAY),
                    "-p" => auto = Some(BMSPlayerMode::PRACTICE),
                    "-r" | "-r1" => auto = Some(BMSPlayerMode::REPLAY_1),
                    "-r2" => auto = Some(BMSPlayerMode::REPLAY_2),
                    "-r3" => auto = Some(BMSPlayerMode::REPLAY_3),
                    "-r4" => auto = Some(BMSPlayerMode::REPLAY_4),
                    "-s" => auto = Some(BMSPlayerMode::PLAY),
                    _ => {}
                }
            } else {
                bms_path = Some(PathBuf::from(s));
                if auto.is_none() {
                    auto = Some(BMSPlayerMode::PLAY);
                }
            }
        }

        // Store bms_path globally
        if let Some(ref p) = bms_path {
            let mut bp = BMS_PATH.get_or_init(|| Mutex::new(None)).lock().unwrap();
            *bp = Some(p.clone());
        }

        let config_exists =
            PathBuf::from("config_sys.json").exists() || PathBuf::from("config.json").exists();
        let has_bms_path = bms_path.is_some();
        if config_exists && (has_bms_path || auto.is_some()) {
            Self::play(bms_path, auto, true, None, None, has_bms_path);
        } else {
            // Launch configuration UI
            // Phase 5+: JavaFX/egui launcher
            log::warn!("not yet implemented: launcher UI");
        }
    }

    pub fn play(
        bms_path: Option<PathBuf>,
        player_mode: Option<BMSPlayerMode>,
        _force_exit: bool,
        config: Option<Config>,
        player: Option<PlayerConfig>,
        song_updated: bool,
    ) {
        let config = config.unwrap_or_else(|| {
            Config::read().unwrap_or_else(|e| {
                error!("Config read failed: {}", e);
                Config::default()
            })
        });

        // Check for illegal songs
        // Phase 5+: getScoreDatabaseAccessor().getSongDatas(...)

        if Self::get_illegal_song_count() > 0 {
            error!(
                "Detected {} illegal BMS songs. Remove them, update song database and restart.",
                Self::get_illegal_song_count()
            );
            std::process::exit(1);
        }

        let player = player.unwrap_or_else(|| {
            let playerpath = &config.playerpath;
            let playername = config.playername.as_deref().unwrap_or("default");
            PlayerConfig::read_player_config(playerpath, playername).unwrap_or_else(|e| {
                error!("Player config read failed: {}", e);
                PlayerConfig::default()
            })
        });

        let _main = MainController::new(bms_path, config, player, player_mode, song_updated);

        // Phase 5+: Lwjgl3Application / Bevy window creation and render loop
        // This is where the application window would be created and the render loop started
        info!("Application started - {}", version::version_long());
    }

    pub fn get_score_database_accessor() -> Option<()> {
        // Phase 5+: SQLiteSongDatabaseAccessor
        None
    }

    pub fn get_version_checker() -> &'static Mutex<Option<Box<dyn VersionChecker>>> {
        VERSION_CHECKER.get_or_init(|| Mutex::new(Some(Box::new(GithubVersionChecker::new()))))
    }

    pub fn set_version_checker(checker: Box<dyn VersionChecker>) {
        let vc = Self::get_version_checker();
        let mut guard = vc.lock().unwrap();
        *guard = Some(checker);
    }

    pub fn get_bms_path() -> Option<PathBuf> {
        BMS_PATH.get().and_then(|m| m.lock().unwrap().clone())
    }

    pub fn put_illegal_song(hash: &str) {
        let mut songs = Self::illegal_songs().lock().unwrap();
        songs.insert(hash.to_string());
    }

    pub fn get_illegal_songs() -> Vec<String> {
        let songs = Self::illegal_songs().lock().unwrap();
        songs.iter().cloned().collect()
    }

    pub fn get_illegal_song_count() -> usize {
        let songs = Self::illegal_songs().lock().unwrap();
        songs.len()
    }

    /// Returns available display modes.
    ///
    /// Translated from: MainLoader.getAvailableDisplayMode()
    /// In Java: Lwjgl3ApplicationConfiguration.getDisplayModes()
    /// In Rust: winit monitor enumeration via global cache.
    pub fn get_available_display_mode() -> Vec<(u32, u32)> {
        // Phase 5+: use winit available_monitors() cache
        log::warn!("not yet implemented: getAvailableDisplayMode");
        vec![(1920, 1080), (1280, 720)]
    }

    /// Returns the desktop display mode.
    ///
    /// Translated from: MainLoader.getDesktopDisplayMode()
    /// In Java: Lwjgl3ApplicationConfiguration.getDisplayMode()
    pub fn get_desktop_display_mode() -> (u32, u32) {
        // Phase 5+: use winit primary monitor
        log::warn!("not yet implemented: getDesktopDisplayMode");
        (1920, 1080)
    }

    /// JavaFX start method (launcher UI entry point).
    ///
    /// Translated from: MainLoader.start(Stage)
    /// In Rust, the launcher UI is handled by egui via LauncherApp.
    pub fn start() {
        log::warn!("not yet implemented: MainLoader.start (egui launcher)");
    }
}
