// Shared types extracted from beatoraja-core to break circular dependencies.
// beatoraja-core, beatoraja-input, and beatoraja-audio all import from this crate.

// Stub types (downstream phase types that cannot be directly imported)
pub mod stubs;

// Foundational types
pub mod clear_type;
pub mod resolution;
pub mod validatable;

// Input constants
pub mod bm_keys;

// Config types
pub mod audio_config;
pub mod config;
pub mod ir_config;
pub mod play_config;
pub mod play_mode_config;
pub mod player_config;
pub mod skin_config;

// Data models
pub mod course_data;
pub mod replay_data;
pub mod score_data;
