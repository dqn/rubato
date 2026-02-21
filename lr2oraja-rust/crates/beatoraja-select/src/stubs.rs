// External dependency stubs for beatoraja-select
// Types that can be replaced with real implementations are re-exported from beatoraja-core.
// Remaining stubs are for types that cannot be replaced due to API incompatibilities.

use std::collections::HashMap;

// ============================================================
// LibGDX types — re-exported from beatoraja-skin stubs
// ============================================================

pub use beatoraja_skin::stubs::Color;
pub use beatoraja_skin::stubs::Pixmap;
pub use beatoraja_skin::stubs::Rectangle;
pub use beatoraja_skin::stubs::TextureRegion;

// ============================================================
// beatoraja core types — re-exported from real implementations
// ============================================================

pub use beatoraja_core::audio_config::AudioConfig;
pub use beatoraja_core::config::{Config, SongPreview};
pub use beatoraja_core::play_config::PlayConfig;
pub use beatoraja_core::play_mode_config::{
    ControllerConfig, KeyboardConfig, MidiConfig, PlayModeConfig,
};
pub use beatoraja_core::player_config::PlayerConfig;
pub use beatoraja_core::resolution::Resolution;
pub use beatoraja_core::score_data::ScoreData;

// ============================================================
// beatoraja.song types — real SongData from beatoraja-types
// ============================================================

pub use beatoraja_types::song_data::SongData;
pub use beatoraja_types::song_data::{
    FAVORITE_CHART, FAVORITE_SONG, FEATURE_CHARGENOTE, FEATURE_HELLCHARGENOTE, FEATURE_LONGNOTE,
    FEATURE_MINENOTE, FEATURE_RANDOM, FEATURE_UNDEFINEDLN, INVISIBLE_CHART, INVISIBLE_SONG,
};

// ============================================================
// beatoraja.song.FolderData — replaced with real type from beatoraja-types
// ============================================================

pub use beatoraja_types::folder_data::FolderData;

// ============================================================
// beatoraja.song.SongDatabaseAccessor — replaced with real trait from beatoraja-types
// ============================================================

pub use beatoraja_types::song_database_accessor::SongDatabaseAccessor;

/// Null implementation of SongDatabaseAccessor for stub contexts
pub struct NullSongDatabaseAccessor;

impl SongDatabaseAccessor for NullSongDatabaseAccessor {
    fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
        todo!("SongDatabaseAccessor.getSongDatas")
    }
    fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
        todo!("SongDatabaseAccessor.getSongDatas(hashes)")
    }
    fn get_song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        todo!("SongDatabaseAccessor.getSongDatas(sql)")
    }
    fn set_song_datas(&self, _songs: &[SongData]) {
        todo!("SongDatabaseAccessor.setSongDatas")
    }
    fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        todo!("SongDatabaseAccessor.getSongDatasByText")
    }
    fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        todo!("SongDatabaseAccessor.getFolderDatas")
    }
}

// ============================================================
// beatoraja.song.SongUtils
// ============================================================

pub struct SongUtils;

impl SongUtils {
    pub fn crc32(_path: &str, _ext: &[&str], _root: &str) -> String {
        todo!("SongUtils.crc32")
    }
}

// ============================================================
// beatoraja.song.SongInformationAccessor
// ============================================================

/// Stub for beatoraja.song.SongInformationAccessor
pub struct SongInformationAccessor;

impl SongInformationAccessor {
    pub fn get_information(&self, _songs: &[SongData]) {
        todo!("SongInformationAccessor.getInformation")
    }
}

// ============================================================
// beatoraja core types (stubbed — cannot be replaced)
// ============================================================

// MainControllerAccess / PlayerResourceAccess — re-exported from beatoraja-types (Phase 15d)
pub use beatoraja_types::main_controller_access::MainControllerAccess;
pub use beatoraja_types::main_state_type::MainStateType as TypesMainStateType;
pub use beatoraja_types::player_resource_access::PlayerResourceAccess;

/// Stub for beatoraja.MainController
#[derive(Debug, Default)]
pub struct MainController;

