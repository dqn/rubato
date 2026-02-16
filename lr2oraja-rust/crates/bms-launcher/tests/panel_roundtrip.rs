// Roundtrip tests: load(config) -> apply(config_out) -> assert fields match.
// Verifies each LauncherPanel preserves configuration values through load/apply.

use bms_config::{
    Config, DisplayMode, DriverType, FrequencyType, IRConfig, PlayerConfig, Resolution, SongPreview,
};
use bms_launcher::panels::{
    audio::AudioPanel, discord::DiscordPanel, input::InputPanel, ir::IrPanel,
    music_select::MusicSelectPanel, obs::ObsPanel, play_option::PlayOptionPanel,
    resource::ResourcePanel, skin::SkinPanel, stream::StreamPanel, video::VideoPanel,
};

// Re-export the trait so load/apply are available.
// The trait is pub but inside the crate; the panels mod is pub, so we can
// access it through the crate's public API.
use bms_launcher::panel::LauncherPanel;

// ---------- helpers ----------

/// Create a validated PlayerConfig (ensures skin array is properly sized, etc.)
fn validated_player_config(mut pc: PlayerConfig) -> PlayerConfig {
    pc.validate();
    pc
}

// ---------- AudioPanel ----------

#[test]
fn audio_panel_roundtrip() {
    let mut config = Config::default();
    config.audio.driver = DriverType::PortAudio;
    config.audio.device_buffer_size = 512;
    config.audio.device_simultaneous_sources = 256;
    config.audio.sample_rate = 48000;
    config.audio.freq_option = FrequencyType::Unprocessed;
    config.audio.fast_forward = FrequencyType::Unprocessed;
    config.audio.systemvolume = 0.8;
    config.audio.keyvolume = 0.3;
    config.audio.bgvolume = 0.6;
    config.audio.normalize_volume = true;
    config.audio.is_loop_result_sound = true;
    config.audio.is_loop_course_result_sound = true;

    let player_config = PlayerConfig::default();
    let mut panel = AudioPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(config_out.audio.driver, DriverType::PortAudio);
    assert_eq!(config_out.audio.device_buffer_size, 512);
    assert_eq!(config_out.audio.device_simultaneous_sources, 256);
    assert_eq!(config_out.audio.sample_rate, 48000);
    assert_eq!(config_out.audio.freq_option, FrequencyType::Unprocessed);
    assert_eq!(config_out.audio.fast_forward, FrequencyType::Unprocessed);
    assert!((config_out.audio.systemvolume - 0.8).abs() < f32::EPSILON);
    assert!((config_out.audio.keyvolume - 0.3).abs() < f32::EPSILON);
    assert!((config_out.audio.bgvolume - 0.6).abs() < f32::EPSILON);
    assert!(config_out.audio.normalize_volume);
    assert!(config_out.audio.is_loop_result_sound);
    assert!(config_out.audio.is_loop_course_result_sound);
}

// ---------- VideoPanel ----------

#[test]
fn video_panel_roundtrip() {
    let mut config = Config::default();
    config.displaymode = DisplayMode::Fullscreen;
    config.resolution = Resolution::Fullhd;
    config.use_resolution = false;
    config.window_width = 1920;
    config.window_height = 1080;
    config.vsync = true;
    config.bga = 2;
    config.bga_expand = 2;
    config.max_frame_per_second = 120;
    config.frameskip = 3;
    config.monitor_name = "DELL U2723QE [0, 0]".to_string();

    let mut player_config = PlayerConfig::default();
    player_config.misslayer_duration = 1500;

    let mut panel = VideoPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(config_out.displaymode, DisplayMode::Fullscreen);
    assert_eq!(config_out.resolution, Resolution::Fullhd);
    assert!(!config_out.use_resolution);
    assert_eq!(config_out.window_width, 1920);
    assert_eq!(config_out.window_height, 1080);
    assert!(config_out.vsync);
    assert_eq!(config_out.bga, 2);
    assert_eq!(config_out.bga_expand, 2);
    assert_eq!(config_out.max_frame_per_second, 120);
    assert_eq!(config_out.frameskip, 3);
    assert_eq!(config_out.monitor_name, "DELL U2723QE [0, 0]");
    assert_eq!(player_config_out.misslayer_duration, 1500);
}

