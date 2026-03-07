use super::*;
use crate::stubs::{MainController, PlayerResource, SkinOffset, TextureRegion, Timer};

/// Test implementation of MainState that provides mutable config access
struct TestMainState {
    timer: Timer,
    main: MainController,
    resource: PlayerResource,
    player_config: rubato_types::player_config::PlayerConfig,
    config: rubato_types::config::Config,
    play_config: rubato_types::play_config::PlayConfig,
    is_selector: bool,
    option_change_played: bool,
    bar_updated: bool,
    executed_events: Vec<(i32, i32, i32)>,
    changed_state: Option<MainStateType>,
    selected_song: Option<BMSPlayerMode>,
}

impl TestMainState {
    fn new() -> Self {
        Self {
            timer: Timer::default(),
            main: MainController { debug: false },
            resource: PlayerResource,
            player_config: rubato_types::player_config::PlayerConfig::default(),
            config: rubato_types::config::Config::default(),
            play_config: rubato_types::play_config::PlayConfig::default(),
            is_selector: true,
            option_change_played: false,
            bar_updated: false,
            executed_events: Vec::new(),
            changed_state: None,
            selected_song: None,
        }
    }
}

impl MainState for TestMainState {
    fn timer(&self) -> &dyn rubato_types::timer_access::TimerAccess {
        &self.timer
    }
    fn get_offset_value(&self, _id: i32) -> Option<&SkinOffset> {
        None
    }
    fn get_main(&self) -> &MainController {
        &self.main
    }
    fn get_image(&self, _id: i32) -> Option<TextureRegion> {
        None
    }
    fn get_resource(&self) -> &PlayerResource {
        &self.resource
    }

    fn is_music_selector(&self) -> bool {
        self.is_selector
    }

    fn player_config_mut(&mut self) -> Option<&mut rubato_types::player_config::PlayerConfig> {
        Some(&mut self.player_config)
    }

    fn get_player_config_ref(&self) -> Option<&rubato_types::player_config::PlayerConfig> {
        Some(&self.player_config)
    }

    fn get_config_mut(&mut self) -> Option<&mut rubato_types::config::Config> {
        Some(&mut self.config)
    }

    fn get_config_ref(&self) -> Option<&rubato_types::config::Config> {
        Some(&self.config)
    }

    fn get_selected_play_config_mut(
        &mut self,
    ) -> Option<&mut rubato_types::play_config::PlayConfig> {
        Some(&mut self.play_config)
    }

    fn get_selected_play_config_ref(&self) -> Option<&rubato_types::play_config::PlayConfig> {
        Some(&self.play_config)
    }

    fn play_option_change_sound(&mut self) {
        self.option_change_played = true;
    }

    fn update_bar_after_change(&mut self) {
        self.bar_updated = true;
    }

    fn execute_event(&mut self, id: i32, arg1: i32, arg2: i32) {
        self.executed_events.push((id, arg1, arg2));
    }

    fn change_state(&mut self, state_type: MainStateType) {
        self.changed_state = Some(state_type);
    }

    fn select_song(&mut self, mode: BMSPlayerMode) {
        self.selected_song = Some(mode);
    }
}

#[test]
fn test_get_event_by_id_known() {
    let event = event_by_id(11).unwrap();
    assert_eq!(event.get_event_id(), EventId(11));
}

#[test]
fn test_get_event_by_id_unknown() {
    let event = event_by_id(9999).unwrap();
    assert_eq!(event.get_event_id(), EventId(9999));
}

#[test]
fn test_get_event_by_name() {
    let event = event_by_name("mode").unwrap();
    assert_eq!(event.get_event_id(), EventId(11));
}

#[test]
fn test_get_event_by_name_unknown() {
    assert!(event_by_name("nonexistent").is_none());
}

#[test]
fn test_gauge_cycle_forward() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.gauge = 3;
    let event = event_by_id(40).unwrap(); // gauge1p
    event.exec(&mut state, 1, 0); // forward
    assert_eq!(state.player_config.play_settings.gauge, 4);
    assert!(state.option_change_played);
}

