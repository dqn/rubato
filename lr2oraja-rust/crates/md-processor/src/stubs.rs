// Stub types for Phase 4 dependencies

// Config is now imported from beatoraja-core (re-exported from beatoraja-types)
pub use beatoraja_core::config::Config;

/// Stub for MainController reference
pub trait MainControllerRef: Send + Sync {
    fn update_song(&self, path: &str, force: bool);
}

/// Stub for ImGuiNotify (logging placeholder)
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(msg: &str) {
        log::info!("[ImGuiNotify] {}", msg);
    }

    pub fn warning(msg: &str) {
        log::warn!("[ImGuiNotify] {}", msg);
    }

    pub fn error(msg: &str) {
        log::error!("[ImGuiNotify] {}", msg);
    }
}
