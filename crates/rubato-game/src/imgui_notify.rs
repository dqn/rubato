// Centralized ImGuiNotify API — log-backed facade.
// The real toast rendering lives in beatoraja-modmenu::imgui_notify;
// this module provides the convenience methods so that downstream crates
// can call ImGuiNotify::info/warning/error/success without depending on
// beatoraja-modmenu (which would introduce circular dependencies).

/// Notification facade (matches Java beatoraja.modmenu.ImGuiNotify convenience API).
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(msg: &str) {
        log::info!("{}", msg);
    }

    pub fn info_with_dismiss(msg: &str, _dismiss_time: i64) {
        log::info!("{}", msg);
    }

    pub fn warning(msg: &str) {
        log::warn!("{}", msg);
    }

    pub fn warning_with_dismiss(msg: &str, _dismiss_time: i64) {
        log::warn!("{}", msg);
    }

    pub fn error(msg: &str) {
        log::error!("{}", msg);
    }

    pub fn error_with_dismiss(msg: &str, _dismiss_time: i64) {
        log::error!("{}", msg);
    }

    pub fn success(msg: &str) {
        log::info!("{}", msg);
    }

    pub fn success_with_dismiss(msg: &str, _dismiss_time: i64) {
        log::info!("{}", msg);
    }
}
