use super::*;

fn create_test_accessor() -> SQLiteSongDatabaseAccessor {
    SQLiteSongDatabaseAccessor::new(":memory:", &[]).unwrap()
}

fn make_test_song(md5: &str, sha256: &str, title: &str) -> SongData {
    let mut sd = SongData::new();
    sd.file.md5 = md5.to_string();
    sd.file.sha256 = sha256.to_string();
    sd.metadata.title = title.to_string();
    sd.file.set_path(format!("test/{}.bms", title));
    sd
}

/// Verify that busy_timeout is set on the connection so that concurrent
/// writers retry instead of immediately failing with SQLITE_BUSY.
#[test]
fn test_connection_has_busy_timeout() {
    let accessor = create_test_accessor();
    let conn = lock_or_recover(&accessor.conn);
    let timeout: i64 = conn
        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
        .unwrap();
    assert!(
        timeout >= 5000,
        "busy_timeout should be at least 5000ms, got {}",
        timeout
    );
}

#[test]
fn test_new_creates_tables() {
    let accessor = create_test_accessor();
    // Verify tables exist by querying them
    let songs = accessor.song_datas("md5", "nonexistent");
    assert!(songs.is_empty());
    let folders = accessor.folder_datas("path", "nonexistent");
    assert!(folders.is_empty());
}

#[test]
fn test_insert_and_get_song_by_md5() {
    let accessor = create_test_accessor();
    let song = make_test_song("abc123", "sha_abc123", "Test Song");
    accessor.insert_song(&song).unwrap();

    let results = accessor.song_datas("md5", "abc123");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "Test Song");
    assert_eq!(results[0].file.md5, "abc123");
}

#[test]
fn test_insert_and_get_song_by_sha256() {
    let accessor = create_test_accessor();
    let song = make_test_song("md5_xyz", "sha256_xyz", "SHA Test");
    accessor.insert_song(&song).unwrap();

    let results = accessor.song_datas("sha256", "sha256_xyz");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "SHA Test");
}

#[test]
fn test_get_song_datas_empty() {
    let accessor = create_test_accessor();
    let results = accessor.song_datas("md5", "nonexistent");
    assert!(results.is_empty());
}

#[test]
fn test_get_song_datas_by_hashes() {
    let accessor = create_test_accessor();
    // SHA256 hashes must be > 32 chars to be classified as sha256
    let sha1 = "a".repeat(64);
    let sha2 = "b".repeat(64);
    let sha3 = "c".repeat(64);
    let song1 = make_test_song("md5_1", &sha1, "Song 1");
    let song2 = make_test_song("md5_2", &sha2, "Song 2");
    let song3 = make_test_song("md5_3", &sha3, "Song 3");
    accessor.insert_song(&song1).unwrap();
    accessor.insert_song(&song2).unwrap();
    accessor.insert_song(&song3).unwrap();

    // Query by sha256 hashes (> 32 chars)
    let hashes = vec![sha1, sha3];
    let results = accessor.song_datas_by_hashes(&hashes);
    assert_eq!(results.len(), 2);
}

#[test]
fn test_get_song_datas_by_hashes_md5() {
    let accessor = create_test_accessor();
    let song1 = make_test_song("md5_short_1", "sha1", "Song Short 1");
    let song2 = make_test_song("md5_short_2", "sha2", "Song Short 2");
    accessor.insert_song(&song1).unwrap();
    accessor.insert_song(&song2).unwrap();

    // Query by md5 hashes (<= 32 chars)
    let hashes = vec!["md5_short_1".to_string()];
    let results = accessor.song_datas_by_hashes(&hashes);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "Song Short 1");
}

#[test]
fn test_get_song_datas_by_text() {
    let accessor = create_test_accessor();
    let mut song = make_test_song("m1", "s1", "Rhythm Action");
    song.metadata.artist = "DJ Test".to_string();
    accessor.insert_song(&song).unwrap();

    let results = accessor.song_datas_by_text("Rhythm");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].metadata.title, "Rhythm Action");

    let results = accessor.song_datas_by_text("DJ Test");
    assert_eq!(results.len(), 1);

    let results = accessor.song_datas_by_text("nonexistent");
    assert!(results.is_empty());
}

