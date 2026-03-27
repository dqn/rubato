// Integration test: ScoreData database round-trip
//
// Creates a ScoreData, writes it to a temporary SQLite database via
// ScoreDatabaseAccessor, reads it back, and verifies all persisted fields match.

use rubato_game::core::score_data::ScoreData;
use rubato_game::core::score_database_accessor::ScoreDatabaseAccessor;

/// Create a ScoreData with non-default values for all database-persisted fields.
///
/// Note: only fields that appear in the "score" table schema are round-tripped
/// through the database. Fields like `player`, `passnotes`, `total_duration`,
/// `avg`, `total_avg`, `stddev`, `assist`, `gauge`, `device_type`, `playmode`,
/// `judge_algorithm`, `rule`, and `skin` are NOT stored in the score table.
fn make_test_score() -> ScoreData {
    use rubato_types::score_data::{JudgeCounts, PlayOption, TimingStats};
    ScoreData {
        sha256: "a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2".to_string(),
        mode: 0,
        clear: 7, // ExHard
        judge_counts: JudgeCounts {
            epg: 500,
            lpg: 480,
            egr: 120,
            lgr: 110,
            egd: 30,
            lgd: 25,
            ebd: 10,
            lbd: 8,
            epr: 3,
            lpr: 2,
            ems: 1,
            lms: 1,
        },
        notes: 1290,
        maxcombo: 1250,
        minbp: 15,
        timing_stats: TimingStats {
            avgjudge: 42000,
            ..Default::default()
        },
        playcount: 50,
        clearcount: 35,
        trophy: "gGhH".to_string(),
        ghost: "test_ghost_data".to_string(),
        play_option: PlayOption {
            option: 3,
            seed: 12345,
            random: 2,
            ..Default::default()
        },
        date: 1700000000,
        state: 1,
        scorehash: "deadbeef01234567deadbeef01234567".to_string(),
        // Fields not persisted in the score table -- use defaults
        ..Default::default()
    }
}

#[test]
fn score_data_roundtrip() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_score.db");

    let accessor =
        ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).expect("Failed to create accessor");
    accessor.create_table().expect("create table");

    let score = make_test_score();
    accessor.set_score_data(&score);

    let restored = accessor
        .score_data(&score.sha256, score.mode)
        .expect("Score should be retrievable after insert");

    // Verify primary key fields
    assert_eq!(restored.sha256, score.sha256);
    assert_eq!(restored.mode, score.mode);

    // Verify clear type
    assert_eq!(restored.clear, score.clear);

    // Verify judge counts (early/late for PG, GR, GD, BD, PR, MS)
    assert_eq!(restored.judge_counts.epg, score.judge_counts.epg);
    assert_eq!(restored.judge_counts.lpg, score.judge_counts.lpg);
    assert_eq!(restored.judge_counts.egr, score.judge_counts.egr);
    assert_eq!(restored.judge_counts.lgr, score.judge_counts.lgr);
    assert_eq!(restored.judge_counts.egd, score.judge_counts.egd);
    assert_eq!(restored.judge_counts.lgd, score.judge_counts.lgd);
    assert_eq!(restored.judge_counts.ebd, score.judge_counts.ebd);
    assert_eq!(restored.judge_counts.lbd, score.judge_counts.lbd);
    assert_eq!(restored.judge_counts.epr, score.judge_counts.epr);
    assert_eq!(restored.judge_counts.lpr, score.judge_counts.lpr);
    assert_eq!(restored.judge_counts.ems, score.judge_counts.ems);
    assert_eq!(restored.judge_counts.lms, score.judge_counts.lms);

    // Verify note/combo/bp/judge stats
    assert_eq!(restored.notes, score.notes);
    assert_eq!(restored.maxcombo, score.maxcombo);
    assert_eq!(restored.minbp, score.minbp);
    assert_eq!(restored.timing_stats.avgjudge, score.timing_stats.avgjudge);

    // Verify play statistics
    assert_eq!(restored.playcount, score.playcount);
    assert_eq!(restored.clearcount, score.clearcount);

    // Verify string fields
    assert_eq!(restored.trophy, score.trophy);
    assert_eq!(restored.ghost, score.ghost);
    assert_eq!(restored.scorehash, score.scorehash);

    // Verify option/random/seed
    assert_eq!(restored.play_option.option, score.play_option.option);
    assert_eq!(restored.play_option.seed, score.play_option.seed);
    assert_eq!(restored.play_option.random, score.play_option.random);

    // Verify date and state
    assert_eq!(restored.date, score.date);
    assert_eq!(restored.state, score.state);
}

#[test]
fn score_data_roundtrip_with_different_mode() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_score_mode.db");

    let accessor =
        ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).expect("Failed to create accessor");
    accessor.create_table().expect("create table");

    // Insert same hash with two different modes
    let mut score_mode0 = make_test_score();
    score_mode0.mode = 0;
    score_mode0.clear = 5; // Normal
    score_mode0.maxcombo = 800;

    let mut score_mode1 = make_test_score();
    score_mode1.mode = 1;
    score_mode1.clear = 7; // ExHard
    score_mode1.maxcombo = 1250;

    accessor.set_score_data(&score_mode0);
    accessor.set_score_data(&score_mode1);

    let restored0 = accessor
        .score_data(&score_mode0.sha256, 0)
        .expect("Mode 0 score should exist");
    let restored1 = accessor
        .score_data(&score_mode1.sha256, 1)
        .expect("Mode 1 score should exist");

    assert_eq!(restored0.mode, 0);
    assert_eq!(restored0.clear, 5);
    assert_eq!(restored0.maxcombo, 800);

    assert_eq!(restored1.mode, 1);
    assert_eq!(restored1.clear, 7);
    assert_eq!(restored1.maxcombo, 1250);
}

#[test]
fn score_data_get_nonexistent_returns_none() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_score_empty.db");

    let accessor =
        ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).expect("Failed to create accessor");
    accessor.create_table().expect("create table");

    let result = accessor.score_data("nonexistent_hash", 0);
    assert!(result.is_none());
}

#[test]
fn score_data_overwrite_same_key() {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join("test_score_overwrite.db");

    let accessor =
        ScoreDatabaseAccessor::new(db_path.to_str().unwrap()).expect("Failed to create accessor");
    accessor.create_table().expect("create table");

    let mut score = make_test_score();
    score.clear = 5;
    score.maxcombo = 600;
    accessor.set_score_data(&score);

    // Overwrite with higher clear and combo (INSERT OR REPLACE on same PK)
    score.clear = 9; // Perfect
    score.maxcombo = 1290;
    accessor.set_score_data(&score);

    let restored = accessor
        .score_data(&score.sha256, score.mode)
        .expect("Overwritten score should exist");

    assert_eq!(restored.clear, 9);
    assert_eq!(restored.maxcombo, 1290);
}
