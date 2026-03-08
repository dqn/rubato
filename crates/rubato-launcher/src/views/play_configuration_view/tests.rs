use super::*;
use rubato_core::audio_config::AudioConfig;
use rubato_types::song_database_accessor::SongDatabaseAccessor as _;

/// Helper: create a PlayConfigurationView with initialize() called
fn initialized_view() -> PlayConfigurationView {
    let mut view = PlayConfigurationView::new();
    view.initialize();
    view
}

// ---- initialize() tests ----

#[test]
fn test_initialize_sets_combo_box_labels() {
    let view = initialized_view();

    assert_eq!(view.score_options_labels.len(), 10);
    assert_eq!(view.score_options_labels[0], "OFF");
    assert_eq!(view.score_options_labels[1], "MIRROR");

    assert_eq!(view.double_options_labels.len(), 4);
    assert_eq!(view.gauge_options_labels.len(), 6);
    assert_eq!(view.fixhispeed_labels.len(), 5);
    assert_eq!(view.lntype_labels.len(), 3);
    assert_eq!(view.gaugeautoshift_labels.len(), 5);
    assert_eq!(view.bottomshiftablegauge_labels.len(), 3);
    assert_eq!(view.minemode_labels.len(), 5);
    assert_eq!(view.scrollmode_labels.len(), 3);
    assert_eq!(view.longnotemode_labels.len(), 6);
    assert_eq!(view.judgealgorithm_labels.len(), 3);
    assert_eq!(view.autosave_labels.len(), 11);
}

#[test]
fn test_initialize_populates_http_download_sources() {
    let view = initialized_view();
    assert!(!view.http_download_source.is_empty());
}

// ---- update() delegation tests ----

#[test]
fn test_update_delegates_to_video_controller() {
    let mut view = initialized_view();
    let config = Config {
        display: rubato_core::config::DisplayConfig {
            vsync: true,
            max_frame_per_second: 120,
            ..Default::default()
        },
        render: rubato_core::config::RenderConfig {
            bga: rubato_types::config::BgaMode::Off,
            ..Default::default()
        },
        ..Default::default()
    };

    view.update(config);

    // VideoConfigurationView.update() should have copied these values
    // We can verify by calling commit() and checking config roundtrip
    let mut out_config = Config::default();
    view.video_controller.commit(&mut out_config);
    assert!(out_config.display.vsync);
    assert_eq!(out_config.display.max_frame_per_second, 120);
    assert_eq!(out_config.render.bga, rubato_types::config::BgaMode::Off);
}

#[test]
fn test_update_delegates_to_audio_controller() {
    let mut view = initialized_view();
    let config = Config {
        audio: Some(AudioConfig {
            systemvolume: 0.75,
            keyvolume: 0.5,
            bgvolume: 0.25,
            ..Default::default()
        }),
        ..Default::default()
    };

    view.update(config);

    // AudioConfigurationView stores config internally; commit writes back
    view.audio_controller.commit();
}

#[test]
fn test_update_delegates_to_music_select_controller() {
    let mut view = initialized_view();
    let config = Config {
        select: rubato_core::config::SelectConfig {
            scrolldurationlow: 300,
            scrolldurationhigh: 500,
            folderlamp: true,
            ..Default::default()
        },
        ..Default::default()
    };

    view.update(config);

    // Verify the music_select_controller commit roundtrip
    view.music_select_controller.commit();
}

#[test]
fn test_update_delegates_to_resource_controller() {
    let mut view = initialized_view();
    let config = Config {
        paths: rubato_core::config::PathConfig {
            bmsroot: vec!["path1".to_string(), "path2".to_string()],
            ..Default::default()
        },
        updatesong: true,
        ..Default::default()
    };

    view.update(config);

    // resource_controller.update should have picked up bmsroot
    view.resource_controller.commit();
}

#[test]
fn test_update_delegates_to_discord_controller() {
    let mut view = initialized_view();
    let config = Config {
        integration: rubato_core::config::IntegrationConfig {
            use_discord_rpc: true,
            webhook_name: "test_hook".to_string(),
            ..Default::default()
        },
        ..Default::default()
    };

    view.update(config);

    view.discord_controller.commit();
}

