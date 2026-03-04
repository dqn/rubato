use std::any::Any;

use crate::course_data::CourseData;
use crate::song_data::SongData;

/// Trait bridge for IR ranking data cache operations.
/// Implemented in beatoraja-ir (RankingDataCache), consumed by beatoraja-core and beatoraja-select.
/// Breaks the core→ir circular dependency.
///
/// The trait uses `dyn Any` for ranking data values because `RankingData` lives in beatoraja-ir
/// and cannot be referenced from beatoraja-types. Callers downcast via
/// `value.downcast_ref::<RankingData>()`.
pub trait RankingDataCacheAccess: Send + Sync {
    /// Get ranking data for a song with given LN mode.
    /// Returns the ranking data as `&dyn Any` (downcast to `RankingData`).
    fn get_song_any(&self, song: &SongData, lnmode: i32) -> Option<&dyn Any>;

    /// Get ranking data for a course with given LN mode.
    /// Returns the ranking data as `&dyn Any` (downcast to `RankingData`).
    fn get_course_any(&self, course: &CourseData, lnmode: i32) -> Option<&dyn Any>;

    /// Put ranking data for a song with given LN mode.
    /// The `data` parameter should be a `Box<RankingData>`.
    fn put_song_any(&mut self, song: &SongData, lnmode: i32, data: Box<dyn Any>);

    /// Put ranking data for a course with given LN mode.
    /// The `data` parameter should be a `Box<RankingData>`.
    fn put_course_any(&mut self, course: &CourseData, lnmode: i32, data: Box<dyn Any>);
}
