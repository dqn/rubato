// External dependency stubs for beatoraja-result crate
// These will be replaced with actual implementations when corresponding phases are translated.

use std::sync::Arc;

// ============================================================
// Re-exports from real crates (Phase 11 stub replacements)
// ============================================================

pub use beatoraja_core::timer_manager::TimerManager;
pub use beatoraja_input::key_command::KeyCommand;
pub use beatoraja_input::keyboard_input_processor::ControlKeys;
pub use beatoraja_skin::skin::Skin;
pub use beatoraja_skin::skin_header::SkinHeader;
pub use beatoraja_skin::skin_object::SkinObjectRenderer;
pub use beatoraja_skin::stubs::Color;
pub use beatoraja_skin::stubs::Pixmap;
pub use beatoraja_skin::stubs::PixmapFormat;
pub use beatoraja_skin::stubs::Rectangle;
pub use beatoraja_skin::stubs::Texture;
pub use beatoraja_skin::stubs::TextureRegion;
use beatoraja_song::song_data::SongData;

// ============================================================
// PlayDataAccessor stub
// ============================================================

/// Stub for bms.player.beatoraja.PlayDataAccessor
pub struct PlayDataAccessor;

impl PlayDataAccessor {
    pub fn exists_replay_data_model(
        &self,
        _model: &bms_model::bms_model::BMSModel,
        _lnmode: i32,
        _index: i32,
    ) -> bool {
        false
    }

    pub fn exists_replay_data_course(
        &self,
        _models: &[bms_model::bms_model::BMSModel],
        _lnmode: i32,
        _index: i32,
        _constraint: &[beatoraja_core::course_data::CourseDataConstraint],
    ) -> bool {
        false
    }

    pub fn read_score_data(
        &self,
        _model: &bms_model::bms_model::BMSModel,
        _lnmode: i32,
    ) -> Option<beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn read_score_data_course(
        &self,
        _models: &[bms_model::bms_model::BMSModel],
        _lnmode: i32,
        _random: i32,
        _constraint: &[beatoraja_core::course_data::CourseDataConstraint],
    ) -> Option<beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn write_score_data(
        &self,
        _score: &beatoraja_core::score_data::ScoreData,
        _model: &bms_model::bms_model::BMSModel,
        _lnmode: i32,
        _update: bool,
    ) {
    }

    pub fn write_score_data_course(
        &self,
        _score: &beatoraja_core::score_data::ScoreData,
        _models: &[bms_model::bms_model::BMSModel],
        _lnmode: i32,
        _random: i32,
        _constraint: &[beatoraja_core::course_data::CourseDataConstraint],
        _update: bool,
    ) {
    }

    pub fn write_replay_data(
        &self,
        _replay: &beatoraja_core::replay_data::ReplayData,
        _model: &bms_model::bms_model::BMSModel,
        _lnmode: i32,
        _index: i32,
    ) {
    }

    pub fn write_replay_data_course(
        &self,
        _replays: &[beatoraja_core::replay_data::ReplayData],
        _models: &[bms_model::bms_model::BMSModel],
        _lnmode: i32,
        _index: i32,
        _constraint: &[beatoraja_core::course_data::CourseDataConstraint],
    ) {
    }
}

// ============================================================
// MainController stub
// ============================================================

/// Stub for bms.player.beatoraja.MainController
pub struct MainController;

impl MainController {
    pub fn get_input_processor(&mut self) -> &mut BMSPlayerInputProcessor {
        log::warn!("not yet implemented: MainController.getInputProcessor");
        // Leak a boxed value to get a &'static mut reference - stub only
        Box::leak(Box::new(BMSPlayerInputProcessor))
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        log::warn!("not yet implemented: MainController.getConfig");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::config::Config> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::config::Config::default)
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        log::warn!("not yet implemented: MainController.getPlayerConfig");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::player_config::PlayerConfig::default)
    }

    pub fn get_ir_status(&self) -> &[IRStatus] {
        log::warn!("not yet implemented: MainController.getIRStatus");
        &[]
    }

    pub fn change_state(&mut self, _state_type: beatoraja_core::main_state::MainStateType) {
        log::warn!("not yet implemented: MainController.changeState");
    }

    pub fn save_last_recording(&self, _tag: &str) {
        log::warn!("not yet implemented: MainController.saveLastRecording");
    }

    pub fn ir_send_status(&self) -> &Vec<IRSendStatusMain> {
        log::warn!("not yet implemented: MainController.irSendStatus");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(Vec::new()))
    }

    pub fn ir_send_status_mut(&mut self) -> &mut Vec<IRSendStatusMain> {
        log::warn!("not yet implemented: MainController.irSendStatus_mut");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(Vec::new()))
    }

    pub fn get_play_data_accessor(&self) -> &PlayDataAccessor {
        log::warn!("not yet implemented: MainController.getPlayDataAccessor");
        static DEFAULT: PlayDataAccessor = PlayDataAccessor;
        &DEFAULT
    }
}

