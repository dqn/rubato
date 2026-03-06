// AbstractResult.java -> abstract_result.rs
// Mechanical line-by-line translation.

use rubato_core::clear_type::ClearType;
use rubato_core::score_data::ScoreData;
use rubato_core::score_data_property::ScoreDataProperty;

use super::stubs::{MainController, RankingData, TimerManager};

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
        let mut count = 0;
        let mut sum = 0;
        let mut sumf: f32 = 0.0;

        for i in 0..self.dist.len() {
            count += self.dist[i];
            sum += self.dist[i] * (i as i32 - self.array_center);
        }

        if count == 0 {
            return;
        }

        self.average = sum as f32 * 1.0 / count as f32;

        for i in 0..self.dist.len() {
            sumf += self.dist[i] as f32
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
    /// Timer manager
    pub timer: TimerManager,
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
            timer: TimerManager::new(),
        }
    }

    pub fn replay_status(&self, index: usize) -> ReplayStatus {
        self.save_replay[index]
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

    pub fn input(&mut self, main: &mut MainController) {
        let input = main.input_processor();
        let mov = -(input.scroll());
        input.reset_scroll();
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
