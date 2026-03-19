use bms_model::mode::Mode;

use crate::score_data::ScoreData;

/// Class for calculating various values from score data
pub struct ScoreDataProperty {
    pub score: Option<ScoreData>,
    pub rival: Option<ScoreData>,

    pub nowpoint: i32,
    pub nowscore: i32,
    pub bestscore: i32,
    pub bestscorerate: f32,
    pub nowbestscore: i32,
    pub nowbestscorerate: f32,

    pub nowrate: f32,
    pub nowrate_int: i32,
    pub nowrate_after_dot: i32,
    pub rate: f32,
    pub rate_int: i32,
    pub rate_after_dot: i32,
    pub bestrate_int: i32,
    pub bestrate_after_dot: i32,

    pub rivalscore: i32,
    pub rivalscorerate: f32,
    pub nowrivalscore: i32,
    pub nowrivalscorerate: f32,
    pub rivalrate_int: i32,
    pub rivalrate_after_dot: i32,
    pub rank: [bool; 27],
    pub nextrank: i32,
    pub nowrank: [bool; 27],
    pub bestrank: [bool; 27],

    pub previous_notes: i32,
    pub best_ghost: Option<Vec<i32>>,
    pub rival_ghost: Option<Vec<i32>>,
    pub use_best_ghost: bool,
    pub use_rival_ghost: bool,

    pub totalnotes: i32,
}

impl Default for ScoreDataProperty {
    fn default() -> Self {
        Self {
            score: None,
            rival: None,
            nowpoint: 0,
            nowscore: 0,
            bestscore: 0,
            bestscorerate: 0.0,
            nowbestscore: 0,
            nowbestscorerate: 0.0,
            nowrate: 0.0,
            nowrate_int: 0,
            nowrate_after_dot: 0,
            rate: 0.0,
            rate_int: 0,
            rate_after_dot: 0,
            bestrate_int: 0,
            bestrate_after_dot: 0,
            rivalscore: 0,
            rivalscorerate: 0.0,
            nowrivalscore: 0,
            nowrivalscorerate: 0.0,
            rivalrate_int: 0,
            rivalrate_after_dot: 0,
            rank: [false; 27],
            nextrank: 0,
            nowrank: [false; 27],
            bestrank: [false; 27],
            previous_notes: 0,
            best_ghost: None,
            rival_ghost: None,
            use_best_ghost: false,
            use_rival_ghost: false,
            totalnotes: 0,
        }
    }
}

impl ScoreDataProperty {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update_score(&mut self, score: Option<&ScoreData>) {
        let notes = score.map_or(0, |s| s.notes);
        self.update_score_with_notes(score, notes);
    }

    pub fn update_score_and_rival(&mut self, score: Option<&ScoreData>, rival: Option<&ScoreData>) {
        self.update_score(score);
        self.rival = rival.cloned();
        let exscore = rival.map_or(0, |r| r.exscore());
        let totalnotes = rival.map_or(0, |r| r.notes);

        self.rivalscore = exscore;
        self.rivalscorerate = if totalnotes == 0 {
            1.0f32
        } else {
            exscore as f32 / (totalnotes as f32 * 2.0)
        };
        self.rivalrate_int = (self.rivalscorerate * 100.0) as i32;
        self.rivalrate_after_dot = ((self.rivalscorerate * 10000.0) as i32) % 100;
    }

