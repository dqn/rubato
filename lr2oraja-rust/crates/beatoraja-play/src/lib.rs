#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_find)]
#![allow(clippy::comparison_chain)]
#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::if_same_then_else)]

pub mod bga;
pub mod bms_player;
pub mod bms_player_rule;
pub mod control_input_processor;
pub mod gauge_property;
pub mod ghost_battle_play;
pub mod groove_gauge;
pub mod judge_algorithm;
pub mod judge_manager;
pub mod judge_property;
pub mod key_input_processor;
pub mod key_sound_processor;
pub mod lane_property;
pub mod lane_renderer;
pub mod play_skin;
pub mod pomyu_chara_processor;
pub mod practice_configuration;
pub mod rhythm_timer_processor;
pub mod skin_bga;
pub mod skin_gauge;
pub mod skin_hidden;
pub mod skin_judge;
pub mod skin_note;
pub mod target_property;

// Re-exports (formerly in stubs.rs)
pub use beatoraja_core::main_controller::MainController;
pub use beatoraja_render::Texture;
