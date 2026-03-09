#[cfg(test)]
use crate::clear_type::ClearType;
#[cfg(test)]
use crate::course_data::CourseDataConstraint;
#[cfg(test)]
use crate::score_data::ScoreData;
use crate::score_data_log_database_accessor::ScoreDataLogDatabaseAccessor;
use crate::score_database_accessor::ScoreDatabaseAccessor;
use crate::score_log_database_accessor::ScoreLogDatabaseAccessor;
static REPLAY: &[&str] = &["", "C", "H"];

/// Play data accessor.
/// Translated from Java: PlayDataAccessor
pub struct PlayDataAccessor {
    hashkey: String,
    player: String,
    playerpath: String,
    scoredb: Option<ScoreDatabaseAccessor>,
    scorelogdb: Option<ScoreLogDatabaseAccessor>,
    scoredatalogdb: Option<ScoreDataLogDatabaseAccessor>,
}

mod core;
mod model_course;

pub use self::core::{CourseScoreWriteContext, ScoreWriteContext};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::score_data::SongTrophy;

    /// Helper: create a PlayDataAccessor backed by in-memory databases via a temp directory.
    fn create_test_accessor(dir: &std::path::Path) -> PlayDataAccessor {
        let mut config = Config::default();
        config.paths.playerpath = dir.to_string_lossy().to_string();
        config.playername = Some("test".to_string());
        PlayDataAccessor::new(&config)
    }

    // ========================================================================
    // Course mode value encoding
    // ========================================================================

    #[test]
    fn test_course_mode_value_encoding_basic() {
        // Formula: ln_part + option*10 + hispeed*100 + judge*1000 + gauge*10000
        // With ln=true, lnmode=1, option=2, constraint=[NoSpeed, NoGood, GaugeLr2]:
        //   ln_part = 1
        //   option  = 2 * 10 = 20
        //   hispeed = 1 * 100 = 100 (NoSpeed)
        //   judge   = 1 * 1000 = 1000 (NoGood)
        //   gauge   = 1 * 10000 = 10000 (GaugeLr2)
        //   total   = 1 + 20 + 100 + 1000 + 10000 = 11121
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Failed.id(); // won't increment clearcount
        newscore.notes = 100;
        newscore.judge_counts.epg = 10;
        newscore.judge_counts.lpg = 10;
        newscore.minbp = 5;

        let hashes = &["abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890"];
        let constraint = &[
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::NoGood,
            CourseDataConstraint::GaugeLr2,
        ];

        accessor.write_score_data_for_course(
            &newscore,
            &core::CourseScoreWriteContext {
                hashes,
                total_notes: 100,
                ln: true,
                lnmode: 1,
                option: 2,
                constraint,
                update_score: true,
            },
        );

        // Read it back: the mode value should be 11121
        let expected_mode = 1 + 2 * 10 + 100 + 1000 + 10000;
        assert_eq!(expected_mode, 11121, "mode formula verification");

        let hash: String = hashes.join("");
        let score = accessor.scoredb.as_ref().unwrap().score_data(&hash, 11121);
        assert!(
            score.is_some(),
            "score should exist with mode=11121 in the database"
        );
    }

    #[test]
    fn test_course_mode_value_encoding_no_ln() {
        // When ln=false, ln_part=0 regardless of lnmode
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Normal.id();
        newscore.notes = 50;
        newscore.judge_counts.epg = 5;
        newscore.minbp = 2;

        let hashes = &["a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2c3d4e5f6a1b2"];

        // No constraints, option=3, ln=false, lnmode=2
        // mode = 0 + 3*10 + 0 + 0 + 0 = 30
        accessor.write_score_data_for_course(
            &newscore,
            &core::CourseScoreWriteContext {
                hashes,
                total_notes: 50,
                ln: false,
                lnmode: 2,
                option: 3,
                constraint: &[],
                update_score: true,
            },
        );

        let hash: String = hashes.join("");
        let score = accessor.scoredb.as_ref().unwrap().score_data(&hash, 30);
        assert!(
            score.is_some(),
            "score should exist with mode=30 (ln disabled, option=3)"
        );
    }

    #[test]
    fn test_course_mode_value_encoding_all_digits() {
        // ln=1, option=2, hispeed=3(NoSpeed=1? no, let's pick specific values)
        // To get the classic 54321 example:
        //   gauge=5 (Gauge24Keys), judge=4(impossible, max is NoGreat=2),
        // Let's verify a realistic max:
        //   gauge: Gauge24Keys => gauge=5, so 5*10000=50000
        //   judge: NoGreat => judge=2, so 2*1000=2000
        //   hispeed: NoSpeed => hispeed=1, so 1*100=100
        //   option=3, so 3*10=30
        //   ln_part=1 (ln=true, lnmode=1)
        //   total = 50000 + 2000 + 100 + 30 + 1 = 52131
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let mut newscore = ScoreData::default();
        newscore.clear = ClearType::Failed.id();
        newscore.notes = 10;
        newscore.judge_counts.epg = 1;
        newscore.minbp = 1;

        let hashes = &["0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"];
        let constraint = &[
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::NoGreat,
            CourseDataConstraint::Gauge24Keys,
        ];

        accessor.write_score_data_for_course(
            &newscore,
            &core::CourseScoreWriteContext {
                hashes,
                total_notes: 10,
                ln: true,
                lnmode: 1,
                option: 3,
                constraint,
                update_score: true,
            },
        );

        let expected_mode = 1 + 3 * 10 + 100 + 2 * 1000 + 5 * 10000;
        assert_eq!(expected_mode, 52131, "mode value for all constraint types");

        let hash: String = hashes.join("");
        let score = accessor
            .scoredb
            .as_ref()
            .unwrap()
            .score_data(&hash, expected_mode);
        assert!(
            score.is_some(),
            "score should exist with mode={}",
            expected_mode
        );
    }

    // ========================================================================
    // compute_constraint_values
    // ========================================================================

    #[test]
    fn test_compute_constraint_values_empty() {
        let (hispeed, judge, gauge) = PlayDataAccessor::compute_constraint_values(&[]);
        assert_eq!(hispeed, 0, "empty constraints: hispeed");
        assert_eq!(judge, 0, "empty constraints: judge");
        assert_eq!(gauge, 0, "empty constraints: gauge");
    }

    #[test]
    fn test_compute_constraint_values_no_speed() {
        let (hispeed, judge, gauge) =
            PlayDataAccessor::compute_constraint_values(&[CourseDataConstraint::NoSpeed]);
        assert_eq!(hispeed, 1, "NoSpeed sets hispeed=1");
        assert_eq!(judge, 0);
        assert_eq!(gauge, 0);
    }

    #[test]
    fn test_compute_constraint_values_no_good() {
        let (hispeed, judge, gauge) =
            PlayDataAccessor::compute_constraint_values(&[CourseDataConstraint::NoGood]);
        assert_eq!(hispeed, 0);
        assert_eq!(judge, 1, "NoGood sets judge=1");
        assert_eq!(gauge, 0);
    }

    #[test]
    fn test_compute_constraint_values_no_great() {
        let (_, judge, _) =
            PlayDataAccessor::compute_constraint_values(&[CourseDataConstraint::NoGreat]);
        assert_eq!(judge, 2, "NoGreat sets judge=2");
    }

    #[test]
    fn test_compute_constraint_values_gauge_variants() {
        let cases = [
            (CourseDataConstraint::GaugeLr2, 1),
            (CourseDataConstraint::Gauge5Keys, 2),
            (CourseDataConstraint::Gauge7Keys, 3),
            (CourseDataConstraint::Gauge9Keys, 4),
            (CourseDataConstraint::Gauge24Keys, 5),
        ];
        for (constraint, expected_gauge) in cases {
            let (_, _, gauge) = PlayDataAccessor::compute_constraint_values(&[constraint]);
            assert_eq!(
                gauge, expected_gauge,
                "{:?} should set gauge={}",
                constraint, expected_gauge
            );
        }
    }

    #[test]
    fn test_compute_constraint_values_combined() {
        let constraints = &[
            CourseDataConstraint::NoSpeed,
            CourseDataConstraint::NoGreat,
            CourseDataConstraint::Gauge7Keys,
        ];
        let (hispeed, judge, gauge) = PlayDataAccessor::compute_constraint_values(constraints);
        assert_eq!(hispeed, 1, "combined: hispeed from NoSpeed");
        assert_eq!(judge, 2, "combined: judge from NoGreat");
        assert_eq!(gauge, 3, "combined: gauge from Gauge7Keys");
    }

    #[test]
    fn test_compute_constraint_values_ignores_non_matching() {
        // Class, Mirror, Random, Ln, Cn, Hcn do not affect hispeed/judge/gauge
        let constraints = &[
            CourseDataConstraint::Class,
            CourseDataConstraint::Mirror,
            CourseDataConstraint::Random,
            CourseDataConstraint::Ln,
            CourseDataConstraint::Cn,
            CourseDataConstraint::Hcn,
        ];
        let (hispeed, judge, gauge) = PlayDataAccessor::compute_constraint_values(constraints);
        assert_eq!(
            hispeed, 0,
            "class/mirror/random/ln/cn/hcn: hispeed unchanged"
        );
        assert_eq!(judge, 0, "class/mirror/random/ln/cn/hcn: judge unchanged");
        assert_eq!(gauge, 0, "class/mirror/random/ln/cn/hcn: gauge unchanged");
    }

    // ========================================================================
    // Course hash construction (first 10 chars of each hash concatenated)
    // ========================================================================

    #[test]
    fn test_course_replay_path_hash_truncation() {
        // get_replay_data_file_path_course truncates each hash to first 10 chars
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "testplayer".to_string(),
            playerpath: "/tmp/test".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let hashes = &[
            "abcdefghij1234567890",
            "1234567890abcdefghij",
            "zzzzzzzzzzxxxxxxxxxx",
        ];
        let path = accessor.get_replay_data_file_path_course(
            hashes,
            false, // ln
            0,     // lnmode
            0,     // index
            &[],   // no constraints
        );

        // Expected: folder/abcdefghij1234567890zzzzzzzzzz
        let expected_hash = "abcdefghij1234567890zzzzzzzzzz";
        assert!(
            path.contains(expected_hash),
            "path should contain truncated+concatenated hash '{}', got: {}",
            expected_hash,
            path
        );
    }

    #[test]
    fn test_course_replay_path_short_hash() {
        // If a hash is shorter than 10 chars, use full hash
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/tmp".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let hashes = &["short", "also"];
        let path = accessor.get_replay_data_file_path_course(hashes, false, 0, 0, &[]);
        assert!(
            path.contains("shortalso"),
            "short hashes should be used as-is, got: {}",
            path
        );
    }

    #[test]
    fn test_course_replay_path_with_ln_prefix() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/tmp".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        // lnmode=1 => prefix "C", lnmode=2 => prefix "H"
        let hashes = &["abcdefghijklmnop"];
        let path_c = accessor.get_replay_data_file_path_course(hashes, true, 1, 0, &[]);
        assert!(
            path_c.contains("Cabcdefghij"),
            "ln=true, lnmode=1 should prefix 'C', got: {}",
            path_c
        );

        let path_h = accessor.get_replay_data_file_path_course(hashes, true, 2, 0, &[]);
        assert!(
            path_h.contains("Habcdefghij"),
            "ln=true, lnmode=2 should prefix 'H', got: {}",
            path_h
        );
    }

    #[test]
    fn test_course_replay_path_with_index() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/tmp".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let hashes = &["abcdefghijklmnop"];
        let path = accessor.get_replay_data_file_path_course(hashes, false, 0, 3, &[]);
        assert!(
            path.ends_with("_3"),
            "index=3 should append '_3', got: {}",
            path
        );

        let path_no_index = accessor.get_replay_data_file_path_course(hashes, false, 0, 0, &[]);
        assert!(
            !path_no_index.ends_with("_0"),
            "index=0 should not append suffix, got: {}",
            path_no_index
        );
    }

    #[test]
    fn test_course_replay_path_with_constraint_suffix() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/tmp".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let hashes = &["abcdefghijklmnop"];
        // NoSpeed ordinal=3, so suffix = "_04" (ordinal+1, 2-digit)
        let path = accessor.get_replay_data_file_path_course(
            hashes,
            false,
            0,
            0,
            &[CourseDataConstraint::NoSpeed],
        );
        assert!(
            path.contains("_04"),
            "NoSpeed (ordinal=3) should produce suffix '_04', got: {}",
            path
        );

        // Class/Mirror/Random are excluded from constraint suffix
        let path_class = accessor.get_replay_data_file_path_course(
            hashes,
            false,
            0,
            0,
            &[CourseDataConstraint::Class],
        );
        assert!(
            !path_class.contains("_01"),
            "Class should be excluded from constraint suffix, got: {}",
            path_class
        );
    }

    // ========================================================================
    // Today player data delta (subtraction logic)
    // ========================================================================

    #[test]
    fn test_read_today_player_data_with_two_rows() {
        // When there are 2 player data rows, today's delta = pd[0] - pd[1]
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let scoredb = accessor.scoredb.as_ref().unwrap();

        // Insert two player data rows (most recent first = pd[0] in DESC order)
        // The set_player_data method uses today's date, so we insert manually.
        let conn = scoredb.connection();

        // Earlier snapshot (yesterday)
        conn.execute(
            "INSERT INTO player (date, playcount, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, playtime, maxcombo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![1000, 10, 5, 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 5, 3, 3600, 200],
        ).unwrap();

        // More recent snapshot (today) - cumulative values are higher
        conn.execute(
            "INSERT INTO player (date, playcount, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, playtime, maxcombo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![2000, 15, 8, 150, 130, 120, 110, 90, 80, 60, 50, 35, 25, 10, 7, 5400, 250],
        ).unwrap();

        let today = accessor.read_today_player_data();
        assert!(
            today.is_some(),
            "should return player data when 2 rows exist"
        );

        let pd = today.unwrap();
        // Delta = pd[0] (date=2000, more recent) - pd[1] (date=1000, earlier)
        assert_eq!(pd.playcount, 15 - 10, "playcount delta");
        assert_eq!(pd.clear, 8 - 5, "clear delta");
        assert_eq!(pd.epg, 150 - 100, "epg delta");
        assert_eq!(pd.lpg, 130 - 90, "lpg delta");
        assert_eq!(pd.egr, 120 - 80, "egr delta");
        assert_eq!(pd.lgr, 110 - 70, "lgr delta");
        assert_eq!(pd.egd, 90 - 60, "egd delta");
        assert_eq!(pd.lgd, 80 - 50, "lgd delta");
        assert_eq!(pd.ebd, 60 - 40, "ebd delta");
        assert_eq!(pd.lbd, 50 - 30, "lbd delta");
        assert_eq!(pd.epr, 35 - 20, "epr delta");
        assert_eq!(pd.lpr, 25 - 10, "lpr delta");
        assert_eq!(pd.ems, 10 - 5, "ems delta");
        assert_eq!(pd.lms, 7 - 3, "lms delta");
        assert_eq!(pd.playtime, 5400 - 3600, "playtime delta");
    }

    #[test]
    fn test_read_today_player_data_single_row() {
        // When there is only 1 row, return it as-is (no subtraction)
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let scoredb = accessor.scoredb.as_ref().unwrap();
        let conn = scoredb.connection();

        conn.execute(
            "INSERT INTO player (date, playcount, clear, epg, lpg, egr, lgr, egd, lgd, ebd, lbd, epr, lpr, ems, lms, playtime, maxcombo) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17)",
            rusqlite::params![1000, 42, 10, 100, 90, 80, 70, 60, 50, 40, 30, 20, 10, 5, 3, 7200, 300],
        ).unwrap();

        let today = accessor.read_today_player_data();
        assert!(
            today.is_some(),
            "should return player data when 1 row exists"
        );

        let pd = today.unwrap();
        assert_eq!(pd.playcount, 42, "single row: playcount returned as-is");
        assert_eq!(pd.epg, 100, "single row: epg returned as-is");
    }

    #[test]
    fn test_read_today_player_data_no_rows() {
        // When there are no rows beyond the initial default, check behavior.
        // Note: create_table inserts a default row, so we test with it removed.
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let scoredb = accessor.scoredb.as_ref().unwrap();
        let conn = scoredb.connection();
        conn.execute("DELETE FROM player", []).unwrap();

        let today = accessor.read_today_player_data();
        assert!(
            today.is_none(),
            "should return None when no player data rows exist"
        );
    }

    // ========================================================================
    // Trophy accumulation (HashSet union semantics)
    // ========================================================================

    #[test]
    fn test_trophy_accumulation_preserves_old_trophies() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let hash = "a".repeat(64);

        // First play: get Easy and Normal trophies
        let mut score1 = ScoreData::default();
        score1.clear = ClearType::Easy.id();
        score1.play_option.gauge = 0;
        score1.play_option.option = 0; // option index 0 = Normal trophy
        score1.notes = 100;
        score1.judge_counts.epg = 50;
        score1.judge_counts.lpg = 30;
        score1.minbp = 5;
        score1.maxcombo = 80;

        accessor.write_score_data(
            &score1,
            &core::ScoreWriteContext {
                hash: &hash,
                contains_undefined_ln: false,
                total_notes: 100,
                lnmode: 0,
                update_score: true,
                last_note_time_us: 60_000_000,
            },
        );

        let saved1 = accessor.read_score_data_by_hash(&hash, false, 0).unwrap();
        let trophies1: std::collections::HashSet<char> = saved1.trophy.chars().collect();
        assert!(
            trophies1.contains(&SongTrophy::Easy.character()),
            "first play should earn Easy trophy, got: {}",
            saved1.trophy
        );
        assert!(
            trophies1.contains(&SongTrophy::Normal.character()),
            "first play with option=0 and clear>=Easy should earn Normal trophy, got: {}",
            saved1.trophy
        );

        // Second play: get Hard trophy with Mirror option
        let mut score2 = ScoreData::default();
        score2.clear = ClearType::Hard.id();
        score2.play_option.gauge = 0;
        score2.play_option.option = 1; // option index 1 = Mirror
        score2.notes = 100;
        score2.judge_counts.epg = 60;
        score2.judge_counts.lpg = 35;
        score2.minbp = 3;
        score2.maxcombo = 90;

        accessor.write_score_data(
            &score2,
            &core::ScoreWriteContext {
                hash: &hash,
                contains_undefined_ln: false,
                total_notes: 100,
                lnmode: 0,
                update_score: true,
                last_note_time_us: 60_000_000,
            },
        );

        let saved2 = accessor.read_score_data_by_hash(&hash, false, 0).unwrap();
        let trophies2: std::collections::HashSet<char> = saved2.trophy.chars().collect();

        // Old trophies should still be present (union semantics)
        assert!(
            trophies2.contains(&SongTrophy::Easy.character()),
            "Easy trophy should be preserved after second play, got: {}",
            saved2.trophy
        );
        assert!(
            trophies2.contains(&SongTrophy::Normal.character()),
            "Normal trophy should be preserved after second play, got: {}",
            saved2.trophy
        );
        // New trophies should be added
        assert!(
            trophies2.contains(&SongTrophy::Hard.character()),
            "Hard trophy should be earned on second play, got: {}",
            saved2.trophy
        );
        assert!(
            trophies2.contains(&SongTrophy::Mirror.character()),
            "Mirror trophy should be earned with option=1 and clear>=Easy, got: {}",
            saved2.trophy
        );
    }

    #[test]
    fn test_trophy_exhard_clear() {
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let hash = "b".repeat(64);

        let mut score = ScoreData::default();
        score.clear = ClearType::ExHard.id();
        score.play_option.gauge = 0;
        score.play_option.option = 0;
        score.notes = 100;
        score.judge_counts.epg = 50;
        score.judge_counts.lpg = 30;
        score.minbp = 2;
        score.maxcombo = 80;

        accessor.write_score_data(
            &score,
            &core::ScoreWriteContext {
                hash: &hash,
                contains_undefined_ln: false,
                total_notes: 100,
                lnmode: 0,
                update_score: true,
                last_note_time_us: 60_000_000,
            },
        );

        let saved = accessor.read_score_data_by_hash(&hash, false, 0).unwrap();
        let trophies: std::collections::HashSet<char> = saved.trophy.chars().collect();

        // ExHard clear should earn both ExHard and Hard trophies
        assert!(
            trophies.contains(&SongTrophy::ExHard.character()),
            "ExHard clear should earn ExHard trophy, got: {}",
            saved.trophy
        );
        assert!(
            trophies.contains(&SongTrophy::Hard.character()),
            "ExHard clear should also earn Hard trophy, got: {}",
            saved.trophy
        );
    }

    #[test]
    fn test_trophy_no_gauge_minus_one() {
        // When gauge == -1, no clear trophies should be added
        let dir = tempfile::tempdir().unwrap();
        let accessor = create_test_accessor(dir.path());

        let hash = "c".repeat(64);

        let mut score = ScoreData::default();
        score.clear = ClearType::Hard.id();
        score.play_option.gauge = -1; // special value: skip clear trophies
        score.play_option.option = 0;
        score.notes = 100;
        score.judge_counts.epg = 50;
        score.judge_counts.lpg = 30;
        score.minbp = 5;
        score.maxcombo = 80;

        accessor.write_score_data(
            &score,
            &core::ScoreWriteContext {
                hash: &hash,
                contains_undefined_ln: false,
                total_notes: 100,
                lnmode: 0,
                update_score: true,
                last_note_time_us: 60_000_000,
            },
        );

        let saved = accessor.read_score_data_by_hash(&hash, false, 0).unwrap();
        let trophies: std::collections::HashSet<char> = saved.trophy.chars().collect();

        // No clear trophies (Easy, Groove, Hard, ExHard) should be present
        assert!(
            !trophies.contains(&SongTrophy::Hard.character()),
            "gauge=-1 should skip Hard trophy, got: {}",
            saved.trophy
        );
        assert!(
            !trophies.contains(&SongTrophy::Easy.character()),
            "gauge=-1 should skip Easy trophy, got: {}",
            saved.trophy
        );
    }

    // ========================================================================
    // Score hash (SHA-256 construction)
    // ========================================================================

    #[test]
    fn test_score_hash_deterministic() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.sha256 = "testhash".to_string();
        score.judge_counts.epg = 10;
        score.judge_counts.lpg = 20;
        score.judge_counts.egr = 5;
        score.judge_counts.lgr = 3;
        score.clear = 5;
        score.minbp = 2;
        score.maxcombo = 100;
        score.mode = 0;
        score.clearcount = 1;
        score.playcount = 5;
        score.play_option.option = 0;
        score.play_option.random = 0;
        score.trophy = "gn".to_string();
        score.date = 1700000000;

        let hash1 = accessor.get_score_hash(&score);
        let hash2 = accessor.get_score_hash(&score);

        assert!(hash1.is_some(), "score hash should be produced");
        assert_eq!(hash1, hash2, "same inputs should produce the same hash");

        // Must start with "035" prefix
        assert!(
            hash1.as_ref().unwrap().starts_with("035"),
            "score hash should start with '035' prefix, got: {}",
            hash1.unwrap()
        );
    }

    #[test]
    fn test_score_hash_changes_with_different_input() {
        let accessor = PlayDataAccessor::null();

        let mut score1 = ScoreData::default();
        score1.sha256 = "hash1".to_string();
        score1.judge_counts.epg = 10;
        score1.date = 1000;

        let mut score2 = ScoreData::default();
        score2.sha256 = "hash2".to_string();
        score2.judge_counts.epg = 10;
        score2.date = 1000;

        let h1 = accessor.get_score_hash(&score1).unwrap();
        let h2 = accessor.get_score_hash(&score2).unwrap();

        assert_ne!(
            h1, h2,
            "different sha256 should produce different score hashes"
        );
    }

    #[test]
    fn test_score_hash_length() {
        // SHA-256 produces 64 hex chars + "035" prefix = 67 chars
        let accessor = PlayDataAccessor::null();
        let score = ScoreData::default();
        let hash = accessor.get_score_hash(&score).unwrap();

        assert_eq!(
            hash.len(),
            67,
            "score hash should be 3 (prefix) + 64 (sha256 hex) = 67 chars, got: {}",
            hash.len()
        );
    }

    // ========================================================================
    // update_score (ScoreLog construction)
    // ========================================================================

    #[test]
    fn test_update_score_clear_improvement() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.clear = 3;
        score.judge_counts.epg = 10;
        score.judge_counts.lpg = 10;
        score.minbp = 10;
        score.maxcombo = 50;

        let mut newscore = ScoreData::default();
        newscore.clear = 5;
        newscore.judge_counts.epg = 10;
        newscore.judge_counts.lpg = 10;
        newscore.minbp = 10;
        newscore.maxcombo = 50;

        let log = accessor.update_score(&mut score, &newscore, "testhash", false);

        assert_eq!(log.oldclear, 3, "log should record old clear");
        assert_eq!(log.clear, 5, "log should record new clear");
        assert!(
            log.sha256.is_some(),
            "sha256 should be set when there's an improvement"
        );
    }

    #[test]
    fn test_update_score_no_improvement() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.clear = 5;
        score.judge_counts.epg = 50;
        score.judge_counts.lpg = 50;
        score.minbp = 2;
        score.maxcombo = 100;

        let mut newscore = ScoreData::default();
        newscore.clear = 3; // lower
        newscore.judge_counts.epg = 30; // lower exscore
        newscore.judge_counts.lpg = 30;
        newscore.minbp = 5; // higher (worse)
        newscore.maxcombo = 80; // lower

        let log = accessor.update_score(&mut score, &newscore, "testhash", true);

        assert!(
            log.sha256.is_none(),
            "sha256 should be None when no improvement"
        );
        assert_eq!(log.oldclear, 5);
        assert_eq!(log.clear, 5, "clear should stay at old value");
        assert_eq!(log.oldscore, score.exscore());
        assert_eq!(log.score, score.exscore(), "score should stay at old value");
    }

    #[test]
    fn test_update_score_exscore_improvement_with_update_flag() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.judge_counts.epg = 10;
        score.judge_counts.lpg = 10;

        let mut newscore = ScoreData::default();
        newscore.judge_counts.epg = 50;
        newscore.judge_counts.lpg = 50;

        let log = accessor.update_score(&mut score, &newscore, "hash", true);

        assert!(
            log.sha256.is_some(),
            "sha256 should be set for exscore improvement"
        );
        assert_eq!(log.oldscore, (10 + 10) * 2, "old exscore = 40");
        assert_eq!(log.score, (50 + 50) * 2, "new exscore = 200");
    }

    #[test]
    fn test_update_score_exscore_not_updated_without_flag() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.judge_counts.epg = 10;
        score.judge_counts.lpg = 10;
        score.minbp = 10;
        score.maxcombo = 50;

        let mut newscore = ScoreData::default();
        newscore.judge_counts.epg = 50;
        newscore.judge_counts.lpg = 50;
        newscore.minbp = 5;
        newscore.maxcombo = 100;

        let log = accessor.update_score(&mut score, &newscore, "hash", false);

        // update_score=false means only clear is checked for updates,
        // exscore/minbp/combo are not updated
        assert_eq!(
            log.score, log.oldscore,
            "exscore should not change when update_score=false"
        );
        assert_eq!(
            log.minbp, log.oldminbp,
            "minbp should not change when update_score=false"
        );
        assert_eq!(
            log.combo, log.oldcombo,
            "combo should not change when update_score=false"
        );
    }

    #[test]
    fn test_update_score_minbp_improvement() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.minbp = 10;

        let mut newscore = ScoreData::default();
        newscore.minbp = 3;

        let log = accessor.update_score(&mut score, &newscore, "hash", true);

        assert_eq!(log.oldminbp, 10, "log should record old minbp");
        assert_eq!(log.minbp, 3, "log should record new minbp");
        assert!(
            log.sha256.is_some(),
            "sha256 should be set for minbp improvement"
        );
    }

    #[test]
    fn test_update_score_combo_improvement() {
        let accessor = PlayDataAccessor::null();

        let mut score = ScoreData::default();
        score.maxcombo = 50;

        let mut newscore = ScoreData::default();
        newscore.maxcombo = 100;

        let log = accessor.update_score(&mut score, &newscore, "hash", true);

        assert_eq!(log.oldcombo, 50, "log should record old combo");
        assert_eq!(log.combo, 100, "log should record new combo");
        assert!(
            log.sha256.is_some(),
            "sha256 should be set for combo improvement"
        );
    }

    // ========================================================================
    // constraint_ordinal mapping
    // ========================================================================

    #[test]
    fn test_constraint_ordinal_all_variants() {
        let expected = [
            (CourseDataConstraint::Class, 0),
            (CourseDataConstraint::Mirror, 1),
            (CourseDataConstraint::Random, 2),
            (CourseDataConstraint::NoSpeed, 3),
            (CourseDataConstraint::NoGood, 4),
            (CourseDataConstraint::NoGreat, 5),
            (CourseDataConstraint::GaugeLr2, 6),
            (CourseDataConstraint::Gauge5Keys, 7),
            (CourseDataConstraint::Gauge7Keys, 8),
            (CourseDataConstraint::Gauge9Keys, 9),
            (CourseDataConstraint::Gauge24Keys, 10),
            (CourseDataConstraint::Ln, 11),
            (CourseDataConstraint::Cn, 12),
            (CourseDataConstraint::Hcn, 13),
        ];
        for (constraint, ordinal) in expected {
            assert_eq!(
                PlayDataAccessor::constraint_ordinal(&constraint),
                ordinal,
                "{:?} should have ordinal {}",
                constraint,
                ordinal
            );
        }
    }

    // ========================================================================
    // Replay data file path (single model)
    // ========================================================================

    #[test]
    fn test_replay_data_file_path_basic() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "player1".to_string(),
            playerpath: "/game".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let path = accessor.get_replay_data_file_path("abc123", false, 0, 0);
        let sep = std::path::MAIN_SEPARATOR;
        let expected = format!("/game{}player1{}replay{}abc123", sep, sep, sep);
        assert_eq!(path, expected, "basic replay path without LN or index");
    }

    #[test]
    fn test_replay_data_file_path_with_ln() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/g".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let sep = std::path::MAIN_SEPARATOR;

        // lnmode=0 => prefix ""
        let path0 = accessor.get_replay_data_file_path("hash", true, 0, 0);
        assert_eq!(
            path0,
            format!("/g{}p{}replay{}hash", sep, sep, sep),
            "lnmode=0 should have empty prefix"
        );

        // lnmode=1 => prefix "C"
        let path1 = accessor.get_replay_data_file_path("hash", true, 1, 0);
        assert_eq!(
            path1,
            format!("/g{}p{}replay{}Chash", sep, sep, sep),
            "lnmode=1 should have 'C' prefix"
        );

        // lnmode=2 => prefix "H"
        let path2 = accessor.get_replay_data_file_path("hash", true, 2, 0);
        assert_eq!(
            path2,
            format!("/g{}p{}replay{}Hhash", sep, sep, sep),
            "lnmode=2 should have 'H' prefix"
        );
    }

    #[test]
    fn test_replay_data_file_path_with_index() {
        let accessor = PlayDataAccessor {
            hashkey: String::new(),
            player: "p".to_string(),
            playerpath: "/g".to_string(),
            scoredb: None,
            scorelogdb: None,
            scoredatalogdb: None,
        };

        let path = accessor.get_replay_data_file_path("hash", false, 0, 2);
        assert!(
            path.ends_with("hash_2"),
            "index=2 should append '_2', got: {}",
            path
        );
    }

    // ========================================================================
    // Null accessor (no-op methods)
    // ========================================================================

    #[test]
    fn test_null_accessor_reads_return_none() {
        let accessor = PlayDataAccessor::null();

        assert!(
            accessor.read_player_data().is_none(),
            "null: read_player_data"
        );
        assert!(
            accessor.read_today_player_data().is_none(),
            "null: read_today_player_data"
        );
        assert!(
            accessor.read_score_data_by_hash("hash", false, 0).is_none(),
            "null: read_score_data_by_hash"
        );
        assert!(accessor.scoredb().is_none(), "null: scoredb");
    }

    #[test]
    fn test_null_accessor_writes_do_not_panic() {
        let accessor = PlayDataAccessor::null();

        let score = ScoreData::default();
        // These should all be no-ops without panicking
        accessor.write_score_data(
            &score,
            &core::ScoreWriteContext {
                hash: "hash",
                contains_undefined_ln: false,
                total_notes: 100,
                lnmode: 0,
                update_score: true,
                last_note_time_us: 60_000_000,
            },
        );
        accessor.write_score_data_for_course(
            &score,
            &core::CourseScoreWriteContext {
                hashes: &["h1", "h2"],
                total_notes: 100,
                ln: false,
                lnmode: 0,
                option: 0,
                constraint: &[],
                update_score: true,
            },
        );
        accessor.update_player_data(&score, 60);
        accessor.delete_score_data("hash", false, 0);
    }
}
