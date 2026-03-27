use std::collections::HashMap;

use crate::sprite_batch::SpriteBatch;
use rubato_types::skin_offset::SkinOffset;

/// Abstracts the beatoraja-skin Skin type so that beatoraja-core can call
/// skin drawing methods without depending on the skin crate (circular dep).
/// The concrete implementation lives in beatoraja-skin::Skin.
///
/// Translated from: Java Skin.drawAllObjects(), updateCustomObjects(), etc.
pub trait SkinDrawable: Send {
    /// Draw all skin objects for the current frame.
    ///
    /// `ctx` provides timer state plus optional MainController capabilities
    /// (event execution, state changes, audio, timer writes).
    fn draw_all_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    );

    /// Update custom timers and events.
    fn update_custom_objects_timed(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
    );

    /// Handle mouse press events (reverse order iteration).
    fn mouse_pressed_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    );

    /// Handle mouse drag events (slider objects only).
    fn mouse_dragged_at(
        &mut self,
        ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        button: i32,
        x: i32,
        y: i32,
    );

    /// Prepare skin for rendering: validate objects, build draw list, load resources.
    fn prepare_skin(&mut self, state_type: Option<rubato_types::main_state_type::MainStateType>);

    /// Dispose all skin objects and release resources.
    fn dispose_skin(&mut self);

    /// Returns the skin's offset configuration entries as (id, SkinOffset) pairs.
    /// Used by MainController to populate MainStateData.offsets during skin loading,
    /// mirroring Java's MainState.setSkin() which copies skin.offset into MainController.offset[].
    fn skin_offsets(&self) -> HashMap<i32, SkinOffset> {
        HashMap::new()
    }

    /// Compute and store note draw commands for the SkinNoteObject.
    ///
    /// The `compute` closure takes `&[SkinLane]` and returns `Vec<DrawCommand>`.
    /// This closure captures the LaneRenderer and DrawLaneContext from
    /// rubato-play, avoiding circular dependencies.
    fn compute_note_draw_commands(
        &mut self,
        _compute: &mut dyn FnMut(
            &[rubato_types::skin_note::SkinLane],
        ) -> Vec<rubato_types::draw_command::DrawCommand>,
    ) {
        // default no-op
    }

    /// Get fadeout duration in milliseconds.
    fn fadeout(&self) -> i32;

    /// Get input start time in milliseconds.
    fn input(&self) -> i32;

    /// Get scene time in milliseconds.
    fn scene(&self) -> i32;

    /// Get skin width.
    fn get_width(&self) -> f32;

    /// Get skin height.
    fn get_height(&self) -> f32;

    /// Swap the internal SpriteBatch with the given one.
    /// Used to let the skin draw into MainController's SpriteBatch.
    fn swap_sprite_batch(&mut self, batch: &mut SpriteBatch);

    /// Returns the skin's offset config entries as (id, SkinOffset) pairs.
    /// Used by MainController to copy skin config offsets into the runtime offset array
    /// after skin loading (Java: MainState.setSkin() copies skin.getOffset() into main.offset[]).
    fn offset_entries(&self) -> Vec<(i32, rubato_types::skin_offset::SkinOffset)> {
        Vec::new()
    }

    /// Execute a custom skin event by ID.
    /// Custom events (1000-1999) are defined by the skin and stored in a HashMap.
    /// This method is called to replay events that were queued during mouse handling,
    /// where the skin was borrowed and could not dispatch events directly.
    fn execute_custom_event(
        &mut self,
        _ctx: &mut dyn rubato_types::skin_render_context::SkinRenderContext,
        _id: i32,
        _arg1: i32,
        _arg2: i32,
    ) {
        // default no-op
    }

    /// Return play-skin-specific metadata (loadstart, loadend, playstart, etc.).
    /// Only meaningful for play skins; other skin types return defaults.
    fn play_skin_properties(&self) -> PlaySkinProperties {
        PlaySkinProperties::default()
    }

    /// Register a texture by image ID for SkinSourceReference resolution at draw time.
    ///
    /// Used to inject BMS resource images (stagefile=100, backbmp=101, banner=102)
    /// into the skin's image registry so that SkinSourceReference-backed objects
    /// can render them. In Java, MainState.getImage(id) accesses these directly
    /// from the BMSResource; in Rust, we populate the skin's registry instead.
    fn register_image(&mut self, _id: i32, _texture: crate::texture::TextureRegion) {
        // default no-op
    }
}

/// Play-skin-specific metadata extracted from the loaded skin.
/// Corresponds to Java's PlaySkin fields that are not on the base Skin class.
#[derive(Clone, Debug)]
pub struct PlaySkinProperties {
    pub loadstart: i32,
    pub loadend: i32,
    pub playstart: i32,
    pub close: i32,
    pub finish_margin: i32,
    pub judgetimer: i32,
    pub judgeregion: i32,
    pub note_expansion_rate: [i32; 2],
}

impl Default for PlaySkinProperties {
    fn default() -> Self {
        Self {
            loadstart: 0,
            loadend: 0,
            playstart: 0,
            close: 0,
            finish_margin: 0,
            judgetimer: 1,
            judgeregion: 0,
            note_expansion_rate: [100, 100],
        }
    }
}
