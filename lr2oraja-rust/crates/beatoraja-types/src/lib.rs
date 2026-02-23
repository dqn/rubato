// Shared types extracted from beatoraja-core to break circular dependencies.
// beatoraja-core, beatoraja-input, and beatoraja-audio all import from this crate.

// Stub types (downstream phase types that cannot be directly imported)
pub mod stubs;

// Types extracted from stubs (Phase 30a/30b)
pub mod bar_sorter;
pub mod bms_player_rule;
pub mod judge_algorithm;
pub mod key_input_log;
pub mod long_note_modifier;
pub mod mine_note_modifier;
pub mod pattern_modify_log;
pub mod scroll_speed_modifier;

// Foundational types
pub mod clear_type;
pub mod ipfs_information;
pub mod resolution;
pub mod validatable;

// Input constants
pub mod bm_keys;

// Skin types
pub mod skin_type;
pub mod skin_widget_focus;

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
pub mod screen_type;

// Lifecycle trait interfaces
pub mod main_controller_access;
pub mod main_state_access;
pub mod player_resource_access;

// Score / song database trait interfaces
pub mod score_database_access;
pub mod song_information_db;

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

// Additional types from Phase 30+ stubs
pub mod abstract_result_access;
pub mod event_type;
pub mod last_played_sort;
pub mod random_history;
pub mod skin_main_state;
pub mod skin_offset;
pub mod timer_access;
pub mod timing_distribution;
