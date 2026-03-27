#![allow(clippy::field_reassign_with_default)]
// Integration test: Config serialize -> deserialize round-trip
//
// Creates a Config with non-default values, serializes to JSON,
// deserializes back, and verifies all fields match.

use std::collections::HashMap;

use rubato_game::core::config::{
    BGA_OFF, BGAEXPAND_FULL, Config, DisplayConfig, DisplayMode, IntegrationConfig, NetworkConfig,
    ObsConfig, PathConfig, RenderConfig, SelectConfig, SongPreview,
};
use rubato_types::audio_config::AudioConfig;
use rubato_types::resolution::Resolution;

/// Create a Config with non-default values for all fields.
fn make_custom_config() -> Config {
    let mut audio = AudioConfig::default();
    audio.systemvolume = 0.8;
    audio.keyvolume = 0.6;
    audio.bgvolume = 0.4;
    audio.device_buffer_size = 512;
    audio.device_simultaneous_sources = 64;
    audio.normalize_volume = true;

    let mut obs_scenes = HashMap::new();
    obs_scenes.insert("play".to_string(), "PlayScene".to_string());
    obs_scenes.insert("select".to_string(), "SelectScene".to_string());

    let mut obs_actions = HashMap::new();
    obs_actions.insert("play".to_string(), "StartRecording".to_string());

    Config {
        playername: Some("TestPlayer".to_string()),
        last_booted_version: "1.0.0".to_string(),
        audio: Some(audio),
        use_song_info: false,
        updatesong: true,
        display: DisplayConfig {
            displaymode: DisplayMode::FULLSCREEN,
            vsync: true,
            resolution: Resolution::FULLHD,
            use_resolution: false,
            window_width: 1920,
            window_height: 1080,
            max_frame_per_second: 120,
            prepare_frame_per_second: 60,
        },
        paths: PathConfig {
            songpath: "custom_song.db".to_string(),
            songinfopath: "custom_songinfo.db".to_string(),
            tablepath: "custom_table".to_string(),
            playerpath: "custom_player".to_string(),
            skinpath: "custom_skin".to_string(),
            bgmpath: "custom_bgm".to_string(),
            soundpath: "custom_sound".to_string(),
            systemfontpath: "custom_font.ttf".to_string(),
            messagefontpath: "custom_msg_font.ttf".to_string(),
            bmsroot: vec!["/songs/root1".to_string(), "/songs/root2".to_string()],
            table_url: vec![
                "https://example.com/table1".to_string(),
                "https://example.com/table2".to_string(),
            ],
            available_url: vec!["https://example.com/avail".to_string()],
        },
        render: RenderConfig {
            bga: BGA_OFF,
            bga_expand: BGAEXPAND_FULL,
            frameskip: 2,
            skin_pixmap_gen: 8,
            stagefile_pixmap_gen: 4,
            banner_pixmap_gen: 4,
            song_resource_gen: 2,
        },
        network: NetworkConfig {
            enable_ipfs: false,
            ipfsurl: "https://custom.ipfs.io/".to_string(),
            enable_http: false,
            download_source: "custom_source".to_string(),
            default_download_url: "https://dl.example.com".to_string(),
            override_download_url: "https://override.example.com".to_string(),
            download_directory: "custom_downloads".to_string(),
            ir_send_count: 10,
        },
        obs: ObsConfig {
            use_obs_ws: true,
            obs_ws_host: "192.168.1.100".to_string(),
            obs_ws_port: 4444,
            obs_ws_pass: "obspassword".to_string(),
            obs_ws_rec_stop_wait: 3000,
            obs_ws_rec_mode: 1,
            obs_scenes,
            obs_actions,
        },
        integration: IntegrationConfig {
            use_discord_rpc: true,
            set_clipboard_screenshot: true,
            monitor_name: "HDMI-1".to_string(),
            webhook_option: 2,
            webhook_name: "MyWebhook".to_string(),
            webhook_avatar: "https://example.com/avatar.png".to_string(),
            webhook_url: vec!["https://hook.example.com/1".to_string()],
        },
        select: SelectConfig {
            folderlamp: false,
            max_search_bar_count: 20,
            skip_decide_screen: true,
            show_no_song_existing_bar: false,
            scrolldurationlow: 500,
            scrolldurationhigh: 100,
            analog_scroll: false,
            analog_ticks_per_scroll: 5,
            song_preview: SongPreview::NONE,
            cache_skin_image: true,
        },
    }
}

