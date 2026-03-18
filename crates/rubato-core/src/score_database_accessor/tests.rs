use rusqlite::Connection;

use crate::player_data::PlayerData;
use crate::score_data::ScoreData;
use crate::sqlite_database_accessor::{Column, SQLiteDatabaseAccessor, Table};

use super::helpers::{local_midnight_timestamp, player_data_to_value, score_data_to_value};
use super::{ScoreDataCollector, ScoreDatabaseAccessor, SongData};

#[test]
fn local_midnight_timestamp_does_not_panic() {
    // This would panic before the fix if called during a DST transition
    // because and_local_timezone().unwrap() fails on Ambiguous/None results.
    let ts = local_midnight_timestamp();
    assert!(ts > 0, "timestamp should be positive");
}

#[test]
fn local_midnight_timestamp_is_start_of_day() {
    let ts = local_midnight_timestamp();
    let now_ts = chrono::Local::now().timestamp();
    // Midnight should be at most 24 hours before now (86400 seconds)
    assert!(
        now_ts - ts < 86400,
        "midnight timestamp should be within the last 24 hours"
    );
    assert!(ts <= now_ts, "midnight should not be in the future");
}

#[test]
fn set_player_data_does_not_panic() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_score.db");
    let accessor = ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).unwrap();
    accessor.create_table().expect("create table");

    let pd = PlayerData {
        playcount: 10,
        clear: 5,
        playtime: 3600,
        ..Default::default()
    };

    // This should not panic even during DST transitions
    accessor.set_player_data(&pd);

    // Verify the data was written
    let loaded = accessor.player_data();
    assert!(loaded.is_some());
    let loaded = loaded.unwrap();
    assert_eq!(loaded.playcount, 10);
    assert_eq!(loaded.clear, 5);
    assert!(loaded.date > 0, "date should be set to local midnight");
}

// --- score_data_to_value tests ---

#[test]
fn test_score_data_to_value_basic() {
    use rubato_types::score_data::{JudgeCounts, PlayOption, TimingStats};
    let sd = ScoreData {
        sha256: "abc123def456".to_string(),
        mode: 7,
        clear: 5,
        judge_counts: JudgeCounts {
            epg: 100,
            lpg: 90,
            egr: 80,
            lgr: 70,
            egd: 10,
            lgd: 9,
            ebd: 3,
            lbd: 2,
            epr: 1,
            lpr: 0,
            ems: 4,
            lms: 5,
        },
        notes: 500,
        maxcombo: 300,
        minbp: 15,
        timing_stats: TimingStats {
            avgjudge: 42,
            ..Default::default()
        },
        playcount: 10,
        clearcount: 7,
        trophy: "g".to_string(),
        ghost: "ghost_data".to_string(),
        play_option: PlayOption {
            option: 2,
            seed: 12345,
            random: 1,
            ..Default::default()
        },
        date: 1700000000,
        state: 3,
        scorehash: "hashvalue".to_string(),
        ..Default::default()
    };

    assert_eq!(
        score_data_to_value(&sd, "sha256"),
        rusqlite::types::Value::Text("abc123def456".to_string())
    );
    assert_eq!(
        score_data_to_value(&sd, "mode"),
        rusqlite::types::Value::Integer(7)
    );
    assert_eq!(
        score_data_to_value(&sd, "clear"),
        rusqlite::types::Value::Integer(5)
    );
    assert_eq!(
        score_data_to_value(&sd, "epg"),
        rusqlite::types::Value::Integer(100)
    );
    assert_eq!(
        score_data_to_value(&sd, "lpg"),
        rusqlite::types::Value::Integer(90)
    );
    assert_eq!(
        score_data_to_value(&sd, "egr"),
        rusqlite::types::Value::Integer(80)
    );
    assert_eq!(
        score_data_to_value(&sd, "lgr"),
        rusqlite::types::Value::Integer(70)
    );
    assert_eq!(
        score_data_to_value(&sd, "egd"),
        rusqlite::types::Value::Integer(10)
    );
    assert_eq!(
        score_data_to_value(&sd, "lgd"),
        rusqlite::types::Value::Integer(9)
    );
    assert_eq!(
        score_data_to_value(&sd, "ebd"),
        rusqlite::types::Value::Integer(3)
    );
    assert_eq!(
        score_data_to_value(&sd, "lbd"),
        rusqlite::types::Value::Integer(2)
    );
    assert_eq!(
        score_data_to_value(&sd, "epr"),
        rusqlite::types::Value::Integer(1)
    );
    assert_eq!(
        score_data_to_value(&sd, "lpr"),
        rusqlite::types::Value::Integer(0)
    );
    assert_eq!(
        score_data_to_value(&sd, "ems"),
        rusqlite::types::Value::Integer(4)
    );
    assert_eq!(
        score_data_to_value(&sd, "lms"),
        rusqlite::types::Value::Integer(5)
    );
    assert_eq!(
        score_data_to_value(&sd, "notes"),
        rusqlite::types::Value::Integer(500)
    );
    // "combo" maps to maxcombo
    assert_eq!(
        score_data_to_value(&sd, "combo"),
        rusqlite::types::Value::Integer(300)
    );
    assert_eq!(
        score_data_to_value(&sd, "minbp"),
        rusqlite::types::Value::Integer(15)
    );
    assert_eq!(
        score_data_to_value(&sd, "avgjudge"),
        rusqlite::types::Value::Integer(42)
    );
    assert_eq!(
        score_data_to_value(&sd, "playcount"),
        rusqlite::types::Value::Integer(10)
    );
    assert_eq!(
        score_data_to_value(&sd, "clearcount"),
        rusqlite::types::Value::Integer(7)
    );
    assert_eq!(
        score_data_to_value(&sd, "trophy"),
        rusqlite::types::Value::Text("g".to_string())
    );
    assert_eq!(
        score_data_to_value(&sd, "ghost"),
        rusqlite::types::Value::Text("ghost_data".to_string())
    );
    assert_eq!(
        score_data_to_value(&sd, "option"),
        rusqlite::types::Value::Integer(2)
    );
    assert_eq!(
        score_data_to_value(&sd, "seed"),
        rusqlite::types::Value::Integer(12345)
    );
    assert_eq!(
        score_data_to_value(&sd, "random"),
        rusqlite::types::Value::Integer(1)
    );
    assert_eq!(
        score_data_to_value(&sd, "date"),
        rusqlite::types::Value::Integer(1700000000)
    );
    assert_eq!(
        score_data_to_value(&sd, "state"),
        rusqlite::types::Value::Integer(3)
    );
    assert_eq!(
        score_data_to_value(&sd, "scorehash"),
        rusqlite::types::Value::Text("hashvalue".to_string())
    );
}

