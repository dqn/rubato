// Stubs for external dependencies not yet available as proper imports.
// These will be replaced with real imports as the corresponding crates are translated.

use beatoraja_core::main_state::MainStateType;

/// Stub for MainController reference.
/// In Java, MainController.getStateType(MainState) returns the MainStateType.
pub struct MainControllerRef;

impl MainControllerRef {
    pub fn get_state_type(
        _state: &dyn beatoraja_core::main_state::MainState,
    ) -> Option<MainStateType> {
        todo!("Phase 8+ dependency: MainController.getStateType")
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
