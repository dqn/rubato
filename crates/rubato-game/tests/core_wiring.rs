// Phase 8: Wiring & Assembly — Verify open→migrate→query chains work end-to-end
//
// These tests verify that the production assembly order works correctly:
// constructing objects via the same code path as production, then asserting
// the first meaningful action succeeds.

use rubato_game::core::score_data::ScoreData;
use rubato_game::core::score_database_accessor::ScoreDatabaseAccessor;

/// ScoreDatabaseAccessor: open → create_table → query chain.
///
/// This is a smoke test for the full lifecycle: create a fresh temp DB,
/// open it, run migrations (create_table), then query. The query should
/// return empty results (no data inserted), proving the schema is valid.
#[test]
fn wiring_score_database_accessor_open_migrate_query() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("score.db");

    // Open — creates the DB file and connection
    let accessor = ScoreDatabaseAccessor::new(&db_path.to_string_lossy()).unwrap();

    // Migrate — creates tables (score, player, info)
    accessor.create_table().expect("create table");

    // Query — should return None (no data)
    let score = accessor.score_data("nonexistent_hash", 0);
    assert!(score.is_none(), "Fresh DB should have no score data");

    // Query player data — create_table inserts a default PlayerData row
    let player = accessor.player_data();
    assert!(
        player.is_some(),
        "create_table should insert a default PlayerData row"
    );
}

/// ScoreDatabaseAccessor: open → migrate → insert → query roundtrip.
///
/// Verifies that data can be written and read back correctly through
/// the full accessor chain.
#[test]
fn wiring_score_database_accessor_insert_and_query_roundtrip() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("score.db");

    let accessor = ScoreDatabaseAccessor::new(&db_path.to_string_lossy()).unwrap();
    accessor.create_table().expect("create table");

    // Insert a score
    let score = ScoreData {
        sha256: "test_hash_abc".to_string(),
        mode: 0,
        clear: 5,
        notes: 1000,
        ..Default::default()
    };
    accessor.set_score_data(&score);

    // Query it back
    let result = accessor.score_data("test_hash_abc", 0);
    assert!(result.is_some(), "Inserted score should be retrievable");
    let result = result.unwrap();
    assert_eq!(result.sha256, "test_hash_abc");
    assert_eq!(result.clear, 5);
    assert_eq!(result.notes, 1000);
}

/// PlayerConfig::default() returns sensible defaults without requiring a config file.
///
/// This documents that PlayerConfig can be constructed without any filesystem
/// dependency, which is important for first-run scenarios and testing.
#[test]
fn wiring_player_config_defaults_without_file() {
    let pc = rubato_game::core::player_config::PlayerConfig::default();

    // Name should have a default value
    assert_eq!(pc.name, "NO NAME");

    // Gauge should be 0 (normal)
    assert_eq!(pc.play_settings.gauge, 0);

    // Judge timing should be 0 (no offset)
    assert_eq!(pc.judge_settings.judgetiming, 0);

    // Target list should not be empty
    assert!(!pc.select_settings.targetlist.is_empty());

    // Autosave replay should have 4 slots, all 0
    assert_eq!(pc.misc_settings.autosavereplay.len(), 4);
}

/// PlayerConfig::read_player_config from a nonexistent path returns defaults
/// (does not panic).
///
/// This is important for first-run scenarios where no player directory exists yet.
#[test]
fn wiring_player_config_read_from_nonexistent_path_returns_defaults() {
    let result = rubato_game::core::player_config::PlayerConfig::read_player_config(
        "/nonexistent/path/that/does/not/exist",
        "nonexistent_player",
    );

    // Should succeed with defaults (not panic or error)
    assert!(
        result.is_ok(),
        "read_player_config should not fail for nonexistent path"
    );

    let pc = result.unwrap();

    // Should have the player ID set
    assert_eq!(pc.id, Some("nonexistent_player".to_string()));

    // Should have default name
    assert_eq!(pc.name, "NO NAME");
}
