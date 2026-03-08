use rubato_song::song_data::SongData;
use rubato_song::song_database_accessor::SongDatabaseAccessor;
use rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;
use tempfile::TempDir;

/// Helper: create a temporary DB accessor.
fn create_temp_accessor() -> (SQLiteSongDatabaseAccessor, TempDir) {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
    (accessor, tmpdir)
}

/// Helper: build a minimal valid SongData.
/// SongData::validate() requires non-empty title AND at least one of md5/sha256.
fn make_song(sha256: &str, title: &str, path: &str) -> SongData {
    let mut sd = SongData::new();
    sd.file.sha256 = sha256.to_string();
    sd.metadata.title = title.to_string();
    sd.file.set_path(path.to_string());
    sd
}

#[test]
fn new_creates_tables() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");

    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    // The DB file should exist on disk after construction.
    assert!(db_path.exists(), "Database file should be created");

    // Both song and folder tables should be queryable without error.
    let songs = accessor.song_datas("md5", "nonexistent");
    assert!(songs.is_empty());
    let folders = accessor.folder_datas("path", "nonexistent");
    assert!(folders.is_empty());
}

#[test]
fn insert_and_query_song() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let song = make_song("abc123", "Test Song", "songs/test.bms");
    accessor.set_song_datas(&[song]);

    let results = accessor.song_datas("sha256", "abc123");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "Test Song");
    assert_eq!(results[0].file.sha256, "abc123");
    assert_eq!(results[0].file.path(), Some("songs/test.bms"));
}

#[test]
fn insert_and_query_by_text() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let mut song = make_song("sha_text1", "Starlight Symphony", "songs/starlight.bms");
    song.metadata.artist = "Aurora".to_string();
    accessor.set_song_datas(&[song]);

    // Search by title fragment
    let results = accessor.song_datas_by_text("Starlight");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "Starlight Symphony");

    // Search by artist
    let results = accessor.song_datas_by_text("Aurora");
    assert_eq!(results.len(), 1);

    // Search for something that does not exist
    let results = accessor.song_datas_by_text("nonexistent_query_xyz");
    assert!(results.is_empty());
}

#[test]
fn empty_db_returns_empty() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let results = accessor.song_datas("sha256", "does_not_exist");
    assert!(results.is_empty());

    let results = accessor.song_datas("md5", "");
    assert!(results.is_empty());

    let results = accessor.song_datas_by_hashes(&["nonexistent_hash".to_string()]);
    assert!(results.is_empty());

    let results = accessor.folder_datas("path", "/no/such/folder");
    assert!(results.is_empty());
}

#[test]
fn reopen_preserves_data() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");

    // First session: create DB and insert a song.
    {
        let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
        let song = make_song("persist_sha256", "Persistent Song", "songs/persist.bms");
        accessor.set_song_datas(&[song]);

        // Sanity check within the same session.
        let results = accessor.song_datas("sha256", "persist_sha256");
        assert_eq!(results.len(), 1);
    }
    // accessor is dropped here, closing the connection.

    // Second session: reopen the same DB file and verify data survived.
    {
        let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
        let results = accessor.song_datas("sha256", "persist_sha256");
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].metadata.title, "Persistent Song");
        assert_eq!(results[0].file.path(), Some("songs/persist.bms"));
    }
}
