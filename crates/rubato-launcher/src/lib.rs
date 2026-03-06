//! Configuration UI views for audio, input, skin, and gameplay settings.

pub mod launcher_ui;
pub mod platform;
pub mod state_factory;
pub mod stubs;
pub mod util;
pub mod views;

pub use launcher_ui::LauncherUi;
pub use launcher_ui::{LauncherResult, run_launcher};
pub use state_factory::LauncherStateFactory;

// Backwards-compatible re-exports for moved modules

// Utility types
pub use util::controller_config_view_model;
pub use util::editable_table_view;
pub use util::javafx_utils;
pub use util::numeric_spinner;
pub use util::song_data_view;
pub use util::spinner_cell;

// Configuration views
pub use views::config::audio_configuration_view;
pub use views::config::discord_configuration_view;
pub use views::config::input_configuration_view;
pub use views::config::ir_configuration_view;
pub use views::config::music_select_configuration_view;
pub use views::config::obs_configuration_view;
pub use views::config::stream_editor_view;
pub use views::config::trainer_view;
pub use views::config::video_configuration_view;

// Editor views
pub use views::editors::course_editor_view;
pub use views::editors::folder_editor_view;
pub use views::editors::table_editor_view;

// Main views
pub use views::play_configuration_view;
pub use views::resource_configuration_view;
pub use views::skin_configuration_view;
