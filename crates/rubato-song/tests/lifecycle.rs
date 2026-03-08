// Phase 7: Cross-Subsystem Lifecycle — Song DB reopen and re-initialization
//
// SQLiteSongDatabaseAccessor uses a direct rusqlite::Connection (not OnceLock).
// This means each instance is independently managed. However, re-opening the
// same DB file should work correctly — data should persist across connections.
// This test verifies the drop → reopen lifecycle that happens when the
// application restarts or when the launcher creates a new accessor.

use rubato_song::song_data::SongData;
use rubato_song::song_database_accessor::SongDatabaseAccessor;
use rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

/// Helper: build a minimal valid SongData.
fn make_song(sha256: &str, title: &str, path: &str) -> SongData {
    let mut sd = SongData::new();
    sd.file.sha256 = sha256.to_string();
    sd.metadata.title = title.to_string();
    sd.set_path(path.to_string());
    sd
}

/// Drop and reopen: data survives the first accessor being dropped.
/// This simulates the launcher → game transition where the launcher's
/// accessor is dropped and the game creates a new one from the same file.
#[test]
fn lifecycle_songdb_drop_and_reopen_preserves_data() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");

    // First session: create and insert
    {
        let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
        let song = make_song("lifecycle_sha", "Lifecycle Test", "songs/lifecycle.bms");
        accessor.set_song_datas(&[song]);

        let results = accessor.song_datas("sha256", "lifecycle_sha");
        assert_eq!(results.len(), 1, "Should find song in first session");
    }
    // accessor dropped here — connection closed

    // Second session: reopen and verify data survived
    {
        let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
        let results = accessor.song_datas("sha256", "lifecycle_sha");
        assert_eq!(results.len(), 1, "Song should survive across sessions");
        assert_eq!(results[0].metadata.title, "Lifecycle Test");
    }
}

/// Multiple accessors to the same file at the same time.
/// SQLite supports multiple readers but only one writer at a time.
/// This test documents that creating two accessors to the same file
/// does not cause initialization errors or table creation conflicts.
#[test]
fn lifecycle_songdb_multiple_accessors_same_file_no_conflict() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");

    // Create first accessor (creates tables)
    let accessor1 = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    // Create second accessor to same file (tables already exist — should not error)
    let accessor2 = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    // Insert via accessor1, read via accessor2
    let song = make_song("multi_sha", "Multi Access", "songs/multi.bms");
    accessor1.set_song_datas(&[song]);

    let results = accessor2.song_datas("sha256", "multi_sha");
    assert_eq!(
        results.len(),
        1,
        "Second accessor should see data from first"
    );
}