    pub fn update_score_with_notes(&mut self, score: Option<&ScoreData>, notes: i32) {
        self.score = score.cloned();
        let exscore = score.map_or(0, |s| s.exscore());
        let totalnotes = score.map_or(0, |s| s.notes);
        if totalnotes > 0 {
            let score = score.expect("score");
            match score.playmode {
                Mode::BEAT_5K | Mode::BEAT_10K => {
                    let raw = (100000i64 * score.judge_count_total(0) as i64
                        + 100000i64 * score.judge_count_total(1) as i64
                        + 50000i64 * score.judge_count_total(2) as i64)
                        / totalnotes as i64;
                    self.nowpoint = raw.clamp(0, i32::MAX as i64) as i32;
                }
                Mode::BEAT_7K | Mode::BEAT_14K => {
                    let term1 = (150000i64 * score.judge_count_total(0) as i64
                        + 100000i64 * score.judge_count_total(1) as i64
                        + 20000i64 * score.judge_count_total(2) as i64)
                        / totalnotes as i64;
                    let term2 = 50000i64 * score.maxcombo as i64 / totalnotes as i64;
                    self.nowpoint = (term1 + term2).clamp(0, i32::MAX as i64) as i32;
                }
                Mode::POPN_5K | Mode::POPN_9K => {
                    let raw = (100000i64 * score.judge_count_total(0) as i64
                        + 70000i64 * score.judge_count_total(1) as i64
                        + 40000i64 * score.judge_count_total(2) as i64)
                        / totalnotes as i64;
                    self.nowpoint = raw.clamp(0, i32::MAX as i64) as i32;
                }
                _ => {
                    let raw = (1000000i64 * score.judge_count_total(0) as i64
                        + 700000i64 * score.judge_count_total(1) as i64
                        + 400000i64 * score.judge_count_total(2) as i64)
                        / totalnotes as i64;
                    self.nowpoint = raw.clamp(0, i32::MAX as i64) as i32;
                }
            }
        } else {
            self.nowpoint = 0;
        }
        self.nowscore = exscore;
        self.rate = if totalnotes == 0 {
            1.0f32
        } else {
            exscore as f32 / (totalnotes as f32 * 2.0)
        };
        self.rate_int = (self.rate * 100.0) as i32;
        self.rate_after_dot = ((self.rate * 10000.0) as i32) % 100;
        self.nowrate = if notes == 0 {
            1.0f32
        } else {
            exscore as f32 / (notes as f32 * 2.0)
        };
        self.nowrate_int = (self.nowrate * 100.0) as i32;
        self.nowrate_after_dot = ((self.nowrate * 10000.0) as i32) % 100;
        self.nextrank = i32::MIN;
        let rank_len = self.rank.len();
        for (i, rank) in self.rank.iter_mut().enumerate() {
            *rank = totalnotes != 0 && self.rate >= 1f32 * i as f32 / rank_len as f32;
            if i % 3 == 0 && !*rank && self.nextrank == i32::MIN {
                self.nextrank = (((i as f64) * (notes as f64 * 2.0) / (rank_len as f64))
                    - (self.rate as f64) * (notes as f64 * 2.0))
                    .ceil() as i32;
            }
        }
        if self.nextrank == i32::MIN {
            self.nextrank = (notes.saturating_mul(2)) - exscore;
        }
        let nowrank_len = self.nowrank.len();
        for (i, nowrank) in self.nowrank.iter_mut().enumerate() {
            *nowrank = totalnotes != 0 && self.nowrate >= 1f32 * i as f32 / nowrank_len as f32;
        }

        if self.use_best_ghost {
            if let Some(ref ghost) = self.best_ghost {
                let end = notes.min(ghost.len() as i32);
                for i in self.previous_notes..end {
                    self.nowbestscore += Self::get_ex_score(ghost[i as usize]);
                }
            }
            self.nowbestscorerate = if totalnotes == 0 {
                0.0
            } else {
                self.nowbestscore as f32 / (totalnotes as f32 * 2.0)
            };
        } else {
            self.nowbestscore = if totalnotes == 0 {
                0
            } else {
                (self.bestscore as i64 * notes as i64 / totalnotes as i64) as i32
            };
            self.nowbestscorerate = if totalnotes == 0 {
                0.0
            } else {
                (self.bestscore as f32) * notes as f32
                    / (totalnotes as f32 * totalnotes as f32 * 2.0)
            };
        }
        if self.use_rival_ghost {
            if let Some(ref ghost) = self.rival_ghost {
                let end = notes.min(ghost.len() as i32);
                for i in self.previous_notes..end {
                    self.nowrivalscore += Self::get_ex_score(ghost[i as usize]);
                }
            }
            self.nowrivalscorerate = if totalnotes == 0 {
                0.0
            } else {
                self.nowrivalscore as f32 / (totalnotes as f32 * 2.0)
            };
        } else {
            self.nowrivalscore = if totalnotes == 0 {
                0
            } else {
                (self.rivalscore as i64 * notes as i64 / totalnotes as i64) as i32
            };
            self.nowrivalscorerate = if totalnotes == 0 {
                0.0
            } else {
                (self.rivalscore as f32) * notes as f32
                    / (totalnotes as f32 * totalnotes as f32 * 2.0)
            };
        }
        self.previous_notes = notes;
    }

