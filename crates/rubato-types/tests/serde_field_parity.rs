// Integration tests verifying that serde field names match Java (libGDX Json) field names.
//
// Java's libGDX Json serializes fields using the Java field name directly.
// Rust serde renames snake_case fields to match Java's camelCase via #[serde(rename)].
// These tests verify bidirectional compatibility: Rust-serialized JSON must be
// readable by Java, and Java-serialized JSON must be readable by Rust.

use rubato_types::config::Config;
use rubato_types::player_config::PlayerConfig;

// ---------------------------------------------------------------------------
// Config: verify critical camelCase field names appear in serialized JSON
// ---------------------------------------------------------------------------

#[test]
fn config_serializes_java_field_names() {
    let config = Config::default();
    let json = serde_json::to_string_pretty(&config).unwrap();

    // DisplayConfig
    assert!(
        json.contains("\"useResolution\""),
        "Config should serialize useResolution, got: {}",
        json
    );
    assert!(
        json.contains("\"windowWidth\""),
        "Config should serialize windowWidth"
    );
    assert!(
        json.contains("\"windowHeight\""),
        "Config should serialize windowHeight"
    );
    assert!(
        json.contains("\"maxFramePerSecond\""),
        "Config should serialize maxFramePerSecond"
    );

    // PathConfig
    assert!(
        json.contains("\"tableURL\""),
        "Config should serialize tableURL (matching Java field name)"
    );

    // RenderConfig
    assert!(
        json.contains("\"bgaExpand\""),
        "Config should serialize bgaExpand"
    );
    assert!(
        json.contains("\"skinPixmapGen\""),
        "Config should serialize skinPixmapGen"
    );

    // NetworkConfig
    assert!(
        json.contains("\"enableIpfs\""),
        "Config should serialize enableIpfs"
    );
    assert!(
        json.contains("\"irSendCount\""),
        "Config should serialize irSendCount"
    );

    // IntegrationConfig: Java field is useDiscordRPC (uppercase RPC)
    assert!(
        json.contains("\"useDiscordRPC\""),
        "Config should serialize useDiscordRPC with uppercase RPC to match Java"
    );
    // Must NOT contain the lowercase variant
    assert!(
        !json.contains("\"useDiscordRpc\""),
        "Config must NOT serialize useDiscordRpc (lowercase); Java uses useDiscordRPC"
    );

    // SelectConfig
    assert!(
        json.contains("\"maxSearchBarCount\""),
        "Config should serialize maxSearchBarCount"
    );
    assert!(
        json.contains("\"songPreview\""),
        "Config should serialize songPreview"
    );

    // SelectConfig fields that stay lowercase in Java
    assert!(
        json.contains("\"folderlamp\""),
        "Config should serialize folderlamp (lowercase in Java)"
    );
    assert!(
        json.contains("\"scrolldurationlow\""),
        "Config should serialize scrolldurationlow (lowercase in Java)"
    );
}

#[test]
fn config_round_trip_preserves_all_fields() {
    let mut config = Config::default();
    config.display.vsync = true;
    config.display.max_frame_per_second = 144;
    config.display.window_width = 1920;
    config.display.window_height = 1080;
    config.integration.use_discord_rpc = true;
    config.select.skip_decide_screen = true;
    config.select.analog_scroll = false;
    config.network.ir_send_count = 10;
    config.render.skin_pixmap_gen = 8;

    let json = serde_json::to_string(&config).unwrap();
    let restored: Config = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.display.vsync, true);
    assert_eq!(restored.display.max_frame_per_second, 144);
    assert_eq!(restored.display.window_width, 1920);
    assert_eq!(restored.display.window_height, 1080);
    assert_eq!(restored.integration.use_discord_rpc, true);
    assert_eq!(restored.select.skip_decide_screen, true);
    assert_eq!(restored.select.analog_scroll, false);
    assert_eq!(restored.network.ir_send_count, 10);
    assert_eq!(restored.render.skin_pixmap_gen, 8);
}