#[test]
fn test_gauge_cycle_backward() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.gauge = 0;
    let event = event_by_id(40).unwrap(); // gauge1p
    event.exec(&mut state, -1, 0); // backward wraps to 5
    assert_eq!(state.player_config.play_settings.gauge, 5);
}

#[test]
fn test_option1p_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.random = 9;
    let event = event_by_id(42).unwrap(); // option1p
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.random, 0); // wraps
}

#[test]
fn test_option1p_cycle_not_music_selector() {
    let mut state = TestMainState::new();
    state.is_selector = false;
    state.player_config.play_settings.random = 9;
    let event = event_by_id(42).unwrap(); // option1p
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.random, 0);
}

#[test]
fn test_option2p_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.random2 = 5;
    let event = event_by_id(43).unwrap(); // option2p
    event.exec(&mut state, -1, 0);
    assert_eq!(state.player_config.play_settings.random2, 4);
}

#[test]
fn test_hsfix_cycle() {
    let mut state = TestMainState::new();
    state.play_config.fixhispeed = 3;
    let event = event_by_id(55).unwrap(); // hsfix
    event.exec(&mut state, 1, 0);
    assert_eq!(state.play_config.fixhispeed, 4);
}

#[test]
fn test_hispeed_forward() {
    let mut state = TestMainState::new();
    state.play_config.hispeed = 1.0;
    state.play_config.hispeedmargin = 0.25;
    let event = event_by_id(57).unwrap(); // hispeed1p
    event.exec(&mut state, 1, 0);
    assert!((state.play_config.hispeed - 1.25).abs() < 0.001);
    assert!(state.option_change_played);
}

#[test]
fn test_hispeed_backward() {
    let mut state = TestMainState::new();
    state.play_config.hispeed = 1.0;
    state.play_config.hispeedmargin = 0.25;
    let event = event_by_id(57).unwrap();
    event.exec(&mut state, -1, 0);
    assert!((state.play_config.hispeed - 0.75).abs() < 0.001);
}

#[test]
fn test_hispeed_clamp_max() {
    let mut state = TestMainState::new();
    state.play_config.hispeed = play_config::HISPEED_MAX;
    state.play_config.hispeedmargin = 0.25;
    let event = event_by_id(57).unwrap();
    event.exec(&mut state, 1, 0);
    assert!((state.play_config.hispeed - play_config::HISPEED_MAX).abs() < 0.001);
    // No sound since value didn't change
    assert!(!state.option_change_played);
}

#[test]
fn test_duration_forward() {
    let mut state = TestMainState::new();
    state.play_config.duration = 500;
    let event = event_by_id(59).unwrap(); // duration1p
    event.exec(&mut state, 1, 0);
    assert_eq!(state.play_config.duration, 501);
}

#[test]
fn test_duration_with_arg2() {
    let mut state = TestMainState::new();
    state.play_config.duration = 500;
    let event = event_by_id(59).unwrap();
    event.exec(&mut state, 1, 10); // increment by 10
    assert_eq!(state.play_config.duration, 510);
}

#[test]
fn test_hispeed_auto_adjust_toggle() {
    let mut state = TestMainState::new();
    assert!(!state.play_config.hispeedautoadjust);
    let event = event_by_id(342).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(state.play_config.hispeedautoadjust);
    event.exec(&mut state, 0, 0);
    assert!(!state.play_config.hispeedautoadjust);
}

#[test]
fn test_lanecover_toggle() {
    let mut state = TestMainState::new();
    assert!(state.play_config.enablelanecover); // default is true
    let event = event_by_id(330).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(!state.play_config.enablelanecover);
}

#[test]
fn test_lift_toggle() {
    let mut state = TestMainState::new();
    assert!(!state.play_config.enablelift);
    let event = event_by_id(331).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(state.play_config.enablelift);
}

#[test]
fn test_hidden_toggle() {
    let mut state = TestMainState::new();
    assert!(!state.play_config.enablehidden);
    let event = event_by_id(332).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(state.play_config.enablehidden);
}

