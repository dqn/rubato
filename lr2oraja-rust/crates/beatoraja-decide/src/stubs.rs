// Stubs for external dependencies not yet available as proper imports.

use beatoraja_core::main_state::MainStateType;
use beatoraja_core::system_sound_manager::SoundType;
use beatoraja_types::config::Config;
use beatoraja_types::course_data::{CourseData, CourseDataConstraint};
use beatoraja_types::groove_gauge::GrooveGauge;
use beatoraja_types::player_config::PlayerConfig;
use beatoraja_types::player_resource_access::PlayerResourceAccess;
use beatoraja_types::replay_data::ReplayData;
use beatoraja_types::score_data::ScoreData;
use beatoraja_types::song_data::SongData;

/// Stub for MainController reference.
/// Retained: get_input_processor/get_audio_processor are crate-specific and not on MainControllerAccess trait.
/// MainControllerAccess trait impl removed (unused — MusicDecide calls methods on concrete type).
pub struct MainControllerRef;

impl MainControllerRef {
    pub fn change_state(&mut self, _state: MainStateType) {
        log::warn!("not yet implemented: MainController.changeState");
    }

    pub fn get_input_processor(&self) -> &InputProcessorStub {
        log::warn!("not yet implemented: MainController.getInputProcessor");
        static DEFAULT: InputProcessorStub = InputProcessorStub;
        &DEFAULT
    }

    pub fn get_audio_processor(&self) -> &AudioProcessorStub {
        log::warn!("not yet implemented: MainController.getAudioProcessor");
        static DEFAULT: AudioProcessorStub = AudioProcessorStub;
        &DEFAULT
    }
}

/// Stub for AudioProcessor reference
pub struct AudioProcessorStub;

impl AudioProcessorStub {
    pub fn set_global_pitch(&self, _pitch: f32) {
        log::warn!("not yet implemented: AudioProcessor.setGlobalPitch");
    }
}

/// Stub for BMSPlayerInputProcessor reference
pub struct InputProcessorStub;

impl InputProcessorStub {
    pub fn get_key_state(&self, _id: i32) -> bool {
        false
    }

    pub fn is_control_key_pressed(&self, _key: ControlKeysStub) -> bool {
        false
    }

    pub fn start_pressed(&self) -> bool {
        false
    }

    pub fn is_select_pressed(&self) -> bool {
        false
    }
}

/// Stub for ControlKeys enum
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ControlKeysStub {
    Enter,
    Escape,
}

/// Stub for PlayerResource reference
pub struct PlayerResourceRef;

impl PlayerResourceRef {
    pub fn set_org_gauge_option(&mut self, _gauge: i32) {
        // stub
    }

    pub fn get_player_config(&self) -> &PlayerConfigRef {
        log::warn!("not yet implemented: PlayerResource.getPlayerConfig");
        static DEFAULT: PlayerConfigRef = PlayerConfigRef { gauge: 0 };
        &DEFAULT
    }
}

impl PlayerResourceAccess for PlayerResourceRef {
    fn get_config(&self) -> &Config {
        log::warn!("not yet implemented: PlayerResourceRef::get_config");
        static DEFAULT: std::sync::OnceLock<Config> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(Config::default)
    }
    fn get_player_config(&self) -> &PlayerConfig {
        log::warn!("not yet implemented: PlayerResourceRef::get_player_config");
        static DEFAULT: std::sync::OnceLock<PlayerConfig> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(PlayerConfig::default)
    }
    fn get_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_rival_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_target_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn get_course_score_data(&self) -> Option<&ScoreData> {
        None
    }
    fn set_course_score_data(&mut self, _score: ScoreData) {}
    fn get_songdata(&self) -> Option<&SongData> {
        None
    }
    fn get_replay_data(&self) -> Option<&ReplayData> {
        None
    }
    fn get_course_replay(&self) -> &[ReplayData] {
        &[]
    }
    fn add_course_replay(&mut self, _rd: ReplayData) {}
    fn get_course_data(&self) -> Option<&CourseData> {
        None
    }
    fn get_course_index(&self) -> usize {
        0
    }
    fn next_course(&mut self) -> bool {
        false
    }
    fn get_constraint(&self) -> Vec<CourseDataConstraint> {
        vec![]
    }
    fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }
    fn get_groove_gauge(&self) -> Option<&GrooveGauge> {
        None
    }
    fn get_course_gauge(&self) -> &Vec<Vec<Vec<f32>>> {
        static EMPTY: Vec<Vec<Vec<f32>>> = Vec::new();
        &EMPTY
    }
    fn add_course_gauge(&mut self, _gauge: Vec<Vec<f32>>) {}
    fn get_maxcombo(&self) -> i32 {
        0
    }
    fn get_org_gauge_option(&self) -> i32 {
        0
    }
    fn set_org_gauge_option(&mut self, _val: i32) {}
    fn get_assist(&self) -> i32 {
        0
    }
    fn is_update_score(&self) -> bool {
        false
    }
    fn is_update_course_score(&self) -> bool {
        false
    }
    fn is_force_no_ir_send(&self) -> bool {
        false
    }
    fn is_freq_on(&self) -> bool {
        false
    }
    fn get_reverse_lookup_data(&self) -> Vec<String> {
        vec![]
    }
    fn get_reverse_lookup_levels(&self) -> Vec<String> {
        vec![]
    }
}

/// Stub for PlayerConfig reference
pub struct PlayerConfigRef {
    pub gauge: i32,
}

/// Stub for Skin (base class for MusicDecideSkin)
pub struct SkinStub {
    input: i32,
    scene: i32,
    fadeout: i32,
}

impl SkinStub {
    pub fn new() -> Self {
        Self {
            input: 0,
            scene: 0,
            fadeout: 0,
        }
    }

    pub fn get_input(&self) -> i32 {
        self.input
    }

    pub fn get_scene(&self) -> i32 {
        self.scene
    }

    pub fn get_fadeout(&self) -> i32 {
        self.fadeout
    }
}

impl Default for SkinStub {
    fn default() -> Self {
        Self::new()
    }
}

/// Stub for load_skin function
pub fn load_skin(_skin_type: beatoraja_skin::skin_type::SkinType) -> Option<SkinStub> {
    log::warn!("not yet implemented: SkinLoader.load");
    None
}

/// Stub for play sound (MainState.play delegates to MainController.getSoundManager())
pub fn play_sound(_sound: SoundType) {
    log::warn!("not yet implemented: MainController.getSoundManager().play()");
}