#[test]
fn config_deserializes_from_java_field_names() {
    // Simulate a Java-produced config JSON with Java field names
    let java_json = r#"{
        "playername": "player1",
        "displaymode": "WINDOW",
        "vsync": true,
        "useResolution": true,
        "windowWidth": 1920,
        "windowHeight": 1080,
        "maxFramePerSecond": 240,
        "prepareFramePerSecond": 0,
        "folderlamp": true,
        "maxSearchBarCount": 10,
        "showNoSongExistingBar": true,
        "scrolldurationlow": 300,
        "scrolldurationhigh": 50,
        "analogScroll": true,
        "analogTicksPerScroll": 3,
        "songPreview": "LOOP",
        "cacheSkinImage": false,
        "useSongInfo": true,
        "songpath": "songdata.db",
        "bga": 0,
        "bgaExpand": 1,
        "frameskip": 1,
        "updatesong": false,
        "skinPixmapGen": 4,
        "stagefilePixmapGen": 2,
        "bannerPixmapGen": 2,
        "songResourceGen": 1,
        "enableIpfs": true,
        "ipfsurl": "https://gateway.ipfs.io/",
        "irSendCount": 5,
        "useDiscordRPC": true,
        "setClipboardScreenshot": false,
        "tableURL": ["https://example.com/table.html"]
    }"#;

    let config: Config = serde_json::from_str(java_json).unwrap();
    assert_eq!(config.playername, Some("player1".to_string()));
    assert_eq!(config.display.window_width, 1920);
    assert_eq!(config.display.window_height, 1080);
    assert_eq!(config.integration.use_discord_rpc, true);
    assert_eq!(config.network.ir_send_count, 5);
    assert_eq!(config.paths.table_url.len(), 1);
}

// ---------------------------------------------------------------------------
// PlayerConfig: verify critical camelCase field names appear in serialized JSON
// ---------------------------------------------------------------------------

#[test]
fn player_config_serializes_java_field_names() {
    let player = PlayerConfig::default();
    let json = serde_json::to_string_pretty(&player).unwrap();

    // PlaySettings
    assert!(
        json.contains("\"chartReplicationMode\""),
        "PlayerConfig should serialize chartReplicationMode"
    );
    assert!(
        json.contains("\"gaugeAutoShift\""),
        "PlayerConfig should serialize gaugeAutoShift"
    );
    assert!(
        json.contains("\"bottomShiftableGauge\""),
        "PlayerConfig should serialize bottomShiftableGauge"
    );

    // Java field is hranThresholdBPM (uppercase BPM)
    assert!(
        json.contains("\"hranThresholdBPM\""),
        "PlayerConfig should serialize hranThresholdBPM with uppercase BPM to match Java"
    );
    assert!(
        !json.contains("\"hranThresholdBpm\""),
        "PlayerConfig must NOT serialize hranThresholdBpm (lowercase); Java uses hranThresholdBPM"
    );

    // Java field is isGuideSE (uppercase SE)
    assert!(
        json.contains("\"isGuideSE\""),
        "PlayerConfig should serialize isGuideSE with uppercase SE to match Java"
    );
    assert!(
        !json.contains("\"isGuideSe\""),
        "PlayerConfig must NOT serialize isGuideSe (lowercase); Java uses isGuideSE"
    );

    // JudgeSettings
    assert!(
        json.contains("\"customJudge\""),
        "PlayerConfig should serialize customJudge"
    );
    assert!(
        json.contains("\"keyJudgeWindowRatePerfectGreat\""),
        "PlayerConfig should serialize keyJudgeWindowRatePerfectGreat"
    );

    // DisplaySettings
    assert!(
        json.contains("\"chartPreview\""),
        "PlayerConfig should serialize chartPreview"
    );
    assert!(
        json.contains("\"misslayerDuration\""),
        "PlayerConfig should serialize misslayerDuration"
    );
    assert!(
        json.contains("\"scrollMode\""),
        "PlayerConfig should serialize scrollMode"
    );

    // NoteModifierSettings
    assert!(
        json.contains("\"longnoteMode\""),
        "PlayerConfig should serialize longnoteMode"
    );
    assert!(
        json.contains("\"sevenToNinePattern\""),
        "PlayerConfig should serialize sevenToNinePattern"
    );

    // SelectSettings
    assert!(
        json.contains("\"isRandomSelect\""),
        "PlayerConfig should serialize isRandomSelect"
    );
    assert!(
        json.contains("\"isWindowHold\""),
        "PlayerConfig should serialize isWindowHold"
    );

    // MiscSettings
    assert!(
        json.contains("\"exitPressDuration\""),
        "PlayerConfig should serialize exitPressDuration"
    );

    // Twitter
    assert!(
        json.contains("\"twitterConsumerKey\""),
        "PlayerConfig should serialize twitterConsumerKey"
    );

    // Stream
    assert!(
        json.contains("\"enableRequest\""),
        "PlayerConfig should serialize enableRequest"
    );

    // SkinHistory
    assert!(
        json.contains("\"skinHistory\""),
        "PlayerConfig should serialize skinHistory"
    );
}

