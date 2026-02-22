// External dependency stubs for beatoraja-select
// Types that can be replaced with real implementations are re-exported from beatoraja-core.
// Remaining stubs are for types that cannot be replaced due to API incompatibilities.

// ============================================================
// LibGDX types — re-exported from beatoraja-skin stubs
// ============================================================

pub use beatoraja_skin::stubs::Color;
pub use beatoraja_skin::stubs::Pixmap;
pub use beatoraja_skin::stubs::PixmapFormat;
pub use beatoraja_skin::stubs::Rectangle;
pub use beatoraja_skin::stubs::Texture;
pub use beatoraja_skin::stubs::TextureRegion;

// ============================================================
// beatoraja core types — re-exported from real implementations
// ============================================================

pub use beatoraja_core::audio_config::AudioConfig;
pub use beatoraja_core::config::{Config, SongPreview};
pub use beatoraja_core::play_config::PlayConfig;
pub use beatoraja_core::player_config::PlayerConfig;
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

// ============================================================
// beatoraja core types (stubbed — cannot be replaced)
// ============================================================

/// Stub for beatoraja.MainState
/// Empty marker trait — get_main() removed (MainController stub deleted, Phase 18e-2).
/// Retained because SkinText/SkinNumber/SkinImage/RankingData stubs accept &dyn MainState.
pub trait MainState {}

/// MainStateType — re-exported from beatoraja-types (Phase 15d)
pub use beatoraja_types::main_state_type::MainStateType;

// BMSPlayerMode: replaced by pub use from beatoraja_core (Phase 18e-7)
pub use beatoraja_core::bms_player_mode::BMSPlayerMode;
// Alias Mode as BMSPlayerModeType to avoid naming conflict with bms_model::mode::Mode
pub use beatoraja_core::bms_player_mode::Mode as BMSPlayerModeType;

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
        log::warn!("not yet implemented: RandomCourseData.get_song_datas");
        Vec::new()
    }
    pub fn lottery_song_datas(&self) {
        log::warn!("not yet implemented: RandomCourseData.lottery_song_datas");
    }
    pub fn create_course_data(&self) -> CourseData {
        log::warn!("not yet implemented: RandomCourseData.create_course_data");
        CourseData::default()
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
        log::warn!("not yet implemented: RankingData.load_song");
    }
    pub fn load_course(&self, _selector: &dyn MainState, _course: &CourseData) {
        log::warn!("not yet implemented: RankingData.load_course");
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
        log::warn!("not yet implemented: RankingDataCache.put_song");
    }
    pub fn put_course(&self, _course: &CourseData, _lnmode: i32, _data: RankingData) {
        log::warn!("not yet implemented: RankingDataCache.put_course");
    }
}

// ============================================================
// beatoraja.input types
// ============================================================

pub use beatoraja_input::bms_player_input_processor::BMSPlayerInputProcessor;
pub use beatoraja_input::key_command::KeyCommand;
pub use beatoraja_input::keyboard_input_processor::ControlKeys;

// ============================================================
// beatoraja.ir types — replaced with real types from beatoraja-ir
// ============================================================

pub use beatoraja_ir::ir_connection::IRConnection;

/// MainController.IRStatus — uses dyn IRConnection trait
pub struct IRStatus {
    pub connection: Box<dyn IRConnection>,
    pub player: beatoraja_ir::ir_player_data::IRPlayerData,
}

// LeaderboardEntry — replaced with real type from beatoraja-ir
pub use beatoraja_ir::leaderboard_entry::LeaderboardEntry;

// IRScoreData — re-exported from beatoraja-ir
pub use beatoraja_ir::ir_score_data::IRScoreData;

// ============================================================
// beatoraja.skin types
// ============================================================

// SkinType moved to beatoraja-types (Phase 15b)
pub use beatoraja_types::skin_type::SkinType;

/// Stub for beatoraja.skin.SkinHeader
pub struct SkinHeader;