#[test]
fn test_set_song_datas_batch() {
    let accessor = create_test_accessor();
    let songs = vec![
        make_test_song("batch_1", "sbatch_1", "Batch Song 1"),
        make_test_song("batch_2", "sbatch_2", "Batch Song 2"),
        make_test_song("batch_3", "sbatch_3", "Batch Song 3"),
    ];

    accessor.set_song_datas(&songs).expect("set_song_datas");

    let results = accessor.song_datas("md5", "batch_1");
    assert_eq!(results.len(), 1);
    let results = accessor.song_datas("md5", "batch_2");
    assert_eq!(results.len(), 1);
    let results = accessor.song_datas("md5", "batch_3");
    assert_eq!(results.len(), 1);
}

#[test]
fn test_insert_and_get_folder() {
    let accessor = create_test_accessor();
    let folder = FolderData {
        title: "Test Folder".to_string(),
        path: "/test/folder/".to_string(),
        parent: "parent_crc".to_string(),
        date: 1000,
        adddate: 2000,
        ..Default::default()
    };
    accessor.insert_folder(&folder).unwrap();

    let results = accessor.folder_datas("path", "/test/folder/");
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].title, "Test Folder");
    assert_eq!(results[0].date, 1000);
}

#[test]
fn test_get_folder_datas_empty() {
    let accessor = create_test_accessor();
    let results = accessor.folder_datas("path", "nonexistent");
    assert!(results.is_empty());
}

#[test]
fn test_add_plugin() {
    let mut accessor = create_test_accessor();
    struct TestPlugin;
    impl SongDatabaseAccessorPlugin for TestPlugin {
        fn update(&self, _model: &BMSModel, song: &mut SongData) {
            song.metadata.tag = "plugin_tag".to_string();
        }
    }
    accessor.add_plugin(Box::new(TestPlugin));
    assert_eq!(accessor.plugins.len(), 1);
}

#[test]
fn test_update_song_datas_scans_bms_files() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("testpack");
    fs::create_dir_all(&bms_dir).unwrap();

    // Write a minimal BMS file
    let bms_content = "\
#PLAYER 1\n\
#GENRE Test\n\
#TITLE Update Test Song\n\
#ARTIST tester\n\
#BPM 120\n\
#PLAYLEVEL 3\n\
#RANK 2\n\
#TOTAL 300\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("test.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    accessor.update_song_datas(None, &bmsroot, true, false, None);

    // Verify the song was inserted
    let songs = accessor.song_datas("title", "Update Test Song");
    assert_eq!(songs.len(), 1);
    assert_eq!(songs[0].metadata.artist, "tester");
    assert!(songs[0].chart.notes > 0);
}

#[test]
fn test_update_song_datas_incremental_skips_unchanged() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("testpack");
    fs::create_dir_all(&bms_dir).unwrap();

    let bms_content = "\
#PLAYER 1\n\
#TITLE Incremental Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("incr.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    // First update
    let listener1 = SongDatabaseUpdateListener::new();
    accessor.update_song_datas_with_listener(None, &bmsroot, false, false, None, &listener1);
    assert_eq!(listener1.new_bms_files_count(), 1);

    // Second update (no changes) - should skip
    let listener2 = SongDatabaseUpdateListener::new();
    accessor.update_song_datas_with_listener(None, &bmsroot, false, false, None, &listener2);
    assert_eq!(listener2.new_bms_files_count(), 0);
    assert_eq!(listener2.bms_files_count(), 1);
}

#[test]
fn test_update_song_datas_creates_folder_records() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("pack1");
    fs::create_dir_all(&bms_dir).unwrap();

    let bms_content = "\
#TITLE Folder Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("folder_test.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    accessor.update_song_datas(None, &bmsroot, true, false, None);

    // Check that folder records were created (at least root and pack1)
    let all_folders: Vec<FolderData> = {
        let conn = accessor.conn.lock().unwrap();
        let mut stmt = conn.prepare("SELECT * FROM folder").unwrap();
        let rows = stmt
            .query_map([], |row| {
                Ok(FolderData {
                    title: row.get::<_, String>(0).unwrap_or_default(),
                    subtitle: row.get::<_, String>(1).unwrap_or_default(),
                    command: row.get::<_, String>(2).unwrap_or_default(),
                    path: row.get::<_, String>(3).unwrap_or_default(),
                    banner: row.get::<_, String>(4).unwrap_or_default(),
                    parent: row.get::<_, String>(5).unwrap_or_default(),
                    folder_type: row.get::<_, i32>(6).unwrap_or(0),
                    date: row.get::<_, i64>(7).unwrap_or(0),
                    adddate: row.get::<_, i64>(8).unwrap_or(0),
                    max: row.get::<_, i32>(9).unwrap_or(0),
                })
            })
            .unwrap();
        rows.flatten().collect()
    };
    assert!(
        !all_folders.is_empty(),
        "Folder records should be created during update"
    );
}

