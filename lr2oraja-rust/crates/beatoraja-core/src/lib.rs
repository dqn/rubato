// Phase 5+ type stubs
pub mod stubs;

// Foundational types
pub mod resolution;
pub mod validatable;

// Config types
pub mod audio_config;
pub mod config;
pub mod ir_config;
pub mod play_config;
pub mod play_mode_config;
pub mod player_config;
pub mod skin_config;

// Enums and small types
pub mod bms_player_mode;
pub mod clear_type;
pub mod version;

// Data models
pub mod course_data;
pub mod player_data;
pub mod player_information;
pub mod random_course_data;
pub mod random_stage_data;
pub mod replay_data;
pub mod score_data;
pub mod score_data_property;
pub mod table_data;
pub mod table_data_bridge;

// Core types
pub mod bms_resource;
pub mod disposable_object;
pub mod main_controller;
pub mod main_loader;
pub mod main_state;
pub mod main_state_listener;
pub mod message_renderer;
pub mod performance_metrics;
pub mod pixmap_resource_pool;
pub mod player_resource;
pub mod resource_pool;
pub mod shader_manager;
pub mod sprite_batch_helper;
pub mod system_sound_manager;
pub mod timer_manager;
pub mod window_command;

// Database accessors
pub mod course_data_accessor;
pub mod play_data_accessor;
pub mod rival_data_accessor;
pub mod score_data_importer;
pub mod score_data_log_database_accessor;
pub mod score_database_accessor;
pub mod score_log_database_accessor;
pub mod sqlite_database_accessor;
pub mod table_data_accessor;

// Config subpackage
pub mod config_pkg;

// Pattern modifiers (merged from beatoraja-pattern crate)
pub mod pattern;

// Robust file I/O with backup (merged from beatoraja-system crate)
pub mod robust_file;
