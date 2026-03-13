use bms_model::mode::Mode;

use super::imgui_notify::{ImGuiNotify, NOTIFICATION_POSITIONS};
use super::stubs::{Config, PlayConfig, PlayerConfig, read_all_player_id};
use rubato_types::main_controller_access::{MainControllerCommand, MainControllerCommandQueue};

use std::sync::Mutex;

static PLAYER_CONFIG: Mutex<Option<PlayerConfig>> = Mutex::new(None);
static COMMAND_QUEUE: Mutex<Option<MainControllerCommandQueue>> = Mutex::new(None);
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
    /// Initialize with a PlayerConfig and command queue for writing changes back to MainController.
    pub fn set_player_config(
        player_config: PlayerConfig,
        config: Config,
        command_queue: MainControllerCommandQueue,
    ) {
        let players = read_all_player_id("player");
        let player_idx = players
            .iter()
            .position(|p| config.playername().is_some_and(|name| p == name))
            .unwrap_or(0);

        *PLAYERS.lock().expect("PLAYERS lock poisoned") = players;
        *SELECTED_PLAYER
            .lock()
            .expect("SELECTED_PLAYER lock poisoned") = player_idx as i32;
        *PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned") = Some(player_config);
        *CONFIG.lock().expect("CONFIG lock poisoned") = Some(config);
        *COMMAND_QUEUE.lock().expect("COMMAND_QUEUE lock poisoned") = Some(command_queue);
    }

    /// Render the misc settings window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        {
            let mode = CURRENT_PLAY_MODE
                .lock()
                .expect("CURRENT_PLAY_MODE lock poisoned");
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
                let mut pos = *NOTIFICATION_POSITION
                    .lock()
                    .expect("NOTIFICATION_POSITION lock poisoned");
                let pos_text = NOTIFICATION_POSITIONS
                    .get(pos as usize)
                    .copied()
                    .unwrap_or("TopLeft");
                egui::ComboBox::from_label("Notification Positions")
                    .selected_text(pos_text)
                    .show_ui(ui, |ui| {
                        for (i, name) in NOTIFICATION_POSITIONS.iter().enumerate() {
                            if ui.selectable_value(&mut pos, i as i32, *name).clicked() {
                                *NOTIFICATION_POSITION
                                    .lock()
                                    .expect("NOTIFICATION_POSITION lock poisoned") = pos;
                                ImGuiNotify::set_notification_position(pos as usize);
                            }
                        }
                    });

                // Play mode selector
                let play_mode_options = get_play_mode_options();
                let mut idx = *PLAY_MODE_VALUE
                    .lock()
                    .expect("PLAY_MODE_VALUE lock poisoned");
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
                                *PLAY_MODE_VALUE
                                    .lock()
                                    .expect("PLAY_MODE_VALUE lock poisoned") = idx;
                                if let Some(mode) = Mode::from_hint(&play_mode_options[i]) {
                                    // Flush current mode's changes before switching
                                    flush_play_config();
                                    change_play_mode(&mode);
                                }
                            }
                        }
                    });

                ui.separator();

                // Lane cover / Hidden / Lift / Constant settings
                let mut dirty = false;

                let mut lift_enabled = *ENABLE_LIFT.lock().expect("ENABLE_LIFT lock poisoned");
                if ui.checkbox(&mut lift_enabled, "Enable Lift").changed() {
                    *ENABLE_LIFT.lock().expect("ENABLE_LIFT lock poisoned") = lift_enabled;
                    dirty = true;
                }
                if lift_enabled {
                    let mut lift_val = *LIFT_VALUE.lock().expect("LIFT_VALUE lock poisoned");
                    if ui
                        .add(egui::Slider::new(&mut lift_val, 0..=1000).text("Lift"))
                        .changed()
                    {
                        *LIFT_VALUE.lock().expect("LIFT_VALUE lock poisoned") = lift_val;
                        dirty = true;
                    }
                }

                let mut hidden_enabled =
                    *ENABLE_HIDDEN.lock().expect("ENABLE_HIDDEN lock poisoned");
                if ui.checkbox(&mut hidden_enabled, "Enable Hidden").changed() {
                    *ENABLE_HIDDEN.lock().expect("ENABLE_HIDDEN lock poisoned") = hidden_enabled;
                    dirty = true;
                }
                if hidden_enabled {
                    let mut hidden_val = *HIDDEN_VALUE.lock().expect("HIDDEN_VALUE lock poisoned");
                    if ui
                        .add(egui::Slider::new(&mut hidden_val, 0..=1000).text("Hidden"))
                        .changed()
                    {
                        *HIDDEN_VALUE.lock().expect("HIDDEN_VALUE lock poisoned") = hidden_val;
                        dirty = true;
                    }
                }

                let mut lc_enabled = *ENABLE_LANECOVER
                    .lock()
                    .expect("ENABLE_LANECOVER lock poisoned");
                if ui.checkbox(&mut lc_enabled, "Enable Lane Cover").changed() {
                    *ENABLE_LANECOVER
                        .lock()
                        .expect("ENABLE_LANECOVER lock poisoned") = lc_enabled;
                    dirty = true;
                }
                if lc_enabled {
                    let mut lc_val = *LANECOVER_VALUE
                        .lock()
                        .expect("LANECOVER_VALUE lock poisoned");
                    if ui
                        .add(egui::Slider::new(&mut lc_val, 0..=1000).text("Lane Cover"))
                        .changed()
                    {
                        *LANECOVER_VALUE
                            .lock()
                            .expect("LANECOVER_VALUE lock poisoned") = lc_val;
                        dirty = true;
                    }
                }

                let mut constant = *ENABLE_CONSTANT
                    .lock()
                    .expect("ENABLE_CONSTANT lock poisoned");
                if ui.checkbox(&mut constant, "Enable Constant").changed() {
                    *ENABLE_CONSTANT
                        .lock()
                        .expect("ENABLE_CONSTANT lock poisoned") = constant;
                    dirty = true;
                }
                if constant {
                    let mut constant_val =
                        *CONSTANT_VALUE.lock().expect("CONSTANT_VALUE lock poisoned");
                    if ui
                        .add(
                            egui::Slider::new(&mut constant_val, 0..=5000)
                                .text("Fade-in Time (ms)"),
                        )
                        .changed()
                    {
                        *CONSTANT_VALUE.lock().expect("CONSTANT_VALUE lock poisoned") =
                            constant_val;
                        dirty = true;
                    }
                }

                // Flush UI state back to PlayerConfig and command queue only when changed
                if dirty {
                    flush_play_config();
                }

                ui.separator();

                // Profile switcher
                profile_switcher_ui(ui);
            });
    }
}

