pub use rubato_types::player_config::*;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use bms::model::mode::Mode;

    #[test]
    fn test_player_config_default_construction() {
        let pc = PlayerConfig::default();
        assert!(pc.id.is_none());
        assert_eq!(pc.name, "NO NAME");
        assert_eq!(pc.play_settings.gauge, 0);
        assert_eq!(pc.play_settings.random, 0);
        assert_eq!(pc.play_settings.random2, 0);
        assert_eq!(pc.play_settings.doubleoption, 0);
        assert_eq!(pc.play_settings.chart_replication_mode, "RIVALCHART");
        assert_eq!(pc.select_settings.targetid, "MAX");
        assert_eq!(pc.judge_settings.judgetiming, 0);
        assert!(!pc.judge_settings.notes_display_timing_auto_adjust);
        assert!(pc.mode.is_none());
        assert_eq!(pc.play_settings.lnmode, 0);
        assert!(!pc.play_settings.forcedcnendings);
        assert!(!pc.judge_settings.custom_judge);
        assert!(!pc.display_settings.bpmguide);
        assert!(!pc.display_settings.showjudgearea);
        assert!(!pc.display_settings.markprocessednote);
    }

    #[test]
    fn test_player_config_default_judge_window_rates() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.judge_settings.key_judge_window_rate_perfect_great, 400);
        assert_eq!(pc.judge_settings.key_judge_window_rate_great, 400);
        assert_eq!(pc.judge_settings.key_judge_window_rate_good, 100);
        assert_eq!(
            pc.judge_settings.scratch_judge_window_rate_perfect_great,
            400
        );
        assert_eq!(pc.judge_settings.scratch_judge_window_rate_great, 400);
        assert_eq!(pc.judge_settings.scratch_judge_window_rate_good, 100);
    }

    #[test]
    fn test_player_config_default_target_list_not_empty() {
        let pc = PlayerConfig::default();
        assert!(!pc.select_settings.targetlist.is_empty());
        assert!(pc.select_settings.targetlist.contains(&"MAX".to_string()));
    }

    #[test]
    fn test_player_config_default_autosave_replay() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.misc_settings.autosavereplay.len(), 4);
        assert!(pc.misc_settings.autosavereplay.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_player_config_serde_roundtrip() {
        let mut pc = PlayerConfig::default();
        pc.id = Some("player1".to_string());
        pc.name = "TestPlayer".to_string();
        pc.play_settings.gauge = 3;
        pc.play_settings.random = 2;
        pc.judge_settings.judgetiming = 50;

        let json = serde_json::to_string(&pc).unwrap();
        let deserialized: PlayerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, Some("player1".to_string()));
        assert_eq!(deserialized.name, "TestPlayer");
        assert_eq!(deserialized.play_settings.gauge, 3);
        assert_eq!(deserialized.play_settings.random, 2);
        assert_eq!(deserialized.judge_settings.judgetiming, 50);
    }

    #[test]
    fn test_player_config_deserialize_empty_json_uses_defaults() {
        let pc: PlayerConfig = serde_json::from_str("{}").unwrap();
        let default = PlayerConfig::default();
        assert_eq!(pc.name, default.name);
        assert_eq!(pc.play_settings.gauge, default.play_settings.gauge);
        assert_eq!(
            pc.judge_settings.judgetiming,
            default.judge_settings.judgetiming
        );
    }

    #[test]
    fn test_player_config_gauge_getter() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.gauge = 5;
        assert_eq!(pc.play_settings.gauge, 5);
    }

    #[test]
    fn test_player_config_random_getter_setter() {
        let mut pc = PlayerConfig::default();

        pc.play_settings.random = 3;
        assert_eq!(pc.play_settings.random, 3);

        pc.play_settings.random2 = 5;
        assert_eq!(pc.play_settings.random2, 5);
    }

    #[test]
    fn test_player_config_doubleoption_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.doubleoption = 2;
        assert_eq!(pc.play_settings.doubleoption, 2);
    }

    #[test]
    fn test_player_config_judgetiming_getter() {
        let mut pc = PlayerConfig::default();
        pc.judge_settings.judgetiming = -100;
        assert_eq!(pc.judge_settings.judgetiming, -100);
    }

    #[test]
    fn test_player_config_lnmode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.lnmode = 2;
        assert_eq!(pc.play_settings.lnmode, 2);
    }

    #[test]
    fn test_player_config_sort_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.select_settings.sort = 3;
        assert_eq!(pc.select_settings.sort, 3);
    }

    #[test]
    fn test_player_config_sortid_getter_setter() {
        let mut pc = PlayerConfig::default();
        assert!(pc.select_settings.sortid.is_none());

        pc.select_settings.sortid = Some("CLEAR".to_string());
        assert_eq!(pc.select_settings.sortid.as_deref(), Some("CLEAR"));
    }

    #[test]
    fn test_player_config_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        assert!(pc.mode().is_none());

        pc.mode = Some(Mode::BEAT_7K);
        assert_eq!(pc.mode(), Some(&Mode::BEAT_7K));

        pc.mode = None;
        assert!(pc.mode().is_none());
    }

    #[test]
    fn test_player_config_boolean_getters() {
        let mut pc = PlayerConfig::default();
        assert!(!pc.select_settings.event_mode);
        assert!(!pc.select_settings.is_random_select);
        assert!(!pc.judge_settings.custom_judge);
        assert!(!pc.display_settings.showjudgearea);
        assert!(!pc.display_settings.markprocessednote);
        assert!(!pc.display_settings.bpmguide);

        pc.select_settings.event_mode = true;
        assert!(pc.select_settings.event_mode);

        pc.judge_settings.custom_judge = true;
        assert!(pc.judge_settings.custom_judge);

        pc.display_settings.showjudgearea = true;
        assert!(pc.display_settings.showjudgearea);

        pc.display_settings.markprocessednote = true;
        assert!(pc.display_settings.markprocessednote);

        pc.display_settings.bpmguide = true;
        assert!(pc.display_settings.bpmguide);
    }

    #[test]
    fn test_player_config_scroll_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.display_settings.scroll_mode = 2;
        assert_eq!(pc.display_settings.scroll_mode, 2);
    }

    #[test]
    fn test_player_config_longnote_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.note_modifier_settings.longnote_mode = 1;
        assert_eq!(pc.note_modifier_settings.longnote_mode, 1);
    }

    #[test]
    fn test_player_config_mine_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.play_settings.mine_mode = 1;
        assert_eq!(pc.play_settings.mine_mode, 1);
    }

    #[test]
    fn test_player_config_misslayer_duration() {
        let mut pc = PlayerConfig::default();
        assert_eq!(pc.get_misslayer_duration(), 500);

        // Test negative clamping
        pc.display_settings.misslayer_duration = -10;
        assert_eq!(pc.get_misslayer_duration(), 0);
        // After the call, the field is clamped to 0
        assert_eq!(pc.display_settings.misslayer_duration, 0);
    }

    #[test]
    fn test_player_config_chart_replication_mode() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.play_settings.chart_replication_mode, "RIVALCHART");
    }

    #[test]
    fn test_player_config_gauge_auto_shift() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.play_settings.gauge_auto_shift, GAUGEAUTOSHIFT_NONE);
    }

    #[test]
    fn test_player_config_targetid() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.select_settings.targetid, "MAX");
    }

    #[test]
    fn test_player_config_musicselectinput() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.select_settings.musicselectinput, 0);
    }

    #[test]
    fn test_player_config_twitter_fields_none_by_default() {
        let pc = PlayerConfig::default();
        assert!(pc.twitter_consumer_key().is_none());
        assert!(pc.twitter_consumer_secret().is_none());
        assert!(pc.twitter_access_token().is_none());
        assert!(pc.twitter_access_token_secret().is_none());
    }

    #[test]
    fn test_player_config_skin_history() {
        let mut pc = PlayerConfig::default();
        assert!(pc.skin_history.is_empty());

        let history = vec![crate::core::skin_config::SkinConfig::default_for_id(0)];
        pc.skin_history = history.clone();
        assert_eq!(pc.skin_history.len(), 1);
    }

    #[test]
    fn test_player_config_get_play_config() {
        let mut pc = PlayerConfig::default();

        // Test different modes return different PlayModeConfig references
        let _ = pc.play_config(Mode::BEAT_5K);
        let _ = pc.play_config(Mode::BEAT_7K);
        let _ = pc.play_config(Mode::POPN_9K);
        let _ = pc.play_config(Mode::KEYBOARD_24K);
    }

    #[test]
    fn test_player_config_get_play_config_by_id() {
        let mut pc = PlayerConfig::default();

        let _ = pc.play_config_by_id(5);
        let _ = pc.play_config_by_id(7);
        let _ = pc.play_config_by_id(9);
        let _ = pc.play_config_by_id(25);
        // Invalid mode defaults to mode7
        let _ = pc.play_config_by_id(999);
    }

    #[test]
    fn test_gaugeautoshift_constants() {
        assert_eq!(GAUGEAUTOSHIFT_NONE, 0);
        assert_eq!(GAUGEAUTOSHIFT_CONTINUE, 1);
        assert_eq!(GAUGEAUTOSHIFT_SURVIVAL_TO_GROOVE, 2);
        assert_eq!(GAUGEAUTOSHIFT_BESTCLEAR, 3);
        assert_eq!(GAUGEAUTOSHIFT_SELECT_TO_UNDER, 4);
    }

    #[test]
    fn test_judgetiming_bounds() {
        assert_eq!(JUDGETIMING_MAX, 500);
        assert_eq!(JUDGETIMING_MIN, -500);
    }

    #[test]
    fn test_player_config_musicselectinput_clamped_on_validate() {
        let mut pc = PlayerConfig::default();
        pc.select_settings.musicselectinput = 99;
        pc.validate();
        assert_eq!(pc.select_settings.musicselectinput, 2);

        pc.select_settings.musicselectinput = -5;
        pc.validate();
        assert_eq!(pc.select_settings.musicselectinput, 0);
    }

    /// scroll_mode valid range is 0..=2 (Off=0, Variable=1, Fixed=2).
    /// Value 3 is out of range and must be clamped down by validate().
    /// Regression test for off-by-one: clamp upper bound was values().len()
    /// (3) instead of values().len() - 1 (2).
    #[test]
    fn test_scroll_mode_clamped_to_valid_range() {
        let mut pc = PlayerConfig::default();

        // Value 2 (Fixed) is the maximum valid value
        pc.display_settings.scroll_mode = 2;
        pc.validate();
        assert_eq!(pc.display_settings.scroll_mode, 2);

        // Value 3 is out of range and must be clamped to 2
        pc.display_settings.scroll_mode = 3;
        pc.validate();
        assert_eq!(
            pc.display_settings.scroll_mode, 2,
            "scroll_mode=3 should be clamped to 2 (max valid index)"
        );

        // Value 100 must also be clamped to 2
        pc.display_settings.scroll_mode = 100;
        pc.validate();
        assert_eq!(pc.display_settings.scroll_mode, 2);

        // Negative values clamped to 0
        pc.display_settings.scroll_mode = -1;
        pc.validate();
        assert_eq!(pc.display_settings.scroll_mode, 0);
    }

    #[test]
    fn test_player_config_corrupt_json_falls_back_to_legacy() {
        let tmp = tempfile::tempdir().unwrap();
        let player_dir = tmp.path().join("player1");
        std::fs::create_dir_all(&player_dir).unwrap();
        // Write corrupt config_player.json
        std::fs::write(player_dir.join("config_player.json"), "{corrupt").unwrap();
        // Write valid legacy config.json
        let legacy = PlayerConfig::default();
        let mut legacy_mod = legacy.clone();
        legacy_mod.play_settings.gauge = 3;
        std::fs::write(
            player_dir.join("config.json"),
            serde_json::to_string(&legacy_mod).unwrap(),
        )
        .unwrap();
        let result = PlayerConfig::read_player_config(tmp.path().to_str().unwrap(), "player1");
        let pc = result.unwrap();
        assert_eq!(pc.play_settings.gauge, 3);
    }
}
