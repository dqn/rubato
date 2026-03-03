#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_find)]
#![allow(clippy::comparison_chain)]
#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::if_same_then_else)]

// Stubs for external dependencies
pub mod stubs;

// Bar types (select.bar package)
pub mod bar;

// Select screen modules
pub mod bar_manager;
pub mod bar_renderer;
pub mod bar_sorter;
pub mod music_select_command;
pub mod music_select_input_processor;
pub mod music_select_key_property;
pub mod music_select_skin;
pub mod music_selector;
pub mod null_song_database_accessor;
pub mod preview_music_processor;
pub mod score_data_cache;
pub mod search_text_field;
pub mod skin_bar;
pub mod skin_distribution_graph;
