// GameScreen -- concrete enum wrapping all production state types.
//
// Replaces `Box<dyn MainState>` with a closed enum so that state dispatch
// becomes a static match instead of a vtable call. Each variant holds the
// concrete state struct directly.

use rubato_core::app_context::AppContext;
use rubato_core::config_pkg::key_configuration::KeyConfiguration;
use rubato_core::config_pkg::skin_configuration::SkinConfiguration;
use rubato_core::main_state::{MainState, MainStateData, MainStateType, StateCreateEffects};
use rubato_play::bms_player::BMSPlayer;
use rubato_render::sprite_batch::SpriteBatch;
use rubato_state::decide::music_decide::MusicDecide;
use rubato_state::result::course_result::CourseResult;
use rubato_state::result::music_result::MusicResult;
use rubato_state::select::music_selector::MusicSelector;

use crate::state_factory::shared_selector::SharedMusicSelectorState;

/// Concrete enum of all production game screens.
///
/// Each variant holds the state struct that implements `MainState`.
/// The `MainState` impl below delegates every method via match dispatch,
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
        }
    };
}

impl MainState for GameScreen {
    fn state_type(&self) -> Option<MainStateType> {
        delegate!(self, state_type() -> Option<MainStateType>)
    }

    fn main_state_data(&self) -> &MainStateData {
        delegate!(self, main_state_data() -> &MainStateData)
    }

    fn main_state_data_mut(&mut self) -> &mut MainStateData {
        delegate!(mut self, main_state_data_mut() -> &mut MainStateData)
    }

    fn create(&mut self) {
        delegate!(mut self, create())
    }

    fn prepare(&mut self) {
        delegate!(mut self, prepare())
    }

    fn shutdown(&mut self) {
        delegate!(mut self, shutdown())
    }

    fn render(&mut self) {
        delegate!(mut self, render())
    }

    fn render_with_ctx(&mut self, ctx: &mut AppContext) {
        delegate!(mut self, render_with_ctx(ctx))
    }

    fn input(&mut self) {
        delegate!(mut self, input())
    }

    fn input_with_ctx(&mut self, ctx: &mut AppContext) {
        delegate!(mut self, input_with_ctx(ctx))
    }

    fn sync_input_from(
        &mut self,
        input: &rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        delegate!(mut self, sync_input_from(input))
    }

    fn sync_input_back_to(
        &mut self,
        input: &mut rubato_input::bms_player_input_processor::BMSPlayerInputProcessor,
    ) {
        delegate!(mut self, sync_input_back_to(input))
    }

    fn sync_audio(&mut self, audio: &mut rubato_audio::audio_system::AudioSystem) {
        delegate!(mut self, sync_audio(audio))
    }

    fn handle_skin_mouse_pressed(&mut self, button: i32, x: i32, y: i32) {
        delegate!(mut self, handle_skin_mouse_pressed(button, x, y))
    }

    fn handle_skin_mouse_dragged(&mut self, button: i32, x: i32, y: i32) {
        delegate!(mut self, handle_skin_mouse_dragged(button, x, y))
    }

    fn render_skin(&mut self, sprite: &mut SpriteBatch) {
        delegate!(mut self, render_skin(sprite))
    }

    fn pause(&mut self) {
        delegate!(mut self, pause())
    }

    fn resume(&mut self) {
        delegate!(mut self, resume())
    }

    fn resize(&mut self, width: i32, height: i32) {
        delegate!(mut self, resize(width, height))
    }

    fn dispose(&mut self) {
        delegate!(mut self, dispose())
    }

    fn execute_event_id(&mut self, id: i32) {
        delegate!(mut self, execute_event_id(id))
    }

    fn execute_event_id_arg(&mut self, id: i32, arg: i32) {
        delegate!(mut self, execute_event_id_arg(id, arg))
    }

    fn execute_event_id_args(&mut self, id: i32, arg1: i32, arg2: i32) {
        delegate!(mut self, execute_event_id_args(id, arg1, arg2))
    }

    fn score_data_property(&self) -> &rubato_core::score_data_property::ScoreDataProperty {
        delegate!(self, score_data_property() -> &rubato_core::score_data_property::ScoreDataProperty)
    }

    fn score_data_property_mut(
        &mut self,
    ) -> &mut rubato_core::score_data_property::ScoreDataProperty {
        delegate!(mut self, score_data_property_mut() -> &mut rubato_core::score_data_property::ScoreDataProperty)
    }

    fn judge_count(&self, judge: i32, fast: bool) -> i32 {
        delegate!(self, judge_count(judge, fast) -> i32)
    }

    fn groove_gauge_value(&self) -> Option<f32> {
        delegate!(self, groove_gauge_value() -> Option<f32>)
    }

    fn get_image(&self, imageid: i32) -> Option<rubato_render::texture::TextureRegion> {
        delegate!(self, get_image(imageid) -> Option<rubato_render::texture::TextureRegion>)
    }

