use bms_model::mode::Mode;

use super::imgui_notify::{ImGuiNotify, NOTIFICATION_POSITIONS};
use super::stubs::{Config, MainController, MainControllerAccess, PlayConfig, read_all_player_id};

use std::sync::Mutex;

static MAIN: Mutex<Option<MainController>> = Mutex::new(None);
static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

// Some of the settings are based on play mode
// WARN: PLAY_MODE_VALUE has an initial value, 1 -> BEAT_7K
static PLAY_MODE_VALUE: Mutex<i32> = Mutex::new(1);
static CURRENT_PLAY_MODE: Mutex<Option<Mode>> = Mutex::new(None);

static NOTIFICATION_POSITION: Mutex<i32> = Mutex::new(0);
static ENABLE_LIFT: Mutex<bool> = Mutex::new(false);
static LIFT_VALUE: Mutex<i32> = Mutex::new(0);
static ENABLE_HIDDEN: Mutex<bool> = Mutex::new(false);
static HIDDEN_VALUE: Mutex<i32> = Mutex::new(0);
static ENABLE_LANECOVER: Mutex<bool> = Mutex::new(false);
static LANECOVER_VALUE: Mutex<i32> = Mutex::new(0);
static LANE_COVER_MARGIN_LOW: Mutex<f32> = Mutex::new(0.0);
static LANE_COVER_MARGIN_HIGH: Mutex<f32> = Mutex::new(0.0);
static LANE_COVER_SWITCH_DURATION: Mutex<i32> = Mutex::new(0);
static ENABLE_CONSTANT: Mutex<bool> = Mutex::new(false);
static CONSTANT_VALUE: Mutex<i32> = Mutex::new(0);
#[allow(dead_code)]
static PROFILE_SWITCHER: Mutex<bool> = Mutex::new(false);
static SELECTED_PLAYER: Mutex<i32> = Mutex::new(0);
static PLAYERS: Mutex<Vec<String>> = Mutex::new(Vec::new());

fn get_play_mode_options() -> Vec<String> {
    let modes = [
        Mode::BEAT_5K,
        Mode::BEAT_7K,
        Mode::BEAT_10K,
        Mode::BEAT_14K,
        Mode::POPN_5K,
        Mode::POPN_9K,
        Mode::KEYBOARD_24K,
        Mode::KEYBOARD_24K_DOUBLE,
    ];
    modes.iter().map(|m| m.hint().to_string()).collect()
}

pub struct MiscSettingMenu;

impl MiscSettingMenu {
    pub fn set_main(main: MainController) {
        let config = main.config().clone();
        let players = read_all_player_id("player");
        let player_idx = players
            .iter()
            .position(|p| config.playername().is_some_and(|name| p == name))
            .unwrap_or(0);

        *PLAYERS.lock().unwrap() = players;
        *SELECTED_PLAYER.lock().unwrap() = player_idx as i32;
        *CONFIG.lock().unwrap() = Some(config);
        *MAIN.lock().unwrap() = Some(main);
    }

