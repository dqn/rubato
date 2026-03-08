use std::collections::HashMap;

use crate::score_data::ScoreData;
use crate::song_data::SongData;

type ReadSingleFn = Box<dyn Fn(&SongData, i32) -> Option<ScoreData> + Send + Sync>;
type ReadMultiFn =
    Box<dyn Fn(&dyn Fn(&SongData, Option<&ScoreData>), &[SongData], i32) + Send + Sync>;

/// Score data cache
/// Translates: bms.player.beatoraja.select.ScoreDataCache
pub struct ScoreDataCache {
    /// Score data caches per LN mode (indices 0..3, where 3 is for songs without undefined LN)
    scorecache: [HashMap<String, Option<ScoreData>>; 4],
    /// Function to read a single score from the source
    read_single: ReadSingleFn,
    /// Function to read multiple scores from the source
    read_multi: ReadMultiFn,
}

impl ScoreDataCache {
    pub fn new(read_single: ReadSingleFn, read_multi: ReadMultiFn) -> Self {
        Self {
            scorecache: [
                HashMap::with_capacity(2000),
                HashMap::with_capacity(2000),
                HashMap::with_capacity(2000),
                HashMap::with_capacity(2000),
            ],
            read_single,
            read_multi,
        }
    }

    fn cache_index(song: &SongData, lnmode: i32) -> usize {
        if song.has_undefined_long_note() {
            lnmode as usize
        } else {
            3
        }
    }

    /// Read score data for the given song and LN mode
    pub fn read_score_data(&mut self, song: &SongData, lnmode: i32) -> Option<&ScoreData> {
        let cacheindex = Self::cache_index(song, lnmode);
        let sha256 = &song.file.sha256;
        if !self.scorecache[cacheindex].contains_key(sha256.as_str()) {
            let score = (self.read_single)(song, lnmode);
            self.scorecache[cacheindex].insert(sha256.clone(), score);
        }
        self.scorecache[cacheindex]
            .get(sha256.as_str())
            .and_then(|s| s.as_ref())
    }

    /// Read score data for multiple songs
    pub fn read_score_datas(
        &mut self,
        collector: &dyn Fn(&SongData, Option<&ScoreData>),
        songs: &[SongData],
        lnmode: i32,
    ) {
        let mut noscore: Vec<SongData> = Vec::new();

        for song in songs {
            let cacheindex = Self::cache_index(song, lnmode);
            let sha256 = &song.file.sha256;
            if self.scorecache[cacheindex].contains_key(sha256.as_str()) {
                let score = self.scorecache[cacheindex]
                    .get(sha256.as_str())
                    .and_then(|s| s.as_ref());
                collector(song, score);
            } else {
                noscore.push(song.clone());
            }
        }

        if noscore.is_empty() {
            return;
        }

        // Use a temporary vec to collect results since we need Fn, not FnMut
        let cached_results = std::sync::Mutex::new(Vec::new());
        let lnmode_copy = lnmode;

        let combined_collector = |song: &SongData, score: Option<&ScoreData>| {
            let cacheindex = Self::cache_index(song, lnmode_copy);
            if let Ok(mut results) = cached_results.lock() {
                results.push((cacheindex, song.file.sha256.clone(), score.cloned()));
            }
            collector(song, score);
        };
        (self.read_multi)(&combined_collector, &noscore, lnmode);

        if let Ok(results) = cached_results.into_inner() {
            for (cacheindex, sha256, score) in results {
                self.scorecache[cacheindex].insert(sha256, score);
            }
        }
    }

    pub fn exists_score_data_cache(&self, song: &SongData, lnmode: i32) -> bool {
        let cacheindex = Self::cache_index(song, lnmode);
        self.scorecache[cacheindex].contains_key(song.file.sha256.as_str())
    }

    pub fn clear(&mut self) {
        for cache in &mut self.scorecache {
            cache.clear();
        }
    }

    pub fn update(&mut self, song: &SongData, lnmode: i32) {
        let cacheindex = Self::cache_index(song, lnmode);
        let score = (self.read_single)(song, lnmode);
        self.scorecache[cacheindex].insert(song.file.sha256.clone(), score);
    }
}
