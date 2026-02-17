//! Song and score database layer backed by rusqlite.
//!
//! Provides [`SongDatabase`] for indexing BMS files with metadata ([`SongData`]),
//! [`ScoreDatabase`] for persisting play results ([`ScoreDataProperty`]),
//! [`CourseDataAccessor`] and [`TableDataAccessor`] for difficulty tables and courses,
//! and [`RivalDataAccessor`] for rival score comparisons.
//! All database access is centralized here to keep SQL logic out of game logic.

pub mod course_data;
pub mod course_data_accessor;
pub mod difficulty_table_parser;
pub mod folder_data;
pub mod player_data;
pub mod player_info;
pub mod random_course_data;
pub mod rival_data_accessor;
pub mod schema;
pub mod score_data_property;
pub mod score_database;
pub mod score_log_database;
pub mod song_data;
pub mod song_database;
pub mod song_information;
pub mod song_information_accessor;
pub mod song_utils;
pub mod table_data;
pub mod table_data_accessor;

pub use course_data::{CourseData, CourseDataConstraint, CourseSongData, TrophyData};
pub use course_data_accessor::CourseDataAccessor;
pub use difficulty_table_parser::{
    DifficultyTableHeader, ParsedChart, ParsedCourse, apply_data_rule, extract_bmstable_url,
    parse_json_data, parse_json_header, resolve_url, to_table_data,
};
pub use folder_data::FolderData;
pub use player_data::PlayerData;
pub use player_info::PlayerInformation;
pub use random_course_data::{RandomCourseData, RandomCourseDataConstraint, RandomStageData};
pub use rival_data_accessor::RivalDataAccessor;
pub use score_data_property::ScoreDataProperty;
pub use score_database::ScoreDatabase;
pub use score_log_database::ScoreDataLogDatabase;
pub use song_data::SongData;
pub use song_database::SongDatabase;
pub use song_information::SongInformation;
pub use song_information_accessor::SongInformationAccessor;
pub use song_utils::{ILLEGAL_SONGS, crc32_path};
pub use table_data::{TableData, TableFolder};
pub use table_data_accessor::TableDataAccessor;
