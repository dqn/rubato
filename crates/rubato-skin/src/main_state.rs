/// Trait for beatoraja.MainState
///
/// Extends `SkinRenderContext` (which extends `TimerAccess`) with methods
/// that require skin-crate-local types (`TextureRegion`, `BMSPlayerMode`).
///
/// All property value, config access, event, gauge, judge, audio, and timer
/// methods are inherited from `SkinRenderContext`.
pub trait MainState: rubato_types::skin_render_context::SkinRenderContext {
    /// Returns the skin image (texture region) for the given reference ID.
    /// Used by SkinSourceReference to look up system-defined images.
    fn skin_image(&self, _id: i32) -> Option<super::render_reexports::TextureRegion> {
        None
    }

    /// Select a song with the given play mode.
    /// Only meaningful for MusicSelector.
    fn select_song(&mut self, _mode: rubato_types::bms_player_mode::BMSPlayerMode) {
        // default no-op
    }
}