/// Build a PlayConfig from the current UI statics, preserving fields not shown in the UI.
fn build_play_config_from_statics() -> PlayConfig {
    let base = get_play_config();
    PlayConfig {
        enablelift: *ENABLE_LIFT.lock().expect("ENABLE_LIFT lock poisoned"),
        lift: *LIFT_VALUE.lock().expect("LIFT_VALUE lock poisoned") as f32 / 1000.0,
        enablehidden: *ENABLE_HIDDEN.lock().expect("ENABLE_HIDDEN lock poisoned"),
        hidden: *HIDDEN_VALUE.lock().expect("HIDDEN_VALUE lock poisoned") as f32 / 1000.0,
        enablelanecover: *ENABLE_LANECOVER
            .lock()
            .expect("ENABLE_LANECOVER lock poisoned"),
        lanecover: *LANECOVER_VALUE
            .lock()
            .expect("LANECOVER_VALUE lock poisoned") as f32
            / 1000.0,
        lanecovermarginlow: *LANE_COVER_MARGIN_LOW
            .lock()
            .expect("LANE_COVER_MARGIN_LOW lock poisoned"),
        lanecovermarginhigh: *LANE_COVER_MARGIN_HIGH
            .lock()
            .expect("LANE_COVER_MARGIN_HIGH lock poisoned"),
        lanecoverswitchduration: *LANE_COVER_SWITCH_DURATION
            .lock()
            .expect("LANE_COVER_SWITCH_DURATION lock poisoned"),
        enable_constant: *ENABLE_CONSTANT
            .lock()
            .expect("ENABLE_CONSTANT lock poisoned"),
        constant_fadein_time: *CONSTANT_VALUE.lock().expect("CONSTANT_VALUE lock poisoned"),
        ..base
    }
}

