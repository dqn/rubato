pub mod launcher_ui;
pub mod platform;
pub mod state_factory;
pub mod stubs;

pub use launcher_ui::LauncherUi;
pub use launcher_ui::{LauncherResult, run_launcher};
pub use state_factory::LauncherStateFactory;

// Utility types
pub mod controller_config_view_model;
pub mod editable_table_view;
pub mod javafx_utils;
pub mod numeric_spinner;
pub mod song_data_view;
pub mod spinner_cell;

// Configuration views
pub mod audio_configuration_view;
pub mod discord_configuration_view;
pub mod input_configuration_view;
pub mod ir_configuration_view;
pub mod music_select_configuration_view;
pub mod stream_editor_view;
pub mod trainer_view;
pub mod video_configuration_view;

// Editor views
pub mod course_editor_view;
pub mod folder_editor_view;
pub mod obs_configuration_view;
pub mod table_editor_view;

// Main views
pub mod play_configuration_view;
pub mod resource_configuration_view;
pub mod skin_configuration_view;