// ---------- InputPanel ----------

#[test]
fn input_panel_roundtrip() {
    let mut player_config = validated_player_config(PlayerConfig::default());

    // Set non-default values for mode7 (index 1 in PLAY_MODE_LABELS)
    let mode7 = &mut player_config.mode7;
    mode7.keyboard.duration = 32;
    mode7.keyboard.mouse_scratch_config.mouse_scratch_enabled = true;
    mode7
        .keyboard
        .mouse_scratch_config
        .mouse_scratch_time_threshold = 200;
    mode7.keyboard.mouse_scratch_config.mouse_scratch_distance = 20;
    mode7.keyboard.mouse_scratch_config.mouse_scratch_mode = 1;

    if let Some(ctrl) = mode7.controller.first_mut() {
        ctrl.jkoc_hack = true;
        ctrl.analog_scratch = true;
        ctrl.analog_scratch_threshold = 75;
        ctrl.analog_scratch_mode = 1;
    }

    player_config.musicselectinput = 1;

    let config = Config::default();
    let mut panel = InputPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = validated_player_config(PlayerConfig::default());
    // Ensure output has at least one controller for mode7
    panel.apply(&mut config_out, &mut player_config_out);

    let mode7_out = &player_config_out.mode7;
    assert_eq!(mode7_out.keyboard.duration, 32);

    let ms = &mode7_out.keyboard.mouse_scratch_config;
    assert!(ms.mouse_scratch_enabled);
    assert_eq!(ms.mouse_scratch_time_threshold, 200);
    assert_eq!(ms.mouse_scratch_distance, 20);
    assert_eq!(ms.mouse_scratch_mode, 1);

    // Controller fields are applied only if the output already has controllers
    if let Some(ctrl) = mode7_out.controller.first() {
        assert!(ctrl.jkoc_hack);
        assert!(ctrl.analog_scratch);
        assert_eq!(ctrl.analog_scratch_threshold, 75);
        assert_eq!(ctrl.analog_scratch_mode, 1);
    }

    assert_eq!(player_config_out.musicselectinput, 1);
}

// ---------- ResourcePanel ----------

#[test]
fn resource_panel_roundtrip() {
    let mut config = Config::default();
    config.bmsroot = vec!["/path/to/bms1".to_string(), "/path/to/bms2".to_string()];
    config.table_url = vec![
        "https://example.com/table1.html".to_string(),
        "https://example.com/table2.html".to_string(),
    ];
    config.bgmpath = "custom/bgm".to_string();
    config.soundpath = "custom/sound".to_string();
    config.skinpath = "custom/skin".to_string();
    config.systemfontpath = "custom/font/system.ttf".to_string();
    config.messagefontpath = "custom/font/message.ttf".to_string();

    let player_config = PlayerConfig::default();
    let mut panel = ResourcePanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(config_out.bmsroot, config.bmsroot);
    assert_eq!(config_out.table_url, config.table_url);
    assert_eq!(config_out.bgmpath, "custom/bgm");
    assert_eq!(config_out.soundpath, "custom/sound");
    assert_eq!(config_out.skinpath, "custom/skin");
    assert_eq!(config_out.systemfontpath, "custom/font/system.ttf");
    assert_eq!(config_out.messagefontpath, "custom/font/message.ttf");
}

// ---------- MusicSelectPanel ----------

