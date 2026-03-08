// Integration test: MainLoader startup flow
//
// Tests MainLoader::start() (launcher entry point) and MainLoader::play()
// (MainController creation). These tests directly verify the startup flow
// including PlayerConfig::init being called.
//
// IMPORTANT: MainLoader uses global statics (ILLEGAL_SONGS, SONGDB).
// All tests that touch global state must hold TEST_LOCK and clear state first.

use std::sync::Mutex;

use rubato_core::config::Config;
use rubato_core::main_controller::MainController;
use rubato_core::main_loader::MainLoader;
use rubato_core::player_config::PlayerConfig;
use rubato_core::resolution::Resolution;
use rubato_types::folder_data::FolderData;
use rubato_types::song_data::SongData;
use rubato_types::song_database_accessor::SongDatabaseAccessor;

/// Global lock to serialize tests that touch shared static state (illegal songs, songdb).
static TEST_LOCK: Mutex<()> = Mutex::new(());

/// Mock SongDatabaseAccessor for testing.
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

impl SongDatabaseAccessor for MockSongDb {
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

    fn set_song_datas(&self, _songs: &[SongData]) {}

    fn song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        Vec::new()
    }

    fn folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        Vec::new()
    }
}

/// Helper: clear all global state and return the lock guard.
fn lock_and_clear_state() -> std::sync::MutexGuard<'static, ()> {
    let guard = TEST_LOCK.lock().unwrap();
    MainLoader::clear_illegal_songs();
    MainLoader::clear_score_database_accessor();
    guard
}

// ---------------------------------------------------------------------------
// Test 1: start_returns_validated_config
// ---------------------------------------------------------------------------

#[test]
fn start_returns_validated_config() {
    // MainLoader::start() reads config (or falls back to defaults), calls validate(),
    // and returns it. After validation, audio should be Some(...) and default paths filled.
    let (config, _player, _title) = MainLoader::start();

    // validate() sets audio = Some(AudioConfig::default()) when None
    assert!(
        config.audio.is_some(),
        "Config.audio should be Some after validation"
    );

    // Default paths should be non-empty after validation
    assert!(
        !config.paths.songpath.is_empty(),
        "songpath should be non-empty"
    );
    assert!(
        !config.paths.songinfopath.is_empty(),
        "songinfopath should be non-empty"
    );
    assert!(
        !config.paths.tablepath.is_empty(),
        "tablepath should be non-empty"
    );
    assert!(
        !config.paths.playerpath.is_empty(),
        "playerpath should be non-empty"
    );
    assert!(
        !config.paths.skinpath.is_empty(),
        "skinpath should be non-empty"
    );
}

// ---------------------------------------------------------------------------
// Test 2: start_title_format
// ---------------------------------------------------------------------------

#[test]
fn start_title_format() {
    // Java: primaryStage.setTitle(MainController.getVersion() + " configuration")
    let (_config, _player, title) = MainLoader::start();

    assert!(
        title.ends_with(" configuration"),
        "Title should end with ' configuration', got: {}",
        title
    );

    // The title prefix should match MainController::get_version()
    let expected_prefix = MainController::get_version();
    assert!(
        title.starts_with(expected_prefix),
        "Title should start with '{}', got: {}",
        expected_prefix,
        title
    );
}

// ---------------------------------------------------------------------------
// Test 3: play_returns_controller_with_config
// ---------------------------------------------------------------------------

#[test]
fn play_returns_controller_with_config() {
    let _lock = lock_and_clear_state();

    let config = Config::default();
    let player = PlayerConfig::default();
    let result = MainLoader::play(None, None, true, Some(config), Some(player), false);

    assert!(
        result.is_ok(),
        "play() should return Ok, got err: {}",
        result
            .as_ref()
            .err()
            .map_or("".to_string(), |e| e.to_string())
    );

    let controller = result.unwrap();
    let cfg = controller.config();

    // play() sets window dimensions from resolution; default resolution is HD (1280x720)
    let expected_w = Resolution::HD.width();
    let expected_h = Resolution::HD.height();
    assert_eq!(
        cfg.display.window_width, expected_w,
        "window_width should be {} (HD), got {}",
        expected_w, cfg.display.window_width
    );
    assert_eq!(
        cfg.display.window_height, expected_h,
        "window_height should be {} (HD), got {}",
        expected_h, cfg.display.window_height
    );
}

// ---------------------------------------------------------------------------
// Test 4: play_sets_window_dimensions
// ---------------------------------------------------------------------------

#[test]
fn play_sets_window_dimensions() {
    let _lock = lock_and_clear_state();

    let mut config = Config::default();
    config.display.resolution = Resolution::FULLHD;
    // Set different initial values to verify they get overwritten
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
    .expect("play() should succeed");

    let cfg = controller.config();
    assert_eq!(
        cfg.display.window_width,
        Resolution::FULLHD.width(),
        "window_width should match FULLHD resolution ({})",
        Resolution::FULLHD.width()
    );
    assert_eq!(
        cfg.display.window_height,
        Resolution::FULLHD.height(),
        "window_height should match FULLHD resolution ({})",
        Resolution::FULLHD.height()
    );
}

// ---------------------------------------------------------------------------
// Test 5: play_returns_error_on_illegal_songs
// ---------------------------------------------------------------------------

#[test]
fn play_returns_error_on_illegal_songs() {
    let _lock = lock_and_clear_state();

    // Inject an illegal song before calling play()
    MainLoader::put_illegal_song("bad");

    let result = MainLoader::play(
        None,
        None,
        true,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    );

    assert!(
        result.is_err(),
        "play() should return Err when illegal songs exist"
    );

    let err_msg = result.err().expect("Expected Err result").to_string();
    assert!(
        err_msg.contains("illegal"),
        "Error message should mention 'illegal', got: {}",
        err_msg
    );
}

