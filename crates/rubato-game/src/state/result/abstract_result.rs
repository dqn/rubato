// AbstractResult.java -> abstract_result.rs
// Mechanical line-by-line translation.

use crate::core::clear_type::ClearType;
use crate::core::score_data::ScoreData;
use crate::core::score_data_property::ScoreDataProperty;

use super::RankingData;

/// Replay data status
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayStatus {
    Exist,
    NotExist,
    Saved,
}

/// Replay auto save constraint
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayAutoSaveConstraint {
    Nothing,
    ScoreUpdate,
    ScoreUpdateOrEqual,
    MisscountUpdate,
    MisscountUpdateOrEqual,
    MaxcomboUpdate,
    MaxcomboUpdateOrEqual,
    ClearUpdate,
    ClearUpdateOrEqual,
    AnyoneUpdate,
    Always,
}

impl ReplayAutoSaveConstraint {
    pub fn is_qualified(&self, oldscore: &ScoreData, newscore: &ScoreData) -> bool {
        match self {
            ReplayAutoSaveConstraint::Nothing => false,
            ReplayAutoSaveConstraint::ScoreUpdate => newscore.exscore() > oldscore.exscore(),
            ReplayAutoSaveConstraint::ScoreUpdateOrEqual => {
                newscore.exscore() >= oldscore.exscore()
            }
            ReplayAutoSaveConstraint::MisscountUpdate => {
                newscore.minbp < oldscore.minbp || oldscore.clear == ClearType::NoPlay.id()
            }
            ReplayAutoSaveConstraint::MisscountUpdateOrEqual => {
                newscore.minbp <= oldscore.minbp || oldscore.clear == ClearType::NoPlay.id()
            }
            ReplayAutoSaveConstraint::MaxcomboUpdate => newscore.maxcombo > oldscore.maxcombo,
            ReplayAutoSaveConstraint::MaxcomboUpdateOrEqual => {
                newscore.maxcombo >= oldscore.maxcombo
            }
            ReplayAutoSaveConstraint::ClearUpdate => newscore.clear > oldscore.clear,
            ReplayAutoSaveConstraint::ClearUpdateOrEqual => newscore.clear >= oldscore.clear,
            ReplayAutoSaveConstraint::AnyoneUpdate => {
                newscore.clear > oldscore.clear
                    || newscore.maxcombo > oldscore.maxcombo
                    || newscore.minbp < oldscore.minbp
                    || newscore.exscore() > oldscore.exscore()
            }
            ReplayAutoSaveConstraint::Always => true,
        }
    }

    pub fn get(index: i32) -> ReplayAutoSaveConstraint {
        let values = Self::values();
        if index < 0 || index as usize >= values.len() {
            return ReplayAutoSaveConstraint::Nothing;
        }
        values[index as usize]
    }

    pub fn values() -> &'static [ReplayAutoSaveConstraint] {
        &[
            ReplayAutoSaveConstraint::Nothing,
            ReplayAutoSaveConstraint::ScoreUpdate,
            ReplayAutoSaveConstraint::ScoreUpdateOrEqual,
            ReplayAutoSaveConstraint::MisscountUpdate,
            ReplayAutoSaveConstraint::MisscountUpdateOrEqual,
            ReplayAutoSaveConstraint::MaxcomboUpdate,
            ReplayAutoSaveConstraint::MaxcomboUpdateOrEqual,
            ReplayAutoSaveConstraint::ClearUpdate,
            ReplayAutoSaveConstraint::ClearUpdateOrEqual,
            ReplayAutoSaveConstraint::AnyoneUpdate,
            ReplayAutoSaveConstraint::Always,
        ]
    }
}

/// Timing distribution
pub struct TimingDistribution {
    array_center: i32,
    dist: Vec<i32>,
    average: f32,
    std_dev: f32,
}