#[test]
fn music_select_panel_roundtrip() {
    let mut config = Config::default();
    config.scrolldurationlow = 150;
    config.scrolldurationhigh = 25;
    config.analog_scroll = false;
    config.analog_ticks_per_scroll = 5;
    config.song_preview = SongPreview::None;
    config.max_search_bar_count = 20;
    config.skip_decide_screen = true;
    config.show_no_song_existing_bar = false;
    config.folderlamp = false;

    let player_config = PlayerConfig::default();
    let mut panel = MusicSelectPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(config_out.scrolldurationlow, 150);
    assert_eq!(config_out.scrolldurationhigh, 25);
    assert!(!config_out.analog_scroll);
    assert_eq!(config_out.analog_ticks_per_scroll, 5);
    assert_eq!(config_out.song_preview, SongPreview::None);
    assert_eq!(config_out.max_search_bar_count, 20);
    assert!(config_out.skip_decide_screen);
    assert!(!config_out.show_no_song_existing_bar);
    assert!(!config_out.folderlamp);
}

// ---------- PlayOptionPanel ----------

#[test]
fn play_option_panel_roundtrip_global_fields() {
    let mut player_config = validated_player_config(PlayerConfig::default());
    player_config.gauge = 3;
    player_config.random = 2;
    player_config.random2 = 4;
    player_config.doubleoption = 1;
    player_config.judgetiming = -50;
    player_config.lnmode = 2;
    player_config.scroll_mode = 1;
    player_config.mine_mode = 3;
    player_config.gauge_auto_shift = 2;
    player_config.bottom_shiftable_gauge = 1;
    player_config.notes_display_timing_auto_adjust = true;
    player_config.custom_judge = true;
    player_config.bpmguide = true;
    player_config.showjudgearea = true;
    player_config.markprocessednote = true;
    player_config.showhiddennote = true;
    player_config.showpastnote = true;
    player_config.is_guide_se = true;
    player_config.is_window_hold = true;
    player_config.chart_preview = false;
    player_config.key_judge_window_rate_perfect_great = 200;
    player_config.key_judge_window_rate_great = 150;
    player_config.key_judge_window_rate_good = 50;
    player_config.scratch_judge_window_rate_perfect_great = 250;
    player_config.scratch_judge_window_rate_great = 175;
    player_config.scratch_judge_window_rate_good = 75;
    player_config.hran_threshold_bpm = 200;
    player_config.extranote_depth = 10;
    player_config.longnote_mode = 3;
    player_config.longnote_rate = 0.5;
    player_config.forcedcnendings = true;
    player_config.seven_to_nine_pattern = 2;
    player_config.seven_to_nine_type = 1;
    player_config.exit_press_duration = 2000;
    player_config.targetid = "RATE_AAA".to_string();
    player_config.autosavereplay = Some(vec![1, 3, 5, 7]);

    let config = Config::default();
    let mut panel = PlayOptionPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = validated_player_config(PlayerConfig::default());
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(player_config_out.gauge, 3);
    assert_eq!(player_config_out.random, 2);
    assert_eq!(player_config_out.random2, 4);
    assert_eq!(player_config_out.doubleoption, 1);
    assert_eq!(player_config_out.judgetiming, -50);
    assert_eq!(player_config_out.lnmode, 2);
    assert_eq!(player_config_out.scroll_mode, 1);
    assert_eq!(player_config_out.mine_mode, 3);
    assert_eq!(player_config_out.gauge_auto_shift, 2);
    assert_eq!(player_config_out.bottom_shiftable_gauge, 1);
    assert!(player_config_out.notes_display_timing_auto_adjust);
    assert!(player_config_out.custom_judge);
    assert!(player_config_out.bpmguide);
    assert!(player_config_out.showjudgearea);
    assert!(player_config_out.markprocessednote);
    assert!(player_config_out.showhiddennote);
    assert!(player_config_out.showpastnote);
    assert!(player_config_out.is_guide_se);
    assert!(player_config_out.is_window_hold);
    assert!(!player_config_out.chart_preview);
    assert_eq!(player_config_out.key_judge_window_rate_perfect_great, 200);
    assert_eq!(player_config_out.key_judge_window_rate_great, 150);
    assert_eq!(player_config_out.key_judge_window_rate_good, 50);
    assert_eq!(
        player_config_out.scratch_judge_window_rate_perfect_great,
        250
    );
    assert_eq!(player_config_out.scratch_judge_window_rate_great, 175);
    assert_eq!(player_config_out.scratch_judge_window_rate_good, 75);
    assert_eq!(player_config_out.hran_threshold_bpm, 200);
    assert_eq!(player_config_out.extranote_depth, 10);
    assert_eq!(player_config_out.longnote_mode, 3);
    assert!((player_config_out.longnote_rate - 0.5).abs() < f64::EPSILON);
    assert!(player_config_out.forcedcnendings);
    assert_eq!(player_config_out.seven_to_nine_pattern, 2);
    assert_eq!(player_config_out.seven_to_nine_type, 1);
    assert_eq!(player_config_out.exit_press_duration, 2000);
    assert_eq!(player_config_out.targetid, "RATE_AAA");
    assert_eq!(player_config_out.autosavereplay, Some(vec![1, 3, 5, 7]));
}

