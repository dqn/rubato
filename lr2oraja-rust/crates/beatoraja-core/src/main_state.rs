use crate::score_data_property::ScoreDataProperty;
use crate::system_sound_manager::SoundType;
use crate::timer_manager::TimerManager;

/// MainStateType - enum for each state in the application
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MainStateType {
    MusicSelect,
    Decide,
    Play,
    Result,
    CourseResult,
    Config,
    SkinConfig,
}

/// MainState - abstract class for each state in the player
///
/// In Java this is an abstract class with fields. In Rust we use a trait
/// plus a shared data struct for common fields.
pub trait MainState {
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

    fn execute_event_id_args(&mut self, id: i32, _arg1: i32, _arg2: i32) {
        // SkinPropertyMapper.isCustomEventId(id) check
        let _ = id;
        todo!("Phase 5+ dependency: skin.executeCustomEvent")
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
        todo!("Phase 5+ dependency: TextureRegion/image resources")
    }

    fn get_sound(&self, _sound: SoundType) -> Option<String> {
        todo!("Phase 5+ dependency: MainController.getSoundManager()")
    }

    fn play_sound(&mut self, sound: SoundType) {
        self.play_sound_loop(sound, false);
    }

    fn play_sound_loop(&mut self, _sound: SoundType, _loop_sound: bool) {
        todo!("Phase 5+ dependency: MainController.getSoundManager().play()")
    }

    fn stop_sound(&mut self, _sound: SoundType) {
        todo!("Phase 5+ dependency: MainController.getSoundManager().stop()")
    }
}

/// Shared data for MainState implementations
pub struct MainStateData {
    /// Skin
    pub skin: Option<SkinStub>,
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
pub struct SkinStub;
pub struct StageStub;
