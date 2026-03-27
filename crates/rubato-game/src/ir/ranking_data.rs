use std::time::{SystemTime, UNIX_EPOCH};

use log::warn;

use crate::core::score_data::ScoreData;

use crate::ir::ir_chart_data::IRChartData;
use crate::ir::ir_connection::IRConnection;
use crate::ir::ir_course_data::IRCourseData;
use crate::ir::ir_score_data::IRScoreData;

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

    /// Load ranking data for a song from IR.
    ///
    /// Translated from: RankingData.load() (song path)
    pub fn load_song(
        &mut self,
        connection: &dyn IRConnection,
        chart: &IRChartData,
        local_score: Option<&ScoreData>,
    ) {
        self.state = ACCESS;
        let response = connection.get_play_data(None, Some(chart));
        if response.is_succeeded() {
            if let Some(data) = response.data {
                self.update_score(&data, local_score);
            } else {
                self.state = FAIL;
                self.stamp_update_time();
            }
        } else {
            warn!("IR ranking data load failed: {}", response.message);
            self.state = FAIL;
            self.stamp_update_time();
        }
    }

    /// Load ranking data for a course from IR.
    ///
    /// Translated from: RankingData.load() (course path)
    pub fn load_course(
        &mut self,
        connection: &dyn IRConnection,
        course: &IRCourseData,
        local_score: Option<&ScoreData>,
    ) {
        self.state = ACCESS;
        let response = connection.get_course_play_data(None, course);
        if response.is_succeeded() {
            if let Some(data) = response.data {
                self.update_score(&data, local_score);
            } else {
                self.state = FAIL;
                self.stamp_update_time();
            }
        } else {
            warn!("IR course ranking data load failed: {}", response.message);
            self.state = FAIL;
            self.stamp_update_time();
        }
    }

    pub fn update_score(&mut self, scores: &[IRScoreData], localscore: Option<&ScoreData>) {
        self.update_score_with_player(scores, localscore, None);
    }

    /// Update ranking data from IR score data.
    /// `player_id` identifies the logged-in player; when provided, rows whose
    /// player name matches this ID are also recognized as "your" score
    /// (in addition to the legacy empty-player-name convention).
    pub fn update_score_with_player(
        &mut self,
        scores: &[IRScoreData],
        localscore: Option<&ScoreData>,
        player_id: Option<&str>,
    ) {
        let first_update = self.scores.is_none();

        let mut sorted_scores: Vec<IRScoreData> = scores.to_vec();
        sorted_scores.sort_by_key(|s| std::cmp::Reverse(s.exscore()));

        let mut scorerankings = Vec::with_capacity(sorted_scores.len());
        for (i, score) in sorted_scores.iter().enumerate() {
            let ranking = if i > 0 && score.exscore() == sorted_scores[i - 1].exscore() {
                scorerankings[i - 1]
            } else {
                (i + 1) as i32
            };
            scorerankings.push(ranking);
        }

        if !first_update {
            self.prevrank = self.irrank;
        }
        self.irtotal = sorted_scores.len() as i32;
        self.lamps = [0; 11];
        self.irrank = 0;
        self.localrank = 0;
        for (score, &ranking) in sorted_scores.iter().zip(scorerankings.iter()) {
            let is_own_score = score.player.is_empty()
                || player_id.is_some_and(|pid| !pid.is_empty() && score.player == pid);
            if self.irrank == 0 && is_own_score {
                self.irrank = ranking;
            }
            if let Some(ls) = localscore
                && self.localrank == 0
                && score.exscore() <= ls.exscore()
            {
                self.localrank = ranking;
            }
            let clear_id = score.clear.id() as usize;
            if clear_id < self.lamps.len() {
                self.lamps[clear_id] += 1;
            }
        }

        self.scores = Some(sorted_scores);
        self.scorerankings = Some(scorerankings);

        if first_update && self.localrank != 0 {
            self.prevrank = std::cmp::max(self.irrank, self.localrank);
        }

        self.state = FINISH;
        self.stamp_update_time();
    }

    /// Record the current time as the last update time.
    /// Called on both success and failure to prevent rapid retries.
    fn stamp_update_time(&mut self) {
        self.last_update_time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as i64;
    }

    /// Get current IR rank for selected song
    pub fn rank(&self) -> i32 {
        self.irrank
    }

    /// Get previous IR rank for selected song
    pub fn previous_rank(&self) -> i32 {
        self.prevrank
    }

    /// Get expected IR rank using local score
    pub fn local_rank(&self) -> i32 {
        self.localrank
    }

    /// Get total player count on IR
    pub fn total_player(&self) -> i32 {
        self.irtotal
    }

    /// Get score data at index. Returns None if index is out of bounds.
    pub fn score(&self, index: i32) -> Option<&IRScoreData> {
        if let Some(ref scores) = self.scores
            && index >= 0
            && (index as usize) < scores.len()
        {
            return Some(&scores[index as usize]);
        }
        None
    }

    /// Get score ranking at index. Returns i32::MIN if index is out of bounds.
    pub fn score_ranking(&self, index: i32) -> i32 {
        if let Some(ref rankings) = self.scorerankings
            && index >= 0
            && (index as usize) < rankings.len()
        {
            return rankings[index as usize];
        }
        i32::MIN
    }

    pub fn clear_count(&self, clear_type: i32) -> i32 {
        if clear_type >= 0 && (clear_type as usize) < self.lamps.len() {
            self.lamps[clear_type as usize]
        } else {
            0
        }
    }

    pub fn state(&self) -> i32 {
        self.state
    }

    /// Get last update time in milliseconds
    pub fn last_update_time(&self) -> i64 {
        self.last_update_time
    }
}