    fn get_ex_score(judge: i32) -> i32 {
        if judge == 0 {
            2
        } else if judge == 1 {
            1
        } else {
            0
        }
    }

    pub fn update_target_score(&mut self, rivalscore: i32) {
        self.rivalscore = rivalscore;
        if self.totalnotes == 0 {
            self.rivalscorerate = 0.0;
            self.rivalrate_int = 0;
            self.rivalrate_after_dot = 0;
        } else {
            self.rivalscorerate = rivalscore as f32 / (self.totalnotes as f32 * 2.0);
            self.rivalrate_int = (self.rivalscorerate * 100.0) as i32;
            self.rivalrate_after_dot = ((self.rivalscorerate * 10000.0) as i32) % 100;
        }
    }

    pub fn set_target_score(&mut self, bestscore: i32, rivalscore: i32, totalnotes: i32) {
        self.set_target_score_with_ghost(bestscore, None, rivalscore, None, totalnotes);
    }

    pub fn set_target_score_with_ghost(
        &mut self,
        bestscore: i32,
        best_ghost: Option<Vec<i32>>,
        rivalscore: i32,
        rival_ghost: Option<Vec<i32>>,
        totalnotes: i32,
    ) {
        // Reset accumulated ghost state to prevent stale values carrying
        // across practice retries or song changes.
        self.nowbestscore = 0;
        self.nowbestscorerate = 0.0;
        self.nowrivalscore = 0;
        self.nowrivalscorerate = 0.0;
        self.previous_notes = 0;

        self.bestscore = bestscore;
        self.best_ghost = best_ghost;
        self.rivalscore = rivalscore;
        self.rival_ghost = rival_ghost;
        self.totalnotes = totalnotes;
        if totalnotes == 0 {
            self.bestscorerate = 0.0;
            self.bestrate_int = 0;
            self.bestrate_after_dot = 0;
            self.rivalscorerate = 0.0;
            for bestrank in self.bestrank.iter_mut() {
                *bestrank = false;
            }
            self.rivalrate_int = 0;
            self.rivalrate_after_dot = 0;
        } else {
            self.bestscorerate = bestscore as f32 / (totalnotes as f32 * 2.0);
            self.bestrate_int = (self.bestscorerate * 100.0) as i32;
            self.bestrate_after_dot = ((self.bestscorerate * 10000.0) as i32) % 100;
            self.rivalscorerate = rivalscore as f32 / (totalnotes as f32 * 2.0);
            let bestrank_len = self.bestrank.len();
            for (i, bestrank) in self.bestrank.iter_mut().enumerate() {
                *bestrank = self.bestscorerate >= 1f32 * i as f32 / bestrank_len as f32;
            }
            self.rivalrate_int = (self.rivalscorerate * 100.0) as i32;
            self.rivalrate_after_dot = ((self.rivalscorerate * 10000.0) as i32) % 100;
        }

        // If ghost and notes count differ (notes changed due to random branching), don't use ghost
        self.use_best_ghost = self
            .best_ghost
            .as_ref()
            .is_some_and(|g| g.len() == totalnotes as usize);
        self.use_rival_ghost = self
            .rival_ghost
            .as_ref()
            .is_some_and(|g| g.len() == totalnotes as usize);
    }