#[test]
fn test_score_data_to_value_default_fields() {
    let sd = ScoreData::default();

    assert_eq!(
        score_data_to_value(&sd, "sha256"),
        rusqlite::types::Value::Text(String::new())
    );
    assert_eq!(
        score_data_to_value(&sd, "mode"),
        rusqlite::types::Value::Integer(0)
    );
    assert_eq!(
        score_data_to_value(&sd, "clear"),
        rusqlite::types::Value::Integer(0)
    );
    assert_eq!(
        score_data_to_value(&sd, "minbp"),
        rusqlite::types::Value::Integer(i32::MAX as i64)
    );
    // score_data_to_value normalizes i64::MAX sentinel to i32::MAX for Java DB compatibility
    assert_eq!(
        score_data_to_value(&sd, "avgjudge"),
        rusqlite::types::Value::Integer(i32::MAX as i64)
    );
    assert_eq!(
        score_data_to_value(&sd, "seed"),
        rusqlite::types::Value::Integer(-1)
    );
    assert_eq!(
        score_data_to_value(&sd, "trophy"),
        rusqlite::types::Value::Text(String::new())
    );
    assert_eq!(
        score_data_to_value(&sd, "ghost"),
        rusqlite::types::Value::Text(String::new())
    );
    assert_eq!(
        score_data_to_value(&sd, "scorehash"),
        rusqlite::types::Value::Text(String::new())
    );
}

#[test]
fn test_score_data_to_value_unknown_column_returns_null() {
    let sd = ScoreData::default();

    assert_eq!(
        score_data_to_value(&sd, "nonexistent"),
        rusqlite::types::Value::Null
    );
    assert_eq!(score_data_to_value(&sd, ""), rusqlite::types::Value::Null);
}

// --- player_data_to_value tests ---

