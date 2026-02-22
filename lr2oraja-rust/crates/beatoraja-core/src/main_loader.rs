use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use log::{error, info};

use beatoraja_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;
use beatoraja_types::validatable::Validatable;

use crate::bms_player_mode::BMSPlayerMode;
use crate::config::Config;
use crate::main_controller::MainController;
use crate::player_config::PlayerConfig;
use crate::version;

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

// Global song database accessor: set by the launcher (which creates SQLiteSongDatabaseAccessor),
// read by MainLoader.get_score_database_accessor(). Wrapped in Mutex for interior mutability
// and to provide Sync (rusqlite::Connection is Send but not Sync).
static SONGDB: OnceLock<Mutex<Option<Box<dyn SongDatabaseAccessorTrait>>>> = OnceLock::new();
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

        // Check for illegal songs via song database
        // Java: for (SongData song : getScoreDatabaseAccessor().getSongDatas(SongUtils.illegalsongs)) {
        //     MainLoader.putIllegalSong(song.getSha256());
        // }
        Self::check_illegal_songs();

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

        let mut main = MainController::new(bms_path, config, player, player_mode, song_updated);

        // Set the song database on the controller if available
        // Java: MainController accesses songdb via MainLoader.getScoreDatabaseAccessor()
        // In Rust, we pass it explicitly since the controller holds it as a field.
        if let Some(songdb) = Self::take_score_database_accessor() {
            main.set_song_database(songdb);
        }

        // Phase 5+: Lwjgl3Application / winit+wgpu window creation and render loop
        // This is where the application window would be created and the render loop started
        info!("Application started - {}", version::version_long());
    }

    /// Returns a reference to the global song database accessor.
    ///
    /// Translated from: MainLoader.getScoreDatabaseAccessor()
    ///
    /// The accessor must be set via `set_score_database_accessor()` before calling this.
    /// In the application, the launcher creates SQLiteSongDatabaseAccessor and sets it.
    fn songdb_lock() -> &'static Mutex<Option<Box<dyn SongDatabaseAccessorTrait>>> {
        SONGDB.get_or_init(|| Mutex::new(None))
    }

    /// Set the global song database accessor.
    ///
    /// Called by the launcher (which has access to beatoraja-song) after creating
    /// SQLiteSongDatabaseAccessor. Must be called before play().
    pub fn set_score_database_accessor(songdb: Box<dyn SongDatabaseAccessorTrait>) {
        let mut guard = Self::songdb_lock().lock().unwrap();
        *guard = Some(songdb);
    }

    /// Take the global song database accessor out of the global slot.
    ///
    /// Used by play() to move the accessor into MainController.
    /// After this call, the global slot is empty (None).
    fn take_score_database_accessor() -> Option<Box<dyn SongDatabaseAccessorTrait>> {
        let mut guard = Self::songdb_lock().lock().unwrap();
        guard.take()
    }

    /// Check for illegal songs using the global song database accessor.
    ///
    /// Translated from Java: MainLoader.play() lines 139-141
    /// ```java
    /// for (SongData song : getScoreDatabaseAccessor().getSongDatas(SongUtils.illegalsongs)) {
    ///     MainLoader.putIllegalSong(song.getSha256());
    /// }
    /// ```
    fn check_illegal_songs() {
        let guard = Self::songdb_lock().lock().unwrap();
        if let Some(ref songdb) = *guard {
            // SongUtils.illegalsongs = ["notme"]
            let illegal_hashes: Vec<String> = vec!["notme".to_string()];
            let songs = songdb.get_song_datas_by_hashes(&illegal_hashes);
            for song in &songs {
                Self::put_illegal_song(&song.sha256);
            }
        }
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
    ///
    /// In Java, this creates a JavaFX Stage with PlayConfigurationView.
    /// In Rust, the launcher UI is handled by egui via LauncherApp (beatoraja-launcher crate).
    /// This method reads config and delegates to the launcher UI.
    pub fn start() {
        let config = Config::read().unwrap_or_else(|e| {
            error!("Config read failed, using defaults: {}", e);
            let mut c = Config::default();
            c.validate();
            c
        });

        info!(
            "{} configuration launcher starting",
            MainController::get_version()
        );

        // The actual egui UI is created by beatoraja-launcher::LauncherUi
        // which is invoked from the binary crate's main().
        // This method serves as the entry point that the binary delegates to.
        let _ = config;
        log::info!("MainLoader.start: config loaded, launcher UI should be invoked from binary");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beatoraja_types::folder_data::FolderData;
    use beatoraja_types::song_data::SongData;

    /// Mock SongDatabaseAccessor for testing
    struct MockSongDb {
        songs: Vec<SongData>,
    }

    impl MockSongDb {
        fn new() -> Self {
            Self { songs: Vec::new() }
        }

        fn with_songs(songs: Vec<SongData>) -> Self {
            Self { songs }
        }
    }

    impl SongDatabaseAccessorTrait for MockSongDb {
        fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
            self.songs.clone()
        }

        fn get_song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData> {
            self.songs
                .iter()
                .filter(|s| hashes.contains(&s.sha256) || hashes.contains(&s.md5))
                .cloned()
                .collect()
        }

        fn get_song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<SongData> {
            Vec::new()
        }

        fn set_song_datas(&self, _songs: &[SongData]) {}

        fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
            Vec::new()
        }

        fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    #[test]
    fn test_put_and_get_illegal_songs() {
        MainLoader::put_illegal_song("abc123");
        let songs = MainLoader::get_illegal_songs();
        assert!(songs.contains(&"abc123".to_string()));
    }

    #[test]
    fn test_illegal_song_count() {
        let initial_count = MainLoader::get_illegal_song_count();
        MainLoader::put_illegal_song("unique_test_hash_12345");
        assert!(MainLoader::get_illegal_song_count() >= initial_count + 1);
    }

    #[test]
    fn test_version_checker_default() {
        let vc = MainLoader::get_version_checker();
        let guard = vc.lock().unwrap();
        assert!(guard.is_some());
    }

    #[test]
    fn test_version_checker_message() {
        let vc = MainLoader::get_version_checker();
        let guard = vc.lock().unwrap();
        let checker = guard.as_ref().unwrap();
        let msg = checker.get_message();
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_set_and_take_score_database_accessor() {
        // Set a mock songdb
        let mock = Box::new(MockSongDb::new());
        MainLoader::set_score_database_accessor(mock);

        // Take it back out
        let taken = MainLoader::take_score_database_accessor();
        assert!(taken.is_some());

        // Now it should be None
        let taken2 = MainLoader::take_score_database_accessor();
        assert!(taken2.is_none());
    }

    // Note: play() integration tests are omitted because play() uses global statics
    // (illegal songs set) and calls std::process::exit(1) if illegals are found.
    // The global state persists across tests, making play() unsafe in unit tests.
    // Integration testing of play() is done via the binary crate.

    #[test]
    fn test_get_available_display_mode() {
        let modes = MainLoader::get_available_display_mode();
        assert!(!modes.is_empty());
        assert!(modes.contains(&(1920, 1080)));
    }

    #[test]
    fn test_get_desktop_display_mode() {
        let mode = MainLoader::get_desktop_display_mode();
        assert_eq!(mode, (1920, 1080));
    }

    #[test]
    fn test_check_illegal_songs_with_no_db() {
        // When no DB is set, check_illegal_songs should not panic
        MainLoader::check_illegal_songs();
    }

    #[test]
    fn test_check_illegal_songs_with_matching_songs() {
        // Create a song with sha256 = "notme"
        let mut song = SongData::new();
        song.sha256 = "notme".to_string();
        song.title = "Illegal Song".to_string();

        let mock = Box::new(MockSongDb::with_songs(vec![song]));
        MainLoader::set_score_database_accessor(mock);

        MainLoader::check_illegal_songs();

        // The illegal song should be recorded
        let illegals = MainLoader::get_illegal_songs();
        assert!(illegals.contains(&"notme".to_string()));

        // Clean up: take the songdb back
        let _ = MainLoader::take_score_database_accessor();
    }
}