impl TimingDistribution {
    pub fn new(range: i32) -> Self {
        Self {
            array_center: range,
            dist: vec![0; (range * 2 + 1) as usize],
            average: f32::MAX,
            std_dev: -1.0,
        }
    }

    pub fn statistic_value_calculate(&mut self) {
        let mut count: i64 = 0;
        let mut sum: i64 = 0;
        let mut sumf: f32 = 0.0;

        for (i, &d) in self.dist.iter().enumerate() {
            count += d as i64;
            sum += d as i64 * (i as i64 - self.array_center as i64);
        }

        if count == 0 {
            return;
        }

        self.average = sum as f32 / count as f32;

        for (i, &d) in self.dist.iter().enumerate() {
            sumf += d as f32
                * (i as i32 as f32 - self.array_center as f32 - self.average)
                * (i as i32 as f32 - self.array_center as f32 - self.average);
        }

        self.std_dev = (sumf / count as f32).sqrt();
    }

    pub fn init(&mut self) {
        self.dist.fill(0);
        self.average = f32::MAX;
        self.std_dev = -1.0;
    }

    pub fn add(&mut self, timing: i32) {
        if -self.array_center <= timing && timing <= self.array_center {
            self.dist[(timing + self.array_center) as usize] += 1;
        }
    }

    pub fn timing_distribution(&self) -> &[i32] {
        &self.dist
    }

    pub fn average(&self) -> f32 {
        self.average
    }

    pub fn std_dev(&self) -> f32 {
        self.std_dev
    }

    pub fn array_center(&self) -> i32 {
        self.array_center
    }
}

pub const STATE_OFFLINE: i32 = 0;
pub const STATE_IR_PROCESSING: i32 = 1;
pub const STATE_IR_FINISHED: i32 = 2;

pub const REPLAY_SIZE: usize = 4;

/// Shared data for AbstractResult (Java abstract class fields)
pub struct AbstractResultData {
    /// State
    pub state: i32,
    /// Ranking data
    pub ranking: Option<RankingData>,
    /// Ranking display offset
    pub ranking_offset: i32,
    /// Average duration of all notes (us)
    pub avgduration: i64,
    pub avg: i64,
    pub stddev: i64,
    /// Timing distribution
    pub timing_distribution: TimingDistribution,
    /// Timing distribution range
    pub dist_range: i32,
    /// Replay data status for each replay slot
    pub save_replay: [ReplayStatus; REPLAY_SIZE],
    /// Gauge type
    pub gauge_type: i32,
    /// Old score data
    pub oldscore: ScoreData,
    /// Score data property
    pub score: ScoreDataProperty,
    /// Cached rubato_types version of timing_distribution for SkinRenderContext.
    /// Updated via `sync_timing_distribution_cache()` after statistics are calculated.
    pub timing_distribution_cache: rubato_types::timing_distribution::TimingDistribution,
}

impl AbstractResultData {
    pub fn new() -> Self {
        let dist_range = 150;
        Self {
            state: STATE_OFFLINE,
            ranking: None,
            ranking_offset: 0,
            avgduration: 0,
            avg: 0,
            stddev: 0,
            timing_distribution: TimingDistribution::new(dist_range),
            dist_range,
            save_replay: [ReplayStatus::NotExist; REPLAY_SIZE],
            gauge_type: 0,
            oldscore: ScoreData::default(),
            score: ScoreDataProperty::new(),
            timing_distribution_cache:
                rubato_types::timing_distribution::TimingDistribution::default(),
        }
    }

    /// Synchronize the rubato_types TimingDistribution cache from the local
    /// TimingDistribution data. Call this after `statistic_value_calculate()`.
    pub fn sync_timing_distribution_cache(&mut self) {
        self.timing_distribution_cache = rubato_types::timing_distribution::TimingDistribution {
            distribution: self.timing_distribution.timing_distribution().to_vec(),
            array_center: self.timing_distribution.array_center(),
            average: self.timing_distribution.average(),
            std_dev: self.timing_distribution.std_dev(),
        };
    }

