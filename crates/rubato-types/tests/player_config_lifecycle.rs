// Integration test: PlayerConfig lifecycle (init -> create -> read -> write)
//
// Tests the full PlayerConfig lifecycle including directory creation,
// config file I/O, and round-trip serialization.

use rubato_types::config::Config;
use rubato_types::player_config::{PlayerConfig, create_player, read_all_player_id};
use tempfile::TempDir;

/// Helper: create a Config with playerpath pointing to a subdirectory of the given tempdir.
fn config_with_playerpath(tempdir: &TempDir, subdir: &str) -> Config {
    Config {
        paths: rubato_types::config::PathConfig {
            playerpath: tempdir.path().join(subdir).to_string_lossy().to_string(),
            ..Default::default()
        },
        ..Default::default()
    }
}

// ---------------------------------------------------------------------------
// init()
// ---------------------------------------------------------------------------

#[test]
fn init_creates_player_directory() {
    let tempdir = TempDir::new().unwrap();
    let mut config = config_with_playerpath(&tempdir, "players");

    // The "players" subdirectory does not exist yet
    assert!(!std::path::Path::new(&config.paths.playerpath).exists());

    PlayerConfig::init(&mut config).unwrap();

    // After init, the playerpath directory should exist
    assert!(std::path::Path::new(&config.paths.playerpath).is_dir());
}

#[test]
fn init_creates_default_player_and_sets_playername() {
    let tempdir = TempDir::new().unwrap();
    let mut config = config_with_playerpath(&tempdir, "players");

    PlayerConfig::init(&mut config).unwrap();

    // Should have created a "player1" subdirectory
    let player1_dir = std::path::Path::new(&config.paths.playerpath).join("player1");
    assert!(player1_dir.is_dir(), "player1 directory should exist");

    // config.playername should be set to "player1"
    assert_eq!(config.playername, Some("player1".to_string()));
}

#[test]
fn init_noop_when_players_exist() {
    let tempdir = TempDir::new().unwrap();
    let mut config = config_with_playerpath(&tempdir, "players");

    // Pre-create the playerpath and an existing player subdirectory
    let existing_player_dir =
        std::path::Path::new(&config.paths.playerpath).join("existing_player");
    std::fs::create_dir_all(&existing_player_dir).unwrap();

    // Set playername to None so we can verify it stays unchanged
    config.playername = None;

    PlayerConfig::init(&mut config).unwrap();

    // No new "player1" directory should have been created
    let player1_dir = std::path::Path::new(&config.paths.playerpath).join("player1");
    assert!(
        !player1_dir.exists(),
        "player1 should NOT be created when players already exist"
    );

    // playername should remain unchanged (None)
    assert_eq!(config.playername, None);

    // Only the existing player directory should be present
    let ids = read_all_player_id(&config.paths.playerpath);
    assert_eq!(ids.len(), 1);
    assert!(ids.contains(&"existing_player".to_string()));
}

#[ignore = "init() looks for playerscore.db relative to CWD, which makes this test environment-dependent"]
#[test]
fn init_copies_score_db_if_exists() {
    // This test is intentionally ignored because PlayerConfig::init() looks for
    // "playerscore.db" in the current working directory. Testing this properly
    // would require changing CWD, which is not thread-safe and could interfere
    // with other tests running in parallel.
}

// ---------------------------------------------------------------------------
// create_player()
// ---------------------------------------------------------------------------

#[test]
fn create_player_writes_config_file() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir.path().to_string_lossy().to_string();

    create_player(&playerpath, "testplayer").unwrap();

    // The directory should exist
    let player_dir = tempdir.path().join("testplayer");
    assert!(player_dir.is_dir(), "Player directory should be created");

    // The config file should exist
    let config_file = player_dir.join("config_player.json");
    assert!(
        config_file.is_file(),
        "config_player.json should be created"
    );

    // The config file should be valid JSON containing the player id
    let content = std::fs::read_to_string(&config_file).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(parsed["id"], "testplayer");
}

// ---------------------------------------------------------------------------
// read / write round-trip
// ---------------------------------------------------------------------------

#[test]
fn read_write_roundtrip() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir.path().to_string_lossy().to_string();

    // Create the player directory
    let player_dir = tempdir.path().join("roundtrip_player");
    std::fs::create_dir_all(&player_dir).unwrap();

    // Build a PlayerConfig with custom values
    let player = PlayerConfig {
        id: Some("roundtrip_player".to_string()),
        name: "test".to_string(),
        play_settings: rubato_types::player_config::PlaySettings {
            gauge: 3,
            random: 5,
            ..Default::default()
        },
        ..Default::default()
    };

    // Write it
    PlayerConfig::write(&playerpath, &player).unwrap();

    // Read it back
    let read_back = PlayerConfig::read_player_config(&playerpath, "roundtrip_player").unwrap();

    assert_eq!(read_back.id, Some("roundtrip_player".to_string()));
    assert_eq!(read_back.name, "test");
    assert_eq!(read_back.play_settings.gauge, 3);
    assert_eq!(read_back.play_settings.random, 5);
}

