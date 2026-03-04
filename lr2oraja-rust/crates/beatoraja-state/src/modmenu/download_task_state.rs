// Re-export from md-processor where the canonical implementation lives.
// DownloadTaskState is used by both beatoraja-select and beatoraja-modmenu;
// placing it in md-processor avoids a circular dependency.
pub use beatoraja_song::md_processor::download_task_state::DownloadTaskState;