/// Stub for beatoraja.skin.SkinText
#[derive(Clone, Debug, Default)]
pub struct SkinText;
impl SkinText {
    pub fn set_text(&self, _text: &str) {}
    pub fn draw(&self, _sprite: &SkinObjectRenderer, _x: f32, _y: f32) {
        log::warn!("not yet implemented: SkinText.draw - rendering dependency");
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
        log::warn!("not yet implemented: SkinNumber.draw - rendering dependency");
    }
    pub fn prepare(&self, _time: i64, _state: &dyn MainState) {}
    pub fn validate(&self) -> bool {
        true
    }
}

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
        log::warn!("not yet implemented: SkinImage.draw - rendering dependency");
    }
    pub fn draw_offset(&self, _sprite: &SkinObjectRenderer, _dx: f32, _dy: f32) {
        log::warn!("not yet implemented: SkinImage.draw_offset - rendering dependency");
    }
    pub fn prepare(&self, _time: i64, _state: &dyn MainState) {}
    pub fn validate(&self) -> bool {
        true
    }
    pub fn get_destination(&self, _time: i64, _state: &dyn MainState) -> Option<Rectangle> {
        None
    }
}

/// Stub for SkinObjectRenderer
pub struct SkinObjectRenderer;

impl SkinObjectRenderer {
    pub fn draw(&self, _image: &Option<TextureRegion>, _x: f32, _y: f32, _w: f32, _h: f32) {
        log::warn!("not yet implemented: SkinObjectRenderer.draw - rendering dependency");
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

// skin_property constants — re-exported from beatoraja-skin
pub use beatoraja_skin::skin_property;

// SoundType — re-exported from beatoraja-core
pub use beatoraja_core::system_sound_manager::SoundType;

// ============================================================
// beatoraja.modmenu types
// ============================================================

// Real type re-export (replaced from stubs)
pub use beatoraja_types::imgui_notify::ImGuiNotify;

// ============================================================
// bms.model.Mode — re-exported from real bms-model crate
// ============================================================

pub use ::bms_model::mode as bms_model;

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

// PlayerInformation: replaced by pub use from beatoraja_core (Phase 18e-7)
pub use beatoraja_core::player_information::PlayerInformation;

// ============================================================
// AudioDriver — stub for beatoraja.audio.AudioDriver
// ============================================================

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
// Resolution — re-exported from beatoraja-core
// ============================================================

pub use beatoraja_core::resolution::Resolution;

// ============================================================
// NullSongDatabaseAccessor — stub implementing SongDatabaseAccessor
// ============================================================

pub struct NullSongDatabaseAccessor;

impl SongDatabaseAccessor for NullSongDatabaseAccessor {
    fn get_song_datas(&self, _key: &str, _value: &str) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas");
        Vec::new()
    }
    fn get_song_datas_by_hashes(&self, _hashes: &[String]) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_hashes");
        Vec::new()
    }
    fn get_song_datas_by_sql(
        &self,
        _sql: &str,
        _score: &str,
        _scorelog: &str,
        _info: Option<&str>,
    ) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_sql");
        Vec::new()
    }
    fn set_song_datas(&self, _songs: &[SongData]) {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.set_song_datas");
    }
    fn get_song_datas_by_text(&self, _text: &str) -> Vec<SongData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_song_datas_by_text");
        Vec::new()
    }
    fn get_folder_datas(&self, _key: &str, _value: &str) -> Vec<FolderData> {
        log::warn!("not yet implemented: NullSongDatabaseAccessor.get_folder_datas");
        Vec::new()
    }
}

// Clipboard: removed (replaced with direct arboard calls — Phase 18e-6)

// ============================================================
// SongManagerMenu — stub for beatoraja.select.SongManagerMenu
// ============================================================

// SongManagerMenu: last-played-sort state moved to beatoraja-types (Phase 18e-8)
// Thin wrapper preserved for API compatibility
pub struct SongManagerMenu;
impl SongManagerMenu {
    pub fn is_last_played_sort_enabled() -> bool {
        beatoraja_types::last_played_sort::is_enabled()
    }
    pub fn force_disable_last_played_sort() {
        beatoraja_types::last_played_sort::force_disable();
    }
}

// ============================================================
// Download task types — stubs for beatoraja.external download
// ============================================================

pub struct DownloadTaskState;

impl DownloadTaskState {
    pub fn get_running_download_tasks() -> std::collections::HashMap<String, DownloadTask> {
        std::collections::HashMap::new()
    }
}

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