#[test]
fn test_update_song_datas_empty_bmsroot() {
    let accessor = create_test_accessor();
    // Should not panic, just log warning and return
    accessor.update_song_datas(None, &[], true, false, None);
}

#[test]
fn test_update_song_datas_preserves_favorites() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("favpack");
    fs::create_dir_all(&bms_dir).unwrap();

    let bms_content = "\
#TITLE Favorite Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("fav.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    // First update
    accessor.update_song_datas(None, &bmsroot, true, false, None);

    // Set favorite on the song
    let songs = accessor.song_datas("title", "Favorite Test");
    assert_eq!(songs.len(), 1);
    let sha256 = songs[0].file.sha256.clone();
    let conn = accessor.conn.lock().unwrap();
    let _ = conn.execute(
        "UPDATE song SET favorite = 3 WHERE sha256 = ?1",
        rusqlite::params![sha256],
    );
    drop(conn);

    // Full re-update (updateAll=true)
    accessor.update_song_datas(None, &bmsroot, true, false, None);

    // Verify favorite is preserved
    let songs = accessor.song_datas("title", "Favorite Test");
    assert_eq!(songs.len(), 1);
    assert_eq!(
        songs[0].favorite, 3,
        "Favorite should be preserved across updates"
    );
}

#[test]
fn test_update_song_datas_auto_difficulty() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("diffpack");
    fs::create_dir_all(&bms_dir).unwrap();

    // "beginner" in subtitle -> difficulty 1
    let bms_content = "\
#TITLE Test\n\
#SUBTITLE beginner\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("diff.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    accessor.update_song_datas(None, &bmsroot, true, false, None);

    let songs = accessor.song_datas("title", "Test");
    assert_eq!(songs.len(), 1);
    assert_eq!(
        songs[0].chart.difficulty, 1,
        "Beginner subtitle should set difficulty to 1"
    );
}

/// Verify that set_song_datas holds the connection lock for the entire
/// transaction, preventing concurrent callers from interleaving SQL
/// statements. Two threads each call set_song_datas with disjoint song
/// batches. After both complete, all songs from both batches must be
/// present (no lost writes due to interleaved BEGIN/COMMIT).
#[test]
fn test_set_song_datas_concurrent_no_interleaving() {
    use std::sync::Arc;

    let db_path = tempfile::NamedTempFile::new().unwrap();
    let accessor =
        Arc::new(SQLiteSongDatabaseAccessor::new(&db_path.path().to_string_lossy(), &[]).unwrap());

    let batch_a: Vec<SongData> = (0..50)
        .map(|i| {
            make_test_song(
                &format!("md5_a_{i}"),
                &format!("sha_a_{i}"),
                &format!("A {i}"),
            )
        })
        .collect();
    let batch_b: Vec<SongData> = (0..50)
        .map(|i| {
            make_test_song(
                &format!("md5_b_{i}"),
                &format!("sha_b_{i}"),
                &format!("B {i}"),
            )
        })
        .collect();

    let acc_a = Arc::clone(&accessor);
    let ba = batch_a.clone();
    let handle_a = std::thread::spawn(move || {
        acc_a.set_song_datas(&ba).expect("set_song_datas thread A");
    });

    let acc_b = Arc::clone(&accessor);
    let bb = batch_b.clone();
    let handle_b = std::thread::spawn(move || {
        acc_b.set_song_datas(&bb).expect("set_song_datas thread B");
    });

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    // All 100 songs must be present (no lost writes from interleaving)
    for i in 0..50 {
        let results = accessor.song_datas("md5", &format!("md5_a_{i}"));
        assert_eq!(results.len(), 1, "missing song A {i}");
        let results = accessor.song_datas("md5", &format!("md5_b_{i}"));
        assert_eq!(results.len(), 1, "missing song B {i}");
    }
}

