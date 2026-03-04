pub use rubato_types::player_config::*;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use bms_model::mode::Mode;

    #[test]
    fn test_player_config_default_construction() {
        let pc = PlayerConfig::default();
        assert!(pc.id.is_none());
        assert_eq!(pc.name, "NO NAME");
        assert_eq!(pc.gauge, 0);
        assert_eq!(pc.random, 0);
        assert_eq!(pc.random2, 0);
        assert_eq!(pc.doubleoption, 0);
        assert_eq!(pc.chart_replication_mode, "RIVALCHART");
        assert_eq!(pc.targetid, "MAX");
        assert_eq!(pc.judgetiming, 0);
        assert!(!pc.notes_display_timing_auto_adjust);
        assert!(pc.mode.is_none());
        assert_eq!(pc.lnmode, 0);
        assert!(!pc.forcedcnendings);
        assert!(!pc.custom_judge);
        assert!(!pc.bpmguide);
        assert!(!pc.showjudgearea);
        assert!(!pc.markprocessednote);
    }

    #[test]
    fn test_player_config_default_judge_window_rates() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.key_judge_window_rate_perfect_great, 400);
        assert_eq!(pc.key_judge_window_rate_great, 400);
        assert_eq!(pc.key_judge_window_rate_good, 100);
        assert_eq!(pc.scratch_judge_window_rate_perfect_great, 400);
        assert_eq!(pc.scratch_judge_window_rate_great, 400);
        assert_eq!(pc.scratch_judge_window_rate_good, 100);
    }

    #[test]
    fn test_player_config_default_target_list_not_empty() {
        let pc = PlayerConfig::default();
        assert!(!pc.targetlist.is_empty());
        assert!(pc.targetlist.contains(&"MAX".to_string()));
    }

    #[test]
    fn test_player_config_default_autosave_replay() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.autosavereplay.len(), 4);
        assert!(pc.autosavereplay.iter().all(|&v| v == 0));
    }

    #[test]
    fn test_player_config_serde_roundtrip() {
        let mut pc = PlayerConfig::default();
        pc.id = Some("player1".to_string());
        pc.name = "TestPlayer".to_string();
        pc.gauge = 3;
        pc.random = 2;
        pc.judgetiming = 50;

        let json = serde_json::to_string(&pc).unwrap();
        let deserialized: PlayerConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.id, Some("player1".to_string()));
        assert_eq!(deserialized.name, "TestPlayer");
        assert_eq!(deserialized.gauge, 3);
        assert_eq!(deserialized.random, 2);
        assert_eq!(deserialized.judgetiming, 50);
    }

    #[test]
    fn test_player_config_deserialize_empty_json_uses_defaults() {
        let pc: PlayerConfig = serde_json::from_str("{}").unwrap();
        let default = PlayerConfig::default();
        assert_eq!(pc.name, default.name);
        assert_eq!(pc.gauge, default.gauge);
        assert_eq!(pc.judgetiming, default.judgetiming);
    }

    #[test]
    fn test_player_config_gauge_getter() {
        let mut pc = PlayerConfig::default();
        pc.gauge = 5;
        assert_eq!(pc.get_gauge(), 5);
    }

    #[test]
    fn test_player_config_random_getter_setter() {
        let mut pc = PlayerConfig::default();

        pc.set_random(3);
        assert_eq!(pc.get_random(), 3);

        pc.set_random2(5);
        assert_eq!(pc.get_random2(), 5);
    }

    #[test]
    fn test_player_config_doubleoption_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_doubleoption(2);
        assert_eq!(pc.get_doubleoption(), 2);
    }

    #[test]
    fn test_player_config_judgetiming_getter() {
        let mut pc = PlayerConfig::default();
        pc.judgetiming = -100;
        assert_eq!(pc.get_judgetiming(), -100);
    }

    #[test]
    fn test_player_config_lnmode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_lnmode(2);
        assert_eq!(pc.get_lnmode(), 2);
    }

    #[test]
    fn test_player_config_sort_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_sort(3);
        assert_eq!(pc.get_sort(), 3);
    }

    #[test]
    fn test_player_config_sortid_getter_setter() {
        let mut pc = PlayerConfig::default();
        assert!(pc.get_sortid().is_none());

        pc.set_sortid("CLEAR".to_string());
        assert_eq!(pc.get_sortid(), Some("CLEAR"));
    }

    #[test]
    fn test_player_config_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        assert!(pc.get_mode().is_none());

        pc.set_mode(Some(Mode::BEAT_7K));
        assert_eq!(pc.get_mode(), Some(&Mode::BEAT_7K));

        pc.set_mode(None);
        assert!(pc.get_mode().is_none());
    }

    #[test]
    fn test_player_config_boolean_getters() {
        let mut pc = PlayerConfig::default();
        assert!(!pc.is_event_mode());
        assert!(!pc.is_random_select());
        assert!(!pc.is_custom_judge());
        assert!(!pc.is_showjudgearea());
        assert!(!pc.is_markprocessednote());
        assert!(!pc.is_bpmguide());

        pc.event_mode = true;
        assert!(pc.is_event_mode());

        pc.set_custom_judge(true);
        assert!(pc.is_custom_judge());

        pc.set_showjudgearea(true);
        assert!(pc.is_showjudgearea());

        pc.set_markprocessednote(true);
        assert!(pc.is_markprocessednote());

        pc.set_bpmguide(true);
        assert!(pc.is_bpmguide());
    }

    #[test]
    fn test_player_config_scroll_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_scroll_mode(2);
        assert_eq!(pc.get_scroll_mode(), 2);
    }

    #[test]
    fn test_player_config_longnote_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_longnote_mode(1);
        assert_eq!(pc.get_longnote_mode(), 1);
    }

    #[test]
    fn test_player_config_mine_mode_getter_setter() {
        let mut pc = PlayerConfig::default();
        pc.set_mine_mode(1);
        assert_eq!(pc.get_mine_mode(), 1);
    }

    #[test]
    fn test_player_config_misslayer_duration() {
        let mut pc = PlayerConfig::default();
        assert_eq!(pc.get_misslayer_duration(), 500);

        // Test negative clamping
        pc.misslayer_duration = -10;
        assert_eq!(pc.get_misslayer_duration(), 0);
        // After the call, the field is clamped to 0
        assert_eq!(pc.misslayer_duration, 0);
    }

    #[test]
    fn test_player_config_chart_replication_mode() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.get_chart_replication_mode(), "RIVALCHART");
    }

    #[test]
    fn test_player_config_gauge_auto_shift() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.get_gauge_auto_shift(), GAUGEAUTOSHIFT_NONE);
    }

    #[test]
    fn test_player_config_targetid() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.get_targetid(), "MAX");
    }

    #[test]
    fn test_player_config_musicselectinput() {
        let pc = PlayerConfig::default();
        assert_eq!(pc.get_musicselectinput(), 0);
    }

    #[test]
    fn test_player_config_twitter_fields_none_by_default() {
        let pc = PlayerConfig::default();
        assert!(pc.get_twitter_consumer_key().is_none());
        assert!(pc.get_twitter_consumer_secret().is_none());
        assert!(pc.get_twitter_access_token().is_none());
        assert!(pc.get_twitter_access_token_secret().is_none());
    }

    #[test]
    fn test_player_config_skin_history() {
        let mut pc = PlayerConfig::default();
        assert!(pc.get_skin_history().is_empty());

        let history = vec![crate::skin_config::SkinConfig::get_default(0)];
        pc.set_skin_history(history.clone());
        assert_eq!(pc.get_skin_history().len(), 1);
    }

    #[test]
    fn test_player_config_get_play_config() {
        let mut pc = PlayerConfig::default();

        // Test different modes return different PlayModeConfig references
        let _ = pc.get_play_config(Mode::BEAT_5K);
        let _ = pc.get_play_config(Mode::BEAT_7K);
        let _ = pc.get_play_config(Mode::POPN_9K);
        let _ = pc.get_play_config(Mode::KEYBOARD_24K);
    }

    #[test]
    fn test_player_config_get_play_config_by_id() {
        let mut pc = PlayerConfig::default();

        let _ = pc.get_play_config_by_id(5);
        let _ = pc.get_play_config_by_id(7);
        let _ = pc.get_play_config_by_id(9);
        let _ = pc.get_play_config_by_id(25);
        // Invalid mode defaults to mode7
        let _ = pc.get_play_config_by_id(999);
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
}