#[test]
fn test_player_data_to_value_basic() {
    let pd = PlayerData {
        date: 1700000000,
        playcount: 50,
        clear: 30,
        epg: 100,
        lpg: 90,
        egr: 80,
        lgr: 70,
        egd: 10,
        lgd: 9,
        ebd: 3,
        lbd: 2,
        epr: 1,
        lpr: 0,
        ems: 4,
        lms: 5,
        playtime: 7200,
        maxcombo: 500,
    };

    assert_eq!(
        player_data_to_value(&pd, "date"),
        rusqlite::types::Value::Integer(1700000000)
    );
    assert_eq!(
        player_data_to_value(&pd, "playcount"),
        rusqlite::types::Value::Integer(50)
    );
    assert_eq!(
        player_data_to_value(&pd, "clear"),
        rusqlite::types::Value::Integer(30)
    );
    assert_eq!(
        player_data_to_value(&pd, "epg"),
        rusqlite::types::Value::Integer(100)
    );
    assert_eq!(
        player_data_to_value(&pd, "lpg"),
        rusqlite::types::Value::Integer(90)
    );
    assert_eq!(
        player_data_to_value(&pd, "egr"),
        rusqlite::types::Value::Integer(80)
    );
    assert_eq!(
        player_data_to_value(&pd, "lgr"),
        rusqlite::types::Value::Integer(70)
    );
    assert_eq!(
        player_data_to_value(&pd, "egd"),
        rusqlite::types::Value::Integer(10)
    );
    assert_eq!(
        player_data_to_value(&pd, "lgd"),
        rusqlite::types::Value::Integer(9)
    );
    assert_eq!(
        player_data_to_value(&pd, "ebd"),
        rusqlite::types::Value::Integer(3)
    );
    assert_eq!(
        player_data_to_value(&pd, "lbd"),
        rusqlite::types::Value::Integer(2)
    );
    assert_eq!(
        player_data_to_value(&pd, "epr"),
        rusqlite::types::Value::Integer(1)
    );
    assert_eq!(
        player_data_to_value(&pd, "lpr"),
        rusqlite::types::Value::Integer(0)
    );
    assert_eq!(
        player_data_to_value(&pd, "ems"),
        rusqlite::types::Value::Integer(4)
    );
    assert_eq!(
        player_data_to_value(&pd, "lms"),
        rusqlite::types::Value::Integer(5)
    );
    assert_eq!(
        player_data_to_value(&pd, "playtime"),
        rusqlite::types::Value::Integer(7200)
    );
    assert_eq!(
        player_data_to_value(&pd, "maxcombo"),
        rusqlite::types::Value::Integer(500)
    );
}

#[test]
fn test_player_data_to_value_unknown_column_returns_null() {
    let pd = PlayerData::default();

    assert_eq!(
        player_data_to_value(&pd, "nonexistent"),
        rusqlite::types::Value::Null
    );
    assert_eq!(player_data_to_value(&pd, ""), rusqlite::types::Value::Null);
}

// --- Roundtrip / integration tests using :memory: DB ---

/// Helper: create an in-memory ScoreDatabaseAccessor with tables initialized.
fn memory_accessor() -> ScoreDatabaseAccessor {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch("PRAGMA shared_cache = ON").unwrap();
    conn.pragma_update(None, "synchronous", "OFF").unwrap();
    conn.pragma_update(None, "cache_size", 2000).unwrap();

    let tables = vec![
        Table::new(
            "info",
            vec![
                Column::with_pk("id", "TEXT", 1, 1),
                Column::with_pk("name", "TEXT", 1, 0),
                Column::new("rank", "TEXT"),
            ],
        ),
        Table::new(
            "player",
            vec![
                Column::with_pk("date", "INTEGER", 0, 1),
                Column::new("playcount", "INTEGER"),
                Column::new("clear", "INTEGER"),
                Column::new("epg", "INTEGER"),
                Column::new("lpg", "INTEGER"),
                Column::new("egr", "INTEGER"),
                Column::new("lgr", "INTEGER"),
                Column::new("egd", "INTEGER"),
                Column::new("lgd", "INTEGER"),
                Column::new("ebd", "INTEGER"),
                Column::new("lbd", "INTEGER"),
                Column::new("epr", "INTEGER"),
                Column::new("lpr", "INTEGER"),
                Column::new("ems", "INTEGER"),
                Column::new("lms", "INTEGER"),
                Column::new("playtime", "INTEGER"),
                Column::new("maxcombo", "INTEGER"),
            ],
        ),
        Table::new(
            "score",
            vec![
                Column::with_pk("sha256", "TEXT", 1, 1),
                Column::with_pk("mode", "INTEGER", 0, 1),
                Column::new("clear", "INTEGER"),
                Column::new("epg", "INTEGER"),
                Column::new("lpg", "INTEGER"),
                Column::new("egr", "INTEGER"),
                Column::new("lgr", "INTEGER"),
                Column::new("egd", "INTEGER"),
                Column::new("lgd", "INTEGER"),
                Column::new("ebd", "INTEGER"),
                Column::new("lbd", "INTEGER"),
                Column::new("epr", "INTEGER"),
                Column::new("lpr", "INTEGER"),
                Column::new("ems", "INTEGER"),
                Column::new("lms", "INTEGER"),
                Column::new("notes", "INTEGER"),
                Column::new("combo", "INTEGER"),
                Column::new("minbp", "INTEGER"),
                Column::with_default("avgjudge", "INTEGER", 1, 0, &i32::MAX.to_string()),
                Column::new("playcount", "INTEGER"),
                Column::new("clearcount", "INTEGER"),
                Column::new("trophy", "TEXT"),
                Column::new("ghost", "TEXT"),
                Column::new("option", "INTEGER"),
                Column::new("seed", "INTEGER"),
                Column::new("random", "INTEGER"),
                Column::new("date", "INTEGER"),
                Column::new("state", "INTEGER"),
                Column::new("scorehash", "TEXT"),
            ],
        ),
    ];

    let base = SQLiteDatabaseAccessor::new(tables);
    let accessor = ScoreDatabaseAccessor { conn, base };
    accessor.base.validate(&accessor.conn).unwrap();
    accessor
}

