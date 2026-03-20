use crate::score_data_property::ScoreDataProperty;
use crate::timer_manager::TimerManager;
use rubato_audio::audio_driver::AudioDriver;
use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
use rubato_render::sprite_batch::SpriteBatch;
use rubato_types::sound_type::SoundType;

// MainStateType moved to beatoraja-types (Phase 15d)
pub use rubato_types::main_state_type::MainStateType;

/// Side effects from state creation that MainController must apply.
///
/// Populated by `BMSPlayer::create()` and consumed by `transition_to_state()`.
/// Since `create()` takes only `&mut self`, it cannot directly access external
/// systems (input processor, audio driver). Instead, it stores the needed actions
/// here and the controller applies them after `create()` returns.
pub struct StateCreateEffects {
    /// If Some, call `input.set_play_config()` with this mode's play config.
    /// Used for PLAY and PRACTICE modes.
    pub play_config_mode: Option<bms_model::mode::Mode>,
    /// If true, call `input.set_enable(false)` for AUTOPLAY/REPLAY.
    pub disable_input: bool,
    /// If true, guide SE should be loaded into the audio driver.
    pub guide_se: bool,
}

/// MainState - abstract class for each state in the player
///
/// In Java this is an abstract class with fields. In Rust we use a trait
/// plus a shared data struct for common fields.
pub trait MainState {
    /// Return the state type for this state.
    ///
    /// Translated from: Java `MainController.getStateType(MainState)` which uses instanceof.
    /// In Rust, each concrete state overrides this to return its own type.
    fn state_type(&self) -> Option<MainStateType> {
        None
    }

    /// Get reference to the shared main state data
    fn main_state_data(&self) -> &MainStateData;

    /// Get mutable reference to the shared main state data
    fn main_state_data_mut(&mut self) -> &mut MainStateData;

    fn create(&mut self);

    fn prepare(&mut self) {
        // default empty
    }

    fn shutdown(&mut self) {
        // default empty
    }

    fn render(&mut self);

    fn input(&mut self) {
        // default empty
    }

    /// Sync live controller input into a state-local wrapper before `input()`.
    fn sync_input_from(&mut self, _input: &BMSPlayerInputProcessor) {
        // default empty
    }

    /// Flush consumed state-local input back to the controller after `input()`.
    fn sync_input_back_to(&mut self, _input: &mut BMSPlayerInputProcessor) {
        // default empty
    }

    /// Give the state one chance per frame to synchronize preview/background audio
    /// using the live audio driver owned by MainController.
    fn sync_audio(&mut self, _audio: &mut dyn AudioDriver) {
        // default empty
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        let data = self.main_state_data_mut();
        if let Some(mut skin) = data.skin.take() {
            skin.mouse_pressed_at(&mut data.timer, button, x, y);
            data.skin = Some(skin);
        }
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        let data = self.main_state_data_mut();
        if let Some(mut skin) = data.skin.take() {
            skin.mouse_dragged_at(&mut data.timer, button, x, y);
            data.skin = Some(skin);
        }
    }

    /// Override point for state-specific rendering within the skin pipeline.
    /// Called by MainController::render() with the sprite batch.
    /// Default: update custom objects + standard skin draw_all_objects cycle.
    fn render_skin(&mut self, sprite: &mut SpriteBatch) {
        let data = self.main_state_data_mut();
        if let Some(mut skin) = data.skin.take() {
            skin.update_custom_objects_timed(&mut data.timer);
            skin.swap_sprite_batch(sprite);
            skin.draw_all_objects_timed(&mut data.timer);
            skin.swap_sprite_batch(sprite);
            data.skin = Some(skin);
        }
    }

    fn pause(&mut self) {
        // default empty
    }

    fn resume(&mut self) {
        // default empty
    }

    fn resize(&mut self, _width: i32, _height: i32) {
        // default empty
    }

    fn dispose(&mut self) {
        let data = self.main_state_data_mut();
        // Java: Optional.ofNullable(skin).ifPresent(skin -> skin.dispose());
        if let Some(ref mut skin) = data.skin {
            skin.dispose_skin();
        }
        data.skin = None;
    }