#[test]
fn config_serialize_deserialize_roundtrip() {
    let config = make_custom_config();

    // Serialize to JSON
    let json = serde_json::to_string_pretty(&config).expect("Config serialization should not fail");

    // Deserialize back
    let restored: Config =
        serde_json::from_str(&json).expect("Config deserialization should not fail");

    // Verify all scalar fields
    assert_eq!(restored.playername, config.playername);
    assert_eq!(restored.last_booted_version, config.last_booted_version);
    assert_eq!(restored.display.vsync, config.display.vsync);
    assert_eq!(
        restored.display.use_resolution,
        config.display.use_resolution
    );
    assert_eq!(restored.display.window_width, config.display.window_width);
    assert_eq!(restored.display.window_height, config.display.window_height);
    assert_eq!(restored.select.folderlamp, config.select.folderlamp);
    assert_eq!(
        restored.display.max_frame_per_second,
        config.display.max_frame_per_second
    );
    assert_eq!(
        restored.display.prepare_frame_per_second,
        config.display.prepare_frame_per_second
    );
    assert_eq!(
        restored.select.max_search_bar_count,
        config.select.max_search_bar_count
    );
    assert_eq!(
        restored.select.skip_decide_screen,
        config.select.skip_decide_screen
    );
    assert_eq!(
        restored.select.show_no_song_existing_bar,
        config.select.show_no_song_existing_bar
    );
    assert_eq!(
        restored.select.scrolldurationlow,
        config.select.scrolldurationlow
    );
    assert_eq!(
        restored.select.scrolldurationhigh,
        config.select.scrolldurationhigh
    );
    assert_eq!(restored.select.analog_scroll, config.select.analog_scroll);
    assert_eq!(
        restored.select.analog_ticks_per_scroll,
        config.select.analog_ticks_per_scroll
    );
    assert_eq!(
        restored.select.cache_skin_image,
        config.select.cache_skin_image
    );
    assert_eq!(restored.use_song_info, config.use_song_info);

    // Verify string paths
    assert_eq!(restored.paths.songpath, config.paths.songpath);
    assert_eq!(restored.paths.songinfopath, config.paths.songinfopath);
    assert_eq!(restored.paths.tablepath, config.paths.tablepath);
    assert_eq!(restored.paths.playerpath, config.paths.playerpath);
    assert_eq!(restored.paths.skinpath, config.paths.skinpath);
    assert_eq!(restored.paths.bgmpath, config.paths.bgmpath);
    assert_eq!(restored.paths.soundpath, config.paths.soundpath);
    assert_eq!(restored.paths.systemfontpath, config.paths.systemfontpath);
    assert_eq!(restored.paths.messagefontpath, config.paths.messagefontpath);

    // Verify vec fields
    assert_eq!(restored.paths.bmsroot, config.paths.bmsroot);
    assert_eq!(restored.paths.table_url, config.paths.table_url);
    assert_eq!(restored.paths.available_url, config.paths.available_url);
    assert_eq!(
        restored.integration.webhook_url,
        config.integration.webhook_url
    );

    // Verify numeric fields
    assert_eq!(restored.render.bga, config.render.bga);
    assert_eq!(restored.render.bga_expand, config.render.bga_expand);
    assert_eq!(restored.render.frameskip, config.render.frameskip);
    assert_eq!(restored.updatesong, config.updatesong);
    assert_eq!(
        restored.render.skin_pixmap_gen,
        config.render.skin_pixmap_gen
    );
    assert_eq!(
        restored.render.stagefile_pixmap_gen,
        config.render.stagefile_pixmap_gen
    );
    assert_eq!(
        restored.render.banner_pixmap_gen,
        config.render.banner_pixmap_gen
    );
    assert_eq!(
        restored.render.song_resource_gen,
        config.render.song_resource_gen
    );
    assert_eq!(restored.network.enable_ipfs, config.network.enable_ipfs);
    assert_eq!(restored.network.ipfsurl, config.network.ipfsurl);
    assert_eq!(restored.network.enable_http, config.network.enable_http);
    assert_eq!(
        restored.network.download_source,
        config.network.download_source
    );
    assert_eq!(
        restored.network.default_download_url,
        config.network.default_download_url
    );
    assert_eq!(
        restored.network.override_download_url,
        config.network.override_download_url
    );
    assert_eq!(
        restored.network.download_directory,
        config.network.download_directory
    );
    assert_eq!(restored.network.ir_send_count, config.network.ir_send_count);
    assert_eq!(
        restored.integration.use_discord_rpc,
        config.integration.use_discord_rpc
    );
    assert_eq!(
        restored.integration.set_clipboard_screenshot,
        config.integration.set_clipboard_screenshot
    );
    assert_eq!(
        restored.integration.monitor_name,
        config.integration.monitor_name
    );

    // Verify webhook fields
    assert_eq!(
        restored.integration.webhook_option,
        config.integration.webhook_option
    );
    assert_eq!(
        restored.integration.webhook_name,
        config.integration.webhook_name
    );
    assert_eq!(
        restored.integration.webhook_avatar,
        config.integration.webhook_avatar
    );

    // Verify OBS fields
    assert_eq!(restored.obs.use_obs_ws, config.obs.use_obs_ws);
    assert_eq!(restored.obs.obs_ws_host, config.obs.obs_ws_host);
    assert_eq!(restored.obs.obs_ws_port, config.obs.obs_ws_port);
    assert_eq!(restored.obs.obs_ws_pass, config.obs.obs_ws_pass);
    assert_eq!(
        restored.obs.obs_ws_rec_stop_wait,
        config.obs.obs_ws_rec_stop_wait
    );
    assert_eq!(restored.obs.obs_ws_rec_mode, config.obs.obs_ws_rec_mode);
    assert_eq!(restored.obs.obs_scenes, config.obs.obs_scenes);
    assert_eq!(restored.obs.obs_actions, config.obs.obs_actions);

    // Verify audio config
    assert!(restored.audio.is_some());
    let restored_audio = restored.audio.as_ref().unwrap();
    let config_audio = config.audio.as_ref().unwrap();
    assert_eq!(restored_audio.systemvolume, config_audio.systemvolume);
    assert_eq!(restored_audio.keyvolume, config_audio.keyvolume);
    assert_eq!(restored_audio.bgvolume, config_audio.bgvolume);
    assert_eq!(
        restored_audio.device_buffer_size,
        config_audio.device_buffer_size
    );
    assert_eq!(
        restored_audio.device_simultaneous_sources,
        config_audio.device_simultaneous_sources
    );
    assert_eq!(
        restored_audio.normalize_volume,
        config_audio.normalize_volume
    );
}

