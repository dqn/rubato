// Integration tests for launcher UI component wiring.
//
// Verifies:
// - PlayConfigurationView exit sets flag instead of calling process::exit
// - PlayConfigurationView defaults are initialized correctly
// - LauncherUi preserves Config fields through construction
// - LauncherUi preserves PlayerConfig fields through construction
// - LauncherUi play_requested is initially false

use rubato_core::config::Config;
use rubato_core::player_config::PlayerConfig;
use rubato_launcher::launcher_ui::LauncherUi;
use rubato_launcher::play_configuration_view::PlayConfigurationView;

#[test]
fn play_configuration_view_exit_sets_flag() {
    let mut view = PlayConfigurationView::new();
    assert!(!view.exit_requested);

    view.exit();

    assert!(view.exit_requested);
}

#[test]
fn play_configuration_view_new_defaults() {
    let view = PlayConfigurationView::new();

    assert!(!view.exit_requested);
    assert_eq!(view.hispeed, 1.0);
    assert_eq!(view.playername, "");
    assert!(view.players.is_empty());
}

#[test]
fn launcher_ui_preserves_config() {
    let mut config = Config::default();
    config.display.vsync = true;
    config.display.max_frame_per_second = 120;
    let player = PlayerConfig::default();

    let ui = LauncherUi::new(config, player);

    assert!(ui.config().display.vsync);
    assert_eq!(ui.config().display.max_frame_per_second, 120);
}

#[test]
fn launcher_ui_preserves_player() {
    let config = Config::default();
    let mut player = PlayerConfig::default();
    player.name = "test_player".to_string();

    let ui = LauncherUi::new(config, player);

    assert_eq!(ui.player().name, "test_player");
}

#[test]
fn launcher_ui_play_requested_initially_false() {
    let ui = LauncherUi::new(Config::default(), PlayerConfig::default());

    assert!(!ui.is_play_requested());
}
