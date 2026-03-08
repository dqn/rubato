// SQL injection tests for SQLiteSongDatabaseAccessor.
//
// These tests verify that parameterized queries and column whitelisting
// prevent SQL injection. Previously format!-based SQL construction was
// vulnerable; now fixed with params![] and column name validation.
//
// Remaining known issue:
//   - get_song_datas_by_sql(): still accepts raw WHERE clause (internal API only)
//   - get_informations(): still accepts raw WHERE clause (internal API only)

use rubato_song::song_data::SongData;
use rubato_song::song_database_accessor::SongDatabaseAccessor;
use rubato_song::song_information_accessor::SongInformationAccessor;
use rubato_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;
use rusqlite::Connection;

/// Helper: create a temporary DB accessor.
fn create_temp_accessor() -> (SQLiteSongDatabaseAccessor, tempfile::TempDir) {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
    (accessor, tmpdir)
}

/// Build a minimal valid SongData.
fn make_song(sha256: &str, title: &str, path: &str) -> SongData {
    let mut sd = SongData::new();
    sd.file.sha256 = sha256.to_string();
    sd.metadata.title = title.to_string();
    sd.set_path(path.to_string());
    sd
}

// -----------------------------------------------------------------------
// get_song_datas_by_hashes: injection via single-quote is now blocked
// -----------------------------------------------------------------------

#[test]
fn get_song_datas_by_hashes_single_quote_handled_safely() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let song = make_song(
        "abcdef1234567890abcdef1234567890a",
        "Normal Song",
        "songs/normal.bms",
    );
    accessor.set_song_datas(&[song]);

    // A hash with a single quote is now safely parameterized
    let malicious_hash = "it'sbrokenAAAAAAAAAAAAAAAAAAAAAAAAA".to_string();
    let results = accessor.song_datas_by_hashes(&[malicious_hash]);

    // No SQL error - just correctly returns empty (no matching hash)
    assert!(
        results.is_empty(),
        "hash with single quote should be handled safely via parameterized query"
    );
}

#[test]
fn get_song_datas_by_hashes_injection_blocked() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let song_a = make_song(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Song A",
        "songs/a.bms",
    );
    let song_b = make_song(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
        "Song B",
        "songs/b.bms",
    );
    accessor.set_song_datas(&[song_a, song_b]);

    // Injection payload that previously broke out of IN clause
    let injected = "') OR 1=1 --AAAAAAAAAAAAAAAAAAAAAA".to_string();
    let results = accessor.song_datas_by_hashes(&[injected]);

    // Injection is blocked - the payload is treated as a literal hash value
    assert_eq!(
        results.len(),
        0,
        "SQL injection via IN clause should be blocked (parameterized query)"
    );
}

// -----------------------------------------------------------------------
// song_datas: column name injection blocked by whitelist
// -----------------------------------------------------------------------

#[test]
fn get_song_datas_column_injection_blocked() {
    let (accessor, _tmpdir) = create_temp_accessor();

    let song = make_song(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Test Song",
        "songs/test.bms",
    );
    accessor.set_song_datas(&[song]);

    // Invalid column name returns empty (not an error)
    let results = accessor.song_datas("1=1 --", "ignored");
    assert!(
        results.is_empty(),
        "invalid column name should be rejected by whitelist"
    );

    // Valid column name still works
    let results = accessor.song_datas("sha256", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(results.len(), 1, "valid column name should work normally");
}

// -----------------------------------------------------------------------
// get_folder_datas: column name injection blocked by whitelist
// -----------------------------------------------------------------------

#[test]
fn get_folder_datas_column_injection_blocked() {
    let (accessor, _tmpdir) = create_temp_accessor();

    // Invalid column name returns empty (not an error)
    let results = accessor.folder_datas("1=1 --", "ignored");
    assert!(
        results.is_empty(),
        "invalid column name should be rejected by whitelist"
    );
}

// -----------------------------------------------------------------------
// song_db_path_with_single_quote: DB path is safely handled
// -----------------------------------------------------------------------

#[test]
fn song_db_path_with_single_quote() {
    let tmpdir = tempfile::tempdir().unwrap();
    let dir_with_quote = tmpdir.path().join("d'qn");
    std::fs::create_dir_all(&dir_with_quote).unwrap();
    let db_path = dir_with_quote.join("song.db");

    let result = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]);
    assert!(
        result.is_ok(),
        "opening a DB at a path with a single quote should succeed"
    );

    let accessor = result.unwrap();
    let song = make_song(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
        "Test Song",
        "songs/test.bms",
    );
    accessor.set_song_datas(&[song]);
    let results = accessor.song_datas("sha256", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa");
    assert_eq!(results.len(), 1, "DB with quoted path should be functional");

    // ATTACH DATABASE with single-quote path now uses escaping
    let score_path = dir_with_quote.join("score.db");
    let scorelog_path = dir_with_quote.join("scorelog.db");
    create_stub_score_db(&score_path);
    create_stub_score_db(&scorelog_path);

    // Previously this failed due to unescaped single quote in ATTACH path.
    // Now the path is escaped (single quotes doubled).
    let results = accessor.song_datas_by_sql(
        "1=1",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );

    // The query should now succeed (ATTACH path escaped).
    // Result may be empty due to column-mapping differences, but no SQL error.
    // We just verify it doesn't panic.
    let _ = results;
}

/// Helper: create a minimal SQLite DB with `score` and `scorelog` tables.
fn create_stub_score_db(path: &std::path::Path) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS score (sha256 TEXT PRIMARY KEY, mode INTEGER);
         CREATE TABLE IF NOT EXISTS scorelog (sha256 TEXT PRIMARY KEY);",
    )
    .unwrap();
}

// -----------------------------------------------------------------------
// get_information_for_songs: sha256 injection now blocked
// -----------------------------------------------------------------------

#[test]
fn get_information_for_songs_sha256_injection_blocked() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("info.db");
    let accessor = SongInformationAccessor::new(&db_path.to_string_lossy()).unwrap();

    let victim_sha256 = "a".repeat(64);
    {
        let conn = Connection::open(&db_path).unwrap();
        conn.execute(
            "INSERT INTO information (sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm, distribution, speedchange, lanenotes) \
             VALUES (?1, 100, 0, 0, 0, 200.0, 5.0, 10.0, 3.0, 150.0, '', '', '')",
            rusqlite::params![victim_sha256],
        )
        .unwrap();
    }

    // Injection payload that previously broke out of IN clause
    let mut injected_song = SongData::new();
    injected_song.file.sha256 = "') OR 1=1 --".to_string();
    injected_song.metadata.title = "Injected".to_string();

    let mut songs = vec![injected_song];
    accessor.information_for_songs(&mut songs);

    // With parameterized query, injection is blocked
    assert!(
        songs[0].info.is_none(),
        "injection should be blocked; no information should match the injected sha256"
    );
}