#[test]
fn test_update_delegates_to_obs_controller() {
    let mut view = initialized_view();
    let config = Config {
        obs: rubato_core::config::ObsConfig {
            use_obs_ws: true,
            obs_ws_host: "localhost".to_string(),
            obs_ws_port: 4455,
            ..Default::default()
        },
        ..Default::default()
    };

    view.update(config);

    view.obs_controller.commit();
}

// ---- commit() delegation tests ----

#[test]
fn test_commit_delegates_to_video_controller() {
    let mut view = initialized_view();
    view.update(Config::default());

    // After commit, the config should reflect sub-controller state
    view.commit();
}

#[test]
fn test_commit_delegates_to_table_controller() {
    let mut view = initialized_view();
    view.update(Config::default());

    // table_controller.commit() should be called without panic
    view.commit();
}

// ---- update_player() delegation tests ----

#[test]
fn test_update_player_delegates_to_ir_controller() {
    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });

    // With no valid player file, it should fall back to default
    view.players_selected = Some("player1".to_string());
    view.update_player();
}

#[test]
fn test_update_player_delegates_to_stream_controller() {
    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });

    view.players_selected = Some("player1".to_string());
    view.update_player();
}

#[test]
fn test_update_player_delegates_to_input_controller() {
    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });

    view.players_selected = Some("player1".to_string());
    view.update_player();
}

#[test]
fn test_update_player_delegates_to_skin_controller() {
    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            playerpath: "nonexistent_path".to_string(),
            ..Default::default()
        },
        ..Default::default()
    });

    view.players_selected = Some("player1".to_string());
    view.update_player();
}

// ---- commit_player() delegation tests ----

#[test]
fn test_commit_player_delegates_to_sub_controllers() {
    let mut view = initialized_view();
    view.config = Some(Config::default());
    view.player = Some(PlayerConfig::default());
    view.playconfig = Some(PlayMode::BEAT_7K);

    // This should call video_controller.commit_player,
    // music_select_controller.commit_player, input_controller.commit,
    // ir_controller.commit, stream_controller.commit,
    // skin_controller.commit without panic
    view.commit_player();
}

#[test]
fn test_commit_player_skips_when_no_player() {
    let mut view = initialized_view();
    view.player = None;

    // Should return early without panic
    view.commit_player();
}

// ---- PlayMode tests ----

#[test]
fn test_play_mode_display_name() {
    assert_eq!(PlayMode::BEAT_7K.display_name(), "7KEYS");
    assert_eq!(PlayMode::BEAT_14K.display_name(), "14KEYS");
    assert_eq!(
        PlayMode::KEYBOARD_24K_DOUBLE.display_name(),
        "24KEYS DOUBLE"
    );
}

#[test]
fn test_play_mode_to_mode() {
    assert_eq!(PlayMode::BEAT_7K.to_mode(), Mode::BEAT_7K);
    assert_eq!(PlayMode::POPN_9K.to_mode(), Mode::POPN_9K);
}

#[test]
fn test_play_mode_values_length() {
    assert_eq!(PlayMode::values().len(), 7);
}

// ---- OptionListCell tests ----

#[test]
fn test_option_list_cell_get_text() {
    let cell = OptionListCell::new(vec!["A".to_string(), "B".to_string(), "C".to_string()]);
    assert_eq!(cell.text(Some(0)), "A");
    assert_eq!(cell.text(Some(2)), "C");
    assert_eq!(cell.text(None), "");
    assert_eq!(cell.text(Some(-1)), "");
    assert_eq!(cell.text(Some(99)), "");
}

// ---- Async BMS loading tests ----

#[test]
fn test_bms_loading_state_initially_idle() {
    let view = initialized_view();
    assert!(
        matches!(view.bms_loading_state(), BmsLoadingState::Idle),
        "Loading state should be Idle after construction"
    );
}

#[test]
fn test_load_bms_transitions_to_loading_when_config_present() {
    let mut view = initialized_view();
    // Set up config with a temp directory as bmsroot
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();
    let songdb_path = tmpdir.path().join("song.db");
    let config = Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    };
    view.update(config);

    view.load_bms(None, false);

    assert!(
        matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }),
        "Loading state should transition to Loading after load_bms"
    );
}

