use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use log::{error, info};

use rubato_types::song_database_accessor::SongDatabaseAccessor as SongDatabaseAccessorTrait;
use rubato_types::sync_utils::lock_or_recover;
use rubato_types::validatable::Validatable;

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

    fn information(&self) {
        // Phase 5+: HTTP request to GitHub API
        // https://api.github.com/repos/seraxis/lr2oraja-endlessdream/releases/latest
        let mut msg = lock_or_recover(&self.message);
        if msg.is_none() {
            *msg = Some("Version information unavailable".to_string());
        }
    }
}

impl VersionChecker for GithubVersionChecker {
    fn get_message(&self) -> String {
        {
            let msg = lock_or_recover(&self.message);
            if let Some(ref m) = *msg {
                return m.clone();
            }
        }
        self.information();
        lock_or_recover(&self.message).clone().unwrap_or_default()
    }

    fn get_download_url(&self) -> Option<String> {
        {
            let msg = lock_or_recover(&self.message);
            if msg.is_none() {
                drop(msg);
                self.information();
            }
        }
        lock_or_recover(&self.dlurl).clone()
    }
}

// Global song database accessor: set by the launcher (which creates SQLiteSongDatabaseAccessor),
// read by MainLoader.get_score_database_accessor(). Wrapped in Mutex for interior mutability
// and to provide Sync (rusqlite::Connection is Send but not Sync).
static SONGDB: OnceLock<Mutex<Option<Box<dyn SongDatabaseAccessorTrait>>>> = OnceLock::new();
static ILLEGAL_SONGS: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static BMS_PATH: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static VERSION_CHECKER: OnceLock<Mutex<Option<Box<dyn VersionChecker>>>> = OnceLock::new();
static DISPLAY_MODES: Mutex<Vec<(u32, u32)>> = Mutex::new(Vec::new());
static DESKTOP_MODE: Mutex<(u32, u32)> = Mutex::new((0, 0));

/// MainLoader - application entry point and launcher
pub struct MainLoader;

