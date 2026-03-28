// GameScreen -- concrete enum wrapping all production state types.
//
// Replaces `Box<dyn MainState>` with a closed enum so that state dispatch
// becomes a static match instead of a vtable call. Each variant holds the
// concrete state struct directly.

use crate::core::app_context::GameContext;
use crate::core::config_pkg::key_configuration::KeyConfiguration;
use crate::core::config_pkg::skin_configuration::SkinConfiguration;
use crate::core::main_state::{
    MainState, MainStateData, MainStateType, StateCreateEffects, StateTransition,
};
use crate::play::bms_player::BMSPlayer;
use crate::decide::music_decide::MusicDecide;
use crate::result::course_result::CourseResult;
use crate::result::music_result::MusicResult;
use crate::select::music_selector::MusicSelector;
use rubato_render::sprite_batch::SpriteBatch;

use crate::state_factory::shared_selector::SharedMusicSelectorState;

/// Concrete enum of all production game screens.
///
/// Each variant holds the state struct that implements `MainState`.
/// Inherent methods below delegate every call via match dispatch,
/// preserving the same semantics as the previous `Box<dyn MainState>` approach
/// but without dynamic dispatch overhead. Each variant is boxed to keep the
/// enum size small (pointer-sized) while retaining static dispatch via match.
pub enum GameScreen {
    Select(Box<MusicSelector>),
    SharedSelect(Box<SharedMusicSelectorState>),
    Decide(Box<MusicDecide>),
    Play(Box<BMSPlayer>),
    Result(Box<MusicResult>),
    CourseResult(Box<CourseResult>),
    Config(Box<KeyConfiguration>),
    SkinConfig(Box<SkinConfiguration>),
    /// Test-only variant for mock states used in unit tests.
    #[cfg(any(test, feature = "test-support"))]
    Mock(Box<dyn MainState>),
}

// Note: GameScreen is not Send because some inner states hold non-Send trait
// objects (dyn MainControllerAccess, dyn AudioDriver). This matches the existing
// Box<dyn MainState> usage which is also not Send. Making it Send requires adding
// Send bounds to those trait objects, which is a separate task.

/// Generates a match arm that delegates a method call to the inner state
/// for every GameScreen variant.
macro_rules! delegate {
    // &self method, no extra args, with return type
    ($self:ident, $method:ident ( ) -> $ret:ty) => {
        match $self {
            GameScreen::Select(s) => s.$method(),
            GameScreen::SharedSelect(s) => s.$method(),
            GameScreen::Decide(s) => s.$method(),
            GameScreen::Play(s) => s.$method(),
            GameScreen::Result(s) => s.$method(),
            GameScreen::CourseResult(s) => s.$method(),
            GameScreen::Config(s) => s.$method(),
            GameScreen::SkinConfig(s) => s.$method(),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method(),
        }
    };
    // &mut self method, no extra args, with return type
    (mut $self:ident, $method:ident ( ) -> $ret:ty) => {
        match $self {
            GameScreen::Select(s) => s.$method(),
            GameScreen::SharedSelect(s) => s.$method(),
            GameScreen::Decide(s) => s.$method(),
            GameScreen::Play(s) => s.$method(),
            GameScreen::Result(s) => s.$method(),
            GameScreen::CourseResult(s) => s.$method(),
            GameScreen::Config(s) => s.$method(),
            GameScreen::SkinConfig(s) => s.$method(),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method(),
        }
    };
    // &self method, with args, with return type
    ($self:ident, $method:ident ( $($arg:ident),+ ) -> $ret:ty) => {
        match $self {
            GameScreen::Select(s) => s.$method($($arg),+),
            GameScreen::SharedSelect(s) => s.$method($($arg),+),
            GameScreen::Decide(s) => s.$method($($arg),+),
            GameScreen::Play(s) => s.$method($($arg),+),
            GameScreen::Result(s) => s.$method($($arg),+),
            GameScreen::CourseResult(s) => s.$method($($arg),+),
            GameScreen::Config(s) => s.$method($($arg),+),
            GameScreen::SkinConfig(s) => s.$method($($arg),+),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method($($arg),+),
        }
    };
    // &mut self method, with args, with return type
    (mut $self:ident, $method:ident ( $($arg:ident),+ ) -> $ret:ty) => {
        match $self {
            GameScreen::Select(s) => s.$method($($arg),+),
            GameScreen::SharedSelect(s) => s.$method($($arg),+),
            GameScreen::Decide(s) => s.$method($($arg),+),
            GameScreen::Play(s) => s.$method($($arg),+),
            GameScreen::Result(s) => s.$method($($arg),+),
            GameScreen::CourseResult(s) => s.$method($($arg),+),
            GameScreen::Config(s) => s.$method($($arg),+),
            GameScreen::SkinConfig(s) => s.$method($($arg),+),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method($($arg),+),
        }
    };
    // &mut self method, no extra args, no return
    (mut $self:ident, $method:ident ( )) => {
        match $self {
            GameScreen::Select(s) => s.$method(),
            GameScreen::SharedSelect(s) => s.$method(),
            GameScreen::Decide(s) => s.$method(),
            GameScreen::Play(s) => s.$method(),
            GameScreen::Result(s) => s.$method(),
            GameScreen::CourseResult(s) => s.$method(),
            GameScreen::Config(s) => s.$method(),
            GameScreen::SkinConfig(s) => s.$method(),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method(),
        }
    };
    // &mut self method, with args, no return
    (mut $self:ident, $method:ident ( $($arg:ident),+ )) => {
        match $self {
            GameScreen::Select(s) => s.$method($($arg),+),
            GameScreen::SharedSelect(s) => s.$method($($arg),+),
            GameScreen::Decide(s) => s.$method($($arg),+),
            GameScreen::Play(s) => s.$method($($arg),+),
            GameScreen::Result(s) => s.$method($($arg),+),
            GameScreen::CourseResult(s) => s.$method($($arg),+),
            GameScreen::Config(s) => s.$method($($arg),+),
            GameScreen::SkinConfig(s) => s.$method($($arg),+),
            #[cfg(any(test, feature = "test-support"))]
            GameScreen::Mock(s) => s.$method($($arg),+),
        }
    };
}