/// Verify that set_song_datas is atomic: either all songs are inserted or
/// none. This tests the transaction boundary by confirming batch insert
/// completes as a unit.
#[test]
fn test_set_song_datas_transaction_atomicity() {
    let accessor = create_test_accessor();
    let songs: Vec<SongData> = (0..10)
        .map(|i| {
            make_test_song(
                &format!("atomic_md5_{i}"),
                &format!("atomic_sha_{i}"),
                &format!("Atomic Song {i}"),
            )
        })
        .collect();

    accessor.set_song_datas(&songs).expect("set_song_datas");

    // All 10 songs must be present
    for i in 0..10 {
        let results = accessor.song_datas("md5", &format!("atomic_md5_{i}"));
        assert_eq!(results.len(), 1, "missing song {i} after batch insert");
    }
}

/// Verify that update_song_datas holds the connection lock for the entire
/// transaction. Two threads each call update_song_datas on the same roots.
/// After both complete, the DB must be in a consistent state with all
/// songs present and no corruption from interleaved transactions.
#[test]
fn test_update_concurrent_no_interleaving() {
    use std::sync::Arc;

    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("pack");
    fs::create_dir_all(&bms_dir).unwrap();

    // Write two BMS files so both updates find them
    let bms_content_a = "\
#TITLE Concurrent A\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    let bms_content_b = "\
#TITLE Concurrent B\n\
#BPM 130\n\
#WAV01 snare.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("a.bms"), bms_content_a).unwrap();
    fs::write(bms_dir.join("b.bms"), bms_content_b).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor =
        Arc::new(SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap());

    // Run two full updates concurrently on the same roots
    let acc_a = Arc::clone(&accessor);
    let roots_a = bmsroot.clone();
    let handle_a = std::thread::spawn(move || {
        acc_a.update_song_datas(None, &roots_a, true, false, None);
    });

    let acc_b = Arc::clone(&accessor);
    let roots_b = bmsroot.clone();
    let handle_b = std::thread::spawn(move || {
        acc_b.update_song_datas(None, &roots_b, true, false, None);
    });

    handle_a.join().unwrap();
    handle_b.join().unwrap();

    // Both songs must be present in the final state (the second update
    // re-scans and re-inserts everything, so both songs survive).
    let results_a = accessor.song_datas("title", "Concurrent A");
    assert_eq!(
        results_a.len(),
        1,
        "song A should be present after concurrent updates"
    );
    let results_b = accessor.song_datas("title", "Concurrent B");
    assert_eq!(
        results_b.len(),
        1,
        "song B should be present after concurrent updates"
    );
}

#[test]
fn escape_sql_like_no_wildcards() {
    assert_eq!(escape_sql_like("normal/path"), "normal/path");
}

#[test]
fn escape_sql_like_percent() {
    assert_eq!(escape_sql_like("foo%bar"), "foo\\%bar");
}

#[test]
fn escape_sql_like_underscore() {
    assert_eq!(escape_sql_like("foo_bar"), "foo\\_bar");
}

#[test]
fn escape_sql_like_backslash() {
    assert_eq!(escape_sql_like("foo\\bar"), "foo\\\\bar");
}

#[test]
fn escape_sql_like_mixed() {
    assert_eq!(escape_sql_like("a%b_c\\d"), "a\\%b\\_c\\\\d");
}

#[test]
fn escape_sql_like_empty() {
    assert_eq!(escape_sql_like(""), "");
}

/// Helper: create a minimal SQLite DB with `score` and `scorelog` tables
/// that `song_datas_by_sql` can ATTACH.
fn create_stub_score_db(path: &std::path::Path) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS score (sha256 TEXT PRIMARY KEY, mode INTEGER);
         CREATE TABLE IF NOT EXISTS scorelog (sha256 TEXT PRIMARY KEY);",
    )
    .unwrap();
}

