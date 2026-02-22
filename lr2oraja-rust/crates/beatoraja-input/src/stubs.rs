// Config types re-exported from beatoraja-types
pub use beatoraja_types::config::Config;
pub use beatoraja_types::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, MidiInput, MidiInputType, MouseScratchConfig,
    PlayModeConfig,
};
pub use beatoraja_types::player_config::PlayerConfig;
pub use beatoraja_types::resolution::Resolution;

// Real implementations moved to dedicated modules (Phase 25a)
pub use crate::gdx_compat::{GdxGraphics, GdxInput, get_shared_key_state, set_shared_key_state};
pub use crate::keys::Keys;

/// Stub for SkinWidgetManager
pub struct SkinWidgetManager;

impl SkinWidgetManager {
    pub fn get_focus() -> bool {
        false
    }
}

/// Stub for Controller (com.badlogic.gdx.controllers.Controller)
pub struct Controller {
    name: String,
}

impl Controller {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    pub fn get_name(&self) -> &str {
        &self.name
    }

    pub fn get_button(&self, _button: i32) -> bool {
        false
    }

    pub fn get_axis(&self, _axis: i32) -> f32 {
        0.0
    }
}