/// Flush current UI state back to the local PlayerConfig and push an UpdatePlayConfig command
/// so MainController stays in sync.
fn flush_play_config() {
    let mode = match *CURRENT_PLAY_MODE
        .lock()
        .expect("CURRENT_PLAY_MODE lock poisoned")
    {
        Some(m) => m,
        None => return,
    };

    let updated = build_play_config_from_statics();

    // Update the local PlayerConfig clone
    {
        let mut pc_guard = PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned");
        if let Some(ref mut pc) = *pc_guard {
            pc.play_config(mode).playconfig = updated.clone();
        }
    }

    // Push command to MainController
    let queue = COMMAND_QUEUE.lock().expect("COMMAND_QUEUE lock poisoned");
    if let Some(ref q) = *queue {
        q.push(MainControllerCommand::UpdatePlayConfig(
            mode,
            Box::new(updated),
        ));
    }
}

/// Get current play mode(5k, 7k...) config from the local PlayerConfig clone.
fn get_play_config() -> PlayConfig {
    let pc_guard = PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned");
    if let Some(ref pc) = *pc_guard {
        let mode = CURRENT_PLAY_MODE
            .lock()
            .expect("CURRENT_PLAY_MODE lock poisoned");
        if let Some(ref mode) = *mode {
            // Use play_config_ref() to avoid the &mut requirement of play_config()
            return pc.play_config_ref(*mode).playconfig.clone();
        }
    }
    PlayConfig::default()
}

/// Change current play mode, resetting related options
fn change_play_mode(mode: &Mode) {
    *CURRENT_PLAY_MODE
        .lock()
        .expect("CURRENT_PLAY_MODE lock poisoned") = Some(*mode);
    let conf = get_play_config();

    *ENABLE_LIFT.lock().expect("ENABLE_LIFT lock poisoned") = conf.enablelift;
    *LIFT_VALUE.lock().expect("LIFT_VALUE lock poisoned") = (conf.lift * 1000.0) as i32;

    *ENABLE_HIDDEN.lock().expect("ENABLE_HIDDEN lock poisoned") = conf.enablehidden;
    *HIDDEN_VALUE.lock().expect("HIDDEN_VALUE lock poisoned") = (conf.hidden * 1000.0) as i32;

    *ENABLE_LANECOVER
        .lock()
        .expect("ENABLE_LANECOVER lock poisoned") = conf.enablelanecover;
    *LANECOVER_VALUE
        .lock()
        .expect("LANECOVER_VALUE lock poisoned") = (conf.lanecover * 1000.0) as i32;
    *LANE_COVER_MARGIN_LOW
        .lock()
        .expect("LANE_COVER_MARGIN_LOW lock poisoned") = conf.lanecovermarginlow;
    *LANE_COVER_MARGIN_HIGH
        .lock()
        .expect("LANE_COVER_MARGIN_HIGH lock poisoned") = conf.lanecovermarginhigh;
    *LANE_COVER_SWITCH_DURATION
        .lock()
        .expect("LANE_COVER_SWITCH_DURATION lock poisoned") = conf.lanecoverswitchduration;

    *ENABLE_CONSTANT
        .lock()
        .expect("ENABLE_CONSTANT lock poisoned") = conf.enable_constant;
    *CONSTANT_VALUE.lock().expect("CONSTANT_VALUE lock poisoned") = conf.constant_fadein_time;
}

