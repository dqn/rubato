// Integration test: PlayDataAccessor path construction and DB initialization
//
// Tests that PlayDataAccessor correctly creates database files, reads/writes
// score data, and handles edge cases like nonexistent directories and null
// accessors.

use rubato_game::core::config::Config;
use rubato_game::core::play_data_accessor::{PlayDataAccessor, ScoreWriteContext};
use rubato_game::core::score_data::ScoreData;

/// Helper: create a Config pointing at a tempdir with the given player name.
fn make_config(playerpath: &str, playername: &str) -> Config {
    Config {
        paths: rubato_game::core::config::PathConfig {
            playerpath: playerpath.to_string(),
            ..Default::default()
        },
        playername: Some(playername.to_string()),
        ..Default::default()
    }
}

#[test]
fn new_creates_all_three_dbs() {
    let dir = tempfile::tempdir().unwrap();
    let player_dir = dir.path().join("player1");
    std::fs::create_dir_all(&player_dir).unwrap();

    let config = make_config(dir.path().to_str().unwrap(), "player1");
    let _accessor = PlayDataAccessor::new(&config);

    assert!(
        player_dir.join("score.db").exists(),
        "score.db should be created"
    );
    assert!(
        player_dir.join("scorelog.db").exists(),
        "scorelog.db should be created"
    );
    assert!(
        player_dir.join("scoredatalog.db").exists(),
        "scoredatalog.db should be created"
    );
}

#[test]
fn new_with_valid_config() {
    let dir = tempfile::tempdir().unwrap();
    let player_dir = dir.path().join("player1");
    std::fs::create_dir_all(&player_dir).unwrap();

    let config = make_config(dir.path().to_str().unwrap(), "player1");
    let accessor = PlayDataAccessor::new(&config);

    assert!(
        accessor.scoredb().is_some(),
        "scoredb should be Some for a valid config"
    );
}

#[test]
fn write_then_read_score() {
    let dir = tempfile::tempdir().unwrap();
    let player_dir = dir.path().join("player1");
    std::fs::create_dir_all(&player_dir).unwrap();

    let config = make_config(dir.path().to_str().unwrap(), "player1");
    let accessor = PlayDataAccessor::new(&config);

    let hash = "aabbccdd00112233aabbccdd00112233aabbccdd00112233aabbccdd00112233";

    let mut newscore = ScoreData::default();
    newscore.judge_counts.epg = 100;
    newscore.judge_counts.lpg = 90;
    newscore.judge_counts.egr = 50;
    newscore.judge_counts.lgr = 40;
    newscore.judge_counts.egd = 10;
    newscore.judge_counts.lgd = 8;
    newscore.judge_counts.ebd = 3;
    newscore.judge_counts.lbd = 2;
    newscore.judge_counts.epr = 1;
    newscore.judge_counts.lpr = 1;
    newscore.judge_counts.ems = 0;
    newscore.judge_counts.lms = 0;
    newscore.clear = 5; // Normal clear
    newscore.maxcombo = 300;
    newscore.minbp = 15;
    newscore.notes = 305;

    // contains_undefined_ln = false, total_notes = 305, lnmode = 0,
    // update_score = true, last_note_time_us = 120_000_000 (2 min)
    accessor.write_score_data(
        &newscore,
        &ScoreWriteContext {
            hash,
            contains_undefined_ln: false,
            total_notes: 305,
            lnmode: 0,
            update_score: true,
            last_note_time_us: 120_000_000,
        },
    );

    let restored = accessor
        .read_score_data_by_hash(hash, false, 0)
        .expect("Score should be readable after write");

    assert_eq!(restored.sha256, hash);
    assert_eq!(restored.judge_counts.epg, 100);
    assert_eq!(restored.judge_counts.lpg, 90);
    assert_eq!(restored.judge_counts.egr, 50);
    assert_eq!(restored.judge_counts.lgr, 40);
    assert_eq!(restored.judge_counts.egd, 10);
    assert_eq!(restored.judge_counts.lgd, 8);
    assert_eq!(restored.judge_counts.ebd, 3);
    assert_eq!(restored.judge_counts.lbd, 2);
    assert_eq!(restored.judge_counts.epr, 1);
    assert_eq!(restored.judge_counts.lpr, 1);
    assert_eq!(restored.judge_counts.ems, 0);
    assert_eq!(restored.judge_counts.lms, 0);
    assert_eq!(restored.clear, 5);
    assert_eq!(restored.maxcombo, 300);
    assert_eq!(restored.minbp, 15);
    assert_eq!(restored.notes, 305);
}

