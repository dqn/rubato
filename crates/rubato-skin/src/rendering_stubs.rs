// Rendering types — re-exported from beatoraja-render (Phase 13c-2).
// All types previously defined as stubs here are now backed by real wgpu implementations.

// Re-export all public types from beatoraja-render
pub use rubato_render::blend::BlendMode;
pub use rubato_render::blend::gl11;
pub use rubato_render::blend::gl20;
pub use rubato_render::color::{Color, Matrix4, Rectangle};
pub use rubato_render::font::{
    BitmapFont, BitmapFontData, FreeTypeFontGenerator, FreeTypeFontParameter, GlyphLayout,
};
pub use rubato_render::pixmap::{BlitRect, Pixmap, PixmapFormat};
pub use rubato_render::shader::ShaderProgram;
pub use rubato_render::sprite_batch::SpriteBatch;
pub use rubato_render::texture::{Texture, TextureFilter, TextureRegion};
pub use rubato_render::{FileHandle, Gdx};
