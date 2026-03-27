use std::sync::{Mutex, MutexGuard};

// Re-exports
pub use rubato_types::imgui_notify::ImGuiNotify;

/// Constants from ObsConfigurationView (from beatoraja-launcher, not yet available)
pub const SCENE_NONE: &str = "(No Change)";
pub const ACTION_NONE: &str = "(Do Nothing)";

/// Acquire a mutex lock, recovering from poison if a thread panicked while holding it.
pub fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|e| e.into_inner())
}

// OBS WebSocket modules
pub mod obs_listener;
pub mod obs_ws_client;
