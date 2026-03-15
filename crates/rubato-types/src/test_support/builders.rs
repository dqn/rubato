//! Convenience builders for creating test data instances.

use crate::course_data::CourseData;
use crate::folder_data::FolderData;
use crate::replay_data::ReplayData;
use crate::score_data::ScoreData;
use crate::song_data::SongData;

/// Create a default SongData with the given title.
pub fn song_data_with_title(title: &str) -> SongData {
    let mut s = SongData::default();
    s.metadata.title = title.to_string();
    s
}

/// Create a default SongData with the given SHA-256 hash.
pub fn song_data_with_hash(sha256: &str) -> SongData {
    let mut s = SongData::default();
    s.file.sha256 = sha256.to_string();
    s
}

/// Create a default SongData with title and path.
pub fn song_data_with_title_and_path(title: &str, path: &str) -> SongData {
    let mut s = SongData::default();
    s.metadata.title = title.to_string();
    s.file.set_path(path.to_string());
    s
}

/// Create a default ScoreData with the given EX score.
pub fn score_data_with_exscore(exscore: i32) -> ScoreData {
    let mut s = ScoreData::default();
    s.judge_counts.epg = exscore / 2;
    s.judge_counts.egr = exscore % 2;
    s
}

/// Create a default FolderData with the given name.
pub fn folder_data_with_name(name: &str) -> FolderData {
    let mut f = FolderData::default();
    f.title = name.to_string();
    f
}

/// Create a default ReplayData.
pub fn replay_data_default() -> ReplayData {
    ReplayData::default()
}

/// Create a default CourseData with the given name.
pub fn course_data_with_name(name: &str) -> CourseData {
    let mut c = CourseData::default();
    c.name = Some(name.to_string());
    c
}
