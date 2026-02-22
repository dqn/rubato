use crate::Texture;
use crate::bga::movie_processor::MovieProcessor;

/// GDX video processor (stub implementation)
pub struct GdxVideoProcessor;

impl Default for GdxVideoProcessor {
    fn default() -> Self {
        Self::new()
    }
}

impl GdxVideoProcessor {
    pub fn new() -> Self {
        GdxVideoProcessor
    }

    /// Get shader program. Returns None (null in Java).
    /// Corresponds to Java getShader() which returns null ShaderProgram.
    pub fn get_shader(&self) -> Option<()> {
        None
    }
}

impl MovieProcessor for GdxVideoProcessor {
    fn get_frame(&mut self, _time: i64) -> Option<Texture> {
        None
    }

    fn play(&mut self, _time: i64, _loop_play: bool) {
        // stub
    }

    fn stop(&mut self) {
        // stub
    }

    fn dispose(&mut self) {
        // stub
    }
}