impl GameScreen {
    pub fn state_type(&self) -> Option<MainStateType> {
        delegate!(self, state_type() -> Option<MainStateType>)
    }

    pub fn main_state_data(&self) -> &MainStateData {
        delegate!(self, main_state_data() -> &MainStateData)
    }

    pub fn main_state_data_mut(&mut self) -> &mut MainStateData {
        delegate!(mut self, main_state_data_mut() -> &mut MainStateData)
    }

    pub fn create(&mut self) {
        delegate!(mut self, create())
    }

    pub fn prepare(&mut self) {
        delegate!(mut self, prepare())
    }

    pub fn shutdown(&mut self) {
        delegate!(mut self, shutdown())
    }

    pub fn render(&mut self) {
        delegate!(mut self, render())
    }

    pub fn input(&mut self) {
        delegate!(mut self, input())
    }

    pub fn render_with_game_context(&mut self, ctx: &mut GameContext) -> StateTransition {
        delegate!(mut self, render_with_game_context(ctx) -> StateTransition)
    }

    pub fn input_with_game_context(&mut self, ctx: &mut GameContext) {
        delegate!(mut self, input_with_game_context(ctx))
    }

    pub fn sync_input_from(
        &mut self,
        input: &rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        delegate!(mut self, sync_input_from(input))
    }

    pub fn sync_input_back_to(
        &mut self,
        input: &mut rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        delegate!(mut self, sync_input_back_to(input))
    }

    pub fn sync_input_snapshot(&mut self, snapshot: &rubato_input::input_snapshot::InputSnapshot) {
        delegate!(mut self, sync_input_snapshot(snapshot))
    }

    pub fn sync_audio(&mut self, audio: &mut rubato_audio::audio_system::AudioSystem) {
        delegate!(mut self, sync_audio(audio))
    }

