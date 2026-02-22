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
pub mod skin_main_state;
pub mod skin_offset;
pub mod skin_type;

// Timer access trait
pub mod timer_access;

// Timing distribution (result screen)
pub mod timing_distribution;

// Event types (music select input processing)
pub mod event_type;

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
pub mod abstract_result_access;
pub mod main_controller_access;
pub mod main_state_access;
pub mod player_resource_access;

// UI notification facade
pub mod imgui_notify;

// Random history (shared between beatoraja-pattern and beatoraja-modmenu)
pub mod random_history;

// Last-played-sort state (shared between beatoraja-select and beatoraja-modmenu)
pub mod last_played_sort;

// Screen type (external-facing state type)
pub mod screen_type;

// Score database access trait
pub mod score_database_access;

// Data models
pub mod course_data;
pub mod folder_data;
pub mod replay_data;
pub mod score_data;
pub mod song_data;
pub mod song_database_accessor;
pub mod song_information;