#[test]
fn test_constant_toggle() {
    let mut state = TestMainState::new();
    assert!(!state.play_config.enable_constant);
    let event = event_by_id(skin_property::OPTION_CONSTANT).unwrap();
    assert_eq!(
        event.get_event_id(),
        EventId(skin_property::OPTION_CONSTANT)
    );
    event.exec(&mut state, 0, 0);
    assert!(state.play_config.enable_constant);
}

#[test]
fn test_bga_cycle() {
    let mut state = TestMainState::new();
    state.config.render.bga = 2;
    let event = event_by_id(72).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.config.render.bga, 0); // wraps from 2
}

#[test]
fn test_bgaexpand_cycle() {
    let mut state = TestMainState::new();
    state.config.render.bga_expand = 0;
    let event = event_by_id(73).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.config.render.bga_expand, 1);
}

#[test]
fn test_notes_display_timing_forward() {
    let mut state = TestMainState::new();
    state.player_config.judge_settings.judgetiming = 0;
    let event = event_by_id(74).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.judge_settings.judgetiming, 1);
}

#[test]
fn test_notes_display_timing_backward() {
    let mut state = TestMainState::new();
    state.player_config.judge_settings.judgetiming = 0;
    let event = event_by_id(74).unwrap();
    event.exec(&mut state, -1, 0);
    assert_eq!(state.player_config.judge_settings.judgetiming, -1);
}

#[test]
fn test_notes_display_timing_clamp_max() {
    let mut state = TestMainState::new();
    state.player_config.judge_settings.judgetiming = rubato_types::player_config::JUDGETIMING_MAX;
    let event = event_by_id(74).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(
        state.player_config.judge_settings.judgetiming,
        rubato_types::player_config::JUDGETIMING_MAX
    );
}

#[test]
fn test_notes_display_timing_auto_adjust() {
    let mut state = TestMainState::new();
    assert!(
        !state
            .player_config
            .judge_settings
            .notes_display_timing_auto_adjust
    );
    let event = event_by_id(75).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(
        state
            .player_config
            .judge_settings
            .notes_display_timing_auto_adjust
    );
}

#[test]
fn test_guide_se_toggle() {
    let mut state = TestMainState::new();
    assert!(!state.player_config.display_settings.is_guide_se);
    let event = event_by_id(343).unwrap();
    event.exec(&mut state, 0, 0);
    assert!(state.player_config.display_settings.is_guide_se);
}

#[test]
fn test_lnmode_noop() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.lnmode = 0;
    let event = event_by_id(308).unwrap();
    event.exec(&mut state, 1, 0);
    // LN mode is disabled; value should not change
    assert_eq!(state.player_config.play_settings.lnmode, 0);
}

#[test]
fn test_autosavereplay_cycle() {
    let mut state = TestMainState::new();
    state.player_config.misc_settings.autosavereplay = vec![5, 0, 0, 0];
    let event = event_by_id(321).unwrap(); // autosavereplay1 (index 0)
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.misc_settings.autosavereplay[0], 6);
}

#[test]
fn test_autosavereplay_wrap() {
    let mut state = TestMainState::new();
    state.player_config.misc_settings.autosavereplay = vec![10, 0, 0, 0];
    let event = event_by_id(321).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.misc_settings.autosavereplay[0], 0); // wraps at 11
}

#[test]
fn test_state_change_keyconfig() {
    let mut state = TestMainState::new();
    let event = event_by_id(13).unwrap(); // keyconfig
    event.exec(&mut state, 0, 0);
    assert_eq!(state.changed_state, Some(MainStateType::Config));
}

#[test]
fn test_state_change_skinconfig() {
    let mut state = TestMainState::new();
    let event = event_by_id(14).unwrap(); // skinconfig
    event.exec(&mut state, 0, 0);
    assert_eq!(state.changed_state, Some(MainStateType::SkinConfig));
}