impl MainControllerAccess for MainController {
    fn get_config(&self) -> &Config {
        todo!()
    }
    fn get_player_config(&self) -> &PlayerConfig {
        todo!()
    }
    fn change_state(&mut self, _state: TypesMainStateType) {
        todo!()
    }
    fn save_config(&self) {
        todo!()
    }
    fn exit(&self) {
        todo!()
    }
    fn save_last_recording(&self, _reason: &str) {
        todo!()
    }
    fn update_song(&mut self, _path: Option<&str>) {
        todo!()
    }
    fn get_player_resource(&self) -> Option<&dyn PlayerResourceAccess> {
        None
    }
    fn get_player_resource_mut(&mut self) -> Option<&mut dyn PlayerResourceAccess> {
        None
    }
}

impl MainController {
    pub fn get_song_database(&self) -> &dyn SongDatabaseAccessor {
        todo!()
    }
    pub fn get_info_database(&self) -> Option<&SongInformationAccessor> {
        todo!()
    }
    pub fn get_play_data_accessor(&self) -> &PlayDataAccessor {
        todo!()
    }
    pub fn get_rival_data_accessor(&self) -> &RivalDataAccessor {
        todo!()
    }
    pub fn get_ir_status(&self) -> &[IRStatus] {
        todo!()
    }
    pub fn get_ranking_data_cache(&self) -> &RankingDataCache {
        todo!()
    }
    pub fn get_input_processor(&self) -> &BMSPlayerInputProcessor {
        todo!()
    }
    pub fn get_sound_manager(&self) -> &SystemSoundManager {
        todo!()
    }
    pub fn get_player_resource_local(&self) -> &PlayerResource {
        todo!()
    }
    pub fn get_current_state(&self) -> &dyn MainState {
        todo!()
    }
    pub fn get_music_download_processor(&self) -> Option<&MusicDownloadProcessor> {
        todo!()
    }
    pub fn get_http_download_processor(&self) -> Option<&HttpDownloadProcessor> {
        todo!()
    }
}

/// Stub for beatoraja.MainState
pub trait MainState {
    fn get_main(&self) -> &MainController;
}

/// MainStateType — re-exported from beatoraja-types (Phase 15d)
pub use beatoraja_types::main_state_type::MainStateType;

/// Stub for beatoraja.ScoreDatabaseAccessor.ScoreDataCollector
pub trait ScoreDataCollector: Fn(&SongData, Option<&ScoreData>) {}
impl<F: Fn(&SongData, Option<&ScoreData>)> ScoreDataCollector for F {}