#[test]
fn play_option_panel_roundtrip_per_mode_fields() {
    // Test per-mode PlayConfig fields through mode7
    let mut player_config = validated_player_config(PlayerConfig::default());
    let pc = &mut player_config.mode7.playconfig;
    pc.hispeed = 5.5;
    pc.duration = 800;
    pc.fixhispeed = 2; // Max BPM
    pc.hispeedmargin = 1.5;
    pc.hispeedautoadjust = true;
    pc.enablelanecover = false;
    pc.lanecover = 0.35;
    pc.lanecovermarginlow = 0.005;
    pc.lanecovermarginhigh = 0.05;
    pc.lanecoverswitchduration = 1000;
    pc.enablelift = true;
    pc.lift = 0.25;
    pc.enablehidden = true;
    pc.hidden = 0.15;
    pc.enable_constant = true;
    pc.constant_fadein_time = 200;
    pc.judgetype = "Duration".to_string();

    let config = Config::default();
    let mut panel = PlayOptionPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = validated_player_config(PlayerConfig::default());
    panel.apply(&mut config_out, &mut player_config_out);

    let pc_out = &player_config_out.mode7.playconfig;
    assert!((pc_out.hispeed - 5.5).abs() < 0.01);
    assert_eq!(pc_out.duration, 800);
    assert_eq!(pc_out.fixhispeed, 2);
    assert!((pc_out.hispeedmargin - 1.5).abs() < 0.01);
    assert!(pc_out.hispeedautoadjust);
    assert!(!pc_out.enablelanecover);
    // Lanecover goes through i32 permille conversion: 0.35 -> 350 -> 0.35
    assert!((pc_out.lanecover - 0.35).abs() < 0.002);
    assert!((pc_out.lanecovermarginlow - 0.005).abs() < 0.002);
    assert!((pc_out.lanecovermarginhigh - 0.05).abs() < 0.002);
    assert_eq!(pc_out.lanecoverswitchduration, 1000);
    assert!(pc_out.enablelift);
    assert!((pc_out.lift - 0.25).abs() < 0.002);
    assert!(pc_out.enablehidden);
    assert!((pc_out.hidden - 0.15).abs() < 0.002);
    assert!(pc_out.enable_constant);
    assert_eq!(pc_out.constant_fadein_time, 200);
    assert_eq!(pc_out.judgetype, "Duration");
}

// ---------- SkinPanel ----------

#[test]
fn skin_panel_roundtrip() {
    let mut player_config = validated_player_config(PlayerConfig::default());

    // Set custom skin paths for a few types
    player_config.skin[0].path = Some("custom/play7.luaskin".to_string());
    player_config.skin[1].path = Some("custom/play5.json".to_string());
    player_config.skin[5].path = Some("custom/select.json".to_string());

    let config = Config::default();
    let mut panel = SkinPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = validated_player_config(PlayerConfig::default());
    panel.apply(&mut config_out, &mut player_config_out);

    assert_eq!(
        player_config_out.skin[0].path.as_deref(),
        Some("custom/play7.luaskin")
    );
    assert_eq!(
        player_config_out.skin[1].path.as_deref(),
        Some("custom/play5.json")
    );
    assert_eq!(
        player_config_out.skin[5].path.as_deref(),
        Some("custom/select.json")
    );
}

