//! Skin data model and loaders for JSON, LR2 CSV, and Lua skin formats.
//!
//! Provides [`skin::Skin`] as the unified skin representation, with loaders
//! in [`loader`] for JSON skins, LR2 CSV skins, and Lua-scripted skins.
//! Key types include [`skin_object::SkinObject`], [`skin_header::SkinHeader`],
//! and various skin element types (image, number, text, gauge, graph, etc.).
//! Consumed by `bms-render` to drive the rendering pipeline.

pub mod bmfont;
pub mod custom_event;
pub mod image_handle;
pub mod loader;
pub mod lr2_font;
pub mod music_select_skin;
pub mod play_skin;
pub mod pomyu_chara_loader;
pub mod property_id;
pub mod property_mapper;
pub mod result_skin;
pub mod skin;
pub mod skin_bar;
pub mod skin_bga;
pub mod skin_bpm_graph;
pub mod skin_distribution_graph;
pub mod skin_float;
pub mod skin_gauge;
pub mod skin_gauge_graph;
pub mod skin_graph;
pub mod skin_header;
pub mod skin_hidden;
pub mod skin_image;
pub mod skin_judge;
pub mod skin_note;
pub mod skin_number;
pub mod skin_object;
pub mod skin_object_type;
pub mod skin_slider;
pub mod skin_source;
pub mod skin_text;
pub mod skin_visualizer;
pub mod stretch_type;