    fn sound(&self, sound: rubato_types::sound_type::SoundType) -> Option<String> {
        delegate!(self, sound(sound) -> Option<String>)
    }

    fn play_sound(&mut self, sound: rubato_types::sound_type::SoundType) {
        delegate!(mut self, play_sound(sound))
    }

    fn play_sound_loop(&mut self, sound: rubato_types::sound_type::SoundType, loop_sound: bool) {
        delegate!(mut self, play_sound_loop(sound, loop_sound))
    }

    fn stop_sound(&mut self, sound: rubato_types::sound_type::SoundType) {
        delegate!(mut self, stop_sound(sound))
    }

    fn load_skin(&mut self, skin_type: i32) {
        delegate!(mut self, load_skin(skin_type))
    }

    fn get_offset_value(&self, id: i32) -> Option<()> {
        delegate!(self, get_offset_value(id) -> Option<()>)
    }

    fn take_pending_state_change(&mut self) -> Option<MainStateType> {
        delegate!(mut self, take_pending_state_change() -> Option<MainStateType>)
    }

    fn take_pending_global_pitch(&mut self) -> Option<f32> {
        delegate!(mut self, take_pending_global_pitch() -> Option<f32>)
    }

    fn drain_pending_sounds(&mut self) -> Vec<(rubato_types::sound_type::SoundType, bool)> {
        delegate!(mut self, drain_pending_sounds() -> Vec<(rubato_types::sound_type::SoundType, bool)>)
    }

    fn take_score_handoff(&mut self) -> Option<rubato_types::score_handoff::ScoreHandoff> {
        delegate!(mut self, take_score_handoff() -> Option<rubato_types::score_handoff::ScoreHandoff>)
    }

    fn take_state_create_effects(&mut self) -> Option<StateCreateEffects> {
        delegate!(mut self, take_state_create_effects() -> Option<StateCreateEffects>)
    }

    fn take_pending_reload_bms(&mut self) -> bool {
        delegate!(mut self, take_pending_reload_bms() -> bool)
    }

    fn take_pending_replay_seed_reset(&mut self) -> bool {
        delegate!(mut self, take_pending_replay_seed_reset() -> bool)
    }

    fn take_pending_quick_retry_score(&mut self) -> Option<rubato_types::score_data::ScoreData> {
        delegate!(mut self, take_pending_quick_retry_score() -> Option<rubato_types::score_data::ScoreData>)
    }

    fn take_pending_quick_retry_replay(&mut self) -> Option<rubato_types::replay_data::ReplayData> {
        delegate!(mut self, take_pending_quick_retry_replay() -> Option<rubato_types::replay_data::ReplayData>)
    }

    fn take_pending_audio_config(&mut self) -> Option<rubato_types::audio_config::AudioConfig> {
        delegate!(mut self, take_pending_audio_config() -> Option<rubato_types::audio_config::AudioConfig>)
    }

    fn take_pending_play_config_update(
        &mut self,
    ) -> Option<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)> {
        delegate!(mut self, take_pending_play_config_update() -> Option<(bms_model::mode::Mode, rubato_types::play_config::PlayConfig)>)
    }

    fn take_pending_player_config_update(
        &mut self,
    ) -> Option<rubato_types::player_config::PlayerConfig> {
        delegate!(mut self, take_pending_player_config_update() -> Option<rubato_types::player_config::PlayerConfig>)
    }

    fn drain_pending_audio_path_plays(&mut self) -> Vec<(String, f32, bool)> {
        delegate!(mut self, drain_pending_audio_path_plays() -> Vec<(String, f32, bool)>)
    }

    fn drain_pending_audio_path_stops(&mut self) -> Vec<String> {
        delegate!(mut self, drain_pending_audio_path_stops() -> Vec<String>)
    }

    fn notify_media_load_finished(&mut self) {
        delegate!(mut self, notify_media_load_finished())
    }

    fn update_loading_progress(&mut self, audio_progress: f32, bga_on: bool) {
        delegate!(mut self, update_loading_progress(audio_progress, bga_on))
    }

    fn receive_updated_play_config(
        &mut self,
        mode: bms_model::mode::Mode,
        play_config: rubato_types::play_config::PlayConfig,
    ) {
        delegate!(mut self, receive_updated_play_config(mode, play_config))
    }

    fn receive_reloaded_model(&mut self, model: bms_model::bms_model::BMSModel) {
        delegate!(mut self, receive_reloaded_model(model))
    }

    fn take_bga_cache(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        delegate!(mut self, take_bga_cache() -> Option<Box<dyn std::any::Any + Send>>)
    }

    fn take_player_resource_box(&mut self) -> Option<Box<dyn std::any::Any + Send>> {
        delegate!(mut self, take_player_resource_box() -> Option<Box<dyn std::any::Any + Send>>)
    }

    fn bms_model(&self) -> Option<&bms_model::bms_model::BMSModel> {
        delegate!(self, bms_model() -> Option<&bms_model::bms_model::BMSModel>)
    }
}
