// External dependency stubs for beatoraja-stream

/// Stub for beatoraja.modmenu.ImGuiNotify
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(msg: &str) {
        log::info!("{}", msg);
    }
    pub fn warning(msg: &str) {
        log::warn!("{}", msg);
    }
}