// ---------- IrPanel ----------

#[test]
fn ir_panel_roundtrip() {
    let mut player_config = PlayerConfig::default();
    player_config.irconfig = Some(vec![
        IRConfig {
            irname: "LR2IR".to_string(),
            userid: "testuser".to_string(),
            password: "testpass".to_string(),
            irsend: 1,
            importscore: true,
            importrival: false,
            ..Default::default()
        },
        IRConfig {
            irname: "BeatorajaIR".to_string(),
            userid: "user2".to_string(),
            password: "pass2".to_string(),
            irsend: 2,
            importscore: false,
            importrival: true,
            ..Default::default()
        },
    ]);

    let config = Config::default();
    let mut panel = IrPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    let configs = player_config_out.irconfig.as_ref().unwrap();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[0].irname, "LR2IR");
    assert_eq!(configs[0].userid, "testuser");
    assert_eq!(configs[0].password, "testpass");
    assert_eq!(configs[0].irsend, 1);
    assert!(configs[0].importscore);
    assert!(!configs[0].importrival);
    assert_eq!(configs[1].irname, "BeatorajaIR");
    assert_eq!(configs[1].userid, "user2");
    assert_eq!(configs[1].irsend, 2);
}

// ---------- DiscordPanel ----------

#[test]
fn discord_panel_roundtrip() {
    let mut config = Config::default();
    config.use_discord_rpc = true;
    config.webhook_option = 2;
    config.webhook_name = "TestBot".to_string();
    config.webhook_avatar = "https://example.com/avatar.png".to_string();
    config.webhook_url = vec![
        "https://discord.com/api/webhooks/123/abc".to_string(),
        "https://discord.com/api/webhooks/456/def".to_string(),
    ];

    let player_config = PlayerConfig::default();
    let mut panel = DiscordPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert!(config_out.use_discord_rpc);
    assert_eq!(config_out.webhook_option, 2);
    assert_eq!(config_out.webhook_name, "TestBot");
    assert_eq!(config_out.webhook_avatar, "https://example.com/avatar.png");
    assert_eq!(config_out.webhook_url.len(), 2);
    assert_eq!(
        config_out.webhook_url[0],
        "https://discord.com/api/webhooks/123/abc"
    );
}

// ---------- ObsPanel ----------

#[test]
fn obs_panel_roundtrip() {
    let mut config = Config::default();
    config.use_obs_ws = true;
    config.obs_ws_host = "192.168.1.100".to_string();
    config.obs_ws_port = 4444;
    config.obs_ws_pass = "secretpass".to_string();
    config.obs_ws_rec_mode = 2;
    config.obs_ws_rec_stop_wait = 10000;

    let player_config = PlayerConfig::default();
    let mut panel = ObsPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert!(config_out.use_obs_ws);
    assert_eq!(config_out.obs_ws_host, "192.168.1.100");
    assert_eq!(config_out.obs_ws_port, 4444);
    assert_eq!(config_out.obs_ws_pass, "secretpass");
    assert_eq!(config_out.obs_ws_rec_mode, 2);
    assert_eq!(config_out.obs_ws_rec_stop_wait, 10000);
}

// ---------- StreamPanel ----------

#[test]
fn stream_panel_roundtrip() {
    let mut player_config = PlayerConfig::default();
    player_config.enable_request = true;
    player_config.notify_request = true;
    player_config.max_request_count = 50;

    let config = Config::default();
    let mut panel = StreamPanel::default();
    panel.load(&config, &player_config);

    let mut config_out = Config::default();
    let mut player_config_out = PlayerConfig::default();
    panel.apply(&mut config_out, &mut player_config_out);

    assert!(player_config_out.enable_request);
    assert!(player_config_out.notify_request);
    assert_eq!(player_config_out.max_request_count, 50);
}

// ---------- TableEditorPanel ----------
// TableEditorPanel.apply() is a no-op (saves to table files independently).
// No roundtrip through Config/PlayerConfig to test. This is documented here
// for completeness.
