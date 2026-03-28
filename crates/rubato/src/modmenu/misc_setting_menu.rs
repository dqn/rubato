use bms::model::mode::Mode;

use super::imgui_notify::{ImGuiNotify, NOTIFICATION_POSITIONS};
use super::{Config, PlayConfig, PlayerConfig, read_all_player_id};

use crate::core::command::Command;
use rubato_skin::sync_utils::lock_or_recover;
use std::sync::{Arc, Mutex};
use std::thread::ThreadId;

/// Combined state that was previously split across two separate statics
/// (`PLAYER_CONFIG` and `CURRENT_PLAY_MODE`) with a lock-ordering constraint.
/// Merging them into a single Mutex eliminates the deadlock risk entirely.
struct MenuState {
    player_config: Option<PlayerConfig>,
    current_play_mode: Option<Mode>,
}

static MENU_STATE: Mutex<MenuState> = Mutex::new(MenuState {
    player_config: None,
    current_play_mode: None,
});

/// Thread ID of the thread that called `set_player_config()`. Used for
/// debug-asserting single-thread access in `show_ui()` and `flush_play_config()`.
static OWNER_THREAD: Mutex<Option<ThreadId>> = Mutex::new(None);
static COMMAND_QUEUE: Mutex<Option<Arc<Mutex<Vec<Command>>>>> = Mutex::new(None);
static CONFIG: Mutex<Option<Config>> = Mutex::new(None);

// Some of the settings are based on play mode
// WARN: PLAY_MODE_VALUE has an initial value, 1 -> BEAT_7K
static PLAY_MODE_VALUE: Mutex<i32> = Mutex::new(1);

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

/// In-game misc settings menu (egui overlay).
///
/// Threading model: `show_ui()` runs on the main thread within the egui render
/// pass, NOT on a separate thread. Config changes are pushed to the unified
/// `Command` queue which is drained by `MainController` at safe points
/// (between state ticks), ensuring no config mutation races with
/// `BMSPlayer`'s own config reads. The static Mutexes here are for global state
/// storage (accessed only from the main thread), not for cross-thread
/// synchronization.
///
/// Config flow: UI statics -> flush_play_config() -> MENU_STATE.player_config +
/// Command queue -> MainController drain -> BMSPlayer.receive_updated_play_config()
/// -> LaneRenderer.apply_play_config().
pub struct MiscSettingMenu;

impl MiscSettingMenu {
    /// Clear all static state. Must be called before `set_player_config` when
    /// re-initializing after a profile switch (`load_new_profile`) so that stale
    /// references to the old `PlayerConfig`, `Config`, and command queue are not
    /// left behind.
    pub fn clear() {
        *lock_or_recover(&OWNER_THREAD) = None;
        {
            let mut state = lock_or_recover(&MENU_STATE);
            state.player_config = None;
            state.current_play_mode = None;
        }
        *lock_or_recover(&COMMAND_QUEUE) = None;
        *lock_or_recover(&CONFIG) = None;
        *lock_or_recover(&PLAY_MODE_VALUE) = 1;
        *lock_or_recover(&NOTIFICATION_POSITION) = 0;
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
        *lock_or_recover(&SELECTED_PLAYER) = 0;
        *lock_or_recover(&PLAYERS) = Vec::new();
    }

    /// Initialize with a PlayerConfig and command queue for writing changes back to MainController.
    pub fn set_player_config(
        player_config: PlayerConfig,
        config: Config,
        commands: Arc<Mutex<Vec<Command>>>,
    ) {
        *lock_or_recover(&OWNER_THREAD) = Some(std::thread::current().id());
        let players = read_all_player_id("player");
        let player_idx = players
            .iter()
            .position(|p| config.playername().is_some_and(|name| p == name))
            .unwrap_or(0);

        *lock_or_recover(&PLAYERS) = players;
        *lock_or_recover(&SELECTED_PLAYER) = player_idx as i32;
        lock_or_recover(&MENU_STATE).player_config = Some(player_config);
        *lock_or_recover(&CONFIG) = Some(config);
        *lock_or_recover(&COMMAND_QUEUE) = Some(commands);
    }

