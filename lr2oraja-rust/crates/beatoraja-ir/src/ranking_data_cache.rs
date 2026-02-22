use std::collections::HashMap;

use sha2::{Digest, Sha256};

use beatoraja_core::course_data::CourseData;
use beatoraja_core::stubs::SongData;

use crate::convert_hex_string;
use crate::ranking_data::RankingData;

/// IR access data cache
///
/// Translated from: RankingDataCache.java
pub struct RankingDataCache {
    /// Score cache: indexed by lnmode (0-3)
    scorecache: [HashMap<String, RankingData>; 4],
    /// Course score cache: indexed by lnmode (0-3)
    cscorecache: [HashMap<String, RankingData>; 4],
}

impl Default for RankingDataCache {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingDataCache {
    pub fn new() -> Self {
        Self {
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
        }
    }

    /// Get ranking data for a song with given LN mode. Returns None if not found.
    pub fn get_song(&self, song: &SongData, lnmode: i32) -> Option<&RankingData> {
        let cacheindex = if song.has_undefined_long_note() {
            lnmode as usize
        } else {
            3
        };
        let sha256 = song.sha256.clone();
        self.scorecache[cacheindex].get(&sha256)
    }

    /// Get ranking data for a course with given LN mode. Returns None if not found.
    pub fn get_course(&self, course: &CourseData, lnmode: i32) -> Option<&RankingData> {
        let mut cacheindex = 3usize;
        for song in course.get_song() {
            if song.has_undefined_long_note() {
                cacheindex = lnmode as usize;
            }
        }
        if let Some(hash) = self.create_course_hash(course) {
            self.cscorecache[cacheindex].get(&hash)
        } else {
            None
        }
    }

    /// Put ranking data for a song with given LN mode.
    pub fn put_song(&mut self, song: &SongData, lnmode: i32, iras: RankingData) {
        let cacheindex = if song.has_undefined_long_note() {
            lnmode as usize
        } else {
            3
        };
        let sha256 = song.sha256.clone();
        self.scorecache[cacheindex].insert(sha256, iras);
    }

    /// Put ranking data for a course with given LN mode.
    pub fn put_course(&mut self, course: &CourseData, lnmode: i32, iras: RankingData) {
        let mut cacheindex = 3usize;
        for song in course.get_song() {
            if song.has_undefined_long_note() {
                cacheindex = lnmode as usize;
            }
        }
        if let Some(hash) = self.create_course_hash(course) {
            self.cscorecache[cacheindex].insert(hash, iras);
        }
    }

    fn create_course_hash(&self, course: &CourseData) -> Option<String> {
        let mut sb = String::new();
        for song in course.get_song() {
            let sha256 = song.sha256.clone();
            if sha256.len() == 64 {
                sb.push_str(&sha256);
            } else {
                return None;
            }
        }
        for constraint in course.get_constraint() {
            sb.push_str(constraint.name_str());
        }
        let mut hasher = Sha256::new();
        hasher.update(sb.as_bytes());
        let result = hasher.finalize();
        Some(convert_hex_string(&result))
    }
}
