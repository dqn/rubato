//! BMS playback engine: judge timing, gauge calculation, lane rendering,
//! and input processing.

pub mod bga;
pub mod bms_player;
pub mod bms_player_rule;
pub mod gauge_property;
pub mod ghost_battle_play;
pub mod groove_gauge;
pub mod input;
pub mod judge;
pub mod lane_property;
pub mod lane_renderer;
pub mod play_skin;
pub mod pomyu_chara_processor;
pub mod practice_configuration;
pub mod rhythm_timer_processor;
pub mod skin;
pub mod target_property;

// Backwards-compatible re-exports for moved modules
pub use input::control_input as control_input_processor;
pub use input::key_input as key_input_processor;
pub use input::key_sound as key_sound_processor;
pub use judge::algorithm as judge_algorithm;
pub use judge::manager as judge_manager;
pub use judge::property as judge_property;
pub use skin::bga as skin_bga;
pub use skin::gauge as skin_gauge;
pub use skin::hidden as skin_hidden;
pub use skin::judge as skin_judge;
pub use skin::note as skin_note;

// Re-exports
pub use crate::core::main_controller::MainController;
pub use rubato_render::Texture;
