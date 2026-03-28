use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::core::app_context::GameContext;
use crate::core::score_data_property::ScoreDataProperty;
use crate::core::timer_manager::TimerManager;
use rubato_audio::audio_system::AudioSystem;
use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
use rubato_input::input_snapshot::InputSnapshot;
use rubato_render::sprite_batch::SpriteBatch;
use rubato_types::skin_offset::SkinOffset;
use rubato_types::sound_type::SoundType;

// MainStateType moved to beatoraja-types (Phase 15d)
pub use rubato_types::main_state_type::MainStateType;

/// Result of a state's render cycle, indicating what should happen next.
///
/// Returned by `render_with_game_context` on `MainState`. States return
/// this to signal whether to continue, change state, or exit.
#[derive(Debug, Clone, PartialEq)]
pub enum StateTransition {
    /// Continue running the current state.
    Continue,
    /// Transition to a different state.
    ChangeTo(MainStateType),
    /// Exit the application.
    Exit,
}

/// Side effects from state creation that MainController must apply.
///
/// Populated by `BMSPlayer::create()` and consumed by `transition_to_state()`.
/// Since `create()` takes only `&mut self`, it cannot directly access external
/// systems (input processor, audio driver). Instead, it stores the needed actions
/// here and the controller applies them after `create()` returns.
pub struct StateCreateEffects {
    /// If Some, call `input.set_play_config()` with this mode's play config.
    /// Used for PLAY and PRACTICE modes.
    pub play_config_mode: Option<bms::model::mode::Mode>,
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

    fn render(&mut self) {
        // default empty -- states use render_with_game_context instead
    }

    fn input(&mut self) {
        // default empty
    }

    /// Render with direct access to the application context.
    ///
    /// All states implement this method to handle rendering and drain
    /// outbox fields into GameContext. Returns a `StateTransition` indicating
    /// whether to continue, change state, or exit.
    fn render_with_game_context(&mut self, _ctx: &mut GameContext) -> StateTransition {
        StateTransition::Continue
    }

    /// Process input with direct access to the application context.
    ///
    /// All states implement this method to handle input processing
    /// with access to GameContext for audio, config, and other shared resources.
    fn input_with_game_context(&mut self, _ctx: &mut GameContext) {
        // default empty
    }

    /// Sync live controller input into a state-local wrapper before `input()`.
    fn sync_input_from(&mut self, _input: &BMSPlayerInputProcessor) {
        // default empty
    }

    /// Receive a read-only snapshot of the current frame's input state.
    ///
    /// Called by MainController after polling input and before `input()`.
    /// States can use this to read input without depending on
    /// BMSPlayerInputProcessor directly. The default implementation is
    /// empty; states opt in by overriding.
    ///
    /// This coexists with `sync_input_from` during migration. Once all
    /// states are migrated, `sync_input_from` / `sync_input_back_to` can
    /// be removed.
    fn sync_input_snapshot(&mut self, _snapshot: &InputSnapshot) {
        // default empty
    }

    /// Flush consumed state-local input back to the controller after `input()`.
    fn sync_input_back_to(&mut self, _input: &mut BMSPlayerInputProcessor) {
        // default empty
    }

    /// Give the state one chance per frame to synchronize preview/background audio
    /// using the live audio driver owned by MainController.
    fn sync_audio(&mut self, _audio: &mut AudioSystem) {
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

    /// Returns the current groove gauge value, or None if no gauge exists.
    ///
    /// Concrete states (e.g. BMSPlayer) override to return the active gauge value.
    fn groove_gauge_value(&self) -> Option<f32> {
        None
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

    /// Take side effects produced by create() for the controller to apply.
    ///
    /// BMSPlayer overrides this to return input mode actions and guide SE flags.
    fn take_state_create_effects(&mut self) -> Option<StateCreateEffects> {
        None
    }

    // --- Inbox pattern methods ---
    // MainController pushes data back into the current state after processing outbox items.

    /// Notify the state that media loading is complete.
    /// Called by MainController each frame when resource.media_load_finished() is true.
    fn notify_media_load_finished(&mut self) {
        // Default no-op — only BMSPlayer uses this.
    }

    /// Update gradual loading progress values each frame.
    /// Called by MainController with audio driver progress and whether BGA is enabled.
    /// BMSPlayer reads its own BGA progress from the BGAProcessor it owns.
    /// Only BMSPlayer uses this for the skin property ID 165 loading bar.
    fn update_loading_progress(&mut self, _audio_progress: f32, _bga_on: bool) {
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
        _mode: bms::model::mode::Mode,
        _play_config: rubato_types::play_config::PlayConfig,
    ) {
        // Default no-op — only BMSPlayer uses this.
    }

    /// Receive a reloaded BMS model from resource after reload_bms_file().
    /// Used by practice mode to get a fresh model without a full state change.
    fn receive_reloaded_model(&mut self, _model: bms::model::bms_model::BMSModel) {
        // Default no-op — only BMSPlayer uses this for practice mode restart.
    }

    /// Take the BGA processor for caching on MainController/PlayerResource.
    ///
    /// Called during state transition when leaving Play state.
    /// BMSPlayer overrides this to return its `Arc<Mutex<BGAProcessor>>`
    /// so it can be reused when entering Play state again, preserving the texture cache.
    ///
    /// Java: BGAProcessor lives in BMSResource and is never destroyed between plays.
    fn take_bga_cache(
        &mut self,
    ) -> Option<Arc<Mutex<crate::play::bga::bga_processor::BGAProcessor>>> {
        None
    }

    /// Take the PlayerResource from this state.
    ///
    /// Called during state transition so MainController can restore the resource.
    /// States that received a PlayerResource via the factory override this to return it.
    fn take_player_resource(&mut self) -> Option<crate::core::player_resource::PlayerResource> {
        None
    }

    /// Return the BMS model if this state owns one.
    ///
    /// Used by MainController to call audio.set_model() during state transition
    /// so keysounds are loaded before playback begins.
    fn bms_model(&self) -> Option<&bms::model::bms_model::BMSModel> {
        None
    }

    /// Downcast to `&dyn Any` for concrete type recovery.
    ///
    /// Concrete wrapper types (e.g. `GameScreen`) override this to return
    /// `Some(self)`, enabling callers to downcast `&dyn MainState` back to
    /// the concrete enum when pattern matching is needed. The default
    /// returns `None` for types that cannot be downcast (e.g. those with
    /// non-`'static` lifetimes).
    fn as_any(&self) -> Option<&dyn Any> {
        None
    }

    /// Downcast to `&mut dyn Any` for concrete type recovery.
    fn as_any_mut(&mut self) -> Option<&mut dyn Any> {
        None
    }
}

// Re-exported from rubato-render (canonical location)
pub use rubato_render::skin_drawable::PlaySkinProperties;
pub use rubato_render::skin_drawable::SkinDrawable;

/// Shared data for MainState implementations
pub struct MainStateData {
    /// Skin (real Skin type via SkinDrawable trait)
    pub skin: Option<Box<dyn SkinDrawable>>,
    /// Timer manager reference
    pub timer: TimerManager,
    /// Score data property
    pub score: ScoreDataProperty,
    /// Skin offset values, populated from skin config during skin loading.
    /// Keyed by offset ID, queried by skin objects during prepare().
    /// Mirrors Java's MainController.offset[] array.
    pub offsets: HashMap<i32, SkinOffset>,
}

impl MainStateData {
    pub fn new(timer: TimerManager) -> Self {
        Self {
            skin: None,
            timer,
            score: ScoreDataProperty::new(),
            offsets: HashMap::new(),
        }
    }
}