    fn execute_event_id(&mut self, id: i32) {
        self.execute_event_id_args(id, 0, 0);
    }

    fn execute_event_id_arg(&mut self, id: i32, arg: i32) {
        self.execute_event_id_args(id, arg, 0);
    }

    fn execute_event_id_args(&mut self, _id: i32, _arg1: i32, _arg2: i32) {
        // Default no-op — requires SkinDrawable to expose execute_custom_event.
        // Skin event dispatch needs SkinPropertyMapper.isCustomEventId(id) check
        // and delegation to Skin.executeCustomEvent(state, id, arg1, arg2).
    }

    fn score_data_property(&self) -> &ScoreDataProperty {
        &self.main_state_data().score
    }

    fn score_data_property_mut(&mut self) -> &mut ScoreDataProperty {
        &mut self.main_state_data_mut().score
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        let score = &self.main_state_data().score;
        if let Some(sd) = score.score_data() {
            sd.judge_count(judge, fast)
        } else {
            0
        }
    }

    fn get_image(&self, _imageid: i32) -> Option<rubato_render::texture::TextureRegion> {
        // Default no-op: concrete states override to return TextureRegion from PlayerResource.
        // Skin rendering uses the skin crate's MainState trait (separate from this trait).
        None
    }

    fn sound(&self, _sound: SoundType) -> Option<String> {
        // Default no-op — concrete states override to delegate to MainControllerAccess
        None
    }

    fn play_sound(&mut self, sound: SoundType) {
        self.play_sound_loop(sound, false);
    }

    fn play_sound_loop(&mut self, _sound: SoundType, _loop_sound: bool) {
        // Default no-op — concrete states override to delegate to MainControllerAccess
    }

    fn stop_sound(&mut self, _sound: SoundType) {
        // Default no-op — concrete states override to delegate to MainControllerAccess
    }

    /// Load skin for the given skin type.
    ///
    /// Translated from: MainState.loadSkin(SkinType)
    fn load_skin(&mut self, _skin_type: i32) {
        // Default no-op — concrete states (e.g. MusicSelector) override to call SkinLoader.load()
    }

    /// Get offset value by ID from MainController.
    ///
    /// Translated from: MainState.getOffsetValue(int)
    fn get_offset_value(&self, _id: i32) -> Option<()> {
        // Default no-op — skin rendering uses the skin crate's MainState trait (separate from
        // this trait) via the TimerOnlyMainState adapter, which needs context expansion to
        // carry offset data from MainController.
        None
    }

    // --- Outbox pattern methods ---
    // BMSPlayer overrides these to expose pending operations.
    // MainController polls them after each render() frame.

