// Rendering types — re-exported from beatoraja-render (Phase 13c-2).
// All types previously defined as stubs here are now backed by real wgpu implementations.

// Re-export all public types from beatoraja-render
pub use beatoraja_render::blend::gl11;
pub use beatoraja_render::blend::gl20;
pub use beatoraja_render::color::{Color, Matrix4, Rectangle};
pub use beatoraja_render::font::{
    BitmapFont, BitmapFontData, FreeTypeFontGenerator, FreeTypeFontParameter, GlyphLayout,
};
pub use beatoraja_render::pixmap::{Pixmap, PixmapFormat};
pub use beatoraja_render::shader::ShaderProgram;
pub use beatoraja_render::sprite_batch::SpriteBatch;
pub use beatoraja_render::texture::{Texture, TextureFilter, TextureRegion};
pub use beatoraja_render::{FileHandle, Gdx};
