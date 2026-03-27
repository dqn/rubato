//! Skin format loaders (LR2 CSV, JSON, Lua), property system,
//! and rendering object hierarchy.

// Property submodule (interfaces + factories)
pub mod property;

// Rendering re-exports (wgpu-backed LibGDX equivalents from rubato-render)
pub mod render_reexports;
// Re-exports (convenience imports for commonly used types)
pub mod reexports;

// Shared utilities (available to both loader and render code)
pub mod util;

// Bitmap font cache (shared between loader and render)
pub mod bitmap_font_cache;

// Real implementations and standalone types
pub mod main_state;
pub mod skin_config_offset;
pub mod skin_resolution;
pub mod skin_timer;
pub mod snapshot_main_state;

// Skin property enums (standalone, no subdir)
pub mod skin_property;

// Organized submodules
pub mod core;
pub mod graphs;
pub mod loaders;
pub mod objects;
pub mod sources;
pub mod text;
pub mod types;

// Skin loaders
pub mod json;
pub mod lr2;
pub mod lua;

// Test helpers
#[cfg(test)]
pub(crate) mod test_helpers;

// Backwards-compatible re-exports for moved modules

// core/
pub use core::custom_event;
pub use core::custom_timer;
pub use core::float_formatter;
pub use core::skin_float;
pub use core::skin_property_mapper;
pub use core::stretch_type;

// types/
pub use types::select_bar_data;
pub use types::skin;
pub use types::skin_bar_object;
pub use types::skin_header;
pub use types::skin_object;
pub use types::skin_type;

// text/
pub use text::skin_text;
pub use text::skin_text_bitmap;
pub use text::skin_text_font;
pub use text::skin_text_image;

// objects/
pub use objects::skin_bga_object;
pub use objects::skin_gauge;
pub use objects::skin_gauge_graph_object;
pub use objects::skin_hidden;
pub use objects::skin_image;
pub use objects::skin_judge_object;
pub use objects::skin_note_object;
pub use objects::skin_number;
pub use objects::skin_slider;

// graphs/
pub use graphs::skin_bpm_graph;
pub use graphs::skin_graph;
pub use graphs::skin_hit_error_visualizer;
pub use graphs::skin_note_distribution_graph;
pub use graphs::skin_timing_distribution_graph;
pub use graphs::skin_timing_visualizer;

// loaders/
pub use loaders::bitmap_font_batch_loader;
// bitmap_font_cache is now a top-level module; loaders::bitmap_font_cache re-exports it
pub use loaders::pomyu_chara_loader;
pub use loaders::skin_data_converter;
pub use loaders::skin_loader;

// sources/
pub use sources::skin_source;
pub use sources::skin_source_image;
pub use sources::skin_source_image_set;
pub use sources::skin_source_movie;
pub use sources::skin_source_reference;
pub use sources::skin_source_set;

// Re-export safe_div_f32 from rubato-skin-model for backwards compatibility.
pub(crate) use rubato_skin_model::safe_div_f32;
