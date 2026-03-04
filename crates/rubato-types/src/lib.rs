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

// Additional types from Phase 30+ stubs
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
