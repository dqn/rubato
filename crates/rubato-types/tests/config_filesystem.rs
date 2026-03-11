// Integration test: Config filesystem lifecycle (read_from / write_to)
//
// Tests the Config struct's round-trip through the filesystem,
// fallback behavior, corrupt-file recovery, and validation side effects.

use std::fs;

use rubato_types::config::{Config, PLAYERPATH_DEFAULT, SONGPATH_DEFAULT};
use rubato_types::validatable::Validatable;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Round-trip
// ---------------------------------------------------------------------------

#[test]
fn write_then_read_roundtrip() {
    let dir = TempDir::new().unwrap();

    let mut config = Config::default();
    config.display.vsync = true;
    config.display.max_frame_per_second = 120;
    config.display.window_width = 1920;
    config.display.window_height = 1080;
    config.paths.playerpath = dir.path().join("player").to_string_lossy().to_string();

    Config::write_to(&config, dir.path()).unwrap();
    let loaded = Config::read_from(dir.path()).unwrap();

    assert!(loaded.display.vsync);
    assert_eq!(loaded.display.max_frame_per_second, 120);
    assert_eq!(loaded.display.window_width, 1920);
    assert_eq!(loaded.display.window_height, 1080);
}

// ---------------------------------------------------------------------------
// Nonexistent directory contents -> validated default
// ---------------------------------------------------------------------------

#[test]
fn read_nonexistent_returns_validated_default() {
    let dir = TempDir::new().unwrap();

    // read_from with no files falls back to Config::default() then validate_config().
    // The default playerpath is "player" which will be created relative to cwd.
    let config = Config::read_from(dir.path()).unwrap();

    // validate() fills in audio when None
    assert!(
        config.audio.is_some(),
        "Validated default should have audio filled in"
    );
    assert_eq!(config.paths.songpath, SONGPATH_DEFAULT);
    assert_eq!(config.paths.playerpath, PLAYERPATH_DEFAULT);
}

// ---------------------------------------------------------------------------
// Corrupt JSON -> backup created, default returned
// ---------------------------------------------------------------------------

#[test]
fn read_corrupt_json_creates_backup_and_returns_error() {
    let dir = TempDir::new().unwrap();

    // Write garbage to config_sys.json
    let config_path = dir.path().join("config_sys.json");
    fs::write(&config_path, b"this is not valid json {{{").unwrap();

    // With only a corrupt config_sys.json and no legacy config.json,
    // read_from should return an error to prevent silent settings loss.
    let result = Config::read_from(dir.path());
    assert!(
        result.is_err(),
        "read_from should error when existing config is corrupt and no fallback exists"
    );
    let err_msg = result.unwrap_err().to_string();
    assert!(
        err_msg.contains("could not be loaded"),
        "Error message should mention load failure, got: {}",
        err_msg
    );

    // A backup file should still have been created
    let backup_path = dir.path().join("config_sys_backup.json");
    assert!(
        backup_path.exists(),
        "Backup file config_sys_backup.json should exist after corrupt config"
    );

    // Backup should contain the original garbage
    let backup_content = fs::read_to_string(&backup_path).unwrap();
    assert_eq!(backup_content, "this is not valid json {{{");
}

#[test]
fn read_corrupt_primary_falls_back_to_valid_legacy() {
    let dir = TempDir::new().unwrap();

    // Write garbage to config_sys.json
    let config_path = dir.path().join("config_sys.json");
    fs::write(&config_path, b"this is not valid json {{{").unwrap();

    // Write a valid legacy config.json
    let mut fallback = Config::default();
    fallback.display.max_frame_per_second = 75;
    fallback.paths.playerpath = dir.path().join("player").to_string_lossy().to_string();
    let json = serde_json::to_string_pretty(&fallback).unwrap();
    fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();

    // Should succeed by falling back to the valid legacy config
    let config = Config::read_from(dir.path()).unwrap();
    assert_eq!(config.display.max_frame_per_second, 75);
}

// ---------------------------------------------------------------------------
// Old format fallback (config.json)
// ---------------------------------------------------------------------------