/// Regression test for R1: song_datas_by_sql column order must match
/// query_songs_with_conn positional indices. Previously `tag` was omitted
/// and several columns were reordered, causing every field from index 7
/// onward to be misassigned.
#[test]
fn test_song_datas_by_sql_column_order() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    // Insert a song with distinctive values in every field so we can
    // detect any column-position mismatch.
    let mut sd = SongData::new();
    sd.file.md5 = "md5_sql_test".to_string();
    sd.file.sha256 = "sha256_sql_test".to_string();
    sd.metadata.title = "SQL Title".to_string();
    sd.metadata.subtitle = "SQL Sub".to_string();
    sd.metadata.genre = "SQL Genre".to_string();
    sd.metadata.artist = "SQL Artist".to_string();
    sd.metadata.subartist = "SQL SubArtist".to_string();
    sd.metadata.tag = "SQL Tag".to_string();
    sd.file.set_path("sql/path.bms".to_string());
    sd.folder = "sql_folder".to_string();
    sd.file.stagefile = "stage.png".to_string();
    sd.file.banner = "banner.png".to_string();
    sd.file.backbmp = "back.bmp".to_string();
    sd.file.preview = "preview.ogg".to_string();
    sd.parent = "sql_parent".to_string();
    sd.chart.level = 7;
    sd.chart.difficulty = 3;
    sd.chart.maxbpm = 200;
    sd.chart.minbpm = 100;
    sd.chart.length = 120;
    sd.chart.mode = 5;
    sd.chart.judge = 2;
    sd.chart.feature = 4;
    sd.chart.content = 6;
    sd.chart.date = 1000;
    sd.favorite = 1;
    sd.chart.adddate = 2000;
    sd.chart.notes = 999;
    sd.file.charthash = Some("charthash_test".to_string());
    accessor.insert_song(&sd).unwrap();

    // Create stub score/scorelog databases required by ATTACH
    let score_path = tmpdir.path().join("score.db");
    let scorelog_path = tmpdir.path().join("scorelog.db");
    create_stub_score_db(&score_path);
    create_stub_score_db(&scorelog_path);

    let results = accessor.song_datas_by_sql(
        "1=1",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(results.len(), 1, "Expected exactly one song from SQL query");
    let r = &results[0];

    // Verify every field was mapped to the correct column position.
    assert_eq!(r.file.md5, "md5_sql_test", "md5 mismatch (index 0)");
    assert_eq!(
        r.file.sha256, "sha256_sql_test",
        "sha256 mismatch (index 1)"
    );
    assert_eq!(r.metadata.title, "SQL Title", "title mismatch (index 2)");
    assert_eq!(
        r.metadata.subtitle, "SQL Sub",
        "subtitle mismatch (index 3)"
    );
    assert_eq!(r.metadata.genre, "SQL Genre", "genre mismatch (index 4)");
    assert_eq!(r.metadata.artist, "SQL Artist", "artist mismatch (index 5)");
    assert_eq!(
        r.metadata.subartist, "SQL SubArtist",
        "subartist mismatch (index 6)"
    );
    assert_eq!(r.metadata.tag, "SQL Tag", "tag mismatch (index 7)");
    assert_eq!(
        r.file.path().unwrap_or(""),
        "sql/path.bms",
        "path mismatch (index 8)"
    );
    assert_eq!(r.folder, "sql_folder", "folder mismatch (index 9)");
    assert_eq!(
        r.file.stagefile, "stage.png",
        "stagefile mismatch (index 10)"
    );
    assert_eq!(r.file.banner, "banner.png", "banner mismatch (index 11)");
    assert_eq!(r.file.backbmp, "back.bmp", "backbmp mismatch (index 12)");
    assert_eq!(r.file.preview, "preview.ogg", "preview mismatch (index 13)");
    assert_eq!(r.parent, "sql_parent", "parent mismatch (index 14)");
    assert_eq!(r.chart.level, 7, "level mismatch (index 15)");
    assert_eq!(r.chart.difficulty, 3, "difficulty mismatch (index 16)");
    assert_eq!(r.chart.maxbpm, 200, "maxbpm mismatch (index 17)");
    assert_eq!(r.chart.minbpm, 100, "minbpm mismatch (index 18)");
    assert_eq!(r.chart.length, 120, "length mismatch (index 19)");
    assert_eq!(r.chart.mode, 5, "mode mismatch (index 20)");
    assert_eq!(r.chart.judge, 2, "judge mismatch (index 21)");
    assert_eq!(r.chart.feature, 4, "feature mismatch (index 22)");
    assert_eq!(r.chart.content, 6, "content mismatch (index 23)");
    assert_eq!(r.chart.date, 1000, "date mismatch (index 24)");
    assert_eq!(r.favorite, 1, "favorite mismatch (index 25)");
    assert_eq!(r.chart.adddate, 2000, "adddate mismatch (index 26)");
    assert_eq!(r.chart.notes, 999, "notes mismatch (index 27)");
    assert_eq!(
        r.file.charthash.as_deref(),
        Some("charthash_test"),
        "charthash mismatch (index 28)"
    );
}

