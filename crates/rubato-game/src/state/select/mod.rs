pub mod ir_status;

// Re-exports

// LibGDX types
pub use rubato_skin::reexports::Color;
pub use rubato_skin::reexports::Pixmap;
pub use rubato_skin::reexports::PixmapFormat;
pub use rubato_skin::reexports::Rectangle;
pub use rubato_skin::reexports::Texture;
pub use rubato_skin::reexports::TextureRegion;

// beatoraja core types
pub use crate::core::audio_config::AudioConfig;
pub use crate::core::config::{BgaMode, Config, SongPreview};
pub use crate::core::play_config::PlayConfig;
pub use crate::core::player_config::PlayerConfig;
pub use crate::core::score_data::ScoreData;

// beatoraja.song types
pub use rubato_types::song_data::SongData;
pub use rubato_types::song_data::{
    FAVORITE_CHART, FAVORITE_SONG, FEATURE_CHARGENOTE, FEATURE_HELLCHARGENOTE, FEATURE_LONGNOTE,
    FEATURE_MINENOTE, FEATURE_RANDOM, FEATURE_UNDEFINEDLN, INVISIBLE_CHART, INVISIBLE_SONG,
};

// FolderData
pub use rubato_types::folder_data::FolderData;

// SongDatabaseAccessor
pub use rubato_types::song_database_accessor::SongDatabaseAccessor;

// MainState
pub use rubato_skin::reexports::MainState;

// MainStateType
pub use rubato_types::main_state_type::MainStateType;

// BMSPlayerMode
pub use crate::core::bms_player_mode::BMSPlayerMode;
pub use crate::core::bms_player_mode::Mode as BMSPlayerModeType;

// CourseData / TrophyData / CourseDataConstraint
pub use rubato_types::course_data::{CourseData, CourseDataConstraint, TrophyData};

// RandomCourseData / RandomStageData
pub use crate::core::random_course_data::RandomCourseData;
pub use crate::core::random_stage_data::RandomStageData;

// TableData / TableFolder
pub use crate::core::table_data::{TableData, TableFolder};

// TableDataAccessor / TableAccessor / DifficultyTableAccessor
pub use crate::core::table_data_accessor::{
    DifficultyTableAccessor, TableAccessor, TableAccessorUpdateSource, TableDataAccessor,
};

// CourseDataAccessor
pub use crate::core::course_data_accessor::CourseDataAccessor;

// RankingData / RankingDataCache
pub use crate::ir::ranking_data::RankingData;
pub use crate::ir::ranking_data_cache::RankingDataCache;
pub use rubato_types::ranking_data_cache_access::RankingDataCacheAccess;

// beatoraja.input types
pub use rubato_input::bms_player_input_processor::BMSPlayerInputProcessor;
pub use rubato_input::key_command::KeyCommand;
pub use rubato_input::keyboard_input_processor::ControlKeys;

// beatoraja.ir types
pub use crate::ir::ir_connection::IRConnection;
pub use crate::ir::ir_score_data::IRScoreData;
pub use crate::ir::leaderboard_entry::LeaderboardEntry;

// beatoraja.skin types
pub use rubato_skin::skin_header::SkinHeader;
pub use rubato_skin::skin_image::SkinImage;
pub use rubato_skin::skin_number::SkinNumber;
pub use rubato_skin::skin_object::SkinObjectRenderer;
pub use rubato_skin::skin_text::{SkinText, SkinTextData, SkinTextEnum};
pub use rubato_types::skin_type::SkinType;

/// SkinRegion -- alias for Rectangle (same fields: x, y, width, height).
/// Kept as type alias to minimize churn in select crate code.
pub type SkinRegion = Rectangle;

// beatoraja.skin.property types
pub use rubato_skin::skin_property;
pub use rubato_types::event_type::EventType;

// SoundType
pub use crate::core::system_sound_manager::SoundType;

// beatoraja.modmenu types
pub use rubato_types::imgui_notify::ImGuiNotify;

// bms.model.Mode
pub use ::bms::model::mode as bms_model;

// PlayerInformation
pub use crate::core::player_information::PlayerInformation;

// Resolution
pub use crate::core::resolution::Resolution;

// NullSongDatabaseAccessor
pub use self::null_song_database_accessor::NullSongDatabaseAccessor;

// MusicSelector -- SongSelectionAccess trait
pub use rubato_types::song_selection_access::SongSelectionAccess;

// Download task types
pub use crate::song::md_processor::download_task::{DownloadTask, DownloadTaskStatus};

// Bar types (select.bar package)
pub mod bar;

// Select screen modules
pub mod bar_manager;
pub mod bar_renderer;
pub mod bar_sorter;
pub mod music_select_command;
pub mod music_select_input_processor;
pub mod music_select_key_property;
pub mod music_select_skin;
pub mod music_selector;
pub mod null_song_database_accessor;
pub mod preview_music_processor;
pub mod score_data_cache;
pub mod search_text_field;
pub mod skin_bar;
pub mod skin_distribution_graph;