    /// Render the misc settings window using egui.
    pub fn show_ui(ctx: &egui::Context) {
        debug_assert!(
            lock_or_recover(&OWNER_THREAD).is_none_or(|tid| tid == std::thread::current().id()),
            "MiscSettingMenu::show_ui() must run on the same thread as set_player_config()"
        );
        {
            let state = lock_or_recover(&MENU_STATE);
            let mode_is_none = state.current_play_mode.is_none();
            drop(state);
            if mode_is_none {
                change_play_mode(&Mode::BEAT_7K);
            }
        }

        let mut open = true;
        egui::Window::new("Misc Settings")
            .open(&mut open)
            .auto_sized()
            .show(ctx, |ui| {
                // Notification position
                let mut pos = *lock_or_recover(&NOTIFICATION_POSITION);
                let pos_text = NOTIFICATION_POSITIONS
                    .get(pos as usize)
                    .copied()
                    .unwrap_or("TopLeft");
                egui::ComboBox::from_label("Notification Positions")
                    .selected_text(pos_text)
                    .show_ui(ui, |ui| {
                        for (i, name) in NOTIFICATION_POSITIONS.iter().enumerate() {
                            if ui.selectable_value(&mut pos, i as i32, *name).clicked() {
                                *lock_or_recover(&NOTIFICATION_POSITION) = pos;
                                ImGuiNotify::set_notification_position(pos as usize);
                            }
                        }
                    });

                // Play mode selector
                let play_mode_options = get_play_mode_options();
                let mut idx = *lock_or_recover(&PLAY_MODE_VALUE);
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
                                *lock_or_recover(&PLAY_MODE_VALUE) = idx;
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

                let mut lift_enabled = *lock_or_recover(&ENABLE_LIFT);
                if ui.checkbox(&mut lift_enabled, "Enable Lift").changed() {
                    *lock_or_recover(&ENABLE_LIFT) = lift_enabled;
                    dirty = true;
                }
                if lift_enabled {
                    let mut lift_val = *lock_or_recover(&LIFT_VALUE);
                    if ui
                        .add(egui::Slider::new(&mut lift_val, 0..=1000).text("Lift"))
                        .changed()
                    {
                        *lock_or_recover(&LIFT_VALUE) = lift_val;
                        dirty = true;
                    }
                }

                let mut hidden_enabled = *lock_or_recover(&ENABLE_HIDDEN);
                if ui.checkbox(&mut hidden_enabled, "Enable Hidden").changed() {
                    *lock_or_recover(&ENABLE_HIDDEN) = hidden_enabled;
                    dirty = true;
                }
                if hidden_enabled {
                    let mut hidden_val = *lock_or_recover(&HIDDEN_VALUE);
                    if ui
                        .add(egui::Slider::new(&mut hidden_val, 0..=1000).text("Hidden"))
                        .changed()
                    {
                        *lock_or_recover(&HIDDEN_VALUE) = hidden_val;
                        dirty = true;
                    }
                }

                let mut lc_enabled = *lock_or_recover(&ENABLE_LANECOVER);
                if ui.checkbox(&mut lc_enabled, "Enable Lane Cover").changed() {
                    *lock_or_recover(&ENABLE_LANECOVER) = lc_enabled;
                    dirty = true;
                }
                if lc_enabled {
                    let mut lc_val = *lock_or_recover(&LANECOVER_VALUE);
                    if ui
                        .add(egui::Slider::new(&mut lc_val, 0..=1000).text("Lane Cover"))
                        .changed()
                    {
                        *lock_or_recover(&LANECOVER_VALUE) = lc_val;
                        dirty = true;
                    }
                }

                let mut constant = *lock_or_recover(&ENABLE_CONSTANT);
                if ui.checkbox(&mut constant, "Enable Constant").changed() {
                    *lock_or_recover(&ENABLE_CONSTANT) = constant;
                    dirty = true;
                }
                if constant {
                    let mut constant_val = *lock_or_recover(&CONSTANT_VALUE);
                    if ui
                        .add(
                            egui::Slider::new(&mut constant_val, 0..=5000)
                                .text("Fade-in Time (ms)"),
                        )
                        .changed()
                    {
                        *lock_or_recover(&CONSTANT_VALUE) = constant_val;
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
///
// Known limitation: reads from process-global MENU_STATE.player_config which may be stale
// if hispeed was changed in-game via scroll keys. A proper fix requires either
// making player_config receive UpdatePlayConfig events, or selectively applying
// only modmenu-managed fields instead of using `..base` spread.
fn build_play_config_from_statics() -> PlayConfig {
    let base = get_play_config();
    PlayConfig {
        enablelift: *lock_or_recover(&ENABLE_LIFT),
        lift: *lock_or_recover(&LIFT_VALUE) as f32 / 1000.0,
        enablehidden: *lock_or_recover(&ENABLE_HIDDEN),
        hidden: *lock_or_recover(&HIDDEN_VALUE) as f32 / 1000.0,
        enablelanecover: *lock_or_recover(&ENABLE_LANECOVER),
        lanecover: *lock_or_recover(&LANECOVER_VALUE) as f32 / 1000.0,
        lanecovermarginlow: *lock_or_recover(&LANE_COVER_MARGIN_LOW),
        lanecovermarginhigh: *lock_or_recover(&LANE_COVER_MARGIN_HIGH),
        lanecoverswitchduration: *lock_or_recover(&LANE_COVER_SWITCH_DURATION),
        enable_constant: *lock_or_recover(&ENABLE_CONSTANT),
        constant_fadein_time: *lock_or_recover(&CONSTANT_VALUE),
        ..base
    }
}

/// Flush current UI state back to the local PlayerConfig and push an UpdatePlayConfig command
/// so MainController stays in sync.
fn flush_play_config() {
    debug_assert!(
        lock_or_recover(&OWNER_THREAD).is_none_or(|tid| tid == std::thread::current().id()),
        "flush_play_config() must run on the same thread as set_player_config()"
    );
    let updated = build_play_config_from_statics();

    // Update the local PlayerConfig clone (merge only modmenu-managed fields
    // so we don't overwrite hispeed/duration that may have been changed live).
    let mode = {
        let mut state = lock_or_recover(&MENU_STATE);
        let mode = match state.current_play_mode {
            Some(m) => m,
            None => return,
        };
        if let Some(ref mut pc) = state.player_config {
            pc.play_config(mode)
                .playconfig
                .apply_modmenu_fields(&updated);
        }
        mode
    };

    // Push to command queue for MainController to drain
    let queue = lock_or_recover(&COMMAND_QUEUE);
    if let Some(ref q) = *queue {
        q.lock()
            .unwrap_or_else(|e| e.into_inner())
            .push(Command::UpdatePlayConfig {
                mode,
                config: Box::new(updated),
            });
    }
}

/// Get current play mode(5k, 7k...) config from the local PlayerConfig clone.
///
/// Both `player_config` and `current_play_mode` are held in the same
/// `MENU_STATE` Mutex, so there is no lock-ordering concern.
fn get_play_config() -> PlayConfig {
    let state = lock_or_recover(&MENU_STATE);
    let mode = match state.current_play_mode {
        Some(m) => m,
        None => return PlayConfig::default(),
    };

    if let Some(ref pc) = state.player_config {
        return pc.play_config_ref(mode).playconfig.clone();
    }
    PlayConfig::default()
}

/// Change current play mode, resetting related options
fn change_play_mode(mode: &Mode) {
    lock_or_recover(&MENU_STATE).current_play_mode = Some(*mode);
    let conf = get_play_config();

    *lock_or_recover(&ENABLE_LIFT) = conf.enablelift;
    *lock_or_recover(&LIFT_VALUE) = (conf.lift * 1000.0) as i32;

    *lock_or_recover(&ENABLE_HIDDEN) = conf.enablehidden;
    *lock_or_recover(&HIDDEN_VALUE) = (conf.hidden * 1000.0) as i32;

    *lock_or_recover(&ENABLE_LANECOVER) = conf.enablelanecover;
    *lock_or_recover(&LANECOVER_VALUE) = (conf.lanecover * 1000.0) as i32;
    *lock_or_recover(&LANE_COVER_MARGIN_LOW) = conf.lanecovermarginlow;
    *lock_or_recover(&LANE_COVER_MARGIN_HIGH) = conf.lanecovermarginhigh;
    *lock_or_recover(&LANE_COVER_SWITCH_DURATION) = conf.lanecoverswitchduration;

    *lock_or_recover(&ENABLE_CONSTANT) = conf.enable_constant;
    *lock_or_recover(&CONSTANT_VALUE) = conf.constant_fadein_time;
}

fn profile_switcher_ui(ui: &mut egui::Ui) {
    let mut switch_clicked = false;
    let mut reload_clicked = false;
    let mut switch_player_id: Option<String> = None;

    {
        let players = lock_or_recover(&PLAYERS);
        let mut selected = *lock_or_recover(&SELECTED_PLAYER);
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
                            *lock_or_recover(&SELECTED_PLAYER) = selected;
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
        let old_players = lock_or_recover(&PLAYERS).clone();
        load_players();
        let new_players = lock_or_recover(&PLAYERS);
        if *new_players == old_players {
            drop(new_players);
            match PlayerConfig::read_player_config("player", &player_id) {
                Ok(new_pc) => {
                    // Update config.playername
                    {
                        let mut config = lock_or_recover(&CONFIG);
                        if let Some(ref mut c) = *config {
                            c.playername = new_pc.id.clone();
                        }
                    }
                    // Push LoadNewProfile before SaveConfig so MainController.config.playername
                    // is updated before the save (the modmenu's local CONFIG clone is separate).
                    {
                        let queue = lock_or_recover(&COMMAND_QUEUE);
                        if let Some(ref q) = *queue {
                            let mut cmds = q.lock().unwrap_or_else(|e| e.into_inner());
                            cmds.push(Command::LoadNewProfile(Box::new(new_pc.clone())));
                            cmds.push(Command::SaveConfig);
                        }
                    }
                    // Update local PlayerConfig and read current play mode
                    let mut state = lock_or_recover(&MENU_STATE);
                    state.player_config = Some(new_pc);
                    let mode = state.current_play_mode.unwrap_or(Mode::BEAT_7K);
                    drop(state);
                    // Refresh play mode settings from the new profile
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
    *lock_or_recover(&PLAYERS) = read_all_player_id("player");
}

#[allow(dead_code)]
fn profile_switcher() {
    let players = lock_or_recover(&PLAYERS);
    let selected = *lock_or_recover(&SELECTED_PLAYER) as usize;

    // ImGui.combo("##Player Profile", SELECTED_PLAYER, players, 4);
    // ImGui.sameLine();

    let config = lock_or_recover(&CONFIG);
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
        *lock_or_recover(&OWNER_THREAD) = None;
        {
            let mut state = lock_or_recover(&MENU_STATE);
            state.player_config = None;
            state.current_play_mode = None;
        }
        *lock_or_recover(&COMMAND_QUEUE) = None;
        *lock_or_recover(&CONFIG) = None;
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

    /// Combined test that exercises flush, change_play_mode, and no-outbox scenarios
    /// in sequence to avoid static interference between parallel test threads.
    #[test]
    fn test_modmenu_config_writeback() {
        reset_statics();

        // --- Part 1: flush_play_config writes to PlayerConfig and modmenu outbox ---
        {
            let mut pc = PlayerConfig::default();
            pc.mode7.playconfig.enablelift = false;
            pc.mode7.playconfig.lift = 0.0;

            let outbox = Arc::new(Mutex::new(Vec::new()));

            {
                let mut state = lock_or_recover(&MENU_STATE);
                state.player_config = Some(pc);
                state.current_play_mode = Some(Mode::BEAT_7K);
            }
            *lock_or_recover(&COMMAND_QUEUE) = Some(outbox.clone());

            // Simulate user enabling lift with value 500
            *lock_or_recover(&ENABLE_LIFT) = true;
            *lock_or_recover(&LIFT_VALUE) = 500;

            flush_play_config();

            // Verify local PlayerConfig was updated
            let state = lock_or_recover(&MENU_STATE);
            let pc = state.player_config.as_ref().unwrap();
            assert!(pc.play_config_ref(Mode::BEAT_7K).playconfig.enablelift);
            assert!(
                (pc.play_config_ref(Mode::BEAT_7K).playconfig.lift - 0.5).abs() < 0.001,
                "lift should be 0.5 (500/1000), got {}",
                pc.play_config_ref(Mode::BEAT_7K).playconfig.lift
            );
            drop(state);

            // Verify play config update was pushed to command queue
            let drained: Vec<_> = std::mem::take(&mut *outbox.lock().unwrap());
            assert_eq!(drained.len(), 1);
            match &drained[0] {
                Command::UpdatePlayConfig { mode, config } => {
                    assert_eq!(*mode, Mode::BEAT_7K);
                    assert!(config.enablelift);
                    assert!((config.lift - 0.5).abs() < 0.001);
                }
                other => panic!(
                    "expected UpdatePlayConfig, got {:?}",
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

            lock_or_recover(&MENU_STATE).player_config = Some(pc);

            change_play_mode(&Mode::BEAT_7K);

            assert!(*lock_or_recover(&ENABLE_LANECOVER));
            assert_eq!(*lock_or_recover(&LANECOVER_VALUE), 350);
            assert!(*lock_or_recover(&ENABLE_CONSTANT));
            assert_eq!(*lock_or_recover(&CONSTANT_VALUE), 200);
        }

        reset_statics();

        // --- Part 3: flush without outbox does not panic ---
        {
            let pc = PlayerConfig::default();
            {
                let mut state = lock_or_recover(&MENU_STATE);
                state.player_config = Some(pc);
                state.current_play_mode = Some(Mode::BEAT_7K);
            }
            *lock_or_recover(&COMMAND_QUEUE) = None;

            *lock_or_recover(&ENABLE_HIDDEN) = true;
            *lock_or_recover(&HIDDEN_VALUE) = 300;

            flush_play_config(); // Should not panic

            let state = lock_or_recover(&MENU_STATE);
            let pc = state.player_config.as_ref().unwrap();
            assert!(pc.play_config_ref(Mode::BEAT_7K).playconfig.enablehidden);
            assert!((pc.play_config_ref(Mode::BEAT_7K).playconfig.hidden - 0.3).abs() < 0.001,);
            drop(state);
        }

        reset_statics();
    }

    /// Regression: flush_play_config must not overwrite hispeed/duration in the
    /// local player_config. If another code path (e.g. scroll wheel) updated
    /// hispeed in the local clone while the modmenu was open, a full-struct write
    /// would clobber it with the stale snapshot value.
    #[test]
    fn test_flush_preserves_non_modmenu_fields_in_local_config() {
        reset_statics();

        let mut pc = PlayerConfig::default();
        // Simulate hispeed changed live (e.g. via scroll wheel) to a non-default value
        pc.mode7.playconfig.hispeed = 5.0;
        pc.mode7.playconfig.duration = 1200;
        pc.mode7.playconfig.fixhispeed = 1; // FIX_HISPEED_STARTBPM
        pc.mode7.playconfig.hispeedmargin = 3.5;
        pc.mode7.playconfig.hispeedautoadjust = true;

        let outbox = Arc::new(Mutex::new(Vec::new()));

        {
            let mut state = lock_or_recover(&MENU_STATE);
            state.player_config = Some(pc);
            state.current_play_mode = Some(Mode::BEAT_7K);
        }
        *lock_or_recover(&COMMAND_QUEUE) = Some(outbox.clone());

        // Simulate user toggling a modmenu field
        *lock_or_recover(&ENABLE_LIFT) = true;
        *lock_or_recover(&LIFT_VALUE) = 250;

        flush_play_config();

        // Non-modmenu fields must be preserved in the local player_config
        let state = lock_or_recover(&MENU_STATE);
        let live = &state
            .player_config
            .as_ref()
            .unwrap()
            .play_config_ref(Mode::BEAT_7K)
            .playconfig;
        assert_eq!(
            live.hispeed, 5.0,
            "hispeed must not be overwritten by flush"
        );
        assert_eq!(
            live.duration, 1200,
            "duration must not be overwritten by flush"
        );
        assert_eq!(live.fixhispeed, 1, "fixhispeed must not be overwritten");
        assert!(
            (live.hispeedmargin - 3.5).abs() < 0.001,
            "hispeedmargin must not be overwritten"
        );
        assert!(
            live.hispeedautoadjust,
            "hispeedautoadjust must not be overwritten"
        );

        // Modmenu-managed fields must be updated
        assert!(live.enablelift);
        assert!((live.lift - 0.25).abs() < 0.001);
        drop(state);

        reset_statics();
    }

    /// Verify that `MiscSettingMenu::clear()` resets all statics, preventing stale
    /// references after a profile switch.
    #[test]
    fn test_clear_resets_all_statics() {
        reset_statics();

        // Populate statics with non-default values
        let pc = PlayerConfig::default();
        let outbox = Arc::new(Mutex::new(Vec::new()));
        let config = Config::default();

        {
            let mut state = lock_or_recover(&MENU_STATE);
            state.player_config = Some(pc);
            state.current_play_mode = Some(Mode::BEAT_7K);
        }
        *lock_or_recover(&COMMAND_QUEUE) = Some(outbox.clone());
        *lock_or_recover(&CONFIG) = Some(config);
        *lock_or_recover(&PLAY_MODE_VALUE) = 3;
        *lock_or_recover(&NOTIFICATION_POSITION) = 2;
        *lock_or_recover(&ENABLE_LIFT) = true;
        *lock_or_recover(&LIFT_VALUE) = 500;
        *lock_or_recover(&ENABLE_HIDDEN) = true;
        *lock_or_recover(&HIDDEN_VALUE) = 300;
        *lock_or_recover(&ENABLE_LANECOVER) = true;
        *lock_or_recover(&LANECOVER_VALUE) = 700;
        *lock_or_recover(&LANE_COVER_MARGIN_LOW) = 0.1;
        *lock_or_recover(&LANE_COVER_MARGIN_HIGH) = 0.9;
        *lock_or_recover(&LANE_COVER_SWITCH_DURATION) = 42;
        *lock_or_recover(&ENABLE_CONSTANT) = true;
        *lock_or_recover(&CONSTANT_VALUE) = 1000;
        *lock_or_recover(&SELECTED_PLAYER) = 2;
        *lock_or_recover(&PLAYERS) = vec!["alice".to_string(), "bob".to_string()];

        // Verify statics are populated
        {
            let state = lock_or_recover(&MENU_STATE);
            assert!(state.player_config.is_some());
            assert!(state.current_play_mode.is_some());
        }
        assert!(lock_or_recover(&COMMAND_QUEUE).is_some());
        assert!(lock_or_recover(&CONFIG).is_some());

        // Clear
        MiscSettingMenu::clear();

        // Verify all Option statics are None
        {
            let state = lock_or_recover(&MENU_STATE);
            assert!(state.player_config.is_none());
            assert!(state.current_play_mode.is_none());
        }
        assert!(lock_or_recover(&COMMAND_QUEUE).is_none());
        assert!(lock_or_recover(&CONFIG).is_none());

        // Verify value statics are reset to defaults
        assert_eq!(*lock_or_recover(&PLAY_MODE_VALUE), 1);
        assert_eq!(*lock_or_recover(&NOTIFICATION_POSITION), 0);
        assert!(!*lock_or_recover(&ENABLE_LIFT));
        assert_eq!(*lock_or_recover(&LIFT_VALUE), 0);
        assert!(!*lock_or_recover(&ENABLE_HIDDEN));
        assert_eq!(*lock_or_recover(&HIDDEN_VALUE), 0);
        assert!(!*lock_or_recover(&ENABLE_LANECOVER));
        assert_eq!(*lock_or_recover(&LANECOVER_VALUE), 0);
        assert_eq!(*lock_or_recover(&LANE_COVER_MARGIN_LOW), 0.0);
        assert_eq!(*lock_or_recover(&LANE_COVER_MARGIN_HIGH), 0.0);
        assert_eq!(*lock_or_recover(&LANE_COVER_SWITCH_DURATION), 0);
        assert!(!*lock_or_recover(&ENABLE_CONSTANT));
        assert_eq!(*lock_or_recover(&CONSTANT_VALUE), 0);
        assert_eq!(*lock_or_recover(&SELECTED_PLAYER), 0);
        assert!(lock_or_recover(&PLAYERS).is_empty());

        // Verify that the command queue static is cleared (old Arc still exists
        // but is no longer reachable via the global static)
        outbox.lock().unwrap().push(Command::SaveConfig);
        assert!(lock_or_recover(&COMMAND_QUEUE).is_none());

        reset_statics();
    }

    /// Verify that `clear()` followed by `set_player_config()` produces a
    /// consistent fresh state (the re-init path after profile switch).
    #[test]
    fn test_clear_then_reinit() {
        reset_statics();

        // Initial setup
        let mut pc = PlayerConfig::default();
        pc.mode7.playconfig.enablelift = true;
        pc.mode7.playconfig.lift = 0.42;
        let outbox = Arc::new(Mutex::new(Vec::new()));
        let config = Config::default();

        MiscSettingMenu::set_player_config(pc, config, outbox.clone());

        // Simulate a play mode selection so current_play_mode is set
        change_play_mode(&Mode::BEAT_7K);
        assert!(*lock_or_recover(&ENABLE_LIFT));
        assert_eq!(*lock_or_recover(&LIFT_VALUE), 420);

        // Simulate profile switch: clear, then reinit with new profile
        MiscSettingMenu::clear();

        let mut new_pc = PlayerConfig::default();
        new_pc.mode7.playconfig.enablelift = false;
        new_pc.mode7.playconfig.lift = 0.0;
        new_pc.mode7.playconfig.enablehidden = true;
        new_pc.mode7.playconfig.hidden = 0.55;

        let new_outbox = Arc::new(Mutex::new(Vec::new()));
        let new_config = Config::default();
        MiscSettingMenu::set_player_config(new_pc, new_config, new_outbox.clone());

        // UI statics should still be cleared until change_play_mode is called
        assert!(!*lock_or_recover(&ENABLE_LIFT));
        assert_eq!(*lock_or_recover(&LIFT_VALUE), 0);

        // After mode change, new profile values should be loaded
        change_play_mode(&Mode::BEAT_7K);
        assert!(!*lock_or_recover(&ENABLE_LIFT));
        assert_eq!(*lock_or_recover(&LIFT_VALUE), 0);
        assert!(*lock_or_recover(&ENABLE_HIDDEN));
        assert_eq!(*lock_or_recover(&HIDDEN_VALUE), 550);

        // Commands should go to the new queue, not the old one
        flush_play_config();
        assert!(
            outbox.lock().unwrap().is_empty(),
            "old queue should receive no commands"
        );
        assert!(
            !new_outbox.lock().unwrap().is_empty(),
            "new queue should receive the flush command"
        );

        reset_statics();
    }

    /// Verify that get_play_config and flush_play_config work correctly
    /// now that player_config and current_play_mode share a single Mutex
    /// (MENU_STATE), eliminating the previous deadlock risk.
    #[test]
    fn test_play_config_and_play_mode_via_menu_state() {
        reset_statics();

        let mut pc = PlayerConfig::default();
        pc.mode7.playconfig.enablelift = true;
        pc.mode7.playconfig.lift = 0.75;
        let outbox = Arc::new(Mutex::new(Vec::new()));

        {
            let mut state = lock_or_recover(&MENU_STATE);
            state.player_config = Some(pc);
            state.current_play_mode = Some(Mode::BEAT_7K);
        }
        *lock_or_recover(&COMMAND_QUEUE) = Some(outbox);

        // get_play_config reads from single MENU_STATE Mutex
        let play_cfg = get_play_config();
        assert!(play_cfg.enablelift);
        assert!((play_cfg.lift - 0.75).abs() < 0.001);

        // flush_play_config must also work without nesting locks
        *lock_or_recover(&ENABLE_LIFT) = false;
        flush_play_config();

        let state = lock_or_recover(&MENU_STATE);
        let updated = &state
            .player_config
            .as_ref()
            .unwrap()
            .play_config_ref(Mode::BEAT_7K)
            .playconfig;
        assert!(!updated.enablelift, "flush must update player_config");
        drop(state);

        // get_play_config with no mode set returns default
        lock_or_recover(&MENU_STATE).current_play_mode = None;
        let default_cfg = get_play_config();
        assert!(!default_cfg.enablelift);
        assert_eq!(default_cfg.lift, PlayConfig::default().lift);

        reset_statics();
    }
}
