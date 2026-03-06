use std::any::Any;

use crate::course_data::CourseData;
use crate::song_data::SongData;

/// Trait bridge for IR ranking data cache operations.
/// Implemented in beatoraja-ir (RankingDataCache), consumed by beatoraja-core and beatoraja-select.
/// Breaks the core→ir circular dependency.
///
/// The trait uses `dyn Any` for ranking data values because `RankingData` lives in beatoraja-ir
/// and cannot be referenced from beatoraja-types. Callers downcast the returned box via
/// `value.downcast::<RankingData>()`.
pub trait RankingDataCacheAccess: Send + Sync {
    /// Clone this cache handle.
    fn clone_box(&self) -> Box<dyn RankingDataCacheAccess>;

    /// Get ranking data for a song with given LN mode.
    /// Returns the ranking data as `Box<dyn Any>` (downcast to `RankingData`).
    fn song_any(&self, song: &SongData, lnmode: i32) -> Option<Box<dyn Any>>;
    /// Get ranking data for a course with given LN mode.
    /// Returns the ranking data as `Box<dyn Any>` (downcast to `RankingData`).
    fn course_any(&self, course: &CourseData, lnmode: i32) -> Option<Box<dyn Any>>;
    /// Put ranking data for a song with given LN mode.
    /// The `data` parameter should be a `Box<RankingData>`.
    fn put_song_any(&mut self, song: &SongData, lnmode: i32, data: Box<dyn Any>);

    /// Put ranking data for a course with given LN mode.
    /// The `data` parameter should be a `Box<RankingData>`.
    fn put_course_any(&mut self, course: &CourseData, lnmode: i32, data: Box<dyn Any>);
}

impl Clone for Box<dyn RankingDataCacheAccess> {
    fn clone(&self) -> Self {
        self.clone_box()
    }
}