#[test]
fn test_play_select_song() {
    let mut state = TestMainState::new();
    let event = event_by_id(15).unwrap(); // play
    event.exec(&mut state, 0, 0);
    assert_eq!(state.selected_song, Some(BMSPlayerMode::PLAY));
}

#[test]
fn test_autoplay_select_song() {
    let mut state = TestMainState::new();
    let event = event_by_id(16).unwrap(); // autoplay
    event.exec(&mut state, 0, 0);
    assert_eq!(state.selected_song, Some(BMSPlayerMode::AUTOPLAY));
}

#[test]
fn test_practice_select_song() {
    let mut state = TestMainState::new();
    let event = event_by_id(315).unwrap(); // practice
    event.exec(&mut state, 0, 0);
    assert_eq!(state.selected_song, Some(BMSPlayerMode::PRACTICE));
}

#[test]
fn test_replay_select_song() {
    let mut state = TestMainState::new();
    let event = event_by_id(19).unwrap(); // replay1
    event.exec(&mut state, 0, 0);
    assert_eq!(state.selected_song, Some(BMSPlayerMode::REPLAY_1));
}

#[test]
fn test_mode_cycle_forward() {
    let mut state = TestMainState::new();
    state.player_config.mode = None; // index 0
    let event = event_by_id(11).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(
        state.player_config.mode,
        Some(bms_model::mode::Mode::BEAT_7K)
    );
    assert!(state.bar_updated);
    assert!(state.option_change_played);
}

#[test]
fn test_mode_cycle_backward() {
    let mut state = TestMainState::new();
    state.player_config.mode = None; // index 0
    let event = event_by_id(11).unwrap();
    event.exec(&mut state, -1, 0);
    assert_eq!(
        state.player_config.mode,
        Some(bms_model::mode::Mode::KEYBOARD_24K_DOUBLE)
    );
}

#[test]
fn test_sort_cycle() {
    let mut state = TestMainState::new();
    state.player_config.select_settings.sort = 0;
    let event = event_by_id(12).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.select_settings.sort, 1);
    assert!(state.bar_updated);
    assert!(state.option_change_played);
}

#[test]
fn test_key_assign_noop() {
    let mut state = TestMainState::new();
    let event = event_by_id(101).unwrap(); // keyassign1
    event.exec(&mut state, 0, 0);
    // Should not modify anything
    assert!(!state.option_change_played);
}

#[test]
fn test_key_assign_event_ids() {
    // Verify the ID mapping: 101-139 for indices 0-38, 150-164 for indices 39-53
    let event = event_by_name("keyassign1").unwrap();
    assert_eq!(event.get_event_id(), EventId(101));
    let event = event_by_name("keyassign39").unwrap();
    assert_eq!(event.get_event_id(), EventId(139));
    let event = event_by_name("keyassign40").unwrap();
    assert_eq!(event.get_event_id(), EventId(150));
    let event = event_by_name("keyassign54").unwrap();
    assert_eq!(event.get_event_id(), EventId(164));
}

#[test]
fn test_delegate_event_for_unknown_id() {
    let mut state = TestMainState::new();
    let event = event_by_id(9999).unwrap();
    event.exec(&mut state, 1, 2);
    assert_eq!(state.executed_events, vec![(9999, 1, 2)]);
}

#[test]
fn test_not_music_selector_skips() {
    let mut state = TestMainState::new();
    state.is_selector = false;
    let event = event_by_id(40).unwrap(); // gauge1p
    event.exec(&mut state, 1, 0);
    // Should not modify config since not MusicSelector
    assert_eq!(state.player_config.play_settings.gauge, 0);
    assert!(!state.option_change_played);
}

#[test]
fn test_target_cycle_not_music_selector() {
    let mut state = TestMainState::new();
    state.is_selector = false;
    state.player_config.select_settings.targetid = "RATE_MAX-".to_string();
    let event = event_by_id(77).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.select_settings.targetid, "MAX");
}

