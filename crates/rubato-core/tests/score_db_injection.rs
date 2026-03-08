// SQL injection tests for ScoreDatabaseAccessor.
//
// These tests verify that parameterized queries prevent SQL injection.
// Previously format!-based SQL construction was vulnerable; now fixed
// with params![] and column name whitelisting.

mod helpers;

use std::collections::HashMap;

use rubato_core::score_data::ScoreData;
use rubato_core::score_database_accessor::{ScoreDataCollector, SongData};

/// Build a minimal ScoreData that passes `Validatable::validate()`.
fn make_score(sha256: &str, mode: i32, clear: i32) -> ScoreData {
    ScoreData {
        sha256: sha256.to_string(),
        mode,
        clear,
        notes: 100,
        ..Default::default()
    }
}

// -----------------------------------------------------------------------
// score_data: hash injection is now blocked by parameterized query
// -----------------------------------------------------------------------

#[test]
fn get_score_data_hash_injection_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    let victim = make_score("victim_hash", 0, 5);
    db.set_score_data(&victim);

    // Injection payload that previously bypassed the WHERE clause
    let result = db.score_data("' OR '1'='1", 0);
    assert!(
        result.is_none(),
        "SQL injection via hash should be blocked by parameterized query"
    );

    // Legitimate query still works
    let legit = db.score_data("victim_hash", 0);
    assert!(legit.is_some(), "legitimate hash query should succeed");
    assert_eq!(legit.unwrap().clear, 5);
}

// -----------------------------------------------------------------------
// set_score_data_map: injection via hash is now blocked
// -----------------------------------------------------------------------

#[test]
fn set_score_data_map_injection_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    let victim = make_score("victim_hash", 0, 3);
    db.set_score_data(&victim);

    // Crafted hash that previously caused SQL injection
    let injected_hash = "x' OR sha256 = 'victim_hash' --";

    let mut values: HashMap<String, String> = HashMap::new();
    values.insert("clear".to_string(), "9".to_string());

    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    map.insert(injected_hash.to_string(), values);

    db.set_score_data_map(&map);

    // Victim row should NOT be modified
    let restored = db
        .score_data("victim_hash", 0)
        .expect("victim row should still exist");
    assert_eq!(
        restored.clear, 3,
        "victim row should retain original clear value (injection blocked)"
    );
}

// -----------------------------------------------------------------------
// set_score_data_map: invalid column names are rejected
// -----------------------------------------------------------------------

#[test]
fn set_score_data_map_rejects_invalid_column() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    let score = make_score("test_hash", 0, 5);
    db.set_score_data(&score);

    let mut values: HashMap<String, String> = HashMap::new();
    // "invalid_col" is not in the whitelist
    values.insert("invalid_col".to_string(), "999".to_string());

    let mut map: HashMap<String, HashMap<String, String>> = HashMap::new();
    map.insert("test_hash".to_string(), values);

    // Should not panic; invalid column is silently skipped
    db.set_score_data_map(&map);

    let restored = db.score_data("test_hash", 0).unwrap();
    assert_eq!(restored.clear, 5, "score should be unchanged");
}

// -----------------------------------------------------------------------
// score_datas_for_songs: IN-clause injection is now blocked
// -----------------------------------------------------------------------

struct CollectAll {
    results: Vec<(String, Option<ScoreData>)>,
}

impl CollectAll {
    fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }
}

impl ScoreDataCollector for CollectAll {
    fn collect(&mut self, song: &SongData, score: Option<&ScoreData>) {
        self.results
            .push((song.file.sha256.clone(), score.cloned()));
    }
}

#[test]
fn get_score_datas_for_songs_injection_blocked() {
    let dir = tempfile::tempdir().unwrap();
    let db = helpers::open_score_db(dir.path());

    let victim_a = make_score("real_hash_aaa", 0, 3);
    let victim_b = make_score("real_hash_bbb", 0, 7);
    db.set_score_data(&victim_a);
    db.set_score_data(&victim_b);

    // Crafted sha256 that previously broke out of IN clause
    let mut injected_song = SongData::default();
    injected_song.file.sha256 = "') OR 1=1 --".to_string();

    let mut collector = CollectAll::new();
    db.score_datas_for_songs(&mut collector, &[injected_song], 0);

    // The injected song should NOT match any real scores
    let matched_scores: Vec<&ScoreData> = collector
        .results
        .iter()
        .filter_map(|(_, s)| s.as_ref())
        .collect();
    assert!(
        matched_scores.is_empty(),
        "SQL injection via IN clause should be blocked by parameterized query"
    );
}