    /// Render the misc settings window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        {
            let mode = CURRENT_PLAY_MODE.lock().unwrap();
            if mode.is_none() {
                drop(mode);
                change_play_mode(&Mode::BEAT_7K);
            }
        }

        let mut open = true;
        egui::Window::new("Misc Settings")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                // Notification position
                let mut pos = *NOTIFICATION_POSITION.lock().unwrap();
                let pos_text = NOTIFICATION_POSITIONS
                    .get(pos as usize)
                    .copied()
                    .unwrap_or("TopLeft");
                egui::ComboBox::from_label("Notification Positions")
                    .selected_text(pos_text)
                    .show_ui(ui, |ui| {
                        for (i, name) in NOTIFICATION_POSITIONS.iter().enumerate() {
                            if ui.selectable_value(&mut pos, i as i32, *name).clicked() {
                                *NOTIFICATION_POSITION.lock().unwrap() = pos;
                                ImGuiNotify::set_notification_position(pos as usize);
                            }
                        }
                    });

                // Play mode selector
                let play_mode_options = get_play_mode_options();
                let mut idx = *PLAY_MODE_VALUE.lock().unwrap();
                let mode_text = play_mode_options
                    .get(idx as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("BEAT_7K");
                egui::ComboBox::from_label("Play Mode")
                    .selected_text(mode_text)
                    .show_ui(ui, |ui| {
                        for (i, option) in play_mode_options.iter().enumerate() {
                            if ui
                                .selectable_value(&mut idx, i as i32, option.as_str())
                                .clicked()
                            {
                                *PLAY_MODE_VALUE.lock().unwrap() = idx;
                                if let Some(mode) = Mode::from_hint(&play_mode_options[i]) {
                                    change_play_mode(&mode);
                                }
                            }
                        }
                    });

                ui.separator();

                // Lane cover / Hidden / Lift / Constant settings
                let mut lift_enabled = *ENABLE_LIFT.lock().unwrap();
                ui.checkbox(&mut lift_enabled, "Enable Lift");
                *ENABLE_LIFT.lock().unwrap() = lift_enabled;
                if lift_enabled {
                    let mut lift_val = *LIFT_VALUE.lock().unwrap();
                    ui.add(egui::Slider::new(&mut lift_val, 0..=1000).text("Lift"));
                    *LIFT_VALUE.lock().unwrap() = lift_val;
                }

                let mut hidden_enabled = *ENABLE_HIDDEN.lock().unwrap();
                ui.checkbox(&mut hidden_enabled, "Enable Hidden");
                *ENABLE_HIDDEN.lock().unwrap() = hidden_enabled;
                if hidden_enabled {
                    let mut hidden_val = *HIDDEN_VALUE.lock().unwrap();
                    ui.add(egui::Slider::new(&mut hidden_val, 0..=1000).text("Hidden"));
                    *HIDDEN_VALUE.lock().unwrap() = hidden_val;
                }

                let mut lc_enabled = *ENABLE_LANECOVER.lock().unwrap();
                ui.checkbox(&mut lc_enabled, "Enable Lane Cover");
                *ENABLE_LANECOVER.lock().unwrap() = lc_enabled;
                if lc_enabled {
                    let mut lc_val = *LANECOVER_VALUE.lock().unwrap();
                    ui.add(egui::Slider::new(&mut lc_val, 0..=1000).text("Lane Cover"));
                    *LANECOVER_VALUE.lock().unwrap() = lc_val;
                }

                let mut constant = *ENABLE_CONSTANT.lock().unwrap();
                ui.checkbox(&mut constant, "Enable Constant");
                *ENABLE_CONSTANT.lock().unwrap() = constant;
                if constant {
                    let mut constant_val = *CONSTANT_VALUE.lock().unwrap();
                    ui.add(
                        egui::Slider::new(&mut constant_val, 0..=5000).text("Fade-in Time (ms)"),
                    );
                    *CONSTANT_VALUE.lock().unwrap() = constant_val;
                }

                ui.separator();

                // Profile switcher
                profile_switcher_ui(ui);
            });
    }
}

/// Get current play mode(5k, 7k...) config, a simple wrapper around MainController
fn get_play_config() -> PlayConfig {
    let main = MAIN.lock().unwrap();
    if let Some(ref m) = *main {
        let mode = CURRENT_PLAY_MODE.lock().unwrap();
        if let Some(ref mode) = *mode {
            let mut player_config = m.player_config().clone();
            let play_mode_config = player_config.play_config(mode.clone());
            return play_mode_config.playconfig.clone();
        }
    }
    PlayConfig::default()
}

/// Change current play mode, resetting related options
fn change_play_mode(mode: &Mode) {
    *CURRENT_PLAY_MODE.lock().unwrap() = Some(mode.clone());
    let conf = get_play_config();

    *ENABLE_LIFT.lock().unwrap() = conf.is_enablelift();
    *LIFT_VALUE.lock().unwrap() = (conf.lift * 1000.0) as i32;

    *ENABLE_HIDDEN.lock().unwrap() = conf.is_enablehidden();
    *HIDDEN_VALUE.lock().unwrap() = (conf.hidden * 1000.0) as i32;

    *ENABLE_LANECOVER.lock().unwrap() = conf.is_enablelanecover();
    *LANECOVER_VALUE.lock().unwrap() = (conf.lanecover * 1000.0) as i32;
    *LANE_COVER_MARGIN_LOW.lock().unwrap() = conf.lanecovermarginlow;
    *LANE_COVER_MARGIN_HIGH.lock().unwrap() = conf.lanecovermarginhigh;
    *LANE_COVER_SWITCH_DURATION.lock().unwrap() = conf.lanecoverswitchduration;

    *ENABLE_CONSTANT.lock().unwrap() = conf.is_enable_constant();
    *CONSTANT_VALUE.lock().unwrap() = conf.constant_fadein_time;
}