// ============================================================
// IR (Internet Ranking) stubs
// ============================================================

/// Stub for bms.player.beatoraja.MainController.IRStatus
pub struct IRStatus {
    pub connection: Arc<dyn IRConnection>,
    pub config: IRConfig,
}

// IRConnection: replaced by pub use from beatoraja_ir (trait)
pub use beatoraja_ir::ir_connection::IRConnection;

// IRConfig: replaced by pub use from beatoraja_core
pub use beatoraja_core::ir_config::IRConfig;

// IRScoreData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ir_score_data::IRScoreData;

// IRCourseData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ir_course_data::IRCourseData;

// RankingData: replaced by pub use from beatoraja_ir
pub use beatoraja_ir::ranking_data::RankingData;

// ============================================================
// MainController.IRSendStatus (for MusicResult)
// ============================================================

/// Stub for bms.player.beatoraja.MainController.IRSendStatus
pub struct IRSendStatusMain {
    pub connection: Arc<dyn IRConnection>,
    pub songdata: SongData,
    pub score: beatoraja_core::score_data::ScoreData,
    pub retry: i32,
}

impl IRSendStatusMain {
    pub fn new(
        connection: Arc<dyn IRConnection>,
        songdata: &SongData,
        score: &beatoraja_core::score_data::ScoreData,
    ) -> Self {
        Self {
            connection,
            songdata: songdata.clone(),
            score: score.clone(),
            retry: 0,
        }
    }

    pub fn send(&mut self) -> bool {
        log::warn!("not yet implemented: IRSendStatus.send");
        false
    }
}

// ============================================================
// Input stubs
// ============================================================

/// Stub for bms.player.beatoraja.input.BMSPlayerInputProcessor
pub struct BMSPlayerInputProcessor;

impl BMSPlayerInputProcessor {
    pub fn get_scroll(&self) -> i32 {
        0
    }

    pub fn reset_scroll(&mut self) {
        // stub
    }

    pub fn get_key_state(&self, _index: i32) -> bool {
        false
    }

    pub fn reset_key_changed_time(&mut self, _index: i32) -> bool {
        false
    }

    pub fn reset_all_key_changed_time(&mut self) {
        // stub
    }

    pub fn is_control_key_pressed(&self, _key: ControlKeys) -> bool {
        false
    }

    pub fn is_activated(&self, _command: KeyCommand) -> bool {
        false
    }
}

// ============================================================
// GrooveGauge stub
// ============================================================

/// Stub for bms.player.beatoraja.play.GrooveGauge
pub struct GrooveGaugeStub {
    pub gauge_type: i32,
}

impl GrooveGaugeStub {
    pub fn get_type(&self) -> i32 {
        self.gauge_type
    }

    pub fn get_gauge_type_length(&self) -> usize {
        9
    }

    pub fn get_gauge(&self, _gauge_type: i32) -> &beatoraja_play::groove_gauge::Gauge {
        log::warn!("not yet implemented: GrooveGauge.getGauge");
        static DEFAULT: std::sync::OnceLock<beatoraja_play::groove_gauge::Gauge> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(|| {
            let model = bms_model::bms_model::BMSModel::default();
            let element = beatoraja_types::gauge_property::GaugeElementProperty {
                modifier: None,
                value: vec![0.0; 6],
                min: 0.0,
                max: 100.0,
                init: 0.0,
                border: 80.0,
                death: 0.0,
                guts: Vec::new(),
            };
            beatoraja_play::groove_gauge::Gauge::new(
                &model,
                element,
                beatoraja_core::clear_type::ClearType::Failed,
            )
        })
    }

    pub fn get_clear_type(&self) -> beatoraja_core::clear_type::ClearType {
        log::warn!("not yet implemented: GrooveGauge.getClearType");
        beatoraja_core::clear_type::ClearType::Failed
    }
}

// ============================================================
// GdxArray (LibGDX) stub
// ============================================================

