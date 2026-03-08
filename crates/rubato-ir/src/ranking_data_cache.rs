use std::any::Any;
use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard};

use log::warn;
use sha2::{Digest, Sha256};

use rubato_core::course_data::CourseData;
use rubato_core::stubs::SongData;
use rubato_types::ranking_data_cache_access::RankingDataCacheAccess;

use crate::convert_hex_string;
use crate::ranking_data::RankingData;

/// IR access data cache
///
/// Translated from: RankingDataCache.java
#[derive(Clone)]
pub struct RankingDataCache {
    inner: Arc<Mutex<RankingDataCacheInner>>,
}

struct RankingDataCacheInner {
    /// Score cache: indexed by lnmode (0-3)
    scorecache: [HashMap<String, RankingData>; 4],
    /// Course score cache: indexed by lnmode (0-3)
    cscorecache: [HashMap<String, RankingData>; 4],
}

fn lock_or_recover<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    match mutex.lock() {
        Ok(guard) => guard,
        Err(poisoned) => poisoned.into_inner(),
    }
}

impl Default for RankingDataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingDataCache {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(Mutex::new(RankingDataCacheInner {
                scorecache: [
                    HashMap::with_capacity(2000),
                    HashMap::with_capacity(2000),
                    HashMap::with_capacity(2000),
                    HashMap::with_capacity(2000),
                ],
                cscorecache: [
                    HashMap::with_capacity(100),
                    HashMap::with_capacity(100),
                    HashMap::with_capacity(100),
                    HashMap::with_capacity(100),
                ],
            })),
        }
    }

    fn song_cache_index(song: &SongData, lnmode: i32) -> usize {
        if song.chart.has_undefined_long_note() {
            lnmode as usize
        } else {
            3
        }
    }

    fn course_cache_index(course: &CourseData, lnmode: i32) -> usize {
        let mut cacheindex = 3usize;
        for song in &course.hash {
            if song.chart.has_undefined_long_note() {
                cacheindex = lnmode as usize;
            }
        }
        cacheindex
    }

    /// Get ranking data for a song with given LN mode. Returns None if not found.
    pub fn song(&self, song: &SongData, lnmode: i32) -> Option<RankingData> {
        let cacheindex = if song.chart.has_undefined_long_note() {
            lnmode as usize
        } else {
            3
        };
        let sha256 = song.file.sha256.clone();
        lock_or_recover(&self.inner).scorecache[cacheindex]
            .get(&sha256)
            .cloned()
    }

    /// Get ranking data for a course with given LN mode. Returns None if not found.
    pub fn course(&self, course: &CourseData, lnmode: i32) -> Option<RankingData> {
        let cacheindex = Self::course_cache_index(course, lnmode);
        if let Some(hash) = self.create_course_hash(course) {
            lock_or_recover(&self.inner).cscorecache[cacheindex]
                .get(&hash)
                .cloned()
        } else {
            None
        }
    }

    /// Put ranking data for a song with given LN mode.
    pub fn put_song(&mut self, song: &SongData, lnmode: i32, iras: RankingData) {
        let cacheindex = Self::song_cache_index(song, lnmode);
        let sha256 = song.file.sha256.clone();
        lock_or_recover(&self.inner).scorecache[cacheindex].insert(sha256, iras);
    }

    /// Put ranking data for a course with given LN mode.
    pub fn put_course(&mut self, course: &CourseData, lnmode: i32, iras: RankingData) {
        let cacheindex = Self::course_cache_index(course, lnmode);
        if let Some(hash) = self.create_course_hash(course) {
            lock_or_recover(&self.inner).cscorecache[cacheindex].insert(hash, iras);
        }
    }

    fn create_course_hash(&self, course: &CourseData) -> Option<String> {
        let mut sb = String::new();
        for song in &course.hash {
            let sha256 = song.file.sha256.clone();
            if sha256.len() == 64 {
                sb.push_str(&sha256);
            } else {
                return None;
            }
        }
        for constraint in &course.constraint {
            sb.push_str(constraint.name_str());
        }
        let mut hasher = Sha256::new();
        hasher.update(sb.as_bytes());
        let result = hasher.finalize();
        Some(convert_hex_string(&result))
    }
}

impl RankingDataCacheAccess for RankingDataCache {
    fn clone_box(&self) -> Box<dyn RankingDataCacheAccess> {
        Box::new(self.clone())
    }

    fn song_any(&self, song: &SongData, lnmode: i32) -> Option<Box<dyn Any>> {
        self.song(song, lnmode)
            .map(|ranking| Box::new(ranking) as Box<dyn Any>)
    }

    fn course_any(&self, course: &CourseData, lnmode: i32) -> Option<Box<dyn Any>> {
        self.course(course, lnmode)
            .map(|ranking| Box::new(ranking) as Box<dyn Any>)
    }

    fn put_song_any(&mut self, song: &SongData, lnmode: i32, data: Box<dyn Any>) {
        if let Ok(ranking) = data.downcast::<RankingData>() {
            self.put_song(song, lnmode, *ranking);
        } else {
            warn!("RankingDataCache::put_song_any: unexpected type (expected RankingData)");
        }
    }

    fn put_course_any(&mut self, course: &CourseData, lnmode: i32, data: Box<dyn Any>) {
        if let Ok(ranking) = data.downcast::<RankingData>() {
            self.put_course(course, lnmode, *ranking);
        } else {
            warn!("RankingDataCache::put_course_any: unexpected type (expected RankingData)");
        }
    }
}