/// Build a valid ScoreData (passes validate()) with given sha256, mode, clear.
#[allow(clippy::field_reassign_with_default)]
fn make_score(sha256: &str, mode: i32, clear: i32) -> ScoreData {
    let mut sd = ScoreData::default();
    sd.sha256 = sha256.to_string();
    sd.mode = mode;
    sd.clear = clear;
    sd.notes = 100;
    sd.passnotes = 100;
    sd.judge_counts.epg = 50;
    sd.judge_counts.lpg = 30;
    sd.judge_counts.egr = 10;
    sd.judge_counts.lgr = 5;
    sd.judge_counts.egd = 2;
    sd.judge_counts.lgd = 1;
    sd.judge_counts.ebd = 1;
    sd.judge_counts.lbd = 0;
    sd.judge_counts.epr = 1;
    sd.judge_counts.lpr = 0;
    sd.judge_counts.ems = 0;
    sd.judge_counts.lms = 0;
    sd.maxcombo = 80;
    sd.minbp = 5;
    sd.timing_stats.avgjudge = 10;
    sd.playcount = 3;
    sd.clearcount = 2;
    sd.trophy = "g".to_string();
    sd.ghost = String::new();
    sd.play_option.option = 0;
    sd.play_option.seed = 42;
    sd.play_option.random = 0;
    sd.date = 1700000000;
    sd.state = 0;
    sd.scorehash = "hash1".to_string();
    sd
}

#[test]
fn test_score_data_roundtrip_via_memory_db() {
    let accessor = memory_accessor();

    let sd = make_score("abc123", 0, 5);
    accessor.set_score_data(&sd);

    let loaded = accessor.score_data("abc123", 0);
    assert!(loaded.is_some(), "score should be retrievable after insert");
    let loaded = loaded.unwrap();

    assert_eq!(loaded.sha256, "abc123");
    assert_eq!(loaded.mode, 0);
    assert_eq!(loaded.clear, 5);
    assert_eq!(loaded.judge_counts.epg, 50);
    assert_eq!(loaded.judge_counts.lpg, 30);
    assert_eq!(loaded.judge_counts.egr, 10);
    assert_eq!(loaded.judge_counts.lgr, 5);
    assert_eq!(loaded.judge_counts.egd, 2);
    assert_eq!(loaded.judge_counts.lgd, 1);
    assert_eq!(loaded.judge_counts.ebd, 1);
    assert_eq!(loaded.judge_counts.lbd, 0);
    assert_eq!(loaded.judge_counts.epr, 1);
    assert_eq!(loaded.judge_counts.lpr, 0);
    assert_eq!(loaded.judge_counts.ems, 0);
    assert_eq!(loaded.judge_counts.lms, 0);
    assert_eq!(loaded.notes, 100);
    assert_eq!(loaded.maxcombo, 80);
    assert_eq!(loaded.minbp, 5);
    assert_eq!(loaded.timing_stats.avgjudge, 10);
    assert_eq!(loaded.playcount, 3);
    assert_eq!(loaded.clearcount, 2);
    assert_eq!(loaded.trophy, "g");
    assert_eq!(loaded.play_option.option, 0);
    assert_eq!(loaded.play_option.seed, 42);
    assert_eq!(loaded.play_option.random, 0);
    assert_eq!(loaded.date, 1700000000);
    assert_eq!(loaded.state, 0);
    assert_eq!(loaded.scorehash, "hash1");
}