/// Stub for com.badlogic.gdx.utils.Array<T>
pub struct GdxArray<T> {
    pub items: Vec<T>,
    pub size: usize,
}

impl<T> GdxArray<T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            size: 0,
        }
    }

    pub fn add(&mut self, value: T) {
        self.items.push(value);
        self.size = self.items.len();
    }

    pub fn get(&self, index: usize) -> &T {
        &self.items[index]
    }
}

impl<T> Default for GdxArray<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, T> IntoIterator for &'a GdxArray<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}

impl<'a, T> IntoIterator for &'a mut GdxArray<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

// ============================================================
// PlayerResource stub
// ============================================================

/// Stub for bms.player.beatoraja.PlayerResource
pub struct PlayerResource {
    pub play_mode: BMSPlayerMode,
}

impl PlayerResource {
    pub fn get_bms_model(&self) -> &bms_model::bms_model::BMSModel {
        log::warn!("not yet implemented: PlayerResource.getBMSModel");
        static DEFAULT: std::sync::OnceLock<bms_model::bms_model::BMSModel> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(bms_model::bms_model::BMSModel::default)
    }

    pub fn get_player_config(&self) -> &beatoraja_core::player_config::PlayerConfig {
        log::warn!("not yet implemented: PlayerResource.getPlayerConfig");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::player_config::PlayerConfig> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::player_config::PlayerConfig::default)
    }

    pub fn get_config(&self) -> &beatoraja_core::config::Config {
        log::warn!("not yet implemented: PlayerResource.getConfig");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::config::Config> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::config::Config::default)
    }

    pub fn get_course_bms_models(&self) -> Option<&[bms_model::bms_model::BMSModel]> {
        None
    }

    pub fn get_play_mode(&self) -> &BMSPlayerMode {
        &self.play_mode
    }

    pub fn get_gauge(&self) -> &[FloatArray] {
        log::warn!("not yet implemented: PlayerResource.getGauge");
        &[]
    }

    pub fn get_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn get_score_data_mut(&mut self) -> Option<&mut beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn get_course_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn get_course_score_data_mut(
        &mut self,
    ) -> Option<&mut beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn set_course_score_data(&mut self, _score: beatoraja_core::score_data::ScoreData) {
        // stub
    }

    pub fn get_ranking_data(&self) -> Option<&RankingData> {
        None
    }

    pub fn set_ranking_data(&mut self, _data: Option<RankingData>) {
        // stub
    }

    pub fn get_replay_data(&self) -> &beatoraja_core::replay_data::ReplayData {
        log::warn!("not yet implemented: PlayerResource.getReplayData");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::replay_data::ReplayData> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::replay_data::ReplayData::default)
    }

    pub fn get_replay_data_mut(&mut self) -> &mut beatoraja_core::replay_data::ReplayData {
        log::warn!("not yet implemented: PlayerResource.getReplayData_mut");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(beatoraja_core::replay_data::ReplayData::default()))
    }

    pub fn get_course_replay(&self) -> &[beatoraja_core::replay_data::ReplayData] {
        log::warn!("not yet implemented: PlayerResource.getCourseReplay");
        &[]
    }

    pub fn get_course_replay_mut(&mut self) -> &mut Vec<beatoraja_core::replay_data::ReplayData> {
        log::warn!("not yet implemented: PlayerResource.getCourseReplay_mut");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(Vec::new()))
    }

    pub fn add_course_replay(&mut self, _replay: &beatoraja_core::replay_data::ReplayData) {
        // stub
    }

    pub fn add_course_gauge(&mut self, _gauge: &[FloatArray]) {
        // stub
    }

    pub fn get_maxcombo(&self) -> i32 {
        0
    }

    pub fn get_target_score_data(&self) -> Option<&beatoraja_core::score_data::ScoreData> {
        None
    }

    pub fn is_update_score(&self) -> bool {
        false
    }

    pub fn is_update_course_score(&self) -> bool {
        false
    }

    pub fn is_force_no_ir_send(&self) -> bool {
        false
    }

    pub fn get_course_data(&self) -> &beatoraja_core::course_data::CourseData {
        log::warn!("not yet implemented: PlayerResource.getCourseData");
        static DEFAULT: std::sync::OnceLock<beatoraja_core::course_data::CourseData> =
            std::sync::OnceLock::new();
        DEFAULT.get_or_init(beatoraja_core::course_data::CourseData::default)
    }

    pub fn get_songdata(&self) -> &SongData {
        log::warn!("not yet implemented: PlayerResource.getSongdata");
        static DEFAULT: std::sync::OnceLock<SongData> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(SongData::default)
    }

    pub fn get_org_gauge_option(&self) -> i32 {
        0
    }

    pub fn get_constraint(&self) -> Vec<beatoraja_core::course_data::CourseDataConstraint> {
        vec![]
    }

    pub fn get_course_index(&self) -> usize {
        0
    }

    pub fn get_assist(&self) -> i32 {
        0
    }

    pub fn next_course(&mut self) -> bool {
        false
    }

    pub fn reload_bms_file(&mut self) {
        // stub
    }

    pub fn is_freq_on(&self) -> bool {
        false
    }

    pub fn get_groove_gauge(&self) -> &GrooveGaugeStub {
        log::warn!("not yet implemented: PlayerResource.getGrooveGauge");
        static DEFAULT: GrooveGaugeStub = GrooveGaugeStub { gauge_type: 0 };
        &DEFAULT
    }

    pub fn get_course_gauge(&self) -> &GdxArray<Vec<FloatArray>> {
        log::warn!("not yet implemented: PlayerResource.getCourseGauge");
        static DEFAULT: std::sync::OnceLock<GdxArray<Vec<FloatArray>>> = std::sync::OnceLock::new();
        DEFAULT.get_or_init(GdxArray::new)
    }

    pub fn get_course_gauge_mut(&mut self) -> &mut GdxArray<Vec<FloatArray>> {
        log::warn!("not yet implemented: PlayerResource.getCourseGauge_mut");
        // Leak a boxed value - stub only, will be replaced with real implementation
        Box::leak(Box::new(GdxArray::new()))
    }
}

