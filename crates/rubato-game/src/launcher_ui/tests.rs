use super::tabs::clamped_option_index;
use super::*;

// -- clamped_option_index --

#[test]
fn clamped_option_index_valid_values() {
    assert_eq!(clamped_option_index(0, 3), 0);
    assert_eq!(clamped_option_index(1, 3), 1);
    assert_eq!(clamped_option_index(2, 3), 2);
}

#[test]
fn clamped_option_index_negative_falls_back_to_zero() {
    assert_eq!(clamped_option_index(-1, 3), 0);
    assert_eq!(clamped_option_index(-100, 3), 0);
    assert_eq!(clamped_option_index(i32::MIN, 3), 0);
}

#[test]
fn clamped_option_index_out_of_bounds_falls_back_to_zero() {
    assert_eq!(clamped_option_index(3, 3), 0);
    assert_eq!(clamped_option_index(100, 3), 0);
    assert_eq!(clamped_option_index(i32::MAX, 3), 0);
}

#[test]
fn clamped_option_index_empty_array_falls_back_to_zero() {
    assert_eq!(clamped_option_index(0, 0), 0);
}

// -- LauncherUi tests --

#[test]
fn test_launcher_ui_new_defaults() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let ui = LauncherUi::new(config, player);

    assert!(!ui.is_play_requested());
    assert!(!ui.exit_requested);
    assert_eq!(ui.selected_tab, Tab::Option);
    assert_eq!(ui.selected_play_mode, 1); // BEAT_7K
}

#[test]
fn test_launcher_ui_config_accessors() {
    let mut config = Config::default();
    config.display.vsync = true;
    config.display.max_frame_per_second = 120;
    let player = PlayerConfig::default();
    let ui = LauncherUi::new(config, player);

    assert!(ui.config().display.vsync);
    assert_eq!(ui.config().display.max_frame_per_second, 120);
}

#[test]
fn test_launcher_ui_player_accessor() {
    let config = Config::default();
    let mut player = PlayerConfig::default();
    player.name = "test_player".to_string();
    let ui = LauncherUi::new(config, player);

    assert_eq!(ui.player().name, "test_player");
}

#[test]
fn test_play_requested_initially_false() {
    let ui = LauncherUi::new(Config::default(), PlayerConfig::default());
    assert!(!ui.is_play_requested());
}

#[test]
fn test_tab_all_returns_11_tabs() {
    // Java: PlayConfigurationView has 11 tabs
    assert_eq!(Tab::all().len(), 11);
}

#[test]
fn test_tab_labels_non_empty() {
    for tab in Tab::all() {
        assert!(!tab.label().is_empty());
    }
}

// ============================================================
// Finding 3: bms_paths populated from config and written back
// ============================================================

#[test]
fn test_bms_paths_populated_from_config_bmsroot() {
    let mut config = Config::default();
    config.paths.bmsroot = vec!["/path/to/bms1".to_string(), "/path/to/bms2".to_string()];
    let player = PlayerConfig::default();
    let ui = LauncherUi::new(config, player);

    assert_eq!(
        ui.bms_paths,
        vec!["/path/to/bms1".to_string(), "/path/to/bms2".to_string()],
        "bms_paths must be populated from config.paths.bmsroot on construction"
    );
}

#[test]
fn test_bms_paths_empty_when_config_bmsroot_empty() {
    let config = Config::default();
    let player = PlayerConfig::default();
    let ui = LauncherUi::new(config, player);

    assert!(
        ui.bms_paths.is_empty(),
        "bms_paths must be empty when config.paths.bmsroot is empty"
    );
}

#[test]
fn test_commit_config_writes_bms_paths_back_to_config() {
    let mut config = Config::default();
    // Use a temp dir for player path to avoid write errors
    let tmpdir = std::env::temp_dir().join(format!(
        "rubato-bms-paths-test-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    config.paths.playerpath = tmpdir.to_string_lossy().into_owned();
    config.playername = Some("test-bms-paths".to_string());

    let player = PlayerConfig::default();
    let mut ui = LauncherUi::new(config, player);
    ui.bms_paths = vec!["/new/path/a".to_string(), "/new/path/b".to_string()];

    ui.commit_config();

    assert_eq!(
        ui.config.paths.bmsroot,
        vec!["/new/path/a".to_string(), "/new/path/b".to_string()],
        "commit_config() must write bms_paths back to config.paths.bmsroot"
    );
}