/// Regression test for R1 (info path): song_datas_by_sql with an info database
/// must also use the correct column order for the INNER JOIN query variant.
#[test]
fn test_song_datas_by_sql_with_info_column_order() {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    let mut sd = SongData::new();
    sd.file.md5 = "md5_info_test".to_string();
    sd.file.sha256 = "sha256_info_test".to_string();
    sd.metadata.title = "Info Title".to_string();
    sd.metadata.tag = "Info Tag".to_string();
    sd.file.set_path("info/path.bms".to_string());
    sd.folder = "info_folder".to_string();
    sd.file.preview = "info_preview.ogg".to_string();
    sd.chart.level = 12;
    sd.chart.length = 90;
    sd.chart.mode = 7;
    sd.chart.notes = 500;
    sd.chart.adddate = 3000;
    accessor.insert_song(&sd).unwrap();

    // Create stub databases
    let score_path = tmpdir.path().join("score.db");
    let scorelog_path = tmpdir.path().join("scorelog.db");
    create_stub_score_db(&score_path);
    create_stub_score_db(&scorelog_path);

    // Create info DB with the `information` table containing the matching sha256
    let info_path = tmpdir.path().join("info.db");
    let info_conn = rusqlite::Connection::open(&info_path).unwrap();
    info_conn
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS information (sha256 TEXT PRIMARY KEY, n INTEGER, ln INTEGER, \
             s INTEGER, ls INTEGER, total REAL, density REAL, peakdensity REAL, enddensity REAL, \
             mainbpm REAL, distribution TEXT, speedchange TEXT, lanenotes TEXT);",
        )
        .unwrap();
    info_conn
        .execute(
            "INSERT INTO information (sha256, n, ln, s, ls, total, density, peakdensity, enddensity, mainbpm, distribution, speedchange, lanenotes) \
             VALUES ('sha256_info_test', 100, 0, 0, 0, 200.0, 5.0, 10.0, 3.0, 150.0, '', '', '')",
            [],
        )
        .unwrap();
    drop(info_conn);

    let results = accessor.song_datas_by_sql(
        "1=1",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        Some(&info_path.to_string_lossy()),
    );
    assert_eq!(
        results.len(),
        1,
        "Expected one song from info-path SQL query"
    );
    let r = &results[0];

    // Verify key fields that would be misassigned with the old column order
    assert_eq!(r.metadata.tag, "Info Tag", "tag mismatch (index 7)");
    assert_eq!(
        r.file.path().unwrap_or(""),
        "info/path.bms",
        "path mismatch (index 8)"
    );
    assert_eq!(r.folder, "info_folder", "folder mismatch (index 9)");
    assert_eq!(
        r.file.preview, "info_preview.ogg",
        "preview mismatch (index 13)"
    );
    assert_eq!(r.chart.level, 12, "level mismatch (index 15)");
    assert_eq!(r.chart.length, 90, "length mismatch (index 19)");
    assert_eq!(r.chart.mode, 7, "mode mismatch (index 20)");
    assert_eq!(r.chart.adddate, 3000, "adddate mismatch (index 26)");
    assert_eq!(r.chart.notes, 500, "notes mismatch (index 27)");
}

/// Regression: if bmsroot contains an empty string, the LIKE pattern becomes
/// '%' which matches ALL rows, causing the incremental DELETE to wipe the
/// entire song/folder table. Empty roots must be filtered out.
#[test]
fn test_incremental_delete_ignores_empty_bmsroot() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("pack");
    fs::create_dir_all(&bms_dir).unwrap();

    let bms_content = "\
#TITLE EmptyRoot Guard\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("guard.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let real_root = tmpdir.path().join("songs").to_string_lossy().to_string();
    let accessor =
        SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[real_root.clone()]).unwrap();

    // Initial full scan
    accessor.update_song_datas(None, &[real_root.clone()], true, false, None);
    let songs = accessor.song_datas("title", "EmptyRoot Guard");
    assert_eq!(songs.len(), 1, "song should exist after initial scan");

    // Incremental update with an empty string mixed into bmsroot.
    // Before the fix, the empty string would produce LIKE '%' and delete everything.
    let roots_with_empty = vec![real_root.clone(), "".to_string()];
    accessor.update_song_datas(None, &roots_with_empty, false, false, None);

    let songs = accessor.song_datas("title", "EmptyRoot Guard");
    assert_eq!(
        songs.len(),
        1,
        "song should NOT be deleted when bmsroot contains an empty string"
    );
}

