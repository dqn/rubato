//! Rendering engine built on Bevy for 2D sprite-based skin display.
//!
//! Uses Bevy 0.15 for window, camera, and 2D sprite rendering.
//! [`skin_renderer::skin_render_system`] iterates skin objects each frame in a
//! procedural render loop. Includes [`bga`] for background animation layers,
//! [`blend`] for custom blend modes, [`font_map`] for bitmap font atlases,
//! and [`mod_menu`] for the in-game debug/mod overlay.

pub mod bga;
pub mod bga_layer_material;
pub mod blend;
pub mod coord;
pub mod distance_field_material;
pub mod draw;
pub mod embedded_textures;
pub mod eval;
pub mod font_map;
pub mod image_loader_bevy;
pub mod message_renderer;
pub mod mod_menu;
pub mod plugin;
pub mod skin_renderer;
pub mod state_provider;
pub mod texture_map;
