// External dependency stubs for beatoraja-select
// Types that can be replaced with real implementations are re-exported from beatoraja-core.
// Remaining stubs are for types that cannot be replaced due to API incompatibilities.

// ============================================================
// LibGDX types — re-exported from beatoraja-skin stubs
// ============================================================

pub use rubato_skin::stubs::Color;
pub use rubato_skin::stubs::Pixmap;
pub use rubato_skin::stubs::PixmapFormat;
pub use rubato_skin::stubs::Rectangle;
pub use rubato_skin::stubs::Texture;
pub use rubato_skin::stubs::TextureRegion;

// ============================================================
// beatoraja core types — re-exported from real implementations
// ============================================================

pub use rubato_core::audio_config::AudioConfig;
pub use rubato_core::config::{BgaMode, Config, SongPreview};
pub use rubato_core::play_config::PlayConfig;
pub use rubato_core::player_config::PlayerConfig;
pub use rubato_core::score_data::ScoreData;

// ============================================================
// beatoraja.song types — real SongData from beatoraja-types
// ============================================================

pub use rubato_types::song_data::SongData;
pub use rubato_types::song_data::{
    FAVORITE_CHART, FAVORITE_SONG, FEATURE_CHARGENOTE, FEATURE_HELLCHARGENOTE, FEATURE_LONGNOTE,
    FEATURE_MINENOTE, FEATURE_RANDOM, FEATURE_UNDEFINEDLN, INVISIBLE_CHART, INVISIBLE_SONG,
};

// ============================================================
// beatoraja.song.FolderData — replaced with real type from beatoraja-types
// ============================================================

pub use rubato_types::folder_data::FolderData;

// ============================================================
// beatoraja.song.SongDatabaseAccessor — replaced with real trait from beatoraja-types
// ============================================================

pub use rubato_types::song_database_accessor::SongDatabaseAccessor;

// ============================================================
// beatoraja core types (stubbed — cannot be replaced)
// ============================================================

// MainState — re-exported from beatoraja-skin (Phase 25d-2)
pub use rubato_skin::stubs::MainState;

/// MainStateType — re-exported from beatoraja-types (Phase 15d)
pub use rubato_types::main_state_type::MainStateType;

// BMSPlayerMode: replaced by pub use from rubato_core (Phase 18e-7)
pub use rubato_core::bms_player_mode::BMSPlayerMode;
// Alias Mode as BMSPlayerModeType to avoid naming conflict with bms_model::mode::Mode
pub use rubato_core::bms_player_mode::Mode as BMSPlayerModeType;

// beatoraja.CourseData / TrophyData / CourseDataConstraint — replaced with real types from beatoraja-types (Phase 15g)
pub use rubato_types::course_data::{CourseData, CourseDataConstraint, TrophyData};

// RandomCourseData: replaced by pub use from beatoraja-core (Phase 18e-10)
pub use rubato_core::random_course_data::RandomCourseData;

// RandomStageData: replaced by pub use from beatoraja-core (Phase 18e-10)
pub use rubato_core::random_stage_data::RandomStageData;

// beatoraja.TableData / TableFolder — replaced with real types from beatoraja-core (Phase 15g)
pub use rubato_core::table_data::{TableData, TableFolder};

// beatoraja.TableDataAccessor / TableAccessor / DifficultyTableAccessor — replaced with real types from beatoraja-core (Phase 15g)
pub use rubato_core::table_data_accessor::{
    DifficultyTableAccessor, TableAccessor, TableAccessorUpdateSource, TableDataAccessor,
};

// beatoraja.CourseDataAccessor — replaced with real type from beatoraja-core (Phase 15g)
pub use rubato_core::course_data_accessor::CourseDataAccessor;

// RankingData: replaced by pub use from beatoraja-ir (Phase 18e-11)
pub use rubato_ir::ranking_data::RankingData;