#[test]
fn config_empty_json_produces_defaults() {
    let config: Config = serde_json::from_str("{}").expect("Empty JSON should deserialize");
    let default = Config::default();

    assert_eq!(config.display.window_width, default.display.window_width);
    assert_eq!(config.display.window_height, default.display.window_height);
    assert_eq!(config.paths.songpath, default.paths.songpath);
    assert_eq!(config.paths.songinfopath, default.paths.songinfopath);
    assert_eq!(config.paths.tablepath, default.paths.tablepath);
    assert_eq!(config.paths.playerpath, default.paths.playerpath);
    assert_eq!(config.paths.skinpath, default.paths.skinpath);
    assert_eq!(
        config.display.max_frame_per_second,
        default.display.max_frame_per_second
    );
    assert_eq!(config.render.bga, default.render.bga);
    assert_eq!(config.render.bga_expand, default.render.bga_expand);
    assert_eq!(config.display.vsync, default.display.vsync);
    assert_eq!(config.select.folderlamp, default.select.folderlamp);
}

#[test]
fn config_partial_json_fills_defaults() {
    let json = r#"{"windowWidth": 3840, "vsync": true}"#;
    let config: Config = serde_json::from_str(json).expect("Partial JSON should deserialize");
    let default = Config::default();

    // Explicitly set fields
    assert_eq!(config.display.window_width, 3840);
    assert!(config.display.vsync);

    // Everything else should be default
    assert_eq!(config.display.window_height, default.display.window_height);
    assert_eq!(config.paths.songpath, default.paths.songpath);
    assert_eq!(config.render.bga, default.render.bga);
    assert_eq!(
        config.display.max_frame_per_second,
        default.display.max_frame_per_second
    );
}