#[test]
fn test_notes_display_timing_works_for_any_state() {
    let mut state = TestMainState::new();
    state.is_selector = false;
    state.player_config.judge_settings.judgetiming = 0;
    let event = event_by_id(74).unwrap();
    event.exec(&mut state, 1, 0);
    // notesdisplaytiming works for any state, not just MusicSelector
    assert_eq!(state.player_config.judge_settings.judgetiming, 1);
    // But sound is only played for MusicSelector
    assert!(!state.option_change_played);
}

#[test]
fn test_extranotedepth_cycle() {
    let mut state = TestMainState::new();
    state.player_config.display_settings.extranote_depth = 3;
    let event = event_by_id(350).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.display_settings.extranote_depth, 0); // wraps at 4
}

#[test]
fn test_minemode_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.mine_mode = 4;
    let event = event_by_id(351).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.mine_mode, 0); // wraps at 5
}

#[test]
fn test_scrollmode_cycle() {
    let mut state = TestMainState::new();
    state.player_config.display_settings.scroll_mode = 2;
    let event = event_by_id(352).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.display_settings.scroll_mode, 0); // wraps at 3
}

#[test]
fn test_longnotemode_cycle() {
    let mut state = TestMainState::new();
    state.player_config.note_modifier_settings.longnote_mode = 5;
    let event = event_by_id(353).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.note_modifier_settings.longnote_mode, 0); // wraps at 6
}

#[test]
fn test_seventonine_pattern_cycle() {
    let mut state = TestMainState::new();
    state
        .player_config
        .note_modifier_settings
        .seven_to_nine_pattern = 6;
    let event = event_by_id(360).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(
        state
            .player_config
            .note_modifier_settings
            .seven_to_nine_pattern,
        0
    ); // wraps at 7
}

#[test]
fn test_seventonine_type_cycle() {
    let mut state = TestMainState::new();
    state
        .player_config
        .note_modifier_settings
        .seven_to_nine_type = 2;
    let event = event_by_id(361).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(
        state
            .player_config
            .note_modifier_settings
            .seven_to_nine_type,
        0
    ); // wraps at 3
}

#[test]
fn test_judge_algorithm_cycle() {
    let mut state = TestMainState::new();
    state.play_config.judgetype = "Combo".to_string();
    let event = event_by_id(340).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.play_config.judgetype, "Duration");
}

#[test]
fn test_judge_algorithm_cycle_backward() {
    let mut state = TestMainState::new();
    state.play_config.judgetype = "Combo".to_string();
    let event = event_by_id(340).unwrap();
    event.exec(&mut state, -1, 0);
    assert_eq!(state.play_config.judgetype, "Lowest");
}

#[test]
fn test_optiondp_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.doubleoption = 3;
    let event = event_by_id(54).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.doubleoption, 0); // wraps at 4
}

#[test]
fn test_gaugeautoshift_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.gauge_auto_shift = 4;
    let event = event_by_id(78).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.gauge_auto_shift, 0); // wraps at 5
}

#[test]
fn test_bottomshiftablegauge_cycle() {
    let mut state = TestMainState::new();
    state.player_config.play_settings.bottom_shiftable_gauge = 2;
    let event = event_by_id(341).unwrap();
    event.exec(&mut state, 1, 0);
    assert_eq!(state.player_config.play_settings.bottom_shiftable_gauge, 0); // wraps at 3
}

#[test]
fn test_all_event_types_have_matching_ids() {
    // Verify every EVENT_TYPES entry creates an event with the correct ID
    for et in EVENT_TYPES.iter() {
        let event = (et.create_event)();
        assert_eq!(
            event.get_event_id(),
            et.id,
            "Event '{}' has mismatched ID: expected {}, got {}",
            et.name,
            et.id,
            event.get_event_id()
        );
    }
}

#[test]
fn test_create_helper_functions() {
    let e = create_zero_arg_event(42);
    assert_eq!(e.get_event_id(), EventId(42));
    let e = create_one_arg_event(43);
    assert_eq!(e.get_event_id(), EventId(43));
    let e = create_two_arg_event(44);
    assert_eq!(e.get_event_id(), EventId(44));
}