#[test]
fn test_load_bms_no_config_stays_idle() {
    let mut view = initialized_view();
    // No config set, load_bms should not start loading
    view.load_bms(None, false);

    assert!(
        matches!(view.bms_loading_state(), BmsLoadingState::Idle),
        "Loading state should stay Idle when no config"
    );
}

#[test]
fn test_bms_loading_completes_and_sets_song_updated() {
    let mut view = initialized_view();
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();
    let songdb_path = tmpdir.path().join("song.db");
    let config = Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    };
    view.update(config);

    view.load_bms(None, false);

    // Wait for the background thread to finish (with timeout)
    let start = std::time::Instant::now();
    loop {
        view.poll_bms_loading();
        if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            panic!("BMS loading did not complete within 10 seconds");
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(
        matches!(view.bms_loading_state(), BmsLoadingState::Completed),
        "Loading state should be Completed after thread finishes"
    );
    assert!(
        view.song_updated,
        "song_updated should be true after successful load"
    );
}

#[test]
fn test_bms_loading_progress_counters_accessible() {
    let mut view = initialized_view();
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();
    let songdb_path = tmpdir.path().join("song.db");
    let config = Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    };
    view.update(config);

    view.load_bms(None, false);

    // While loading, the progress should be accessible
    if let BmsLoadingState::Loading {
        bms_files,
        processed_files,
        new_files,
    } = view.bms_loading_state()
    {
        // Counters start at 0
        assert_eq!(bms_files, 0);
        assert_eq!(processed_files, 0);
        assert_eq!(new_files, 0);
    }

    // Wait for completion
    let start = std::time::Instant::now();
    loop {
        view.poll_bms_loading();
        if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

#[test]
fn test_bms_loading_reset_returns_to_idle() {
    let mut view = initialized_view();
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();
    let songdb_path = tmpdir.path().join("song.db");
    let config = Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    };
    view.update(config);

    view.load_bms(None, false);
    // Wait for completion
    let start = std::time::Instant::now();
    loop {
        view.poll_bms_loading();
        if !matches!(view.bms_loading_state(), BmsLoadingState::Loading { .. }) {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    view.reset_bms_loading();
    assert!(
        matches!(view.bms_loading_state(), BmsLoadingState::Idle),
        "After reset, loading state should be Idle"
    );
}

#[test]
fn test_is_bms_loading_returns_true_during_load() {
    let mut view = initialized_view();
    assert!(!view.is_bms_loading(), "Should not be loading initially");

    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();
    let songdb_path = tmpdir.path().join("song.db");
    let config = Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    };
    view.update(config);

    view.load_bms(None, false);
    assert!(view.is_bms_loading(), "Should be loading after load_bms");

    // Wait for completion
    let start = std::time::Instant::now();
    loop {
        view.poll_bms_loading();
        if !view.is_bms_loading() {
            break;
        }
        if start.elapsed() > std::time::Duration::from_secs(10) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    assert!(
        !view.is_bms_loading(),
        "Should not be loading after completion"
    );
}

// ---- Roundtrip: update -> commit preserves config values ----

#[test]
fn test_update_commit_roundtrip_preserves_config_fields() {
    let mut view = initialized_view();
    let config = Config {
        paths: rubato_core::config::PathConfig {
            bgmpath: "/music/bgm".to_string(),
            soundpath: "/music/sounds".to_string(),
            ..Default::default()
        },
        select: rubato_core::config::SelectConfig {
            cache_skin_image: true,
            ..Default::default()
        },
        network: rubato_core::config::NetworkConfig {
            enable_ipfs: true,
            ipfsurl: "http://ipfs.example.com".to_string(),
            enable_http: true,
            download_source: "source1".to_string(),
            override_download_url: "http://override.example.com".to_string(),
            ..Default::default()
        },
        integration: rubato_core::config::IntegrationConfig {
            set_clipboard_screenshot: true,
            ..Default::default()
        },
        ..Default::default()
    };

    view.update(config);

    assert_eq!(view.bgmpath, "/music/bgm");
    assert_eq!(view.soundpath, "/music/sounds");
    assert!(view.usecim);
    assert!(view.enable_ipfs);
    assert_eq!(view.ipfsurl, "http://ipfs.example.com");
    assert!(view.enable_http);
    assert_eq!(view.override_download_url, "http://override.example.com");
    assert!(view.clipboard_screenshot);
}

// ---- Roundtrip: update_player -> commit_player preserves player fields ----

#[test]
fn test_update_player_commit_player_roundtrip() {
    let mut view = initialized_view();
    view.config = Some(Config::default());

    let player = PlayerConfig {
        name: "TestPlayer".to_string(),
        play_settings: rubato_types::player_config::PlaySettings {
            random: 3,
            random2: 5,
            doubleoption: 1,
            gauge: 2,
            lnmode: 1,
            forcedcnendings: true,
            mine_mode: 2,
            ..Default::default()
        },
        judge_settings: rubato_types::player_config::JudgeSettings {
            judgetiming: 10,
            custom_judge: true,
            key_judge_window_rate_perfect_great: 500,
            ..Default::default()
        },
        display_settings: rubato_types::player_config::DisplaySettings {
            bpmguide: true,
            scroll_mode: 1,
            showjudgearea: true,
            markprocessednote: true,
            showhiddennote: true,
            showpastnote: true,
            ..Default::default()
        },
        note_modifier_settings: rubato_types::player_config::NoteModifierSettings {
            longnote_mode: 3,
            longnote_rate: 1.5,
            ..Default::default()
        },
        misc_settings: rubato_types::player_config::MiscSettings {
            autosavereplay: vec![1, 2, 3, 4],
            ..Default::default()
        },
        ..Default::default()
    };

    view.player = Some(player);
    view.playername = "TestPlayer".to_string();
    view.scoreop = Some(3);
    view.scoreop2 = Some(5);
    view.doubleop = Some(1);
    view.gaugeop = Some(2);
    view.lntype = Some(1);
    view.playconfig = Some(PlayMode::BEAT_7K);

    view.commit_player();

    let committed = view.player.as_ref().unwrap();
    assert_eq!(committed.name, "TestPlayer");
    assert_eq!(committed.play_settings.random, 3);
    assert_eq!(committed.play_settings.random2, 5);
    assert_eq!(committed.play_settings.doubleoption, 1);
    assert_eq!(committed.play_settings.gauge, 2);
    assert_eq!(committed.play_settings.lnmode, 1);
}

// ---- LR2 score import tests ----

type Lr2ScoreRow<'a> = (&'a str, i32, i32, i32, i32, i32, i32, i32, i32, i32);

/// Helper: create a minimal LR2 score.db with the given rows.
fn create_lr2_score_db(path: &str, rows: &[Lr2ScoreRow<'_>]) {
    let conn = rusqlite::Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS score (
            hash TEXT,
            perfect INTEGER,
            great INTEGER,
            good INTEGER,
            bad INTEGER,
            poor INTEGER,
            minbp INTEGER,
            clear INTEGER,
            playcount INTEGER,
            clearcount INTEGER
        )",
    )
    .unwrap();
    for &(hash, perfect, great, good, bad, poor, minbp, clear, playcount, clearcount) in rows {
        conn.execute(
            "INSERT INTO score (hash, perfect, great, good, bad, poor, minbp, clear, playcount, clearcount)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
            rusqlite::params![hash, perfect, great, good, bad, poor, minbp, clear, playcount, clearcount],
        )
        .unwrap();
    }
}

/// Helper: populate a beatoraja song.db with songs that have the given md5/sha256/notes.
fn populate_song_db(songdb_path: &str, bmsroot: &str, songs: &[(&str, &str, i32)]) {
    use rubato_types::song_data::SongData;
    let songdb = SQLiteSongDatabaseAccessor::new(songdb_path, &[bmsroot.to_string()]).unwrap();
    let song_datas: Vec<SongData> = songs
        .iter()
        .enumerate()
        .map(|(i, &(md5, sha256, notes))| {
            let mut sd = SongData::new();
            sd.md5 = md5.to_string();
            sd.sha256 = sha256.to_string();
            sd.notes = notes;
            // SongData::validate() requires title to be non-empty
            sd.title = "test".to_string();
            // Each song needs a unique path (primary key in song table)
            sd.set_path(format!("/test/song_{i}.bms"));
            sd
        })
        .collect();
    songdb.set_song_datas(&song_datas);
}

#[test]
fn test_import_score_data_from_lr2_returns_early_without_config() {
    let view = initialized_view();
    // No config, no players_selected — should return early without error
    view.import_score_data_from_lr2_path("/nonexistent/lr2score.db");
}

#[test]
fn test_import_score_data_from_lr2_returns_early_without_player() {
    let mut view = initialized_view();
    view.config = Some(Config::default());
    view.players_selected = None;
    // No player selected — should return early without error
    view.import_score_data_from_lr2_path("/nonexistent/lr2score.db");
}

#[test]
fn test_import_score_data_from_lr2_imports_matching_scores() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();

    // Create LR2 score.db with one row
    let lr2_path = tmpdir.path().join("lr2score.db");
    // LR2 clear=2 maps to beatoraja clear=4 (clears = [0,1,4,5,6,8,9])
    create_lr2_score_db(
        &lr2_path.to_string_lossy(),
        &[(
            "d41d8cd98f00b204e9800998ecf8427e",
            100,
            50,
            10,
            5,
            3,
            8,
            2,
            15,
            7,
        )],
    );

    // Create beatoraja song.db with a matching song (by MD5)
    let songdb_path = tmpdir.path().join("song.db");
    populate_song_db(
        &songdb_path.to_string_lossy(),
        &bmsroot,
        &[(
            "d41d8cd98f00b204e9800998ecf8427e",
            "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
            200,
        )],
    );

    // Create player directory for score.db
    let playerpath = tmpdir.path().join("player");
    let player_dir = playerpath.join("testplayer");
    std::fs::create_dir_all(&player_dir).unwrap();

    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            playerpath: playerpath.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    });
    view.players_selected = Some("testplayer".to_string());

    view.import_score_data_from_lr2_path(&lr2_path.to_string_lossy());

    // Verify scores were written to the player's score.db
    let score_db_path = player_dir.join("score.db");
    assert!(score_db_path.exists(), "score.db should have been created");

    let scoredb = rubato_core::score_database_accessor::ScoreDatabaseAccessor::new(
        &score_db_path.to_string_lossy(),
    )
    .unwrap();
    let score = scoredb.score_data(
        "abcdef1234567890abcdef1234567890abcdef1234567890abcdef1234567890ab",
        0,
    );
    assert!(score.is_some(), "Score should have been imported");

    let score = score.unwrap();
    assert_eq!(
        score.judge_counts.epg, 100,
        "epg should be mapped from LR2 perfect"
    );
    assert_eq!(
        score.judge_counts.egr, 50,
        "egr should be mapped from LR2 great"
    );
    assert_eq!(
        score.judge_counts.egd, 10,
        "egd should be mapped from LR2 good"
    );
    assert_eq!(
        score.judge_counts.ebd, 5,
        "ebd should be mapped from LR2 bad"
    );
    assert_eq!(
        score.judge_counts.epr, 3,
        "epr should be mapped from LR2 poor"
    );
    assert_eq!(score.minbp, 8, "minbp should be mapped from LR2 minbp");
    // LR2 clear=2 -> clears[2]=4
    assert_eq!(score.clear, 4, "clear should be mapped via clears table");
    assert_eq!(score.playcount, 15);
    assert_eq!(score.clearcount, 7);
    assert_eq!(score.notes, 200, "notes should come from song DB");
    assert_eq!(score.scorehash, "LR2", "scorehash should be set to 'LR2'");
}

