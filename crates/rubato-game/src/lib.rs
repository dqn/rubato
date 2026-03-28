//! Unified game crate: core engine, playback, states, IR, song DB,
//! external integrations, and launcher UI.

// Merged application-layer modules
pub mod core;
pub mod external;
pub mod ir;
pub mod play;
pub mod song;
pub mod state;

// Launcher modules (original rubato-launcher)
pub mod display_mode;
pub mod game_screen;
pub mod launcher_ui;
pub mod main_loader;
pub mod platform;
pub mod state_factory;
pub mod util;
pub mod version_checker;
pub mod views;

// Trait bridges and shared types (moved from rubato-types)
pub mod abstract_result_access;
pub mod event_type;
pub mod http_download_submitter;
pub mod imgui_access;
pub mod imgui_notify;
pub mod ir_resend_service;
pub mod ir_rival_provider;
pub mod main_state_access;
pub mod music_download_access;
pub mod obs_access;
pub mod random_history;
pub mod ranking_data_cache_access;
pub mod score_data_cache;
pub mod score_database_access;
pub mod score_handoff;
pub mod song_database_accessor;
pub mod song_information_db;
pub mod stream_controller_access;
pub mod table_update_source;

// Test support (moved from rubato-types)
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

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