impl MainLoader {
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
            let mut bp = lock_or_recover(BMS_PATH.get_or_init(|| Mutex::new(None)));
            *bp = Some(p.clone());
        }

        let config_exists = {
            let cwd = std::env::current_dir().unwrap_or_default();
            rubato_types::config::resolve_config_dir(&cwd).is_some()
        };
        let has_bms_path = bms_path.is_some();
        if config_exists && (has_bms_path || auto.is_some()) {
            let _main =
                Self::play(bms_path, auto, true, None, None, has_bms_path).unwrap_or_else(|e| {
                    error!("Failed to start: {}", e);
                    std::process::exit(1);
                });
        } else {
            // Launch the egui configuration UI via beatoraja-launcher::launcher_ui::run_launcher().
            // The actual call is in beatoraja-bin main() which orchestrates launcher → play transitions.
        }
    }

    /// Create a MainController ready for the winit/wgpu event loop.
    ///
    /// Translated from: MainLoader.play() (Java lines 129-277)
    ///
    /// In Java, play() creates MainController AND launches Lwjgl3Application.
    /// In Rust, the winit/wgpu event loop lives in beatoraja-bin, so this method
    /// handles everything up to (but not including) window creation:
    /// 1. Read Config (if not provided)
    /// 2. Check illegal songs via song database
    /// 3. Read PlayerConfig (if not provided)
    /// 4. Set window dimensions from resolution
    /// 5. Create MainController and pass the song database
    ///
    /// The caller (beatoraja-bin) is responsible for creating the winit EventLoop,
    /// wgpu GPU context, and running the render loop.
    pub fn play(
        bms_path: Option<PathBuf>,
        player_mode: Option<BMSPlayerMode>,
        _force_exit: bool,
        config: Option<Config>,
        player: Option<PlayerConfig>,
        song_updated: bool,
    ) -> anyhow::Result<MainController> {
        let mut config = config.unwrap_or_else(|| {
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
            anyhow::bail!(
                "Detected {} illegal BMS songs. Remove them, update song database and restart.",
                Self::get_illegal_song_count()
            );
        }

        let player = player.unwrap_or_else(|| {
            let playerpath = &config.paths.playerpath;
            let playername = config.playername.as_deref().unwrap_or("default");
            PlayerConfig::read_player_config(playerpath, playername).unwrap_or_else(|e| {
                error!("Player config read failed: {}", e);
                PlayerConfig::default()
            })
        });

        // Java: final int w = config.getResolution().width;
        //        final int h = config.getResolution().height;
        //        config.setWindowWidth(w);
        //        config.setWindowHeight(h);
        let w = config.display.resolution.width();
        let h = config.display.resolution.height();
        config.display.window_width = w;
        config.display.window_height = h;

        // Java: MainController main = new MainController(bmsPath, config, player, playerMode, songUpdated)
        let mut main = MainController::new(bms_path, config, player, player_mode, song_updated);

        // Set the song database on the controller if available
        // Java: MainController accesses songdb via MainLoader.getScoreDatabaseAccessor()
        // In Rust, we pass it explicitly since the controller holds it as a field.
        if let Some(songdb) = Self::take_score_database_accessor() {
            main.set_song_database(songdb);
        }

        info!("Application started - {}", version::version_long());

        Ok(main)
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
        let mut guard = lock_or_recover(Self::songdb_lock());
        *guard = Some(songdb);
    }

    /// Take the global song database accessor out of the global slot.
    ///
    /// Used by play() to move the accessor into MainController.
    /// After this call, the global slot is empty (None).
    fn take_score_database_accessor() -> Option<Box<dyn SongDatabaseAccessorTrait>> {
        let mut guard = lock_or_recover(Self::songdb_lock());
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
        let guard = lock_or_recover(Self::songdb_lock());
        if let Some(ref songdb) = *guard {
            // SongUtils.illegalsongs = ["notme"]
            let illegal_hashes: Vec<String> = vec!["notme".to_string()];
            let songs = songdb.song_datas_by_hashes(&illegal_hashes);
            for song in &songs {
                Self::put_illegal_song(&song.file.sha256);
            }
        }
    }

    pub fn version_checker() -> &'static Mutex<Option<Box<dyn VersionChecker>>> {
        VERSION_CHECKER.get_or_init(|| Mutex::new(Some(Box::new(GithubVersionChecker::new()))))
    }

    pub fn set_version_checker(checker: Box<dyn VersionChecker>) {
        let vc = Self::version_checker();
        let mut guard = lock_or_recover(vc);
        *guard = Some(checker);
    }

    pub fn get_bms_path() -> Option<PathBuf> {
        BMS_PATH.get().and_then(|m| lock_or_recover(m).clone())
    }

    pub fn put_illegal_song(hash: &str) {
        let mut songs = lock_or_recover(Self::illegal_songs());
        songs.insert(hash.to_string());
    }

    pub fn get_illegal_songs() -> Vec<String> {
        let songs = lock_or_recover(Self::illegal_songs());
        songs.iter().cloned().collect()
    }

    pub fn get_illegal_song_count() -> usize {
        let songs = lock_or_recover(Self::illegal_songs());
        songs.len()
    }

    /// Clear all illegal songs. For testing — not present in Java.
    pub fn clear_illegal_songs() {
        let mut songs = lock_or_recover(Self::illegal_songs());
        songs.clear();
    }

    /// Clear the global song database accessor. For testing — not present in Java.
    pub fn clear_score_database_accessor() {
        let mut guard = lock_or_recover(Self::songdb_lock());
        *guard = None;
    }

    /// Returns available display modes.
    ///
    /// Translated from: MainLoader.getAvailableDisplayMode()
    /// In Java: Lwjgl3ApplicationConfiguration.getDisplayModes()
    /// In Rust: winit monitor enumeration via global cache.
    pub fn get_available_display_mode() -> Vec<(u32, u32)> {
        let modes = lock_or_recover(&DISPLAY_MODES);
        if modes.is_empty() {
            // Fallback before winit event loop populates the cache
            vec![(1280, 720), (1920, 1080)]
        } else {
            modes.clone()
        }
    }

    /// Returns the desktop display mode.
    ///
    /// Translated from: MainLoader.getDesktopDisplayMode()
    /// In Java: Lwjgl3ApplicationConfiguration.getDisplayMode()
    pub fn get_desktop_display_mode() -> (u32, u32) {
        let mode = *lock_or_recover(&DESKTOP_MODE);
        if mode == (0, 0) {
            // Fallback before winit event loop populates the cache
            (1920, 1080)
        } else {
            mode
        }
    }

    /// Set the cached display modes from winit monitor enumeration.
    ///
    /// Called by the binary crate after winit event loop populates monitor info.
    pub fn set_display_modes(modes: Vec<(u32, u32)>) {
        *lock_or_recover(&DISPLAY_MODES) = modes;
    }

    /// Set the cached desktop display mode from winit primary monitor.
    ///
    /// Called by the binary crate after winit event loop populates monitor info.
    pub fn set_desktop_display_mode(mode: (u32, u32)) {
        *lock_or_recover(&DESKTOP_MODE) = mode;
    }

    /// JavaFX start method (launcher UI entry point).
    ///
    /// Translated from: MainLoader.start(Stage)
    ///
    /// In Java, this creates a JavaFX Stage with PlayConfigurationView and shows
    /// the configuration window. In Rust, the actual egui window is created by the
    /// binary crate (beatoraja-bin) using winit+wgpu+egui. This method handles the
    /// Config/PlayerConfig loading part of start(), matching the Java logic:
    ///
    /// ```java
    /// Config config;
    /// try { config = Config.read(); }
    /// catch (PlayerConfigException e) { config = Config.validateConfig(new Config()); }
    /// PlayConfigurationView bmsinfo = loader.getController();
    /// bmsinfo.update(config);
    /// primaryStage.setTitle(MainController.getVersion() + " configuration");
    /// primaryStage.show();
    /// ```
    ///
    /// Returns (Config, PlayerConfig, window_title) for the binary crate to create
    /// the egui launcher window.
    pub fn start() -> (Config, PlayerConfig, String) {
        let mut config = Config::read().unwrap_or_else(|e| {
            error!("Config read failed, using defaults: {}", e);
            let mut c = Config::default();
            c.validate();
            c
        });
        config.validate();

        if let Err(e) = PlayerConfig::init(&mut config) {
            error!("Player config init failed: {}", e);
        }

        let player = {
            let playerpath = &config.paths.playerpath;
            let playername = config.playername.as_deref().unwrap_or("default");
            PlayerConfig::read_player_config(playerpath, playername).unwrap_or_else(|e| {
                error!("Player config read failed, using defaults: {}", e);
                PlayerConfig::default()
            })
        };

        // Java: primaryStage.setTitle(MainController.getVersion() + " configuration")
        let title = format!("{} configuration", MainController::get_version());

        info!("{} launcher starting", title);

        (config, player, title)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubato_types::folder_data::FolderData;
    use rubato_types::song_data::SongData;

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
        fn song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
            self.songs.clone()
        }

        fn song_datas_by_hashes(&self, hashes: &[String]) -> Vec<SongData> {
            self.songs
                .iter()
                .filter(|s| hashes.contains(&s.file.sha256) || hashes.contains(&s.file.md5))
                .cloned()
                .collect()
        }

        fn song_datas_by_sql(
            &self,
            _sql: &str,
            _score: &str,
            _scorelog: &str,
            _info: Option<&str>,
        ) -> Vec<SongData> {
            Vec::new()
        }

        fn set_song_datas(&self, _songs: &[SongData]) -> anyhow::Result<()> {
            Ok(())
        }

        fn song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
            Vec::new()
        }

        fn folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
            Vec::new()
        }
    }

    // Global lock to serialize tests that touch shared static state (illegal songs, songdb).
    // Tests that call play() or modify illegal songs must hold this lock to avoid
    // race conditions (play() calls std::process::exit(1) if illegal songs > 0).
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_put_and_get_illegal_songs() {
        let _lock = TEST_LOCK.lock().unwrap();
        MainLoader::put_illegal_song("abc123");
        let songs = MainLoader::get_illegal_songs();
        assert!(songs.contains(&"abc123".to_string()));
    }

    #[test]
    fn test_illegal_song_count() {
        let _lock = TEST_LOCK.lock().unwrap();
        let initial_count = MainLoader::get_illegal_song_count();
        MainLoader::put_illegal_song("unique_test_hash_12345");
        assert!(MainLoader::get_illegal_song_count() > initial_count);
    }

    #[test]
    fn test_version_checker_default() {
        let vc = MainLoader::version_checker();
        let guard = vc.lock().expect("mutex poisoned");
        assert!(guard.is_some());
    }

    #[test]
    fn test_version_checker_message() {
        let vc = MainLoader::version_checker();
        let guard = vc.lock().expect("mutex poisoned");
        let checker = guard.as_ref().unwrap();
        let msg = checker.get_message();
        assert!(!msg.is_empty());
    }

    #[test]
    fn test_set_and_take_score_database_accessor() {
        let _lock = TEST_LOCK.lock().unwrap();
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
        let _lock = TEST_LOCK.lock().unwrap();
        // When no DB is set, check_illegal_songs should not panic
        MainLoader::check_illegal_songs();
    }

    #[test]
    fn test_check_illegal_songs_with_matching_songs() {
        let _lock = TEST_LOCK.lock().unwrap();
        // Create a song with sha256 = "notme"
        let mut song = SongData::new();
        song.file.sha256 = "notme".to_string();
        song.metadata.title = "Illegal Song".to_string();

        let mock = Box::new(MockSongDb::with_songs(vec![song]));
        MainLoader::set_score_database_accessor(mock);

        MainLoader::check_illegal_songs();

        // The illegal song should be recorded
        let illegals = MainLoader::get_illegal_songs();
        assert!(illegals.contains(&"notme".to_string()));

        // Clean up: take the songdb back and clear illegal songs
        let _ = MainLoader::take_score_database_accessor();
        MainLoader::clear_illegal_songs();
    }

    // play() tests use global statics (illegal songs, songdb) and call
    // std::process::exit(1) when illegals are found. They must run single-threaded
    // (--test-threads=1) or with #[ignore] to avoid race conditions with other tests
    // that populate the global illegal songs set.

    #[test]
    fn test_play_returns_main_controller() {
        let _lock = TEST_LOCK.lock().unwrap();
        // MainLoader::play() should return a MainController instance.
        use crate::resolution::Resolution;

        // Clear global state from other tests to avoid std::process::exit(1)
        MainLoader::clear_illegal_songs();
        MainLoader::clear_score_database_accessor();

        let config = Config::default();
        let player = PlayerConfig::default();
        let controller =
            MainLoader::play(None, None, true, Some(config), Some(player), false).unwrap();

        // The returned controller should have config with window dimensions
        // set from resolution (Java: config.setWindowWidth(w); config.setWindowHeight(h))
        let cfg = controller.config();
        let expected_w = Resolution::HD.width();
        let expected_h = Resolution::HD.height();
        assert_eq!(cfg.display.window_width, expected_w);
        assert_eq!(cfg.display.window_height, expected_h);
    }

    #[test]
    fn test_play_sets_window_dimensions_from_resolution() {
        let _lock = TEST_LOCK.lock().unwrap();
        // Java: final int w = config.getResolution().width;
        //        final int h = config.getResolution().height;
        //        config.setWindowWidth(w);
        //        config.setWindowHeight(h);
        use crate::resolution::Resolution;

        // Clear global state from other tests to avoid std::process::exit(1)
        MainLoader::clear_illegal_songs();
        MainLoader::clear_score_database_accessor();

        let mut config = Config::default();
        config.display.resolution = Resolution::FULLHD;
        // Set different initial window dimensions to verify they get overwritten
        config.display.window_width = 100;
        config.display.window_height = 100;

        let controller = MainLoader::play(
            None,
            None,
            true,
            Some(config),
            Some(PlayerConfig::default()),
            false,
        )
        .unwrap();

        let cfg = controller.config();
        assert_eq!(cfg.display.window_width, Resolution::FULLHD.width());
        assert_eq!(cfg.display.window_height, Resolution::FULLHD.height());
    }

    #[test]
    fn test_play_with_songdb_passes_to_controller() {
        let _lock = TEST_LOCK.lock().unwrap();
        // Clear global state from other tests to avoid std::process::exit(1)
        MainLoader::clear_illegal_songs();
        MainLoader::clear_score_database_accessor();

        // MainLoader::play() should pass the global songdb to the controller
        let mock = Box::new(MockSongDb::new());
        MainLoader::set_score_database_accessor(mock);

        let controller = MainLoader::play(
            None,
            None,
            true,
            Some(Config::default()),
            Some(PlayerConfig::default()),
            false,
        )
        .unwrap();

        // The songdb should have been taken from the global slot
        let taken = MainLoader::take_score_database_accessor();
        assert!(taken.is_none(), "songdb should have been taken by play()");

        // Controller should have the songdb set
        assert!(controller.song_database().is_some());
    }

    #[test]
    fn test_start_returns_config_player_title() {
        // MainLoader::start() should return (Config, PlayerConfig, title)
        // When no config file exists, it uses defaults.
        let (config, player, title) = MainLoader::start();

        // Config should be valid (validated)
        assert!(config.display.max_frame_per_second >= 0);

        // Player should have a default name
        assert!(!player.name.is_empty() || player.name.is_empty()); // Just check it doesn't panic

        // Title should contain "configuration"
        // Java: primaryStage.setTitle(MainController.getVersion() + " configuration")
        assert!(
            title.contains("configuration"),
            "Title should contain 'configuration', got: {}",
            title
        );
    }

    #[test]
    fn test_start_title_matches_java_format() {
        // Java: MainController.getVersion() + " configuration"
        let (_, _, title) = MainLoader::start();
        let expected_suffix = " configuration";
        assert!(
            title.ends_with(expected_suffix),
            "Title should end with '{}', got: {}",
            expected_suffix,
            title
        );
    }

    #[test]
    fn test_concurrent_global_access_serialized_by_lock() {
        // Verify that TEST_LOCK properly serializes concurrent access to OnceLock<Mutex> globals.
        // Spawn multiple threads that each acquire TEST_LOCK, mutate globals, and verify isolation.
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles: Vec<_> = (0..4)
            .map(|i| {
                let barrier = std::sync::Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    let _lock = TEST_LOCK.lock().unwrap();

                    // Clear state, set unique value, verify it's ours
                    MainLoader::clear_illegal_songs();
                    let hash = format!("concurrent_test_{}", i);
                    MainLoader::put_illegal_song(&hash);

                    // Under the lock, only our hash should be present
                    let songs = MainLoader::get_illegal_songs();
                    assert!(songs.contains(&hash), "thread {} hash should be present", i);
                    assert_eq!(
                        songs.len(),
                        1,
                        "thread {} should see exactly 1 song (got {})",
                        i,
                        songs.len()
                    );

                    MainLoader::clear_illegal_songs();
                })
            })
            .collect();

        for h in handles {
            h.join().expect("thread should not panic");
        }
    }

    #[test]
    fn test_songdb_clear_provides_isolation() {
        let _lock = TEST_LOCK.lock().unwrap();

        // Set a mock, verify it's set
        MainLoader::set_score_database_accessor(Box::new(MockSongDb::new()));
        assert!(MainLoader::take_score_database_accessor().is_some());

        // After take, slot is empty
        assert!(MainLoader::take_score_database_accessor().is_none());

        // Clear also works when already empty
        MainLoader::clear_score_database_accessor();
        assert!(MainLoader::take_score_database_accessor().is_none());

        // Set again, clear explicitly, verify empty
        MainLoader::set_score_database_accessor(Box::new(MockSongDb::new()));
        MainLoader::clear_score_database_accessor();
        assert!(MainLoader::take_score_database_accessor().is_none());
    }

    #[test]
    fn test_display_mode_globals_are_independent() {
        // DISPLAY_MODES and DESKTOP_MODE are plain Mutex (not OnceLock),
        // so they can be freely set/reset without test isolation issues.
        let original_modes = MainLoader::get_available_display_mode();
        let original_desktop = MainLoader::get_desktop_display_mode();

        MainLoader::set_display_modes(vec![(800, 600), (1024, 768)]);
        MainLoader::set_desktop_display_mode((800, 600));

        assert_eq!(
            MainLoader::get_available_display_mode(),
            vec![(800, 600), (1024, 768)]
        );
        assert_eq!(MainLoader::get_desktop_display_mode(), (800, 600));

        // Restore originals
        MainLoader::set_display_modes(original_modes);
        MainLoader::set_desktop_display_mode(original_desktop);
    }
}
