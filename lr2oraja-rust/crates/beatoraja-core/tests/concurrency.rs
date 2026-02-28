// Phase 12: Concurrency — Test concurrent ScoreDatabaseAccessor access
//
// rusqlite::Connection is Send but not Sync, so a single ScoreDatabaseAccessor
// cannot be shared across threads directly. This test verifies that multiple
// ScoreDatabaseAccessor instances can safely read from the same SQLite DB file
// concurrently, which is the realistic multi-threaded access pattern.

use std::sync::Arc;
use std::thread;

use beatoraja_core::score_data::ScoreData;
use beatoraja_core::score_database_accessor::ScoreDatabaseAccessor;

/// Two threads reading from the same DB file concurrently via separate connections.
///
/// This verifies that SQLite's default locking is sufficient for concurrent reads,
/// and that ScoreDatabaseAccessor doesn't have any hidden global state that would
/// cause race conditions.
#[test]
fn concurrency_score_db_read_access_separate_connections() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("score.db");

    // Create and populate the DB
    {
        let accessor = ScoreDatabaseAccessor::new(&db_path.to_string_lossy()).unwrap();
        accessor.create_table();

        // Insert test data
        let score = ScoreData {
            sha256: "concurrent_test_hash".to_string(),
            mode: 0,
            clear: 7,
            notes: 2000,
            ..Default::default()
        };
        accessor.set_score_data(&score);
    }

    let db_path_str = Arc::new(db_path.to_string_lossy().to_string());

    // Spawn two reader threads, each with its own connection
    let path1 = Arc::clone(&db_path_str);
    let path2 = Arc::clone(&db_path_str);

    let handle1 = thread::spawn(move || {
        let accessor = ScoreDatabaseAccessor::new(&path1).unwrap();
        let result = accessor.get_score_data("concurrent_test_hash", 0);
        assert!(result.is_some(), "Thread 1 should read the score");
        assert_eq!(result.unwrap().clear, 7);
    });

    let handle2 = thread::spawn(move || {
        let accessor = ScoreDatabaseAccessor::new(&path2).unwrap();
        let result = accessor.get_score_data("concurrent_test_hash", 0);
        assert!(result.is_some(), "Thread 2 should read the score");
        assert_eq!(result.unwrap().clear, 7);
    });

    // Both threads should complete without panic or deadlock
    handle1.join().expect("Thread 1 panicked");
    handle2.join().expect("Thread 2 panicked");
}

/// Concurrent read and write to the same DB file via separate connections.
///
/// Tests that SQLite's WAL mode (if enabled) or default locking handles
/// the case where one thread reads while another writes.
#[test]
fn concurrency_score_db_read_write_access() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("score_rw.db");

    // Create the DB with initial data
    {
        let accessor = ScoreDatabaseAccessor::new(&db_path.to_string_lossy()).unwrap();
        accessor.create_table();

        let score = ScoreData {
            sha256: "rw_test_hash".to_string(),
            mode: 0,
            clear: 3,
            notes: 500,
            ..Default::default()
        };
        accessor.set_score_data(&score);
    }

    let db_path_str = Arc::new(db_path.to_string_lossy().to_string());
    let path_reader = Arc::clone(&db_path_str);
    let path_writer = Arc::clone(&db_path_str);

    // Reader thread: reads existing data
    let reader = thread::spawn(move || {
        let accessor = ScoreDatabaseAccessor::new(&path_reader).unwrap();
        // Read multiple times to increase chance of concurrent access
        for _ in 0..10 {
            let result = accessor.get_score_data("rw_test_hash", 0);
            assert!(result.is_some(), "Reader should find the score");
        }
    });

    // Writer thread: writes new data
    let writer = thread::spawn(move || {
        let accessor = ScoreDatabaseAccessor::new(&path_writer).unwrap();
        for i in 0..10 {
            let score = ScoreData {
                sha256: format!("new_hash_{}", i),
                mode: 0,
                clear: i,
                notes: 100,
                ..Default::default()
            };
            accessor.set_score_data(&score);
        }
    });

    // Both threads should complete without panic or deadlock
    reader.join().expect("Reader thread panicked");
    writer.join().expect("Writer thread panicked");
}