#[test]
fn player_config_round_trip_preserves_all_fields() {
    let player = PlayerConfig {
        id: Some("testplayer".to_string()),
        name: "TestName".to_string(),
        play_settings: rubato_types::player_config::PlaySettings {
            gauge: 3,
            random: 5,
            hran_threshold_bpm: 150,
            mine_mode: 2,
            ..Default::default()
        },
        judge_settings: rubato_types::player_config::JudgeSettings {
            judgetiming: -50,
            custom_judge: true,
            key_judge_window_rate_perfect_great: 300,
            ..Default::default()
        },
        display_settings: rubato_types::player_config::DisplaySettings {
            is_guide_se: true,
            misslayer_duration: 300,
            scroll_mode: 2,
            ..Default::default()
        },
        note_modifier_settings: rubato_types::player_config::NoteModifierSettings {
            longnote_mode: 1,
            longnote_rate: 0.8,
            ..Default::default()
        },
        select_settings: rubato_types::player_config::SelectSettings {
            is_random_select: true,
            event_mode: true,
            ..Default::default()
        },
        misc_settings: rubato_types::player_config::MiscSettings {
            exit_press_duration: 2000,
            ..Default::default()
        },
        ..Default::default()
    };

    let json = serde_json::to_string(&player).unwrap();
    let restored: PlayerConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.name, "TestName");
    assert_eq!(restored.play_settings.gauge, 3);
    assert_eq!(restored.play_settings.random, 5);
    assert_eq!(restored.play_settings.hran_threshold_bpm, 150);
    assert_eq!(restored.play_settings.mine_mode, 2);
    assert_eq!(restored.judge_settings.judgetiming, -50);
    assert_eq!(restored.judge_settings.custom_judge, true);
    assert_eq!(
        restored.judge_settings.key_judge_window_rate_perfect_great,
        300
    );
    assert_eq!(restored.display_settings.is_guide_se, true);
    assert_eq!(restored.display_settings.misslayer_duration, 300);
    assert_eq!(restored.display_settings.scroll_mode, 2);
    assert_eq!(restored.note_modifier_settings.longnote_mode, 1);
    assert_eq!(restored.note_modifier_settings.longnote_rate, 0.8);
    assert_eq!(restored.select_settings.is_random_select, true);
    assert_eq!(restored.select_settings.event_mode, true);
    assert_eq!(restored.misc_settings.exit_press_duration, 2000);
}

