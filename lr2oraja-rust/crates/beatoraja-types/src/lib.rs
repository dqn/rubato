// Shared types extracted from beatoraja-core to break circular dependencies.
// beatoraja-core, beatoraja-input, and beatoraja-audio all import from this crate.

// Stub types (downstream phase types that cannot be directly imported)
pub mod stubs;

// Foundational types
pub mod clear_type;
pub mod ipfs_information;
pub mod resolution;
pub mod validatable;

// Input constants
pub mod bm_keys;

// Skin types
pub mod skin_type;

// Gauge types
pub mod gauge_property;
pub mod groove_gauge;

// Config types
pub mod audio_config;
pub mod config;
pub mod ir_config;
pub mod play_config;
pub mod play_mode_config;
pub mod player_config;
pub mod skin_config;

// State types
pub mod main_state_type;

// Lifecycle trait interfaces
pub mod main_controller_access;
pub mod player_resource_access;

// UI notification facade
pub mod imgui_notify;

// Data models
pub mod course_data;
pub mod folder_data;
pub mod replay_data;
pub mod score_data;
pub mod song_data;
pub mod song_database_accessor;
pub mod song_information;