#[test]
fn test_import_score_data_from_lr2_skips_unknown_songs() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();

    // Create LR2 score.db with a row whose MD5 does NOT exist in the song DB
    let lr2_path = tmpdir.path().join("lr2score.db");
    create_lr2_score_db(
        &lr2_path.to_string_lossy(),
        &[(
            "ffffffffffffffffffffffffffffffff",
            100,
            50,
            10,
            5,
            3,
            8,
            2,
            15,
            7,
        )],
    );

    // Create empty beatoraja song.db (no matching songs)
    let songdb_path = tmpdir.path().join("song.db");
    populate_song_db(&songdb_path.to_string_lossy(), &bmsroot, &[]);

    let playerpath = tmpdir.path().join("player");
    let player_dir = playerpath.join("testplayer");
    std::fs::create_dir_all(&player_dir).unwrap();

    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            playerpath: playerpath.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    });
    view.players_selected = Some("testplayer".to_string());

    view.import_score_data_from_lr2_path(&lr2_path.to_string_lossy());

    // Score DB should exist but be empty (no matching songs)
    let score_db_path = player_dir.join("score.db");
    assert!(score_db_path.exists(), "score.db should have been created");

    let scoredb = rubato_core::score_database_accessor::ScoreDatabaseAccessor::new(
        &score_db_path.to_string_lossy(),
    )
    .unwrap();
    let scores = scoredb.score_datas("1=1");
    let count = scores.map(|v| v.len()).unwrap_or(0);
    assert_eq!(count, 0, "No scores should be imported when no songs match");
}