// ---------------------------------------------------------------------------
// Test 6: play_passes_songdb_to_controller
// ---------------------------------------------------------------------------

#[test]
fn play_passes_songdb_to_controller() {
    let _lock = lock_and_clear_state();

    // Set a mock songdb in the global slot
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
    .expect("play() should succeed");

    // The controller should have received the songdb
    assert!(
        controller.song_database().is_some(),
        "Controller should have a song database after play() with songdb set"
    );
}

// ---------------------------------------------------------------------------
// Test 7: play_clears_songdb_after_take
// ---------------------------------------------------------------------------

#[test]
fn play_clears_songdb_after_take() {
    let _lock = lock_and_clear_state();

    // Set a mock songdb
    let mock = Box::new(MockSongDb::new());
    MainLoader::set_score_database_accessor(mock);

    // First play() should take the songdb
    let controller1 = MainLoader::play(
        None,
        None,
        true,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("first play() should succeed");

    assert!(
        controller1.song_database().is_some(),
        "First controller should have songdb"
    );

    // Second play() should NOT have the songdb (it was taken by the first call)
    let controller2 = MainLoader::play(
        None,
        None,
        true,
        Some(Config::default()),
        Some(PlayerConfig::default()),
        false,
    )
    .expect("second play() should succeed");

    assert!(
        controller2.song_database().is_none(),
        "Second controller should NOT have songdb (already taken by first play())"
    );
}

// ---------------------------------------------------------------------------
// Test 8: start_reads_player_config_with_correct_id
// ---------------------------------------------------------------------------

#[test]
fn start_reads_player_config_with_correct_id() {
    // MainLoader::start() calls PlayerConfig::init() and then reads the player config.
    // When no config file exists on disk, it falls back to defaults.
    // The returned PlayerConfig should have the default name set.
    let (_config, player, _title) = MainLoader::start();

    // Default PlayerConfig has name = "NO NAME"
    assert!(
        !player.name.is_empty(),
        "Player name should not be empty, got: '{}'",
        player.name
    );
    assert_eq!(
        player.name, "NO NAME",
        "Default player name should be 'NO NAME', got: '{}'",
        player.name
    );
}

// ---------------------------------------------------------------------------
// Test 9: start_then_play_sequential_lifecycle
// ---------------------------------------------------------------------------

/// Simulate the logical flow of launch() → play() without GUI.
///
/// MainLoader::start() (the launcher path) followed by MainLoader::play()
/// (the game path) must work in sequence without corrupting global state.
/// This catches issues where the first subsystem leaves stale state that
/// breaks the second.
#[test]
fn start_then_play_sequential_lifecycle() {
    let _lock = lock_and_clear_state();

    // Phase 1: launcher startup (MainLoader::start)
    let (config, player, title) = MainLoader::start();
    assert!(
        title.ends_with(" configuration"),
        "start() title should end with ' configuration', got: {}",
        title
    );

    // Phase 2: game startup (MainLoader::play) using the config from start()
    let result = MainLoader::play(None, None, true, Some(config), Some(player), false);
    assert!(
        result.is_ok(),
        "play() after start() should succeed, got err: {}",
        result
            .as_ref()
            .err()
            .map_or("".to_string(), |e| e.to_string())
    );

    let controller = result.unwrap();
    let cfg = controller.config();

    // Config should be valid — window dimensions set from resolution
    assert!(
        cfg.display.window_width > 0,
        "window_width should be positive"
    );
    assert!(
        cfg.display.window_height > 0,
        "window_height should be positive"
    );

    // Global state should be clean
    assert_eq!(
        MainLoader::get_illegal_song_count(),
        0,
        "illegal songs should be empty after clean lifecycle"
    );
}

// ---------------------------------------------------------------------------
// Test 10: play_twice_sequential_does_not_corrupt_state
// ---------------------------------------------------------------------------

/// Call MainLoader::play() twice in sequence to verify that the first call
/// does not leave stale global state that breaks the second.
#[test]
fn play_twice_sequential_does_not_corrupt_state() {
    let _lock = lock_and_clear_state();

    // First play()
    let config1 = Config::default();
    let player1 = PlayerConfig::default();
    let result1 = MainLoader::play(None, None, true, Some(config1), Some(player1), false);
    assert!(
        result1.is_ok(),
        "first play() should succeed, got err: {}",
        result1
            .as_ref()
            .err()
            .map_or("".to_string(), |e| e.to_string())
    );

    let controller1 = result1.unwrap();
    let w1 = controller1.config().display.window_width;
    let h1 = controller1.config().display.window_height;

    // Second play() — must also succeed with independent config
    let mut config2 = Config::default();
    config2.display.resolution = Resolution::FULLHD;
    let player2 = PlayerConfig::default();
    let result2 = MainLoader::play(None, None, true, Some(config2), Some(player2), false);
    assert!(
        result2.is_ok(),
        "second play() should succeed, got err: {}",
        result2
            .as_ref()
            .err()
            .map_or("".to_string(), |e| e.to_string())
    );

    let controller2 = result2.unwrap();
    let w2 = controller2.config().display.window_width;
    let h2 = controller2.config().display.window_height;

    // The two controllers should have independent configs — different resolutions
    assert_ne!(
        (w1, h1),
        (w2, h2),
        "second play() should have different resolution (FULLHD vs HD default)"
    );

    // Global state should still be clean
    assert_eq!(
        MainLoader::get_illegal_song_count(),
        0,
        "illegal songs should be empty after two clean play() calls"
    );
}