fn profile_switcher_ui(ui: &mut egui::Ui) {
    let mut switch_clicked = false;
    let mut reload_clicked = false;
    let mut switch_player_id: Option<String> = None;

    {
        let players = PLAYERS.lock().expect("PLAYERS lock poisoned");
        let mut selected = *SELECTED_PLAYER
            .lock()
            .expect("SELECTED_PLAYER lock poisoned");
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
                            *SELECTED_PLAYER
                                .lock()
                                .expect("SELECTED_PLAYER lock poisoned") = selected;
                        }
                    }
                });

            switch_clicked = ui.button("Switch").clicked();
            if switch_clicked {
                let sel = selected as usize;
                if sel < players.len() {
                    switch_player_id = Some(players[sel].clone());
                }
            }
            reload_clicked = ui.button("Reload list").clicked();
            ui.label("Player Profile");
        });
    }

    // Handle switch outside the players lock to allow reload + re-lock
    if switch_clicked && let Some(player_id) = switch_player_id {
        let old_players = PLAYERS.lock().expect("PLAYERS lock poisoned").clone();
        load_players();
        let new_players = PLAYERS.lock().expect("PLAYERS lock poisoned");
        if *new_players == old_players {
            drop(new_players);
            match PlayerConfig::read_player_config("player", &player_id) {
                Ok(new_pc) => {
                    // Update config.playername
                    {
                        let mut config = CONFIG.lock().expect("CONFIG lock poisoned");
                        if let Some(ref mut c) = *config {
                            c.playername = new_pc.id.clone();
                        }
                    }
                    // Push SaveConfig and LoadNewProfile commands via the queue
                    {
                        let queue = COMMAND_QUEUE.lock().expect("COMMAND_QUEUE lock poisoned");
                        if let Some(ref q) = *queue {
                            q.push(MainControllerCommand::SaveConfig);
                            q.push(MainControllerCommand::LoadNewProfile(Box::new(
                                new_pc.clone(),
                            )));
                        }
                    }
                    // Update local PlayerConfig
                    *PLAYER_CONFIG.lock().expect("PLAYER_CONFIG lock poisoned") = Some(new_pc);
                    // Refresh play mode settings from the new profile
                    let mode = CURRENT_PLAY_MODE
                        .lock()
                        .expect("CURRENT_PLAY_MODE lock poisoned")
                        .unwrap_or(Mode::BEAT_7K);
                    change_play_mode(&mode);
                    log::info!("Profile switched to: {}", player_id);
                }
                Err(e) => {
                    log::error!("Failed to read player config '{}': {}", player_id, e);
                }
            }
        } else {
            log::info!("Player list changed during switch; aborting profile switch");
        }
    }
    if reload_clicked {
        load_players();
    }
}

fn load_players() {
    *PLAYERS.lock().expect("PLAYERS lock poisoned") = read_all_player_id("player");
}