    /// Take pending state change request (e.g., Play -> Result).
    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        None
    }

    /// Take pending global pitch change (e.g., reset to 1.0 on state transition).
    fn take_pending_global_pitch(&mut self) -> Option<f32> {
        None
    }

    /// Drain pending system sound requests (e.g., PLAY_READY, PLAY_STOP).
    fn drain_pending_sounds(&mut self) -> Vec<(SoundType, bool)> {
        vec![]
    }

    /// Take pending score handoff data for PlayerResource.
    fn take_score_handoff(&mut self) -> Option<rubato_types::score_handoff::ScoreHandoff> {
        None
    }

    /// Take side effects produced by create() for the controller to apply.
    ///
    /// BMSPlayer overrides this to return input mode actions and guide SE flags.
    fn take_state_create_effects(&mut self) -> Option<StateCreateEffects> {
        None
    }

    /// Take pending BMS file reload request (for quick retry).
    fn take_pending_reload_bms(&mut self) -> bool {
        false
    }

    /// Take pending replay seed reset flag (quick retry START/assist).
    /// When true, MainController should set `resource.replay.randomoptionseed = -1`.
    fn take_pending_replay_seed_reset(&mut self) -> bool {
        false
    }

    /// Take pending score data from quick retry (SELECT key).
    /// When Some, MainController should set `resource.score_data` to the value.
    fn take_pending_quick_retry_score(&mut self) -> Option<rubato_types::score_data::ScoreData> {
        None
    }

    /// Take pending play config update to push back to MainController's PlayerConfig.
    ///
    /// In Java, BMSPlayer writes directly to `main.getPlayerConfig()` (shared reference).
    /// In Rust, BMSPlayer owns a clone, so save_config() must push changes back via this outbox.
    fn take_pending_play_config_update(
        &mut self,
    ) -> Option<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)> {
        None
    }

    // --- Inbox pattern methods ---
    // MainController pushes data back into the current state after processing outbox items.

    /// Notify the state that media loading is complete.
    /// Called by MainController each frame when resource.media_load_finished() is true.
    fn notify_media_load_finished(&mut self) {
        // Default no-op — only BMSPlayer uses this.
    }

    /// Receive an updated PlayConfig pushed from MainController after modmenu changes.
    ///
    /// In Java, BMSPlayer accesses `main.getPlayerConfig()` (shared reference), so
    /// modmenu changes take effect immediately when play resumes. In Rust, BMSPlayer
    /// owns a clone of PlayerConfig, so MainController must push updates through
    /// this method to keep the clone in sync.
    fn receive_updated_play_config(
        &mut self,
        _mode: bms_model::mode::Mode,
        _play_config: rubato_types::play_config::PlayConfig,
    ) {
        // Default no-op — only BMSPlayer uses this.
    }

    /// Receive a reloaded BMS model from resource after reload_bms_file().
    /// Used by practice mode to get a fresh model without a full state change.
    fn receive_reloaded_model(&mut self, _model: bms_model::bms_model::BMSModel) {
        // Default no-op — only BMSPlayer uses this for practice mode restart.
    }

    /// Take the BGA processor for caching on MainController/PlayerResource.
    ///
    /// Called during state transition when leaving Play state.
    /// BMSPlayer overrides this to return its `Arc<Mutex<BGAProcessor>>` (type-erased)
    /// so it can be reused when entering Play state again, preserving the texture cache.
    ///
    /// Java: BGAProcessor lives in BMSResource and is never destroyed between plays.
    fn take_bga_cache(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        None
    }

    /// Take the PlayerResource (type-erased) from this state.
    ///
    /// Called during state transition so MainController can restore the resource.
    /// States that received a PlayerResource via the factory override this to return it.
    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        None
    }

    /// Return the BMS model if this state owns one.
    ///
    /// Used by MainController to call audio.set_model() during state transition
    /// so keysounds are loaded before playback begins.
    fn bms_model(&self) -> Option<&bms_model::bms_model::BMSModel> {
        None
    }
}

/// Trait for skin drawing integration.
///
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
    fn prepare_skin(&mut self);

    /// Dispose all skin objects and release resources.
    fn dispose_skin(&mut self);

    /// Compute and store note draw commands for the SkinNoteObject.
    ///
    /// The lane_renderer and ctx are type-erased as `&mut dyn Any` / `Box<dyn Any>`
    /// to avoid circular dependencies (LaneRenderer/DrawLaneContext live in
    /// rubato-play which depends on rubato-core). The concrete Skin implementation
    /// downcasts them and calls `LaneRenderer::draw_lane()` with its own SkinLane data.
    fn compute_note_draw_commands(
        &mut self,
        _lane_renderer: &mut dyn std::any::Any,
        _ctx: Box<dyn std::any::Any>,
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
}

/// Shared data for MainState implementations
pub struct MainStateData {
    /// Skin (real Skin type via SkinDrawable trait)
    pub skin: Option<Box<dyn SkinDrawable>>,
    /// Timer manager reference
    pub timer: TimerManager,
    /// Score data property
    pub score: ScoreDataProperty,
}

impl MainStateData {
    pub fn new(timer: TimerManager) -> Self {
        Self {
            skin: None,
            timer,
            score: ScoreDataProperty::new(),
        }
    }
}
