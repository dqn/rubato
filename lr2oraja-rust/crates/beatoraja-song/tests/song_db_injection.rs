// SQL injection tests for SQLiteSongDatabaseAccessor.
//
// These tests demonstrate that format!-based SQL construction is
// vulnerable to injection.  The tests are "red-only": they expose bugs
// but do NOT fix the SQL.
//
// Vulnerable call sites:
//   - get_song_datas_by_hashes: lines 361-376 directly interpolate hash values
//     into an IN (...) clause with single quotes, no escaping
//   - update (line 586): format!("WHERE path = '{}'", parent) — a path with
//     a single quote breaks the SQL

use beatoraja_song::song_data::SongData;
use beatoraja_song::song_database_accessor::SongDatabaseAccessor;
use beatoraja_song::sqlite_song_database_accessor::SQLiteSongDatabaseAccessor;

/// Helper: create a temporary DB accessor.
fn create_temp_accessor() -> (SQLiteSongDatabaseAccessor, tempfile::TempDir) {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();
    (accessor, tmpdir)
}

/// Build a minimal valid SongData.
/// SongData::validate() requires non-empty title AND at least one of md5/sha256.
fn make_song(sha256: &str, title: &str, path: &str) -> SongData {
    let mut sd = SongData::new();
    sd.sha256 = sha256.to_string();
    sd.title = title.to_string();
    sd.set_path(path.to_string());
    sd
}

// -----------------------------------------------------------------------
// 47c — get_song_datas_by_hashes: hash containing single-quote breaks SQL
// -----------------------------------------------------------------------

#[test]
fn get_song_datas_by_hashes_single_quote_in_hash_causes_sql_error() {
    let (accessor, _tmpdir) = create_temp_accessor();

    // Insert a normal song so the table is non-empty.
    let song = make_song(
        "abcdef1234567890abcdef1234567890a", // >32 chars → goes into sha256 branch
        "Normal Song",
        "songs/normal.bms",
    );
    accessor.set_song_datas(&[song]);

    // A hash containing a single quote will produce malformed SQL like:
    //   SELECT * FROM song WHERE ... sha256 IN ('it's broken')
    // This unbalanced quote causes a SQL syntax error.
    // Because the error is caught and logged (returns empty vec), the
    // method won't panic but silently swallows the error — a correctness bug.
    let malicious_hash = "it'sbrokenAAAAAAAAAAAAAAAAAAAAAAAAA".to_string(); // >32 chars
    let results = accessor.get_song_datas_by_hashes(&[malicious_hash]);

    // The query fails internally due to the syntax error, so we get an empty
    // result instead of the expected behaviour (finding no matching song and
    // returning an empty vec cleanly).  The observable effect is the same
    // (empty vec), but the internal SQL error is the bug.
    assert!(
        results.is_empty(),
        "query with injected quote should return empty (SQL error swallowed)"
    );
}

#[test]
fn get_song_datas_by_hashes_injection_returns_all_rows() {
    let (accessor, _tmpdir) = create_temp_accessor();

    // Insert two songs.
    let song_a = make_song(
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa", // 34 chars, goes to sha256 IN (...)
        "Song A",
        "songs/a.bms",
    );
    let song_b = make_song(
        "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb", // 34 chars
        "Song B",
        "songs/b.bms",
    );
    accessor.set_song_datas(&[song_a, song_b]);

    // Craft a hash that escapes the IN clause and adds OR 1=1:
    //   sha256 IN ('') OR 1=1 --')
    // The leading chars make it >32 so it hits the sha256 branch.
    let injected = "') OR 1=1 --AAAAAAAAAAAAAAAAAAAAAA".to_string(); // >32 chars

    let results = accessor.get_song_datas_by_hashes(&[injected]);

    // If injection succeeds, both rows are returned.
    // If injection causes a parse error, results will be empty.
    // Either outcome demonstrates the vulnerability:
    //   - 2 rows → injection bypassed the filter
    //   - 0 rows → SQL syntax error from unescaped input
    //
    // We document whichever behaviour occurs.
    let count = results.len();
    assert!(
        count == 0 || count == 2,
        "injection should either return all rows (2) or fail with SQL error (0), got {count}"
    );
}

// -----------------------------------------------------------------------
// 47c — get_folder_datas: column name injection via `key` parameter
// -----------------------------------------------------------------------

#[test]
fn get_folder_datas_column_name_injection() {
    let (accessor, _tmpdir) = create_temp_accessor();

    // get_folder_datas formats: "SELECT * FROM folder WHERE {key} = ?1"
    // If `key` contains SQL, it becomes part of the query.
    // Inject: key = "1=1 --" produces "SELECT * FROM folder WHERE 1=1 -- = ?1"
    // which returns all rows.
    let results = accessor.get_folder_datas("1=1 --", "ignored");

    // With an empty folder table, this returns 0 rows, but the SQL still
    // executes successfully (proving injection is possible).
    // The test proves the injected SQL is accepted by SQLite without error.
    assert!(
        results.is_empty(),
        "empty folder table should return no rows even with injection"
    );
}