#[cfg(test)]
#[allow(clippy::field_reassign_with_default)]
mod tests {
    use super::*;
    use crate::core::clear_type::ClearType;

    fn make_ir_score(
        player: &str,
        epg: i32,
        lpg: i32,
        egr: i32,
        lgr: i32,
        clear: ClearType,
    ) -> IRScoreData {
        let mut s = crate::core::score_data::ScoreData::default();
        s.player = player.to_string();
        s.judge_counts.epg = epg;
        s.judge_counts.lpg = lpg;
        s.judge_counts.egr = egr;
        s.judge_counts.lgr = lgr;
        s.clear = clear.id();
        let mut ir = IRScoreData::new(&s);
        ir.player = player.to_string();
        ir
    }

    #[test]
    fn test_ranking_data_initial_state() {
        let rd = RankingData::new();
        assert_eq!(rd.rank(), 0);
        assert_eq!(rd.previous_rank(), 0);
        assert_eq!(rd.local_rank(), 0);
        assert_eq!(rd.total_player(), 0);
        assert_eq!(rd.state(), NONE);
        assert!(rd.score(0).is_none());
    }

    #[test]
    fn test_update_score_sorts_by_exscore_descending() {
        let mut rd = RankingData::new();
        let scores = vec![
            make_ir_score("low", 10, 10, 5, 5, ClearType::Normal), // exscore = 50
            make_ir_score("high", 50, 50, 20, 20, ClearType::Hard), // exscore = 240
            make_ir_score("mid", 30, 30, 10, 10, ClearType::Easy), // exscore = 140
        ];
        rd.update_score(&scores, None);

        assert_eq!(rd.total_player(), 3);
        assert_eq!(rd.score(0).unwrap().exscore(), 240);
        assert_eq!(rd.score(1).unwrap().exscore(), 140);
        assert_eq!(rd.score(2).unwrap().exscore(), 50);
    }

    #[test]
    fn test_update_score_rankings_with_ties() {
        let mut rd = RankingData::new();
        let scores = vec![
            make_ir_score("a", 50, 50, 20, 20, ClearType::Normal), // exscore = 240
            make_ir_score("b", 50, 50, 20, 20, ClearType::Normal), // exscore = 240 (tie)
            make_ir_score("c", 10, 10, 5, 5, ClearType::Normal),   // exscore = 50
        ];
        rd.update_score(&scores, None);

        // Both tied scores should share rank 1
        assert_eq!(rd.score_ranking(0), 1);
        assert_eq!(rd.score_ranking(1), 1);
        // Third place is rank 3 (not 2)
        assert_eq!(rd.score_ranking(2), 3);
    }

    #[test]
    fn test_update_score_irrank_from_empty_player() {
        let mut rd = RankingData::new();
        // Empty player name indicates "own score" in IR
        let scores = vec![
            make_ir_score("other", 50, 50, 20, 20, ClearType::Hard), // rank 1
            make_ir_score("", 30, 30, 10, 10, ClearType::Normal),    // rank 2, own score
        ];
        rd.update_score(&scores, None);

        assert_eq!(rd.rank(), 2); // own IR rank
    }

    #[test]
    fn test_update_score_clear_count() {
        let mut rd = RankingData::new();
        let scores = vec![
            make_ir_score("a", 50, 50, 20, 20, ClearType::Normal), // clear id = 5
            make_ir_score("b", 30, 30, 10, 10, ClearType::Normal), // clear id = 5
            make_ir_score("c", 10, 10, 5, 5, ClearType::Hard),     // clear id = 6
        ];
        rd.update_score(&scores, None);

        assert_eq!(rd.clear_count(5), 2); // Normal
        assert_eq!(rd.clear_count(6), 1); // Hard
        assert_eq!(rd.clear_count(0), 0); // NoPlay
    }

    #[test]
    fn test_get_score_out_of_bounds() {
        let rd = RankingData::new();
        assert!(rd.score(-1).is_none());
        assert!(rd.score(0).is_none());
        assert!(rd.score(100).is_none());
    }

    #[test]
    fn test_get_score_ranking_out_of_bounds() {
        let rd = RankingData::new();
        assert_eq!(rd.score_ranking(-1), i32::MIN);
        assert_eq!(rd.score_ranking(0), i32::MIN);
    }

    #[test]
    fn test_get_clear_count_out_of_bounds() {
        let rd = RankingData::new();
        assert_eq!(rd.clear_count(-1), 0);
        assert_eq!(rd.clear_count(99), 0);
    }

    #[test]
    fn test_update_score_sets_state_finish() {
        let mut rd = RankingData::new();
        rd.update_score(&[], None);
        assert_eq!(rd.state(), FINISH);
        assert!(rd.last_update_time() > 0);
    }

    #[test]
    fn test_update_score_local_rank() {
        let mut rd = RankingData::new();
        let scores = vec![
            make_ir_score("a", 50, 50, 20, 20, ClearType::Hard), // exscore = 240
            make_ir_score("b", 30, 30, 10, 10, ClearType::Normal), // exscore = 140
            make_ir_score("c", 10, 10, 5, 5, ClearType::Easy),   // exscore = 50
        ];

        // Local score with exscore 150 should be ranked 2 (between 240 and 140)
        let mut local = crate::core::score_data::ScoreData::default();
        local.judge_counts.epg = 35;
        local.judge_counts.lpg = 35;
        local.judge_counts.egr = 10;
        local.judge_counts.lgr = 10;
        // local exscore = (35+35)*2 + 10 + 10 = 160

        rd.update_score(&scores, Some(&local));
        assert_eq!(rd.local_rank(), 2);
    }
}