#[test]
fn new_creates_player_directory_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let player_dir = dir.path().join("freshplayer");

    // The player directory does not exist yet
    assert!(!player_dir.exists());

    let config = make_config(dir.path().to_str().unwrap(), "freshplayer");
    let accessor = PlayDataAccessor::new(&config);

    // The player directory should have been created
    assert!(player_dir.is_dir(), "player directory should be created");

    // All databases should be successfully opened
    assert!(
        accessor.scoredb().is_some(),
        "scoredb should be Some when directory is auto-created"
    );

    // Database files should exist
    assert!(
        player_dir.join("score.db").exists(),
        "score.db should be created"
    );
    assert!(
        player_dir.join("scorelog.db").exists(),
        "scorelog.db should be created"
    );
    assert!(
        player_dir.join("scoredatalog.db").exists(),
        "scoredatalog.db should be created"
    );
}

#[test]
fn nonexistent_dir_returns_none_dbs() {
    let config = make_config("/nonexistent/path/that/does/not/exist", "ghost_player");
    let accessor = PlayDataAccessor::new(&config);

    assert!(
        accessor.scoredb().is_none(),
        "scoredb should be None when directory does not exist"
    );
}

#[test]
fn null_accessor() {
    let accessor = PlayDataAccessor::null();

    assert!(
        accessor.read_player_data().is_none(),
        "null accessor read_player_data should return None"
    );
    assert!(
        accessor.scoredb().is_none(),
        "null accessor scoredb should return None"
    );
}

#[test]
fn replay_path_construction() {
    let dir = tempfile::tempdir().unwrap();
    let player_dir = dir.path().join("player1");
    std::fs::create_dir_all(&player_dir).unwrap();

    let config = make_config(dir.path().to_str().unwrap(), "player1");
    let accessor = PlayDataAccessor::new(&config);

    let hash = "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef";

    // No replay file exists, so exists_replay_data should return false
    assert!(
        !accessor.exists_replay_data(hash, false, 0, 0),
        "exists_replay_data should return false when no replay file exists"
    );

    // Verify the expected replay directory structure:
    // The replay file would be at {playerpath}/{player}/replay/{hash}.brd
    let expected_replay_dir = player_dir.join("replay");
    let expected_replay_file = expected_replay_dir.join(format!("{}.brd", hash));

    // Create the replay directory and a dummy .brd file to confirm the path
    std::fs::create_dir_all(&expected_replay_dir).unwrap();
    std::fs::write(&expected_replay_file, b"dummy").unwrap();

    // Now exists_replay_data should return true (file exists at expected path)
    assert!(
        accessor.exists_replay_data(hash, false, 0, 0),
        "exists_replay_data should return true when replay file exists at the expected path"
    );

    // Verify that a different hash does not match
    assert!(
        !accessor.exists_replay_data(
            "0000000000000000000000000000000000000000000000000000000000000000",
            false,
            0,
            0
        ),
        "exists_replay_data should return false for a different hash"
    );

    // Verify LN mode prefix: lnmode=2 uses "H" prefix
    let ln_hash = "1111111111111111111111111111111111111111111111111111111111111111";
    let ln_replay_file = expected_replay_dir.join(format!("H{}.brd", ln_hash));
    std::fs::write(&ln_replay_file, b"dummy").unwrap();
    assert!(
        accessor.exists_replay_data(ln_hash, true, 2, 0),
        "exists_replay_data with ln=true, lnmode=2 should use 'H' prefix"
    );

    // Verify index suffix: index=1 appends "_1"
    let idx_hash = "2222222222222222222222222222222222222222222222222222222222222222";
    let idx_replay_file = expected_replay_dir.join(format!("{}_1.brd", idx_hash));
    std::fs::write(&idx_replay_file, b"dummy").unwrap();
    assert!(
        accessor.exists_replay_data(idx_hash, false, 0, 1),
        "exists_replay_data with index=1 should append '_1' suffix"
    );
}