    pub fn now_score(&self) -> i32 {
        self.nowpoint
    }

    pub fn now_ex_score(&self) -> i32 {
        self.nowscore
    }

    pub fn now_best_score(&self) -> i32 {
        self.nowbestscore
    }

    pub fn now_rival_score(&self) -> i32 {
        self.nowrivalscore
    }

    pub fn qualify_rank(&self, index: usize) -> bool {
        self.rank.get(index).copied().unwrap_or(false)
    }

    pub fn qualify_now_rank(&self, index: usize) -> bool {
        self.nowrank.get(index).copied().unwrap_or(false)
    }

    pub fn qualify_best_rank(&self, index: usize) -> bool {
        self.bestrank.get(index).copied().unwrap_or(false)
    }

    pub fn now_rate(&self) -> f32 {
        self.nowrate
    }

    pub fn now_rate_int(&self) -> i32 {
        self.nowrate_int
    }

    pub fn now_rate_after_dot(&self) -> i32 {
        self.nowrate_after_dot
    }

    pub fn rival_rate_int(&self) -> i32 {
        self.rivalrate_int
    }

    pub fn rival_rate_after_dot(&self) -> i32 {
        self.rivalrate_after_dot
    }

    pub fn rate(&self) -> f32 {
        self.rate
    }

    pub fn next_rank(&self) -> i32 {
        self.nextrank
    }

    pub fn rate_int(&self) -> i32 {
        self.rate_int
    }

    pub fn rate_after_dot(&self) -> i32 {
        self.rate_after_dot
    }

    pub fn best_score(&self) -> i32 {
        self.bestscore
    }

    pub fn best_score_rate(&self) -> f32 {
        self.bestscorerate
    }

    pub fn best_rate_int(&self) -> i32 {
        self.bestrate_int
    }

    pub fn best_rate_after_dot(&self) -> i32 {
        self.bestrate_after_dot
    }

    pub fn now_best_score_rate(&self) -> f32 {
        self.nowbestscorerate
    }

    pub fn rival_score(&self) -> i32 {
        self.rivalscore
    }

    pub fn rival_score_rate(&self) -> f32 {
        self.rivalscorerate
    }

    pub fn now_rival_score_rate(&self) -> f32 {
        self.nowrivalscorerate
    }

    pub fn score_data(&self) -> Option<&ScoreData> {
        self.score.as_ref()
    }

    pub fn rival_score_data(&self) -> Option<&ScoreData> {
        self.rival.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qualify_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        // Array is [bool; 27], so index 27 is out of bounds.
        assert!(!prop.qualify_rank(27));
        assert!(!prop.qualify_rank(100));
        assert!(!prop.qualify_rank(usize::MAX));
    }