// RankingDataCache: concrete type from beatoraja-ir (Phase 18e-11)
// The trait RankingDataCacheAccess is in beatoraja-types for cross-crate bridging.
pub use rubato_ir::ranking_data_cache::RankingDataCache;
pub use rubato_types::ranking_data_cache_access::RankingDataCacheAccess;

// ============================================================
// beatoraja.input types
// ============================================================

pub use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
pub use rubato_input::key_command::KeyCommand;
pub use rubato_input::keyboard_input_processor::ControlKeys;

// ============================================================
// beatoraja.ir types — replaced with real types from beatoraja-ir
// ============================================================

pub use rubato_ir::ir_connection::IRConnection;

/// MainController.IRStatus — uses dyn IRConnection trait
pub struct IRStatus {
    pub connection: Box<dyn IRConnection>,
    pub player: rubato_ir::ir_player_data::IRPlayerData,
}

// LeaderboardEntry — replaced with real type from beatoraja-ir
pub use rubato_ir::leaderboard_entry::LeaderboardEntry;

// IRScoreData — re-exported from beatoraja-ir
pub use rubato_ir::ir_score_data::IRScoreData;

// ============================================================
// beatoraja.skin types
// ============================================================

// SkinType moved to beatoraja-types (Phase 15b)
pub use rubato_types::skin_type::SkinType;

// SkinHeader: replaced by pub use from beatoraja-skin (Phase 18e-10)
pub use rubato_skin::skin_header::SkinHeader;

// Real types re-exported from beatoraja-skin (stubs replaced post-62)
pub use rubato_skin::skin_image::SkinImage;
pub use rubato_skin::skin_number::SkinNumber;
pub use rubato_skin::skin_object::SkinObjectRenderer;
pub use rubato_skin::skin_text::{SkinText, SkinTextData};

/// SkinRegion — alias for Rectangle (same fields: x, y, width, height).
/// Kept as type alias to minimize churn in select crate code.
pub type SkinRegion = Rectangle;

// ============================================================
// beatoraja.skin.property types
// ============================================================

// EventType — re-exported from beatoraja-types (Phase 25d-2)
pub use rubato_types::event_type::EventType;

// skin_property constants — re-exported from beatoraja-skin
pub use rubato_skin::skin_property;

// SoundType — re-exported from beatoraja-core
pub use rubato_core::system_sound_manager::SoundType;

// ============================================================
// beatoraja.modmenu types
// ============================================================

// Real type re-export (replaced from stubs)
pub use rubato_types::imgui_notify::ImGuiNotify;

// ============================================================
// bms.model.Mode — re-exported from real bms-model crate
// ============================================================

pub use ::bms_model::mode as bms_model;

// TimerState: removed (dead code — never used outside stubs.rs)

// PlayerInformation: replaced by pub use from rubato_core (Phase 18e-7)
pub use rubato_core::player_information::PlayerInformation;

// ============================================================
// Resolution — re-exported from beatoraja-core
// ============================================================

pub use rubato_core::resolution::Resolution;

// NullSongDatabaseAccessor: moved to crate::null_song_database_accessor (Phase 18e-10)
pub use super::null_song_database_accessor::NullSongDatabaseAccessor;

// Clipboard: removed (replaced with direct arboard calls — Phase 18e-6)

// ============================================================
// SongManagerMenu — stub for beatoraja.select.SongManagerMenu
// ============================================================

// SongManagerMenu: last-played-sort state moved to beatoraja-types (Phase 18e-8)
// Thin wrapper preserved for API compatibility
pub struct SongManagerMenu;
impl SongManagerMenu {
    pub fn is_last_played_sort_enabled() -> bool {
        rubato_types::last_played_sort::is_enabled()
    }
    pub fn force_disable_last_played_sort() {
        rubato_types::last_played_sort::force_disable();
    }
}

// ============================================================
// Download task types — re-exported from md-processor
// ============================================================

pub use rubato_song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};
