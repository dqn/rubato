#![allow(clippy::needless_range_loop)]
#![allow(clippy::manual_find)]
#![allow(clippy::comparison_chain)]
#![allow(unused_parens)]
#![allow(dead_code)]
#![allow(clippy::manual_clamp)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::type_complexity)]
#![allow(clippy::if_same_then_else)]

// Property submodule (interfaces + factories)
pub mod property;

// Rendering stubs (LibGDX graphics types — deferred to Phase 13 Bevy replacement)
pub mod rendering_stubs;
// Stubs for external dependencies (lifecycle types, Phase 7+)
pub mod stubs;

// Core skin types
pub mod custom_event;
pub mod custom_timer;
pub mod float_formatter;
pub mod skin_float;
pub mod skin_property;
pub mod skin_property_mapper;
pub mod skin_type;
pub mod stretch_type;

// Skin source types
pub mod skin_source;
pub mod skin_source_image;
pub mod skin_source_image_set;
pub mod skin_source_movie;
pub mod skin_source_reference;
pub mod skin_source_set;

// Skin object types
pub mod bitmap_font_batch_loader;
pub mod bitmap_font_cache;
pub mod pomyu_chara_loader;
pub mod skin;
pub mod skin_bar_object;
pub mod skin_bga_object;
pub mod skin_bpm_graph;
pub mod skin_gauge;
pub mod skin_gauge_graph_object;
pub mod skin_graph;
pub mod skin_header;
pub mod skin_hidden;
pub mod skin_hit_error_visualizer;
pub mod skin_image;
pub mod skin_judge_object;
pub mod skin_loader;
pub mod skin_note_distribution_graph;
pub mod skin_note_object;
pub mod skin_number;
pub mod skin_object;
pub mod skin_slider;
pub mod skin_text;
pub mod skin_text_bitmap;
pub mod skin_text_font;
pub mod skin_text_image;
pub mod skin_timing_distribution_graph;
pub mod skin_timing_visualizer;

// Skin data converter (SkinData -> Skin)
pub mod skin_data_converter;

// Skin loaders
pub mod json;
pub mod lr2;
pub mod lua;

// Test helpers
#[cfg(test)]
pub(crate) mod test_helpers;