#[test]
fn read_nonexistent_returns_default() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir.path().to_string_lossy().to_string();

    // Create the player directory (but no config file inside)
    let player_dir = tempdir.path().join("ghost_player");
    std::fs::create_dir_all(&player_dir).unwrap();

    // Reading a player that has a directory but no config file should return defaults
    let player = PlayerConfig::read_player_config(&playerpath, "ghost_player").unwrap();

    // id should be set to the requested playerid
    assert_eq!(player.id, Some("ghost_player".to_string()));

    // Other fields should be defaults
    assert_eq!(player.name, "NO NAME");
    assert_eq!(player.play_settings.gauge, 0);
    assert_eq!(player.play_settings.random, 0);
}

// ---------------------------------------------------------------------------
// write creates parent directory (self-sufficiency)
// ---------------------------------------------------------------------------

#[test]
fn write_creates_parent_directory_when_missing() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir
        .path()
        .join("nonexistent")
        .to_string_lossy()
        .to_string();

    // The playerpath directory does not exist yet
    assert!(!std::path::Path::new(&playerpath).exists());

    let player = PlayerConfig {
        id: Some("newplayer".to_string()),
        name: "Fresh Player".to_string(),
        ..Default::default()
    };

    // write() should create the directory and succeed
    PlayerConfig::write(&playerpath, &player).unwrap();

    // Verify the config file was written
    let config_file = tempdir
        .path()
        .join("nonexistent")
        .join("newplayer")
        .join("config_player.json");
    assert!(
        config_file.is_file(),
        "config_player.json should be created"
    );

    // Verify content is correct
    let read_back = PlayerConfig::read_player_config(&playerpath, "newplayer").unwrap();
    assert_eq!(read_back.name, "Fresh Player");
}

// ---------------------------------------------------------------------------
// read_all_player_id()
// ---------------------------------------------------------------------------

#[test]
fn read_all_player_id_lists_directories() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir.path().to_string_lossy().to_string();

    // Create multiple player subdirectories
    std::fs::create_dir(tempdir.path().join("alice")).unwrap();
    std::fs::create_dir(tempdir.path().join("bob")).unwrap();
    std::fs::create_dir(tempdir.path().join("charlie")).unwrap();

    // Also create a file (should NOT appear in results)
    std::fs::write(tempdir.path().join("not_a_player.txt"), "ignored").unwrap();

    let mut ids = read_all_player_id(&playerpath);
    ids.sort();

    assert_eq!(ids, vec!["alice", "bob", "charlie"]);
}

// ---------------------------------------------------------------------------
// Full lifecycle: init -> read
// ---------------------------------------------------------------------------

#[test]
fn init_then_read_full_cycle() {
    let tempdir = TempDir::new().unwrap();
    let mut config = config_with_playerpath(&tempdir, "players");

    // init creates "player1"
    PlayerConfig::init(&mut config).unwrap();

    // Read the config that was created by init
    let player = PlayerConfig::read_player_config(&config.paths.playerpath, "player1").unwrap();

    assert_eq!(player.id, Some("player1".to_string()));
    // Name should be the default since init creates a default PlayerConfig
    assert_eq!(player.name, "NO NAME");
}

// ---------------------------------------------------------------------------
// write preserves player id
// ---------------------------------------------------------------------------

#[test]
fn write_preserves_player_id() {
    let tempdir = TempDir::new().unwrap();
    let playerpath = tempdir.path().to_string_lossy().to_string();

    // Create the player directory
    let player_dir = tempdir.path().join("myid");
    std::fs::create_dir_all(&player_dir).unwrap();

    // Build a PlayerConfig with a specific id
    let player = PlayerConfig {
        id: Some("myid".to_string()),
        name: "Custom Player".to_string(),
        ..Default::default()
    };

    // Write it
    PlayerConfig::write(&playerpath, &player).unwrap();

    // Read it back
    let read_back = PlayerConfig::read_player_config(&playerpath, "myid").unwrap();

    assert_eq!(read_back.id, Some("myid".to_string()));
    assert_eq!(read_back.name, "Custom Player");
}
