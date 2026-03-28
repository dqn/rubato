pub mod download_task;
pub mod download_task_state;

// Re-exports
pub use crate::core::config::Config;
pub use crate::imgui_notify::ImGuiNotify;

/// Trait for MainController reference in md-processor
pub trait MainControllerRef: Send + Sync {
    fn update_song(&self, path: &str, force: bool);
}
pub mod http_download_processor;
pub mod http_download_source;
pub mod http_download_source_meta;
pub mod ipfs_information;
pub mod konmai_download_source;
pub mod music_database_accessor;
pub mod music_download_processor;
pub mod wriggle_download_source;