    pub fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        delegate!(mut self, handle_skin_mouse_pressed(button, x, y))
    }

    pub fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        delegate!(mut self, handle_skin_mouse_dragged(button, x, y))
    }

    pub fn render_skin(&mut self, sprite: &mut SpriteBatch) {
        delegate!(mut self, render_skin(sprite))
    }

    pub fn pause(&mut self) {
        delegate!(mut self, pause())
    }

    pub fn resume(&mut self) {
        delegate!(mut self, resume())
    }

    pub fn resize(&mut self, width: i32, height: i32) {
        delegate!(mut self, resize(width, height))
    }

    pub fn dispose(&mut self) {
        delegate!(mut self, dispose())
    }

    pub fn execute_event_id(&mut self, id: i32) {
        delegate!(mut self, execute_event_id(id))
    }

    pub fn execute_event_id_arg(&mut self, id: i32, arg: i32) {
        delegate!(mut self, execute_event_id_arg(id, arg))
    }

    pub fn execute_event_id_args(&mut self, id: i32, arg1: i32, arg2: i32) {
        delegate!(mut self, execute_event_id_args(id, arg1, arg2))
    }

    pub fn score_data_property(&self) -> &crate::core::score_data_property::ScoreDataProperty {
        delegate!(self, score_data_property() -> &crate::core::score_data_property::ScoreDataProperty)
    }

    pub fn score_data_property_mut(
        &mut self,
    ) -> &mut crate::core::score_data_property::ScoreDataProperty {
        delegate!(mut self, score_data_property_mut() -> &mut crate::core::score_data_property::ScoreDataProperty)
    }

    pub fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        delegate!(self, judge_count(judge, fast) -> i32)
    }

    pub fn groove_gauge_value(&self) -> Option<f32> {
        delegate!(self, groove_gauge_value() -> Option<f32>)
    }

    pub fn get_image(&self, imageid: i32) -> Option<rubato_render::texture::TextureRegion> {
        delegate!(self, get_image(imageid) -> Option<rubato_render::texture::TextureRegion>)
    }

    pub fn sound(&self, sound: rubato_skin::sound_type::SoundType) -> Option<String> {
        delegate!(self, sound(sound) -> Option<String>)
    }

    pub fn play_sound(&mut self, sound: rubato_skin::sound_type::SoundType) {
        delegate!(mut self, play_sound(sound))
    }

    pub fn play_sound_loop(&mut self, sound: rubato_skin::sound_type::SoundType, loop_sound: bool) {
        delegate!(mut self, play_sound_loop(sound, loop_sound))
    }

    pub fn stop_sound(&mut self, sound: rubato_skin::sound_type::SoundType) {
        delegate!(mut self, stop_sound(sound))
    }

    pub fn load_skin(&mut self, skin_type: i32) {
        delegate!(mut self, load_skin(skin_type))
    }

    pub fn get_offset_value(&self, id: i32) -> Option<()> {
        delegate!(self, get_offset_value(id) -> Option<()>)
    }

    pub fn take_state_create_effects(&mut self) -> Option<StateCreateEffects> {
        delegate!(mut self, take_state_create_effects() -> Option<StateCreateEffects>)
    }

    pub fn notify_media_load_finished(&mut self) {
        delegate!(mut self, notify_media_load_finished())
    }

    pub fn update_loading_progress(&mut self, audio_progress: f32, bga_on: bool) {
        delegate!(mut self, update_loading_progress(audio_progress, bga_on))
    }

    pub fn receive_updated_play_config(
        &mut self,
        mode: bms::model::mode::Mode,
        play_config: rubato_skin::play_config::PlayConfig,
    ) {
        delegate!(mut self, receive_updated_play_config(mode, play_config))
    }

    pub fn receive_reloaded_model(&mut self, model: bms::model::bms_model::BMSModel) {
        delegate!(mut self, receive_reloaded_model(model))
    }

    pub fn take_bga_cache(
        &mut self,
    ) -> Option<std::sync::Arc<std::sync::Mutex<crate::play::bga::bga_processor::BGAProcessor>>>
    {
        delegate!(mut self, take_bga_cache() -> Option<std::sync::Arc<std::sync::Mutex<crate::play::bga::bga_processor::BGAProcessor>>>)
    }

    pub fn take_player_resource(&mut self) -> Option<crate::core::player_resource::PlayerResource> {
        delegate!(mut self, take_player_resource() -> Option<crate::core::player_resource::PlayerResource>)
    }

    pub fn bms_model(&self) -> Option<&bms::model::bms_model::BMSModel> {
        delegate!(self, bms_model() -> Option<&bms::model::bms_model::BMSModel>)
    }
}
