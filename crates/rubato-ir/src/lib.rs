//! Internet Ranking (IR) client for LR2IR score submission and ranking data.

// Re-exports (formerly in stubs.rs)
pub use bms_model::bms_decoder::convert_hex_string;
pub use rubato_core::pattern::lr2_random::LR2Random;
pub use rubato_core::pattern::random::Random;
pub use rubato_song::song_data::SongData;
pub use rubato_types::imgui_notify::ImGuiNotify;

// IR data types
pub mod ir_account;
pub mod ir_chart_data;
pub mod ir_connection;
pub mod ir_connection_manager;
pub mod ir_course_data;
pub mod ir_player_data;
pub mod ir_response;
pub mod ir_score_data;
pub mod ir_table_data;

// Leaderboard
pub mod leaderboard_entry;

// LR2 IR
pub mod lr2_ghost_data;
pub mod lr2_ir_connection;

// Ranking
pub mod ranking_data;
pub mod ranking_data_cache;

// IR rival provider (implementation of beatoraja-types::IRRivalProvider)
pub mod ir_rival_provider_impl;
