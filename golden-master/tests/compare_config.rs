// Golden master tests: compare Rust Config/PlayerConfig deserialization against Java-generated fixtures

use std::path::Path;

use bms_model::mode::Mode;
use rubato_types::audio_config::{DriverType, FrequencyType};
use rubato_types::config::{Config, DisplayMode, SongPreview};
use rubato_types::player_config::PlayerConfig;
use rubato_types::resolution::Resolution;
use rubato_types::validatable::Validatable;

fn fixtures_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .leak()
}

/// Pre-process JSON to fix Java mode hint format ("beat-7k") to Rust enum variant ("BEAT_7K").
fn fix_mode_hint(json: &str) -> String {
    let mut value: serde_json::Value = serde_json::from_str(json).expect("JSON parse failed");
    if let Some(obj) = value.as_object_mut()
        && let Some(mode_val) = obj
            .get("mode")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
    {
        // Convert hint format to enum variant: "beat-7k" -> "BEAT_7K"
        let converted = mode_val.replace('-', "_").to_uppercase();
        if Mode::from_hint(&mode_val).is_some() {
            obj.insert("mode".to_string(), serde_json::Value::String(converted));
        }
    }
    serde_json::to_string(&value).unwrap()
}

// --- System Config tests ---

#[test]
fn config_system_deserialize() {
    let path = fixtures_dir().join("config_system.json");
    assert!(
        path.exists(),
        "Config fixture not found: {}. Run `just golden-master-config-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    let config: Config = serde_json::from_str(&content).expect("Failed to deserialize Config");

    // Verify non-default values were deserialized correctly
    assert_eq!(config.playername.as_deref(), Some("TestPlayer"));
    assert_eq!(config.last_booted_version, "0.8.8");
    assert!(matches!(
        config.display.displaymode,
        DisplayMode::FULLSCREEN
    ));
    assert!(config.display.vsync);
    assert_eq!(config.display.resolution, Resolution::FULLHD);
    assert!(!config.display.use_resolution);
    assert_eq!(config.display.window_width, 1920);
    assert_eq!(config.display.window_height, 1080);
    assert!(!config.select.folderlamp);

    // Audio (Option<AudioConfig>)
    let audio = config
        .audio
        .as_ref()
        .expect("audio should be Some after deserialization");
    assert!(matches!(audio.driver, DriverType::PortAudio));
    assert_eq!(audio.driver_name.as_deref(), Some("TestDriver"));
    assert_eq!(audio.device_buffer_size, 512);
    assert_eq!(audio.device_simultaneous_sources, 256);
    assert_eq!(audio.sample_rate, 44100);
    assert!(matches!(audio.freq_option, FrequencyType::UNPROCESSED));
    assert!(matches!(audio.fast_forward, FrequencyType::UNPROCESSED));
    assert!((audio.systemvolume - 0.8_f32).abs() < 0.001);
    assert!((audio.keyvolume - 0.7_f32).abs() < 0.001);
    assert!((audio.bgvolume - 0.6_f32).abs() < 0.001);
    assert!(audio.normalize_volume);
    assert!(audio.is_loop_result_sound);
    assert!(audio.is_loop_course_result_sound);

    // Frame/scroll settings
    assert_eq!(config.display.max_frame_per_second, 120);
    assert_eq!(config.display.prepare_frame_per_second, 60);
    assert_eq!(config.select.max_search_bar_count, 20);
    assert!(config.select.skip_decide_screen);
    assert!(!config.select.show_no_song_existing_bar);
    assert_eq!(config.select.scrolldurationlow, 200);
    assert_eq!(config.select.scrolldurationhigh, 80);
    assert!(!config.select.analog_scroll);
    assert_eq!(config.select.analog_ticks_per_scroll, 5);
    assert!(matches!(config.select.song_preview, SongPreview::ONCE));
    assert!(config.select.cache_skin_image);
    assert!(!config.use_song_info);

    // Paths
    assert_eq!(config.paths.songpath, "custom_songdata.db");
    assert_eq!(config.paths.songinfopath, "custom_songinfo.db");
    assert_eq!(config.paths.tablepath, "custom_table");
    assert_eq!(config.paths.playerpath, "custom_player");
    assert_eq!(config.paths.skinpath, "custom_skin");
    assert_eq!(config.paths.bgmpath, "custom_bgm");
    assert_eq!(config.paths.soundpath, "custom_sound");
    assert_eq!(config.paths.systemfontpath, "font/custom.ttf");
    assert_eq!(config.paths.messagefontpath, "font/custom_msg.ttf");

    // Arrays
    assert_eq!(
        config.paths.bmsroot,
        vec!["C:\\BMS\\songs", "D:\\BMS\\extra"]
    );
    assert_eq!(
        config.paths.table_url,
        vec![
            "https://example.com/table1.html",
            "https://example.com/table2.html"
        ]
    );
    assert_eq!(
        config.paths.available_url,
        vec!["https://example.com/available.html"]
    );

    // BGA/resource
    assert_eq!(config.render.bga, rubato_types::config::BgaMode::Auto);
    assert_eq!(
        config.render.bga_expand,
        rubato_types::config::BgaExpand::Off
    );
    assert_eq!(config.render.frameskip, 0);
    assert!(config.updatesong);
    assert_eq!(config.render.skin_pixmap_gen, 8);
    assert_eq!(config.render.stagefile_pixmap_gen, 4);
    assert_eq!(config.render.banner_pixmap_gen, 3);
    assert_eq!(config.render.song_resource_gen, 2);

    // Network
    assert!(!config.network.enable_ipfs);
    assert_eq!(config.network.ipfsurl, "https://custom-gateway.io/");
    assert!(!config.network.enable_http);
    assert_eq!(config.network.download_source, "custom_source");
    assert_eq!(
        config.network.default_download_url,
        "https://download.example.com"
    );
    assert_eq!(
        config.network.override_download_url,
        "https://override.example.com"
    );
    assert_eq!(config.network.download_directory, "custom_download");

    // IR/Discord/screenshot
    assert_eq!(config.network.ir_send_count, 10);
    assert!(config.integration.use_discord_rpc);
    assert!(config.integration.set_clipboard_screenshot);
    assert_eq!(config.integration.monitor_name, "HDMI-1");

    // Webhook
    assert_eq!(config.integration.webhook_option, 1);
    assert_eq!(config.integration.webhook_name, "TestHook");
    assert_eq!(
        config.integration.webhook_avatar,
        "https://example.com/avatar.png"
    );
    assert_eq!(
        config.integration.webhook_url,
        vec!["https://discord.com/api/webhooks/test1"]
    );

    // OBS
    assert!(config.obs.use_obs_ws);
    assert_eq!(config.obs.obs_ws_host, "192.168.1.100");
    assert_eq!(config.obs.obs_ws_port, 4444);
    assert_eq!(config.obs.obs_ws_pass, "secret123");
    assert_eq!(config.obs.obs_ws_rec_stop_wait, 3000);
    assert_eq!(config.obs.obs_ws_rec_mode, 1);
    assert_eq!(
        config.obs.obs_scenes.get("play").map(String::as_str),
        Some("Gaming Scene")
    );
    assert_eq!(
        config.obs.obs_scenes.get("select").map(String::as_str),
        Some("Menu Scene")
    );
    assert_eq!(
        config.obs.obs_actions.get("start").map(String::as_str),
        Some("StartRecording")
    );
}

#[test]
fn config_system_validate_after_deserialize() {
    let path = fixtures_dir().join("config_system.json");
    if !path.exists() {
        return; // Skip if fixtures not generated
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let mut config: Config = serde_json::from_str(&content).unwrap();
    config.validate();

    // After validation, values within valid range should remain unchanged
    assert_eq!(config.display.window_width, 1920);
    assert_eq!(config.display.window_height, 1080);
    assert_eq!(config.display.max_frame_per_second, 120);
    assert_eq!(config.select.scrolldurationlow, 200);
    assert_eq!(config.render.bga, rubato_types::config::BgaMode::Auto);
    assert_eq!(
        config.render.bga_expand,
        rubato_types::config::BgaExpand::Off
    );
}

#[test]
fn config_system_serde_round_trip() {
    let path = fixtures_dir().join("config_system.json");
    if !path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let config: Config = serde_json::from_str(&content).unwrap();

    // Serialize back to JSON and deserialize again
    let json = serde_json::to_string_pretty(&config).unwrap();
    let config2: Config = serde_json::from_str(&json).unwrap();

    assert_eq!(config.playername, config2.playername);
    assert!(matches!(
        (&config.display.displaymode, &config2.display.displaymode),
        (DisplayMode::FULLSCREEN, DisplayMode::FULLSCREEN)
            | (DisplayMode::BORDERLESS, DisplayMode::BORDERLESS)
            | (DisplayMode::WINDOW, DisplayMode::WINDOW)
    ));
    assert_eq!(config.display.resolution, config2.display.resolution);
    assert_eq!(
        config.display.max_frame_per_second,
        config2.display.max_frame_per_second
    );
    assert_eq!(config.paths.table_url, config2.paths.table_url);
    assert_eq!(config.obs.obs_ws_port, config2.obs.obs_ws_port);
}

// --- Player Config tests ---

#[test]
fn config_player_deserialize() {
    let path = fixtures_dir().join("config_player.json");
    assert!(
        path.exists(),
        "Player config fixture not found: {}. Run `just golden-master-config-gen` first.",
        path.display()
    );
    let content = std::fs::read_to_string(&path).expect("Failed to read fixture");
    // Pre-process JSON to fix mode hint format
    let fixed_content = fix_mode_hint(&content);
    let pc: PlayerConfig =
        serde_json::from_str(&fixed_content).expect("Failed to deserialize PlayerConfig");

    // Basic fields
    assert_eq!(pc.id.as_deref(), Some("player_001"));
    assert_eq!(pc.name, "TestPlayer");
    assert_eq!(pc.play_settings.gauge, 3);
    assert_eq!(pc.play_settings.random, 5);
    assert_eq!(pc.play_settings.random2, 2);
    assert_eq!(pc.play_settings.doubleoption, 1);
    assert_eq!(pc.play_settings.chart_replication_mode, "NONE");
    assert_eq!(pc.select_settings.targetid, "RATE_AA");
    assert_eq!(
        pc.select_settings.targetlist,
        vec!["RATE_A", "RATE_AA", "MAX"]
    );
    assert_eq!(pc.judge_settings.judgetiming, -15);
    assert!(pc.judge_settings.notes_display_timing_auto_adjust);
    assert_eq!(pc.mode, Some(Mode::BEAT_7K));
    assert_eq!(pc.display_settings.misslayer_duration, 300);
    assert_eq!(pc.play_settings.lnmode, 1);
    assert!(pc.play_settings.forcedcnendings);

    // Scroll/longnote/mine
    assert_eq!(pc.display_settings.scroll_mode, 1);
    assert_eq!(pc.display_settings.scroll_section, 8);
    assert!((pc.display_settings.scroll_rate - 0.75).abs() < 0.001);
    assert_eq!(pc.note_modifier_settings.longnote_mode, 2);
    assert!((pc.note_modifier_settings.longnote_rate - 0.8).abs() < 0.001);
    assert_eq!(pc.play_settings.mine_mode, 2);

    // Custom judge
    assert!(pc.judge_settings.custom_judge);
    assert_eq!(pc.judge_settings.key_judge_window_rate_perfect_great, 200);
    assert_eq!(pc.judge_settings.key_judge_window_rate_great, 150);
    assert_eq!(pc.judge_settings.key_judge_window_rate_good, 50);
    assert_eq!(
        pc.judge_settings.scratch_judge_window_rate_perfect_great,
        300
    );
    assert_eq!(pc.judge_settings.scratch_judge_window_rate_great, 250);
    assert_eq!(pc.judge_settings.scratch_judge_window_rate_good, 80);

    // Assist
    assert!(pc.display_settings.bpmguide);
    assert_eq!(pc.display_settings.extranote_type, 1);
    assert_eq!(pc.display_settings.extranote_depth, 5);
    assert!(pc.display_settings.extranote_scratch);

    // Display
    assert!(pc.display_settings.showjudgearea);
    assert!(pc.display_settings.markprocessednote);

    // H-RANDOM/gauge auto shift
    assert_eq!(pc.play_settings.hran_threshold_bpm, 180);
    assert_eq!(pc.play_settings.gauge_auto_shift, 2);
    assert_eq!(pc.play_settings.bottom_shiftable_gauge, 1);

    // Auto-save replay (Vec<i32>, not Option<Vec<i32>>)
    assert_eq!(pc.misc_settings.autosavereplay, vec![1, 0, 1, 0]);

    // 7to9
    assert_eq!(pc.note_modifier_settings.seven_to_nine_pattern, 3);
    assert_eq!(pc.note_modifier_settings.seven_to_nine_type, 1);

    // Exit
    assert_eq!(pc.misc_settings.exit_press_duration, 2000);

    // Misc flags
    assert!(pc.display_settings.is_guide_se);
    assert!(pc.select_settings.is_window_hold);
    assert!(pc.select_settings.is_random_select);

    // Mode7 PlayConfig
    let mode7 = &pc.mode7;
    assert_eq!(mode7.version, 1);
    assert!((mode7.playconfig.hispeed - 3.5_f32).abs() < 0.001);
    assert_eq!(mode7.playconfig.duration, 300);
    assert!(mode7.playconfig.enable_constant);
    assert_eq!(mode7.playconfig.constant_fadein_time, 50);
    assert_eq!(mode7.playconfig.fixhispeed, 1);
    assert!((mode7.playconfig.hispeedmargin - 0.5_f32).abs() < 0.001);
    assert!((mode7.playconfig.lanecover - 0.3_f32).abs() < 0.001);
    assert!(!mode7.playconfig.enablelanecover);
    assert!((mode7.playconfig.lift - 0.15_f32).abs() < 0.001);
    assert!(mode7.playconfig.enablelift);
    assert!((mode7.playconfig.hidden - 0.05_f32).abs() < 0.001);
    assert!(mode7.playconfig.enablehidden);
    assert_eq!(mode7.playconfig.lanecoverswitchduration, 300);
    assert!(mode7.playconfig.hispeedautoadjust);
    assert_eq!(mode7.playconfig.judgetype, "Duration");

    // Mode7 keyboard
    assert_eq!(
        mode7.keyboard.keys,
        vec![54, 47, 52, 32, 31, 34, 50, 59, 129]
    );
    assert_eq!(mode7.keyboard.start, 45);
    assert_eq!(mode7.keyboard.select, 51);

    // Mode7 controller
    assert_eq!(mode7.controller.len(), 1);
    assert_eq!(mode7.controller[0].name, "IIDX Controller");
    assert_eq!(mode7.controller[0].keys, vec![3, 6, 2, 7, 1, 4, 39, 37, 36]);
    assert!(mode7.controller[0].jkoc_hack);
    assert!(mode7.controller[0].analog_scratch);
    assert_eq!(mode7.controller[0].analog_scratch_mode, 1);
    assert_eq!(mode7.controller[0].analog_scratch_threshold, 30);

    // Display/sort
    assert!(pc.display_settings.showhiddennote);
    assert!(pc.display_settings.showpastnote);
    assert!(!pc.display_settings.chart_preview);
    assert_eq!(pc.select_settings.sort, 3);
    assert_eq!(pc.select_settings.sortid.as_deref(), Some("TITLE"));
    assert_eq!(pc.select_settings.musicselectinput, 1);

    // IR config (Vec<Option<IRConfig>>, not Option<Vec<IRConfig>>)
    let irconfigs: Vec<_> = pc.irconfig.iter().filter_map(|c| c.as_ref()).collect();
    assert_eq!(irconfigs.len(), 1);
    assert_eq!(irconfigs[0].irname, "LR2IR");
    assert_eq!(irconfigs[0].cuserid, "encrypted_user");
    assert_eq!(irconfigs[0].cpassword, "encrypted_pass");
    assert_eq!(irconfigs[0].irsend, 1);
    assert!(irconfigs[0].importscore);
    assert!(!irconfigs[0].importrival);

    // Stream/event
    assert!(pc.enable_request);
    assert!(pc.notify_request);
    assert_eq!(pc.max_request_count, 50);
    assert!(pc.select_settings.event_mode);
}

#[test]
fn config_player_validate_after_deserialize() {
    let path = fixtures_dir().join("config_player.json");
    if !path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let fixed_content = fix_mode_hint(&content);
    let mut pc: PlayerConfig = serde_json::from_str(&fixed_content).unwrap();
    pc.validate();

    // Values within valid range should remain unchanged after validation
    assert_eq!(pc.play_settings.gauge, 3);
    assert_eq!(pc.play_settings.random, 5);
    assert_eq!(pc.judge_settings.judgetiming, -15);
    assert_eq!(pc.select_settings.sort, 3);
    assert_eq!(pc.play_settings.lnmode, 1);

    // Skin array should be normalized to expected size
    let max_skin_id = rubato_types::skin_type::SkinType::max_skin_type_id() as usize;
    assert_eq!(pc.skin.len(), max_skin_id + 1);

    // autosavereplay should remain length 4
    assert_eq!(pc.misc_settings.autosavereplay.len(), 4);
}

#[test]
fn config_player_serde_round_trip() {
    let path = fixtures_dir().join("config_player.json");
    if !path.exists() {
        return;
    }
    let content = std::fs::read_to_string(&path).unwrap();
    let fixed_content = fix_mode_hint(&content);
    let pc: PlayerConfig = serde_json::from_str(&fixed_content).unwrap();

    let json = serde_json::to_string_pretty(&pc).unwrap();
    let pc2: PlayerConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(pc.name, pc2.name);
    assert_eq!(pc.play_settings.gauge, pc2.play_settings.gauge);
    assert_eq!(pc.play_settings.random, pc2.play_settings.random);
    assert_eq!(
        pc.judge_settings.judgetiming,
        pc2.judge_settings.judgetiming
    );
    assert_eq!(
        pc.select_settings.targetlist,
        pc2.select_settings.targetlist
    );
    assert_eq!(
        pc.mode7.playconfig.judgetype,
        pc2.mode7.playconfig.judgetype
    );
}