    #[allow(dead_code)] // visibility intentionally reduced; callers will be wired later
    pub(crate) fn replay_status(&self, index: usize) -> Option<ReplayStatus> {
        self.save_replay.get(index).copied()
    }

    pub fn gauge_type(&self) -> i32 {
        self.gauge_type
    }

    pub fn state(&self) -> i32 {
        self.state
    }

    pub fn ranking_data(&self) -> Option<&RankingData> {
        self.ranking.as_ref()
    }

    pub fn ir_rank(&self) -> i32 {
        if let Some(ref r) = self.ranking {
            r.rank()
        } else {
            0
        }
    }

    pub fn old_ir_rank(&self) -> i32 {
        if let Some(ref r) = self.ranking {
            r.previous_rank()
        } else {
            0
        }
    }

    pub fn ir_total_player(&self) -> i32 {
        if let Some(ref r) = self.ranking {
            r.total_player()
        } else {
            0
        }
    }

    pub fn average_duration(&self) -> i64 {
        self.avgduration
    }

    pub fn average(&self) -> i64 {
        self.avg
    }

    pub fn stddev(&self) -> i64 {
        self.stddev
    }

    pub fn old_score(&self) -> &ScoreData {
        &self.oldscore
    }

    pub fn timing_distribution(&self) -> &TimingDistribution {
        &self.timing_distribution
    }

    pub fn input(&mut self, snapshot: &rubato_input::input_snapshot::InputSnapshot) {
        let mov = snapshot.scroll_y as i32;
        if mov != 0
            && let Some(ref ranking) = self.ranking
        {
            let total = ranking.total_player();
            let ranking_max = 1i32.max(total);
            self.ranking_offset = (self.ranking_offset + mov).clamp(0, ranking_max - 1);
        }
    }

    pub fn ranking_offset(&self) -> i32 {
        self.ranking_offset
    }

    pub fn ranking_position(&self) -> f32 {
        let ranking_max: i32 = if let Some(ref r) = self.ranking {
            let total = r.total_player();
            1i32.max(total)
        } else {
            1
        };
        self.ranking_offset as f32 / ranking_max as f32
    }

    pub fn set_ranking_position(&mut self, value: f32) {
        if (0.0..1.0).contains(&value) {
            let ranking_max: i32 = if let Some(ref r) = self.ranking {
                let total = r.total_player();
                1i32.max(total)
            } else {
                1
            };
            self.ranking_offset = (ranking_max as f32 * value) as i32;
        }
    }

    pub fn score_data_property(&self) -> &ScoreDataProperty {
        &self.score
    }

    pub fn score_data_property_mut(&mut self) -> &mut ScoreDataProperty {
        &mut self.score
    }
}

impl Default for AbstractResultData {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: build a ScoreData with specific fields set for is_qualified tests.
    fn make_score(exscore_epg: i32, minbp: i32, maxcombo: i32, clear: i32) -> ScoreData {
        let mut s = ScoreData::default();
        // exscore = epg*2 + lpg*2 + egr + lgr; set epg only for simplicity
        s.judge_counts.epg = exscore_epg;
        s.minbp = minbp;
        s.maxcombo = maxcombo;
        s.clear = clear;
        s
    }

    // ---- ReplayAutoSaveConstraint::is_qualified tests ----

    #[test]
    fn nothing_never_qualifies() {
        let old = make_score(10, 5, 50, 5);
        let new = make_score(20, 1, 100, 9);
        assert!(!ReplayAutoSaveConstraint::Nothing.is_qualified(&old, &new));
    }

    #[test]
    fn always_qualifies() {
        let old = make_score(10, 5, 50, 5);
        let new = make_score(0, 100, 0, 0);
        assert!(ReplayAutoSaveConstraint::Always.is_qualified(&old, &new));
    }

