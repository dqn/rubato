// Stubs for external dependencies not yet available as proper imports.
// These will be replaced with real imports as the corresponding crates are translated.

use beatoraja_core::main_state::MainStateType;

// MainControllerAccess — re-exported from beatoraja-types (Phase 15d)
pub use beatoraja_types::main_controller_access::MainControllerAccess;
pub use beatoraja_types::main_state_type::MainStateType as TypesMainStateType;
pub use beatoraja_types::player_resource_access::PlayerResourceAccess;

/// Stub for MainController reference.
/// In Java, MainController.getStateType(MainState) returns the MainStateType.
pub struct MainControllerRef;

impl MainControllerRef {
    fn null_config() -> &'static beatoraja_types::config::Config {
        use std::sync::OnceLock;
        static CONFIG: OnceLock<beatoraja_types::config::Config> = OnceLock::new();
        CONFIG.get_or_init(beatoraja_types::config::Config::default)
    }

    fn null_player_config() -> &'static beatoraja_types::player_config::PlayerConfig {
        use std::sync::OnceLock;
        static PCONFIG: OnceLock<beatoraja_types::player_config::PlayerConfig> = OnceLock::new();
        PCONFIG.get_or_init(beatoraja_types::player_config::PlayerConfig::default)
    }
}

impl MainControllerAccess for MainControllerRef {
    fn get_config(&self) -> &beatoraja_types::config::Config {
        log::warn!("MainControllerRef::get_config called — returning default");
        Self::null_config()
    }
    fn get_player_config(&self) -> &beatoraja_types::player_config::PlayerConfig {
        log::warn!("MainControllerRef::get_player_config called — returning default");
        Self::null_player_config()
    }
    fn change_state(&mut self, _state: TypesMainStateType) {
        log::warn!("MainControllerRef::change_state called — no-op");
    }
    fn save_config(&self) {
        log::warn!("MainControllerRef::save_config called — no-op");
    }
    fn exit(&self) {
        log::warn!("MainControllerRef::exit called — no-op");
    }
    fn save_last_recording(&self, _reason: &str) {
        log::warn!("MainControllerRef::save_last_recording called — no-op");
    }
    fn update_song(&mut self, _path: Option<&str>) {
        log::warn!("MainControllerRef::update_song called — no-op");
    }
    fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }
    fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }
}

impl MainControllerRef {
    pub fn get_state_type(
        _state: &dyn beatoraja_core::main_state::MainState,
    ) -> Option<MainStateType> {
        log::warn!("not yet implemented: MainController.getStateType");
        None
    }
}

/// Stub for ImGuiNotify (from beatoraja-modmenu, not yet available as cross-dep)
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(message: &str) {
        log::info!("ImGuiNotify: {}", message);
    }
}

/// Constants from ObsConfigurationView (from beatoraja-launcher, not yet available)
pub const SCENE_NONE: &str = "(No Change)";
pub const ACTION_NONE: &str = "(Do Nothing)";
