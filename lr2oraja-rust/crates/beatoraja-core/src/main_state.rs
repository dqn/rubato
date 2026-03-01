use crate::score_data_property::ScoreDataProperty;
use crate::timer_manager::TimerManager;
use beatoraja_types::sound_type::SoundType;

// MainStateType moved to beatoraja-types (Phase 15d)
pub use beatoraja_types::main_state_type::MainStateType;

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
        data.skin = None;
        data.stage = None;
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

    fn get_score_data_property(&self) -> &ScoreDataProperty {
        &self.main_state_data().score
    }

    fn get_score_data_property_mut(&mut self) -> &mut ScoreDataProperty {
        &mut self.main_state_data_mut().score
    }

    fn get_judge_count(&self, judge: i32, fast: bool) -> i32 {
        let score = &self.main_state_data().score;
        if let Some(sd) = score.get_score_data() {
            sd.get_judge_count(judge, fast)
        } else {
            0
        }
    }

    fn get_image(&self, _imageid: i32) -> Option<()> {
        // Default no-op: concrete states override to return TextureRegion from PlayerResource.
        // Skin rendering uses the skin crate's MainState trait (separate from this trait).
        None
    }

    fn get_sound(&self, _sound: SoundType) -> Option<String> {
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
        ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
    );

    /// Update custom timers and events.
    fn update_custom_objects_timed(
        &mut self,
        ctx: &mut dyn beatoraja_types::skin_render_context::SkinRenderContext,
    );

    /// Handle mouse press events (reverse order iteration).
    fn mouse_pressed_at(&mut self, button: i32, x: i32, y: i32);

    /// Handle mouse drag events (slider objects only).
    fn mouse_dragged_at(&mut self, button: i32, x: i32, y: i32);

    /// Prepare skin for rendering: validate objects, build draw list, load resources.
    fn prepare_skin(&mut self);

    /// Dispose all skin objects and release resources.
    fn dispose_skin(&mut self);

    /// Get fadeout duration in milliseconds.
    fn get_fadeout(&self) -> i32;

    /// Get input start time in milliseconds.
    fn get_input(&self) -> i32;

    /// Get scene time in milliseconds.
    fn get_scene(&self) -> i32;

    /// Get skin width.
    fn get_width(&self) -> f32;

    /// Get skin height.
    fn get_height(&self) -> f32;
}

/// Shared data for MainState implementations
pub struct MainStateData {
    /// Skin (real Skin type via SkinDrawable trait)
    pub skin: Option<Box<dyn SkinDrawable>>,
    /// Stage (scene2d)
    pub stage: Option<StageStub>,
    /// Timer manager reference
    pub timer: TimerManager,
    /// Score data property
    pub score: ScoreDataProperty,
}

impl MainStateData {
    pub fn new(timer: TimerManager) -> Self {
        Self {
            skin: None,
            stage: None,
            timer,
            score: ScoreDataProperty::new(),
        }
    }
}

// Phase 5+ stubs for types used in MainState
pub struct StageStub;