fn profile_switcher_ui(ui: &mut egui::Ui) {
    let players = PLAYERS.lock().unwrap();
    let mut selected = *SELECTED_PLAYER.lock().unwrap();
    let selected_text = players
        .get(selected as usize)
        .map(|s| s.as_str())
        .unwrap_or("(none)");

    ui.horizontal(|ui| {
        egui::ComboBox::from_id_salt("player_profile")
            .selected_text(selected_text)
            .show_ui(ui, |ui| {
                for (i, player) in players.iter().enumerate() {
                    if ui
                        .selectable_value(&mut selected, i as i32, player.as_str())
                        .clicked()
                    {
                        *SELECTED_PLAYER.lock().unwrap() = selected;
                    }
                }
            });

        if ui.button("Switch").clicked() {
            // Profile switch logic (deferred: requires MainController integration)
        }
        if ui.button("Reload list").clicked() {
            load_players();
        }
        ui.label("Player Profile");
    });
}

fn load_players() {
    *PLAYERS.lock().unwrap() = read_all_player_id("player");
}

#[allow(dead_code)]
fn profile_switcher() {
    let players = PLAYERS.lock().unwrap();
    let selected = *SELECTED_PLAYER.lock().unwrap() as usize;

    // ImGui.combo("##Player Profile", SELECTED_PLAYER, players, 4);
    // ImGui.sameLine();

    let main = MAIN.lock().unwrap();
    let config = CONFIG.lock().unwrap();
    let _can_click = if let (Some(_m), Some(c)) = (&*main, &*config) {
        // In Java: main.getCurrentState() instanceof MusicSelector
        // && !config.getPlayername().equals(players[SELECTED_PLAYER.get()])
        selected < players.len() && c.playername() != Some(players[selected].as_str())
    } else {
        false
    };
    drop(main);
    drop(config);

    // ImGui.beginDisabled(!canClick);
    // boolean switchClicked = ImGui.button("Switch");
    // ImGui.endDisabled();
    // ImGui.sameLine();
    // boolean reloadClicked = ImGui.button("Reload list");
    // ImGui.sameLine();
    // ImGui.text("Player Profile");

    // Switch logic
    // if (switchClicked) {
    //     String[] oldPlayers = players;
    //     loadPlayers();
    //     if (Arrays.equals(players, oldPlayers)) {
    //         PlayerConfig newPlayerConfig = PlayerConfig.readPlayerConfig("player", players[SELECTED_PLAYER.get()]);
    //         main.saveConfig();
    //         main.loadNewProfile(newPlayerConfig);
    //         changePlayMode(CURRENT_PLAY_MODE);
    //     }
    // }
    // if (reloadClicked) { loadPlayers(); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_play_mode_options_count() {
        let options = get_play_mode_options();
        assert_eq!(options.len(), 8);
    }

    #[test]
    fn test_get_play_mode_options_contains_expected_modes() {
        let options = get_play_mode_options();
        // Mode::hint() returns lowercase hyphenated strings like "beat-5k", "beat-7k"
        assert!(options.iter().any(|o| o.contains("5k")));
        assert!(options.iter().any(|o| o.contains("7k")));
        assert!(options.iter().any(|o| o.contains("10k")));
        assert!(options.iter().any(|o| o.contains("14k")));
    }

    #[test]
    fn test_get_play_mode_options_all_nonempty() {
        let options = get_play_mode_options();
        for option in &options {
            assert!(!option.is_empty(), "Play mode option should not be empty");
        }
    }
}