#[test]
fn read_old_format_fallback() {
    let dir = TempDir::new().unwrap();

    // Write a valid config to the old path (config.json)
    let mut config = Config::default();
    config.display.vsync = true;
    config.display.max_frame_per_second = 60;
    config.paths.playerpath = dir.path().join("player").to_string_lossy().to_string();

    let json = serde_json::to_string_pretty(&config).unwrap();
    fs::write(dir.path().join("config.json"), json.as_bytes()).unwrap();

    let loaded = Config::read_from(dir.path()).unwrap();

    assert!(loaded.display.vsync);
    assert_eq!(loaded.display.max_frame_per_second, 60);
}

// ---------------------------------------------------------------------------
// New format preferred over old format
// ---------------------------------------------------------------------------

#[test]
fn read_new_format_preferred() {
    let dir = TempDir::new().unwrap();
    let player_dir = dir.path().join("player").to_string_lossy().to_string();

    // Write config_sys.json with max_frame_per_second = 144
    let mut new_config = Config::default();
    new_config.display.max_frame_per_second = 144;
    new_config.paths.playerpath = player_dir.clone();
    let new_json = serde_json::to_string_pretty(&new_config).unwrap();
    fs::write(dir.path().join("config_sys.json"), new_json.as_bytes()).unwrap();

    // Write config.json with max_frame_per_second = 60
    let mut old_config = Config::default();
    old_config.display.max_frame_per_second = 60;
    old_config.paths.playerpath = player_dir;
    let old_json = serde_json::to_string_pretty(&old_config).unwrap();
    fs::write(dir.path().join("config.json"), old_json.as_bytes()).unwrap();

    let loaded = Config::read_from(dir.path()).unwrap();

    // Should use config_sys.json (new format), not config.json
    assert_eq!(
        loaded.display.max_frame_per_second, 144,
        "read_from should prefer config_sys.json over config.json"
    );
}

// ---------------------------------------------------------------------------
// validate() fills empty paths with defaults
// ---------------------------------------------------------------------------

#[test]
fn validate_fills_empty_paths() {
    let mut config = Config::default();
    config.paths.songpath = String::new();
    config.paths.playerpath = String::new();
    config.paths.skinpath = String::new();
    config.paths.tablepath = String::new();
    config.paths.songinfopath = String::new();

    config.validate();

    assert_eq!(config.paths.songpath, "songdata.db");
    assert_eq!(config.paths.playerpath, "player");
    assert_eq!(config.paths.skinpath, "skin");
    assert_eq!(config.paths.tablepath, "table");
    assert_eq!(config.paths.songinfopath, "songinfo.db");
}

// ---------------------------------------------------------------------------
// validate() clamps out-of-range values
// ---------------------------------------------------------------------------

#[test]
fn validate_clamps_values() {
    let mut config = Config::default();
    config.display.max_frame_per_second = 999_999;
    config.display.window_width = -1;
    config.display.window_height = -1;
    config.select.max_search_bar_count = 0;
    config.select.scrolldurationlow = 1;
    config.select.scrolldurationhigh = 0;
    config.network.ir_send_count = 0;

    config.validate();

    // max_frame_per_second clamped to [0, 50000]
    assert_eq!(config.display.max_frame_per_second, 50000);

    // window_width clamped to [SD.width (640), ULTRAHD.width (3840)]
    assert_eq!(config.display.window_width, 640);

    // window_height clamped to [SD.height (480), ULTRAHD.height (2160)]
    assert_eq!(config.display.window_height, 480);

    // max_search_bar_count clamped to [1, 100]
    assert_eq!(config.select.max_search_bar_count, 1);

    // scrolldurationlow clamped to [2, 1000]
    assert_eq!(config.select.scrolldurationlow, 2);

    // scrolldurationhigh clamped to [1, 1000]
    assert_eq!(config.select.scrolldurationhigh, 1);

    // ir_send_count clamped to [1, 100]
    assert_eq!(config.network.ir_send_count, 1);
}

// ---------------------------------------------------------------------------
// validate_config() calls PlayerConfig::init which creates directories
// ---------------------------------------------------------------------------

#[test]
fn validate_config_calls_player_init() {
    let dir = TempDir::new().unwrap();
    let player_dir = dir.path().join("my_players");

    assert!(
        !player_dir.exists(),
        "Player directory should not exist before validate_config"
    );

    let mut config = Config::default();
    config.paths.playerpath = player_dir.to_string_lossy().to_string();

    let _config = Config::validate_config(config).unwrap();

    assert!(
        player_dir.exists(),
        "PlayerConfig::init should have created the player directory"
    );
}