    #[test]
    fn score_update_strictly_better() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(11, 5, 50, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(9, 5, 50, 5);
        assert!(ReplayAutoSaveConstraint::ScoreUpdate.is_qualified(&old, &better));
        assert!(!ReplayAutoSaveConstraint::ScoreUpdate.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::ScoreUpdate.is_qualified(&old, &worse));
    }

    #[test]
    fn score_update_or_equal() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(11, 5, 50, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(9, 5, 50, 5);
        assert!(ReplayAutoSaveConstraint::ScoreUpdateOrEqual.is_qualified(&old, &better));
        assert!(ReplayAutoSaveConstraint::ScoreUpdateOrEqual.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::ScoreUpdateOrEqual.is_qualified(&old, &worse));
    }

    #[test]
    fn misscount_update_strictly_better() {
        // minbp lower is better
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 4, 50, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 6, 50, 5);
        assert!(ReplayAutoSaveConstraint::MisscountUpdate.is_qualified(&old, &better));
        assert!(!ReplayAutoSaveConstraint::MisscountUpdate.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::MisscountUpdate.is_qualified(&old, &worse));
    }

    #[test]
    fn misscount_update_noplay_special_case() {
        // When old clear == NoPlay (id=0), MisscountUpdate qualifies regardless of minbp
        let old_noplay = make_score(10, 5, 50, ClearType::NoPlay.id());
        let new_worse_bp = make_score(10, 100, 50, 5);
        assert!(ReplayAutoSaveConstraint::MisscountUpdate.is_qualified(&old_noplay, &new_worse_bp));
    }

    #[test]
    fn misscount_update_or_equal() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 4, 50, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 6, 50, 5);
        assert!(ReplayAutoSaveConstraint::MisscountUpdateOrEqual.is_qualified(&old, &better));
        assert!(ReplayAutoSaveConstraint::MisscountUpdateOrEqual.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::MisscountUpdateOrEqual.is_qualified(&old, &worse));
    }

    #[test]
    fn misscount_update_or_equal_noplay_special_case() {
        let old_noplay = make_score(10, 5, 50, ClearType::NoPlay.id());
        let new_worse_bp = make_score(10, 100, 50, 5);
        assert!(
            ReplayAutoSaveConstraint::MisscountUpdateOrEqual
                .is_qualified(&old_noplay, &new_worse_bp)
        );
    }

    #[test]
    fn maxcombo_update_strictly_better() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 5, 51, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 5, 49, 5);
        assert!(ReplayAutoSaveConstraint::MaxcomboUpdate.is_qualified(&old, &better));
        assert!(!ReplayAutoSaveConstraint::MaxcomboUpdate.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::MaxcomboUpdate.is_qualified(&old, &worse));
    }

    #[test]
    fn maxcombo_update_or_equal() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 5, 51, 5);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 5, 49, 5);
        assert!(ReplayAutoSaveConstraint::MaxcomboUpdateOrEqual.is_qualified(&old, &better));
        assert!(ReplayAutoSaveConstraint::MaxcomboUpdateOrEqual.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::MaxcomboUpdateOrEqual.is_qualified(&old, &worse));
    }

    #[test]
    fn clear_update_strictly_better() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 5, 50, 6);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 5, 50, 4);
        assert!(ReplayAutoSaveConstraint::ClearUpdate.is_qualified(&old, &better));
        assert!(!ReplayAutoSaveConstraint::ClearUpdate.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::ClearUpdate.is_qualified(&old, &worse));
    }

    #[test]
    fn clear_update_or_equal() {
        let old = make_score(10, 5, 50, 5);
        let better = make_score(10, 5, 50, 6);
        let equal = make_score(10, 5, 50, 5);
        let worse = make_score(10, 5, 50, 4);
        assert!(ReplayAutoSaveConstraint::ClearUpdateOrEqual.is_qualified(&old, &better));
        assert!(ReplayAutoSaveConstraint::ClearUpdateOrEqual.is_qualified(&old, &equal));
        assert!(!ReplayAutoSaveConstraint::ClearUpdateOrEqual.is_qualified(&old, &worse));
    }

    #[test]
    fn anyone_update_each_dimension() {
        let old = make_score(10, 5, 50, 5);

        // Only clear improved
        let clear_up = make_score(10, 5, 50, 6);
        assert!(ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &clear_up));

        // Only maxcombo improved
        let combo_up = make_score(10, 5, 51, 5);
        assert!(ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &combo_up));

        // Only minbp improved (lower is better)
        let bp_up = make_score(10, 4, 50, 5);
        assert!(ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &bp_up));

        // Only exscore improved
        let score_up = make_score(11, 5, 50, 5);
        assert!(ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &score_up));

        // Nothing improved
        let same = make_score(10, 5, 50, 5);
        assert!(!ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &same));

        // Everything worse
        let worse = make_score(9, 6, 49, 4);
        assert!(!ReplayAutoSaveConstraint::AnyoneUpdate.is_qualified(&old, &worse));
    }

    // ---- ReplayAutoSaveConstraint::get tests ----

    #[test]
    fn get_valid_indices() {
        let values = ReplayAutoSaveConstraint::values();
        for (i, expected) in values.iter().enumerate() {
            assert_eq!(ReplayAutoSaveConstraint::get(i as i32), *expected);
        }
    }

    #[test]
    fn get_out_of_bounds_returns_nothing() {
        assert_eq!(
            ReplayAutoSaveConstraint::get(-1),
            ReplayAutoSaveConstraint::Nothing
        );
        assert_eq!(
            ReplayAutoSaveConstraint::get(ReplayAutoSaveConstraint::values().len() as i32),
            ReplayAutoSaveConstraint::Nothing
        );
        assert_eq!(
            ReplayAutoSaveConstraint::get(100),
            ReplayAutoSaveConstraint::Nothing
        );
    }

    // ---- TimingDistribution tests ----

    #[test]
    fn timing_distribution_basic_average() {
        let mut td = TimingDistribution::new(10);
        td.add(3);
        td.add(-3);
        td.statistic_value_calculate();
        assert!((td.average() - 0.0).abs() < 0.01);
    }

    #[test]
    fn timing_distribution_no_overflow_with_large_counts() {
        // Simulate a scenario that would overflow i32 accumulators:
        // range=100, all bins at max. count * offset can exceed i32::MAX.
        let mut td = TimingDistribution::new(100);
        // Fill bins so that sum of (d * offset) would overflow i32.
        // With 201 bins, placing large values at the extremes stresses the accumulator.
        for i in 0..td.dist.len() {
            td.dist[i] = 100_000;
        }
        // count = 201 * 100_000 = 20_100_000
        // sum = sum of (100_000 * (i - 100)) for i in 0..201
        //     = 100_000 * sum(i - 100 for i in 0..201)
        //     = 100_000 * 0 = 0  (symmetric)
        // With i32, intermediate products like 100_000 * 100 = 10_000_000 are fine,
        // but let's use bigger values to actually trigger the issue.
        td.dist[0] = i32::MAX / 2; // bin at offset -100
        td.dist[200] = i32::MAX / 2; // bin at offset +100
        // sum_i64 = (MAX/2)*(-100) + (MAX/2)*(100) + small terms = ~0
        // count_i64 = MAX/2 + MAX/2 + 199*100_000 = ~2^31 + 19_900_000
        // This would overflow i32 for count alone.
        td.statistic_value_calculate();
        // The key assertion: no panic, and average is near 0 (symmetric distribution).
        assert!(
            td.average().abs() < 1.0,
            "average should be near 0, got {}",
            td.average()
        );
        assert!(td.std_dev() > 0.0, "std_dev should be positive");
    }

    #[test]
    fn timing_distribution_empty_does_not_divide_by_zero() {
        let mut td = TimingDistribution::new(10);
        td.statistic_value_calculate();
        assert_eq!(td.average(), f32::MAX);
        assert_eq!(td.std_dev(), -1.0);
    }
}
