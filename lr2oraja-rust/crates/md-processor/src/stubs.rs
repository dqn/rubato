// Stub types for Phase 4 dependencies

// Config is now imported from beatoraja-core (re-exported from beatoraja-types)
pub use beatoraja_core::config::Config;

/// Stub for MainController reference
pub trait MainControllerRef: Send + Sync {
    fn update_song(&self, path: &str, force: bool);
}

// Real type re-export (replaced from stubs)
pub use beatoraja_types::imgui_notify::ImGuiNotify;