/// Regression test for R3: checked_parent must be populated after a folder
/// query to avoid redundant lookups. When update_parent_when_missing is true
/// and the parent folder already exists in the database, a second call with
/// the same parent should skip the folder query (cached in checked_parent).
#[test]
fn test_checked_parent_populated_after_folder_query() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bms_dir = tmpdir.path().join("songs").join("pack");
    fs::create_dir_all(&bms_dir).unwrap();

    let bms_content = "\
#TITLE CheckedParent Test\n\
#BPM 120\n\
#WAV01 kick.wav\n\
#00111:01\n\
";
    fs::write(bms_dir.join("cp.bms"), bms_content).unwrap();

    let db_path = tmpdir.path().join("song.db");
    let bmsroot = vec![tmpdir.path().join("songs").to_string_lossy().to_string()];
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &bmsroot).unwrap();

    // Initial scan to populate folder records
    accessor.update_song_datas(None, &bmsroot, true, false, None);

    let bms_path = bms_dir.join("cp.bms").to_string_lossy().to_string();

    // First update with update_parent_when_missing=true
    accessor.update_song_datas(Some(&bms_path), &bmsroot, false, true, None);

    // After the first call, the parent should be in checked_parent
    let checked = accessor
        .checked_parent
        .lock()
        .expect("checked_parent lock poisoned");
    let parent_path = Path::new(&bms_path)
        .parent()
        .map(|pp| pp.to_string_lossy().to_string())
        .unwrap_or_default();
    assert!(
        checked.contains(&parent_path),
        "checked_parent should contain '{}' after first update_parent_when_missing call",
        parent_path
    );
}

/// Helper: set up a song database with one song and stub score DBs for
/// authorizer tests.
fn setup_authorizer_test() -> (
    SQLiteSongDatabaseAccessor,
    tempfile::TempDir,
    std::path::PathBuf,
    std::path::PathBuf,
) {
    let tmpdir = tempfile::tempdir().unwrap();
    let db_path = tmpdir.path().join("song.db");
    let accessor = SQLiteSongDatabaseAccessor::new(&db_path.to_string_lossy(), &[]).unwrap();

    let mut sd = SongData::new();
    sd.file.md5 = "md5_auth".to_string();
    sd.file.sha256 = "sha256_auth".to_string();
    sd.metadata.title = "Auth Test Song".to_string();
    sd.chart.level = 5;
    sd.file.set_path("auth/test.bms".to_string());
    accessor.insert_song(&sd).unwrap();

    let score_path = tmpdir.path().join("score.db");
    let scorelog_path = tmpdir.path().join("scorelog.db");
    create_stub_score_db(&score_path);
    create_stub_score_db(&scorelog_path);

    (accessor, tmpdir, score_path, scorelog_path)
}

