use std::time::{SystemTime, UNIX_EPOCH};

use log::{trace, warn};

use beatoraja_core::score_data::ScoreData;

use crate::ir_score_data::IRScoreData;

/// IR ranking data
///
/// Translated from: RankingData.java
///
/// IR access state constants
pub const NONE: i32 = 0;
pub const ACCESS: i32 = 1;
pub const FINISH: i32 = 2;
pub const FAIL: i32 = 3;

#[derive(Clone, Debug)]
pub struct RankingData {
    /// Current IR rank for selected song
    irrank: i32,
    /// Previous IR rank for selected song
    prevrank: i32,
    /// Expected IR rank using local score
    localrank: i32,
    /// Total IR play count
    irtotal: i32,
    /// Clear lamp counts
    lamps: [i32; 11],
    /// All score data
    scores: Option<Vec<IRScoreData>>,
    /// Ranking for each score
    scorerankings: Option<Vec<i32>>,
    /// IR access state
    state: i32,
    /// Last update time
    last_update_time: i64,
}

impl Default for RankingData {
    fn default() -> Self {
        Self::new()
    }
}

impl RankingData {
    pub fn new() -> Self {
        Self {
            irrank: 0,
            prevrank: 0,
            localrank: 0,
            irtotal: 0,
            lamps: [0; 11],
            scores: None,
            scorerankings: None,
            state: NONE,
            last_update_time: 0,
        }
    }

    /// Load ranking data from IR.
    /// In Java, this spawns a thread. In Rust, we provide the async logic
    /// but the caller is responsible for spawning.
    ///
    /// Note: The full implementation requires MainState and IRStatus which are
    /// not yet available. This is a stub that should be called with pre-fetched data.
    pub fn load_stub(&mut self) {
        self.state = ACCESS;
        // In Java:
        // Thread irprocess = new Thread(() -> {
        //     final IRStatus[] ir = mainstate.main.getIRStatus();
        //     IRResponse<IRScoreData[]> response = null;
        //     if(song instanceof SongData) {
        //         response = ir[0].connection.getPlayData(null, new IRChartData((SongData) song));
        //     } else if(song instanceof CourseData) {
        //         response = ir[0].connection.getCoursePlayData(null, new IRCourseData((CourseData) song, mainstate.main.getPlayerConfig().getLnmode()));
        //     }
        //     if(response.isSucceeded()) {
        //         updateScore(response.getData(), mainstate.getScoreDataProperty().getScoreData());
        //         state = FINISH;
        //     } else {
        //         state = FAIL;
        //     }
        //     lastUpdateTime = System.currentTimeMillis();
        // });
        // irprocess.start();
        log::warn!(
            "not yet implemented: RankingData.load requires MainState and IRStatus dependencies"
        );
    }

    pub fn update_score(&mut self, scores: &[IRScoreData], localscore: Option<&ScoreData>) {
        let first_update = self.scores.is_none();

        let mut sorted_scores: Vec<IRScoreData> = scores.to_vec();
        sorted_scores.sort_by_key(|s| std::cmp::Reverse(s.get_exscore()));

        let mut scorerankings = vec![0i32; sorted_scores.len()];
        for i in 0..scorerankings.len() {
            scorerankings[i] =
                if i > 0 && sorted_scores[i].get_exscore() == sorted_scores[i - 1].get_exscore() {
                    scorerankings[i - 1]
                } else {
                    (i + 1) as i32
                };
        }

        if !first_update {
            self.prevrank = self.irrank;
        }
        self.scores = Some(sorted_scores.clone());
        self.scorerankings = Some(scorerankings.clone());
        self.irtotal = sorted_scores.len() as i32;
        self.lamps = [0; 11];
        self.irrank = 0;
        self.localrank = 0;
        for i in 0..sorted_scores.len() {
            if self.irrank == 0 && sorted_scores[i].player.is_empty() {
                self.irrank = scorerankings[i];
            }
            if let Some(ls) = localscore
                && self.localrank == 0
                && sorted_scores[i].get_exscore() <= ls.get_exscore()
            {
                self.localrank = scorerankings[i];
            }
            let clear_id = sorted_scores[i].clear.id() as usize;
            if clear_id < self.lamps.len() {
                self.lamps[clear_id] += 1;
            }
        }

        if first_update && self.localrank != 0 {
            self.prevrank = std::cmp::max(self.irrank, self.localrank);
        }

        self.state = FINISH;
        self.last_update_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
    }

    /// Get current IR rank for selected song
    pub fn get_rank(&self) -> i32 {
        self.irrank
    }

    /// Get previous IR rank for selected song
    pub fn get_previous_rank(&self) -> i32 {
        self.prevrank
    }

    /// Get expected IR rank using local score
    pub fn get_local_rank(&self) -> i32 {
        self.localrank
    }

    /// Get total player count on IR
    pub fn get_total_player(&self) -> i32 {
        self.irtotal
    }

    /// Get score data at index. Returns None if index is out of bounds.
    pub fn get_score(&self, index: i32) -> Option<&IRScoreData> {
        if let Some(ref scores) = self.scores
            && index >= 0
            && (index as usize) < scores.len()
        {
            return Some(&scores[index as usize]);
        }
        None
    }

    /// Get score ranking at index. Returns i32::MIN if index is out of bounds.
    pub fn get_score_ranking(&self, index: i32) -> i32 {
        if let Some(ref rankings) = self.scorerankings
            && index >= 0
            && (index as usize) < rankings.len()
        {
            return rankings[index as usize];
        }
        i32::MIN
    }

    pub fn get_clear_count(&self, clear_type: i32) -> i32 {
        if clear_type >= 0 && (clear_type as usize) < self.lamps.len() {
            self.lamps[clear_type as usize]
        } else {
            0
        }
    }

    pub fn get_state(&self) -> i32 {
        self.state
    }

    /// Get last update time in milliseconds
    pub fn get_last_update_time(&self) -> i64 {
        self.last_update_time
    }
}