#[test]
fn test_set_and_get_score_data() {
    let accessor = memory_accessor();

    // Write two scores with different sha256
    let sd1 = make_score("hash_aaa", 0, 3);
    let sd2 = make_score("hash_bbb", 0, 7);
    accessor.set_score_data(&sd1);
    accessor.set_score_data(&sd2);

    // Retrieve each independently
    let loaded1 = accessor.score_data("hash_aaa", 0).unwrap();
    assert_eq!(loaded1.sha256, "hash_aaa");
    assert_eq!(loaded1.clear, 3);

    let loaded2 = accessor.score_data("hash_bbb", 0).unwrap();
    assert_eq!(loaded2.sha256, "hash_bbb");
    assert_eq!(loaded2.clear, 7);

    // Non-existent hash returns None
    assert!(accessor.score_data("hash_zzz", 0).is_none());

    // Wrong mode returns None
    assert!(accessor.score_data("hash_aaa", 99).is_none());
}

#[test]
fn test_get_score_data_picks_best_clear() {
    let accessor = memory_accessor();

    // Insert a score, then overwrite with a higher clear via INSERT OR REPLACE.
    // Since (sha256, mode) is the primary key, second insert replaces the first.
    let sd_low = make_score("hash_best", 0, 2);
    accessor.set_score_data(&sd_low);

    let mut sd_high = make_score("hash_best", 0, 8);
    sd_high.judge_counts.epg = 70;
    // Different mode so both exist
    sd_high.mode = 1;
    accessor.set_score_data(&sd_high);

    // mode=0 returns the original clear=2 score
    let loaded0 = accessor.score_data("hash_best", 0).unwrap();
    assert_eq!(loaded0.clear, 2);

    // mode=1 returns the clear=8 score
    let loaded1 = accessor.score_data("hash_best", 1).unwrap();
    assert_eq!(loaded1.clear, 8);
    assert_eq!(loaded1.judge_counts.epg, 70);
}

#[test]
fn test_get_score_datas_for_songs_empty() {
    let accessor = memory_accessor();

    struct TestCollector {
        calls: Vec<(String, Option<i32>)>,
    }
    impl ScoreDataCollector for TestCollector {
        fn collect(&mut self, song: &SongData, score: Option<&ScoreData>) {
            self.calls
                .push((song.file.sha256.clone(), score.map(|s| s.clear)));
        }
    }

    let mut collector = TestCollector { calls: vec![] };
    let songs: Vec<SongData> = vec![];
    accessor.score_datas_for_songs(&mut collector, &songs, 0);

    assert!(
        collector.calls.is_empty(),
        "empty songs list should produce no collector calls"
    );
}

#[test]
fn test_delete_score_data() {
    let accessor = memory_accessor();

    let sd = make_score("hash_del", 0, 5);
    accessor.set_score_data(&sd);
    assert!(accessor.score_data("hash_del", 0).is_some());

    accessor.delete_score_data("hash_del", 0);
    assert!(
        accessor.score_data("hash_del", 0).is_none(),
        "score should be deleted"
    );
}

#[test]
fn test_set_score_data_batch() {
    let accessor = memory_accessor();

    let sd1 = make_score("batch_1", 0, 3);
    let sd2 = make_score("batch_2", 0, 6);
    let sd3 = make_score("batch_3", 0, 9);
    accessor.set_score_data_batch(&[&sd1, &sd2, &sd3]);

    assert_eq!(accessor.score_data("batch_1", 0).unwrap().clear, 3);
    assert_eq!(accessor.score_data("batch_2", 0).unwrap().clear, 6);
    assert_eq!(accessor.score_data("batch_3", 0).unwrap().clear, 9);
}

#[test]
fn test_get_score_datas_sql_filter() {
    let accessor = memory_accessor();

    let sd1 = make_score("sql_a", 0, 3);
    let mut sd2 = make_score("sql_b", 0, 8);
    sd2.playcount = 20;
    accessor.set_score_data(&sd1);
    accessor.set_score_data(&sd2);

    let results = accessor.score_datas("playcount >= 20").unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].sha256, "sql_b");
}