    #[test]
    fn qualify_now_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        assert!(!prop.qualify_now_rank(27));
        assert!(!prop.qualify_now_rank(100));
    }

    #[test]
    fn qualify_best_rank_out_of_bounds_returns_false() {
        let prop = ScoreDataProperty::default();
        assert!(!prop.qualify_best_rank(27));
        assert!(!prop.qualify_best_rank(100));
    }

    #[test]
    fn qualify_rank_valid_index_returns_value() {
        let mut prop = ScoreDataProperty::default();
        prop.rank[18] = true;
        assert!(prop.qualify_rank(18));
        assert!(!prop.qualify_rank(17));
    }

    /// set_target_score_with_ghost must reset accumulated ghost state
    /// so that practice retries do not carry stale nowbestscore /
    /// nowrivalscore / previous_notes from the prior attempt.
    #[test]
    fn set_target_score_with_ghost_resets_stale_state() {
        let mut prop = ScoreDataProperty::default();

        // Simulate a first play: set ghost and accumulate scores
        let ghost = vec![0, 0, 1, 1, 2]; // PG, PG, GR, GR, GD
        prop.set_target_score_with_ghost(8, Some(ghost.clone()), 6, Some(ghost.clone()), 5);

        // Simulate updating scores during play to accumulate state
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 3;
        sd.notes = 5;
        prop.update_score_with_notes(Some(&sd), 3);

        // After partial play, these should be non-zero
        assert!(prop.nowbestscore > 0 || prop.nowrivalscore > 0 || prop.previous_notes > 0);

        // Now simulate a practice retry: call set_target_score_with_ghost again
        prop.set_target_score_with_ghost(8, Some(ghost.clone()), 6, Some(ghost), 5);

        // Accumulated state must be reset
        assert_eq!(
            prop.nowbestscore, 0,
            "nowbestscore should be reset on retry"
        );
        assert_eq!(
            prop.nowrivalscore, 0,
            "nowrivalscore should be reset on retry"
        );
        assert_eq!(
            prop.previous_notes, 0,
            "previous_notes should be reset on retry"
        );
    }

    #[test]
    fn update_score_zero_notes_sets_rate_one() {
        let mut prop = ScoreDataProperty::default();
        let sd = ScoreData::new(Mode::BEAT_7K);
        // sd.notes = 0 by default
        prop.update_score(Some(&sd));
        // When totalnotes == 0, rate should be 1.0
        assert_eq!(prop.rate, 1.0);
        assert_eq!(prop.nowpoint, 0);
    }

    #[test]
    fn update_score_none_leaves_defaults() {
        let mut prop = ScoreDataProperty::default();
        prop.update_score(None);
        assert_eq!(prop.nowpoint, 0);
        assert_eq!(prop.nowscore, 0);
        assert_eq!(prop.rate, 1.0);
    }

    // -- Score calculation for different modes --

    #[test]
    fn update_score_beat_5k_mode_calculation() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_5K);
        sd.judge_counts.epg = 50; // PG count
        sd.judge_counts.lpg = 50;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // BEAT_5K: (100000 * PG_total + 100000 * GR_total + 50000 * GD_total) / notes
        // = (100000 * 100 + 0 + 0) / 100 = 100000
        assert_eq!(prop.nowpoint, 100000);
    }

    #[test]
    fn update_score_beat_7k_mode_includes_combo() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 100;
        sd.maxcombo = 100;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // BEAT_7K: term1 = (150000 * 100) / 100 = 150000
        //          term2 = 50000 * 100 / 100 = 50000
        //          total = 200000
        assert_eq!(prop.nowpoint, 200000);
    }

    #[test]
    fn update_score_popn_mode_calculation() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::POPN_9K);
        sd.judge_counts.epg = 100;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // POPN: (100000 * 100 + 0 + 0) / 100 = 100000
        assert_eq!(prop.nowpoint, 100000);
    }

    // -- Rate calculations --

    #[test]
    fn rate_calculation_half_perfect() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 50;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // exscore = 50 * 2 = 100, totalnotes = 100, rate = 100 / 200 = 0.5
        assert!((prop.rate - 0.5).abs() < 0.001);
        assert_eq!(prop.rate_int, 50);
        assert_eq!(prop.rate_after_dot, 0);
    }

    #[test]
    fn rate_calculation_full_perfect() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 100;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        assert!((prop.rate - 1.0).abs() < 0.001);
        assert_eq!(prop.rate_int, 100);
    }

    // -- Rival score --

    #[test]
    fn update_score_and_rival_sets_rival_fields() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 50;
        sd.notes = 100;

        let mut rival = ScoreData::new(Mode::BEAT_7K);
        rival.judge_counts.epg = 80;
        rival.notes = 100;

        prop.update_score_and_rival(Some(&sd), Some(&rival));
        // rival exscore = 80 * 2 = 160
        assert_eq!(prop.rivalscore, 160);
        assert!((prop.rivalscorerate - 0.8).abs() < 0.001);
    }

    #[test]
    fn update_score_and_rival_none_rival() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 50;
        sd.notes = 100;

        prop.update_score_and_rival(Some(&sd), None);
        assert_eq!(prop.rivalscore, 0);
        assert_eq!(prop.rivalscorerate, 1.0); // zero totalnotes for rival
    }

    // -- Target score --

    #[test]
    fn set_target_score_zero_notes() {
        let mut prop = ScoreDataProperty::default();
        prop.set_target_score(100, 50, 0);
        assert_eq!(prop.bestscorerate, 0.0);
        assert_eq!(prop.rivalscorerate, 0.0);
        assert!(prop.bestrank.iter().all(|&b| !b));
    }

    #[test]
    fn set_target_score_calculates_rates() {
        let mut prop = ScoreDataProperty::default();
        prop.set_target_score(100, 50, 100);
        // bestscorerate = 100 / 200 = 0.5
        assert!((prop.bestscorerate - 0.5).abs() < 0.001);
        assert_eq!(prop.bestrate_int, 50);
        // rivalscorerate = 50 / 200 = 0.25
        assert!((prop.rivalscorerate - 0.25).abs() < 0.001);
    }

    // -- Ghost tracking --

    #[test]
    fn set_target_score_with_ghost_enables_ghost_tracking() {
        let mut prop = ScoreDataProperty::default();
        let ghost = vec![0, 0, 1, 1, 2]; // 5 notes
        prop.set_target_score_with_ghost(10, Some(ghost), 5, None, 5);
        assert!(prop.use_best_ghost);
        assert!(!prop.use_rival_ghost); // no rival ghost
    }

    #[test]
    fn set_target_score_with_ghost_mismatched_length_disables_ghost() {
        let mut prop = ScoreDataProperty::default();
        let ghost = vec![0, 0, 1]; // 3 entries but 5 total notes
        prop.set_target_score_with_ghost(10, Some(ghost), 5, None, 5);
        assert!(!prop.use_best_ghost); // length mismatch
    }

    #[test]
    fn update_target_score_zero_totalnotes() {
        let mut prop = ScoreDataProperty::default();
        prop.totalnotes = 0;
        prop.update_target_score(100);
        assert_eq!(prop.rivalscore, 100);
        assert_eq!(prop.rivalscorerate, 0.0);
        assert_eq!(prop.rivalrate_int, 0);
        assert_eq!(prop.rivalrate_after_dot, 0);
    }

    #[test]
    fn update_target_score_with_totalnotes() {
        let mut prop = ScoreDataProperty::default();
        prop.totalnotes = 100;
        prop.update_target_score(100);
        assert_eq!(prop.rivalscore, 100);
        // 100 / 200 = 0.5
        assert!((prop.rivalscorerate - 0.5).abs() < 0.001);
        assert_eq!(prop.rivalrate_int, 50);
        assert_eq!(prop.rivalrate_after_dot, 0);
    }

    // -- Rank array boundary --

    #[test]
    fn rank_array_filled_correctly_for_max_score() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 100;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // All ranks should be true (rate = 1.0)
        assert!(prop.rank.iter().all(|&r| r));
    }

    #[test]
    fn rank_array_all_false_for_zero_totalnotes() {
        let mut prop = ScoreDataProperty::default();
        let sd = ScoreData::new(Mode::BEAT_7K);
        // notes = 0
        prop.update_score(Some(&sd));
        assert!(prop.rank.iter().all(|&r| !r));
    }

    // -- Nextrank calculation --

    #[test]
    fn nextrank_for_max_score_is_zero_gap() {
        let mut prop = ScoreDataProperty::default();
        let mut sd = ScoreData::new(Mode::BEAT_7K);
        sd.judge_counts.epg = 100;
        sd.notes = 100;
        prop.update_score(Some(&sd));
        // At max, nextrank = (notes * 2) - exscore = 200 - 200 = 0
        assert_eq!(prop.nextrank, 0);
    }
}
