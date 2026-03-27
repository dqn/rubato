pub use rubato_types::score_data::*;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    #[test]
    fn test_score_data_default_construction() {
        let sd = ScoreData::default();
        assert_eq!(sd.player, "unknown");
        assert_eq!(sd.mode, 0);
        assert_eq!(sd.clear, 0);
        assert_eq!(sd.judge_counts.epg, 0);
        assert_eq!(sd.judge_counts.lpg, 0);
        assert_eq!(sd.judge_counts.egr, 0);
        assert_eq!(sd.judge_counts.lgr, 0);
        assert_eq!(sd.judge_counts.egd, 0);
        assert_eq!(sd.judge_counts.lgd, 0);
        assert_eq!(sd.judge_counts.ebd, 0);
        assert_eq!(sd.judge_counts.lbd, 0);
        assert_eq!(sd.judge_counts.epr, 0);
        assert_eq!(sd.judge_counts.lpr, 0);
        assert_eq!(sd.judge_counts.ems, 0);
        assert_eq!(sd.judge_counts.lms, 0);
        assert_eq!(sd.maxcombo, 0);
        assert_eq!(sd.notes, 0);
        assert_eq!(sd.passnotes, 0);
        assert_eq!(sd.minbp, i32::MAX);
        assert_eq!(sd.timing_stats.avgjudge, i64::MAX);
        assert_eq!(sd.play_option.seed, -1);
    }

    #[test]
    fn test_score_data_new_with_mode() {
        let sd = ScoreData::new(Mode::BEAT_5K);
        assert!(matches!(sd.playmode, Mode::BEAT_5K));

        let sd = ScoreData::new(Mode::POPN_9K);
        assert!(matches!(sd.playmode, Mode::POPN_9K));
    }

    #[test]
    fn test_score_data_getters_basic() {
        let mut sd = ScoreData::default();
        sd.sha256 = "abc123".to_string();
        sd.player = "player1".to_string();
        sd.mode = 7;
        sd.clear = 5;
        sd.date = 1234567890;
        sd.playcount = 10;
        sd.clearcount = 3;
        sd.maxcombo = 200;
        sd.notes = 500;
        sd.passnotes = 450;
        sd.minbp = 5;
        sd.play_option.random = 1;
        sd.play_option.option = 2;
        sd.play_option.seed = 42;
        sd.play_option.assist = 0;
        sd.play_option.gauge = 3;
        sd.state = 1;
        sd.scorehash = "hash123".to_string();
        sd.trophy = "trophy1".to_string();

        assert_eq!(sd.sha256, "abc123");
        assert_eq!(sd.player, "player1");
        assert_eq!(sd.mode, 7);
        assert_eq!(sd.clear, 5);
        assert_eq!(sd.date, 1234567890);
        assert_eq!(sd.playcount, 10);
        assert_eq!(sd.clearcount, 3);
        assert_eq!(sd.maxcombo, 200);
        assert_eq!(sd.notes, 500);
        assert_eq!(sd.passnotes, 450);
        assert_eq!(sd.minbp, 5);
        assert_eq!(sd.play_option.random, 1);
        assert_eq!(sd.play_option.option, 2);
        assert_eq!(sd.play_option.seed, 42);
        assert_eq!(sd.play_option.assist, 0);
        assert_eq!(sd.play_option.gauge, 3);
        assert_eq!(sd.state, 1);
        assert_eq!(sd.scorehash, "hash123");
        assert_eq!(sd.trophy, "trophy1");
    }

    #[test]
    fn test_score_data_judge_getters() {
        let mut sd = ScoreData::default();
        sd.judge_counts.epg = 10;
        sd.judge_counts.lpg = 20;
        sd.judge_counts.egr = 30;
        sd.judge_counts.lgr = 40;
        sd.judge_counts.egd = 5;
        sd.judge_counts.lgd = 6;
        sd.judge_counts.ebd = 2;
        sd.judge_counts.lbd = 3;
        sd.judge_counts.epr = 1;
        sd.judge_counts.lpr = 2;
        sd.judge_counts.ems = 0;
        sd.judge_counts.lms = 1;

        assert_eq!(sd.judge_counts.epg, 10);
        assert_eq!(sd.judge_counts.lpg, 20);
        assert_eq!(sd.judge_counts.egr, 30);
        assert_eq!(sd.judge_counts.lgr, 40);
        assert_eq!(sd.judge_counts.egd, 5);
        assert_eq!(sd.judge_counts.lgd, 6);
        assert_eq!(sd.judge_counts.ebd, 2);
        assert_eq!(sd.judge_counts.lbd, 3);
        assert_eq!(sd.judge_counts.epr, 1);
        assert_eq!(sd.judge_counts.lpr, 2);
        assert_eq!(sd.judge_counts.ems, 0);
        assert_eq!(sd.judge_counts.lms, 1);
    }

    #[test]
    fn test_score_data_exscore_calculation() {
        let mut sd = ScoreData::default();
        sd.judge_counts.epg = 10;
        sd.judge_counts.lpg = 20;
        sd.judge_counts.egr = 30;
        sd.judge_counts.lgr = 40;

        // exscore = (epg + lpg) * 2 + egr + lgr
        // = (10 + 20) * 2 + 30 + 40 = 60 + 70 = 130
        assert_eq!(sd.exscore(), 130);
    }

    #[test]
    fn test_score_data_exscore_zero() {
        let sd = ScoreData::default();
        assert_eq!(sd.exscore(), 0);
    }

    #[test]
    fn test_score_data_get_judge_count() {
        let mut sd = ScoreData::default();
        sd.judge_counts.epg = 10;
        sd.judge_counts.lpg = 20;
        sd.judge_counts.egr = 30;
        sd.judge_counts.lgr = 40;
        sd.judge_counts.egd = 5;
        sd.judge_counts.lgd = 6;
        sd.judge_counts.ebd = 2;
        sd.judge_counts.lbd = 3;
        sd.judge_counts.epr = 1;
        sd.judge_counts.lpr = 2;
        sd.judge_counts.ems = 7;
        sd.judge_counts.lms = 8;

        // judge 0 = PG
        assert_eq!(sd.judge_count(0, true), 10); // epg
        assert_eq!(sd.judge_count(0, false), 20); // lpg
        // judge 1 = GR
        assert_eq!(sd.judge_count(1, true), 30); // egr
        assert_eq!(sd.judge_count(1, false), 40); // lgr
        // judge 2 = GD
        assert_eq!(sd.judge_count(2, true), 5); // egd
        assert_eq!(sd.judge_count(2, false), 6); // lgd
        // judge 3 = BD
        assert_eq!(sd.judge_count(3, true), 2); // ebd
        assert_eq!(sd.judge_count(3, false), 3); // lbd
        // judge 4 = PR
        assert_eq!(sd.judge_count(4, true), 1); // epr
        assert_eq!(sd.judge_count(4, false), 2); // lpr
        // judge 5 = MS
        assert_eq!(sd.judge_count(5, true), 7); // ems
        assert_eq!(sd.judge_count(5, false), 8); // lms
        // invalid judge
        assert_eq!(sd.judge_count(6, true), 0);
        assert_eq!(sd.judge_count(-1, false), 0);
    }

    #[test]
    fn test_score_data_get_judge_count_total() {
        let mut sd = ScoreData::default();
        sd.judge_counts.epg = 10;
        sd.judge_counts.lpg = 20;
        sd.judge_counts.egr = 30;
        sd.judge_counts.lgr = 40;

        assert_eq!(sd.judge_count_total(0), 30); // epg + lpg
        assert_eq!(sd.judge_count_total(1), 70); // egr + lgr
    }

    #[test]
    fn test_score_data_add_judge_count() {
        let mut sd = ScoreData::default();

        sd.add_judge_count(0, true, 5);
        assert_eq!(sd.judge_counts.epg, 5);
        sd.add_judge_count(0, false, 3);
        assert_eq!(sd.judge_counts.lpg, 3);

        sd.add_judge_count(1, true, 10);
        assert_eq!(sd.judge_counts.egr, 10);
        sd.add_judge_count(1, false, 8);
        assert_eq!(sd.judge_counts.lgr, 8);

        sd.add_judge_count(2, true, 2);
        assert_eq!(sd.judge_counts.egd, 2);
        sd.add_judge_count(2, false, 1);
        assert_eq!(sd.judge_counts.lgd, 1);

        sd.add_judge_count(3, true, 4);
        assert_eq!(sd.judge_counts.ebd, 4);
        sd.add_judge_count(3, false, 7);
        assert_eq!(sd.judge_counts.lbd, 7);

        sd.add_judge_count(4, true, 6);
        assert_eq!(sd.judge_counts.epr, 6);
        sd.add_judge_count(4, false, 9);
        assert_eq!(sd.judge_counts.lpr, 9);

        sd.add_judge_count(5, true, 11);
        assert_eq!(sd.judge_counts.ems, 11);
        sd.add_judge_count(5, false, 12);
        assert_eq!(sd.judge_counts.lms, 12);

        // Invalid judge - no change
        sd.add_judge_count(6, true, 100);
        sd.add_judge_count(-1, false, 100);
    }

    #[test]
    fn test_score_data_add_judge_count_accumulates() {
        let mut sd = ScoreData::default();
        sd.add_judge_count(0, true, 5);
        sd.add_judge_count(0, true, 3);
        assert_eq!(sd.judge_counts.epg, 8);
    }

    #[test]
    fn test_score_data_set_player() {
        let mut sd = ScoreData::default();

        sd.set_player(Some("test_player"));
        assert_eq!(sd.player, "test_player");

        sd.set_player(None);
        assert_eq!(sd.player, "");
    }

    #[test]
    fn test_score_data_ghost_encode_decode_roundtrip() {
        let mut sd = ScoreData::default();
        sd.notes = 5;

        let ghost = vec![0, 1, 2, 1, 0];
        sd.encode_ghost(Some(&ghost));
        assert!(
            !sd.ghost.is_empty(),
            "ghost should be encoded to non-empty string"
        );

        let decoded = sd.decode_ghost();
        assert!(decoded.is_some());
        let decoded = decoded.unwrap();
        assert_eq!(decoded, ghost);
    }

    #[test]
    fn test_score_data_ghost_encode_none() {
        let mut sd = ScoreData::default();
        sd.ghost = "something".to_string();

        sd.encode_ghost(None);
        assert!(sd.ghost.is_empty());
    }

    #[test]
    fn test_score_data_ghost_encode_empty() {
        let mut sd = ScoreData::default();
        sd.ghost = "something".to_string();

        sd.encode_ghost(Some(&[]));
        assert!(sd.ghost.is_empty());
    }

    #[test]
    fn test_score_data_decode_ghost_empty_string() {
        let sd = ScoreData::default();
        assert!(sd.decode_ghost().is_none());
    }

    #[test]
    fn test_score_data_update_clear_improves() {
        let mut current = ScoreData::default();
        current.clear = 3;

        let mut newscore = ScoreData::default();
        newscore.clear = 5;

        let updated = current.update(&newscore, false);
        assert!(updated);
        assert_eq!(current.clear, 5);
    }

    #[test]
    fn test_score_data_update_clear_no_downgrade() {
        let mut current = ScoreData::default();
        current.clear = 5;

        let mut newscore = ScoreData::default();
        newscore.clear = 3;

        let updated = current.update(&newscore, false);
        assert!(!updated);
        assert_eq!(current.clear, 5);
    }

    #[test]
    fn test_score_data_update_exscore_improves() {
        let mut current = ScoreData::default();
        current.judge_counts.epg = 5;
        current.judge_counts.lpg = 5;

        let mut newscore = ScoreData::default();
        newscore.judge_counts.epg = 10;
        newscore.judge_counts.lpg = 10;
        newscore.judge_counts.egr = 5;

        let updated = current.update(&newscore, true);
        assert!(updated);
        assert_eq!(current.judge_counts.epg, 10);
        assert_eq!(current.judge_counts.lpg, 10);
        assert_eq!(current.judge_counts.egr, 5);
    }

    #[test]
    fn test_score_data_update_minbp_improves() {
        let mut current = ScoreData::default();
        current.minbp = 10;

        let mut newscore = ScoreData::default();
        newscore.minbp = 5;

        let updated = current.update(&newscore, true);
        assert!(updated);
        assert_eq!(current.minbp, 5);
    }

    #[test]
    fn test_score_data_update_combo_improves() {
        let mut current = ScoreData::default();
        current.maxcombo = 100;

        let mut newscore = ScoreData::default();
        newscore.maxcombo = 200;

        let updated = current.update(&newscore, true);
        assert!(updated);
        assert_eq!(current.maxcombo, 200);
    }

    #[test]
    fn test_score_data_serde_roundtrip() {
        let mut sd = ScoreData::default();
        sd.sha256 = "test_hash".to_string();
        sd.player = "player1".to_string();
        sd.judge_counts.epg = 100;
        sd.judge_counts.lpg = 200;
        sd.judge_counts.egr = 50;
        sd.judge_counts.lgr = 60;
        sd.clear = 5;
        sd.notes = 500;
        sd.maxcombo = 300;

        let json = serde_json::to_string(&sd).unwrap();
        let deserialized: ScoreData = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.sha256, "test_hash");
        assert_eq!(deserialized.player, "player1");
        assert_eq!(deserialized.judge_counts.epg, 100);
        assert_eq!(deserialized.judge_counts.lpg, 200);
        assert_eq!(deserialized.judge_counts.egr, 50);
        assert_eq!(deserialized.judge_counts.lgr, 60);
        assert_eq!(deserialized.clear, 5);
        assert_eq!(deserialized.notes, 500);
        assert_eq!(deserialized.maxcombo, 300);
    }

    #[test]
    fn test_score_data_display() {
        let sd = ScoreData::default();
        let display = format!("{}", sd);
        assert!(display.starts_with('{'));
        assert!(display.ends_with('}'));
        assert!(display.contains("\"Epg\""));
        assert!(display.contains("\"Exscore\""));
    }

    #[test]
    fn test_song_trophy_character_mapping() {
        assert_eq!(SongTrophy::Easy.character(), 'g');
        assert_eq!(SongTrophy::Groove.character(), 'G');
        assert_eq!(SongTrophy::Hard.character(), 'h');
        assert_eq!(SongTrophy::ExHard.character(), 'H');
        assert_eq!(SongTrophy::Normal.character(), 'n');
        assert_eq!(SongTrophy::Mirror.character(), 'm');
        assert_eq!(SongTrophy::Random.character(), 'r');
        assert_eq!(SongTrophy::RRandom.character(), 'o');
        assert_eq!(SongTrophy::SRandom.character(), 's');
        assert_eq!(SongTrophy::HRandom.character(), 'p');
        assert_eq!(SongTrophy::Spiral.character(), 'P');
        assert_eq!(SongTrophy::AllScr.character(), 'a');
        assert_eq!(SongTrophy::ExRandom.character(), 'R');
        assert_eq!(SongTrophy::ExSRandom.character(), 'S');
        assert_eq!(SongTrophy::Battle.character(), 'B');
        assert_eq!(SongTrophy::BattleAssist.character(), 'b');
    }

    #[test]
    fn test_song_trophy_values_count() {
        assert_eq!(SongTrophy::values().len(), 16);
    }

    #[test]
    fn test_song_trophy_get_trophy_valid() {
        assert_eq!(SongTrophy::trophy('g'), Some(SongTrophy::Easy));
        assert_eq!(SongTrophy::trophy('G'), Some(SongTrophy::Groove));
        assert_eq!(SongTrophy::trophy('H'), Some(SongTrophy::ExHard));
    }

    #[test]
    fn test_song_trophy_get_trophy_invalid() {
        assert_eq!(SongTrophy::trophy('x'), None);
        assert_eq!(SongTrophy::trophy('Z'), None);
    }

    #[test]
    fn test_song_trophy_roundtrip_via_character() {
        for trophy in SongTrophy::values() {
            let c = trophy.character();
            let recovered = SongTrophy::trophy(c);
            assert_eq!(recovered, Some(*trophy));
        }
    }

    #[test]
    fn test_score_data_trophy_constants() {
        assert_eq!(ScoreData::TROPHY_EASY, SongTrophy::Easy);
        assert_eq!(ScoreData::TROPHY_GROOVE, SongTrophy::Groove);
        assert_eq!(ScoreData::TROPHY_HARD, SongTrophy::Hard);
        assert_eq!(ScoreData::TROPHY_EXHARD, SongTrophy::ExHard);
        assert_eq!(ScoreData::TROPHY_NORMAL, SongTrophy::Normal);
        assert_eq!(ScoreData::TROPHY_MIRROR, SongTrophy::Mirror);
        assert_eq!(ScoreData::TROPHY_RANDOM, SongTrophy::Random);
        assert_eq!(ScoreData::TROPHY_R_RANDOM, SongTrophy::RRandom);
        assert_eq!(ScoreData::TROPHY_S_RANDOM, SongTrophy::SRandom);
        assert_eq!(ScoreData::TROPHY_H_RANDOM, SongTrophy::HRandom);
        assert_eq!(ScoreData::TROPHY_SPIRAL, SongTrophy::Spiral);
        assert_eq!(ScoreData::TROPHY_ALL_SCR, SongTrophy::AllScr);
        assert_eq!(ScoreData::TROPHY_EX_RANDOM, SongTrophy::ExRandom);
        assert_eq!(ScoreData::TROPHY_EX_S_RANDOM, SongTrophy::ExSRandom);
        assert_eq!(ScoreData::TROPHY_BATTLE, SongTrophy::Battle);
        assert_eq!(ScoreData::TROPHY_BATTLE_ASSIST, SongTrophy::BattleAssist);
    }
}