/// Legitimate WHERE clauses must still work through the read-only authorizer.
#[test]
fn test_song_datas_by_sql_allows_read_queries() {
    let (accessor, _tmpdir, score_path, scorelog_path) = setup_authorizer_test();

    // Simple equality
    let results = accessor.song_datas_by_sql(
        "level = 5",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(results.len(), 1, "level = 5 should match one song");

    // Tautology
    let results = accessor.song_datas_by_sql(
        "1=1",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(results.len(), 1, "1=1 should match all songs");

    // No match
    let results = accessor.song_datas_by_sql(
        "level = 99",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert!(results.is_empty(), "level = 99 should match nothing");
}

/// Legitimate WHERE clauses with subqueries must still work through the
/// read-only authorizer.
#[test]
fn test_song_datas_by_sql_allows_subquery() {
    let (accessor, _tmpdir, score_path, scorelog_path) = setup_authorizer_test();

    let results = accessor.song_datas_by_sql(
        "md5 IN (SELECT md5 FROM song) AND 1=(SELECT count(*) FROM song WHERE md5='md5_auth')",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(results.len(), 1, "read-only subquery should work");
}

/// Verify the authorizer is properly removed after the query, so subsequent
/// trusted operations (like DETACH) work correctly.
#[test]
fn test_song_datas_by_sql_authorizer_cleanup() {
    let (accessor, _tmpdir, score_path, scorelog_path) = setup_authorizer_test();

    // Run a query - this installs and removes the authorizer
    let results = accessor.song_datas_by_sql(
        "1=1",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(results.len(), 1);

    // Run another query to verify the authorizer was properly cleaned up
    // (ATTACH in the second call would fail if authorizer was still active)
    let results = accessor.song_datas_by_sql(
        "level = 5",
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(
        results.len(),
        1,
        "second query should work after authorizer cleanup"
    );
}

/// The read-only authorizer blocks destructive operations when set on a
/// connection. This tests the authorizer directly against the underlying
/// SQLite connection to verify it blocks INSERT, UPDATE, DELETE, DROP, and
/// ATTACH at the prepare stage.
#[test]
fn test_read_only_authorizer_blocks_destructive_ops() {
    use super::read_only_authorizer;
    use rusqlite::hooks::{AuthContext, Authorization};

    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE t (id INTEGER, name TEXT)")
        .unwrap();
    conn.execute("INSERT INTO t VALUES (1, 'hello')", [])
        .unwrap();

    // Install the read-only authorizer
    conn.authorizer(Some(read_only_authorizer));

    // SELECT should succeed
    let count: i64 = conn
        .query_row("SELECT count(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "SELECT should work with read-only authorizer");

    // INSERT should be blocked
    let result = conn.execute("INSERT INTO t VALUES (2, 'evil')", []);
    assert!(result.is_err(), "INSERT should be denied by authorizer");

    // UPDATE should be blocked
    let result = conn.execute("UPDATE t SET name = 'pwned'", []);
    assert!(result.is_err(), "UPDATE should be denied by authorizer");

    // DELETE should be blocked
    let result = conn.execute("DELETE FROM t", []);
    assert!(result.is_err(), "DELETE should be denied by authorizer");

    // DROP TABLE should be blocked
    let result = conn.execute_batch("DROP TABLE t");
    assert!(result.is_err(), "DROP TABLE should be denied by authorizer");

    // ATTACH should be blocked
    let result = conn.execute("ATTACH DATABASE ':memory:' AS evil", []);
    assert!(result.is_err(), "ATTACH should be denied by authorizer");

    // Remove the authorizer
    conn.authorizer(None::<fn(AuthContext<'_>) -> Authorization>);

    // Verify data is intact
    let count: i64 = conn
        .query_row("SELECT count(*) FROM t", [], |row| row.get(0))
        .unwrap();
    assert_eq!(count, 1, "data should be intact after blocked operations");
}

/// Regression: ChartInfo.date and ChartInfo.adddate must be i64 to avoid
/// Y2038 overflow. Timestamps after 2038-01-19 03:14:07 UTC exceed i32::MAX.
#[test]
fn test_y2038_date_round_trip() {
    let accessor = create_test_accessor();

    let mut sd = make_test_song("y2038_test", "y2038_sha", "Y2038 Song");
    // Timestamp after Y2038: 2040-01-01 00:00:00 UTC = 2208988800
    sd.chart.date = 2_208_988_800;
    sd.chart.adddate = 2_208_988_800;
    accessor.insert_song(&sd).unwrap();

    let results = accessor.song_datas("md5", "y2038_test");
    assert_eq!(results.len(), 1);
    assert_eq!(
        results[0].chart.date, 2_208_988_800,
        "date must survive round-trip without i32 truncation"
    );
    assert_eq!(
        results[0].chart.adddate, 2_208_988_800,
        "adddate must survive round-trip without i32 truncation"
    );
}

/// Defense-in-depth: SQL strings longer than MAX_COURSE_SQL_LENGTH (4096) must
/// be rejected early with empty results, preventing abuse via oversized
/// WHERE clauses from .lr2crs course files.
#[test]
fn test_song_datas_by_sql_rejects_oversized_sql() {
    let (accessor, _tmpdir, score_path, scorelog_path) = setup_authorizer_test();

    // A SQL string just at the limit should still work
    let at_limit = format!("level = 5 {}", " ".repeat(4096 - "level = 5 ".len()));
    assert_eq!(at_limit.len(), 4096);
    let results = accessor.song_datas_by_sql(
        &at_limit,
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert_eq!(
        results.len(),
        1,
        "SQL at exactly 4096 chars should still execute"
    );

    // A SQL string exceeding the limit should return empty results
    let over_limit = format!("level = 5 {}", " ".repeat(4097 - "level = 5 ".len()));
    assert_eq!(over_limit.len(), 4097);
    let results = accessor.song_datas_by_sql(
        &over_limit,
        &score_path.to_string_lossy(),
        &scorelog_path.to_string_lossy(),
        None,
    );
    assert!(
        results.is_empty(),
        "SQL exceeding 4096 chars should return empty results"
    );
}
