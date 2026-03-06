pub use rubato_types::config::*;

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default_construction() {
        let config = Config::default();
        assert!(config.playername.is_none());
        assert_eq!(config.window_width, 1280);
        assert_eq!(config.window_height, 720);
        assert!(config.folderlamp);
        assert_eq!(config.max_frame_per_second, 240);
        assert_eq!(config.max_search_bar_count, 10);
        assert!(!config.skip_decide_screen);
        assert!(config.show_no_song_existing_bar);
        assert!(config.use_resolution);
        assert!(!config.vsync);
    }

    #[test]
    fn test_config_default_paths() {
        let config = Config::default();
        assert_eq!(config.songpath, SONGPATH_DEFAULT);
        assert_eq!(config.songinfopath, SONGINFOPATH_DEFAULT);
        assert_eq!(config.tablepath, TABLEPATH_DEFAULT);
        assert_eq!(config.playerpath, PLAYERPATH_DEFAULT);
        assert_eq!(config.skinpath, SKINPATH_DEFAULT);
        assert_eq!(config.bgmpath, "bgm");
        assert_eq!(config.soundpath, "sound");
    }

    #[test]
    fn test_config_default_bga() {
        let config = Config::default();
        assert_eq!(config.bga, BGA_ON);
        assert_eq!(config.bga_expand, BGAEXPAND_KEEP_ASPECT_RATIO);
    }

    #[test]
    fn test_config_default_table_urls_not_empty() {
        let config = Config::default();
        assert!(!config.table_url.is_empty());
    }

    #[test]
    fn test_config_serde_roundtrip() {
        let config = Config::default();
        let json = serde_json::to_string_pretty(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(config.window_width, deserialized.window_width);
        assert_eq!(config.window_height, deserialized.window_height);
        assert_eq!(config.songpath, deserialized.songpath);
        assert_eq!(config.songinfopath, deserialized.songinfopath);
        assert_eq!(config.tablepath, deserialized.tablepath);
        assert_eq!(config.playerpath, deserialized.playerpath);
        assert_eq!(config.skinpath, deserialized.skinpath);
        assert_eq!(
            config.max_frame_per_second,
            deserialized.max_frame_per_second
        );
        assert_eq!(config.bga, deserialized.bga);
        assert_eq!(config.bga_expand, deserialized.bga_expand);
        assert_eq!(config.vsync, deserialized.vsync);
        assert_eq!(config.folderlamp, deserialized.folderlamp);
    }

    #[test]
    fn test_config_serde_with_custom_values() {
        let mut config = Config::default();
        config.playername = Some("TestPlayer".to_string());
        config.window_width = 1920;
        config.window_height = 1080;
        config.vsync = true;
        config.bga = BGA_OFF;

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: Config = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.playername, Some("TestPlayer".to_string()));
        assert_eq!(deserialized.window_width, 1920);
        assert_eq!(deserialized.window_height, 1080);
        assert!(deserialized.vsync);
        assert_eq!(deserialized.bga, BGA_OFF);
    }

    #[test]
    fn test_config_deserialize_empty_json_uses_defaults() {
        let config: Config = serde_json::from_str("{}").unwrap();
        let default = Config::default();
        assert_eq!(config.window_width, default.window_width);
        assert_eq!(config.window_height, default.window_height);
        assert_eq!(config.songpath, default.songpath);
    }

    #[test]
    fn test_config_getters() {
        let mut config = Config::default();
        config.playername = Some("Player1".to_string());

        assert_eq!(config.playername(), Some("Player1"));
        assert_eq!(config.songpath, SONGPATH_DEFAULT);
        assert_eq!(config.songinfopath, SONGINFOPATH_DEFAULT);
        assert_eq!(config.tablepath, TABLEPATH_DEFAULT);
        assert_eq!(config.playerpath, PLAYERPATH_DEFAULT);
        assert_eq!(config.skinpath, SKINPATH_DEFAULT);
        assert_eq!(config.bgmpath, "bgm");
        assert_eq!(config.soundpath, "sound");
        assert_eq!(config.max_frame_per_second, 240);
        assert_eq!(config.max_search_bar_count, 10);
        assert_eq!(config.bga, BGA_ON);
        assert_eq!(config.bga_expand, BGAEXPAND_KEEP_ASPECT_RATIO);
        assert_eq!(config.frameskip, 1);
    }

    #[test]
    fn test_config_is_show_no_song_existing_bar() {
        let mut config = Config::default();

        // Both true by default
        assert!(config.is_show_no_song_existing_bar());

        // Even if show_no_song_existing_bar is false, enable_http makes it true
        config.show_no_song_existing_bar = false;
        config.enable_http = true;
        assert!(config.is_show_no_song_existing_bar());

        // Both false
        config.show_no_song_existing_bar = false;
        config.enable_http = false;
        assert!(!config.is_show_no_song_existing_bar());

        // Only show_no_song_existing_bar true
        config.show_no_song_existing_bar = true;
        config.enable_http = false;
        assert!(config.is_show_no_song_existing_bar());
    }

    #[test]
    fn test_config_set_analog_ticks_per_scroll() {
        let mut config = Config::default();

        config.set_analog_ticks_per_scroll(5);
        assert_eq!(config.analog_ticks_per_scroll, 5);

        // Should clamp to minimum of 1
        config.set_analog_ticks_per_scroll(0);
        assert_eq!(config.analog_ticks_per_scroll, 1);

        config.set_analog_ticks_per_scroll(-10);
        assert_eq!(config.analog_ticks_per_scroll, 1);
    }

    #[test]
    fn test_config_get_obs_ws_pass() {
        let mut config = Config::default();

        // Empty password returns None
        assert!(config.obs_ws_pass().is_none());

        // Whitespace-only password returns None
        config.obs_ws_pass = "   ".to_string();
        assert!(config.obs_ws_pass().is_none());

        // Valid password returns Some
        config.obs_ws_pass = "secret123".to_string();
        assert_eq!(config.obs_ws_pass(), Some("secret123"));
    }

    #[test]
    fn test_config_set_obs_ws_port() {
        let mut config = Config::default();

        config.set_obs_ws_port(8080);
        assert_eq!(config.obs_ws_port, 8080);

        // Clamp to valid range
        config.set_obs_ws_port(-1);
        assert_eq!(config.obs_ws_port, 0);

        config.set_obs_ws_port(70000);
        assert_eq!(config.obs_ws_port, 65535);
    }

    #[test]
    fn test_config_set_obs_ws_rec_stop_wait() {
        let mut config = Config::default();

        config.set_obs_ws_rec_stop_wait(3000);
        assert_eq!(config.obs_ws_rec_stop_wait, 3000);

        config.set_obs_ws_rec_stop_wait(-1);
        assert_eq!(config.obs_ws_rec_stop_wait, 0);

        config.set_obs_ws_rec_stop_wait(20000);
        assert_eq!(config.obs_ws_rec_stop_wait, 10000);
    }

    #[test]
    fn test_config_obs_scenes() {
        let mut config = Config::default();

        // Set a scene
        config.set_obs_scene("play".to_string(), Some("PlayScene".to_string()));
        assert_eq!(config.obs_scene("play"), Some(&"PlayScene".to_string()));

        // Remove with None
        config.set_obs_scene("play".to_string(), None);
        assert!(config.obs_scene("play").is_none());

        // Remove with empty string
        config.set_obs_scene("select".to_string(), Some("SelectScene".to_string()));
        config.set_obs_scene("select".to_string(), Some(String::new()));
        assert!(config.obs_scene("select").is_none());
    }

    #[test]
    fn test_config_obs_actions() {
        let mut config = Config::default();

        config.set_obs_action("play".to_string(), Some("StartRecording".to_string()));
        assert_eq!(
            config.obs_action("play"),
            Some(&"StartRecording".to_string())
        );

        config.set_obs_action("play".to_string(), None);
        assert!(config.obs_action("play").is_none());
    }

    #[test]
    fn test_config_get_override_download_url() {
        let mut config = Config::default();

        // Empty returns None
        assert!(config.override_download_url().is_none());

        // Non-empty returns Some
        config.override_download_url = "https://example.com".to_string();
        assert_eq!(config.override_download_url(), Some("https://example.com"));
    }

    #[test]
    fn test_config_webhook_getters() {
        let mut config = Config::default();
        config.webhook_option = 1;
        config.webhook_name = "MyBot".to_string();
        config.webhook_avatar = "https://example.com/avatar.png".to_string();
        config.webhook_url = vec!["https://hook.example.com".to_string()];

        assert_eq!(config.webhook_option, 1);
        assert_eq!(config.webhook_name, "MyBot");
        assert_eq!(config.webhook_avatar, "https://example.com/avatar.png");
        assert_eq!(config.webhook_url.len(), 1);
    }

    #[test]
    fn test_display_mode_default() {
        let mode = DisplayMode::default();
        assert!(matches!(mode, DisplayMode::WINDOW));
    }

    #[test]
    fn test_song_preview_default() {
        let preview = SongPreview::default();
        assert!(matches!(preview, SongPreview::LOOP));
    }
}