#[test]
fn player_config_deserializes_from_java_field_names() {
    // Simulate a Java-produced player config JSON
    let java_json = r#"{
        "id": "player1",
        "name": "TestPlayer",
        "gauge": 2,
        "random": 3,
        "random2": 0,
        "doubleoption": 0,
        "chartReplicationMode": "RIVALCHART",
        "lnmode": 0,
        "gaugeAutoShift": 1,
        "bottomShiftableGauge": 0,
        "hranThresholdBPM": 120,
        "mineMode": 0,
        "judgetiming": 0,
        "notesDisplayTimingAutoAdjust": false,
        "customJudge": false,
        "keyJudgeWindowRatePerfectGreat": 400,
        "keyJudgeWindowRateGreat": 400,
        "keyJudgeWindowRateGood": 100,
        "scratchJudgeWindowRatePerfectGreat": 400,
        "scratchJudgeWindowRateGreat": 400,
        "scratchJudgeWindowRateGood": 100,
        "bpmguide": false,
        "showjudgearea": false,
        "markprocessednote": false,
        "showhiddennote": false,
        "showpastnote": false,
        "chartPreview": true,
        "isGuideSE": true,
        "misslayerDuration": 500,
        "extranoteType": 0,
        "extranoteDepth": 0,
        "extranoteScratch": false,
        "scrollMode": 0,
        "scrollSection": 4,
        "scrollRate": 0.5,
        "longnoteMode": 0,
        "longnoteRate": 1.0,
        "sevenToNinePattern": 0,
        "sevenToNineType": 0,
        "sort": 0,
        "musicselectinput": 0,
        "isRandomSelect": false,
        "isWindowHold": false,
        "eventMode": false,
        "targetid": "MAX",
        "exitPressDuration": 1000,
        "enableRequest": false,
        "notifyRequest": false,
        "maxRequestCount": 30,
        "skinHistory": [],
        "skin": []
    }"#;

    let player: PlayerConfig = serde_json::from_str(java_json).unwrap();
    assert_eq!(player.name, "TestPlayer");
    assert_eq!(player.play_settings.gauge, 2);
    assert_eq!(player.play_settings.random, 3);
    assert_eq!(player.play_settings.gauge_auto_shift, 1);
    assert_eq!(player.play_settings.hran_threshold_bpm, 120);
    assert_eq!(player.display_settings.is_guide_se, true);
    assert_eq!(player.display_settings.misslayer_duration, 500);
    assert_eq!(player.select_settings.is_random_select, false);
    assert_eq!(player.misc_settings.exit_press_duration, 1000);
}

// ---------------------------------------------------------------------------
// PlayConfig: verify field names
// ---------------------------------------------------------------------------

#[test]
fn play_config_serializes_java_field_names() {
    let config = rubato_types::play_config::PlayConfig::default();
    let json = serde_json::to_string_pretty(&config).unwrap();

    assert!(
        json.contains("\"enableConstant\""),
        "PlayConfig should serialize enableConstant"
    );
    assert!(
        json.contains("\"constantFadeinTime\""),
        "PlayConfig should serialize constantFadeinTime"
    );
    // These fields are lowercase in Java too
    assert!(
        json.contains("\"hispeed\""),
        "PlayConfig should serialize hispeed"
    );
    assert!(
        json.contains("\"fixhispeed\""),
        "PlayConfig should serialize fixhispeed"
    );
    assert!(
        json.contains("\"enablelanecover\""),
        "PlayConfig should serialize enablelanecover"
    );
}

// ---------------------------------------------------------------------------
// SkinConfig: verify field names
// ---------------------------------------------------------------------------

#[test]
fn skin_config_round_trip() {
    let mut sc = rubato_types::skin_config::SkinConfig::default();
    sc.path = Some("skin/default/play7.json".to_string());

    let json = serde_json::to_string(&sc).unwrap();
    let restored: rubato_types::skin_config::SkinConfig = serde_json::from_str(&json).unwrap();

    assert_eq!(restored.path, Some("skin/default/play7.json".to_string()));
}