#[test]
fn config_roundtrip_preserves_resolution_enum() {
    let resolutions = [
        Resolution::SD,
        Resolution::SVGA,
        Resolution::XGA,
        Resolution::HD,
        Resolution::FULLHD,
        Resolution::WQHD,
        Resolution::ULTRAHD,
    ];

    for res in &resolutions {
        let mut config = Config::default();
        config.display.resolution = *res;

        let json = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(
            restored.display.resolution, config.display.resolution,
            "Resolution {:?} should survive round-trip",
            res
        );
    }
}

#[test]
fn config_roundtrip_preserves_display_mode_variants() {
    // Test each DisplayMode variant
    let modes_json = [
        (r#""FULLSCREEN""#, "FULLSCREEN"),
        (r#""BORDERLESS""#, "BORDERLESS"),
        (r#""WINDOW""#, "WINDOW"),
    ];

    for (json_val, label) in &modes_json {
        let json = format!(r#"{{"displaymode": {}}}"#, json_val);
        let config: Config = serde_json::from_str(&json).unwrap();

        // Re-serialize and deserialize
        let json2 = serde_json::to_string(&config).unwrap();
        let restored: Config = serde_json::from_str(&json2).unwrap();

        // Serialize again to compare
        let json3 = serde_json::to_string(&restored).unwrap();
        assert_eq!(
            json2, json3,
            "DisplayMode {} should survive round-trip",
            label
        );
    }
}

#[test]
fn config_roundtrip_preserves_obs_maps() {
    let mut config = Config::default();

    config
        .obs
        .obs_scenes
        .insert("play".to_string(), "PlayScene".to_string());
    config
        .obs
        .obs_scenes
        .insert("select".to_string(), "SelectScene".to_string());
    config
        .obs
        .obs_scenes
        .insert("result".to_string(), "ResultScene".to_string());

    config
        .obs
        .obs_actions
        .insert("play".to_string(), "StartRecording".to_string());
    config
        .obs
        .obs_actions
        .insert("result".to_string(), "StopRecording".to_string());

    let json = serde_json::to_string_pretty(&config).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.obs.obs_scenes.len(), 3);
    assert_eq!(
        restored.obs.obs_scenes.get("play"),
        Some(&"PlayScene".to_string())
    );
    assert_eq!(
        restored.obs.obs_scenes.get("select"),
        Some(&"SelectScene".to_string())
    );
    assert_eq!(
        restored.obs.obs_scenes.get("result"),
        Some(&"ResultScene".to_string())
    );

    assert_eq!(restored.obs.obs_actions.len(), 2);
    assert_eq!(
        restored.obs.obs_actions.get("play"),
        Some(&"StartRecording".to_string())
    );
    assert_eq!(
        restored.obs.obs_actions.get("result"),
        Some(&"StopRecording".to_string())
    );
}
