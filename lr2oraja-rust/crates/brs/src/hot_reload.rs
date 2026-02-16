// Hot reload system: F5 reloads config and skin from disk.

use bevy::input::ButtonInput;
use bevy::prelude::*;
use tracing::{info, warn};

use crate::{BrsConfig, BrsPlayerConfig, BrsSkinManager, StateUiResources};

/// Bevy system: F5 triggers hot reload of config files and current skin.
///
/// Skips when egui (ModMenu) has keyboard focus.
pub fn hot_reload_system(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut config: ResMut<BrsConfig>,
    mut player_config: ResMut<BrsPlayerConfig>,
    mut skin_manager: ResMut<BrsSkinManager>,
    mod_menu: Res<bms_render::mod_menu::ModMenuState>,
    ui_res: Res<StateUiResources>,
) {
    if mod_menu.wants_keyboard {
        return;
    }

    if !keyboard.just_pressed(KeyCode::F5) {
        return;
    }

    info!("Hot reload triggered (F5)");

    // Reload config from disk.
    match bms_config::Config::read(&ui_res.config_paths.config) {
        Ok(new_config) => {
            config.0 = new_config;
            info!("Reloaded config from {:?}", ui_res.config_paths.config);
        }
        Err(e) => {
            warn!("Failed to reload config: {e}");
        }
    }

    // Reload player config from disk.
    match bms_config::PlayerConfig::read(&ui_res.config_paths.player_config) {
        Ok(new_player_config) => {
            player_config.0 = new_player_config;
            info!(
                "Reloaded player config from {:?}",
                ui_res.config_paths.player_config
            );
        }
        Err(e) => {
            warn!("Failed to reload player config: {e}");
        }
    }

    // Re-request current skin load if a skin is active.
    if let Some(skin_type) = skin_manager.0.current_type() {
        skin_manager.0.request_load(skin_type);
        info!("Requested skin reload for {:?}", skin_type);
    }
}