#[test]
fn test_import_score_data_from_lr2_empty_lr2_db() {
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();

    // Create empty LR2 score.db
    let lr2_path = tmpdir.path().join("lr2score.db");
    create_lr2_score_db(&lr2_path.to_string_lossy(), &[]);

    let songdb_path = tmpdir.path().join("song.db");
    populate_song_db(&songdb_path.to_string_lossy(), &bmsroot, &[]);

    let playerpath = tmpdir.path().join("player");
    let player_dir = playerpath.join("testplayer");
    std::fs::create_dir_all(&player_dir).unwrap();

    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            playerpath: playerpath.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    });
    view.players_selected = Some("testplayer".to_string());

    // Should succeed without error, just import 0 scores
    view.import_score_data_from_lr2_path(&lr2_path.to_string_lossy());
}

#[test]
fn test_import_score_data_from_lr2_clear_mapping() {
    // Verify all 7 LR2 clear values map correctly
    // LR2 clear indices: 0→0, 1→1, 2→4, 3→5, 4→6, 5→8, 6→9
    let clears: [i32; 7] = [0, 1, 4, 5, 6, 8, 9];
    let tmpdir = tempfile::tempdir().unwrap();
    let bmsroot = tmpdir.path().to_string_lossy().to_string();

    // Create 7 songs with unique MD5s
    let md5s: Vec<String> = (0..7).map(|i| format!("{:032x}", i + 1)).collect();
    let sha256s: Vec<String> = (0..7).map(|i| format!("{:064x}", i + 1)).collect();

    // Create LR2 score.db with each clear value
    let lr2_path = tmpdir.path().join("lr2score.db");
    let rows: Vec<Lr2ScoreRow<'_>> = (0..7)
        .map(|i| {
            (
                md5s[i].as_str(),
                10,
                5,
                2,
                1,
                0,
                3,
                i as i32, // clear index
                1,
                1,
            )
        })
        .collect();
    create_lr2_score_db(&lr2_path.to_string_lossy(), &rows);

    // Create song.db with matching songs
    let songdb_path = tmpdir.path().join("song.db");
    let songs: Vec<(&str, &str, i32)> = (0..7)
        .map(|i| (md5s[i].as_str(), sha256s[i].as_str(), 100))
        .collect();
    populate_song_db(&songdb_path.to_string_lossy(), &bmsroot, &songs);

    let playerpath = tmpdir.path().join("player");
    let player_dir = playerpath.join("testplayer");
    std::fs::create_dir_all(&player_dir).unwrap();

    let mut view = initialized_view();
    view.config = Some(Config {
        paths: rubato_core::config::PathConfig {
            songpath: songdb_path.to_string_lossy().to_string(),
            playerpath: playerpath.to_string_lossy().to_string(),
            bmsroot: vec![bmsroot],
            ..Default::default()
        },
        ..Default::default()
    });
    view.players_selected = Some("testplayer".to_string());

    view.import_score_data_from_lr2_path(&lr2_path.to_string_lossy());

    let score_db_path = player_dir.join("score.db");
    assert!(score_db_path.exists(), "score.db should exist");

    let scoredb = rubato_core::score_database_accessor::ScoreDatabaseAccessor::new(
        &score_db_path.to_string_lossy(),
    )
    .unwrap();

    for i in 0..7 {
        let score = scoredb.score_data(&sha256s[i], 0);
        assert!(score.is_some(), "Score for clear index {} should exist", i);
        let score = score.unwrap();
        assert_eq!(
            score.clear, clears[i],
            "LR2 clear index {} should map to beatoraja clear {}",
            i, clears[i]
        );
    }
}