#[allow(dead_code)]
fn profile_switcher() {
    let players = PLAYERS.lock().expect("PLAYERS lock poisoned");
    let selected = *SELECTED_PLAYER
        .lock()
        .expect("SELECTED_PLAYER lock poisoned") as usize;

    // ImGui.combo("##Player Profile", SELECTED_PLAYER, players, 4);
    // ImGui.sameLine();

    let config = CONFIG.lock().expect("CONFIG lock poisoned");
    let _can_click = if let Some(c) = &*config {
        // In Java: main.getCurrentState() instanceof MusicSelector
        // && !config.getPlayername().equals(players[SELECTED_PLAYER.get()])
        selected < players.len() && c.playername() != Some(players[selected].as_str())
    } else {
        false
    };
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

    /// Helper to recover a lock that may have been poisoned by a previous test panic.
    fn lock_or_recover<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
        match mutex.lock() {
            Ok(guard) => guard,
            Err(poisoned) => poisoned.into_inner(),
        }
    }

    /// Reset all statics to their default state. Uses lock_or_recover to handle
    /// poisoned mutexes from prior panics.
    fn reset_statics() {
        *lock_or_recover(&PLAYER_CONFIG) = None;
        *lock_or_recover(&COMMAND_QUEUE) = None;
        *lock_or_recover(&CONFIG) = None;
        *lock_or_recover(&CURRENT_PLAY_MODE) = None;
        *lock_or_recover(&PLAY_MODE_VALUE) = 1;
        *lock_or_recover(&ENABLE_LIFT) = false;
        *lock_or_recover(&LIFT_VALUE) = 0;
        *lock_or_recover(&ENABLE_HIDDEN) = false;
        *lock_or_recover(&HIDDEN_VALUE) = 0;
        *lock_or_recover(&ENABLE_LANECOVER) = false;
        *lock_or_recover(&LANECOVER_VALUE) = 0;
        *lock_or_recover(&LANE_COVER_MARGIN_LOW) = 0.0;
        *lock_or_recover(&LANE_COVER_MARGIN_HIGH) = 0.0;
        *lock_or_recover(&LANE_COVER_SWITCH_DURATION) = 0;
        *lock_or_recover(&ENABLE_CONSTANT) = false;
        *lock_or_recover(&CONSTANT_VALUE) = 0;
    }

    /// Combined test that exercises flush, change_play_mode, and no-queue scenarios
    /// in sequence to avoid static interference between parallel test threads.
    #[test]
    fn test_modmenu_config_writeback() {
        reset_statics();

        // --- Part 1: flush_play_config writes to PlayerConfig and command queue ---
        {
            let mut pc = PlayerConfig::default();
            pc.mode7.playconfig.enablelift = false;
            pc.mode7.playconfig.lift = 0.0;

            let queue = MainControllerCommandQueue::new();

            *lock_or_recover(&PLAYER_CONFIG) = Some(pc);
            *lock_or_recover(&COMMAND_QUEUE) = Some(queue.clone());
            *lock_or_recover(&CURRENT_PLAY_MODE) = Some(Mode::BEAT_7K);

            // Simulate user enabling lift with value 500
            *lock_or_recover(&ENABLE_LIFT) = true;
            *lock_or_recover(&LIFT_VALUE) = 500;

            flush_play_config();

            // Verify local PlayerConfig was updated
            let pc_guard = lock_or_recover(&PLAYER_CONFIG);
            let pc = pc_guard.as_ref().unwrap();
            assert!(pc.play_config_ref(Mode::BEAT_7K).playconfig.enablelift);
            assert!(
                (pc.play_config_ref(Mode::BEAT_7K).playconfig.lift - 0.5).abs() < 0.001,
                "lift should be 0.5 (500/1000), got {}",
                pc.play_config_ref(Mode::BEAT_7K).playconfig.lift
            );
            drop(pc_guard);

            // Verify command was pushed
            let commands = queue.drain();
            assert_eq!(commands.len(), 1);
            match &commands[0] {
                MainControllerCommand::UpdatePlayConfig(mode, config) => {
                    assert_eq!(*mode, Mode::BEAT_7K);
                    assert!(config.enablelift);
                    assert!((config.lift - 0.5).abs() < 0.001);
                }
                other => panic!(
                    "Expected UpdatePlayConfig, got {:?}",
                    std::mem::discriminant(other)
                ),
            }
        }

        reset_statics();

        // --- Part 2: change_play_mode loads from PlayerConfig ---
        {
            let mut pc = PlayerConfig::default();
            pc.mode7.playconfig.enablelanecover = true;
            pc.mode7.playconfig.lanecover = 0.35;
            pc.mode7.playconfig.enable_constant = true;
            pc.mode7.playconfig.constant_fadein_time = 200;

            *lock_or_recover(&PLAYER_CONFIG) = Some(pc);

            change_play_mode(&Mode::BEAT_7K);

            assert!(*lock_or_recover(&ENABLE_LANECOVER));
            assert_eq!(*lock_or_recover(&LANECOVER_VALUE), 350);
            assert!(*lock_or_recover(&ENABLE_CONSTANT));
            assert_eq!(*lock_or_recover(&CONSTANT_VALUE), 200);
        }

        reset_statics();

        // --- Part 3: flush without command queue does not panic ---
        {
            let pc = PlayerConfig::default();
            *lock_or_recover(&PLAYER_CONFIG) = Some(pc);
            *lock_or_recover(&COMMAND_QUEUE) = None;
            *lock_or_recover(&CURRENT_PLAY_MODE) = Some(Mode::BEAT_7K);

            *lock_or_recover(&ENABLE_HIDDEN) = true;
            *lock_or_recover(&HIDDEN_VALUE) = 300;

            flush_play_config(); // Should not panic

            let pc_guard = lock_or_recover(&PLAYER_CONFIG);
            let pc = pc_guard.as_ref().unwrap();
            assert!(pc.play_config_ref(Mode::BEAT_7K).playconfig.enablehidden);
            assert!((pc.play_config_ref(Mode::BEAT_7K).playconfig.hidden - 0.3).abs() < 0.001,);
            drop(pc_guard);
        }

        reset_statics();
    }
}