// ============================================================
// BMSPlayerMode stub
// ============================================================

/// Stub for bms.player.beatoraja.BMSPlayerMode
#[derive(Clone, Debug)]
pub struct BMSPlayerMode {
    pub mode: BMSPlayerModeType,
}

/// Stub for bms.player.beatoraja.BMSPlayerMode.Mode
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BMSPlayerModeType {
    Play,
    Practice,
    Replay,
    ReplayDifferent,
}

// ============================================================
// FloatArray (LibGDX) stub
// ============================================================

/// Stub for com.badlogic.gdx.utils.FloatArray
#[derive(Clone, Debug, Default)]
pub struct FloatArray {
    pub items: Vec<f32>,
    pub size: usize,
}

impl FloatArray {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            size: 0,
        }
    }

    pub fn add(&mut self, value: f32) {
        self.items.push(value);
        self.size = self.items.len();
    }

    pub fn get(&self, index: usize) -> f32 {
        self.items[index]
    }

    pub fn add_all(&mut self, other: &FloatArray) {
        self.items.extend_from_slice(&other.items);
        self.size = self.items.len();
    }
}

/// Stub for com.badlogic.gdx.utils.IntArray
#[derive(Clone, Debug, Default)]
pub struct IntArray {
    pub items: Vec<i32>,
    pub size: usize,
}

impl IntArray {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            size: 0,
        }
    }

    pub fn add(&mut self, value: i32) {
        self.items.push(value);
        self.size = self.items.len();
    }

    pub fn get(&self, index: usize) -> i32 {
        self.items[index]
    }

    pub fn contains(&self, value: i32) -> bool {
        self.items.contains(&value)
    }
}

// Skin: replaced by pub use beatoraja_skin::skin::Skin
// SkinHeader: replaced by pub use beatoraja_skin::skin_header::SkinHeader
// Color: replaced by pub use beatoraja_skin::stubs::Color
// Rectangle: replaced by pub use beatoraja_skin::stubs::Rectangle
// SkinObjectRenderer: replaced by pub use beatoraja_skin::skin_object::SkinObjectRenderer

// TextureRegion, Texture, Pixmap: replaced by pub use beatoraja_skin::stubs::*

/// Stub for SkinObject base (partial — only what SkinGaugeGraphObject needs)
pub struct SkinObjectData {
    pub region: Rectangle,
}

// TimerManager: replaced by pub use beatoraja_core::timer_manager::TimerManager

// ============================================================
// EventFactory stub
// ============================================================

/// Stub for bms.player.beatoraja.skin.property.EventFactory.EventType
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventType {
    #[allow(non_camel_case_types)]
    open_ir,
}

// ============================================================
// FreqTrainerMenu stub
// ============================================================

pub fn is_freq_trainer_enabled() -> bool {
    false
}

pub fn is_freq_negative() -> bool {
    false
}
