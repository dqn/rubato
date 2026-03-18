//! Shared type definitions, configuration structures, and trait interfaces
//! for the rubato BMS player.

// Shared types
pub mod bar_sorter;
pub mod bms_player_input_device;
pub mod bms_player_rule;
pub mod judge_algorithm;
pub mod key_input_log;
pub mod long_note_modifier;
pub mod mine_note_modifier;
pub mod pattern_modify_log;
pub mod scroll_speed_modifier;

// IR connection registry (stub, will become a trait in Phase 6)
pub mod ir_connection_registry;

// Top-level re-exports
pub use bar_sorter::{BarSorter, BarSorterEntry};
pub use bms_player_rule::BMSPlayerRule;
pub use groove_gauge::GrooveGauge;
pub use ir_connection_registry::IRConnectionManager;
pub use judge_algorithm::JudgeAlgorithm;
pub use key_input_log::KeyInputLog;
pub use pattern_modify_log::PatternModifyLog;
pub use skin_type::SkinType;
pub use song_data::SongData;

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
pub mod sound_type;

// Lifecycle trait interfaces
pub mod main_controller_access;
pub mod main_state_access;
pub mod player_resource_access;

// Score / song database trait interfaces
pub mod http_download_submitter;
pub mod score_database_access;
pub mod song_information_db;
pub mod table_update_source;

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

// Semantic newtypes for type safety
pub mod event_id;
pub mod timer_id;
pub mod value_id;

// Additional shared types
pub mod abstract_result_access;
pub mod event_type;
pub mod last_played_sort;
pub mod random_history;
pub mod skin_main_state;
pub mod skin_offset;
pub mod target_list;
pub mod timer_access;
pub mod timing_distribution;

// Player types
pub mod player_data;
pub mod player_information;

// Song selection interface (modmenu↔select bridge)
pub mod song_selection_access;

// Score data property (calculated values from score data)
pub mod score_data_property;

// Skin render context (SkinDrawable expansion)
pub mod skin_render_context;

// Distribution data (SkinDistributionGraph bridge)
pub mod distribution_data;

// IR rival provider (core↔ir bridge for rival score fetching)
pub mod ir_rival_provider;

// Score data cache (shared between core and select)
pub mod score_data_cache;

// Ranking data cache trait (core↔ir bridge for IR ranking cache)
pub mod ranking_data_cache_access;

// Input processor access (keyboard control keys + commands)
pub mod input_processor_access;

// Score handoff (Play→Result data transfer via outbox pattern)
pub mod score_handoff;

// Target property trait (core↔play bridge for score target)
pub mod target_property_access;

// IR resend service (core↔result bridge for background IR score retry)
pub mod ir_resend_service;

// Stream controller access (core↔stream bridge for named pipe listener)
pub mod stream_controller_access;

// Music download access (core↔md-processor bridge for IPFS download)
pub mod music_download_access;

// OBS WebSocket access (core↔obs bridge)
pub mod obs_access;

// ImGui overlay access (core↔modmenu bridge)
pub mod imgui_access;

// Offset capabilities (shared boolean group for custom skin offsets)
pub mod offset_capabilities;

// Global FPS counter (main loop → skin property ID 20)
pub mod fps_counter;

// Process-global monotonic clock (equivalent to Java System.nanoTime())
pub mod monotonic_clock;

// State machine observability events (E2E testing)
pub mod state_event;

// Synchronization utilities (poison-recovery pattern)
pub mod sync_utils;

// Test support utilities (behind feature gate or in test builds)
#[cfg(any(test, feature = "test-support"))]
pub mod test_support;
