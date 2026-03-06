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
    assert!(matches!(config.displaymode, DisplayMode::FULLSCREEN));
    assert!(config.vsync);
    assert_eq!(config.resolution, Resolution::FULLHD);
    assert!(!config.use_resolution);
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);
    assert!(!config.folderlamp);

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
    assert_eq!(config.max_frame_per_second, 120);
    assert_eq!(config.prepare_frame_per_second, 60);
    assert_eq!(config.max_search_bar_count, 20);
    assert!(config.skip_decide_screen);
    assert!(!config.show_no_song_existing_bar);
    assert_eq!(config.scrolldurationlow, 200);
    assert_eq!(config.scrolldurationhigh, 80);
    assert!(!config.analog_scroll);
    assert_eq!(config.analog_ticks_per_scroll, 5);
    assert!(matches!(config.song_preview, SongPreview::ONCE));
    assert!(config.cache_skin_image);
    assert!(!config.use_song_info);

    // Paths
    assert_eq!(config.songpath, "custom_songdata.db");
    assert_eq!(config.songinfopath, "custom_songinfo.db");
    assert_eq!(config.tablepath, "custom_table");
    assert_eq!(config.playerpath, "custom_player");
    assert_eq!(config.skinpath, "custom_skin");
    assert_eq!(config.bgmpath, "custom_bgm");
    assert_eq!(config.soundpath, "custom_sound");
    assert_eq!(config.systemfontpath, "font/custom.ttf");
    assert_eq!(config.messagefontpath, "font/custom_msg.ttf");

    // Arrays
    assert_eq!(config.bmsroot, vec!["C:\\BMS\\songs", "D:\\BMS\\extra"]);
    assert_eq!(
        config.table_url,
        vec![
            "https://example.com/table1.html",
            "https://example.com/table2.html"
        ]
    );
    assert_eq!(
        config.available_url,
        vec!["https://example.com/available.html"]
    );

    // BGA/resource
    assert_eq!(config.bga, 1);
    assert_eq!(config.bga_expand, 2);
    assert_eq!(config.frameskip, 0);
    assert!(config.updatesong);
    assert_eq!(config.skin_pixmap_gen, 8);
    assert_eq!(config.stagefile_pixmap_gen, 4);
    assert_eq!(config.banner_pixmap_gen, 3);
    assert_eq!(config.song_resource_gen, 2);

    // Network
    assert!(!config.enable_ipfs);
    assert_eq!(config.ipfsurl, "https://custom-gateway.io/");
    assert!(!config.enable_http);
    assert_eq!(config.download_source, "custom_source");
    assert_eq!(config.default_download_url, "https://download.example.com");
    assert_eq!(config.override_download_url, "https://override.example.com");
    assert_eq!(config.download_directory, "custom_download");

    // IR/Discord/screenshot
    assert_eq!(config.ir_send_count, 10);
    assert!(config.use_discord_rpc);
    assert!(config.set_clipboard_screenshot);
    assert_eq!(config.monitor_name, "HDMI-1");

    // Webhook
    assert_eq!(config.webhook_option, 1);
    assert_eq!(config.webhook_name, "TestHook");
    assert_eq!(config.webhook_avatar, "https://example.com/avatar.png");
    assert_eq!(
        config.webhook_url,
        vec!["https://discord.com/api/webhooks/test1"]
    );

    // OBS
    assert!(config.use_obs_ws);
    assert_eq!(config.obs_ws_host, "192.168.1.100");
    assert_eq!(config.obs_ws_port, 4444);
    assert_eq!(config.obs_ws_pass, "secret123");
    assert_eq!(config.obs_ws_rec_stop_wait, 3000);
    assert_eq!(config.obs_ws_rec_mode, 1);
    assert_eq!(
        config.obs_scenes.get("play").map(String::as_str),
        Some("Gaming Scene")
    );
    assert_eq!(
        config.obs_scenes.get("select").map(String::as_str),
        Some("Menu Scene")
    );
    assert_eq!(
        config.obs_actions.get("start").map(String::as_str),
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
    assert_eq!(config.window_width, 1920);
    assert_eq!(config.window_height, 1080);
    assert_eq!(config.max_frame_per_second, 120);
    assert_eq!(config.scrolldurationlow, 200);
    assert_eq!(config.bga, 1);
    assert_eq!(config.bga_expand, 2);
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
        (&config.displaymode, &config2.displaymode),
        (DisplayMode::FULLSCREEN, DisplayMode::FULLSCREEN)
            | (DisplayMode::BORDERLESS, DisplayMode::BORDERLESS)
            | (DisplayMode::WINDOW, DisplayMode::WINDOW)
    ));
    assert_eq!(config.resolution, config2.resolution);
    assert_eq!(config.max_frame_per_second, config2.max_frame_per_second);
    assert_eq!(config.table_url, config2.table_url);
    assert_eq!(config.obs_ws_port, config2.obs_ws_port);
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
    assert_eq!(pc.gauge, 3);
    assert_eq!(pc.random, 5);
    assert_eq!(pc.random2, 2);
    assert_eq!(pc.doubleoption, 1);
    assert_eq!(pc.chart_replication_mode, "NONE");
    assert_eq!(pc.targetid, "RATE_AA");
    assert_eq!(pc.targetlist, vec!["RATE_A", "RATE_AA", "MAX"]);
    assert_eq!(pc.judgetiming, -15);
    assert!(pc.notes_display_timing_auto_adjust);
    assert_eq!(pc.mode, Some(Mode::BEAT_7K));
    assert_eq!(pc.misslayer_duration, 300);
    assert_eq!(pc.lnmode, 1);
    assert!(pc.forcedcnendings);

    // Scroll/longnote/mine
    assert_eq!(pc.scroll_mode, 1);
    assert_eq!(pc.scroll_section, 8);
    assert!((pc.scroll_rate - 0.75).abs() < 0.001);
    assert_eq!(pc.longnote_mode, 2);
    assert!((pc.longnote_rate - 0.8).abs() < 0.001);
    assert_eq!(pc.mine_mode, 2);

    // Custom judge
    assert!(pc.custom_judge);
    assert_eq!(pc.key_judge_window_rate_perfect_great, 200);
    assert_eq!(pc.key_judge_window_rate_great, 150);
    assert_eq!(pc.key_judge_window_rate_good, 50);
    assert_eq!(pc.scratch_judge_window_rate_perfect_great, 300);
    assert_eq!(pc.scratch_judge_window_rate_great, 250);
    assert_eq!(pc.scratch_judge_window_rate_good, 80);

    // Assist
    assert!(pc.bpmguide);
    assert_eq!(pc.extranote_type, 1);
    assert_eq!(pc.extranote_depth, 5);
    assert!(pc.extranote_scratch);

    // Display
    assert!(pc.showjudgearea);
    assert!(pc.markprocessednote);

    // H-RANDOM/gauge auto shift
    assert_eq!(pc.hran_threshold_bpm, 180);
    assert_eq!(pc.gauge_auto_shift, 2);
    assert_eq!(pc.bottom_shiftable_gauge, 1);

    // Auto-save replay (Vec<i32>, not Option<Vec<i32>>)
    assert_eq!(pc.autosavereplay, vec![1, 0, 1, 0]);

    // 7to9
    assert_eq!(pc.seven_to_nine_pattern, 3);
    assert_eq!(pc.seven_to_nine_type, 1);

    // Exit
    assert_eq!(pc.exit_press_duration, 2000);

    // Misc flags
    assert!(pc.is_guide_se);
    assert!(pc.is_window_hold);
    assert!(pc.is_random_select);

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
    assert!(pc.showhiddennote);
    assert!(pc.showpastnote);
    assert!(!pc.chart_preview);
    assert_eq!(pc.sort, 3);
    assert_eq!(pc.sortid.as_deref(), Some("TITLE"));
    assert_eq!(pc.musicselectinput, 1);

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
    assert!(pc.event_mode);
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
    assert_eq!(pc.gauge, 3);
    assert_eq!(pc.random, 5);
    assert_eq!(pc.judgetiming, -15);
    assert_eq!(pc.sort, 3);
    assert_eq!(pc.lnmode, 1);

    // Skin array should be normalized to expected size
    let max_skin_id = rubato_types::skin_type::SkinType::max_skin_type_id() as usize;
    assert_eq!(pc.skin.len(), max_skin_id + 1);

    // autosavereplay should remain length 4
    assert_eq!(pc.autosavereplay.len(), 4);
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
    assert_eq!(pc.gauge, pc2.gauge);
    assert_eq!(pc.random, pc2.random);
    assert_eq!(pc.judgetiming, pc2.judgetiming);
    assert_eq!(pc.targetlist, pc2.targetlist);
    assert_eq!(
        pc.mode7.playconfig.judgetype,
        pc2.mode7.playconfig.judgetype
    );
}