/// Stub for beatoraja.BMSPlayerMode
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BMSPlayerMode {
    pub mode: BMSPlayerModeType,
    pub id: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BMSPlayerModeType {
    Play,
    AutoPlay,
    Practice,
    Replay,
}

impl BMSPlayerMode {
    pub const PLAY: BMSPlayerMode = BMSPlayerMode {
        mode: BMSPlayerModeType::Play,
        id: 0,
    };
    pub const AUTOPLAY: BMSPlayerMode = BMSPlayerMode {
        mode: BMSPlayerModeType::AutoPlay,
        id: 1,
    };
    pub const PRACTICE: BMSPlayerMode = BMSPlayerMode {
        mode: BMSPlayerModeType::Practice,
        id: 2,
    };

    pub fn get_replay_mode(index: i32) -> BMSPlayerMode {
        BMSPlayerMode {
            mode: BMSPlayerModeType::Replay,
            id: index + 3,
        }
    }
}

// beatoraja.CourseData / TrophyData / CourseDataConstraint — replaced with real types from beatoraja-types (Phase 15g)
pub use beatoraja_types::course_data::{CourseData, CourseDataConstraint, TrophyData};

/// Stub for beatoraja.RandomCourseData
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RandomCourseData {
    pub name: String,
    pub stage: Vec<RandomStageData>,
    pub constraint: Vec<CourseDataConstraint>,
}

impl RandomCourseData {
    pub const EMPTY: &'static [RandomCourseData] = &[];

    pub fn get_name(&self) -> &str {
        &self.name
    }
    pub fn get_stage(&self) -> &[RandomStageData] {
        &self.stage
    }
    pub fn get_song_datas(&self) -> Vec<SongData> {
        todo!()
    }
    pub fn lottery_song_datas(&self, _main: &MainController) {
        todo!()
    }
    pub fn create_course_data(&self) -> CourseData {
        todo!()
    }
}

/// Stub for beatoraja.RandomStageData
#[derive(Clone, Debug, Default, serde::Deserialize)]
pub struct RandomStageData;

// beatoraja.TableData / TableFolder — replaced with real types from beatoraja-core (Phase 15g)
pub use beatoraja_core::table_data::{TableData, TableFolder};

// beatoraja.TableDataAccessor / TableAccessor / DifficultyTableAccessor — replaced with real types from beatoraja-core (Phase 15g)
pub use beatoraja_core::table_data_accessor::{
    DifficultyTableAccessor, TableAccessor, TableDataAccessor,
};

// beatoraja.CourseDataAccessor — replaced with real type from beatoraja-core (Phase 15g)
pub use beatoraja_core::course_data_accessor::CourseDataAccessor;

/// Stub for beatoraja.PlayDataAccessor
pub struct PlayDataAccessor;

impl PlayDataAccessor {
    pub fn read_score_data_single(
        &self,
        _hash: &str,
        _ln: bool,
        _lnmode: i32,
    ) -> Option<ScoreData> {
        todo!()
    }
    pub fn read_score_data_multi(
        &self,
        _hashes: &[String],
        _ln: bool,
        _lnmode: i32,
        _mode: i32,
        _constraints: &[CourseDataConstraint],
    ) -> Option<ScoreData> {
        todo!()
    }
    pub fn read_score_datas(
        &self,
        _collector: &dyn Fn(&SongData, Option<&ScoreData>),
        _songs: &[SongData],
        _lnmode: i32,
    ) {
        todo!()
    }
    pub fn exists_replay_data_single(
        &self,
        _hash: &str,
        _ln: bool,
        _lnmode: i32,
        _index: i32,
    ) -> bool {
        todo!()
    }
    pub fn exists_replay_data_multi(
        &self,
        _hashes: &[String],
        _ln: bool,
        _lnmode: i32,
        _index: i32,
        _constraints: &[CourseDataConstraint],
    ) -> bool {
        todo!()
    }
    pub fn read_replay_data(&self, _model: &(), _lnmode: i32, _id: i32) -> Option<ReplayData> {
        todo!()
    }
    pub fn read_player_data(&self) -> PlayerData {
        todo!()
    }
}

/// Stub for beatoraja.ReplayData
/// Cannot be replaced: real type references KeyInputLog/PatternModifyLog stubs
#[derive(Clone, Debug, Default)]
pub struct ReplayData {
    pub randomoption: i32,
    pub randomoptionseed: i64,
    pub randomoption2: i32,
    pub randomoption2seed: i64,
    pub doubleoption: i32,
    pub rand: i32,
}

/// Stub for beatoraja.PlayerData
#[derive(Clone, Debug, Default)]
pub struct PlayerData;

/// Stub for beatoraja.RivalDataAccessor
pub struct RivalDataAccessor;

impl RivalDataAccessor {
    pub fn get_rival_count(&self) -> i32 {
        0
    }
    pub fn get_rival_information(&self, _index: i32) -> Option<&PlayerInformation> {
        None
    }
    pub fn get_rival_score_data_cache(&self, _index: i32) -> Option<&ScoreDataCacheStub> {
        None
    }
}

/// Stub for beatoraja.PlayerInformation
#[derive(Clone, Debug, Default)]
pub struct PlayerInformation {
    pub name: String,
}

impl PlayerInformation {
    pub fn get_name(&self) -> &str {
        &self.name
    }
}

/// Stub for score data cache (abstract in Java)
pub struct ScoreDataCacheStub;

/// Stub for beatoraja.PlayerResource
pub struct PlayerResource;

impl PlayerResourceAccess for PlayerResource {
    fn get_config(&self) -> &Config {
        todo!()
    }
    fn get_player_config(&self) -> &PlayerConfig {
        todo!()
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
    fn get_replay_data(&self) -> Option<&beatoraja_types::replay_data::ReplayData> {
        None
    }
    fn get_course_replay(&self) -> &[beatoraja_types::replay_data::ReplayData] {
        &[]
    }
    fn add_course_replay(&mut self, _rd: beatoraja_types::replay_data::ReplayData) {}
    fn get_course_data(&self) -> Option<&beatoraja_types::course_data::CourseData> {
        None
    }
    fn get_course_index(&self) -> usize {
        0
    }
    fn next_course(&mut self) -> bool {
        false
    }
    fn get_constraint(&self) -> Vec<beatoraja_types::course_data::CourseDataConstraint> {
        vec![]
    }
    fn get_gauge(&self) -> Option<&Vec<Vec<f32>>> {
        None
    }
    fn get_groove_gauge(&self) -> Option<&beatoraja_types::groove_gauge::GrooveGauge> {
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

/// Stub for beatoraja.RankingData
pub struct RankingData {
    pub state: i32,
    pub last_update_time: i64,
    pub total_player: i32,
}

impl Default for RankingData {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingData {
    pub const ACCESS: i32 = 0;
    pub const FINISH: i32 = 1;
    pub const FAIL: i32 = 2;

    pub fn new() -> Self {
        Self {
            state: 0,
            last_update_time: 0,
            total_player: 0,
        }
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }
    pub fn get_last_update_time(&self) -> i64 {
        self.last_update_time
    }
    pub fn get_total_player(&self) -> i32 {
        self.total_player
    }
    pub fn load_song(&self, _selector: &dyn MainState, _song: &SongData) {
        todo!()
    }
    pub fn load_course(&self, _selector: &dyn MainState, _course: &CourseData) {
        todo!()
    }
}

/// Stub for beatoraja.RankingDataCache
pub struct RankingDataCache;

impl RankingDataCache {
    pub fn get_song(&self, _song: &SongData, _lnmode: i32) -> Option<&RankingData> {
        None
    }
    pub fn get_course(&self, _course: &CourseData, _lnmode: i32) -> Option<&RankingData> {
        None
    }
    pub fn put_song(&self, _song: &SongData, _lnmode: i32, _data: RankingData) {
        todo!()
    }
    pub fn put_course(&self, _course: &CourseData, _lnmode: i32, _data: RankingData) {
        todo!()
    }
}

/// Stub for beatoraja.PixmapResourcePool
pub struct PixmapResourcePool;

impl PixmapResourcePool {
    pub fn new(_gen: i32) -> Self {
        Self
    }
    pub fn get(&self, _path: &str) -> Option<Pixmap> {
        todo!()
    }
    pub fn dispose(&self) {}
    pub fn dispose_old(&self) {}
}

// ============================================================
// beatoraja.input types
// ============================================================

/// Stub for beatoraja.input.BMSPlayerInputProcessor
pub struct BMSPlayerInputProcessor;

impl BMSPlayerInputProcessor {
    pub fn get_key_state(&self, _key: i32) -> bool {
        false
    }
    pub fn is_analog_input(&self, _key: usize) -> bool {
        false
    }
    pub fn get_analog_diff_and_reset(&self, _key: usize, _threshold: i32) -> i32 {
        0
    }
    pub fn reset_key_changed_time(&self, _key: i32) -> bool {
        false
    }
    pub fn start_pressed(&self) -> bool {
        false
    }
    pub fn is_select_pressed(&self) -> bool {
        false
    }
    pub fn get_scroll(&self) -> i32 {
        0
    }
    pub fn reset_scroll(&self) {}
    pub fn get_control_key_state(&self, _key: ControlKeys) -> bool {
        false
    }
    pub fn is_control_key_pressed(&self, _key: ControlKeys) -> bool {
        false
    }
    pub fn is_activated(&self, _cmd: KeyCommand) -> bool {
        false
    }
    pub fn set_keyboard_config(&self, _config: &KeyboardConfig) {}
    pub fn set_controller_config(&self, _config: &ControllerConfig) {}
    pub fn set_midi_config(&self, _config: &MidiConfig) {}
    pub fn get_keyboard_input_processor(&self) -> &KeyBoardInputProcessor {
        todo!()
    }
}

/// Stub for ControlKeys
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlKeys {
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Escape,
}

/// Stub for KeyCommand
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyCommand {
    OpenSkinConfiguration,
    AutoplayFolder,
    OpenIr,
    AddFavoriteSong,
    AddFavoriteChart,
    UpdateFolder,
    OpenExplorer,
    CopySongMd5Hash,
    CopySongSha256Hash,
    CopyHighlightedMenuText,
}

/// Stub for KeyBoardInputProcessor
pub struct KeyBoardInputProcessor;

impl KeyBoardInputProcessor {
    pub fn set_text_input_mode(&self, _mode: bool) {}
}

// ============================================================
// beatoraja.ir types
// ============================================================

// ============================================================
// beatoraja.ir types — replaced with real types from beatoraja-ir
// ============================================================

pub use beatoraja_ir::ir_chart_data::IRChartData;
pub use beatoraja_ir::ir_connection::IRConnection;
pub use beatoraja_ir::ir_player_data::IRPlayerData;
pub use beatoraja_ir::ir_response::IRResponse;
pub use beatoraja_ir::ir_score_data::IRScoreData;
pub use beatoraja_ir::ir_table_data::IRTableData;

/// MainController.IRStatus — uses dyn IRConnection trait
pub struct IRStatus {
    pub connection: Box<dyn IRConnection>,
    pub player: IRPlayerData,
}

// LeaderboardEntry — replaced with real type from beatoraja-ir
pub use beatoraja_ir::leaderboard_entry::LeaderboardEntry;

// LR2IRConnection — replaced with real type from beatoraja-ir
pub use beatoraja_ir::lr2_ir_connection::LR2IRConnection;

// LR2GhostData — replaced with real type from beatoraja-ir
pub use beatoraja_ir::lr2_ghost_data::LR2GhostData;

// ============================================================
// beatoraja.play types
// ============================================================

/// Stub for GhostBattlePlay
pub struct GhostBattlePlay;

impl GhostBattlePlay {
    pub fn setup(_random: i32, _lane_order: &[i32]) {
        todo!()
    }
}

// ============================================================
// beatoraja.skin types
// ============================================================

// SkinType moved to beatoraja-types (Phase 15b)
pub use beatoraja_types::skin_type::SkinType;

/// Stub for beatoraja.skin.SkinObject
#[derive(Clone, Debug, Default)]
pub struct SkinObject {
    pub draw: bool,
    pub region: SkinRegion,
}

#[derive(Clone, Debug, Default)]
pub struct SkinRegion {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// Stub for beatoraja.skin.SkinImage
#[derive(Clone, Debug, Default)]
pub struct SkinImage {
    pub draw: bool,
    pub region: SkinRegion,
}

impl SkinImage {
    pub fn draw(
        &self,
        _sprite: &SkinObjectRenderer,
        _time: i64,
        _state: &dyn MainState,
        _value: i32,
        _dx: f32,
        _dy: f32,
    ) {
        todo!()
    }
    pub fn draw_offset(&self, _sprite: &SkinObjectRenderer, _dx: f32, _dy: f32) {
        todo!()
    }
    pub fn prepare(&self, _time: i64, _state: &dyn MainState) {}
    pub fn validate(&self) -> bool {
        true
    }
    pub fn get_destination(&self, _time: i64, _state: &dyn MainState) -> Option<Rectangle> {
        None
    }
}

/// Stub for beatoraja.skin.SkinText
#[derive(Clone, Debug, Default)]
pub struct SkinText;

impl SkinText {
    pub fn set_text(&self, _text: &str) {}
    pub fn draw(&self, _sprite: &SkinObjectRenderer, _x: f32, _y: f32) {
        todo!()
    }
    pub fn prepare(&self, _time: i64, _state: &dyn MainState) {}
    pub fn prepare_font(&self, _chars: &str) {}
    pub fn validate(&self) -> bool {
        true
    }
}

/// Stub for beatoraja.skin.SkinNumber
#[derive(Clone, Debug, Default)]
pub struct SkinNumber;

impl SkinNumber {
    pub fn draw(
        &self,
        _sprite: &SkinObjectRenderer,
        _time: i64,
        _value: i32,
        _state: &dyn MainState,
        _x: f32,
        _y: f32,
    ) {
        todo!()
    }
    pub fn prepare(&self, _time: i64, _state: &dyn MainState) {}
    pub fn validate(&self) -> bool {
        true
    }
}

/// Stub for beatoraja.skin.SkinSource
pub trait SkinSource {
    fn get_image(&self, time: i64, state: &dyn MainState) -> Option<TextureRegion>;
}

/// Stub for beatoraja.skin.SkinSourceImage
pub struct SkinSourceImage;

/// Stub for SkinObjectRenderer
pub struct SkinObjectRenderer;

impl SkinObjectRenderer {
    pub fn draw(&self, _image: &Option<TextureRegion>, _x: f32, _y: f32, _w: f32, _h: f32) {
        todo!()
    }
}

/// Stub for beatoraja.skin.SkinHeader
#[derive(Clone, Debug, Default)]
pub struct SkinHeader;

/// Stub for beatoraja.skin.Skin
pub struct SkinStub {
    pub input: i64,
}

impl SkinStub {
    pub fn get_input(&self) -> i64 {
        self.input
    }
}

// ============================================================
// beatoraja.skin.property types
// ============================================================

/// Stub for beatoraja.skin.property.EventFactory.EventType
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum EventType {
    Mode,
    Sort,
    Lnmode,
    Option1p,
    Option2p,
    Optiondp,
    Gauge1p,
    Hsfix,
    Target,
    Bga,
    GaugeAutoShift,
    NotesDisplayTiming,
    NotesDisplayTimingAutoAdjust,
    Duration1p,
    Rival,
    OpenDocument,
    OpenWithExplorer,
    OpenIr,
    FavoriteSong,
    FavoriteChart,
    UpdateFolder,
    OpenDownloadSite,
}

/// Stub for beatoraja.skin.property.StringPropertyFactory
pub struct StringPropertyFactory;

impl StringPropertyFactory {
    pub fn get_string_property(_name: &str) -> Box<dyn StringProperty> {
        todo!()
    }
}

pub trait StringProperty {
    fn get(&self, state: &dyn MainState) -> String;
}

// skin_property constants — re-exported from beatoraja-skin
pub use beatoraja_skin::skin_property;

// ============================================================
// beatoraja.SystemSoundManager
// ============================================================

/// Stub for SystemSoundManager
pub struct SystemSoundManager;

// SoundType — re-exported from beatoraja-core
pub use beatoraja_core::system_sound_manager::SoundType;

// ============================================================
// beatoraja.audio types
// ============================================================

/// Stub for AudioDriver
pub struct AudioDriver;

impl AudioDriver {
    pub fn play(&self, _path: &str, _volume: f32, _looping: bool) {}
    pub fn stop(&self, _path: &str) {}
    pub fn dispose(&self, _path: &str) {}
    pub fn is_playing(&self, _path: &str) -> bool {
        false
    }
    pub fn set_volume(&self, _path: &str, _volume: f32) {}
}

// ============================================================
// beatoraja.external types
// ============================================================

/// Stub for beatoraja.external.BMSSearchAccessor
pub struct BMSSearchAccessor;

impl BMSSearchAccessor {
    pub fn new(_tablepath: &str) -> Self {
        Self
    }
}

impl TableAccessor for BMSSearchAccessor {
    fn name(&self) -> &str {
        "BMS Search"
    }
    fn read(&self) -> Option<TableData> {
        None
    }
    fn write(&self, _td: &mut TableData) {}
}

// ============================================================
// beatoraja.modmenu types
// ============================================================

/// Stub for beatoraja.modmenu.ImGuiNotify
pub struct ImGuiNotify;

impl ImGuiNotify {
    pub fn info(_msg: &str) {
        log::info!("{}", _msg);
    }
    pub fn info_with_duration(_msg: &str, _duration: i32) {
        log::info!("{}", _msg);
    }
    pub fn warning(_msg: &str) {
        log::warn!("{}", _msg);
    }
    pub fn error(_msg: &str) {
        log::error!("{}", _msg);
    }
    pub fn error_with_duration(_msg: &str, _duration: i32) {
        log::error!("{}", _msg);
    }
}

/// Stub for beatoraja.modmenu.SongManagerMenu
pub struct SongManagerMenu;

impl SongManagerMenu {
    pub fn is_last_played_sort_enabled() -> bool {
        false
    }
    pub fn force_disable_last_played_sort() {}
}

/// Stub for beatoraja.modmenu.DownloadTaskState
pub struct DownloadTaskState;

impl DownloadTaskState {
    pub fn get_running_download_tasks() -> HashMap<String, DownloadTask> {
        HashMap::new()
    }
}

/// Stub for bms.tool.mdprocessor.DownloadTask
#[derive(Clone, Debug)]
pub struct DownloadTask {
    pub hash: String,
    pub download_size: i64,
    pub content_length: i64,
    pub status: DownloadTaskStatus,
}

impl DownloadTask {
    pub fn get_hash(&self) -> &str {
        &self.hash
    }
    pub fn get_download_size(&self) -> i64 {
        self.download_size
    }
    pub fn get_content_length(&self) -> i64 {
        self.content_length
    }
    pub fn get_download_task_status(&self) -> &DownloadTaskStatus {
        &self.status
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DownloadTaskStatus {
    Prepare,
    Downloading,
    Downloaded,
    Extracted,
    Error,
    Cancel,
}

/// Stub for beatoraja.MusicDownloadProcessor
pub struct MusicDownloadProcessor;

impl MusicDownloadProcessor {
    pub fn is_alive(&self) -> bool {
        false
    }
    pub fn start(&self, _song: &SongData) {
        todo!()
    }
}

/// Stub for bms.tool.mdprocessor.HttpDownloadProcessor
pub struct HttpDownloadProcessor;

impl HttpDownloadProcessor {
    pub fn submit_md5_task(&self, _md5: &str, _title: &str) {
        todo!()
    }
}

// ============================================================
// beatoraja.ScoreDataProperty
// ============================================================

/// Stub for ScoreDataProperty
pub struct ScoreDataProperty;

impl ScoreDataProperty {
    pub fn update(&self, _score: Option<&ScoreData>, _rival_score: Option<&ScoreData>) {}
}

// ============================================================
// beatoraja.MainLoader
// ============================================================

/// Stub for MainLoader
pub struct MainLoader;

impl MainLoader {
    pub fn get_illegal_song_count() -> i32 {
        0
    }
    pub fn get_illegal_songs() -> Vec<SongData> {
        vec![]
    }
}

// ============================================================
// beatoraja.PerformanceMetrics
// ============================================================

/// Stub for PerformanceMetrics
pub struct PerformanceMetrics;

// ============================================================
// bms.model.Mode — re-exported from real bms-model crate
// ============================================================

pub use ::bms_model::mode as bms_model;

// ============================================================
// bms.tool.util.Pair
// ============================================================

/// Stub for bms.tool.util.Pair
#[derive(Clone, Debug)]
pub struct Pair<A, B> {
    pub first: A,
    pub second: B,
}

impl<A, B> Pair<A, B> {
    pub fn of(first: A, second: B) -> Self {
        Self { first, second }
    }
    pub fn get_first(&self) -> &A {
        &self.first
    }
    pub fn get_second(&self) -> &B {
        &self.second
    }
}

impl<A: Clone, B: Clone> Pair<A, B> {
    pub fn project_first(pairs: &[Self]) -> Vec<A> {
        pairs.iter().map(|p| p.first.clone()).collect()
    }
}

// ============================================================
// Timer stub
// ============================================================

/// Stub for timer used in MainState
pub struct TimerState {
    pub now_time: i64,
}

impl TimerState {
    pub fn get_now_time(&self) -> i64 {
        self.now_time
    }
    pub fn get_timer(&self, _id: i32) -> i64 {
        0
    }
    pub fn set_timer_on(&self, _id: i32) {}
    pub fn set_timer_off(&self, _id: i32) {}
    pub fn switch_timer(&self, _id: i32, _on: bool) {}
}

// ============================================================
// Clipboard stub
// ============================================================

/// Stub for clipboard access
pub struct Clipboard;

impl Clipboard {
    pub fn set_contents(_text: &str) {
        // stub: would copy to system clipboard
    }
}
